# Tasks List for App Management - Raycast Parity

> **Spec:** `/droidz/specs/2026-01-21-app-management-raycast-parity/spec.md`  
> **Created:** 2026-01-21  
> **Total Tasks:** 40

---

## Task Group 1: Data Models & Configuration

Foundation tasks that define the data structures used throughout the implementation.

### Task 1.1: Extend Application Models

- **Description**: Update `models.rs` to include new fields for running state, auto quit configuration, and responding status as specified in Section 5.1.
- **Dependencies**: None
- **Acceptance Criteria**:
  - `RunningApplication` struct includes `is_responding: bool` and `launch_time: DateTime<Utc>`
  - `ApplicationWithState` struct is added combining app info with running state and auto quit settings
  - Existing tests pass
- **Complexity**: Small

### Task 1.2: Create Auto Quit Config Model

- **Description**: Define `AutoQuitConfig` and `AutoQuitAppConfig` structures as specified in Section 3.7. This will be stored in `~/.config/photoncast/auto_quit.toml`.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `AutoQuitConfig` with `apps: HashMap<BundleId, AutoQuitAppConfig>`
  - `AutoQuitAppConfig` with `enabled`, `timeout_minutes`, `last_active` fields
  - TOML serialization/deserialization implemented
  - Default timeout is 3 minutes
- **Complexity**: Small

### Task 1.3: Extend Uninstall Models

- **Description**: Add new `RelatedFileCategory` variants for Cookies, WebKit, HTTPStorages, and Group Containers as specified in Section 3.8.
- **Dependencies**: None
- **Acceptance Criteria**:
  - `RelatedFileCategory` enum includes: `Cookies`, `WebKit`, `HTTPStorages`, `GroupContainers`
  - `RelatedFile` struct includes `selected: bool` field for user deselection
  - `UninstallPreview` includes `space_freed_formatted: String`
- **Complexity**: Small

---

## Task Group 2: Running App Detection Enhancement

Enhance process detection to support new running app features.

### Task 2.1: Add Is Responding Detection

- **Description**: Implement `is_app_responding()` function in `process.rs` to detect if an app is responsive using Apple Events timeout or `NSRunningApplication` APIs.
- **Dependencies**: None
- **Acceptance Criteria**:
  - `is_app_responding(pid: i32) -> bool` function implemented
  - Returns `false` if app doesn't respond within 2 seconds
  - Uses `NSRunningApplication.isFinishedLaunching` or Apple Events
- **Complexity**: Medium

### Task 2.2: Enhance Running App Information

- **Description**: Extend `get_running_apps()` to include `is_responding`, `is_hidden`, and `launch_time` fields.
- **Dependencies**: Task 2.1, Task 1.1
- **Acceptance Criteria**:
  - `RunningApp` struct populated with all new fields
  - `is_hidden` correctly detected via `NSRunningApplication.isHidden`
  - `launch_time` populated from process launch date
- **Complexity**: Small

### Task 2.3: Add Is Running by Bundle ID

- **Description**: Implement `is_app_running(bundle_id: &str) -> bool` as a convenience function for quick running status checks.
- **Dependencies**: None
- **Acceptance Criteria**:
  - Function returns `true` if any running app matches the bundle ID
  - Case-insensitive bundle ID matching
- **Complexity**: Small

---

## Task Group 3: App Actions Module

Create new `actions.rs` module for Show in Finder, Copy actions, and Hide.

### Task 3.1: Create Actions Module Structure

- **Description**: Create new `actions.rs` module in `photoncast-apps` crate and register it in `lib.rs`.
- **Dependencies**: None
- **Acceptance Criteria**:
  - `actions.rs` file created
  - Module exported from `lib.rs`
  - Basic error handling types defined
- **Complexity**: Small

### Task 3.2: Implement Show in Finder

- **Description**: Implement `reveal_in_finder(path: &Path)` function using `NSWorkspace.selectFile:inFileViewerRootedAtPath:` or `open -R` command.
- **Dependencies**: Task 3.1
- **Acceptance Criteria**:
  - App bundle is selected in Finder
  - Finder window brought to foreground
  - Works for apps in `/Applications`, `~/Applications`, and other locations
  - Error returned if path doesn't exist
