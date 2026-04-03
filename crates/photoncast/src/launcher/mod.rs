//! Main launcher window component for PhotonCast.
//!
//! This module contains the `LauncherWindow` struct that implements
//! the GPUI `Render` trait for the main launcher UI.
//!
//! # Animations
//!
//! The launcher supports the following animations (all respecting reduce motion):
//! - Window appear: 150ms ease-out scale with immediate visibility
//! - Window dismiss: 100ms ease-in fade + scale down
//! - Selection change: 80ms ease-in-out background transition
//! - Hover highlight: 60ms linear background transition

// Note: clippy::unreadable_literal, unused_self, suboptimal_flops, struct_excessive_bools
// were previously suppressed at module level but removed during cleanup.
// If clippy flags these, add targeted #[allow(...)] on specific items.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;
use parking_lot::RwLock;
use photoncast_apps::{
    AppManager, AppsConfig, AutoQuitConfig, AutoQuitManager, UninstallPreview,
    DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES,
};
use photoncast_calculator::commands::{is_calculator_expression, CalculatorCommand};
use photoncast_calculator::{CalculatorResult, CalculatorResultKind};
use photoncast_timer::commands::TimerManager;

use crate::app_events::{self, AppEvent};
use crate::constants::{
    ThemeColorSet, EXPANDED_HEIGHT, ICON_SIZE_LG, ICON_SIZE_MD, ICON_SIZE_SM, LAUNCHER_HEIGHT,
    LAUNCHER_WIDTH, LIST_ITEM_HEIGHT, SEARCH_BAR_HEIGHT, TEXT_SIZE_LG, TEXT_SIZE_MD,
};
use crate::extension_views::ExtensionViewCallbackPayload;
use crate::{
    Activate, Cancel, ConfirmDialog, CopyBundleId, CopyFile, CopyPath, ForceQuitApp, HideApp,
    NextGroup, OpenPreferences, PreviousGroup, QuickLook, QuickSelect1, QuickSelect2, QuickSelect3,
    QuickSelect4, QuickSelect5, QuickSelect6, QuickSelect7, QuickSelect8, QuickSelect9, QuitApp,
    RevealInFinder, SelectNext, SelectPrevious, ShowActionsMenu, ShowInFinder, ToggleAutoQuit,
    UninstallApp, LAUNCHER_BORDER_RADIUS,
};

use photoncast_core::app::integration::PhotonCastApp;
use photoncast_core::commands::{CommandExecutor, ConfirmationDialog, SystemCommand};
use photoncast_core::indexer::{AppScanner, AppWatcher, WatchEvent};
use photoncast_core::platform::launch::AppLauncher;
use photoncast_core::search::{
    IconSource, ResultType as CoreResultType, SearchAction, SearchResult, SearchResultId,
};
use photoncast_core::storage::{Database, UsageTracker};
use photoncast_core::theme::PhotonTheme;
use photoncast_core::ui::animations::{
    ease_in, ease_in_out, ease_out, lerp, selection_change_duration, window_appear_duration,
    window_dismiss_duration, WindowAnimationState, WINDOW_APPEAR_OPACITY_END,
    WINDOW_APPEAR_OPACITY_START, WINDOW_APPEAR_SCALE_END, WINDOW_APPEAR_SCALE_START,
    WINDOW_DISMISS_SCALE_END,
};

mod actions;
mod animation;
mod calculator;
mod calendar;
mod indexing;
mod render;
mod render_actions;
mod render_calendar;
mod render_query;
mod render_results;
mod search;
mod uninstall;

/// Type alias – the launcher uses the shared [`ThemeColorSet`] from constants.
type LauncherColors = ThemeColorSet;

fn get_launcher_colors(cx: &ViewContext<LauncherWindow>) -> LauncherColors {
    let theme = cx.try_global::<PhotonTheme>().cloned().unwrap_or_default();
    LauncherColors::from_theme(&theme)
}

/// Search icon size
#[allow(dead_code)]
const SEARCH_ICON_SIZE: Pixels = px(20.0);
/// Result item height (alias for the shared LIST_ITEM_HEIGHT constant)
const RESULT_ITEM_HEIGHT: Pixels = LIST_ITEM_HEIGHT;
/// Maximum visible results
const MAX_VISIBLE_RESULTS: usize = 8;

// ============================================================================
// LauncherWindow sub-structs — logical grouping of related state
// ============================================================================

/// Search-related state (hot-path fields grouped for cache locality)
pub struct SearchState {
    /// Current search query
    pub query: SharedString,
    /// Cursor position in the query (character index)
    pub cursor_position: usize,
    /// Selection anchor position (where selection started, None if no selection)
    pub selection_anchor: Option<usize>,
    /// Time when cursor last moved (for blink reset)
    pub cursor_blink_epoch: Instant,
    /// Currently selected result index
    pub selected_index: usize,
    /// Filtered results for current query
    pub results: Vec<ResultItem>,
    /// Base search results for current query (without calculator)
    pub base_results: Vec<SearchResult>,
    /// Core search results (for activation)
    pub core_results: Vec<SearchResult>,
    /// Current search mode (Normal or `FileSearch`)
    pub mode: SearchMode,
    /// Suggestions (recent/frequent apps shown when query is empty)
    pub suggestions: Vec<SearchResult>,
    /// Generation counter for debounced normal-mode search.
    pub normal_search_generation: u64,
    /// Cancellation flag for the currently scheduled normal-mode search.
    pub normal_search_cancel: Option<Arc<AtomicBool>>,
}

