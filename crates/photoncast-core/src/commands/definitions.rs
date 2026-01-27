//! Command definitions and registry.
//!
//! This module contains the definitions for all built-in system commands.

/// A system command that can be executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemCommand {
    /// Search for files using Spotlight (enters File Search Mode).
    SearchFiles,
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
    /// Toggle launch at login.
    ToggleLaunchAtLogin,
    /// Open Preferences.
    Preferences,
    /// Create a new quicklink.
    CreateQuicklink,
    /// Manage quicklinks.
    ManageQuicklinks,
    /// Browse the bundled quicklinks library.
    BrowseQuicklinkLibrary,
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
            // Search Files appears first - prominent command for file search mode
            CommandInfo {
                command: Self::SearchFiles,
                name: "Search Files",
                aliases: &["files", "find", "documents", "search files", "spotlight"],
                description: "Find files using Spotlight",
                icon: "magnifyingglass",
                requires_confirmation: false,
            },
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
            CommandInfo {
                command: Self::ToggleLaunchAtLogin,
                name: "Toggle Launch at Login",
                aliases: &["launch at login", "startup", "auto start", "login item"],
                description: "Enable or disable launching PhotonCast at login",
                icon: "power",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::Preferences,
                name: "Preferences",
                aliases: &["preferences", "settings", "config", "configure"],
                description: "Open PhotonCast preferences",
                icon: "gear",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::CreateQuicklink,
                name: "Create Quicklink",
                aliases: &[
                    "create quicklink",
                    "new quicklink",
                    "add quicklink",
                    "quicklink",
                ],
                description: "Create a new quicklink",
                icon: "link",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::ManageQuicklinks,
                name: "Manage Quicklinks",
                aliases: &["manage quicklinks", "edit quicklinks", "quicklinks"],
                description: "Manage your quicklinks",
                icon: "list",
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::BrowseQuicklinkLibrary,
                name: "Browse Quicklink Library",
                aliases: &[
                    "quicklink library",
                    "browse quicklinks",
                    "quicklinks library",
                ],
                description: "Browse and add quicklinks from the library",
                icon: "book",
                requires_confirmation: false,
            },
        ]
    }

    /// Returns the command ID as a string.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::SearchFiles => "search_files",
            Self::Sleep => "sleep",
            Self::SleepDisplays => "sleep_displays",
            Self::LockScreen => "lock_screen",
            Self::Restart => "restart",
            Self::ShutDown => "shut_down",
            Self::LogOut => "log_out",
            Self::EmptyTrash => "empty_trash",
            Self::ScreenSaver => "screen_saver",
            Self::ToggleAppearance => "toggle_appearance",
            Self::ToggleLaunchAtLogin => "toggle_launch_at_login",
            Self::Preferences => "preferences",
            Self::CreateQuicklink => "create_quicklink",
            Self::ManageQuicklinks => "manage_quicklinks",
            Self::BrowseQuicklinkLibrary => "browse_quicklink_library",
        }
    }

    /// Returns the display name of the command.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::SearchFiles => "Search Files",
            Self::Sleep => "Sleep",
            Self::SleepDisplays => "Sleep Displays",
            Self::LockScreen => "Lock Screen",
            Self::Restart => "Restart",
            Self::ShutDown => "Shut Down",
            Self::LogOut => "Log Out",
            Self::EmptyTrash => "Empty Trash",
            Self::ScreenSaver => "Screen Saver",
            Self::ToggleAppearance => "Toggle Appearance",
            Self::ToggleLaunchAtLogin => "Toggle Launch at Login",
            Self::Preferences => "Preferences",
            Self::CreateQuicklink => "Create Quicklink",
            Self::ManageQuicklinks => "Manage Quicklinks",
            Self::BrowseQuicklinkLibrary => "Browse Quicklink Library",
        }
    }

    /// Returns the search aliases for the command.
    #[must_use]
    pub const fn aliases(&self) -> &'static [&'static str] {
        match self {
            Self::SearchFiles => &["files", "find", "documents", "search files", "spotlight"],
            Self::Sleep => &["sleep", "suspend"],
            Self::SleepDisplays => &["sleep displays", "display sleep"],
            Self::LockScreen => &["lock"],
            Self::Restart => &["restart", "reboot"],
            Self::ShutDown => &["shutdown", "power off"],
            Self::LogOut => &["logout", "sign out"],
            Self::EmptyTrash => &["empty trash", "clear trash"],
            Self::ScreenSaver => &["screensaver"],
            Self::ToggleAppearance => &["dark mode", "light mode", "toggle dark"],
            Self::ToggleLaunchAtLogin => {
                &["launch at login", "startup", "auto start", "login item"]
            },
            Self::Preferences => &["preferences", "settings", "config", "configure"],
            Self::CreateQuicklink => &[
                "create quicklink",
                "new quicklink",
                "add quicklink",
                "quicklink",
            ],
            Self::ManageQuicklinks => &["manage quicklinks", "edit quicklinks", "quicklinks"],
            Self::BrowseQuicklinkLibrary => &[
                "quicklink library",
                "browse quicklinks",
                "quicklinks library",
            ],
        }
    }

    /// Returns the description of the command.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::SearchFiles => "Find files using Spotlight",
            Self::Sleep => "Put Mac to sleep",
            Self::SleepDisplays => "Turn off displays",
            Self::LockScreen => "Lock your Mac",
            Self::Restart => "Restart your Mac",
            Self::ShutDown => "Shut down your Mac",
            Self::LogOut => "Log out current user",
            Self::EmptyTrash => "Empty the Trash",
            Self::ScreenSaver => "Start screen saver",
            Self::ToggleAppearance => "Switch between light and dark mode",
            Self::ToggleLaunchAtLogin => "Enable or disable launching PhotonCast at login",
            Self::Preferences => "Open PhotonCast preferences",
            Self::CreateQuicklink => "Create a new quicklink",
            Self::ManageQuicklinks => "Manage your quicklinks",
            Self::BrowseQuicklinkLibrary => "Browse and add quicklinks from the library",
        }
    }

    /// Returns the icon name for the command.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::SearchFiles => "magnifyingglass",
            Self::Sleep => "moon",
            Self::SleepDisplays | Self::ScreenSaver => "monitor",
            Self::LockScreen => "lock",
            Self::Restart => "rotate-ccw",
            Self::ShutDown => "power",
            Self::LogOut => "log-out",
            Self::EmptyTrash => "trash",
            Self::ToggleAppearance => "sun-moon",
            Self::ToggleLaunchAtLogin => "power",
            Self::Preferences => "gear",
            Self::CreateQuicklink => "link",
            Self::ManageQuicklinks => "list",
            Self::BrowseQuicklinkLibrary => "book",
        }
    }

    /// Returns whether this command requires confirmation before execution.
    #[must_use]
    pub const fn requires_confirmation(&self) -> bool {
        matches!(
            self,
            Self::Restart | Self::ShutDown | Self::LogOut | Self::EmptyTrash
        )
        // ToggleLaunchAtLogin does not require confirmation
    }

    /// Returns whether this command is a mode-switching command (doesn't execute directly).
    ///
    /// Mode-switching commands like `SearchFiles` or `Preferences` don't execute an action themselves;
    /// instead, they trigger a UI mode change in the launcher.
    #[must_use]
    pub const fn is_mode_command(&self) -> bool {
        matches!(self, Self::SearchFiles)
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
