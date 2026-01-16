//! Main launcher window component.

/// The main launcher window that contains the search bar and results list.
#[derive(Debug)]
pub struct LauncherWindow {
    /// Current search query.
    pub query: String,
    /// Whether the window is visible.
    pub visible: bool,
    /// Currently selected result index.
    pub selected_index: usize,
}

impl LauncherWindow {
    /// Creates a new launcher window.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            query: String::new(),
            visible: false,
            selected_index: 0,
        }
    }

    /// Toggles the visibility of the launcher window.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.query.clear();
            self.selected_index = 0;
        }
    }

    /// Shows the launcher window.
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
    }

    /// Hides the launcher window.
    pub fn hide(&mut self) {
        self.visible = false;
    }
}

impl Default for LauncherWindow {
    fn default() -> Self {
        Self::new()
    }
}