/// Window animation state
pub struct AnimationState {
    /// Window animation state
    pub window_state: WindowAnimationState,
    /// Time when the current animation started
    pub start: Option<Instant>,
    /// Previously selected index (for selection change animation)
    pub previous_selected_index: Option<usize>,
    /// Time when selection changed (for selection animation)
    pub selection_start: Option<Instant>,
    /// Index of the currently hovered result item (for hover animation)
    #[allow(dead_code)]
    pub hovered_index: Option<usize>,
    /// Hover animation starts per item (for smooth hover transitions)
    #[allow(dead_code)]
    pub hover_starts: std::collections::HashMap<usize, Instant>,
}

/// Calculator evaluation state
pub struct CalculatorState {
    /// Calculator command state
    pub command: Arc<RwLock<CalculatorCommand>>,
    /// Tokio runtime for calculator evaluation
    pub runtime: Arc<tokio::runtime::Runtime>,
    /// Latest calculator result for current query
    pub result: Option<CalculatorResult>,
    /// Calculator evaluation generation (incremented on each keystroke)
    pub generation: u64,
}

/// File search overlay state
pub struct FileSearchState {
    /// File search view (shown when in FileSearch mode)
    pub view: Option<View<crate::file_search_view::FileSearchView>>,
    /// Whether file search is loading
    pub loading: bool,
    /// Last file search query (for debouncing)
    pub pending_query: Option<String>,
    /// File search debounce generation (incremented on each keystroke)
    pub generation: u64,
}

/// Extension view overlay state
pub struct ExtensionViewState {
    /// Extension view (shown when an extension renders a view)
    pub view: Option<AnyView>,
    /// ID of the extension whose view is displayed
    pub id: Option<String>,
}

/// Uninstall preview state
pub struct UninstallState {
    /// Current uninstall preview being displayed
    pub preview: Option<UninstallPreview>,
    /// Selected index in the uninstall files list
    pub files_selected_index: usize,
}

/// Meeting / calendar state
pub struct MeetingState {
    /// Next upcoming meeting (shown at top of launcher)
    pub next_meeting: Option<photoncast_calendar::CalendarEvent>,
    /// Whether the next meeting widget is selected (for navigation)
    pub selected: bool,
    /// All calendar events (stored when entering calendar mode, used for filtering)
    pub all_events: Vec<photoncast_calendar::CalendarEvent>,
}

/// Toast notification state
pub struct ToastState {
    /// Current toast message to display
    pub message: Option<String>,
    /// When the toast was shown (for auto-dismiss)
    pub shown_at: Option<Instant>,
}

/// Actions menu state (Cmd+K)
pub struct ActionsMenuState {
    /// Whether the actions menu is visible
    pub visible: bool,
    /// Selected index in the actions menu (for keyboard navigation)
    pub selected_index: usize,
}

/// Auto quit settings and management state
pub struct AutoQuitState {
    /// Currently selected app for auto-quit settings (bundle_id, app_name)
    pub settings_app: Option<(String, String)>,
    /// Selected timeout index in auto-quit settings (0 = toggle, 1-7 = timeout options)
    pub settings_index: usize,
    /// Auto quit manager
    pub manager: Arc<RwLock<AutoQuitManager>>,
    /// Whether we're in the "Manage Auto Quits" mode
    pub manage_mode: bool,
    /// Selected index in the manage auto quits list
    pub manage_index: usize,
}

/// The main launcher window state
pub struct LauncherWindow {
    // Sub-structs grouping related state
    search: SearchState,
    animation: AnimationState,
    calculator: CalculatorState,
    file_search: FileSearchState,
    extension_view: ExtensionViewState,
    uninstall: UninstallState,
    meeting: MeetingState,
    toast: ToastState,
    actions_menu: ActionsMenuState,
    auto_quit: AutoQuitState,

    // Remaining fields that don't fit neatly into groups
    visible: bool,
    focus_handle: FocusHandle,
    photoncast_app: Arc<RwLock<PhotonCastApp>>,
    app_launcher: Arc<AppLauncher>,
    command_executor: Arc<CommandExecutor>,
    index_started: Arc<AtomicBool>,
    index_initialized: bool,
    pending_confirmation: Option<(SystemCommand, ConfirmationDialog)>,
    pending_permissions_consent: Option<crate::permissions_dialog::PendingPermissionsConsent>,
    first_launch_consent_queue: Vec<(
        String,
        photoncast_core::extensions::permissions::PermissionsDialog,
    )>,
    first_launch_checked: bool,
    timer_manager: Arc<tokio::sync::RwLock<TimerManager>>,
    app_manager: Arc<AppManager>,
    results_scroll_handle: gpui::ScrollHandle,
    previous_frontmost_app: Option<String>,
    previous_frontmost_window_title: Option<String>,
    _appearance_subscription: Option<Subscription>,
    /// Active Quick Look (qlmanage) child process, if any.
    /// Stored so it can be killed when a new preview is requested or the launcher is hidden.
    qlmanage_child: Option<std::process::Child>,
}

#[derive(Clone)]
pub struct LauncherSharedState {
    photoncast_app: Arc<RwLock<PhotonCastApp>>,
    app_launcher: Arc<AppLauncher>,
    command_executor: Arc<CommandExecutor>,
    index_started: Arc<AtomicBool>,
    calculator_command: Arc<RwLock<CalculatorCommand>>,
    calculator_runtime: Arc<tokio::runtime::Runtime>,
    timer_manager: Arc<tokio::sync::RwLock<TimerManager>>,
    app_manager: Arc<AppManager>,
}

