//! Navigation system for extension views.
//!
//! Provides a navigation stack for managing view history with:
//! - Push/pop/replace operations with animations
//! - Keyboard shortcuts (Escape, Cmd+[) for navigation
//! - Per-extension state preservation
//! - Thread-safe view updates via ViewHandle

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use abi_stable::std_types::{ROption, RString, RVec};
use gpui::prelude::FluentBuilder;
use gpui::*;
use parking_lot::Mutex;
use photoncast_extension_api::{ExtensionView, ListItem, ViewHandle, ViewHandleTrait};

use super::colors::ExtensionViewColors;
use super::dimensions::*;
use super::{render_extension_view, ActionCallback, CLOSE_VIEW_ACTION};

// ============================================================================
// Actions
// ============================================================================

actions!(
    extension_navigation,
    [NavigateBack, NavigateBackAlt, NavigateToRoot]
);

/// Registers key bindings for extension view navigation.
pub fn register_key_bindings(cx: &mut gpui::AppContext) {
    cx.bind_keys([
        KeyBinding::new("escape", NavigateBack, Some("NavigationContainer")),
        KeyBinding::new("cmd-[", NavigateBackAlt, Some("NavigationContainer")),
        KeyBinding::new("cmd-shift-[", NavigateToRoot, Some("NavigationContainer")),
    ]);
}

// ============================================================================
// Navigation Trait
// ============================================================================

/// Navigation operations for extension views.
pub trait Navigation {
    /// Pushes a new view onto the navigation stack with slide-right animation.
    fn push(&self, view: ExtensionView);

    /// Pops the current view and returns to the previous view with slide-left animation.
    fn pop(&self);

    /// Replaces the current view without animation (or with crossfade).
    fn replace(&self, view: ExtensionView);

    /// Pops all views except the root view.
    fn pop_to_root(&self);

    /// Returns the current stack depth.
    fn depth(&self) -> usize;

    /// Returns true if we can pop (not at root).
    fn can_pop(&self) -> bool;
}

// ============================================================================
// Animation Direction
// ============================================================================

/// Animation direction for view transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    /// Slide from right (push).
    SlideRight,
    /// Slide from left (pop).
    SlideLeft,
    /// Crossfade (replace).
    Crossfade,
    /// No animation.
    None,
}

// ============================================================================
// Navigation Entry
// ============================================================================

/// A single entry in the navigation stack.
pub(crate) struct NavigationEntry {
    /// The extension view data.
    view: ExtensionView,
    /// The rendered GPUI view.
    rendered: AnyView,
    /// Unique identifier for this entry.
    id: u64,
}

// ============================================================================
// NavigationStack
// ============================================================================

/// Manages the navigation history for extension views.
pub struct NavigationStack {
    /// The view stack.
    stack: Vec<NavigationEntry>,
    /// Next entry ID.
    next_id: u64,
    /// Extension ID this stack belongs to.
    extension_id: String,
}

impl NavigationStack {
    /// Creates a new empty navigation stack.
    pub fn new(extension_id: impl Into<String>) -> Self {
        Self {
            stack: Vec::new(),
            next_id: 0,
            extension_id: extension_id.into(),
        }
    }

    /// Returns the extension ID this stack belongs to.
    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    /// Returns the current stack depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Returns true if the stack has more than one entry.
    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }

    /// Returns the current (topmost) view.
    pub fn current(&self) -> Option<&AnyView> {
        self.stack.last().map(|e| &e.rendered)
    }

    /// Returns the current view data.
    pub fn current_view(&self) -> Option<&ExtensionView> {
        self.stack.last().map(|e| &e.view)
    }

    /// Pushes a new view onto the stack.
    pub fn push(
        &mut self,
        view: ExtensionView,
        action_callback: Option<ActionCallback>,
        cx: &mut WindowContext,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let rendered = render_extension_view(view.clone(), action_callback, cx);
        self.stack.push(NavigationEntry { view, rendered, id });
        id
    }

    /// Pops the topmost view from the stack.
    pub fn pop(&mut self) -> Option<NavigationEntry> {
        if self.stack.len() > 1 {
            self.stack.pop()
        } else {
            None
        }
    }

    /// Replaces the current view.
    pub fn replace(
        &mut self,
        view: ExtensionView,
        action_callback: Option<ActionCallback>,
        cx: &mut WindowContext,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let rendered = render_extension_view(view.clone(), action_callback, cx);

        if self.stack.is_empty() {
            self.stack.push(NavigationEntry { view, rendered, id });
        } else {
            let len = self.stack.len();
            self.stack[len - 1] = NavigationEntry { view, rendered, id };
        }
        id
    }

    /// Pops all views except the root.
    pub fn pop_to_root(&mut self) {
        if self.stack.len() > 1 {
            self.stack.truncate(1);
        }
    }

    /// Clears all views from the stack.
    pub fn clear(&mut self) {
        self.stack.clear();
    }
}

