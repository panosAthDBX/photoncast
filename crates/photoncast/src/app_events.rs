use once_cell::sync::OnceCell;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppEvent {
    ToggleLauncher,
    OpenPreferences,
    OpenClipboardHistory,
    OpenQuickLinks,
    ExecuteQuickLink {
        id: String,
        url_template: String,
        arguments: String,
    },
    OpenCalendar { command_id: String },
    OpenSleepTimer { expression: String },
    OpenApps { command_id: String },
    QuitApp,
    // Quicklink management events
    CreateQuicklink,
    ManageQuicklinks,
    BrowseQuicklinkLibrary,
    // Timer events (from background polling thread)
    TimerExpired { action: String },
    // Window management events (executed outside GPUI context to avoid reentrancy)
    ExecuteWindowCommand { 
        command_id: String,
        /// The bundle ID of the app that was frontmost before Photoncast opened
        target_bundle_id: Option<String>,
        /// The title of the window that was frontmost before Photoncast opened
        target_window_title: Option<String>,
    },
}

static EVENT_SENDER: OnceCell<Sender<AppEvent>> = OnceCell::new();

pub fn set_event_sender(sender: Sender<AppEvent>) {
    let _ = EVENT_SENDER.set(sender);
}

pub fn send_event(event: AppEvent) -> Result<(), String> {
    EVENT_SENDER
        .get()
        .ok_or_else(|| "App event sender not initialized".to_string())?
        .send(event)
        .map_err(|e| e.to_string())
}