impl LauncherSharedState {
    #[must_use]
    pub fn new(shared_runtime: Arc<tokio::runtime::Runtime>) -> Self {
        // Use persistent database for usage tracking (frecency/recommendations)
        let db_path = photoncast_core::utils::paths::data_dir().join("usage.db");
        let usage_tracker = match Database::open(&db_path) {
            Ok(db) => {
                tracing::info!("Opened usage database at {:?}", db_path);
                UsageTracker::new(db)
            },
            Err(e) => {
                tracing::warn!(
                    "Failed to open database at {:?}: {}, falling back to in-memory",
                    db_path,
                    e
                );
                // Single fallback attempt - if in-memory fails, panic is acceptable
                // since we can't function without any database at all
                let fallback_db = Database::open_in_memory()
                    .expect("Critical: cannot open even in-memory database");
                UsageTracker::new(fallback_db)
            },
        };

        let config = photoncast_core::app::integration::IntegrationConfig {
            search_timeout_ms: 100,
            include_files: false,
            ..Default::default()
        };

        let usage_tracker = Arc::new(usage_tracker);
        let mut app = PhotonCastApp::with_config(config);
        app.set_usage_tracker(Arc::clone(&usage_tracker));
        let photoncast_app = Arc::new(RwLock::new(app));
        let app_launcher = Arc::new(AppLauncher::with_shared_tracker(usage_tracker));
        let command_executor = Arc::new(CommandExecutor::new());
        let calculator_command = Arc::new(RwLock::new(CalculatorCommand::new()));
        let calculator_runtime = Arc::clone(&shared_runtime);
        let timer_db_path = photoncast_core::utils::paths::data_dir().join("timer.db");
        #[allow(clippy::arc_with_non_send_sync)]
        let timer_manager = Arc::new(tokio::sync::RwLock::new({
            shared_runtime
                .block_on(TimerManager::new(timer_db_path.clone()))
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        "Failed to open timer db at {:?}: {}, using /tmp fallback",
                        timer_db_path,
                        e
                    );
                    shared_runtime
                        .block_on(TimerManager::new(std::path::PathBuf::from(
                            "/tmp/photoncast_timer.db",
                        )))
                        .expect("Critical: cannot initialize timer manager even with fallback path")
                })
        }));
        let app_manager = Arc::new(AppManager::new(AppsConfig::default()));

        Self {
            photoncast_app,
            app_launcher,
            command_executor,
            index_started: Arc::new(AtomicBool::new(false)),
            calculator_command,
            calculator_runtime,
            timer_manager,
            app_manager,
        }
    }

    /// Returns a reference to the timer manager for background polling
    pub fn timer_manager(&self) -> Arc<tokio::sync::RwLock<TimerManager>> {
        Arc::clone(&self.timer_manager)
    }

    /// Invalidates the quicklinks cache, causing a reload on next search.
    /// Call this after adding, updating, or deleting quicklinks.
    pub fn invalidate_quicklinks_cache(&self) {
        self.photoncast_app.read().invalidate_quicklinks_cache();
    }

    /// Returns a clone of the PhotonCastApp reference.
    /// Useful for callbacks that need to invalidate caches.
    pub fn photoncast_app(&self) -> Arc<RwLock<PhotonCastApp>> {
        Arc::clone(&self.photoncast_app)
    }
}

/// A single result item for UI display
#[derive(Clone)]
pub struct ResultItem {
    #[allow(dead_code)]
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    /// Emoji fallback icon
    pub icon_emoji: SharedString,
    /// Path to the app icon (.icns file) if available
    pub icon_path: Option<std::path::PathBuf>,
    pub result_type: ResultType,
    /// Bundle ID for applications (used for running/auto-quit indicators)
    pub bundle_id: Option<String>,
    /// App path for applications (used for reveal in finder, uninstall)
    pub app_path: Option<std::path::PathBuf>,
    /// Whether this result requires permissions consent
    pub requires_permissions: bool,
}

/// Type of search result for grouping
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ResultType {
    Application,
    Command,
    QuickLink,
    File,
    Folder,
    Calculator,
}

impl ResultType {
    #[allow(dead_code)]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Application => "Apps",
            Self::Command => "Commands",
            Self::QuickLink => "Quick Links",
            Self::File => "Files",
            Self::Folder => "Folders",
            Self::Calculator => "Calculator",
        }
    }
}

impl From<CoreResultType> for ResultType {
    fn from(core_type: CoreResultType) -> Self {
        match core_type {
            CoreResultType::Application => Self::Application,
            CoreResultType::SystemCommand
            | CoreResultType::CustomCommand
            | CoreResultType::Extension => Self::Command,
            CoreResultType::QuickLink => Self::QuickLink,
            CoreResultType::File => Self::File,
            CoreResultType::Folder => Self::Folder,
        }
    }
}

/// Search mode determines the UI state and behavior.
#[derive(Clone, Default, Debug)]
pub enum SearchMode {
    /// Normal search mode: Apps + Commands (default)
    #[default]
    Normal,
    /// File Search Mode: Spotlight-based file search
    FileSearch,
    Calendar {
        title: String,
        events: Vec<photoncast_calendar::CalendarEvent>,
        error: Option<String>,
    },
}

