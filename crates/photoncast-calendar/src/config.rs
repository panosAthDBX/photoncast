use serde::{Deserialize, Serialize};

/// Calendar integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CalendarConfig {
    /// Whether calendar integration is enabled.
    pub enabled: bool,
    /// Number of days ahead to fetch events (default: 7).
    pub days_ahead: u32,
    /// Whether to show all-day events first in each day's list.
    pub show_all_day_first: bool,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            days_ahead: 7,
            show_all_day_first: true,
        }
    }
}