- **Complexity**: Small

### Task 3.3: Implement Copy Path

- **Description**: Implement `copy_path_to_clipboard(path: &Path)` function to copy the full POSIX path to system clipboard.
- **Dependencies**: Task 3.1
- **Acceptance Criteria**:
  - Full path copied in POSIX format (e.g., `/Applications/Slack.app`)
  - Uses macOS clipboard APIs (`NSPasteboard`)
  - Returns success/failure result
- **Complexity**: Small

### Task 3.4: Implement Copy Bundle ID

- **Description**: Implement `copy_bundle_id_to_clipboard(bundle_id: &str)` function to copy bundle identifier to clipboard.
- **Dependencies**: Task 3.1
- **Acceptance Criteria**:
  - Bundle ID copied exactly as provided
  - Uses macOS clipboard APIs (`NSPasteboard`)
  - Returns success/failure result
- **Complexity**: Small

### Task 3.5: Implement Hide App

- **Description**: Implement `hide_app(bundle_id: &str)` function using `NSRunningApplication.hide()` or AppleScript.
- **Dependencies**: Task 3.1
- **Acceptance Criteria**:
  - App windows are hidden (not closed)
  - Works for any running application
  - Error returned if app is not running
- **Complexity**: Small

---

## Task Group 4: Quit and Force Quit Enhancement

Enhance existing quit functionality with confirmation and not-responding detection.

### Task 4.1: Enhance Quit with Timeout

- **Description**: Update `quit_app()` to wait up to 5 seconds for app to quit and return success/failure status.
- **Dependencies**: None
- **Acceptance Criteria**:
  - Function waits up to 5 seconds for graceful quit
  - Returns `Ok(true)` if app quit successfully
  - Returns `Ok(false)` if timeout reached
  - Uses `NSRunningApplication.terminate()`
- **Complexity**: Small

### Task 4.2: Add Force Quit Confirmation Logic

- **Description**: Add helper function to determine if force quit confirmation should be shown, skipping for non-responding apps.
- **Dependencies**: Task 2.1
- **Acceptance Criteria**:
  - `should_confirm_force_quit(pid: i32) -> bool` function
  - Returns `false` if app is not responding
  - Returns `true` otherwise (confirmation needed)
- **Complexity**: Small

### Task 4.3: Implement Quit by Bundle ID

- **Description**: Add `quit_app_by_bundle_id(bundle_id: &str)` convenience function for quitting by bundle ID instead of PID.
- **Dependencies**: Task 2.3
- **Acceptance Criteria**:
  - Finds running app by bundle ID
  - Calls existing quit logic with found PID
  - Returns error if app not running
- **Complexity**: Small

---

## Task Group 5: Auto Quit Feature (Rename from App Sleep)

Rename and enhance the existing App Sleep feature to Auto Quit for Raycast parity.

### Task 5.1: Rename Sleep Module to Auto Quit

- **Description**: Rename `sleep.rs` to `auto_quit.rs` and update all references throughout the codebase.
- **Dependencies**: None
- **Acceptance Criteria**:
  - File renamed from `sleep.rs` to `auto_quit.rs`
  - Module export in `lib.rs` updated
  - All internal imports updated
  - `AppSleepManager` renamed to `AutoQuitManager`
  - `AppSleepConfig` renamed to `AutoQuitConfig`
- **Complexity**: Medium

### Task 5.2: Implement Activity Tracking

- **Description**: Add activity tracking to `AutoQuitManager` that records when each app was last active (had focus).
- **Dependencies**: Task 5.1, Task 1.2
- **Acceptance Criteria**:
  - `activity_tracker: HashMap<BundleId, Instant>` field added
  - `on_app_activated(bundle_id: &str)` method updates timestamp
  - Activity persists for the session duration
- **Complexity**: Small

### Task 5.3: Implement Inactive App Check

