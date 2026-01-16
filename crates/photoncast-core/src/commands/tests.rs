//! Unit tests for the commands module.

use super::*;

mod command_definitions {
    use super::*;

    #[test]
    fn test_all_commands_returns_all_variants() {
        let commands = SystemCommand::all();
        assert_eq!(commands.len(), 9, "expected 9 system commands");

        // Verify all command types are present
        let command_ids: Vec<&str> = commands.iter().map(|c| c.command.id()).collect();
        assert!(command_ids.contains(&"sleep"));
        assert!(command_ids.contains(&"sleep_displays"));
        assert!(command_ids.contains(&"lock_screen"));
        assert!(command_ids.contains(&"restart"));
        assert!(command_ids.contains(&"shut_down"));
        assert!(command_ids.contains(&"log_out"));
        assert!(command_ids.contains(&"empty_trash"));
        assert!(command_ids.contains(&"screen_saver"));
        assert!(command_ids.contains(&"toggle_appearance"));
    }

    #[test]
    fn test_command_id_is_unique() {
        let commands = SystemCommand::all();
        let mut ids: Vec<&str> = commands.iter().map(|c| c.command.id()).collect();
        ids.sort();
        let original_len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "command IDs should be unique");
    }

    #[test]
    fn test_command_name_is_non_empty() {
        let commands = SystemCommand::all();
        for cmd in &commands {
            assert!(!cmd.name.is_empty(), "command name should not be empty");
            assert!(
                !cmd.command.name().is_empty(),
                "command.name() should not be empty"
            );
            assert_eq!(cmd.name, cmd.command.name(), "name should match");
        }
    }

    #[test]
    fn test_command_description_is_non_empty() {
        let commands = SystemCommand::all();
        for cmd in &commands {
            assert!(
                !cmd.description.is_empty(),
                "command description should not be empty"
            );
            assert!(
                !cmd.command.description().is_empty(),
                "command.description() should not be empty"
            );
            assert_eq!(
                cmd.description,
                cmd.command.description(),
                "description should match"
            );
        }
    }

    #[test]
    fn test_command_icon_is_non_empty() {
        let commands = SystemCommand::all();
        for cmd in &commands {
            assert!(!cmd.icon.is_empty(), "command icon should not be empty");
            assert!(
                !cmd.command.icon().is_empty(),
                "command.icon() should not be empty"
            );
            assert_eq!(cmd.icon, cmd.command.icon(), "icon should match");
        }
    }

    #[test]
    fn test_command_aliases_are_non_empty() {
        let commands = SystemCommand::all();
        for cmd in &commands {
            assert!(
                !cmd.aliases.is_empty(),
                "command should have at least one alias"
            );
            assert!(
                !cmd.command.aliases().is_empty(),
                "command.aliases() should not be empty"
            );
            assert_eq!(cmd.aliases, cmd.command.aliases(), "aliases should match");
        }
    }

    #[test]
    fn test_requires_confirmation_consistency() {
        let commands = SystemCommand::all();
        for cmd in &commands {
            assert_eq!(
                cmd.requires_confirmation,
                cmd.command.requires_confirmation(),
                "requires_confirmation should match for {}",
                cmd.name
            );
        }
    }

    #[test]
    fn test_destructive_commands_require_confirmation() {
        // These commands are destructive and should require confirmation
        assert!(SystemCommand::Restart.requires_confirmation());
        assert!(SystemCommand::ShutDown.requires_confirmation());
        assert!(SystemCommand::LogOut.requires_confirmation());
        assert!(SystemCommand::EmptyTrash.requires_confirmation());
    }

    #[test]
    fn test_non_destructive_commands_no_confirmation() {
        // These commands are non-destructive and should not require confirmation
        assert!(!SystemCommand::Sleep.requires_confirmation());
        assert!(!SystemCommand::SleepDisplays.requires_confirmation());
        assert!(!SystemCommand::LockScreen.requires_confirmation());
        assert!(!SystemCommand::ScreenSaver.requires_confirmation());
        assert!(!SystemCommand::ToggleAppearance.requires_confirmation());
    }

    #[test]
    fn test_command_info_returns_correct_data() {
        let cmd = SystemCommand::Sleep;
        let info = cmd.info();

        assert_eq!(info.command, SystemCommand::Sleep);
        assert_eq!(info.name, "Sleep");
        assert_eq!(info.description, "Put Mac to sleep");
        assert_eq!(info.icon, "moon");
        assert!(!info.requires_confirmation);
        assert!(info.aliases.contains(&"sleep"));
        assert!(info.aliases.contains(&"suspend"));
    }

    #[test]
    fn test_command_display_impl() {
        assert_eq!(format!("{}", SystemCommand::Sleep), "Sleep");
        assert_eq!(format!("{}", SystemCommand::LockScreen), "Lock Screen");
        assert_eq!(format!("{}", SystemCommand::ShutDown), "Shut Down");
    }
}