// ============================================================================
// NavigationContainer
// ============================================================================

/// Container view that manages navigation for extension views.
pub struct NavigationContainer {
    /// The navigation stack.
    stack: NavigationStack,
    /// Current animation state.
    animation: Option<NavigationAnimation>,
    /// Focus handle.
    focus_handle: FocusHandle,
    /// Action callback for child views.
    action_callback: Option<ActionCallback>,
    /// Loading state.
    loading: bool,
    /// Error message.
    error: Option<String>,
    /// Sender for external updates (shared with handles).
    update_sender: Arc<Mutex<Sender<ViewUpdate>>>,
    /// Receiver for external updates.
    update_receiver: mpsc::Receiver<ViewUpdate>,
    /// Whether the container is still valid.
    valid: Arc<AtomicBool>,
    /// Current view generation (for stale handle detection).
    generation: Arc<AtomicU64>,
}

/// Animation state during view transitions.
struct NavigationAnimation {
    /// Animation direction.
    direction: AnimationDirection,
    /// Animation progress (0.0 to 1.0).
    progress: f32,
    /// Previous view (during transition).
    previous_view: Option<AnyView>,
    /// Animation start time.
    start_time: std::time::Instant,
}

/// Types of view updates that can be sent via ViewHandle.
pub(crate) enum ViewUpdate {
    /// Replace entire view.
    ReplaceView(ExtensionView),
    /// Update list items only.
    UpdateItems(RVec<ListItem>),
    /// Set loading state.
    SetLoading(bool),
    /// Set error state.
    SetError(Option<String>),
    /// Push a new view onto the stack.
    PushView(ExtensionView),
    /// Pop the current view.
    PopView,
    /// Pop to root view.
    PopToRoot,
}

impl NavigationContainer {
    /// Creates a new navigation container with an initial view.
    pub fn new(
        initial_view: ExtensionView,
        extension_id: impl Into<String>,
        action_callback: Option<ActionCallback>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        cx.focus(&focus_handle);

        let mut stack = NavigationStack::new(extension_id);
        stack.push(initial_view, action_callback.clone(), cx);

        // Create channel for external updates
        let (tx, rx) = mpsc::channel();

        let container = Self {
            stack,
            animation: None,
            focus_handle,
            action_callback,
            loading: false,
            error: None,
            update_sender: Arc::new(Mutex::new(tx)),
            update_receiver: rx,
            valid: Arc::new(AtomicBool::new(true)),
            generation: Arc::new(AtomicU64::new(0)),
        };

        // Start polling for external updates
        container.start_update_polling(cx);

        container
    }

    /// Creates a ViewHandle for external async updates.
    pub fn create_handle(&self, cx: &ViewContext<Self>) -> ViewHandle {
        let handle = NavigationViewHandle::new(
            cx.view().downgrade(),
            self.valid.clone(),
            self.generation.load(Ordering::SeqCst),
            self.update_sender.clone(),
        );
        ViewHandle::new(handle)
    }

    /// Starts polling for external updates from ViewHandle.
    fn start_update_polling(&self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            const POLL_INTERVAL_MS: u64 = 16; // ~60 FPS

            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(POLL_INTERVAL_MS))
                    .await;