impl LauncherWindow {
    pub(super) fn handle_extension_view_callback(
        &mut self,
        payload: ExtensionViewCallbackPayload,
        cx: &mut ViewContext<Self>,
    ) {
        let current_extension = self.extension_view.id.as_deref();
        if current_extension != Some(payload.extension_id()) {
            tracing::warn!(
                callback_extension_id = %payload.extension_id(),
                active_extension_id = ?current_extension,
                "Ignoring extension callback for inactive extension view"
            );
            return;
        }

        match payload {
            ExtensionViewCallbackPayload::CloseView { .. } => {
                self.close_extension_view(cx);
            },
            ExtensionViewCallbackPayload::CallbackAction {
                extension_id,
                action_id,
            } => {
                tracing::info!(
                    extension_id = %extension_id,
                    action_id = %action_id,
                    "Extension callback action triggered"
                );
            },
            ExtensionViewCallbackPayload::SubmitForm {
                extension_id,
                values_json,
            } => {
                tracing::info!(
                    extension_id = %extension_id,
                    payload_size = values_json.len(),
                    "Extension form submitted"
                );
                self.close_extension_view(cx);
            },
            ExtensionViewCallbackPayload::DelegatedAction {
                extension_id,
                action_id,
                action,
                should_close,
            } => {
                let result = self
                    .photoncast_app
                    .read()
                    .execute_extension_view_action(&extension_id, &action);

                match result {
                    Ok(()) => {
                        tracing::info!(
                            extension_id = %extension_id,
                            action_id = %action_id,
                            "Executed delegated extension action"
                        );
                        if should_close {
                            self.hide(cx);
                        }
                    },
                    Err(
                        photoncast_core::app::integration::ExtensionActionError::PermissionsConsentRequired {
                            extension_id: ext_id,
                            dialog,
                        },
                    ) => {
                        tracing::info!(
                            extension_id = %ext_id,
                            action_id = %action_id,
                            "Extension delegated action requires permissions consent"
                        );
                        self.pending_permissions_consent =
                            Some(crate::permissions_dialog::PendingPermissionsConsent {
                                dialog,
                                pending_command: None,
                                is_first_launch: false,
                            });
                        cx.notify();
                    },
                    Err(err) => {
                        tracing::error!(
                            extension_id = %extension_id,
                            action_id = %action_id,
                            error = %err,
                            "Failed to execute delegated extension action"
                        );
                    },
                }
            },
        }
    }

    pub(super) fn show_extension_view(
        &mut self,
        extension_id: &str,
        ext_view: photoncast_extension_api::ExtensionView,
        cx: &mut ViewContext<Self>,
    ) {
        tracing::info!(
            extension_id = %extension_id,
            "Extension rendered a view, displaying it"
        );

        let view_handle = cx.view().downgrade();
        let action_callback: crate::extension_views::ActionCallback =
            std::sync::Arc::new(move |payload, cx| {
                if let Some(view) = view_handle.upgrade() {
                    view.update(cx, |launcher, cx| {
                        launcher.handle_extension_view_callback(payload, cx);
                    });
                }
            });

        let rendered = crate::extension_views::render_extension_view(
            ext_view,
            extension_id.to_string(),
            Some(action_callback),
            cx,
        );

        // Focus the extension view so it receives keyboard events
        if let Ok(list_view) = rendered
            .clone()
            .downcast::<crate::extension_views::ExtensionListView>()
        {
            cx.focus_view(&list_view);
        }
        self.extension_view.view = Some(rendered);
        self.extension_view.id = Some(extension_id.to_string());

        // Resize window to fit extension view
        crate::platform::resize_window(
            crate::constants::LAUNCHER_WIDTH.0.into(),
            crate::constants::EXPANDED_HEIGHT.0.into(),
        );
        cx.notify();
    }

    /// Creates a new launcher window
    pub fn new(cx: &mut ViewContext<Self>, shared_state: &LauncherSharedState) -> Self {
        let focus_handle = cx.focus_handle();

        // Request focus immediately
        cx.focus(&focus_handle);

        let photoncast_app = Arc::clone(&shared_state.photoncast_app);
        let app_launcher = Arc::clone(&shared_state.app_launcher);
        let command_executor = Arc::clone(&shared_state.command_executor);
        let index_started = Arc::clone(&shared_state.index_started);
        let calculator_command = Arc::clone(&shared_state.calculator_command);
        let calculator_runtime = Arc::clone(&shared_state.calculator_runtime);
        let timer_manager = Arc::clone(&shared_state.timer_manager);
        let app_manager = Arc::clone(&shared_state.app_manager);

        let mut window = Self {
            search: SearchState {
                query: SharedString::default(),
                cursor_position: 0,
                selection_anchor: None,
                cursor_blink_epoch: Instant::now(),
                selected_index: 0,
                results: vec![],
                base_results: vec![],
                core_results: vec![],
                mode: SearchMode::Normal,
                suggestions: vec![],
                normal_search_generation: 0,
                normal_search_cancel: None,
            },
            animation: AnimationState {
                window_state: WindowAnimationState::Hidden,
                start: None,
                previous_selected_index: None,
                selection_start: None,
                hovered_index: None,
                hover_starts: std::collections::HashMap::new(),
            },
            calculator: CalculatorState {
                command: calculator_command,
                runtime: calculator_runtime,
                result: None,
                generation: 0,
            },
            file_search: FileSearchState {
                view: None,
                loading: false,
                pending_query: None,
                generation: 0,
            },
            extension_view: ExtensionViewState {
                view: None,
                id: None,
            },
            uninstall: UninstallState {
                preview: None,
                files_selected_index: 0,
            },
            meeting: MeetingState {
                next_meeting: None,
                selected: false,
                all_events: vec![],
            },
            toast: ToastState {
                message: None,
                shown_at: None,
            },
            actions_menu: ActionsMenuState {
                visible: false,
                selected_index: 0,
            },
            auto_quit: AutoQuitState {
                settings_app: None,
                settings_index: 0,
                manager: Arc::new(RwLock::new(
                    AutoQuitManager::load()
                        .unwrap_or_else(|_| AutoQuitManager::new(AutoQuitConfig::default())),
                )),
                manage_mode: false,
                manage_index: 0,
            },
            visible: true,
            focus_handle,
            photoncast_app,
            app_launcher,
            command_executor,
            index_started,
            index_initialized: false,
            pending_confirmation: None,
            pending_permissions_consent: None,
            first_launch_consent_queue: Vec::new(),
            first_launch_checked: false,
            timer_manager,
            app_manager,
            results_scroll_handle: gpui::ScrollHandle::new(),
            previous_frontmost_app: None,
            previous_frontmost_window_title: None,
            _appearance_subscription: None,
            qlmanage_child: None,
        };

        // Set up appearance observation for auto theme switching.
        // Stored on the window so the subscription lives as long as the window.
        window._appearance_subscription =
            Some(cx.observe_window_appearance(|_view: &mut Self, cx| {
                use photoncast_core::platform::appearance::flavor_from_window_appearance;
                let appearance = cx.window_appearance();
                let current_theme = cx.try_global::<PhotonTheme>().cloned();
                if let Some(theme) = current_theme {
                    if theme.auto_sync {
                        let new_flavor = flavor_from_window_appearance(appearance);
                        if theme.flavor != new_flavor {
                            tracing::info!(
                                "System appearance changed: {:?} -> {:?}",
                                theme.flavor,
                                new_flavor
                            );
                            let new_theme =
                                PhotonTheme::new(new_flavor, theme.accent).with_auto_sync(true);
                            cx.set_global(new_theme);
                            cx.refresh();
                        }
                    }
                }
            }));

        // Start the appear animation
        window.start_appear_animation(cx);

        // Start the auto-quit background timer
        window.start_auto_quit_timer(cx);

        // Fetch next meeting (doesn't depend on index)
        window.fetch_next_meeting(cx);

        if !window.index_started.swap(true, Ordering::AcqRel) {
            // Spawn async task to index applications
            window.start_app_indexing(cx);
        } else if window.photoncast_app.read().app_count() > 0 {
            window.index_initialized = true;
            // Load suggestions since index is ready
            window.load_suggestions(cx);
        }

        window
    }