mod confirmation_dialog {
    use super::*;
    use crate::commands::system::ConfirmationDialog;

    #[test]
    fn test_confirmation_dialog_for_restart() {
        let dialog = ConfirmationDialog::for_command(&SystemCommand::Restart);
        assert!(dialog.is_some());
        let dialog = dialog.unwrap();
        assert!(dialog.title.contains("Restart"));
        assert!(dialog.is_destructive);
        assert!(!dialog.confirm_label.is_empty());
        assert!(!dialog.cancel_label.is_empty());
    }

    #[test]
    fn test_confirmation_dialog_for_shutdown() {
        let dialog = ConfirmationDialog::for_command(&SystemCommand::ShutDown);
        assert!(dialog.is_some());
        let dialog = dialog.unwrap();
        assert!(dialog.title.contains("Shut Down"));
        assert!(dialog.is_destructive);
    }

    #[test]
    fn test_confirmation_dialog_for_logout() {
        let dialog = ConfirmationDialog::for_command(&SystemCommand::LogOut);
        assert!(dialog.is_some());
        let dialog = dialog.unwrap();
        assert!(dialog.title.contains("Log Out"));
        assert!(dialog.is_destructive);
    }

    #[test]
    fn test_confirmation_dialog_for_empty_trash() {
        let dialog = ConfirmationDialog::for_command(&SystemCommand::EmptyTrash);
        assert!(dialog.is_some());
        let dialog = dialog.unwrap();
        assert!(dialog.title.contains("Trash"));
        assert!(dialog.is_destructive);
        assert!(dialog.message.contains("cannot be undone"));
    }

    #[test]
    fn test_no_confirmation_dialog_for_non_destructive() {
        assert!(ConfirmationDialog::for_command(&SystemCommand::Sleep).is_none());
        assert!(ConfirmationDialog::for_command(&SystemCommand::SleepDisplays).is_none());
        assert!(ConfirmationDialog::for_command(&SystemCommand::LockScreen).is_none());
        assert!(ConfirmationDialog::for_command(&SystemCommand::ScreenSaver).is_none());
        assert!(ConfirmationDialog::for_command(&SystemCommand::ToggleAppearance).is_none());
    }

    #[test]
    fn test_confirmation_dialog_via_command_method() {
        // Destructive commands should have confirmation dialogs
        assert!(SystemCommand::Restart.confirmation_dialog().is_some());
        assert!(SystemCommand::ShutDown.confirmation_dialog().is_some());
        assert!(SystemCommand::LogOut.confirmation_dialog().is_some());
        assert!(SystemCommand::EmptyTrash.confirmation_dialog().is_some());

        // Non-destructive commands should not have confirmation dialogs
        assert!(SystemCommand::Sleep.confirmation_dialog().is_none());
        assert!(SystemCommand::LockScreen.confirmation_dialog().is_none());
    }
}

mod command_error {
    use crate::commands::system::CommandError;

    #[test]
    fn test_execution_failed_error_display() {
        let error = CommandError::ExecutionFailed {
            command: "sleep".to_string(),
            reason: "permission denied".to_string(),
        };
        let display = format!("{error}");
        assert!(display.contains("sleep"));
        assert!(display.contains("permission denied"));
    }

    #[test]
    fn test_authorization_required_error_display() {
        let error = CommandError::AuthorizationRequired {
            command: "restart".to_string(),
        };
        let display = format!("{error}");
        assert!(display.contains("restart"));
        assert!(display.contains("authorization"));
    }

    #[test]
    fn test_not_available_error_display() {
        let error = CommandError::NotAvailable;
        let display = format!("{error}");
        assert!(display.contains("not available"));
    }

    #[test]
    fn test_user_message_execution_failed() {
        let error = CommandError::ExecutionFailed {
            command: "sleep".to_string(),
            reason: "permission denied".to_string(),
        };
        let message = error.user_message();
        assert!(message.contains("sleep"));
        assert!(message.contains("permission"));
    }

    #[test]
    fn test_user_message_authorization_required() {
        let error = CommandError::AuthorizationRequired {
            command: "restart".to_string(),
        };
        let message = error.user_message();
        assert!(message.contains("restart"));
        assert!(message.contains("System Settings"));
    }

    #[test]
    fn test_user_message_not_available() {
        let error = CommandError::NotAvailable;
        let message = error.user_message();
        assert!(message.contains("not available"));
    }

