# Tasks List for Window Management - Raycast Parity

> **Spec:** `/droidz/specs/2026-01-21-window-management-raycast-parity/spec.md`  
> **Created:** 2026-01-21  
> **Total Tasks:** 55  
> **Estimated Time:** ~6 days

---

## Task Group 1: Crate Setup & Core Models

Foundation tasks for creating the new `photoncast-window` crate and defining core data structures.

### Task 1.1: Create photoncast-window Crate Structure

- **Description**: Create the new `photoncast-window` crate with proper directory structure as specified in Section 5.1.
- **Dependencies**: None
- **Acceptance Criteria**:
  - Crate created at `crates/photoncast-window/`
  - `Cargo.toml` with appropriate dependencies (core-graphics, objc2, etc.)
  - Module files: `lib.rs`, `manager.rs`, `commands.rs`, `accessibility.rs`, `display.rs`, `config.rs`
  - Crate added to workspace `Cargo.toml`
- **Complexity**: Small

### Task 1.2: Define Window Command Types

- **Description**: Create `commands.rs` with all window command enum variants (halves, quarters, thirds, centering, maximize, restore, resize, display move).
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `WindowCommand` enum with all variants as specified
  - Each command has associated display name and default shortcut
  - Commands grouped logically (Halves, Quarters, Thirds, etc.)
- **Complexity**: Small

### Task 1.3: Define Configuration Models

- **Description**: Create `config.rs` with `WindowManagementConfig` struct for user preferences (gap, cycle timeout, respect menu bar/dock).
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `WindowManagementConfig` with fields: `window_gap: u32`, `respect_menu_bar: bool`, `respect_dock: bool`, `cycle_timeout_ms: u64`
  - TOML serialization/deserialization implemented
  - Sensible defaults (gap: 0, respect both: true, timeout: 500ms)
- **Complexity**: Small

### Task 1.4: Define CGRect and ScreenBounds Types

- **Description**: Define core geometry types for window frame calculations including `CGRect` wrapper and `ScreenBounds` struct.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `CGRect` struct with `x`, `y`, `width`, `height` fields
  - `ScreenBounds` struct with convenience methods
  - Conversion traits for interop with macOS types
- **Complexity**: Small

---

## Task Group 2: Accessibility API Integration

Implement macOS Accessibility API wrappers for window manipulation.

### Task 2.1: Implement Accessibility Permission Check

- **Description**: Implement `check_accessibility_permission()` and `request_accessibility_permission()` functions using `AXIsProcessTrusted` and `AXIsProcessTrustedWithOptions`.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `check_accessibility_permission() -> bool` returns current permission state
  - `request_accessibility_permission()` prompts user if not granted
  - Opens System Preferences → Privacy → Accessibility when needed
- **Complexity**: Medium

### Task 2.2: Implement Get Focused Window

- **Description**: Implement function to get the currently focused window using `AXUIElementCopyAttributeValue(kAXFocusedWindowAttribute)`.
- **Dependencies**: Task 2.1
- **Acceptance Criteria**:
  - `get_focused_window() -> Option<AXWindow>` function
  - Returns None if no window is focused or permission denied
  - Includes process ID of owning application
- **Complexity**: Medium

### Task 2.3: Implement Get Window Frame

- **Description**: Implement function to get current window position and size using `kAXPositionAttribute` and `kAXSizeAttribute`.
- **Dependencies**: Task 2.2, Task 1.4
- **Acceptance Criteria**:
  - `get_window_frame(window: &AXWindow) -> Result<CGRect>`
  - Correctly handles coordinate system (top-left origin)
  - Returns error if window is inaccessible
- **Complexity**: Small

### Task 2.4: Implement Set Window Frame

- **Description**: Implement function to set window position and size using `AXUIElementSetAttributeValue` with AppleScript fallback for apps that reject AX resize.
- **Dependencies**: Task 2.3
- **Acceptance Criteria**:
  - `set_window_frame(window: &AXWindow, frame: CGRect) -> Result<()>`
  - Uses Rectangle-style approach: size → position → size (some apps need this order)
  - Temporarily disables `AXEnhancedUserInterface` if enabled (blocks resize in some apps)
  - Falls back to System Events AppleScript for apps that reject AX resize (e.g., Ghostty, some Electron apps)
  - Returns error only if position setting fails (resize failures are tolerated with fallback)