    /// Reset query, cursor position, and selection
    fn reset_query(&mut self) {
        self.search.query = SharedString::default();
        self.search.cursor_position = 0;
        self.search.selection_anchor = None;
        self.search.cursor_blink_epoch = Instant::now();
    }

    /// Reset cursor blink timer (call on any cursor movement)
    fn reset_cursor_blink(&mut self) {
        self.search.cursor_blink_epoch = Instant::now();
    }

    /// Check if cursor should be visible based on blink timing
    fn cursor_visible(&self) -> bool {
        const BLINK_INTERVAL_MS: u128 = 530;
        let elapsed = self.search.cursor_blink_epoch.elapsed().as_millis();
        (elapsed / BLINK_INTERVAL_MS) % 2 == 0
    }

    const fn calculator_icon(result: &CalculatorResult) -> char {
        match &result.kind {
            CalculatorResultKind::Math => '🔢',
            CalculatorResultKind::Currency { .. } => '💱',
            CalculatorResultKind::Unit { .. } => '📏',
            CalculatorResultKind::DateTime => '📅',
        }
    }

    /// Toggle the visibility of the launcher window
    #[allow(dead_code)]
    pub fn toggle(&mut self, cx: &mut ViewContext<Self>) {
        tracing::debug!("toggle() called, visible was {}", self.visible);
        self.visible = !self.visible;
        if self.visible {
            tracing::debug!(
                "toggle: showing window, calling fetch_next_meeting and load_suggestions"
            );
            self.reset_query();
            self.search.selected_index = 0;
            self.animation.previous_selected_index = None;
            // Select meeting by default if available, otherwise first result
            self.meeting.selected = self.meeting.next_meeting.is_some();
            // Reset scroll position
            self.results_scroll_handle
                .set_offset(gpui::Point::default());
            cx.focus(&self.focus_handle);
            self.start_appear_animation(cx);
            self.fetch_next_meeting(cx);
            self.load_suggestions(cx);
        } else {
            self.start_dismiss_animation(cx);
        }
    }

    /// Shows the launcher window with animation
    #[allow(dead_code)]
    pub fn show(&mut self, cx: &mut ViewContext<Self>) {
        self.visible = true;
        self.reset_query();
        self.search.selected_index = 0;
        self.animation.previous_selected_index = None;
        cx.focus(&self.focus_handle);
        self.start_appear_animation(cx);
        self.fetch_next_meeting(cx);
        self.load_suggestions(cx);

        // Check for first-launch extension consents (only once)
        if !self.first_launch_checked {
            self.first_launch_checked = true;
            self.check_first_launch_consents(cx);
        }
    }

    /// Checks for extensions requiring first-launch consent and queues them.
    fn check_first_launch_consents(&mut self, cx: &mut ViewContext<Self>) {
        let extensions = self
            .photoncast_app
            .read()
            .get_extensions_requiring_consent();

        if extensions.is_empty() {
            return;
        }

        tracing::info!(
            count = extensions.len(),
            "Found extensions requiring first-launch consent"
        );

        // Queue all extensions needing consent
        self.first_launch_consent_queue = extensions;

        // Show the first one
        self.show_next_first_launch_consent(cx);
    }

    /// Shows the next extension consent dialog from the queue.
    fn show_next_first_launch_consent(&mut self, cx: &mut ViewContext<Self>) {
        if let Some((extension_id, dialog)) = self.first_launch_consent_queue.first().cloned() {
            tracing::info!(
                extension_id = %extension_id,
                "Showing first-launch consent dialog"
            );
            self.pending_permissions_consent =
                Some(crate::permissions_dialog::PendingPermissionsConsent {
                    dialog,
                    pending_command: None,
                    is_first_launch: true,
                });
            cx.notify();
        }
    }

    /// Kills any running qlmanage (Quick Look) process.
    ///
    /// Called before spawning a new preview or when the launcher is hidden
    /// to prevent orphaned qlmanage processes.
    fn kill_qlmanage(&mut self) {
        if let Some(ref mut child) = self.qlmanage_child {
            if let Err(e) = child.kill() {
                tracing::debug!("Failed to kill qlmanage process: {e}");
            } else {
                // Reap the child to avoid zombie processes
                let _ = child.wait();
            }
        }
        self.qlmanage_child = None;
    }

