//! Animation methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Starts the window appear animation.
    pub(super) fn start_appear_animation(&mut self, cx: &mut ViewContext<Self>) {
        let duration = window_appear_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.animation.window_state = WindowAnimationState::Visible;
            self.animation.start = None;
        } else {
            self.animation.window_state = WindowAnimationState::Appearing;
            self.animation.start = Some(Instant::now());
            // Schedule a refresh to drive the animation
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16); // ~60 FPS
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if this.animation.window_state == WindowAnimationState::Appearing {
                                if let Some(start) = this.animation.start {
                                    let elapsed = start.elapsed();
                                    let total = window_appear_duration();
                                    if elapsed >= total {
                                        this.animation.window_state = WindowAnimationState::Visible;
                                        this.animation.start = None;
                                        cx.notify();
                                        return false; // Animation complete
                                    }
                                    cx.notify();
                                    return true; // Continue animation
                                }
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }

        // Start cursor blink timer
        self.search.cursor_blink_epoch = Instant::now();
        cx.spawn(|this, mut cx| async move {
            let blink_interval = Duration::from_millis(530);
            loop {
                gpui::Timer::after(blink_interval).await;
                let should_continue = this
                    .update(&mut cx, |this, cx| {
                        if this.visible {
                            cx.notify(); // Trigger redraw for cursor blink
                            true
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);
                if !should_continue {
                    break;
                }
            }
        })
        .detach();

        cx.notify();
    }

    /// Starts the window dismiss animation.
    pub(super) fn start_dismiss_animation(&mut self, cx: &mut ViewContext<Self>) {
        let duration = window_dismiss_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.animation.window_state = WindowAnimationState::Hidden;
            self.animation.start = None;
            // Close window but keep app running (for hotkey re-activation)
            let () = cx.remove_window();
        } else {
            self.animation.window_state = WindowAnimationState::Dismissing;
            self.animation.start = Some(Instant::now());
            // Schedule a refresh to drive the animation
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16); // ~60 FPS
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if this.animation.window_state == WindowAnimationState::Dismissing {
                                if let Some(start) = this.animation.start {
                                    let elapsed = start.elapsed();
                                    let total = window_dismiss_duration();
                                    if elapsed >= total {
                                        this.animation.window_state = WindowAnimationState::Hidden;
                                        this.animation.start = None;
                                        // Close window but keep app running
                                        let () = cx.remove_window();
                                        return false; // Animation complete
                                    }
                                    cx.notify();
                                    return true; // Continue animation
                                }
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
        cx.notify();
    }

    /// Calculates the current animation progress (0.0 to 1.0).
    pub(super) fn animation_progress(&self) -> f32 {
        match (self.animation.window_state, self.animation.start) {
            (WindowAnimationState::Appearing, Some(start)) => {
                let elapsed = start.elapsed();
                let total = window_appear_duration();
                if total.is_zero() {
                    1.0
                } else {
                    (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0)
                }
            },
            (WindowAnimationState::Dismissing, Some(start)) => {
                let elapsed = start.elapsed();
                let total = window_dismiss_duration();
                if total.is_zero() {
                    1.0
                } else {
                    (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0)
                }
            },
            (WindowAnimationState::Hidden, _) => 0.0,
            _ => 1.0,
        }
    }

    /// Calculates the current opacity based on animation state.
    pub(super) fn current_opacity(&self) -> f32 {
        match self.animation.window_state {
            WindowAnimationState::Appearing => {
                let progress = ease_out(self.animation_progress());
                lerp(
                    WINDOW_APPEAR_OPACITY_START,
                    WINDOW_APPEAR_OPACITY_END,
                    progress,
                )
            },
            WindowAnimationState::Dismissing => {
                let progress = ease_in(self.animation_progress());
                lerp(
                    WINDOW_APPEAR_OPACITY_END,
                    WINDOW_APPEAR_OPACITY_START,
                    progress,
                )
            },
            WindowAnimationState::Visible => 1.0,
            WindowAnimationState::Hidden => 0.0,
        }
    }

    /// Calculates the current scale based on animation state.
    #[allow(dead_code)]
    pub(super) fn current_scale(&self) -> f32 {
        match self.animation.window_state {
            WindowAnimationState::Appearing => {
                let progress = ease_out(self.animation_progress());
                lerp(WINDOW_APPEAR_SCALE_START, WINDOW_APPEAR_SCALE_END, progress)
            },
            WindowAnimationState::Dismissing => {
                let progress = ease_in(self.animation_progress());
                lerp(WINDOW_APPEAR_SCALE_END, WINDOW_DISMISS_SCALE_END, progress)
            },
            WindowAnimationState::Visible => 1.0,
            WindowAnimationState::Hidden => WINDOW_APPEAR_SCALE_START,
        }
    }

    /// Starts the selection change animation.
    pub(super) fn start_selection_animation(&mut self, previous_index: usize, cx: &mut ViewContext<Self>) {
        self.animation.previous_selected_index = Some(previous_index);
        let duration = selection_change_duration();
        if duration.is_zero() {
            // Reduce motion: skip animation
            self.animation.selection_start = None;
            self.animation.previous_selected_index = None;
        } else {
            self.animation.selection_start = Some(Instant::now());
            // Schedule animation updates
            cx.spawn(|this, mut cx| async move {
                let frame_duration = Duration::from_millis(16);
                loop {
                    gpui::Timer::after(frame_duration).await;
                    let should_continue = this
                        .update(&mut cx, |this, cx| {
                            if let Some(start) = this.animation.selection_start {
                                let elapsed = start.elapsed();
                                let total = selection_change_duration();
                                if elapsed >= total {
                                    this.animation.selection_start = None;
                                    this.animation.previous_selected_index = None;
                                    cx.notify();
                                    return false;
                                }
                                cx.notify();
                                return true;
                            }
                            false
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        break;
                    }
                }
            })
            .detach();
        }
    }

    /// Calculates the selection animation progress (0.0 to 1.0).
    #[allow(dead_code)]
    pub(super) fn selection_animation_progress(&self) -> f32 {
        if let Some(start) = self.animation.selection_start {
            let elapsed = start.elapsed();
            let total = selection_change_duration();
            if total.is_zero() {
                1.0
            } else {
                ease_in_out((elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0))
            }
        } else {
            1.0
        }
    }
}
