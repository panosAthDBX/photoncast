//! Window management commands for the launcher.

use crate::{DisplayDirection, WindowLayout, WindowManager};
use parking_lot::RwLock;
use std::rc::Rc;

/// Window management command state.
pub struct WindowCommand {
    /// The window manager instance.
    manager: Rc<RwLock<WindowManager>>,
}

impl WindowCommand {
    /// Creates a new window command.
    #[must_use]
    pub const fn new(manager: Rc<RwLock<WindowManager>>) -> Self {
        Self { manager }
    }

    /// Creates a new window command with default configuration.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self {
            manager: Rc::new(RwLock::new(WindowManager::default())),
        }
    }

    /// Checks if accessibility permissions are granted.
    #[cfg(target_os = "macos")]
    pub fn has_permission(&self) -> bool {
        let mut manager = self.manager.write();
        manager.has_accessibility_permission()
    }

    /// Requests accessibility permissions.
    #[cfg(target_os = "macos")]
    pub fn request_permission(&self) -> crate::Result<()> {
        let mut manager = self.manager.write();
        manager.request_accessibility_permission()
    }

    /// Gets the bundle ID of the frontmost application.
    #[cfg(target_os = "macos")]
    pub fn get_frontmost_bundle_id(&self) -> crate::Result<String> {
        let manager = self.manager.read();
        manager.get_frontmost_bundle_id()
    }

    /// Finds and activates the first visible app that isn't the given bundle ID.
    #[cfg(target_os = "macos")]
    pub fn activate_any_app_except(&self, except_bundle_id: &str) -> crate::Result<String> {
        let manager = self.manager.read();
        manager.activate_any_app_except(except_bundle_id)
    }

    /// Applies a window layout.
    #[cfg(target_os = "macos")]
    pub fn apply_layout(&self, layout: WindowLayout) -> crate::Result<()> {
        let mut manager = self.manager.write();
        manager.apply_layout(layout)
    }

    /// Moves the frontmost window to another display.
    #[cfg(target_os = "macos")]
    pub fn move_to_display(&self, direction: DisplayDirection) -> crate::Result<()> {
        let mut manager = self.manager.write();
        manager.move_to_display(direction)
    }

    /// Gets the number of connected displays.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn display_count(&self) -> usize {
        let manager = self.manager.read();
        manager.displays().len()
    }

    /// Refreshes the display list.
    #[cfg(target_os = "macos")]
    pub fn refresh_displays(&self) {
        let mut manager = self.manager.write();
        manager.refresh_displays();
    }
}

impl Default for WindowCommand {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// Information about a window management command.
#[derive(Debug, Clone)]
pub struct WindowCommandInfo {
    /// The layout or action.
    pub command_type: WindowCommandType,
    /// Display name.
    pub name: &'static str,
    /// Description.
    pub description: &'static str,
    /// Icon name.
    pub icon: &'static str,
    /// Command ID.
    pub id: &'static str,
}

/// Window management command type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCommandType {
    /// Apply a window layout.
    Layout(WindowLayout),
    /// Move to next display.
    MoveNextDisplay,
    /// Move to previous display.
    MovePreviousDisplay,
    /// Move to specific display.
    MoveToDisplay(usize),
}

impl WindowCommandInfo {
    /// Returns information about all available window commands.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let mut commands = vec![];

        // Layout commands
        for layout in WindowLayout::all() {
            commands.push(Self::for_layout(layout));
        }

        // Display movement commands
        commands.extend(vec![
            Self {
                command_type: WindowCommandType::MoveNextDisplay,
                name: "Move to Next Display",
                description: "Move window to the next display",
                icon: "monitor-arrow-right",
                id: "window_move_next_display",
            },
            Self {
                command_type: WindowCommandType::MovePreviousDisplay,
                name: "Move to Previous Display",
                description: "Move window to the previous display",
                icon: "monitor-arrow-left",
                id: "window_move_previous_display",
            },
            Self {
                command_type: WindowCommandType::MoveToDisplay(1),
                name: "Move to Display 1",
                description: "Move window to display 1",
                icon: "monitor",
                id: "window_move_display_1",
            },
            Self {
                command_type: WindowCommandType::MoveToDisplay(2),
                name: "Move to Display 2",
                description: "Move window to display 2",
                icon: "monitor",
                id: "window_move_display_2",
            },
            Self {
                command_type: WindowCommandType::MoveToDisplay(3),
                name: "Move to Display 3",
                description: "Move window to display 3",
                icon: "monitor",
                id: "window_move_display_3",
            },
        ]);