    /// Hides the launcher window with animation
    pub fn hide(&mut self, cx: &mut ViewContext<Self>) {
        if matches!(self.search.mode, SearchMode::Calendar { .. }) {
            self.exit_calendar_mode(cx);
            return;
        }

        // Clean up file search mode if active
        if matches!(self.search.mode, SearchMode::FileSearch) {
            self.search.mode = SearchMode::Normal;
            self.file_search.view = None;
        }

        // Clean up extension view if active
        if self.extension_view.view.is_some() {
            self.extension_view.view = None;
            self.extension_view.id = None;
            // Resize window back to normal
            crate::platform::resize_window(
                crate::constants::LAUNCHER_WIDTH.0.into(),
                crate::constants::LAUNCHER_HEIGHT.0.into(),
            );
        }

        // Kill any running Quick Look preview process
        self.kill_qlmanage();

        self.visible = false;
        self.start_dismiss_animation(cx);
    }

    /// Closes the extension view and returns to the search view.
    /// Called when the extension view's callback protocol emits `CloseView`.
    pub(super) fn close_extension_view(&mut self, cx: &mut ViewContext<Self>) {
        if self.extension_view.view.is_some() {
            self.extension_view.view = None;
            self.extension_view.id = None;
            // Resize window back to normal
            crate::platform::resize_window(
                crate::constants::LAUNCHER_WIDTH.0.into(),
                crate::constants::LAUNCHER_HEIGHT.0.into(),
            );
            // Re-focus the launcher's search input
            cx.focus(&self.focus_handle);
            cx.notify();
        }
    }

    /// Sets the bundle ID and window title that was frontmost before Photoncast opened.
    /// Used for window management commands to target the correct window.
    pub fn set_previous_frontmost_window(
        &mut self,
        bundle_id: Option<String>,
        window_title: Option<String>,
    ) {
        self.previous_frontmost_app = bundle_id;
        self.previous_frontmost_window_title = window_title;
    }

    // Action handlers

    /// Confirms and executes the pending command
    fn confirm_pending_command(&mut self, cx: &mut ViewContext<Self>) {
        if let Some((cmd, _dialog)) = self.pending_confirmation.take() {
            let executor = Arc::clone(&self.command_executor);
            let cmd_name = cmd.name();

            match executor.execute(cmd) {
                Ok(()) => {
                    tracing::info!("Executed confirmed command: {}", cmd_name);
                },
                Err(e) => {
                    tracing::error!("Failed to execute {}: {}", cmd_name, e);
                },
            }
            self.hide(cx);
        }
    }

    /// Handles the `ConfirmDialog` action (Enter key in confirmation dialog)
    fn confirm_dialog(&mut self, _: &ConfirmDialog, cx: &mut ViewContext<Self>) {
        if self.pending_confirmation.is_some() {
            self.confirm_pending_command(cx);
        }
    }

    /// Cancels the pending confirmation dialog and returns to search
    fn cancel_confirmation(&mut self, cx: &mut ViewContext<Self>) {
        if self.pending_confirmation.is_some() {
            self.pending_confirmation = None;
            cx.notify();
        }
    }

    /// Accepts the pending permissions consent and optionally executes the pending command.
    fn accept_permissions_consent(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(consent) = self.pending_permissions_consent.take() {
            let extension_id = consent.dialog.extension_id.clone();
            let pending_command = consent.pending_command.clone();
            let is_first_launch = consent.is_first_launch;

            // Accept permissions
            let accept_result = self
                .photoncast_app
                .read()
                .accept_extension_permissions(&extension_id);

            if let Err(e) = accept_result {
                tracing::error!(
                    extension_id = %extension_id,
                    error = %e,
                    "Failed to accept extension permissions"
                );
                // Continue to process first-launch queue even on error
                if is_first_launch {
                    self.advance_first_launch_queue(cx);
                } else {
                    cx.notify();
                }
                return;
            }

            tracing::info!(extension_id = %extension_id, "Permissions accepted");

            // If there's a pending command, execute it now (on-demand flow)
            if let Some((ext_id, cmd_id)) = pending_command {
                let result = self
                    .photoncast_app
                    .read()
                    .launch_extension_command(&ext_id, &cmd_id);

                match result {
                    Ok(()) => {
                        tracing::info!(
                            extension_id = %ext_id,
                            command_id = %cmd_id,
                            "Extension command executed after consent"
                        );
                        // Check if the extension rendered a view
                        let pending_view = self.photoncast_app.read().take_extension_view(&ext_id);
                        if let Some(ext_view) = pending_view {
                            self.show_extension_view(&ext_id, ext_view, cx);
                        } else {
                            self.hide(cx);
                        }
                    },
                    Err(e) => {
                        tracing::error!(
                            extension_id = %ext_id,
                            command_id = %cmd_id,
                            error = %e,
                            "Failed to execute extension command after consent"
                        );
                    },
                }
            } else if is_first_launch {
                // First-launch flow: move to next extension in queue
                self.advance_first_launch_queue(cx);
                return;
            }

            cx.notify();
        }
    }

    /// Advances the first-launch consent queue to the next extension.
    fn advance_first_launch_queue(&mut self, cx: &mut ViewContext<Self>) {
        // Remove the first extension from the queue (already processed)
        if !self.first_launch_consent_queue.is_empty() {
            self.first_launch_consent_queue.remove(0);
        }

        // Show next dialog or finish
        if !self.first_launch_consent_queue.is_empty() {
            self.show_next_first_launch_consent(cx);
        } else {
            cx.notify();
        }
    }

    /// Denies the pending permissions consent and closes the dialog.
    fn deny_permissions_consent(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(consent) = self.pending_permissions_consent.take() {
            let extension_id = consent.dialog.extension_id.clone();
            let is_first_launch = consent.is_first_launch;

            tracing::info!(extension_id = %extension_id, "Permissions denied by user");

            // If first-launch flow, continue to next extension
            if is_first_launch {
                self.advance_first_launch_queue(cx);
            } else {
                cx.notify();
            }
        }
    }

