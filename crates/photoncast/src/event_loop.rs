//! Application event loop handler.
//!
//! Extracts the event-dispatching logic from `main()` into a dedicated struct,
//! keeping each [`AppEvent`] handler in its own method for readability.

use std::sync::Arc;

use gpui::*;
use parking_lot::RwLock;
use tracing::{error, info, warn};

use crate::app_events::AppEvent;
use crate::launcher::{LauncherSharedState, LauncherWindow};
use crate::preferences_window::PreferencesWindow;
use crate::{
    open_argument_input_window, open_clipboard_window, open_create_quicklink_window,
    open_launcher_window, open_manage_quicklinks_window, open_preferences_window,
    open_quicklinks_window, open_timer_window, ClipboardState,
};
use photoncast_clipboard::ui::ClipboardHistoryView;
use photoncast_quicklinks::ui::{CreateQuicklinkView, QuicklinksManageView};

/// Holds all mutable window handles and shared resources used by the event loop.
pub(crate) struct EventLoopState {
    pub current_handle: Option<WindowHandle<LauncherWindow>>,
    pub clipboard_handle: Option<WindowHandle<ClipboardHistoryView>>,
    pub quicklinks_handle: Option<WindowHandle<photoncast_quicklinks::ui::QuickLinksView>>,
    pub timer_handle: Option<WindowHandle<photoncast_timer::ui::TimerDisplay>>,
    pub preferences_handle: Option<WindowHandle<PreferencesWindow>>,
    pub create_quicklink_handle: Option<WindowHandle<CreateQuicklinkView>>,
    pub manage_quicklinks_handle: Option<WindowHandle<QuicklinksManageView>>,

    pub launcher_state: LauncherSharedState,
    pub clipboard_state: Option<Arc<RwLock<ClipboardState>>>,
    pub quicklinks_storage: photoncast_quicklinks::QuickLinksStorage,
    pub shared_rt: Arc<tokio::runtime::Runtime>,
    pub app_manager: photoncast_apps::AppManager,
    pub calendar_command: photoncast_calendar::CalendarCommand,
}

impl EventLoopState {
    fn foreground_launcher(cx: &mut ViewContext<LauncherWindow>, branch: &'static str) {
        if let Err(err) = crate::platform::activate_ignoring_other_apps() {
            tracing::error!(
                "Failed to foreground PhotonCast for {} launcher window: {}",
                branch,
                err
            );
        }
        cx.activate(true);
        cx.activate_window();
        cx.focus_self();
    }

    /// Dispatch a single [`AppEvent`] to the appropriate handler.
    ///
    /// Returns `false` when the event loop should stop (quit / channel closed).
    pub fn handle_event(&mut self, event: AppEvent, cx: &mut AsyncAppContext) -> bool {
        match event {
            AppEvent::ToggleLauncher => self.handle_toggle_launcher(cx),
            AppEvent::OpenPreferences => self.handle_open_preferences(cx),
            AppEvent::OpenClipboardHistory => self.handle_open_clipboard_history(cx),
            AppEvent::OpenQuickLinks => self.handle_open_quick_links(cx),
            AppEvent::OpenCalendar { command_id } => {
                self.handle_open_calendar(&command_id, cx);
            },
            AppEvent::OpenSleepTimer { expression } => {
                self.handle_open_sleep_timer(&expression, cx);
            },
            AppEvent::OpenApps { command_id } => self.handle_open_apps(&command_id),
            AppEvent::CreateQuicklink => self.handle_create_quicklink(cx),
            AppEvent::ManageQuicklinks => self.handle_manage_quicklinks(cx),
            AppEvent::BrowseQuicklinkLibrary => self.handle_browse_quicklink_library(cx),
            AppEvent::ExecuteQuickLink {
                id,
                url_template,
                arguments,
            } => self.handle_execute_quicklink(&id, &url_template, &arguments, cx),
            AppEvent::QuitApp => {
                info!("Quit requested from menu bar");
                let _ = cx.update(|cx| {
                    cx.quit();
                });
                return false;
            },
            AppEvent::TimerExpired { action } => {
                info!("Timer expired, action executed: {}", action);
            },
            AppEvent::ExecuteWindowCommand {
                command_id,
                target_bundle_id,
                target_window_title,
            } => {
                info!("Window command requested: {}", command_id);
                crate::execute_window_command(&command_id, target_bundle_id, target_window_title);
            },
        }
        true
    }