- **Description**: Implement `check_and_quit_inactive()` method that checks for and quits inactive apps based on their configured timeouts.
- **Dependencies**: Task 5.2, Task 4.3
- **Acceptance Criteria**:
  - Checks all enabled apps against their timeout
  - Uses graceful quit (not force quit)
  - Returns list of apps that were quit
  - Called every 30 seconds by the app
- **Complexity**: Medium

### Task 5.4: Implement Auto Quit Enable/Disable

- **Description**: Add methods to enable/disable auto quit for specific apps with persistence.
- **Dependencies**: Task 5.1, Task 1.2
- **Acceptance Criteria**:
  - `enable_auto_quit(bundle_id: &str, timeout_minutes: u32)` method
  - `disable_auto_quit(bundle_id: &str)` method
  - Configuration saved to `~/.config/photoncast/auto_quit.toml`
  - Default timeout of 3 minutes when not specified
- **Complexity**: Small

### Task 5.5: Add Suggested Apps List

- **Description**: Create a static list of suggested apps for Auto Quit (messaging, calendar, social apps) as specified in Section 3.7.
- **Dependencies**: None
- **Acceptance Criteria**:
  - `SUGGESTED_AUTO_QUIT_APPS` constant with bundle IDs
  - Categories: Messaging (Slack, Discord, Messages), Calendar, Social, Email
  - Used for quick setup UI
- **Complexity**: Small

---

## Task Group 6: Uninstaller Enhancement

Enhance the existing uninstaller with additional file categories and user selection.

### Task 6.1: Add New Search Paths

- **Description**: Extend `RELATED_FILE_SEARCH_PATHS` to include Cookies, WebKit, HTTPStorages, and Group Containers paths.
- **Dependencies**: Task 1.3
- **Acceptance Criteria**:
  - Paths added: `~/Library/Cookies/`, `~/Library/WebKit/`, `~/Library/HTTPStorages/`, `~/Library/Group Containers/`
  - Both user and system library paths included where applicable
- **Complexity**: Small

### Task 6.2: Implement Group Container Detection

- **Description**: Add logic to detect app group containers by matching group identifiers from the app's entitlements.
- **Dependencies**: Task 6.1
- **Acceptance Criteria**:
  - Reads app entitlements to find group identifiers
  - Matches `~/Library/Group Containers/<GroupID>`
  - Only includes containers actually used by the app
- **Complexity**: Medium

### Task 6.3: Add User File Selection Support

- **Description**: Implement ability for users to select/deselect individual files in the uninstall preview.
- **Dependencies**: Task 1.3
- **Acceptance Criteria**:
  - `RelatedFile.selected` field respected during uninstall
  - `uninstall()` function only removes selected files
  - Default is all files selected
- **Complexity**: Small

### Task 6.4: Add System App Protection

- **Description**: Implement safety checks to prevent uninstalling system and Apple apps.
- **Dependencies**: None
- **Acceptance Criteria**:
  - Apps in `/System/Applications/*` are protected
  - Apps with bundle ID starting with `com.apple.` are protected
  - Clear error message when attempting to uninstall protected apps
- **Complexity**: Small

### Task 6.5: Format Space Freed Display

- **Description**: Add human-readable formatting for total space freed (e.g., "863 MB").
- **Dependencies**: None
- **Acceptance Criteria**:
  - `UninstallPreview.space_freed_formatted` populated
  - Uses appropriate units (KB, MB, GB)
  - Individual file sizes also formatted
- **Complexity**: Small

---

## Task Group 7: UI Integration

Integrate all features into the launcher UI.

### Task 7.1: Implement Running App Indicator

- **Description**: Add green dot indicator to app icons in the launcher for running apps.
- **Dependencies**: Task 2.2
- **Acceptance Criteria**:
  - 8px green dot displayed bottom-right of app icon
  - Only shown for currently running apps
  - Dot color: Green (matching Raycast)
- **Complexity**: Medium

### Task 7.2: Implement Auto Quit Indicator

- **Description**: Add orange dot indicator below the running indicator for apps with Auto Quit enabled.
- **Dependencies**: Task 5.1, Task 7.1
- **Acceptance Criteria**:
  - Orange dot shown for apps with Auto Quit enabled
  - Position: Below green running dot
  - Tooltip shows timeout (e.g., "Auto Quit enabled (3 min)")