                let should_continue = this
                    .update(&mut cx, |container, cx| {
                        container.process_pending_updates(cx);
                        container.valid.load(Ordering::SeqCst)
                    })
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }
            }
        })
        .detach();
    }

    /// Processes any pending updates from the channel.
    fn process_pending_updates(&mut self, cx: &mut ViewContext<Self>) {
        let mut had_updates = false;

        // Process all pending updates
        loop {
            match self.update_receiver.try_recv() {
                Ok(update) => {
                    had_updates = true;
                    match update {
                        ViewUpdate::ReplaceView(view) => {
                            self.replace_view(view, cx);
                        },
                        ViewUpdate::UpdateItems(items) => {
                            self.update_items(items, cx);
                        },
                        ViewUpdate::SetLoading(loading) => {
                            self.set_loading(loading, cx);
                        },
                        ViewUpdate::SetError(error) => {
                            self.set_error(error, cx);
                        },
                        ViewUpdate::PushView(view) => {
                            self.push_view(view, cx);
                        },
                        ViewUpdate::PopView => {
                            self.pop_view(cx);
                        },
                        ViewUpdate::PopToRoot => {
                            self.pop_to_root(cx);
                        },
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        if had_updates {
            cx.notify();
        }
    }

    /// Pushes a new view with animation.
    pub fn push_view(&mut self, view: ExtensionView, cx: &mut ViewContext<Self>) {
        let previous = self.stack.current().cloned();

        // Start animation
        self.animation = Some(NavigationAnimation {
            direction: AnimationDirection::SlideRight,
            progress: 0.0,
            previous_view: previous,
            start_time: std::time::Instant::now(),
        });

        // Push the new view
        self.stack.push(view, self.action_callback.clone(), cx);
        self.generation.fetch_add(1, Ordering::SeqCst);

        // Start animation timer
        self.start_animation_timer(cx);
        cx.notify();
    }

    /// Pops the current view with animation.
    pub fn pop_view(&mut self, cx: &mut ViewContext<Self>) {
        if !self.stack.can_pop() {
            // Can't pop - trigger cancel callback instead
            if let Some(callback) = &self.action_callback {
                callback(CLOSE_VIEW_ACTION, cx);
            }
            return;
        }

        let previous = self.stack.current().cloned();

        // Start animation
        self.animation = Some(NavigationAnimation {
            direction: AnimationDirection::SlideLeft,
            progress: 0.0,
            previous_view: previous,
            start_time: std::time::Instant::now(),
        });

        // Pop the view
        self.stack.pop();
        self.generation.fetch_add(1, Ordering::SeqCst);

        // Start animation timer
        self.start_animation_timer(cx);
        cx.notify();
    }

    /// Replaces the current view (no animation or crossfade).
    pub fn replace_view(&mut self, view: ExtensionView, cx: &mut ViewContext<Self>) {
        self.stack.replace(view, self.action_callback.clone(), cx);
        self.generation.fetch_add(1, Ordering::SeqCst);
        cx.notify();
    }

    /// Pops to the root view.
    pub fn pop_to_root(&mut self, cx: &mut ViewContext<Self>) {
        if self.stack.depth() > 1 {
            self.stack.pop_to_root();
            self.generation.fetch_add(1, Ordering::SeqCst);
            cx.notify();
        }
    }

    /// Sets the loading state.
    pub fn set_loading(&mut self, loading: bool, cx: &mut ViewContext<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Sets the error state.
    pub fn set_error(&mut self, error: Option<String>, cx: &mut ViewContext<Self>) {
        self.error = error;
        cx.notify();
    }

    /// Updates list items in the current view if it's a ListView.
    pub fn update_items(&mut self, items: RVec<ListItem>, cx: &mut ViewContext<Self>) {
        if let Some(view) = self.stack.current() {
            super::update_view_items(view, items, cx);
        }
    }

    /// Starts the animation timer.
    fn start_animation_timer(&self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            const ANIMATION_DURATION_MS: u64 = 200;
            const FRAME_INTERVAL_MS: u64 = 16;

            let start = std::time::Instant::now();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(FRAME_INTERVAL_MS))
                    .await;

                let elapsed = start.elapsed().as_millis() as f32;
                let progress = (elapsed / ANIMATION_DURATION_MS as f32).min(1.0);

                let done = this
                    .update(&mut cx, |this, cx| {
                        if let Some(ref mut anim) = this.animation {
                            anim.progress = ease_out_cubic(progress);
                        }

                        if progress >= 1.0 {
                            this.animation = None;
                            true
                        } else {
                            cx.notify();
                            false
                        }
                    })
                    .unwrap_or(true);

                if done {
                    let _ = this.update(&mut cx, |_, cx| cx.notify());
                    break;
                }
            }
        })
        .detach();
    }

    // ========================================================================
    // Action Handlers
    // ========================================================================

    fn navigate_back(&mut self, _: &NavigateBack, cx: &mut ViewContext<Self>) {
        self.pop_view(cx);
    }

    fn navigate_back_alt(&mut self, _: &NavigateBackAlt, cx: &mut ViewContext<Self>) {
        self.pop_view(cx);
    }

    fn navigate_to_root(&mut self, _: &NavigateToRoot, cx: &mut ViewContext<Self>) {
        self.pop_to_root(cx);
    }

    // ========================================================================
    // Rendering
    // ========================================================================

    /// Renders the navigation header with back button.
    fn render_nav_header(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        if self.stack.depth() <= 1 {
            return div().flex_shrink_0();
        }

        div()
            .w_full()
            .h(px(36.0))
            .px(PADDING)
            .flex()
            .items_center()
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .cursor_pointer()
                    .text_sm()
                    .text_color(colors.accent)
                    .hover(|el| el.text_color(colors.accent_hover))
                    .child("←")
                    .child("Back"),
            )
    }

    /// Renders the loading overlay.
    fn render_loading(&self, colors: &ExtensionViewColors) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.5))
            .child(
                div()
                    .p(px(24.0))
                    .rounded(BORDER_RADIUS)
                    .bg(colors.surface_elevated)
                    .shadow_lg()
                    .child(div().text_color(colors.text).child("Loading...")),
            )
    }

    /// Renders the error state.
    fn render_error(&self, error: &str, colors: &ExtensionViewColors) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.5))
            .child(
                div()
                    .p(px(24.0))
                    .rounded(BORDER_RADIUS)
                    .bg(colors.surface_elevated)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.0))
                    .child(div().text_2xl().child("⚠️"))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(colors.error)
                            .child("Error"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.text_muted)
                            .max_w(px(300.0))
                            .flex()
                            .justify_center()
                            .child(error.to_string()),
                    )
                    .child(
                        div()
                            .mt(px(8.0))
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(BORDER_RADIUS)
                            .bg(colors.accent)
                            .text_color(gpui::white())
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .cursor_pointer()
                            .child("Retry"),
                    ),
            )
    }
}