    // ------------------------------------------------------------------
    // Individual event handlers
    // ------------------------------------------------------------------

    fn handle_toggle_launcher(&mut self, cx: &mut AsyncAppContext) {
        tracing::debug!("Toggle launcher requested - capturing frontmost window NOW");

        let (previous_app, previous_window_title) = crate::get_frontmost_window_info();
        tracing::debug!(
            "Captured frontmost: app={:?}, window={:?}",
            previous_app,
            previous_window_title
        );

        let _ = cx.update(|cx| {
            let window_exists = self.current_handle.as_ref().is_some_and(|h| {
                h.update(cx, |view, cx| {
                    view.set_previous_frontmost_window(
                        previous_app.clone(),
                        previous_window_title.clone(),
                    );
                    view.toggle(cx);
                    if view.is_visible() {
                        Self::foreground_launcher(cx, "existing");
                    }
                })
                .is_ok()
            });

            if !window_exists {
                self.current_handle = open_launcher_window(cx, &self.launcher_state);
                if let Some(ref h) = self.current_handle {
                    let _ = h.update(cx, |view, cx| {
                        view.set_previous_frontmost_window(
                            previous_app.clone(),
                            previous_window_title.clone(),
                        );
                        Self::foreground_launcher(cx, "new");
                    });
                }
            }
        });
    }

    fn handle_open_preferences(&mut self, cx: &mut AsyncAppContext) {
        info!("Open preferences requested");
        let _ = cx.update(|cx| {
            if let Some(ref h) = self.preferences_handle {
                if h.update(cx, |_, cx| {
                    cx.activate(true);
                    cx.activate_window();
                })
                .is_err()
                {
                    self.preferences_handle = None;
                }
            }

            if self.preferences_handle.is_none() {
                let app = self.launcher_state.photoncast_app();
                if let Some(handle) = open_preferences_window(cx, Some(app)) {
                    let _ = handle.update(cx, |_, cx| {
                        cx.activate(true);
                        cx.activate_window();
                    });
                    self.preferences_handle = Some(handle);
                }
            }
        });
    }

    fn handle_open_clipboard_history(&mut self, cx: &mut AsyncAppContext) {
        info!("Open clipboard history requested");
        let mut activated = false;
        if let Some(ref h) = self.clipboard_handle {
            if h.update(cx, |view, cx| {
                view.refresh(cx);
                cx.activate(true);
                cx.activate_window();
                cx.focus_self();
            })
            .is_ok()
            {
                activated = true;
            } else {
                self.clipboard_handle = None;
            }
        }

        if !activated {
            if let Some(ref clipboard_state) = self.clipboard_state {
                let state = clipboard_state.clone();
                let _ = cx.update(|cx| {
                    if let Some(handle) = open_clipboard_window(cx, &state) {
                        let _ = handle.update(cx, |view, cx| {
                            view.refresh(cx);
                            cx.activate(true);
                            cx.activate_window();
                            cx.focus_self();
                        });
                        self.clipboard_handle = Some(handle);
                    }
                });
            } else {
                warn!("Clipboard not available");
            }
        }
    }

    fn handle_open_quick_links(&mut self, cx: &mut AsyncAppContext) {
        info!("Open quick links requested");
        let mut activated = false;
        if let Some(ref h) = self.quicklinks_handle {
            if h.update(cx, |_view, cx| {
                cx.activate(true);
                cx.activate_window();
            })
            .is_ok()
            {
                activated = true;
            } else {
                self.quicklinks_handle = None;
            }
        }

        if !activated {
            let _ = cx.update(|cx| {
                if let Some(handle) = open_quicklinks_window(cx) {
                    let _ = handle.update(cx, |_view, cx| {
                        cx.activate(true);
                        cx.activate_window();
                    });
                    self.quicklinks_handle = Some(handle);
                }
            });
        }

        // Load quicklinks asynchronously on a background thread, then update the view
        if let Some(handle) = self.quicklinks_handle {
            let storage = self.quicklinks_storage.clone();
            cx.spawn(|mut cx| async move {
                let links = cx
                    .background_executor()
                    .spawn(async move { storage.load_all_sync().unwrap_or_default() })
                    .await;
                let _ = handle.update(&mut cx, |view, cx| {
                    view.set_links(links, cx);
                });
            })
            .detach();
        }
    }