- **Complexity**: Small

### Task 7.3: Create App Action Panel

- **Description**: Implement the app-specific action panel (⌘K) with grouped actions as specified in Section 4.2.
- **Dependencies**: Task 3.2, Task 3.3, Task 3.4, Task 3.5, Task 4.1
- **Acceptance Criteria**:
  - Actions grouped: Primary, Info, Auto Quit, Danger Zone
  - Keyboard shortcuts displayed and functional
  - Running app actions only shown for running apps
  - Auto Quit toggle reflects current state
- **Complexity**: Large

### Task 7.4: Implement Action Keyboard Shortcuts

- **Description**: Add keyboard shortcut handlers for all app actions.
- **Dependencies**: Task 7.3
- **Acceptance Criteria**:
  - `⌘⇧F` - Show in Finder
  - `⌘⇧C` - Copy Path
  - `⌘⇧B` - Copy Bundle ID
  - `⌘Q` - Quit (running apps only)
  - `⌘⌥Q` - Force Quit (running apps only)
  - `⌘H` - Hide (running apps only)
  - `⌘⌫` - Uninstall
- **Complexity**: Medium

### Task 7.5: Create Uninstall Preview UI

- **Description**: Implement the uninstall preview dialog as specified in Section 3.8 showing all files to be removed.
- **Dependencies**: Task 6.1, Task 6.2, Task 6.3
- **Acceptance Criteria**:
  - Shows app name and total size
  - Lists all related files by category
  - Individual items can be selected/deselected
  - Shows total space to be freed
  - Three options: Uninstall, Keep Related Files, Cancel
- **Complexity**: Large

### Task 7.6: Create Auto Quit Settings UI

- **Description**: Implement per-app Auto Quit settings panel as specified in Section 4.3.
- **Dependencies**: Task 5.4
- **Acceptance Criteria**:
  - Toggle to enable/disable Auto Quit
  - Dropdown for timeout selection (1, 2, 3, 5, 10, 15, 30 minutes)
  - Accessible from action panel and Preferences
- **Complexity**: Medium

### Task 7.7: Create Manage Auto Quits Command

- **Description**: Implement dedicated command to view and manage all apps with Auto Quit enabled as specified in Section 4.4.
- **Dependencies**: Task 5.4, Task 7.6
- **Acceptance Criteria**:
  - Lists all apps with Auto Quit enabled
  - Shows timeout for each app
  - Quick disable button for each app
  - Add App button to enable for new apps
- **Complexity**: Medium

### Task 7.8: Add Toast Notifications

- **Description**: Implement toast notifications for copy actions and auto quit events.
- **Dependencies**: Task 3.3, Task 3.4
- **Acceptance Criteria**:
  - "Path copied to clipboard" toast on Copy Path
  - "Bundle ID copied to clipboard" toast on Copy Bundle ID
  - Optional notification when app is auto-quit
- **Complexity**: Small

---

## Task Group 8: Testing

Comprehensive testing for all new features.

### Task 8.1: Unit Tests - Running App Detection

- **Description**: Write unit tests for running app detection and is_responding functionality.
- **Dependencies**: Task 2.1, Task 2.2, Task 2.3
- **Acceptance Criteria**:
  - `test_running_app_detection` - Verify running apps correctly detected
  - `test_is_responding` - Verify responding detection works
  - `test_is_running_by_bundle_id` - Verify bundle ID lookup
- **Complexity**: Medium

### Task 8.2: Unit Tests - App Actions

- **Description**: Write unit tests for all app actions (Show in Finder, Copy, Hide).
- **Dependencies**: Task 3.2, Task 3.3, Task 3.4, Task 3.5
- **Acceptance Criteria**:
  - `test_show_in_finder` - Verify Finder reveals correct path
  - `test_copy_bundle_id` - Verify correct bundle ID copied
  - `test_copy_path` - Verify correct path copied
  - `test_hide_app` - Verify app windows hidden
- **Complexity**: Medium

### Task 8.3: Unit Tests - Quit Operations

