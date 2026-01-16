//! Integration tests for global hotkey registration.
//!
//! These tests verify the hotkey system works correctly on macOS.
//! Some tests require accessibility permissions to fully run.

use photoncast_core::platform::{
    check_accessibility_permission, detect_hotkey_conflict, HotkeyBinding, HotkeyError,
    HotkeyManager, KeyCode, Modifiers,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Tests that HotkeyBinding has correct default values.
#[test]
fn test_default_binding() {
    let binding = HotkeyBinding::default();

    assert_eq!(binding.key, KeyCode::Space, "Default key should be Space");
    assert!(
        binding.modifiers.command,
        "Default binding should have Command modifier"
    );
    assert!(
        !binding.modifiers.option,
        "Default binding should not have Option modifier"
    );
    assert!(
        !binding.modifiers.control,
        "Default binding should not have Control modifier"
    );
    assert!(
        !binding.modifiers.shift,
        "Default binding should not have Shift modifier"
    );
}

/// Tests that HotkeyBinding validation works correctly.
#[test]
fn test_binding_validation() {
    // Valid binding with modifier
    let valid = HotkeyBinding::new(KeyCode::Space, Modifiers::COMMAND);
    assert!(valid.is_valid(), "Binding with modifier should be valid");

    // Invalid binding without modifier
    let invalid = HotkeyBinding::new(KeyCode::Space, Modifiers::NONE);
    assert!(
        !invalid.is_valid(),
        "Binding without modifier should be invalid"
    );

    // Valid with multiple modifiers
    let multi = HotkeyBinding::new(KeyCode::P, Modifiers::COMMAND_SHIFT);
    assert!(
        multi.is_valid(),
        "Binding with multiple modifiers should be valid"
    );
}

/// Tests that HotkeyBinding display formatting works.
#[test]
fn test_binding_display() {
    let binding = HotkeyBinding::default();
    assert_eq!(binding.to_string(), "⌘Space");

    let binding = HotkeyBinding::new(KeyCode::P, Modifiers::COMMAND_SHIFT);
    assert_eq!(binding.to_string(), "⇧⌘P");

    let binding = HotkeyBinding::new(
        KeyCode::A,
        Modifiers {
            command: true,
            option: true,
            control: false,
            shift: false,
        },
    );
    assert_eq!(binding.to_string(), "⌥⌘A");
}

/// Tests KeyCode conversion from u16.
#[test]
fn test_keycode_from_u16() {
    // Valid keycodes
    assert_eq!(KeyCode::from_u16(49), Some(KeyCode::Space));
    assert_eq!(KeyCode::from_u16(0), Some(KeyCode::A));
    assert_eq!(KeyCode::from_u16(36), Some(KeyCode::Return));
    assert_eq!(KeyCode::from_u16(53), Some(KeyCode::Escape));

    // Invalid keycodes
    assert_eq!(KeyCode::from_u16(200), None);
    assert_eq!(KeyCode::from_u16(255), None);
}

/// Tests that KeyCode round-trips correctly.
#[test]
fn test_keycode_roundtrip() {
    let keys = [
        KeyCode::Space,
        KeyCode::A,
        KeyCode::Z,
        KeyCode::Return,
        KeyCode::F1,
        KeyCode::F12,
    ];

    for key in keys {
        let code = key.as_u16();
        let recovered = KeyCode::from_u16(code);
        assert_eq!(recovered, Some(key), "KeyCode should round-trip: {:?}", key);
    }
}

/// Tests Modifiers display formatting.
#[test]
fn test_modifiers_display() {
    assert_eq!(Modifiers::COMMAND.to_string(), "⌘");
    assert_eq!(Modifiers::OPTION.to_string(), "⌥");
    assert_eq!(Modifiers::CONTROL.to_string(), "⌃");
    assert_eq!(Modifiers::SHIFT.to_string(), "⇧");
    assert_eq!(Modifiers::NONE.to_string(), "");

    // Order should be ⌃⌥⇧⌘
    let all = Modifiers {
        command: true,
        option: true,
        control: true,
        shift: true,
    };
    assert_eq!(all.to_string(), "⌃⌥⇧⌘");
}

/// Tests that HotkeyManager can be created.
#[test]
fn test_hotkey_manager_new() {
    let binding = HotkeyBinding::default();
    let manager = HotkeyManager::new(binding.clone());

    assert_eq!(manager.binding(), &binding);
    assert!(!manager.is_registered());
}

/// Tests that registration fails without accessibility permission.
#[test]
fn test_registration_requires_permission() {
    // Skip if permission is already granted
    if check_accessibility_permission() {
        println!("Skipping test - accessibility permission already granted");
        return;
    }

    let binding = HotkeyBinding::default();
    let mut manager = HotkeyManager::new(binding);

    let result = manager.register(|| {});

    match result {
        Err(HotkeyError::PermissionDenied) => {
            // Expected
        },
        Err(e) => panic!("Expected PermissionDenied, got: {:?}", e),
        Ok(()) => panic!("Expected error without permission"),
    }
}

/// Tests registration with accessibility permission.
#[cfg(target_os = "macos")]
#[test]
fn test_registration_with_permission() {
    // Skip if no permission
    if !check_accessibility_permission() {
        println!("Skipping test - accessibility permission not granted");
        return;
    }

    // Use a binding that doesn't conflict with Spotlight
    let binding = HotkeyBinding::new(KeyCode::P, Modifiers::COMMAND_SHIFT);
    let mut manager = HotkeyManager::new(binding);

    let callback_called = Arc::new(AtomicBool::new(false));

    let result = manager.register({
        let flag = Arc::clone(&callback_called);
        move || {
            flag.store(true, Ordering::Relaxed);
        }
    });

    assert!(result.is_ok(), "Registration should succeed: {:?}", result);
    assert!(manager.is_registered());

    // Clean up
    manager.unregister();
    assert!(!manager.is_registered());
}

/// Tests that invalid binding is rejected.
#[test]
fn test_invalid_binding_rejected() {
    // Skip if no permission (permission is checked first)
    if !check_accessibility_permission() {
        println!("Skipping test - accessibility permission not granted");
        return;
    }

    let invalid_binding = HotkeyBinding::new(KeyCode::Space, Modifiers::NONE);
    let mut manager = HotkeyManager::new(invalid_binding);

    let result = manager.register(|| {});

    match result {
        Err(HotkeyError::InvalidBinding) => {
            // Expected
        },
        Err(e) => panic!("Expected InvalidBinding, got: {:?}", e),
        Ok(()) => panic!("Should reject invalid binding"),
    }
}

/// Tests conflict detection for Spotlight.
#[test]
fn test_spotlight_conflict_detection() {
    // Default binding (Cmd+Space) may conflict with Spotlight
    let binding = HotkeyBinding::default();
    let conflict = detect_hotkey_conflict(&binding);

    // We can't guarantee Spotlight is enabled, but we test the function doesn't panic
    if let Some(app) = conflict {
        assert_eq!(app, "Spotlight");
    }
}

/// Tests that non-conflicting bindings are accepted.
#[test]
fn test_no_conflict_for_custom_binding() {
    // Cmd+Shift+P should not conflict
    let binding = HotkeyBinding::new(KeyCode::P, Modifiers::COMMAND_SHIFT);
    let conflict = detect_hotkey_conflict(&binding);

    assert!(conflict.is_none(), "Custom binding should not conflict");
}

/// Tests HotkeyError user messages.
#[test]
fn test_error_user_messages() {
    // PermissionDenied
    let err = HotkeyError::PermissionDenied;
    let msg = err.user_message();
    assert!(
        msg.contains("accessibility"),
        "Permission error should mention accessibility"
    );
    assert!(err.is_recoverable());

    // ConflictDetected
    let err = HotkeyError::ConflictDetected {
        app: "Spotlight".to_string(),
    };
    let msg = err.user_message();
    assert!(
        msg.contains("conflict") || msg.contains("use"),
        "Conflict error should explain the issue"
    );
    assert!(err.is_recoverable());

    // RegistrationFailed
    let err = HotkeyError::RegistrationFailed {
        reason: "test".to_string(),
    };
    let msg = err.user_message();
    assert!(
        msg.contains("Failed") || msg.contains("failed"),
        "Registration error should explain failure"
    );
    assert!(err.is_recoverable());

    // InvalidBinding
    let err = HotkeyError::InvalidBinding;
    let msg = err.user_message();
    assert!(
        msg.contains("invalid"),
        "Invalid binding error should mention invalid"
    );
    assert!(!err.is_recoverable()); // This one is not recoverable
}

/// Tests error action hints.
#[test]
fn test_error_action_hints() {
    assert_eq!(
        HotkeyError::PermissionDenied.action_hint(),
        "Open System Settings"
    );
    assert_eq!(
        HotkeyError::ConflictDetected {
            app: "Test".to_string()
        }
        .action_hint(),
        "Change Shortcut"
    );
    assert_eq!(
        HotkeyError::RegistrationFailed {
            reason: "test".to_string()
        }
        .action_hint(),
        "Retry"
    );
    assert_eq!(
        HotkeyError::InvalidBinding.action_hint(),
        "Choose Different Shortcut"
    );
}

/// Tests that manager properly cleans up on drop.
#[cfg(target_os = "macos")]
#[test]
fn test_manager_cleanup_on_drop() {
    if !check_accessibility_permission() {
        println!("Skipping test - accessibility permission not granted");
        return;
    }

    let binding = HotkeyBinding::new(KeyCode::Q, Modifiers::COMMAND_SHIFT);

    {
        let mut manager = HotkeyManager::new(binding.clone());
        let _ = manager.register(|| {});
        // Manager dropped here
    }

    // Create a new manager with the same binding - should work if cleanup was proper
    let mut manager2 = HotkeyManager::new(binding);
    let result = manager2.register(|| {});
    assert!(
        result.is_ok(),
        "Should be able to re-register after cleanup: {:?}",
        result
    );
}

/// Tests modifiers any/is_empty helpers.
#[test]
fn test_modifiers_helpers() {
    assert!(!Modifiers::NONE.any());
    assert!(Modifiers::NONE.is_empty());

    assert!(Modifiers::COMMAND.any());
    assert!(!Modifiers::COMMAND.is_empty());

    let multi = Modifiers {
        command: true,
        shift: true,
        option: false,
        control: false,
    };
    assert!(multi.any());
    assert!(!multi.is_empty());
}

/// Tests CGEventFlags conversion (macOS only).
#[cfg(target_os = "macos")]
#[test]
fn test_cg_flags_conversion() {
    // Test command flag
    let flags = Modifiers::COMMAND.to_cg_flags();
    let converted = Modifiers::from_cg_flags(flags);
    assert_eq!(converted, Modifiers::COMMAND);

    // Test option flag
    let flags = Modifiers::OPTION.to_cg_flags();
    let converted = Modifiers::from_cg_flags(flags);
    assert_eq!(converted, Modifiers::OPTION);

    // Test all modifiers
    let all = Modifiers {
        command: true,
        option: true,
        control: true,
        shift: true,
    };
    let flags = all.to_cg_flags();
    let converted = Modifiers::from_cg_flags(flags);
    assert_eq!(converted, all);
}

/// Tests hotkey binding matches function (macOS only).
#[cfg(target_os = "macos")]
#[test]
fn test_binding_matches() {
    let binding = HotkeyBinding::new(KeyCode::Space, Modifiers::COMMAND);

    // Should match: Space with Command
    let space_code = KeyCode::Space.as_u16();
    let cmd_flags = Modifiers::COMMAND.to_cg_flags();
    assert!(binding.matches(space_code, cmd_flags));

    // Should not match: Space without modifiers
    assert!(!binding.matches(space_code, 0));

    // Should not match: Different key with Command
    let a_code = KeyCode::A.as_u16();
    assert!(!binding.matches(a_code, cmd_flags));

    // Should not match: Space with wrong modifier
    let opt_flags = Modifiers::OPTION.to_cg_flags();
    assert!(!binding.matches(space_code, opt_flags));
}
