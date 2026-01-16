//! Empty state, loading state, and error state components.
//!
//! This module provides UI components for different launcher states when
//! there are no search results to display.
//!
//! # Components
//!
//! - [`EmptyState`] - Displayed when no query or no results found
//! - [`LoadingState`] - Displayed during application indexing
//! - [`ErrorState`] - Displayed when an error occurs
//!
//! # State Management
//!
//! The [`LauncherState`] enum tracks the current state of the launcher
//! and determines which component to render.

use crate::search::SearchResult;

// ============================================================================
// LauncherState - Central state enum
// ============================================================================

/// The current state of the launcher UI.
///
/// This enum determines what content is displayed in the results area.
#[derive(Debug, Clone)]
pub enum LauncherState {
    /// Empty state - no query entered or no results found.
    Empty {
        /// Whether the user has entered a query.
        has_query: bool,
        /// The query that returned no results (if `has_query` is true).
        query: Option<String>,
    },
    /// Loading state - application is indexing or searching.
    Loading {
        /// The loading message to display.
        message: String,
        /// Optional progress as a fraction (0.0 to 1.0).
        progress: Option<f32>,
        /// Optional progress text (e.g., "Found 142 of ~200 apps").
        progress_text: Option<String>,
    },
    /// Results state - search results are available.
    Results {
        /// The search results to display.
        items: Vec<SearchResult>,
        /// The currently selected index.
        selected: usize,
    },
    /// Error state - an error occurred.
    Error {
        /// The error that occurred.
        error: AppError,
        /// Available actions to recover from the error.
        actions: Vec<ErrorAction>,
    },
}

impl Default for LauncherState {
    fn default() -> Self {
        Self::Empty {
            has_query: false,
            query: None,
        }
    }
}

impl LauncherState {
    /// Creates an empty state with no query.
    #[must_use]
    pub const fn empty() -> Self {
        Self::Empty {
            has_query: false,
            query: None,
        }
    }

    /// Creates an empty state with no results for a query.
    #[must_use]
    pub fn no_results(query: impl Into<String>) -> Self {
        Self::Empty {
            has_query: true,
            query: Some(query.into()),
        }
    }

    /// Creates a loading state.
    #[must_use]
    pub fn loading(message: impl Into<String>) -> Self {
        Self::Loading {
            message: message.into(),
            progress: None,
            progress_text: None,
        }
    }

    /// Creates a loading state with progress.
    #[must_use]
    pub fn loading_with_progress(
        message: impl Into<String>,
        progress: f32,
        progress_text: impl Into<String>,
    ) -> Self {
        Self::Loading {
            message: message.into(),
            progress: Some(progress.clamp(0.0, 1.0)),
            progress_text: Some(progress_text.into()),
        }
    }

    /// Creates a results state.
    #[must_use]
    pub fn results(items: Vec<SearchResult>) -> Self {
        Self::Results { items, selected: 0 }
    }

    /// Creates an error state.
    #[must_use]
    pub fn error(error: AppError) -> Self {
        let actions = error.default_actions();
        Self::Error { error, actions }
    }

    /// Creates an error state with custom actions.
    #[must_use]
    pub fn error_with_actions(error: AppError, actions: Vec<ErrorAction>) -> Self {
        Self::Error { error, actions }
    }

    /// Returns true if this is an empty state.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty { .. })
    }

    /// Returns true if this is a loading state.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }

    /// Returns true if this is a results state.
    #[must_use]
    pub const fn is_results(&self) -> bool {
        matches!(self, Self::Results { .. })
    }

    /// Returns true if this is an error state.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
}

// ============================================================================
// AppError - Application error type
// ============================================================================

/// An application error that can be displayed to the user.
#[derive(Debug, Clone)]
pub struct AppError {
    /// Error title (e.g., "Indexing failed").
    pub title: String,
    /// Error message with details.
    pub message: String,
    /// Error code for programmatic handling.
    pub code: ErrorCode,
}

