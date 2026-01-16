//! GPUI action definitions.
//!
//! Actions are used to communicate user intent throughout the application.

/// Select the next result in the list.
#[derive(Debug, Clone, Default)]
pub struct SelectNext;

/// Select the previous result in the list.
#[derive(Debug, Clone, Default)]
pub struct SelectPrevious;

/// Activate the currently selected result.
#[derive(Debug, Clone, Default)]
pub struct Activate;

/// Cancel the current operation / close the launcher.
#[derive(Debug, Clone, Default)]
pub struct Cancel;

/// Toggle the launcher window visibility.
#[derive(Debug, Clone, Default)]
pub struct ToggleLauncher;

/// Quick select action with index 1-9.
#[derive(Debug, Clone)]
pub struct QuickSelect {
    /// The index to select (1-9).
    pub index: u8,
}

impl QuickSelect {
    /// Creates a new quick select action.
    #[must_use]
    pub const fn new(index: u8) -> Self {
        Self { index }
    }
}

/// Open the preferences window.
#[derive(Debug, Clone, Default)]
pub struct OpenPreferences;

/// Cycle to the next result group.
#[derive(Debug, Clone, Default)]
pub struct NextGroup;

/// Cycle to the previous result group.
#[derive(Debug, Clone, Default)]
pub struct PreviousGroup;