- **Complexity**: Medium

### Task 2.5: Implement Toggle Fullscreen

- **Description**: Implement fullscreen toggle using `kAXFullscreenAttribute`.
- **Dependencies**: Task 2.2
- **Acceptance Criteria**:
  - `toggle_fullscreen(window: &AXWindow) -> Result<()>`
  - Enters native macOS fullscreen mode (separate Space)
  - `is_fullscreen(window: &AXWindow) -> bool` helper function
- **Complexity**: Small

---

## Task Group 3: Display Management

Implement multi-display detection and bounds calculation.

### Task 3.1: Implement Display Enumeration

- **Description**: Implement function to get all connected displays using `CGGetActiveDisplayList` or `NSScreen.screens`.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `get_displays() -> Vec<Display>` function
  - Returns displays in System Preferences arrangement order
  - Includes display ID, name, and main display flag
- **Complexity**: Medium

### Task 3.2: Implement Display Bounds Calculation

- **Description**: Implement function to get display bounds including visible frame (excluding menu bar and dock).
- **Dependencies**: Task 3.1, Task 1.4
- **Acceptance Criteria**:
  - `get_display_bounds(display: &Display) -> ScreenBounds`
  - `get_visible_bounds(display: &Display) -> ScreenBounds` (excludes menu bar/dock)
  - Correctly handles dock on different sides (bottom/left/right)
- **Complexity**: Medium

### Task 3.3: Implement Display for Window

- **Description**: Implement function to determine which display a window is currently on.
- **Dependencies**: Task 3.1, Task 2.3
- **Acceptance Criteria**:
  - `get_display_for_window(window: &AXWindow) -> Option<Display>`
  - Determines display based on window center point
  - Handles windows spanning multiple displays
- **Complexity**: Small

### Task 3.4: Implement Next/Previous Display Logic

- **Description**: Implement logic to get next/previous display in arrangement order with wraparound.
- **Dependencies**: Task 3.1
- **Acceptance Criteria**:
  - `get_next_display(current: &Display) -> Display`
  - `get_previous_display(current: &Display) -> Display`
  - Wraps around at first/last display
- **Complexity**: Small

---

## Task Group 4: Frame Calculations

Implement frame calculation logic for all window positions.

### Task 4.1: Implement ScreenBounds Helper Methods

- **Description**: Add helper methods to `ScreenBounds` for applying gaps and basic coordinate math.
- **Dependencies**: Task 1.4, Task 1.3
- **Acceptance Criteria**:
  - `visible_bounds(&self, gap: u32) -> ScreenBounds` applies gaps
  - `center_point(&self) -> (f64, f64)` returns center coordinates
  - Gap subtracted from all edges
- **Complexity**: Small

### Task 4.2: Implement Halves Calculations

- **Description**: Implement frame calculations for left/right/top/bottom halves.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - `left_half(&self) -> CGRect` - left 50% of screen
  - `right_half(&self) -> CGRect` - right 50% of screen
  - `top_half(&self) -> CGRect` - top 50% of screen
  - `bottom_half(&self) -> CGRect` - bottom 50% of screen
  - All respect configured gap
- **Complexity**: Small

### Task 4.3: Implement Quarters Calculations

- **Description**: Implement frame calculations for all four corners.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - `top_left_quarter(&self) -> CGRect` - top-left 25%
  - `top_right_quarter(&self) -> CGRect` - top-right 25%
  - `bottom_left_quarter(&self) -> CGRect` - bottom-left 25%
  - `bottom_right_quarter(&self) -> CGRect` - bottom-right 25%
  - All respect configured gap
- **Complexity**: Small

### Task 4.4: Implement Thirds Calculations