impl AppError {
    /// Creates a new application error.
    #[must_use]
    pub fn new(title: impl Into<String>, message: impl Into<String>, code: ErrorCode) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            code,
        }
    }

    /// Creates an indexing error.
    #[must_use]
    pub fn indexing_failed(message: impl Into<String>) -> Self {
        Self::new("Indexing failed", message, ErrorCode::IndexingFailed)
    }

    /// Creates a permission error.
    #[must_use]
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new("Permission denied", message, ErrorCode::PermissionDenied)
    }

    /// Creates a search error.
    #[must_use]
    pub fn search_failed(message: impl Into<String>) -> Self {
        Self::new("Search failed", message, ErrorCode::SearchFailed)
    }

    /// Creates a database error.
    #[must_use]
    pub fn database_error(message: impl Into<String>) -> Self {
        Self::new("Database error", message, ErrorCode::DatabaseError)
    }

    /// Returns the default actions for this error.
    #[must_use]
    pub fn default_actions(&self) -> Vec<ErrorAction> {
        match self.code {
            ErrorCode::IndexingFailed => vec![
                ErrorAction::retry("Retry"),
                ErrorAction::open_folder("/Applications"),
            ],
            ErrorCode::PermissionDenied => vec![ErrorAction::open_settings()],
            ErrorCode::SearchFailed => vec![ErrorAction::retry("Retry")],
            ErrorCode::DatabaseError => vec![ErrorAction::retry("Retry")],
            ErrorCode::Unknown => vec![],
        }
    }

    /// Returns true if this error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self.code,
            ErrorCode::IndexingFailed | ErrorCode::SearchFailed
        )
    }
}

/// Error codes for programmatic handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Application indexing failed.
    IndexingFailed,
    /// Permission was denied.
    PermissionDenied,
    /// Search operation failed.
    SearchFailed,
    /// Database operation failed.
    DatabaseError,
    /// Unknown error.
    Unknown,
}

// ============================================================================
// ErrorAction - Actions for error recovery
// ============================================================================

/// An action that can be taken to recover from an error.
#[derive(Debug, Clone)]
pub struct ErrorAction {
    /// The action identifier.
    pub id: String,
    /// Display label for the action.
    pub label: String,
    /// The type of action.
    pub action_type: ErrorActionType,
    /// Whether this is the primary action.
    pub is_primary: bool,
}

impl ErrorAction {
    /// Creates a retry action.
    #[must_use]
    pub fn retry(label: impl Into<String>) -> Self {
        Self {
            id: "retry".to_string(),
            label: label.into(),
            action_type: ErrorActionType::Retry,
            is_primary: true,
        }
    }

    /// Creates an open folder action.
    #[must_use]
    pub fn open_folder(path: impl Into<String>) -> Self {
        Self {
            id: "open_folder".to_string(),
            label: "Open Folder".to_string(),
            action_type: ErrorActionType::OpenFolder { path: path.into() },
            is_primary: false,
        }
    }

    /// Creates an open system settings action.
    #[must_use]
    pub fn open_settings() -> Self {
        Self {
            id: "open_settings".to_string(),
            label: "Open System Settings".to_string(),
            action_type: ErrorActionType::OpenSettings,
            is_primary: true,
        }
    }

    /// Creates a dismiss action.
    #[must_use]
    pub fn dismiss(label: impl Into<String>) -> Self {
        Self {
            id: "dismiss".to_string(),
            label: label.into(),
            action_type: ErrorActionType::Dismiss,
            is_primary: false,
        }
    }
}

/// The type of error recovery action.
#[derive(Debug, Clone)]
pub enum ErrorActionType {
    /// Retry the failed operation.
    Retry,
    /// Open a folder in Finder.
    OpenFolder {
        /// Path to the folder.
        path: String,
    },
    /// Open system settings.
    OpenSettings,
    /// Dismiss the error.
    Dismiss,
}

// ============================================================================
// EmptyState - No query or no results component
// ============================================================================

