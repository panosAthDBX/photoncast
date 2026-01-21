//! Calendar commands for the launcher.

use crate::models::CalendarEvent;
use crate::{CalendarManager, Result};
use parking_lot::RwLock;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(target_os = "macos")]
use crate::eventkit::EventKitManager;

/// Calendar command state.
pub struct CalendarCommand {
    /// The calendar manager instance.
    #[allow(dead_code)]
    manager: Arc<RwLock<CalendarManager>>,

    /// EventKit manager for macOS.
    #[cfg(target_os = "macos")]
    eventkit: Rc<RwLock<EventKitManager>>,
}

impl CalendarCommand {
    /// Creates a new calendar command.
    #[must_use]
    pub fn new(manager: Arc<RwLock<CalendarManager>>) -> Self {
        Self {
            manager,
            #[cfg(target_os = "macos")]
            eventkit: Rc::new(RwLock::new(EventKitManager::new())),
        }
    }

    /// Creates a new calendar command with default configuration.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(Arc::new(RwLock::new(CalendarManager::default())))
    }

    /// Checks if calendar permissions are granted.
    #[cfg(target_os = "macos")]
    pub fn has_permission(&self) -> bool {
        let eventkit = self.eventkit.read();
        eventkit.check_permission() == crate::eventkit::PermissionStatus::Authorized
    }

    /// Checks if calendar permissions are granted (non-macOS stub).
    #[cfg(not(target_os = "macos"))]
    pub fn has_permission(&self) -> bool {
        false
    }

    /// Requests calendar permissions.
    #[cfg(target_os = "macos")]
    pub fn request_permission(&self) -> Result<()> {
        self.eventkit.write().request_permission()?;
        Ok(())
    }

    /// Requests calendar permissions (non-macOS stub).
    #[cfg(not(target_os = "macos"))]
    pub fn request_permission(&self) -> Result<()> {
        Err(crate::CalendarError::Message {
            message: "Calendar integration is only available on macOS".to_string(),
        })
    }

    /// Fetches today's events.
    #[cfg(target_os = "macos")]
    pub fn fetch_today_events(&self) -> Result<Vec<CalendarEvent>> {
        let mut eventkit = self.eventkit.write();
        eventkit.fetch_today_events()
    }

    /// Fetches today's events (non-macOS stub).
    #[cfg(not(target_os = "macos"))]
    pub fn fetch_today_events(&self) -> Result<Vec<CalendarEvent>> {
        Err(crate::CalendarError::Message {
            message: "Calendar integration is only available on macOS".to_string(),
        })
    }

    /// Fetches this week's events.
    #[cfg(target_os = "macos")]
    pub fn fetch_week_events(&self) -> Result<Vec<CalendarEvent>> {
        let mut eventkit = self.eventkit.write();
        eventkit.fetch_week_events()
    }

    /// Fetches this week's events (non-macOS stub).
    #[cfg(not(target_os = "macos"))]
    pub fn fetch_week_events(&self) -> Result<Vec<CalendarEvent>> {
        Err(crate::CalendarError::Message {
            message: "Calendar integration is only available on macOS".to_string(),
        })
    }

    /// Fetches upcoming events for the next N days.
    #[cfg(target_os = "macos")]
    pub fn fetch_upcoming_events(&self, days: i64) -> Result<Vec<CalendarEvent>> {
        let mut eventkit = self.eventkit.write();
        eventkit.fetch_upcoming_events(days)
    }

    /// Fetches upcoming events for the next N days (non-macOS stub).
    #[cfg(not(target_os = "macos"))]
    pub fn fetch_upcoming_events(&self, _days: i64) -> Result<Vec<CalendarEvent>> {
        Err(crate::CalendarError::Message {
            message: "Calendar integration is only available on macOS".to_string(),
        })
    }
}

impl Default for CalendarCommand {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// Information about a calendar command.
#[derive(Debug, Clone)]
pub struct CalendarCommandInfo {
    /// Command type.
    pub command_type: CalendarCommandType,
    /// Display name.
    pub name: &'static str,
    /// Description.
    pub description: &'static str,
    /// Icon name.
    pub icon: &'static str,
    /// Command ID.
    pub id: &'static str,
}

/// Calendar command type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarCommandType {
    /// Show today's events.
    Today,
    /// Show this week's events.
    Week,
    /// Show upcoming events (7 days).
    Upcoming,
}