- **Description**: Implement frame calculations for thirds (33%) and two-thirds (66%) positions.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - `first_third(&self) -> CGRect` - left 33%
  - `center_third(&self) -> CGRect` - center 33%
  - `last_third(&self) -> CGRect` - right 33%
  - `first_two_thirds(&self) -> CGRect` - left 66%
  - `last_two_thirds(&self) -> CGRect` - right 66%
  - All respect configured gap
- **Complexity**: Small

### Task 4.5: Implement Centering Calculations

- **Description**: Implement frame calculations for centering windows at various sizes.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - `center(current_size: (f64, f64)) -> CGRect` - center at current size
  - `center_half(&self) -> CGRect` - center at 50% screen width
  - `center_two_thirds(&self) -> CGRect` - center at 66% screen width
  - Height preserved or calculated proportionally
- **Complexity**: Small

### Task 4.6: Implement Maximize Calculations

- **Description**: Implement frame calculations for maximize and almost maximize.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - `maximize(&self) -> CGRect` - fills visible screen area
  - `almost_maximize(&self, margin: u32) -> CGRect` - fills with margin (default 20px)
  - Does not enter macOS native fullscreen
- **Complexity**: Small

### Task 4.7: Implement Reasonable Size Calculation

- **Description**: Implement the "Reasonable Size" algorithm as specified in Section 3.6.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - Target: 75% width, 80% height of screen
  - Minimum: 800x600
  - Maximum: screen size minus margins
  - Window centered after resize
- **Complexity**: Small

### Task 4.8: Implement Resize Calculations

- **Description**: Implement frame calculations for make smaller/larger commands.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - `make_smaller(current: CGRect, percent: f64) -> CGRect` - shrink by percentage
  - `make_larger(current: CGRect, percent: f64, max: ScreenBounds) -> CGRect` - grow by percentage
  - Resize from center (window stays centered)
  - Minimum size: 400x300
- **Complexity**: Small

---

## Task Group 5: Window Manager Implementation

Implement the core WindowManager struct and command execution.

### Task 5.1: Create WindowManager Struct

- **Description**: Create the main `WindowManager` struct with state for cycling, history, and configuration.
- **Dependencies**: Task 1.2, Task 1.3
- **Acceptance Criteria**:
  - `WindowManager` struct with `config`, `last_command`, `window_history` fields
  - `new(config: WindowManagementConfig) -> Self` constructor
  - Thread-safe (can be shared across async contexts)
- **Complexity**: Small

### Task 5.2: Implement Window History Storage

- **Description**: Implement `WindowHistory` struct for storing previous window frames for restore functionality.
- **Dependencies**: Task 5.1, Task 1.4
- **Acceptance Criteria**:
  - `WindowHistory` with `HashMap<WindowId, Vec<SavedFrame>>`
  - `save(window_id, frame)` stores frame with timestamp
  - `restore(window_id) -> Option<CGRect>` pops last saved frame
  - Maximum 10 frames per window
- **Complexity**: Small

### Task 5.3: Implement Cycle Detection

- **Description**: Implement logic to detect when the same command is pressed repeatedly for cycling.
- **Dependencies**: Task 5.1
- **Acceptance Criteria**:
  - `should_cycle(&self, command: WindowCommand) -> bool`
  - Returns true if same command within timeout (default 500ms)
  - `last_command` field updated on each command execution
- **Complexity**: Small

### Task 5.4: Implement Cycle State Tracking

- **Description**: Implement tracking of current cycle position for each window.
- **Dependencies**: Task 5.3
- **Acceptance Criteria**:
  - Tracks cycle position per window per command type
  - `get_cycle_index(window_id, command) -> usize`
  - `advance_cycle(window_id, command)` increments position
  - `reset_cycle(window_id, command)` on new command or timeout
- **Complexity**: Medium

### Task 5.5: Implement Execute Command

- **Description**: Implement main `execute_command(command: WindowCommand)` method that routes to specific handlers.
- **Dependencies**: Tasks 5.1-5.4, Tasks 2.2-2.4, Tasks 3.2-3.3, Tasks 4.2-4.8
- **Acceptance Criteria**:
  - Gets focused window
  - Gets current display and bounds
  - Saves current frame to history before modification
  - Calculates new frame based on command
  - Applies new frame
  - Updates cycle tracking