/// Empty state component for when there are no results to display.
///
/// # Display Variants
///
/// - **No Query**: "Type to search apps, commands, and files"
/// - **No Results**: 'No results for "query"'
///
/// # Example
///
/// ```
/// use photoncast_core::ui::EmptyState;
///
/// // No query entered yet
/// let empty = EmptyState::no_query();
///
/// // Query entered but no results
/// let no_results = EmptyState::no_results("xyznonexistent");
/// ```
#[derive(Debug, Clone, Default)]
pub struct EmptyState {
    /// Whether the user has entered a query.
    pub has_query: bool,
    /// The query that returned no results (if `has_query` is true).
    pub query: Option<String>,
}

impl EmptyState {
    /// Creates a no-query empty state.
    #[must_use]
    pub const fn no_query() -> Self {
        Self {
            has_query: false,
            query: None,
        }
    }

    /// Creates a no-results empty state.
    #[must_use]
    pub fn no_results(query: impl Into<String>) -> Self {
        Self {
            has_query: true,
            query: Some(query.into()),
        }
    }

    /// Returns the primary message to display.
    #[must_use]
    pub fn message(&self) -> String {
        if self.has_query {
            if let Some(ref query) = self.query {
                format!("No results for \"{}\"", query)
            } else {
                "No results found".to_string()
            }
        } else {
            "Type to search apps, commands, and files".to_string()
        }
    }

    /// Returns the secondary message (hint) to display.
    #[must_use]
    pub fn hint(&self) -> Option<&'static str> {
        if self.has_query {
            Some("Try a different search term")
        } else {
            None
        }
    }

    /// Returns keyboard hints to display.
    #[must_use]
    pub fn keyboard_hints(&self) -> Vec<KeyboardHint> {
        if self.has_query {
            vec![]
        } else {
            vec![
                KeyboardHint::new("↑↓", "Navigate"),
                KeyboardHint::new("↵", "Open"),
                KeyboardHint::new("esc", "Close"),
            ]
        }
    }
}

// ============================================================================
// LoadingState - Indexing/loading component
// ============================================================================

/// Loading state component for displaying progress during indexing.
///
/// # Example
///
/// ```
/// use photoncast_core::ui::LoadingState;
///
/// // Simple loading
/// let loading = LoadingState::new("Indexing applications...");
///
/// // Loading with progress
/// let loading = LoadingState::with_progress(
///     "Indexing applications...",
///     0.71,
///     "Found 142 of ~200 apps",
/// );
/// ```
#[derive(Debug, Clone)]
pub struct LoadingState {
    /// The loading message to display.
    pub message: String,
    /// Optional progress as a fraction (0.0 to 1.0).
    pub progress: Option<f32>,
    /// Optional progress text.
    pub progress_text: Option<String>,
}

impl LoadingState {
    /// Creates a new loading state.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            progress: None,
            progress_text: None,
        }
    }

    /// Creates a loading state with progress.
    #[must_use]
    pub fn with_progress(
        message: impl Into<String>,
        progress: f32,
        progress_text: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            progress: Some(progress.clamp(0.0, 1.0)),
            progress_text: Some(progress_text.into()),
        }
    }

    /// Sets the progress value.
    #[must_use]
    pub fn set_progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Sets the progress text.
    #[must_use]
    pub fn set_progress_text(mut self, text: impl Into<String>) -> Self {
        self.progress_text = Some(text.into());
        self
    }

    /// Returns true if this loading state has progress information.
    #[must_use]
    pub const fn has_progress(&self) -> bool {
        self.progress.is_some()
    }

    /// Returns the progress percentage (0-100).
    #[must_use]
    pub fn progress_percentage(&self) -> Option<u8> {
        self.progress.map(|p| (p * 100.0) as u8)
    }

    /// Returns the loading spinner character for animation.
    ///
    /// The spinner cycles through: ◐ ◓ ◑ ◒
    #[must_use]
    pub const fn spinner_char(frame: usize) -> char {
        const SPINNER_CHARS: [char; 4] = ['◐', '◓', '◑', '◒'];
        SPINNER_CHARS[frame % 4]
    }
}

