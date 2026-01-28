//! EventKit integration for macOS calendar access.

use crate::conference::detect_conference_url;
use crate::error::{CalendarError, Result};
use crate::models::CalendarEvent;
use chrono::{DateTime, Local, TimeZone};
use std::fmt;

#[cfg(target_os = "macos")]
use block2::StackBlock;
#[cfg(target_os = "macos")]
use std::sync::mpsc;

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEvent, EKEventStore};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSDate, NSError};

/// EventKit permission status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    /// Permission not determined yet.
    NotDetermined,
    /// Permission denied.
    Denied,
    /// Permission authorized (full access).
    Authorized,
    /// Permission restricted (parental controls, etc.).
    Restricted,
    /// Write-only access (cannot read events).
    WriteOnly,
}

#[cfg(target_os = "macos")]
impl From<EKAuthorizationStatus> for PermissionStatus {
    fn from(status: EKAuthorizationStatus) -> Self {
        match status {
            EKAuthorizationStatus::Restricted => Self::Restricted,
            EKAuthorizationStatus::Denied => Self::Denied,
            EKAuthorizationStatus::FullAccess => Self::Authorized,
            EKAuthorizationStatus::WriteOnly => Self::WriteOnly,
            _ => Self::NotDetermined,
        }
    }
}

/// EventKit manager for calendar access.
pub struct EventKitManager {
    /// Whether permissions have been requested.
    permission_requested: bool,
    #[cfg(target_os = "macos")]
    /// The event store instance.
    event_store: Option<Retained<EKEventStore>>,
}