- **Complexity**: Medium

---

## Task Group 6: Window Commands Implementation

Implement individual command handlers.

### Task 6.1: Implement Halves Commands

- **Description**: Implement Left Half, Right Half, Top Half, Bottom Half commands with cycling support.
- **Dependencies**: Task 5.5, Task 4.2
- **Acceptance Criteria**:
  - All four half commands functional
  - Cycling: Half → Third → Two Thirds → Half
  - Smooth transition when cycling
- **Complexity**: Medium

### Task 6.2: Implement Quarters Commands

- **Description**: Implement Top Left, Top Right, Bottom Left, Bottom Right quarter commands.
- **Dependencies**: Task 5.5, Task 4.3
- **Acceptance Criteria**:
  - All four quarter commands functional
  - No cycling for quarters (fixed 25% size)
- **Complexity**: Small

### Task 6.3: Implement Thirds Commands

- **Description**: Implement First Third, Center Third, Last Third, First Two Thirds, Last Two Thirds commands.
- **Dependencies**: Task 5.5, Task 4.4
- **Acceptance Criteria**:
  - All five thirds commands functional
  - Works well on ultrawide monitors
- **Complexity**: Small

### Task 6.4: Implement Centering Commands

- **Description**: Implement Center, Center Half, Center Two Thirds commands with cycling support.
- **Dependencies**: Task 5.5, Task 4.5
- **Acceptance Criteria**:
  - All three centering commands functional
  - Center cycling: Current Size → Half → Two Thirds → Current Size
  - Window stays centered when cycling
- **Complexity**: Small

### Task 6.5: Implement Maximize Commands

- **Description**: Implement Maximize and Almost Maximize commands.
- **Dependencies**: Task 5.5, Task 4.6
- **Acceptance Criteria**:
  - Maximize fills visible screen area (menu bar/dock remain)
  - Almost Maximize leaves configurable margin (default 20px)
  - Works correctly with dock on any side
- **Complexity**: Small

### Task 6.6: Implement Toggle Fullscreen Command

- **Description**: Implement Toggle Fullscreen command using accessibility API.
- **Dependencies**: Task 5.5, Task 2.5
- **Acceptance Criteria**:
  - Enters/exits native macOS fullscreen mode
  - Window moves to its own Space
  - Menu bar and dock hidden in fullscreen
- **Complexity**: Small

### Task 6.7: Implement Restore Command

- **Description**: Implement Restore command to return window to previous size/position.
- **Dependencies**: Task 5.5, Task 5.2
- **Acceptance Criteria**:
  - Restores to exact previous frame from history
  - If no history, centers window at "reasonable size"
  - Multiple restores step back through history
- **Complexity**: Small

### Task 6.8: Implement Reasonable Size Command

- **Description**: Implement Reasonable Size command using the algorithm from spec.
- **Dependencies**: Task 5.5, Task 4.7
- **Acceptance Criteria**:
  - Applies reasonable size algorithm
  - Window centered after resize
  - Respects minimum (800x600) and maximum bounds
- **Complexity**: Small

### Task 6.9: Implement Resize Commands

- **Description**: Implement Make Smaller and Make Larger commands.
- **Dependencies**: Task 5.5, Task 4.8
- **Acceptance Criteria**:
  - Make Smaller shrinks by 10%
  - Make Larger grows by 10%
  - Resize from center
  - Respects min (400x300) and max (screen size) bounds
- **Complexity**: Small

---

## Task Group 7: Multi-Display Support

Implement window movement between displays.

### Task 7.1: Implement Position Preservation Logic

- **Description**: Implement logic to calculate relative position on source display and apply to target display.
- **Dependencies**: Task 3.2, Task 1.4
- **Acceptance Criteria**:
  - Calculate relative position (0.0-1.0) on current display
  - Apply relative position to target display bounds
  - Window maintains visual position (e.g., left half stays left half)