    /// Get the current selection range (start, end) where start <= end
    fn selection_range(&self) -> Option<(usize, usize)> {
        self.search.selection_anchor.map(|anchor| {
            if anchor <= self.search.cursor_position {
                (anchor, self.search.cursor_position)
            } else {
                (self.search.cursor_position, anchor)
            }
        })
    }

    /// Delete selected text and return the new query, or None if no selection
    fn delete_selection(&mut self) -> Option<String> {
        if let Some((start, end)) = self.selection_range() {
            let chars: Vec<char> = self.search.query.chars().collect();
            let new_query: String = chars[..start].iter().chain(chars[end..].iter()).collect();
            self.search.cursor_position = start;
            self.search.selection_anchor = None;
            Some(new_query)
        } else {
            None
        }
    }

    /// Find the previous word boundary from the given position
    fn prev_word_boundary(&self, pos: usize) -> usize {
        let chars: Vec<char> = self.search.query.chars().collect();
        if pos == 0 {
            return 0;
        }
        let mut i = pos - 1;
        // Skip whitespace
        while i > 0 && chars[i].is_whitespace() {
            i -= 1;
        }
        // Skip word characters
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        i
    }

    /// Find the next word boundary from the given position
    fn next_word_boundary(&self, pos: usize) -> usize {
        let chars: Vec<char> = self.search.query.chars().collect();
        let len = chars.len();
        if pos >= len {
            return len;
        }
        let mut i = pos;
        // Skip current word
        while i < len && !chars[i].is_whitespace() {
            i += 1;
        }
        // Skip whitespace
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        i
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        // If file search view is active, forward all key events to it
        if let Some(file_search_view) = &self.file_search.view {
            file_search_view.update(cx, |view, cx| {
                view.handle_key_down(event, cx);
            });
            return;
        }

        let key = event.keystroke.key.as_str();
        let shift = event.keystroke.modifiers.shift;
        let cmd = event.keystroke.modifiers.platform;
        let chars: Vec<char> = self.search.query.chars().collect();
        let len = chars.len();

        // Handle Tab for quicklink autocomplete
        if key == "tab" && !shift {
            if let Some(core_result) = self.search.core_results.get(self.search.selected_index) {
                if let SearchAction::ExecuteQuickLink { url_template, .. } = &core_result.action {
                    if photoncast_quicklinks::placeholder::requires_user_input(url_template) {
                        let autocomplete =
                            if let Some(alias_match) = core_result.subtitle.strip_prefix('/') {
                                alias_match
                                    .split(" · ")
                                    .next()
                                    .unwrap_or(&core_result.title)
                            } else {
                                &core_result.title
                            };
                        let new_query = format!("{} ", autocomplete);
                        self.search.cursor_position = new_query.chars().count();
                        self.search.selection_anchor = None;
                        self.search.query = SharedString::from(new_query);
                        self.on_query_change(self.search.query.clone(), cx);
                        self.reset_cursor_blink();
                        cx.notify();
                    }
                }
            }
            return;
        }

        // Cmd+A: Select all
        if cmd && key == "a" {
            if !self.search.query.is_empty() {
                self.search.selection_anchor = Some(0);
                self.search.cursor_position = len;
                self.reset_cursor_blink();
                cx.notify();
            }
            return;
        }

        // Cmd+C: Copy selection
        if cmd && key == "c" {
            if let Some((start, end)) = self.selection_range() {
                let selected: String = chars[start..end].iter().collect();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
            }
            return;
        }

        // Cmd+X: Cut selection
        if cmd && key == "x" {
            if let Some((start, end)) = self.selection_range() {
                let selected: String = chars[start..end].iter().collect();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
                if let Some(new_query) = self.delete_selection() {
                    self.search.query = SharedString::from(new_query);
                    self.on_query_change(self.search.query.clone(), cx);
                    self.reset_cursor_blink();
                    cx.notify();
                }
            }
            return;
        }

        // Cmd+V: Paste
        if cmd && key == "v" {
            if let Some(clipboard) = cx.read_from_clipboard() {
                if let Some(text) = clipboard.text() {
                    // Delete selection first if any
                    if self.search.selection_anchor.is_some() {
                        if let Some(new_query) = self.delete_selection() {
                            self.search.query = SharedString::from(new_query);
                        }
                    }
                    // Insert at cursor
                    let chars: Vec<char> = self.search.query.chars().collect();
                    let before: String = chars[..self.search.cursor_position].iter().collect();
                    let after: String = chars[self.search.cursor_position..].iter().collect();
                    let new_query = format!("{}{}{}", before, text, after);
                    self.search.cursor_position += text.chars().count();
                    self.search.query = SharedString::from(new_query);
                    self.on_query_change(self.search.query.clone(), cx);
                    self.reset_cursor_blink();
                    cx.notify();
                }
            }
            return;
        }

        let alt = event.keystroke.modifiers.alt;

        // Arrow keys for cursor movement and selection
        if key == "left" {
            if cmd && shift {
                // Cmd+Shift+Left: Select to beginning
                if self.search.selection_anchor.is_none() {
                    self.search.selection_anchor = Some(self.search.cursor_position);
                }
                self.search.cursor_position = 0;
            } else if alt && shift {
                // Option+Shift+Left: Select word left
                if self.search.selection_anchor.is_none() {
                    self.search.selection_anchor = Some(self.search.cursor_position);
                }
                self.search.cursor_position = self.prev_word_boundary(self.search.cursor_position);
            } else if cmd {
                // Cmd+Left: Move to beginning
                self.search.cursor_position = 0;
                self.search.selection_anchor = None;
            } else if alt {
                // Option+Left: Move word left
                self.search.cursor_position = self.prev_word_boundary(self.search.cursor_position);
                self.search.selection_anchor = None;
            } else if shift {
                // Shift+Left: Extend selection left
                if self.search.selection_anchor.is_none() {
                    self.search.selection_anchor = Some(self.search.cursor_position);
                }
                if self.search.cursor_position > 0 {
                    self.search.cursor_position -= 1;
                }
            } else {
                // Left: Move cursor left (collapse selection if any)
                if self.search.selection_anchor.is_some() {
                    if let Some((start, _)) = self.selection_range() {
                        self.search.cursor_position = start;
                    }
                    self.search.selection_anchor = None;
                } else if self.search.cursor_position > 0 {
                    self.search.cursor_position -= 1;
                }
            }
            self.reset_cursor_blink();
            cx.notify();
            return;
        }

        if key == "right" {
            if cmd && shift {
                // Cmd+Shift+Right: Select to end
                if self.search.selection_anchor.is_none() {
                    self.search.selection_anchor = Some(self.search.cursor_position);
                }
                self.search.cursor_position = len;
            } else if alt && shift {
                // Option+Shift+Right: Select word right
                if self.search.selection_anchor.is_none() {
                    self.search.selection_anchor = Some(self.search.cursor_position);
                }
                self.search.cursor_position = self.next_word_boundary(self.search.cursor_position);
            } else if cmd {
                // Cmd+Right: Move to end
                self.search.cursor_position = len;
                self.search.selection_anchor = None;
            } else if alt {
                // Option+Right: Move word right
                self.search.cursor_position = self.next_word_boundary(self.search.cursor_position);
                self.search.selection_anchor = None;
            } else if shift {
                // Shift+Right: Extend selection right
                if self.search.selection_anchor.is_none() {
                    self.search.selection_anchor = Some(self.search.cursor_position);
                }
                if self.search.cursor_position < len {
                    self.search.cursor_position += 1;
                }
            } else {
                // Right: Move cursor right (collapse selection if any)
                if self.search.selection_anchor.is_some() {
                    if let Some((_, end)) = self.selection_range() {
                        self.search.cursor_position = end;
                    }
                    self.search.selection_anchor = None;
                } else if self.search.cursor_position < len {
                    self.search.cursor_position += 1;
                }
            }
            self.reset_cursor_blink();
            cx.notify();
            return;
        }

        // Backspace: Delete selection or character/word before cursor
        if key == "backspace" {
            if let Some(new_query) = self.delete_selection() {
                self.search.query = SharedString::from(new_query);
                self.on_query_change(self.search.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            } else if alt && self.search.cursor_position > 0 {
                // Option+Backspace: Delete word
                let word_start = self.prev_word_boundary(self.search.cursor_position);
                let new_query: String = chars[..word_start]
                    .iter()
                    .chain(chars[self.search.cursor_position..].iter())
                    .collect();
                self.search.cursor_position = word_start;
                self.search.query = SharedString::from(new_query);
                self.on_query_change(self.search.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            } else if self.search.cursor_position > 0 {
                let new_query: String = chars[..self.search.cursor_position - 1]
                    .iter()
                    .chain(chars[self.search.cursor_position..].iter())
                    .collect();
                self.search.cursor_position -= 1;
                self.search.query = SharedString::from(new_query);
                self.on_query_change(self.search.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            }
            return;
        }

        // Delete: Delete selection or character after cursor
        if key == "delete" {
            if let Some(new_query) = self.delete_selection() {
                self.search.query = SharedString::from(new_query);
                self.on_query_change(self.search.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            } else if self.search.cursor_position < len {
                let new_query: String = chars[..self.search.cursor_position]
                    .iter()
                    .chain(chars[self.search.cursor_position + 1..].iter())
                    .collect();
                self.search.query = SharedString::from(new_query);
                self.on_query_change(self.search.query.clone(), cx);
                self.reset_cursor_blink();
                cx.notify();
            }
            return;
        }

        // Ignore other modifier combinations (except shift for uppercase)
        if cmd || event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
            return;
        }

        // Handle regular character input
        let input_text = if let Some(ime_key) = &event.keystroke.ime_key {
            Some(ime_key.clone())
        } else if key.len() == 1 {
            let ch = if shift {
                key.to_uppercase()
            } else {
                key.to_string()
            };
            Some(ch)
        } else {
            None
        };

        if let Some(text) = input_text {
            // Delete selection first if any
            let chars: Vec<char> = if self.search.selection_anchor.is_some() {
                if let Some(new_query) = self.delete_selection() {
                    self.search.query = SharedString::from(new_query);
                }
                self.search.query.chars().collect()
            } else {
                chars
            };

            // Insert at cursor
            let before: String = chars[..self.search.cursor_position].iter().collect();
            let after: String = chars[self.search.cursor_position..].iter().collect();
            let new_query = format!("{}{}{}", before, text, after);
            self.search.cursor_position += text.chars().count();
            self.search.query = SharedString::from(new_query);
            self.on_query_change(self.search.query.clone(), cx);
            self.reset_cursor_blink();
            cx.notify();
        }
    }

    /// Shows a toast notification message
    pub fn show_toast(&mut self, message: String, cx: &mut ViewContext<Self>) {
        self.toast.message = Some(message);
        self.toast.shown_at = Some(Instant::now());

        // Auto-dismiss after 2 seconds
        cx.spawn(|this, mut cx| async move {
            gpui::Timer::after(Duration::from_millis(2000)).await;
            let _ = this.update(&mut cx, |this, cx| {
                this.toast.message = None;
                this.toast.shown_at = None;
                cx.notify();
            });
        })
        .detach();

        cx.notify();
    }
}

// ============================================================================
// Helper Functions (public for testing)
// ============================================================================

/// Re-export AppleScript escaping from core for backward compatibility.
pub use photoncast_core::platform::file_actions::escape_applescript_string as escape_path_for_applescript;