impl Default for LoadingState {
    fn default() -> Self {
        Self::new("Loading...")
    }
}

// ============================================================================
// ErrorState - Error display component
// ============================================================================

/// Error state component for displaying errors with recovery actions.
///
/// # Example
///
/// ```
/// use photoncast_core::ui::{ErrorState, AppError, ErrorAction};
///
/// // Create an error state
/// let error = AppError::indexing_failed("Unable to read /Applications");
/// let state = ErrorState::new(error);
///
/// // Create with custom actions
/// let error = AppError::permission_denied("Check folder permissions");
/// let state = ErrorState::with_actions(error, vec![
///     ErrorAction::retry("Try Again"),
///     ErrorAction::open_folder("/Applications"),
/// ]);
/// ```
#[derive(Debug, Clone)]
pub struct ErrorState {
    /// The error to display.
    pub error: AppError,
    /// Available recovery actions.
    pub actions: Vec<ErrorAction>,
    /// The currently focused action index.
    pub focused_action: usize,
}

impl ErrorState {
    /// Creates a new error state with default actions.
    #[must_use]
    pub fn new(error: AppError) -> Self {
        let actions = error.default_actions();
        Self {
            error,
            actions,
            focused_action: 0,
        }
    }

    /// Creates an error state with custom actions.
    #[must_use]
    pub fn with_actions(error: AppError, actions: Vec<ErrorAction>) -> Self {
        Self {
            error,
            actions,
            focused_action: 0,
        }
    }

    /// Returns the error icon name.
    #[must_use]
    pub const fn icon_name(&self) -> &'static str {
        match self.error.code {
            ErrorCode::PermissionDenied => "shield-alert",
            ErrorCode::IndexingFailed => "folder-x",
            ErrorCode::SearchFailed => "search-x",
            ErrorCode::DatabaseError => "database",
            ErrorCode::Unknown => "alert-triangle",
        }
    }

    /// Returns the primary action if one exists.
    #[must_use]
    pub fn primary_action(&self) -> Option<&ErrorAction> {
        self.actions.iter().find(|a| a.is_primary)
    }

    /// Returns the currently focused action.
    #[must_use]
    pub fn focused_action(&self) -> Option<&ErrorAction> {
        self.actions.get(self.focused_action)
    }

    /// Focuses the next action.
    pub fn focus_next(&mut self) {
        if !self.actions.is_empty() {
            self.focused_action = (self.focused_action + 1) % self.actions.len();
        }
    }

    /// Focuses the previous action.
    pub fn focus_previous(&mut self) {
        if !self.actions.is_empty() {
            self.focused_action = if self.focused_action == 0 {
                self.actions.len() - 1
            } else {
                self.focused_action - 1
            };
        }
    }
}

// ============================================================================
// KeyboardHint - Keyboard shortcut hint
// ============================================================================

/// A keyboard shortcut hint for display.
#[derive(Debug, Clone)]
pub struct KeyboardHint {
    /// The key(s) to press.
    pub keys: String,
    /// Description of what the key does.
    pub description: String,
}