- **Complexity**: Medium

### Task 7.2: Implement Move to Next Display

- **Description**: Implement command to move focused window to next display in arrangement order.
- **Dependencies**: Task 7.1, Task 3.4, Task 5.5
- **Acceptance Criteria**:
  - Window moves to next display
  - Relative position preserved
  - Wraps to first display after last
- **Complexity**: Small

### Task 7.3: Implement Move to Previous Display

- **Description**: Implement command to move focused window to previous display in arrangement order.
- **Dependencies**: Task 7.1, Task 3.4, Task 5.5
- **Acceptance Criteria**:
  - Window moves to previous display
  - Relative position preserved
  - Wraps to last display after first
- **Complexity**: Small

---

## Task Group 8: Global Hotkey Registration

Integrate window commands with the global hotkey system.

### Task 8.1: Define Default Hotkey Mappings

- **Description**: Define default keyboard shortcuts for all window commands as specified in Section 3.
- **Dependencies**: Task 1.2
- **Acceptance Criteria**:
  - All shortcuts defined in configuration
  - Follows spec defaults (⌃⌥← for Left Half, etc.)
  - User can override in preferences
- **Complexity**: Small

### Task 8.2: Implement Hotkey Registration

- **Description**: Register all window management hotkeys with the main app's hotkey system.
- **Dependencies**: Task 8.1, Task 5.5
- **Acceptance Criteria**:
  - All hotkeys registered on app launch
  - Hotkeys trigger corresponding WindowManager commands
  - Hotkeys work globally (not just when PhotonCast focused)
- **Complexity**: Medium

### Task 8.3: Implement Hotkey Customization

- **Description**: Allow users to customize hotkeys for window management commands.
- **Dependencies**: Task 8.2
- **Acceptance Criteria**:
  - Custom hotkeys persisted in preferences
  - Hotkey conflicts detected and reported
  - UI for changing hotkeys in Preferences
- **Complexity**: Medium

---

## Task Group 9: Search Integration

Add window management commands to the search index.

### Task 9.1: Define Search Entries for Commands

- **Description**: Create search index entries for all window management commands.
- **Dependencies**: Task 1.2
- **Acceptance Criteria**:
  - All commands appear in search (e.g., "left half", "maximize")
  - Includes command icon, name, description, and shortcut
  - Aliases for common variations (e.g., "snap left" → "Left Half")
- **Complexity**: Small

### Task 9.2: Implement Command Execution from Search

- **Description**: Implement search result action that executes the window command.
- **Dependencies**: Task 9.1, Task 5.5
- **Acceptance Criteria**:
  - Selecting search result executes command
  - PhotonCast window hidden before execution
  - Focused window before PhotonCast opened is targeted
- **Complexity**: Medium

### Task 9.3: Add Command Icons

- **Description**: Implement icons for window management commands in search results.
- **Dependencies**: Task 9.1
- **Acceptance Criteria**:
  - Icons match spec (◧ for halves, ◰ for quarters, etc.)
  - Icons rendered clearly at search result size
  - Consistent visual style with other search results
- **Complexity**: Small

---

## Task Group 10: UI Integration

Integrate window management with preferences and visual feedback.

### Task 10.1: Create Window Management Preferences Section

- **Description**: Add Window Management section to Preferences with gap, timeout, and shortcut settings.
- **Dependencies**: Task 1.3, Task 8.3
- **Acceptance Criteria**:
  - Gap setting (0-50px slider)
  - Cycle timeout setting
  - Respect menu bar/dock toggles
  - Shortcut customization UI
- **Complexity**: Medium

### Task 10.2: Implement Visual Feedback Overlay (Optional)

- **Description**: Show brief overlay indicating target position when command executes.
- **Dependencies**: Task 5.5
- **Acceptance Criteria**:
  - Overlay appears for 200ms
  - Blue highlight on target area
  - Dimmed overlay on rest of screen
  - Can be disabled in preferences
- **Complexity**: Large

### Task 10.3: Implement Permission Request UI