    #[test]
    fn test_is_recoverable() {
        let error = CommandError::ExecutionFailed {
            command: "sleep".to_string(),
            reason: "temporary failure".to_string(),
        };
        assert!(error.is_recoverable());

        let error = CommandError::AuthorizationRequired {
            command: "restart".to_string(),
        };
        assert!(!error.is_recoverable());

        let error = CommandError::NotAvailable;
        assert!(!error.is_recoverable());
    }
}

mod applescript {
    // Note: Actual AppleScript execution tests are skipped in CI
    // because they require macOS-specific permissions and may have side effects.
    // These tests verify the interface and error handling.

    #[test]
    fn test_applescript_module_exists() {
        // Just verify the module compiles and functions are accessible
        use crate::commands::system::{run_applescript, run_applescript_with_output};

        // These functions exist and have the correct signature
        let _: fn(&str) -> anyhow::Result<()> = run_applescript;
        let _: fn(&str) -> anyhow::Result<String> = run_applescript_with_output;
    }
}

mod command_executor {
    use crate::commands::{CommandExecutor, InMemoryUsageTracker, SystemCommand};

    #[test]
    fn test_executor_lookup_valid_command() {
        let executor = CommandExecutor::new();

        assert!(executor.lookup("sleep").is_some());
        assert!(executor.lookup("restart").is_some());
        assert!(executor.lookup("lock_screen").is_some());
    }

    #[test]
    fn test_executor_lookup_invalid_command() {
        let executor = CommandExecutor::new();

        assert!(executor.lookup("nonexistent").is_none());
        assert!(executor.lookup("").is_none());
        assert!(executor.lookup("invalid_command").is_none());
    }

    #[test]
    fn test_executor_lookup_all_commands() {
        let executor = CommandExecutor::new();

        // All commands should be lookupable by their ID
        for cmd_info in SystemCommand::all() {
            let result = executor.lookup(cmd_info.command.id());
            assert!(
                result.is_some(),
                "command '{}' should be lookupable by ID '{}'",
                cmd_info.name,
                cmd_info.command.id()
            );
            assert_eq!(result.unwrap(), cmd_info.command);
        }
    }

    #[test]
    fn test_executor_with_usage_tracker() {
        let tracker = InMemoryUsageTracker::new();
        let executor = CommandExecutor::with_tracker(tracker);

        // Lookup should still work
        assert!(executor.lookup("sleep").is_some());
    }

    #[test]
    fn test_execute_by_id_invalid() {
        let executor = CommandExecutor::new();

        let result = executor.execute_by_id("nonexistent");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("command not found"));
    }
}

mod usage_tracker {
    use crate::commands::{CommandUsageTracker, InMemoryUsageTracker, NoOpUsageTracker};

    #[test]
    fn test_noop_tracker_returns_defaults() {
        let tracker = NoOpUsageTracker;

        assert_eq!(tracker.get_execution_count("sleep"), 0);
        assert_eq!(tracker.get_last_execution("sleep"), None);

        // Recording should be a no-op
        tracker.record_execution("sleep");
        assert_eq!(tracker.get_execution_count("sleep"), 0);
    }

    #[test]
    fn test_in_memory_tracker_records_execution() {
        let tracker = InMemoryUsageTracker::new();

        assert_eq!(tracker.get_execution_count("sleep"), 0);
        assert_eq!(tracker.get_last_execution("sleep"), None);

        tracker.record_execution("sleep");
        assert_eq!(tracker.get_execution_count("sleep"), 1);
        assert!(tracker.get_last_execution("sleep").is_some());
    }

    #[test]
    fn test_in_memory_tracker_increments_count() {
        let tracker = InMemoryUsageTracker::new();

        tracker.record_execution("restart");
        tracker.record_execution("restart");
        tracker.record_execution("restart");

        assert_eq!(tracker.get_execution_count("restart"), 3);
    }

    #[test]
    fn test_in_memory_tracker_tracks_multiple_commands() {
        let tracker = InMemoryUsageTracker::new();

        tracker.record_execution("sleep");
        tracker.record_execution("restart");
        tracker.record_execution("sleep");

        assert_eq!(tracker.get_execution_count("sleep"), 2);
        assert_eq!(tracker.get_execution_count("restart"), 1);
        assert_eq!(tracker.get_execution_count("lock_screen"), 0);
    }

    #[test]
    fn test_in_memory_tracker_updates_timestamp() {
        let tracker = InMemoryUsageTracker::new();

        tracker.record_execution("sleep");
        let first_timestamp = tracker.get_last_execution("sleep").unwrap();

        // Small delay to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));

        tracker.record_execution("sleep");
        let second_timestamp = tracker.get_last_execution("sleep").unwrap();

        assert!(second_timestamp >= first_timestamp);
    }
}
