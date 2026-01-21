//! Calendar data models.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// A calendar event from EventKit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Event ID.
    pub id: String,
    /// Event title.
    pub title: String,
    /// Start time.
    pub start: DateTime<Local>,
    /// End time.
    pub end: DateTime<Local>,
    /// Whether this is an all-day event.
    pub is_all_day: bool,
    /// Event location.
    pub location: Option<String>,
    /// Event notes.
    pub notes: Option<String>,
    /// Attendees.
    pub attendees: Vec<Attendee>,
    /// Conference URL (if detected).
    pub conference_url: Option<String>,
    /// Calendar color.
    pub calendar_color: String,
    /// Calendar name.
    pub calendar_name: String,
}

impl CalendarEvent {
    /// Creates a new calendar event.
    #[must_use]
    pub fn new(id: String, title: String, start: DateTime<Local>, end: DateTime<Local>) -> Self {
        Self {
            id,
            title,
            start,
            end,
            is_all_day: false,
            location: None,
            notes: None,
            attendees: Vec::new(),
            conference_url: None,
            calendar_color: "#0000FF".to_string(), // Default blue
            calendar_name: String::new(),
        }
    }

    /// Returns the duration of the event.
    #[must_use]
    pub fn duration(&self) -> chrono::Duration {
        self.end - self.start
    }

    /// Checks if the event is currently happening.
    #[must_use]
    pub fn is_happening_now(&self) -> bool {
        let now = Local::now();
        now >= self.start && now <= self.end
    }

    /// Checks if the event starts within the given minutes.
    #[must_use]
    pub fn starts_within_minutes(&self, minutes: i64) -> bool {
        let now = Local::now();
        let threshold = now + chrono::Duration::minutes(minutes);
        self.start > now && self.start <= threshold
    }

    /// Returns whether this event has a conference link.
    #[must_use]
    pub const fn has_conference_link(&self) -> bool {
        self.conference_url.is_some()
    }
}

/// A calendar event attendee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    /// Attendee name.
    pub name: String,
    /// Attendee email.
    pub email: Option<String>,
    /// Whether the attendee is the organizer.
    pub is_organizer: bool,
}

impl Attendee {
    /// Creates a new attendee.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self {
            name,
            email: None,
            is_organizer: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_event_creation() {
        let now = Local::now();
        let later = now + chrono::Duration::hours(1);

        let event = CalendarEvent::new("test-id".to_string(), "Test Event".to_string(), now, later);

        assert_eq!(event.id, "test-id");
        assert_eq!(event.title, "Test Event");
        assert_eq!(event.duration(), chrono::Duration::hours(1));
    }

    #[test]
    fn test_starts_within_minutes() {
        let now = Local::now();
        let in_ten_minutes = now + chrono::Duration::minutes(10);
        let in_twenty_minutes = now + chrono::Duration::minutes(20);

        let event = CalendarEvent::new(
            "test".to_string(),
            "Test".to_string(),
            in_ten_minutes,
            in_ten_minutes + chrono::Duration::hours(1),
        );

        assert!(event.starts_within_minutes(15));
        assert!(!event.starts_within_minutes(5));

        let later_event = CalendarEvent::new(
            "test2".to_string(),
            "Test2".to_string(),
            in_twenty_minutes,
            in_twenty_minutes + chrono::Duration::hours(1),
        );

        assert!(later_event.starts_within_minutes(30));
        assert!(!later_event.starts_within_minutes(15));
    }

    #[test]
    fn test_attendee_creation() {
        let attendee = Attendee::new("John Doe".to_string());
        assert_eq!(attendee.name, "John Doe");
        assert!(!attendee.is_organizer);
    }
}