    fn handle_open_calendar(&mut self, command_id: &str, cx: &mut AsyncAppContext) {
        info!("Open calendar requested: {}", command_id);
        let (title, result) = match command_id {
            "calendar_today" => ("Today's Events", self.calendar_command.fetch_today_events()),
            "calendar_week" => ("This Week", self.calendar_command.fetch_week_events()),
            "calendar_upcoming" => (
                "My Schedule",
                self.calendar_command.fetch_upcoming_events(7),
            ),
            other => {
                warn!("Unknown calendar command: {}", other);
                (
                    "My Schedule",
                    self.calendar_command.fetch_upcoming_events(7),
                )
            },
        };

        match result {
            Ok(events) => {
                info!("Calendar fetch returned {} events", events.len());
                let title = title.to_string();
                let events = events.clone();
                let _ = cx.update(|cx| {
                    if let Some(ref h) = self.current_handle {
                        let _ = h.update(cx, |view, cx| {
                            view.show_calendar(title, events, cx);
                            cx.activate(true);
                            cx.activate_window();
                            cx.focus_self();
                        });
                    } else {
                        self.current_handle = open_launcher_window(cx, &self.launcher_state);
                    }
                });
            },
            Err(err) => {
                warn!("Calendar fetch failed: {}", err);
                let title = title.to_string();
                let error = err.to_string();
                let _ = cx.update(|cx| {
                    if let Some(ref h) = self.current_handle {
                        let _ = h.update(cx, |view, cx| {
                            view.show_calendar_error(title, error, cx);
                            cx.activate(true);
                            cx.activate_window();
                            cx.focus_self();
                        });
                    } else {
                        self.current_handle = open_launcher_window(cx, &self.launcher_state);
                    }
                });
            },
        }
    }

    fn handle_open_sleep_timer(&mut self, expression: &str, cx: &mut AsyncAppContext) {
        info!("Open sleep timer requested: {}", expression);
        let mut activated = false;
        if let Some(ref h) = self.timer_handle {
            if h.update(cx, |view, cx| {
                view.set_timer(None, cx);
                cx.activate(true);
                cx.activate_window();
            })
            .is_ok()
            {
                activated = true;
            } else {
                self.timer_handle = None;
            }
        }

        if !activated {
            let _ = cx.update(|cx| {
                if let Some(handle) = open_timer_window(cx) {
                    let _ = handle.update(cx, |view, cx| {
                        view.set_timer(None, cx);
                        cx.activate(true);
                        cx.activate_window();
                    });
                    self.timer_handle = Some(handle);
                }
            });
        }
    }

    fn handle_open_apps(&self, command_id: &str) {
        info!("Open apps management requested: {}", command_id);
        if let Err(e) = self.app_manager.get_running_apps() {
            warn!("Apps management failed: {}", e);
        }
    }

    fn handle_create_quicklink(&mut self, cx: &mut AsyncAppContext) {
        info!("Create quicklink requested");
        let _ = cx.update(|cx| {
            if let Some(ref h) = self.create_quicklink_handle {
                if h.update(
                    cx,
                    |_: &mut CreateQuicklinkView, cx: &mut ViewContext<CreateQuicklinkView>| {
                        cx.activate(true);
                        cx.activate_window();
                    },
                )
                .is_err()
                {
                    self.create_quicklink_handle = None;
                }
            }

            if self.create_quicklink_handle.is_none() {
                if let Some(handle) = open_create_quicklink_window(
                    cx,
                    self.quicklinks_storage.clone(),
                    self.shared_rt.handle().clone(),
                    &self.launcher_state,
                ) {
                    let _ = handle.update(
                        cx,
                        |_: &mut CreateQuicklinkView, cx: &mut ViewContext<CreateQuicklinkView>| {
                            cx.activate(true);
                            cx.activate_window();
                        },
                    );
                    self.create_quicklink_handle = Some(handle);
                }
            }
        });
    }