- **Description**: Write unit tests for quit and force quit functionality.
- **Dependencies**: Task 4.1, Task 4.2, Task 4.3
- **Acceptance Criteria**:
  - `test_quit_app` - Verify graceful quit works
  - `test_force_quit_app` - Verify force quit terminates process
  - `test_quit_by_bundle_id` - Verify bundle ID lookup works
- **Complexity**: Medium

### Task 8.4: Unit Tests - Auto Quit

- **Description**: Write unit tests for Auto Quit activity tracking and timeout logic.
- **Dependencies**: Task 5.2, Task 5.3, Task 5.4
- **Acceptance Criteria**:
  - `test_auto_quit_tracking` - Verify activity tracking works
  - `test_auto_quit_timeout` - Verify apps quit after timeout
  - `test_auto_quit_enable_disable` - Verify config persistence
- **Complexity**: Medium

### Task 8.5: Unit Tests - Uninstaller

- **Description**: Write unit tests for enhanced uninstaller functionality.
- **Dependencies**: Task 6.1, Task 6.2, Task 6.3, Task 6.4
- **Acceptance Criteria**:
  - `test_uninstall_preview` - Verify all related files found
  - `test_uninstall_execution` - Verify files moved to trash
  - `test_system_app_protection` - Verify system apps protected
  - `test_group_container_detection` - Verify group containers found
- **Complexity**: Medium

### Task 8.6: Integration Tests - UI Indicators

- **Description**: Write integration tests for running and auto quit indicators in the UI.
- **Dependencies**: Task 7.1, Task 7.2
- **Acceptance Criteria**:
  - `test_app_search_shows_running_indicator` - UI shows running dot
  - `test_app_search_shows_auto_quit_indicator` - UI shows auto quit dot
- **Complexity**: Medium

### Task 8.7: Integration Tests - Action Panel

- **Description**: Write integration tests for the action panel functionality.
- **Dependencies**: Task 7.3, Task 7.4
- **Acceptance Criteria**:
  - `test_action_panel_shows_running_actions` - Quit/Force Quit shown for running apps
  - `test_action_panel_keyboard_shortcuts` - All shortcuts work correctly
- **Complexity**: Medium

### Task 8.8: Integration Tests - Persistence

- **Description**: Write integration tests for configuration persistence.
- **Dependencies**: Task 5.4, Task 6.3
- **Acceptance Criteria**:
  - `test_auto_quit_persists_across_restarts` - Config survives app restart
  - `test_uninstall_cleans_all_files` - All selected files removed
- **Complexity**: Medium

---

## Summary

| Group | Tasks | Complexity |
|-------|-------|------------|
| 1. Data Models & Configuration | 3 | Small |
| 2. Running App Detection | 3 | Small-Medium |
| 3. App Actions Module | 5 | Small |
| 4. Quit/Force Quit Enhancement | 3 | Small |
| 5. Auto Quit Feature | 5 | Small-Medium |
| 6. Uninstaller Enhancement | 5 | Small-Medium |
| 7. UI Integration | 8 | Small-Large |
| 8. Testing | 8 | Medium |
| **Total** | **40** | |

---

## Recommended Implementation Order

### Phase 1: Foundation (Tasks 1.x, 2.x)
Build data models and process detection first as they're dependencies for other features.

### Phase 2: Backend Actions (Tasks 3.x, 4.x, 5.x, 6.x)
Implement all backend functionality before UI integration.

### Phase 3: UI Integration (Tasks 7.x)
Wire up all actions to the UI with indicators and action panel.

### Phase 4: Testing (Tasks 8.x)
Comprehensive testing after implementation is complete.

---

## Critical Path

```
Task 1.1 → Task 1.2 → Task 5.1 → Task 5.2 → Task 5.3 → Task 7.2 → Task 7.7
                  ↓
Task 2.1 → Task 2.2 → Task 7.1 → Task 7.3 → Task 7.4
                  ↓
Task 3.1 → Tasks 3.2-3.5 → Task 7.3
                  ↓
Task 1.3 → Task 6.1 → Task 6.2 → Task 7.5
```

The critical path runs through the data models, running app detection, and UI integration for the action panel.