impl Navigation for NavigationContainer {
    fn push(&self, _view: ExtensionView) {
        // This is called from outside - need to use a channel for thread safety
        // For now, this is a placeholder
    }

    fn pop(&self) {
        // This is called from outside - need to use a channel for thread safety
    }

    fn replace(&self, _view: ExtensionView) {
        // This is called from outside - need to use a channel for thread safety
    }

    fn pop_to_root(&self) {
        // This is called from outside - need to use a channel for thread safety
    }

    fn depth(&self) -> usize {
        self.stack.depth()
    }

    fn can_pop(&self) -> bool {
        self.stack.can_pop()
    }
}

impl FocusableView for NavigationContainer {
    fn focus_handle(&self, _cx: &gpui::AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NavigationContainer {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = ExtensionViewColors::from_context(cx);
        let has_nav = self.stack.depth() > 1;

        div()
            .key_context("NavigationContainer")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::navigate_back))
            .on_action(cx.listener(Self::navigate_back_alt))
            .on_action(cx.listener(Self::navigate_to_root))
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            // Navigation header
            .when(has_nav, |el| el.child(self.render_nav_header(&colors)))
            // Current view with animation
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .relative()
                    .when_some(self.stack.current().cloned(), |el, view| {
                        if let Some(ref anim) = self.animation {
                            let offset = match anim.direction {
                                AnimationDirection::SlideRight => {
                                    // New view slides in from right
                                    px(VIEW_WIDTH.0 * (1.0 - anim.progress))
                                },
                                AnimationDirection::SlideLeft => {
                                    // View slides out to right
                                    px(VIEW_WIDTH.0 * anim.progress)
                                },
                                _ => px(0.0),
                            };

                            el.child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .left(offset)
                                    .child(view),
                            )
                        } else {
                            el.child(view)
                        }
                    }),
            )
            // Loading overlay
            .when(self.loading, |el| el.child(self.render_loading(&colors)))
            // Error overlay
            .when_some(self.error.clone(), |el, error| {
                el.child(self.render_error(&error, &colors))
            })
    }
}

impl Drop for NavigationContainer {
    fn drop(&mut self) {
        // Mark the container as invalid to prevent stale handles from updating
        self.valid.store(false, Ordering::SeqCst);
    }
}