impl EventKitManager {
    /// Creates a new EventKit manager.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            permission_requested: false,
            #[cfg(target_os = "macos")]
            event_store: None,
        }
    }

    #[cfg(target_os = "macos")]
    fn get_or_create_store(&mut self) -> &Retained<EKEventStore> {
        if self.event_store.is_none() {
            let store = unsafe { EKEventStore::new() };
            self.event_store = Some(store);
        }
        self.event_store.as_ref().unwrap()
    }

    /// Checks the current permission status.
    #[must_use]
    pub fn check_permission(&self) -> PermissionStatus {
        #[cfg(target_os = "macos")]
        {
            let status =
                unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
            status.into()
        }

        #[cfg(not(target_os = "macos"))]
        PermissionStatus::NotDetermined
    }

    /// Requests calendar access permission from the user.
    ///
    /// Note: This is a simplified implementation. Full async permission handling
    /// requires running the main event loop. For now, we just check the status
    /// after requesting access - the user will need to grant permission manually
    /// in System Settings if the status is NotDetermined.
    #[cfg(target_os = "macos")]
    pub fn request_permission(&mut self) -> Result<PermissionStatus> {
        self.permission_requested = true;

        let current = self.check_permission();
        if current != PermissionStatus::NotDetermined {
            return Ok(current);
        }

        tracing::debug!("Requesting calendar permission...");

        let store = self.get_or_create_store();
        let (tx, rx) = mpsc::channel();
        let handler =
            StackBlock::new(move |granted: objc2::runtime::Bool, _error: *mut NSError| {
                let _ = tx.send(granted.as_bool());
            });
        #[allow(deprecated)]
        unsafe {
            store.requestAccessToEntityType_completion(
                EKEntityType::Event,
                std::ptr::addr_of!(*handler) as *mut _,
            );
        };

        if rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap_or(false)
        {
            return Ok(self.check_permission());
        }

        let status = self.check_permission();
        if status == PermissionStatus::NotDetermined {
            tracing::debug!("Opening System Settings for calendar permission...");
            let _ = std::process::Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars")
                .spawn();
        }

        Ok(status)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn request_permission(&mut self) -> Result<PermissionStatus> {
        self.permission_requested = true;
        Ok(PermissionStatus::NotDetermined)
    }

    /// Fetches events for the given date range.
    #[cfg(target_os = "macos")]
    pub fn fetch_events(
        &mut self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Vec<CalendarEvent>> {
        // Check permission first
        let permission = self.check_permission();
        tracing::debug!("Calendar permission status: {:?}", permission);
        if permission != PermissionStatus::Authorized {
            return Err(CalendarError::Message {
                message: format!("Calendar access not authorized: {:?}", permission),
            });
        }

        let store = self.get_or_create_store();

        // Convert chrono dates to NSDate
        #[allow(clippy::cast_precision_loss)]
        let start_timestamp = start.timestamp() as f64;
        #[allow(clippy::cast_precision_loss)]
        let end_timestamp = end.timestamp() as f64;

        let start_date = NSDate::dateWithTimeIntervalSince1970(start_timestamp);
        let end_date = NSDate::dateWithTimeIntervalSince1970(end_timestamp);

        // Create predicate for the date range
        let predicate = unsafe {
            store.predicateForEventsWithStartDate_endDate_calendars(&start_date, &end_date, None)
        };

        // Fetch events
        let events = unsafe { store.eventsMatchingPredicate(&predicate) };

        // Convert to CalendarEvent
        let mut result = Vec::new();
        let count = events.count();
        for i in 0..count {
            let event = events.objectAtIndex(i);
            if let Some(calendar_event) = Self::convert_event(&event) {
                result.push(calendar_event);
            }
        }

        // Sort by start time
        result.sort_by(|a, b| a.start.cmp(&b.start));

        tracing::debug!(
            "EventKit: Found {} events between {} and {}",
            result.len(),
            start.format("%Y-%m-%d %H:%M"),
            end.format("%Y-%m-%d %H:%M")
        );

        Ok(result)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn fetch_events(
        &mut self,
        _start: DateTime<Local>,
        _end: DateTime<Local>,
    ) -> Result<Vec<CalendarEvent>> {
        Err(CalendarError::Message {
            message: "Calendar integration only available on macOS".to_string(),
        })
    }

    /// Converts an EKEvent to a CalendarEvent.
    #[cfg(target_os = "macos")]
    #[allow(clippy::unnecessary_wraps)]
    fn convert_event(event: &EKEvent) -> Option<CalendarEvent> {
        // Get event ID
        let id = unsafe { event.eventIdentifier() }.map_or_else(
            || {
                format!(
                    "event-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                )
            },
            |s| s.to_string(),
        );

        // Get title
        let title = unsafe { event.title() }.to_string();

        // Get start and end dates
        let start_date = unsafe { event.startDate() };
        let end_date = unsafe { event.endDate() };

        let start_timestamp = start_date.timeIntervalSince1970();
        let end_timestamp = end_date.timeIntervalSince1970();

        #[allow(clippy::cast_possible_truncation)]
        let start = Local
            .timestamp_opt(start_timestamp.trunc() as i64, 0)
            .single()
            .unwrap_or_else(Local::now);
        #[allow(clippy::cast_possible_truncation)]
        let end = Local
            .timestamp_opt(end_timestamp.trunc() as i64, 0)
            .single()
            .unwrap_or_else(Local::now);

        // Get all-day status
        let is_all_day = unsafe { event.isAllDay() };

        // Get location
        let location = unsafe { event.location() }.map(|s| s.to_string());

        // Get notes
        let notes = unsafe { event.notes() }.map(|s| s.to_string());

        // Get URL
        let url = unsafe { event.URL() }
            .and_then(|u| u.absoluteString())
            .map(|s| s.to_string());

        // Detect conference URL from location and notes
        // Also check the URL field itself
        let conference_url =
            detect_conference_url(location.as_deref(), notes.as_deref()).or_else(|| {
                // Check if the URL itself is a conference URL
                url.as_deref()
                    .filter(|u| {
                        u.contains("zoom.us")
                            || u.contains("meet.google.com")
                            || u.contains("teams.microsoft.com")
                    })
                    .map(String::from)
            });

        // Get calendar info
        let (calendar_name, calendar_color) = unsafe { event.calendar() }.map_or_else(
            || ("Unknown".to_string(), "#888888".to_string()),
            |cal| {
                let name = unsafe { cal.title() }.to_string();
                // Calendar color is not easily accessible in objc2-event-kit
                // Use a default color
                (name, "#0088FF".to_string())
            },
        );

        Some(CalendarEvent {
            id,
            title,
            start,
            end,
            is_all_day,
            location,
            notes,
            attendees: Vec::new(), // Attendees require additional features
            conference_url,
            calendar_color,
            calendar_name,
        })
    }

    /// Fetches events for the next N days.
    pub fn fetch_upcoming_events(&mut self, days: i64) -> Result<Vec<CalendarEvent>> {
        if !self.permission_requested {
            let _ = self.request_permission();
        }

        let start = Local::now();
        let end = start + chrono::Duration::days(days);
        self.fetch_events(start, end)
    }

    fn start_of_day(now: DateTime<Local>) -> DateTime<Local> {
        now.date_naive().and_hms_opt(0, 0, 0).map_or(now, |dt| {
            match Local.from_local_datetime(&dt) {
                chrono::LocalResult::Single(value) => value,
                chrono::LocalResult::Ambiguous(first, _) => first,
                chrono::LocalResult::None => now,
            }
        })
    }

    /// Fetches events for today.
    pub fn fetch_today_events(&mut self) -> Result<Vec<CalendarEvent>> {
        if !self.permission_requested {
            let _ = self.request_permission();
        }

        let now = Local::now();
        let start = Self::start_of_day(now);
        let end = start + chrono::Duration::days(1);
        self.fetch_events(start, end)
    }

    /// Fetches events for this week.
    pub fn fetch_week_events(&mut self) -> Result<Vec<CalendarEvent>> {
        if !self.permission_requested {
            let _ = self.request_permission();
        }

        let now = Local::now();
        let start = Self::start_of_day(now);
        let end = start + chrono::Duration::weeks(1);
        self.fetch_events(start, end)
    }
}

impl Default for EventKitManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for EventKitManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventKitManager")
            .field("permission_requested", &self.permission_requested)
            .field("permission_status", &self.check_permission())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eventkit_manager_creation() {
        let manager = EventKitManager::new();
        assert!(!manager.permission_requested);
    }

    #[test]
    fn test_permission_status() {
        let manager = EventKitManager::new();
        let status = manager.check_permission();
        // Status depends on system state
        println!("Current permission status: {:?}", status);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_fetch_today_events_without_permission() {
        let mut manager = EventKitManager::new();
        // This may succeed if permission was previously granted, or fail if not
        let result = manager.fetch_today_events();
        println!("Fetch today events result: {:?}", result);
    }
}