    fn handle_manage_quicklinks(&mut self, cx: &mut AsyncAppContext) {
        info!("Manage quicklinks requested");
        let _ = cx.update(|cx| {
            if let Some(ref h) = self.manage_quicklinks_handle {
                if h.update(cx, |_view, cx| {
                    cx.activate(true);
                    cx.activate_window();
                })
                .is_err()
                {
                    self.manage_quicklinks_handle = None;
                }
            }

            if self.manage_quicklinks_handle.is_none() {
                if let Some(handle) = open_manage_quicklinks_window(
                    cx,
                    self.quicklinks_storage.clone(),
                    &self.shared_rt,
                    false,
                    &self.launcher_state,
                ) {
                    let _ = handle.update(cx, |_, cx| {
                        cx.activate(true);
                        cx.activate_window();
                    });
                    self.manage_quicklinks_handle = Some(handle);
                }
            }
        });

        // Load quicklinks asynchronously on a background thread, then update the view
        if let Some(handle) = self.manage_quicklinks_handle {
            let storage = self.quicklinks_storage.clone();
            cx.spawn(|mut cx| async move {
                let links = cx
                    .background_executor()
                    .spawn(async move { storage.load_all_sync().unwrap_or_default() })
                    .await;
                let _ = handle.update(&mut cx, |view, cx| {
                    view.set_quicklinks(links, cx);
                });
            })
            .detach();
        }
    }

    fn handle_browse_quicklink_library(&mut self, cx: &mut AsyncAppContext) {
        info!("Browse quicklink library requested");
        let _ = cx.update(|cx| {
            if let Some(ref h) = self.manage_quicklinks_handle {
                if h.update(cx, |view, cx| {
                    if !view.is_showing_library() {
                        view.toggle_library(cx);
                    }
                    cx.activate(true);
                    cx.activate_window();
                })
                .is_err()
                {
                    self.manage_quicklinks_handle = None;
                }
            }

            if self.manage_quicklinks_handle.is_none() {
                if let Some(handle) = open_manage_quicklinks_window(
                    cx,
                    self.quicklinks_storage.clone(),
                    &self.shared_rt,
                    true,
                    &self.launcher_state,
                ) {
                    let _ = handle.update(cx, |_, cx| {
                        cx.activate(true);
                        cx.activate_window();
                    });
                    self.manage_quicklinks_handle = Some(handle);
                }
            }
        });

        // Load quicklinks asynchronously on a background thread, then update the view
        if let Some(handle) = self.manage_quicklinks_handle {
            let storage = self.quicklinks_storage.clone();
            cx.spawn(|mut cx| async move {
                let links = cx
                    .background_executor()
                    .spawn(async move { storage.load_all_sync().unwrap_or_default() })
                    .await;
                let _ = handle.update(&mut cx, |view, cx| {
                    view.set_quicklinks(links, cx);
                });
            })
            .detach();
        }
    }

    fn handle_execute_quicklink(
        &self,
        id: &str,
        url_template: &str,
        arguments: &str,
        cx: &mut AsyncAppContext,
    ) {
        let final_url = if !arguments.is_empty() {
            photoncast_quicklinks::placeholder::substitute_argument(url_template, arguments)
        } else {
            url_template.to_string()
        };

        if photoncast_quicklinks::placeholder::requires_user_input(&final_url) {
            // Load quicklinks asynchronously to find the one we need
            let storage = self.quicklinks_storage.clone();
            let id = id.to_string();
            cx.spawn(|cx| async move {
                let links = cx
                    .background_executor()
                    .spawn(async move { storage.load_all_sync() })
                    .await;
                if let Ok(links) = links {
                    if let Some(link) = links.into_iter().find(|l| l.id.as_str() == id) {
                        let _ = cx.update(|cx| {
                            if let Some(handle) = open_argument_input_window(cx, link) {
                                let _ = handle.update(cx, |_, cx| {
                                    cx.activate(true);
                                    cx.activate_window();
                                });
                            }
                        });
                    }
                }
            })
            .detach();
        } else {
            info!("Execute quicklink: {}", final_url);
            if let Err(e) = photoncast_core::platform::launch::open_url(&final_url) {
                error!("Failed to open quicklink URL: {}", e);
            }
        }
    }
}
