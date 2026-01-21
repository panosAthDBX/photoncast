use once_cell::sync::OnceCell;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
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