- **Description**: Create UI flow for requesting accessibility permission on first use.
- **Dependencies**: Task 2.1
- **Acceptance Criteria**:
  - Detects when permission not granted
  - Shows explanation dialog
  - Button to open System Preferences
  - Re-checks permission after user grants
- **Complexity**: Medium

---

## Task Group 11: Testing

Comprehensive testing for all window management functionality.

### Task 11.1: Unit Tests - Frame Calculations

- **Description**: Write unit tests for all frame calculation methods (halves, quarters, thirds, centering, maximize).
- **Dependencies**: Tasks 4.2-4.8
- **Acceptance Criteria**:
  - `test_frame_calculation_halves` - all four half positions
  - `test_frame_calculation_quarters` - all four quarter positions
  - `test_frame_calculation_thirds` - all five thirds positions
  - `test_frame_calculation_centering` - all three centering options
  - `test_frame_calculation_with_gap` - gaps applied correctly
- **Complexity**: Medium

### Task 11.2: Unit Tests - Cycle Detection

- **Description**: Write unit tests for cycle detection and state management.
- **Dependencies**: Tasks 5.3-5.4
- **Acceptance Criteria**:
  - `test_cycle_detection` - detects repeated commands within timeout
  - `test_cycle_timeout` - cycle resets after timeout
  - `test_cycle_different_command` - cycle resets on different command
  - `test_cycle_advancement` - cycle index increments correctly
- **Complexity**: Medium

### Task 11.3: Unit Tests - Window History

- **Description**: Write unit tests for window history save/restore functionality.
- **Dependencies**: Task 5.2
- **Acceptance Criteria**:
  - `test_history_save` - frames saved correctly
  - `test_history_restore` - returns most recent frame
  - `test_history_max_limit` - old frames evicted after limit
  - `test_history_multiple_windows` - separate history per window
- **Complexity**: Small

### Task 11.4: Unit Tests - Multi-Display

- **Description**: Write unit tests for multi-display position calculation.
- **Dependencies**: Task 7.1
- **Acceptance Criteria**:
  - `test_relative_position_calculation` - correct 0.0-1.0 values
  - `test_position_preservation` - position matches on new display
  - `test_display_wraparound` - next/previous wraps correctly
- **Complexity**: Medium

### Task 11.5: Integration Tests - Accessibility

- **Description**: Write integration tests for accessibility permission check and window manipulation.
- **Dependencies**: Tasks 2.1-2.4
- **Acceptance Criteria**:
  - `test_accessibility_permission_check` - returns correct status
  - `test_get_focused_window` - returns window or None
  - `test_window_frame_get_set` - can read and write frame
- **Complexity**: Medium

### Task 11.6: Integration Tests - Window Commands

- **Description**: Write integration tests that actually move test windows.
- **Dependencies**: Tasks 6.1-6.9
- **Acceptance Criteria**:
  - `test_window_move_left_half` - window moves to left half
  - `test_window_maximize` - window fills visible area
  - `test_window_restore` - window returns to previous position
  - `test_window_cycle` - cycling through sizes works
- **Complexity**: Medium

### Task 11.7: Integration Tests - Multi-Display

- **Description**: Write integration tests for moving windows between displays (requires multi-display setup).
- **Dependencies**: Tasks 7.2-7.3
- **Acceptance Criteria**:
  - `test_move_to_next_display` - window moves to next display
  - `test_move_to_previous_display` - window moves to previous display
  - `test_position_preserved` - relative position maintained
- **Complexity**: Medium

### Task 11.8: Integration Tests - Search

- **Description**: Write integration tests for window commands appearing in search and executing correctly.
- **Dependencies**: Tasks 9.1-9.2
- **Acceptance Criteria**:
  - `test_window_commands_in_search` - commands appear in results
  - `test_command_execution_from_search` - selecting result executes command
  - `test_search_aliases` - alternative names work
- **Complexity**: Small

---

## Summary