// ============================================================================
// ViewHandle Implementation
// ============================================================================

/// Thread-safe handle for updating navigation views.
struct NavigationViewHandle {
    /// Flag to check if handle is still valid.
    valid: Arc<AtomicBool>,
    /// Generation when this handle was created.
    generation: u64,
    /// Sender for view updates.
    sender: Arc<Mutex<Sender<ViewUpdate>>>,
}

impl NavigationViewHandle {
    fn new(
        _view: WeakView<NavigationContainer>,
        valid: Arc<AtomicBool>,
        generation: u64,
        sender: Arc<Mutex<Sender<ViewUpdate>>>,
    ) -> Self {
        Self {
            valid,
            generation,
            sender,
        }
    }

    /// Checks if this handle is still valid.
    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }

    /// Sends an update through the channel.
    fn send_update(&self, update: ViewUpdate) {
        if !self.is_valid() {
            return;
        }
        let sender = self.sender.lock();
        let _ = sender.send(update);
    }
}

impl ViewHandleTrait for NavigationViewHandle {
    fn update(&self, view: ExtensionView) {
        self.send_update(ViewUpdate::ReplaceView(view));
    }

    fn update_items(&self, items: RVec<ListItem>) {
        self.send_update(ViewUpdate::UpdateItems(items));
    }

    fn set_loading(&self, loading: bool) {
        self.send_update(ViewUpdate::SetLoading(loading));
    }

    fn set_error(&self, error: ROption<RString>) {
        let error_str = error.into_option().map(|s| s.to_string());
        self.send_update(ViewUpdate::SetError(error_str));
    }
}

// SAFETY: NavigationViewHandle contains only an Arc<AtomicBool> (Send+Sync),
// a u64 (Send+Sync), and an Arc<Mutex<Sender<ViewUpdate>>> (Send+Sync).
// It communicates with the NavigationContainer exclusively via message passing
// through the mpsc channel. The handle itself does not access any non-thread-safe
// state directly.
unsafe impl Send for NavigationViewHandle {}
unsafe impl Sync for NavigationViewHandle {}

// ============================================================================
// Animation Utilities
// ============================================================================

/// Cubic ease-out animation curve.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

// ============================================================================
// Navigation Controller
// ============================================================================

/// Controller for managing navigation from action handlers.
///
/// This provides a way for action handlers to trigger navigation
/// operations without direct access to the ViewContext.
#[derive(Clone)]
pub struct NavigationController {
    /// Sender for view updates.
    sender: Arc<Mutex<Sender<ViewUpdate>>>,
    /// Flag to check if container is still valid.
    valid: Arc<AtomicBool>,
}

impl NavigationController {
    /// Creates a new navigation controller.
    pub fn new(sender: Arc<Mutex<Sender<ViewUpdate>>>, valid: Arc<AtomicBool>) -> Self {
        Self { sender, valid }
    }

    /// Creates a navigation controller from a NavigationContainer.
    pub fn from_container(container: &NavigationContainer) -> Self {
        Self {
            sender: container.update_sender.clone(),
            valid: container.valid.clone(),
        }
    }

    /// Sends an update through the channel.
    fn send_update(&self, update: ViewUpdate) {
        if !self.valid.load(Ordering::SeqCst) {
            return;
        }
        let sender = self.sender.lock();
        let _ = sender.send(update);
    }

    /// Pushes a view onto the navigation stack.
    pub fn push(&self, view: ExtensionView) {
        self.send_update(ViewUpdate::PushView(view));
    }

    /// Pops the current view.
    pub fn pop(&self) {
        self.send_update(ViewUpdate::PopView);
    }

    /// Replaces the current view.
    pub fn replace(&self, view: ExtensionView) {
        self.send_update(ViewUpdate::ReplaceView(view));
    }

    /// Pops to the root view.
    pub fn pop_to_root(&self) {
        self.send_update(ViewUpdate::PopToRoot);
    }
}

#[cfg(test)]
mod tests {
    use super::ease_out_cubic;

    #[test]
    fn test_ease_out_cubic() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < 0.001);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 0.001);
        // Midpoint should be > 0.5 (curve accelerates early)
        assert!(ease_out_cubic(0.5) > 0.5);
    }
}
