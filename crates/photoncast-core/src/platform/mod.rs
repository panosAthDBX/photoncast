//! macOS platform integration.
//!
//! This module contains macOS-specific functionality for hotkeys,
//! accessibility, Spotlight, and app launching.

pub mod accessibility;
pub mod appearance;
pub mod dock_visibility;
pub mod file_actions;
pub mod file_browser;
pub mod hotkey;
pub mod hotkey_settings;
pub mod launch;
pub mod login_item;
pub mod menu_bar;
pub mod spotlight;
pub mod updates;

pub use accessibility::{
    check_accessibility_permission, check_permission_silent, get_permission_status,
    open_accessibility_settings, request_accessibility_permission, PermissionPoller,
    PermissionStatus,
};
#[cfg(target_os = "macos")]
pub use file_actions::{
    compress, copy_file_to_clipboard, delete_permanently, duplicate_file, get_apps_for_file,
    get_file_info, move_file, move_to_trash, open_with_app, rename_file, validate_filename,
    AppInfo, FileActionError, FileInfo,
};
pub use file_browser::{DirectoryEntry, FileBrowser};
pub use hotkey::{
    detect_hotkey_conflict, is_spotlight_enabled, ConflictInfo, DoubleTapDetector, HotkeyBinding,
    HotkeyError, HotkeyManager, Modifier, Modifiers,
};
pub use hotkey_settings::{
    config_to_binding, default_config_path, format_binding, is_modifier_key, is_reserved_key,
    is_valid_modifier, load_config, parse_modifier, parse_modifiers, save_hotkey_config,
    validate_binding, HotkeyChangeManager, HotkeySettings, HotkeySettingsError, KeyCaptureState,
};
pub use launch::{
    launch_app_by_bundle_id, launch_app_by_path, open_file, reveal_in_finder, AppLauncher,
    LaunchError,
};
pub use login_item::{LoginItemError, LoginItemManager, LoginItemStatus};
pub use menu_bar::{
    default_menu_items, MenuBarAction, MenuBarConfig, MenuBarError, MenuBarHandler, MenuBarManager,
    MenuBarStatus, MenuItem,
};
pub use spotlight::{
    FileKind, FileResult, SpotlightError, SpotlightProvider, SpotlightQuery, DEFAULT_MAX_RESULTS,
    DEFAULT_TIMEOUT_MS,
};
pub use dock_visibility::{
    get_dock_visibility, set_dock_visibility, toggle_dock_visibility, DockVisibilityError,
    DockVisibilityManager,
};
pub use updates::{
    AvailableUpdate, UpdateConfig, UpdateError, UpdateManager, UpdateStatus, DEFAULT_CHECK_INTERVAL,
    DEFAULT_FEED_URL,
};