impl CalendarCommandInfo {
    /// Returns information about all available calendar commands.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                command_type: CalendarCommandType::Today,
                name: "Today's Events",
                description: "View events for today",
                icon: "calendar-today",
                id: "calendar_today",
            },
            Self {
                command_type: CalendarCommandType::Week,
                name: "This Week",
                description: "View events for this week",
                icon: "calendar-week",
                id: "calendar_week",
            },
            Self {
                command_type: CalendarCommandType::Upcoming,
                name: "My Schedule",
                description: "View upcoming events (7 days)",
                icon: "calendar",
                id: "calendar_upcoming",
            },
        ]
    }
}

/// Calendar event actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarAction {
    /// Join the conference meeting.
    JoinMeeting,
    /// Open event in Calendar.app.
    OpenInCalendar,
    /// Copy event details to clipboard.
    CopyDetails,
}

impl CalendarAction {
    /// Returns all available actions for an event.
    #[must_use]
    pub fn all_for_event(event: &CalendarEvent) -> Vec<Self> {
        let mut actions = Vec::new();

        // Join Meeting is primary action if conference link exists
        if event.has_conference_link() {
            actions.push(Self::JoinMeeting);
        }

        actions.push(Self::OpenInCalendar);
        actions.push(Self::CopyDetails);

        actions
    }

    /// Returns the action name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::JoinMeeting => "Join Meeting",
            Self::OpenInCalendar => "Open in Calendar",
            Self::CopyDetails => "Copy Details",
        }
    }

    /// Returns the action shortcut.
    #[must_use]
    pub const fn shortcut(self) -> &'static str {
        match self {
            Self::JoinMeeting => "Enter",
            Self::OpenInCalendar => "Cmd+O",
            Self::CopyDetails => "Cmd+C",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_command_creation() {
        let command = CalendarCommand::with_default_config();
        // Should initialize successfully
        drop(command);
    }

    #[test]
    fn test_command_info_all() {
        let commands = CalendarCommandInfo::all();
        assert_eq!(commands.len(), 3);

        // Check that all command types are included
        assert!(commands
            .iter()
            .any(|c| c.command_type == CalendarCommandType::Today));
        assert!(commands
            .iter()
            .any(|c| c.command_type == CalendarCommandType::Week));
        assert!(commands
            .iter()
            .any(|c| c.command_type == CalendarCommandType::Upcoming));
    }

    #[test]
    fn test_calendar_action_for_event() {
        let event = crate::models::CalendarEvent {
            id: "test".to_string(),
            title: "Test Event".to_string(),
            start: chrono::Local::now(),
            end: chrono::Local::now() + chrono::Duration::hours(1),
            is_all_day: false,
            location: None,
            notes: None,
            attendees: Vec::new(),
            conference_url: Some("https://zoom.us/j/123456".to_string()),
            calendar_color: "#0000FF".to_string(),
            calendar_name: "Work".to_string(),
        };

        let actions = CalendarAction::all_for_event(&event);

        // Should have Join Meeting as first action when conference link exists
        assert_eq!(actions[0], CalendarAction::JoinMeeting);
        assert!(actions.contains(&CalendarAction::OpenInCalendar));
        assert!(actions.contains(&CalendarAction::CopyDetails));
    }

    #[test]
    fn test_calendar_action_without_conference() {
        let event = crate::models::CalendarEvent {
            id: "test".to_string(),
            title: "Test Event".to_string(),
            start: chrono::Local::now(),
            end: chrono::Local::now() + chrono::Duration::hours(1),
            is_all_day: false,
            location: None,
            notes: None,
            attendees: Vec::new(),
            conference_url: None,
            calendar_color: "#0000FF".to_string(),
            calendar_name: "Work".to_string(),
        };

        let actions = CalendarAction::all_for_event(&event);

        // Should NOT have Join Meeting when no conference link
        assert!(!actions.contains(&CalendarAction::JoinMeeting));
        assert!(actions.contains(&CalendarAction::OpenInCalendar));
        assert!(actions.contains(&CalendarAction::CopyDetails));
    }
}