impl KeyboardHint {
    /// Creates a new keyboard hint.
    #[must_use]
    pub fn new(keys: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            keys: keys.into(),
            description: description.into(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod launcher_state {
        use super::*;

        #[test]
        fn test_default_is_empty() {
            let state = LauncherState::default();
            assert!(state.is_empty());
            assert!(!state.is_loading());
            assert!(!state.is_results());
            assert!(!state.is_error());
        }

        #[test]
        fn test_empty_state() {
            let state = LauncherState::empty();
            assert!(state.is_empty());
            if let LauncherState::Empty { has_query, query } = state {
                assert!(!has_query);
                assert!(query.is_none());
            } else {
                panic!("Expected Empty state");
            }
        }

        #[test]
        fn test_no_results_state() {
            let state = LauncherState::no_results("test query");
            assert!(state.is_empty());
            if let LauncherState::Empty { has_query, query } = state {
                assert!(has_query);
                assert_eq!(query, Some("test query".to_string()));
            } else {
                panic!("Expected Empty state");
            }
        }

        #[test]
        fn test_loading_state() {
            let state = LauncherState::loading("Indexing...");
            assert!(state.is_loading());
            if let LauncherState::Loading {
                message,
                progress,
                progress_text,
            } = state
            {
                assert_eq!(message, "Indexing...");
                assert!(progress.is_none());
                assert!(progress_text.is_none());
            } else {
                panic!("Expected Loading state");
            }
        }

        #[test]
        fn test_loading_with_progress() {
            let state = LauncherState::loading_with_progress("Indexing...", 0.5, "50% complete");
            if let LauncherState::Loading {
                message,
                progress,
                progress_text,
            } = state
            {
                assert_eq!(message, "Indexing...");
                assert_eq!(progress, Some(0.5));
                assert_eq!(progress_text, Some("50% complete".to_string()));
            } else {
                panic!("Expected Loading state");
            }
        }

        #[test]
        fn test_progress_clamping() {
            let state = LauncherState::loading_with_progress("Test", 1.5, "Over 100%");
            if let LauncherState::Loading { progress, .. } = state {
                assert_eq!(progress, Some(1.0));
            } else {
                panic!("Expected Loading state");
            }

            let state = LauncherState::loading_with_progress("Test", -0.5, "Negative");
            if let LauncherState::Loading { progress, .. } = state {
                assert_eq!(progress, Some(0.0));
            } else {
                panic!("Expected Loading state");
            }
        }

        #[test]
        fn test_error_state() {
            let error = AppError::indexing_failed("Test error");
            let state = LauncherState::error(error);
            assert!(state.is_error());
        }
    }

    mod app_error {
        use super::*;

        #[test]
        fn test_new_error() {
            let error = AppError::new("Title", "Message", ErrorCode::Unknown);
            assert_eq!(error.title, "Title");
            assert_eq!(error.message, "Message");
            assert_eq!(error.code, ErrorCode::Unknown);
        }

        #[test]
        fn test_indexing_failed() {
            let error = AppError::indexing_failed("Cannot read directory");
            assert_eq!(error.title, "Indexing failed");
            assert_eq!(error.code, ErrorCode::IndexingFailed);
            assert!(error.is_recoverable());
        }

        #[test]
        fn test_permission_denied() {
            let error = AppError::permission_denied("Access denied");
            assert_eq!(error.title, "Permission denied");
            assert_eq!(error.code, ErrorCode::PermissionDenied);
            assert!(!error.is_recoverable());
        }

        #[test]
        fn test_default_actions() {
            let error = AppError::indexing_failed("Test");
            let actions = error.default_actions();
            assert_eq!(actions.len(), 2);
            assert!(actions[0].is_primary);
            assert_eq!(actions[0].id, "retry");
        }
    }

    mod error_action {
        use super::*;

        #[test]
        fn test_retry_action() {
            let action = ErrorAction::retry("Try Again");
            assert_eq!(action.id, "retry");
            assert_eq!(action.label, "Try Again");
            assert!(action.is_primary);
            assert!(matches!(action.action_type, ErrorActionType::Retry));
        }

        #[test]
        fn test_open_folder_action() {
            let action = ErrorAction::open_folder("/Applications");
            assert_eq!(action.id, "open_folder");
            assert!(!action.is_primary);
            if let ErrorActionType::OpenFolder { path } = action.action_type {
                assert_eq!(path, "/Applications");
            } else {
                panic!("Expected OpenFolder action type");
            }
        }

        #[test]
        fn test_open_settings_action() {
            let action = ErrorAction::open_settings();
            assert_eq!(action.id, "open_settings");
            assert!(action.is_primary);
            assert!(matches!(action.action_type, ErrorActionType::OpenSettings));
        }
    }

    mod empty_state {
        use super::*;

        #[test]
        fn test_no_query() {
            let state = EmptyState::no_query();
            assert!(!state.has_query);
            assert!(state.query.is_none());
            assert_eq!(state.message(), "Type to search apps, commands, and files");
            assert!(state.hint().is_none());
            assert!(!state.keyboard_hints().is_empty());
        }

        #[test]
        fn test_no_results() {
            let state = EmptyState::no_results("firefox");
            assert!(state.has_query);
            assert_eq!(state.query, Some("firefox".to_string()));
            assert_eq!(state.message(), "No results for \"firefox\"");
            assert_eq!(state.hint(), Some("Try a different search term"));
            assert!(state.keyboard_hints().is_empty());
        }
    }

    mod loading_state {
        use super::*;

        #[test]
        fn test_new() {
            let state = LoadingState::new("Loading...");
            assert_eq!(state.message, "Loading...");
            assert!(state.progress.is_none());
            assert!(state.progress_text.is_none());
            assert!(!state.has_progress());
        }

        #[test]
        fn test_with_progress() {
            let state = LoadingState::with_progress("Indexing...", 0.71, "Found 142 of ~200 apps");
            assert_eq!(state.message, "Indexing...");
            assert_eq!(state.progress, Some(0.71));
            assert_eq!(
                state.progress_text,
                Some("Found 142 of ~200 apps".to_string())
            );
            assert!(state.has_progress());
            assert_eq!(state.progress_percentage(), Some(71));
        }

        #[test]
        fn test_spinner_char() {
            assert_eq!(LoadingState::spinner_char(0), '◐');
            assert_eq!(LoadingState::spinner_char(1), '◓');
            assert_eq!(LoadingState::spinner_char(2), '◑');
            assert_eq!(LoadingState::spinner_char(3), '◒');
            assert_eq!(LoadingState::spinner_char(4), '◐'); // Wraps around
        }

        #[test]
        fn test_builder_methods() {
            let state = LoadingState::new("Test")
                .set_progress(0.5)
                .set_progress_text("Half done");
            assert_eq!(state.progress, Some(0.5));
            assert_eq!(state.progress_text, Some("Half done".to_string()));
        }
    }

    mod error_state {
        use super::*;

        #[test]
        fn test_new() {
            let error = AppError::indexing_failed("Test");
            let state = ErrorState::new(error);
            assert_eq!(state.error.title, "Indexing failed");
            assert!(!state.actions.is_empty());
            assert_eq!(state.focused_action, 0);
        }

        #[test]
        fn test_icon_name() {
            let state = ErrorState::new(AppError::indexing_failed("Test"));
            assert_eq!(state.icon_name(), "folder-x");

            let state = ErrorState::new(AppError::permission_denied("Test"));
            assert_eq!(state.icon_name(), "shield-alert");
        }

        #[test]
        fn test_focus_navigation() {
            let error = AppError::indexing_failed("Test");
            let mut state = ErrorState::new(error);
            assert_eq!(state.focused_action, 0);

            state.focus_next();
            assert_eq!(state.focused_action, 1);

            state.focus_next();
            assert_eq!(state.focused_action, 0); // Wraps around

            state.focus_previous();
            assert_eq!(state.focused_action, 1); // Wraps to end
        }

        #[test]
        fn test_primary_action() {
            let error = AppError::indexing_failed("Test");
            let state = ErrorState::new(error);
            let primary = state.primary_action();
            assert!(primary.is_some());
            assert!(primary.unwrap().is_primary);
        }
    }

    mod keyboard_hint {
        use super::*;

        #[test]
        fn test_new() {
            let hint = KeyboardHint::new("⌘K", "Quick search");
            assert_eq!(hint.keys, "⌘K");
            assert_eq!(hint.description, "Quick search");
        }
    }
}
