//! Command definitions and registry.
//!
//! This module contains the definitions for all built-in system commands.

/// A system command that can be executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemCommand {
    /// Put the Mac to sleep.
    Sleep,
    /// Sleep only the displays.
    SleepDisplays,
    /// Lock the screen.
    LockScreen,
    /// Restart the Mac.
    Restart,
    /// Shut down the Mac.
    ShutDown,
    /// Log out the current user.
    LogOut,
    /// Empty the Trash.
    EmptyTrash,
    /// Start the screen saver.
    ScreenSaver,
    /// Toggle dark mode.
    ToggleAppearance,
}

/// Information about a system command.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    /// The command.
    pub command: SystemCommand,
    /// Display name.
    pub name: &'static str,
    /// Search aliases.
    pub aliases: &'static [&'static str],
    /// Description.
    pub description: &'static str,
    /// Icon name.
    pub icon: &'static str,
    /// Whether this command requires confirmation.
    pub requires_confirmation: bool,
}

impl SystemCommand {
    /// Returns information about all available system commands.
    #[must_use]
    pub fn all() -> Vec<CommandInfo> {
        vec![
            CommandInfo {
                command: Self::Sleep,
                name: "Sleep",
                aliases: &["sleep", "suspend"],
                description: "Put Mac to sleep",
                icon: "moon",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::SleepDisplays,
                name: "Sleep Displays",
                aliases: &["sleep displays", "display sleep"],
                description: "Turn off displays",
                icon: "monitor",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::LockScreen,
                name: "Lock Screen",
                aliases: &["lock"],
                description: "Lock your Mac",
                icon: "lock",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::Restart,
                name: "Restart",
                aliases: &["restart", "reboot"],
                description: "Restart your Mac",
                icon: "rotate-ccw",
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::ShutDown,
                name: "Shut Down",
                aliases: &["shutdown", "power off"],
                description: "Shut down your Mac",
                icon: "power",
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::LogOut,
                name: "Log Out",
                aliases: &["logout", "sign out"],
                description: "Log out current user",
                icon: "log-out",
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::EmptyTrash,
                name: "Empty Trash",
                aliases: &["empty trash", "clear trash"],
                description: "Empty the Trash",
                icon: "trash",
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::ScreenSaver,
                name: "Screen Saver",
                aliases: &["screensaver"],
                description: "Start screen saver",
                icon: "monitor",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::ToggleAppearance,
                name: "Toggle Appearance",
                aliases: &["dark mode", "light mode", "toggle dark"],
                description: "Switch between light and dark mode",
                icon: "sun-moon",
                requires_confirmation: false,
            },
        ]
    }

    /// Returns the command ID as a string.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Sleep => "sleep",
            Self::SleepDisplays => "sleep_displays",
            Self::LockScreen => "lock_screen",
            Self::Restart => "restart",
            Self::ShutDown => "shut_down",
            Self::LogOut => "log_out",
            Self::EmptyTrash => "empty_trash",
            Self::ScreenSaver => "screen_saver",
            Self::ToggleAppearance => "toggle_appearance",
        }
    }

    /// Returns the display name of the command.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sleep => "Sleep",
            Self::SleepDisplays => "Sleep Displays",
            Self::LockScreen => "Lock Screen",
            Self::Restart => "Restart",
            Self::ShutDown => "Shut Down",
            Self::LogOut => "Log Out",
            Self::EmptyTrash => "Empty Trash",
            Self::ScreenSaver => "Screen Saver",
            Self::ToggleAppearance => "Toggle Appearance",
        }
    }

    /// Returns the search aliases for the command.
    #[must_use]
    pub const fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::Sleep => &["sleep", "suspend"],
            Self::SleepDisplays => &["sleep displays", "display sleep"],
            Self::LockScreen => &["lock"],
            Self::Restart => &["restart", "reboot"],
            Self::ShutDown => &["shutdown", "power off"],
            Self::LogOut => &["logout", "sign out"],
            Self::EmptyTrash => &["empty trash", "clear trash"],
            Self::ScreenSaver => &["screensaver"],
            Self::ToggleAppearance => &["dark mode", "light mode", "toggle dark"],
        }
    }

    /// Returns the description of the command.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Sleep => "Put Mac to sleep",
            Self::SleepDisplays => "Turn off displays",
            Self::LockScreen => "Lock your Mac",
            Self::Restart => "Restart your Mac",
            Self::ShutDown => "Shut down your Mac",
            Self::LogOut => "Log out current user",
            Self::EmptyTrash => "Empty the Trash",
            Self::ScreenSaver => "Start screen saver",
            Self::ToggleAppearance => "Switch between light and dark mode",
        }
    }

    /// Returns the icon name for the command.
    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Sleep => "moon",
            Self::SleepDisplays | Self::ScreenSaver => "monitor",
            Self::LockScreen => "lock",
            Self::Restart => "rotate-ccw",
            Self::ShutDown => "power",
            Self::LogOut => "log-out",
            Self::EmptyTrash => "trash",
            Self::ToggleAppearance => "sun-moon",
        }
    }

    /// Returns whether this command requires confirmation before execution.
    #[must_use]
    pub const fn requires_confirmation(&self) -> bool {
        matches!(
            self,
            Self::Restart | Self::ShutDown | Self::LogOut | Self::EmptyTrash
        )
    }

    /// Returns the command info for this command.
    #[must_use]
    pub fn info(&self) -> CommandInfo {
        CommandInfo {
            command: *self,
            name: self.name(),
            aliases: self.aliases(),
            description: self.description(),
            icon: self.icon(),
            requires_confirmation: self.requires_confirmation(),
        }
    }
}

impl std::fmt::Display for SystemCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