| Group | Tasks | Complexity |
|-------|-------|------------|
| 1. Crate Setup & Core Models | 4 | Small |
| 2. Accessibility API Integration | 5 | Small-Medium |
| 3. Display Management | 4 | Small-Medium |
| 4. Frame Calculations | 8 | Small |
| 5. Window Manager Implementation | 5 | Small-Medium |
| 6. Window Commands Implementation | 9 | Small-Medium |
| 7. Multi-Display Support | 3 | Small-Medium |
| 8. Global Hotkey Registration | 3 | Small-Medium |
| 9. Search Integration | 3 | Small-Medium |
| 10. UI Integration | 3 | Medium-Large |
| 11. Testing | 8 | Small-Medium |
| **Total** | **55** | |

---

## Recommended Implementation Order

### Phase 1: Foundation (Day 1)
**Tasks:** 1.1-1.4, 2.1-2.5

Build the crate structure, core models, and accessibility API wrappers first as they're the foundation for everything else.

### Phase 2: Display & Calculations (Day 2)
**Tasks:** 3.1-3.4, 4.1-4.8

Implement display management and all frame calculations. These are pure functions that can be thoroughly unit tested.

### Phase 3: Core Manager & Commands (Days 3-4)
**Tasks:** 5.1-5.5, 6.1-6.9, 7.1-7.3

Implement the WindowManager and all individual commands. Multi-display support included.

### Phase 4: Integration (Day 5)
**Tasks:** 8.1-8.3, 9.1-9.3, 10.1, 10.3

Integrate with global hotkeys, search index, and preferences UI.

### Phase 5: Testing & Polish (Day 6)
**Tasks:** 11.1-11.8, 10.2 (optional)

Comprehensive testing and optional visual feedback overlay.

---

## Critical Path

```
Task 1.1 → Task 1.2 ─────────────────────────────────────────────────┐
    │                                                                 │
    ├─→ Task 1.3 → Task 4.1 → Tasks 4.2-4.8 ────────────────────┐   │
    │                                                            │   │
    ├─→ Task 1.4 → Task 2.3 → Task 2.4 ──────────────────────────┤   │
    │                                                            │   │
    └─→ Task 2.1 → Task 2.2 → Task 3.1 → Tasks 3.2-3.4 ─────────┤   │
                                                                 │   │
                                                                 ▼   ▼
                                                           Task 5.1 → Task 5.5
                                                                 │
                                                                 ▼
                                                     Tasks 6.1-6.9, 7.1-7.3
                                                                 │
                                                                 ▼
                                              Tasks 8.1-8.3, 9.1-9.3, 10.1-10.3
```

The critical path runs through:
1. Crate setup and core types
2. Accessibility APIs (permission and window frame manipulation)
3. Display management
4. Frame calculations
5. Window Manager and command execution
6. Integration with hotkeys and search

---

## Dependencies Graph (Simplified)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Task 1.1 (Crate Setup)                          │
└───────────────┬─────────────────┬────────────────────┬─────────────────┘
                │                 │                    │
                ▼                 ▼                    ▼
         Task 1.2           Task 1.3             Task 1.4
         (Commands)         (Config)             (Types)
                │                 │                    │
                │                 ▼                    ▼
                │           Task 4.1 ───────────► Task 2.3
                │           (Bounds)              (Get Frame)
                │                 │                    │
                ▼                 ▼                    ▼
         Task 8.1 ◄──────── Tasks 4.2-4.8         Task 2.4
         (Hotkeys)          (Calculations)        (Set Frame)
                │                 │                    │
                │                 └────────┬───────────┘
                │                          │
                │                          ▼
                │                    Task 5.1-5.5
                │                   (WindowManager)
                │                          │
                └──────────────────────────┼───────────────────────────────┐
                                           │                               │
                                           ▼                               ▼
                                    Tasks 6.1-6.9                    Tasks 9.1-9.3
                                    (Commands)                       (Search)
                                           │                               │
                                           ▼                               │
                                    Tasks 7.1-7.3                          │
                                    (Multi-Display)                        │
                                           │                               │
                                           └───────────────────────────────┘
                                                          │
                                                          ▼
                                                   Tasks 11.1-11.8
                                                    (Testing)
```

---

*End of Tasks List*