        commands
    }

    /// Creates command info for a layout.
    #[must_use]
    pub const fn for_layout(layout: WindowLayout) -> Self {
        let (icon, description) = match layout {
            WindowLayout::LeftHalf => (
                "layout-sidebar-left",
                "Position window on the left half of the screen",
            ),
            WindowLayout::RightHalf => (
                "layout-sidebar-right",
                "Position window on the right half of the screen",
            ),
            WindowLayout::TopHalf => (
                "layout-top",
                "Position window on the top half of the screen",
            ),
            WindowLayout::BottomHalf => (
                "layout-bottom",
                "Position window on the bottom half of the screen",
            ),
            WindowLayout::TopLeft => ("layout-top-left", "Position window in the top left quarter"),
            WindowLayout::TopRight => (
                "layout-top-right",
                "Position window in the top right quarter",
            ),
            WindowLayout::BottomLeft => (
                "layout-bottom-left",
                "Position window in the bottom left quarter",
            ),
            WindowLayout::BottomRight => (
                "layout-bottom-right",
                "Position window in the bottom right quarter",
            ),
            WindowLayout::FirstThird => (
                "layout-left",
                "Position window in the first third of the screen",
            ),
            WindowLayout::CenterThird => (
                "layout-center",
                "Position window in the center third of the screen",
            ),
            WindowLayout::LastThird => (
                "layout-right",
                "Position window in the last third of the screen",
            ),
            WindowLayout::FirstTwoThirds => (
                "layout-left-wide",
                "Position window in the first two thirds",
            ),
            WindowLayout::LastTwoThirds => (
                "layout-right-wide",
                "Position window in the last two thirds",
            ),
            WindowLayout::Maximize => ("maximize", "Maximize window to fill the screen"),
            WindowLayout::Center => ("layout-center", "Center window on the screen"),
            WindowLayout::Restore => ("restore", "Restore window to previous position"),
            WindowLayout::AlmostMaximize => ("maximize", "Almost maximize window with small margins"),
            WindowLayout::CenterHalf => ("layout-center", "Center window at 50% width, full height"),
            WindowLayout::CenterTwoThirds => ("layout-center", "Center window at 66% width, full height"),
            WindowLayout::ReasonableSize => ("layout-center", "Set window to reasonable size"),
            WindowLayout::MakeSmaller => ("minimize", "Make window smaller"),
            WindowLayout::MakeLarger => ("maximize", "Make window larger"),
            WindowLayout::ToggleFullscreen => ("fullscreen", "Toggle fullscreen mode"),
        };

        Self {
            command_type: WindowCommandType::Layout(layout),
            name: layout.name(),
            description,
            icon,
            id: layout.id(),
        }
    }
}

/// Suggested keyboard shortcuts for window layouts.
///
/// These are suggestions only - no default hotkeys are set to avoid conflicts.
/// Users can configure these in preferences.
#[derive(Debug, Clone)]
pub struct SuggestedShortcut {
    /// The command ID.
    pub command_id: &'static str,
    /// Suggested shortcut (using Hyper key: Cmd+Ctrl+Opt+Shift).
    pub shortcut: &'static str,
    /// Alternative shortcut without Hyper key.
    pub alt_shortcut: &'static str,
}

impl SuggestedShortcut {
    /// Returns suggested keyboard shortcuts for window management.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                command_id: "window_left_half",
                shortcut: "Hyper+Left",
                alt_shortcut: "Ctrl+Opt+Left",
            },
            Self {
                command_id: "window_right_half",
                shortcut: "Hyper+Right",
                alt_shortcut: "Ctrl+Opt+Right",
            },
            Self {
                command_id: "window_top_half",
                shortcut: "Hyper+Up",
                alt_shortcut: "Ctrl+Opt+Up",
            },
            Self {
                command_id: "window_bottom_half",
                shortcut: "Hyper+Down",
                alt_shortcut: "Ctrl+Opt+Down",
            },
            Self {
                command_id: "window_maximize",
                shortcut: "Hyper+Return",
                alt_shortcut: "Ctrl+Opt+Return",
            },
            Self {
                command_id: "window_center",
                shortcut: "Hyper+C",
                alt_shortcut: "Ctrl+Opt+C",
            },
            Self {
                command_id: "window_restore",
                shortcut: "Hyper+Backspace",
                alt_shortcut: "Ctrl+Opt+Backspace",
            },
            Self {
                command_id: "window_move_next_display",
                shortcut: "Hyper+N",
                alt_shortcut: "Ctrl+Opt+Cmd+Right",
            },
            Self {
                command_id: "window_move_previous_display",
                shortcut: "Hyper+P",
                alt_shortcut: "Ctrl+Opt+Cmd+Left",
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_command_creation() {
        let command = WindowCommand::with_default_config();
        // Should initialize successfully
        drop(command);
    }

    #[test]
    fn test_command_info_all() {
        let commands = WindowCommandInfo::all();

        // Should have layout commands + display movement commands
        assert!(!commands.is_empty());

        // Check that all layouts are included
        let layout_count = WindowLayout::all().len();
        assert!(commands.len() >= layout_count);
    }

    #[test]
    fn test_command_info_for_layout() {
        let info = WindowCommandInfo::for_layout(WindowLayout::LeftHalf);
        assert_eq!(info.name, "Left Half");
        assert_eq!(info.id, "window_left_half");
        assert!(!info.description.is_empty());
    }

    #[test]
    fn test_suggested_shortcuts() {
        let shortcuts = SuggestedShortcut::all();
        assert!(!shortcuts.is_empty());

        // All should have both regular and hyper shortcuts
        for shortcut in shortcuts {
            assert!(!shortcut.shortcut.is_empty());
            assert!(!shortcut.alt_shortcut.is_empty());
        }
    }
}
