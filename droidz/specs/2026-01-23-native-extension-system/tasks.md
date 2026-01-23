# Tasks List for Sprint 6 - Native Extension System

> **Spec:** `/droidz/specs/2026-01-23-native-extension-system/spec.md`  
> **Created:** 2026-01-23  
> **Total Tasks:** 62

---

## Task Group 1: Extension API Crate & ABI (Foundation)

### Task 1.1: Create `photoncast-extension-api` crate structure
- **Description**: Add a new workspace crate with `abi_stable` and initial module layout for the extension API surface.
- **Dependencies**: None
- **Acceptance Criteria**:
  - Crate exists at `crates/photoncast-extension-api/` with `lib.rs` and module stubs
  - Workspace `Cargo.toml` includes the new crate
  - Crate compiles with `abi_stable` dependency enabled
- **Complexity**: Small

### Task 1.2: Define core `Extension` trait and `ExtensionContext`
- **Description**: Add the primary extension trait plus context object (data/cache dirs, host services, preferences, storage, runtime).
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `Extension` trait includes `manifest`, `activate`, `deactivate`, `search_provider`, `commands`
  - `ExtensionContext` exposes data/cache paths and service handles
  - Types are `abi_stable`-compatible
- **Complexity**: Medium

### Task 1.3: Define Search Provider API types
- **Description**: Implement `ExtensionSearchProvider`, `ExtensionSearchItem`, `ExtensionAction`, and `IconSource` types.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `ExtensionSearchProvider::search(query, max_results)` returns items
  - `ExtensionSearchItem` includes title, subtitle, icon, score, actions
  - Types are exported from API crate and ABI-stable
- **Complexity**: Medium

### Task 1.4: Define Command API types
- **Description**: Implement `ExtensionCommand`, `CommandMode`, and `CommandHandler` for extension commands.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `CommandMode` supports `Search`, `View`, `NoView`
  - `ExtensionCommand` includes id, name, keywords, handler
  - Command types are ABI-stable and public
- **Complexity**: Medium

### Task 1.5: Define View Schema types and `ViewHandle`
- **Description**: Add `ExtensionView` enum (List/Detail/Form) and a `ViewHandle` update interface.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - `ExtensionView` covers List/Detail/Form with fields per spec
  - `ViewHandle::update(view)` signature exists
  - Types are ABI-stable and exported
- **Complexity**: Medium

### Task 1.6: Define Storage & Preference API types
- **Description**: Add `PreferenceStore`, preference definitions, and `ExtensionStorage` CRUD interfaces.
- **Dependencies**: Task 1.1
- **Acceptance Criteria**:
  - Preference types support string/number/boolean/secret/select/file/directory
  - Storage supports `get`, `set`, `delete`, `list`
  - Types are ABI-stable and usable from extensions
- **Complexity**: Medium

### Task 1.7: Add ABI-stable entrypoint module
- **Description**: Provide a versioned root module and exported entrypoint function for extensions.
- **Dependencies**: Tasks 1.2–1.6
- **Acceptance Criteria**:
  - Root module includes API version constant
  - Exported symbol is discoverable by host loader
  - ABI checks can validate compatibility via `abi_stable`
- **Complexity**: Medium

---

## Task Group 2: Manifest Parsing & Validation

### Task 2.1: Implement manifest structs and TOML parsing
- **Description**: Add host-side manifest structs and parse `extension.toml` into typed data.
- **Dependencies**: None
- **Acceptance Criteria**:
  - `extension.toml` is parsed into a manifest struct
  - Required fields in `[extension]`, `[entry]`, and `[[commands]]` are handled
  - Parse errors include file path and line context
- **Complexity**: Medium

### Task 2.2: Validate manifest rules
- **Description**: Implement validation for reverse-DNS id, SemVer, API version support, entry path existence, and unique command IDs.
- **Dependencies**: Task 2.1
- **Acceptance Criteria**:
  - Invalid `extension.id` rejected
  - Non-SemVer `version` rejected
  - Unsupported `api_version` rejected
  - `entry.path` must exist and be `.dylib`
  - Duplicate command IDs rejected
- **Complexity**: Medium

### Task 2.3: Add `ManifestError` types
- **Description**: Provide typed errors for parse/validation with actionable messages.
- **Dependencies**: Tasks 2.1–2.2
- **Acceptance Criteria**:
  - Error enum covers parse, IO, and validation failures
  - Errors include field name and reason
  - Errors are surfaced in host logs and UI where applicable
- **Complexity**: Small

### Task 2.4: Implement manifest loader with caching
- **Description**: Load manifests from extension directories and cache results in memory.
- **Dependencies**: Tasks 2.1–2.3
- **Acceptance Criteria**:
  - Loader returns manifests keyed by `extension.id`
  - Cache invalidates when manifest file changes
  - Invalid manifests are recorded with error state
- **Complexity**: Medium

---

## Task Group 3: Extension Discovery & Registry

### Task 3.1: Implement extension discovery paths
- **Description**: Discover extensions in Application Support and optional dev paths with location restrictions.
- **Dependencies**: Task 2.4
- **Acceptance Criteria**:
  - Scans `~/Library/Application Support/PhotonCast/extensions/`
  - Optional dev paths supported via config/env
  - Non-allowed locations are ignored with warnings
- **Complexity**: Small

### Task 3.2: Build `ExtensionRegistry`
- **Description**: Create registry to track manifests, enablement, state, and last errors.
- **Dependencies**: Tasks 2.4, 3.1
- **Acceptance Criteria**:
  - Registry supports add/update/remove/list by id
  - Registry holds current state and last error
  - Registry exposes enabled/disabled status
- **Complexity**: Medium

### Task 3.3: Persist enable/disable state
- **Description**: Store extension enablement in config and restore at startup.
- **Dependencies**: Task 3.2
- **Acceptance Criteria**:
  - Enable/disable changes persist across restarts
  - Disabled extensions are not loaded/activated
  - Config schema handles new extensions gracefully
- **Complexity**: Small

### Task 3.4: Implement lifecycle state machine types
- **Description**: Define `ExtensionState` and valid transitions (Discovered → Loaded → Active, etc.).
- **Dependencies**: Task 3.2
- **Acceptance Criteria**:
  - Enum covers Discovered, Loaded, Active, Disabled, Failed, Unloaded
  - Transition helper enforces valid moves
  - Invalid transitions return typed errors
- **Complexity**: Small

---

## Task Group 4: Extension Loader & Lifecycle Management

### Task 4.1: Implement libloading wrapper
- **Description**: Load extension dylibs and resolve the ABI entrypoint symbol.
- **Dependencies**: Task 1.7
- **Acceptance Criteria**:
  - Dylib loads via `libloading`
  - Entry symbol resolves to ABI root module
  - Loader returns clear errors on failure
- **Complexity**: Medium

### Task 4.2: Add ABI/API version compatibility checks
- **Description**: Validate API version and ABI compatibility during load.
- **Dependencies**: Tasks 1.7, 4.1
- **Acceptance Criteria**:
  - Incompatible `api_version` blocks activation
  - ABI mismatch yields explicit error
  - Compatibility checks are covered by unit tests
- **Complexity**: Medium

### Task 4.3: Implement load + activate pipeline
- **Description**: Create ExtensionManager flow to load, construct context, and call `activate()`.
- **Dependencies**: Tasks 3.2, 4.1, 5.1, 5.2
- **Acceptance Criteria**:
  - State transitions to Loaded → Active on success
  - Providers and commands are registered on activate
  - Activation errors set state to Failed
- **Complexity**: Medium

### Task 4.4: Implement deactivate + unload pipeline
- **Description**: Call `deactivate()`, unregister providers/commands, and unload dylib.
- **Dependencies**: Task 4.3
- **Acceptance Criteria**:
  - Deactivate invoked before unload
  - Providers/commands removed from registries
  - State transitions to Unloaded or Disabled
- **Complexity**: Medium

### Task 4.5: Add failure handling and rate limiting
- **Description**: Mark extensions as Failed on errors and rate-limit repeated failures.
- **Dependencies**: Tasks 3.2, 4.3, 4.4
- **Acceptance Criteria**:
  - Failed extensions are disabled until changed
  - Repeated failures are throttled
  - User receives a toast/notification on failure
- **Complexity**: Medium

---

## Task Group 5: Host Services for Extensions

### Task 5.1: Implement `ExtensionHost` services
- **Description**: Bridge host actions (toast, URL/file open, reveal, clipboard, selected text) to extension API.
- **Dependencies**: Task 1.2
- **Acceptance Criteria**:
  - All host methods in spec are implemented
  - Clipboard read/write works for extensions
  - Errors are surfaced as typed results
- **Complexity**: Medium

### Task 5.2: Wire extension data/cache directories
- **Description**: Provide per-extension data/cache dirs and ensure they exist.
- **Dependencies**: Task 1.2
- **Acceptance Criteria**:
  - Data dir and cache dir are created on first use
  - Paths are namespaced by `extension.id`
  - Paths are injected into `ExtensionContext`
- **Complexity**: Small

### Task 5.3: Implement `ExtensionStorage` (SQLite)
- **Description**: Add namespaced key/value storage backed by SQLite.
- **Dependencies**: Task 1.6
- **Acceptance Criteria**:
  - `get/set/delete/list` implemented
  - Data is isolated by `extension.id`
  - Storage is accessible via `ExtensionContext`
- **Complexity**: Medium

### Task 5.4: Implement `PreferenceStore` with Keychain support
- **Description**: Store preferences with typed values and secrets in macOS Keychain.
- **Dependencies**: Task 1.6
- **Acceptance Criteria**:
  - Non-secret values stored in app config
  - Secret values stored/retrieved from Keychain
  - Validation enforces required preferences
- **Complexity**: Medium

### Task 5.5: Permissions summary on enable
- **Description**: Show permissions from manifest when enabling an extension and store acceptance.
- **Dependencies**: Tasks 2.2, 3.3
- **Acceptance Criteria**:
  - Enable flow displays permissions list
  - User acceptance stored in config
  - Extension activation blocked until accepted
- **Complexity**: Small

---

## Task Group 6: Search Provider & Command Integration

### Task 6.1: Register extension search providers
- **Description**: Add extension providers to the search engine registry.
- **Dependencies**: Tasks 1.3, 4.3
- **Acceptance Criteria**:
  - Providers are discoverable in search pipeline
  - Providers receive query and `max_results`
  - Providers can be enabled/disabled dynamically
- **Complexity**: Medium

### Task 6.2: Merge provider results with isolation
- **Description**: Merge extension results with built-ins, enforce per-provider limits, and isolate errors.
- **Dependencies**: Task 6.1
- **Acceptance Criteria**:
  - Results capped by `SearchConfig.max_results_per_provider`
  - Provider errors do not crash search
  - Failed providers are logged and skipped
- **Complexity**: Medium

### Task 6.3: Integrate `ExtensionCommand` into launcher search
- **Description**: Surface commands in global search for `CommandMode::Search`.
- **Dependencies**: Tasks 1.4, 4.3
- **Acceptance Criteria**:
  - Commands appear with keywords and icons
  - Command results behave like built-in actions
  - Disabled extensions’ commands are hidden
- **Complexity**: Medium

### Task 6.4: Implement command execution pipeline
- **Description**: Execute commands for Search/View/NoView modes and pass runtime context.
- **Dependencies**: Task 6.3
- **Acceptance Criteria**:
  - View commands open a host-rendered view
  - NoView commands run actions without UI
  - Errors surface as toasts without crashing host
- **Complexity**: Medium

### Task 6.5: Implement ListView rendering
- **Description**: Render ListView with sections, items, search bar, and empty state.
- **Dependencies**: Tasks 1.5, 6.4
- **Acceptance Criteria**:
  - Sections render with optional titles
  - Items show icon, title, subtitle, accessories
  - Search bar filters items with throttling
  - Empty state renders with icon, title, description, actions
  - Keyboard navigation works (↑↓, Enter, ⌘1-9)
- **Complexity**: Large

### Task 6.6: Implement ListView split-view with Preview
- **Description**: Add split-view mode with preview panel for ListItems.
- **Dependencies**: Task 6.5
- **Acceptance Criteria**:
  - `show_preview: true` enables right panel
  - Preview renders Markdown, Image, or Metadata
  - Selection updates preview in real-time
  - Preview panel respects max dimensions (256x256 images)
- **Complexity**: Medium

### Task 6.7: Implement DetailView rendering
- **Description**: Render DetailView with markdown content and metadata.
- **Dependencies**: Tasks 1.5, 6.4
- **Acceptance Criteria**:
  - Markdown rendered with host styles
  - Metadata items show label/value pairs
  - Links in metadata are clickable
  - Tags render with semantic colors
- **Complexity**: Medium

### Task 6.8: Implement FormView rendering
- **Description**: Render FormView with all field types and validation.
- **Dependencies**: Tasks 1.5, 6.4
- **Acceptance Criteria**:
  - TextField, TextArea, Password, Number, Checkbox implemented
  - Dropdown with options implemented
  - FilePicker and DirectoryPicker use native dialogs
  - DatePicker implemented
  - Validation errors shown inline
  - Submit button with keyboard shortcut (⌘⏎)
- **Complexity**: Large

### Task 6.9: Implement GridView rendering
- **Description**: Render GridView with image items for visual content.
- **Dependencies**: Tasks 1.5, 6.4
- **Acceptance Criteria**:
  - Grid columns configurable (2-6)
  - Items show image, title, subtitle
  - Image sources: Path, URL, Base64, SfSymbol
  - Keyboard navigation works
  - Empty state renders correctly
- **Complexity**: Medium

### Task 6.10: Implement Action system types and registration
- **Description**: Define Action struct and ActionHandler variants.
- **Dependencies**: Task 1.2
- **Acceptance Criteria**:
  - Action struct with id, title, icon, shortcut, style, handler
  - ActionStyle: Default, Destructive, Primary
  - ActionHandler: Callback, OpenUrl, OpenFile, CopyToClipboard, PushView, SubmitForm
  - Shortcut struct with key and modifiers
- **Complexity**: Medium

### Task 6.11: Implement standard Action builders
- **Description**: Add convenience constructors for common actions.
- **Dependencies**: Task 6.10
- **Acceptance Criteria**:
  - `Action::copy()` - copies text, shows toast
  - `Action::open_url()` - opens in browser
  - `Action::open_file()` - opens with default app
  - `Action::reveal_in_finder()` - reveals in Finder
  - `Action::quick_look()` - shows Quick Look preview
  - `Action::delete_with_confirmation()` - confirms before callback
- **Complexity**: Medium

### Task 6.12: Integrate Actions with Cmd+K menu
- **Description**: Render item actions in the existing Cmd+K actions menu.
- **Dependencies**: Tasks 6.5, 6.10
- **Acceptance Criteria**:
  - Cmd+K shows actions for selected item
  - Actions show title, icon, shortcut hint
  - Destructive actions shown in red
  - Primary action highlighted
  - Shortcuts work directly (without opening menu)
- **Complexity**: Medium

### Task 6.13: Implement Navigation API
- **Description**: Add push/pop/replace view navigation for extensions.
- **Dependencies**: Task 6.5
- **Acceptance Criteria**:
  - `push()` adds view to stack with slide animation
  - `pop()` returns to previous view
  - `replace()` swaps current view
  - `pop_to_root()` clears stack
  - Escape/Cmd+[ triggers pop
  - Navigation state preserved per-extension
- **Complexity**: Medium

### Task 6.14: Implement ViewHandle async updates
- **Description**: Allow extensions to update views asynchronously.
- **Dependencies**: Tasks 6.5, 6.13
- **Acceptance Criteria**:
  - `update()` replaces entire view
  - `update_items()` efficiently updates list items
  - `set_loading()` shows/hides loading spinner
  - `set_error()` shows error state
  - Updates are thread-safe, dispatched to main thread
  - Stale handles ignored safely
- **Complexity**: Medium

### Task 6.15: Implement Design System enforcement
- **Description**: Ensure all extension views conform to design constraints.
- **Dependencies**: Tasks 6.5-6.9
- **Acceptance Criteria**:
  - Icons auto-scaled to 16/24/32px
  - Thumbnails capped at 64x64 (list) / 256x256 (preview)
  - Text truncated with ellipsis
  - TagColor mapped to theme colors
  - Typography uses host font system
  - Animations use host timing (150ms transitions)
- **Complexity**: Medium

---

## Task Group 7: Custom Commands Feature

### Task 7.1: Add custom commands table migration
- **Description**: Add SQLite migration for `custom_commands` table per spec.
- **Dependencies**: None
- **Acceptance Criteria**:
  - Table includes all fields from spec (timeouts, flags, stats)
  - Migration runs cleanly on startup
  - Schema version updated
- **Complexity**: Small

### Task 7.2: Implement CustomCommandStore CRUD
- **Description**: Add create/update/delete/list operations and stats updates.
- **Dependencies**: Task 7.1
- **Acceptance Criteria**:
  - CRUD functions for custom commands
  - `run_count` and `last_run_at` updated on execution
  - Disabled commands are filtered where appropriate
- **Complexity**: Medium

### Task 7.3: Implement placeholder expansion
- **Description**: Expand `{query}`, `{selection}`, `{clipboard}`, `{env:VAR}` before execution.
- **Dependencies**: Task 7.2
- **Acceptance Criteria**:
  - Placeholder expansion covers all spec variants
  - Missing values resolve to empty string safely
  - Unit tests cover each placeholder type
- **Complexity**: Medium

### Task 7.4: Implement command execution pipeline
- **Description**: Run shell commands with timeout, env, cwd, and output capture.
- **Dependencies**: Task 7.3
- **Acceptance Criteria**:
  - Uses `tokio::process::Command` with optional `shell -lc`
  - Timeout enforced (`timeout_ms`)
  - Captures stdout/stderr and exit code
- **Complexity**: Medium

### Task 7.5: Add output view + toast notifications
- **Description**: Show success/failure toasts and an output detail view.
- **Dependencies**: Task 7.4
- **Acceptance Criteria**:
  - Success shows `Toast::Success`
  - Failure shows `Toast::Failure` with "View Output" action
  - Detail view shows command, status, stdout/stderr
- **Complexity**: Medium

### Task 7.6: Implement confirmation flow
- **Description**: Require confirmation when `requires_confirmation` is true.
- **Dependencies**: Task 7.2
- **Acceptance Criteria**:
  - Confirmation prompt appears before execution
  - User cancel prevents command execution
  - Setting is respected per command
- **Complexity**: Small

### Task 7.7: Integrate custom commands into search
- **Description**: Expose custom commands as search results with keywords and alias.
- **Dependencies**: Tasks 7.2, 7.4
- **Acceptance Criteria**:
  - Commands appear in search with alias matching
  - Selecting a command executes with query text
  - Disabled commands are hidden
- **Complexity**: Medium

### Task 7.8: Persist output capture with size limits
- **Description**: Store captured output and enforce 64KB truncation.
- **Dependencies**: Task 7.4
- **Acceptance Criteria**:
  - Output stored in DB or cache per run
  - Output truncated at 64KB with indicator
  - View renders truncated output safely
- **Complexity**: Small

---

## Task Group 8: Hot-Reload Support

### Task 8.1: Add dev mode config + env flag
- **Description**: Gate hot reload with `extensions.dev_mode` or `PHOTONCAST_DEV_EXTENSIONS`.
- **Dependencies**: Task 3.2
- **Acceptance Criteria**:
  - Dev mode toggles hot reload behavior
  - Env var overrides config
  - Dev mode is logged on startup
- **Complexity**: Small

### Task 8.2: Implement file watcher for manifests and dylibs
- **Description**: Watch `extension.toml` and `.dylib` for changes in dev mode.
- **Dependencies**: Task 8.1
- **Acceptance Criteria**:
  - File watcher detects changes reliably
  - Events are debounced to avoid duplicate reloads
  - Only dev-mode extensions are watched
- **Complexity**: Medium

### Task 8.3: Implement versioned cache copy for dylibs
- **Description**: Copy dylibs to `cache/extensions/<id>/<timestamp>.dylib` before load.
- **Dependencies**: Task 4.1
- **Acceptance Criteria**:
  - New dylib is copied to cache path on reload
  - Loader uses cached path to bypass OS caching
  - Cache cleanup removes old versions
- **Complexity**: Medium

### Task 8.4: Implement reload pipeline
- **Description**: Execute `deactivate → unload → load → activate` on change.
- **Dependencies**: Tasks 4.3, 4.4, 8.2, 8.3
- **Acceptance Criteria**:
  - Reload cycles without app restart
  - Providers/commands are re-registered
  - State transitions reflect reload progress
- **Complexity**: Medium

### Task 8.5: Handle reload failure + timing logs
- **Description**: Mark Failed on reload errors and log reload duration.
- **Dependencies**: Task 8.4
- **Acceptance Criteria**:
  - Failed reloads disable extension until next change
  - Reload duration logged (<250ms target)
  - User receives error toast on failure
- **Complexity**: Small

---

## Task Group 9: Reference Extensions

### Task 9.1: Scaffold reference extensions workspace layout
- **Description**: Create extension directories, Cargo configs (cdylib), and base entrypoints.
- **Dependencies**: Tasks 1.1, 1.7
- **Acceptance Criteria**:
  - Each reference extension builds as a cdylib
  - Base entrypoint exports ABI root module
  - Manifests are included per extension
- **Complexity**: Medium

### Task 9.2: GitHub extension manifest + preferences
- **Description**: Add manifest and preferences (`api_token`, `default_org`) for GitHub extension.
- **Dependencies**: Task 9.1
- **Acceptance Criteria**:
  - `extension.toml` matches spec with permissions and commands
  - Preferences are declared with correct types
  - Command metadata is registered
- **Complexity**: Small

### Task 9.3: GitHub extension API client + search
- **Description**: Implement GitHub repo search and map results to list items.
- **Dependencies**: Tasks 9.1, 6.5
- **Acceptance Criteria**:
  - Search uses token when provided
  - Results include name, description, stars, language
  - API errors are handled gracefully
- **Complexity**: Medium

### Task 9.4: GitHub extension actions
- **Description**: Add actions to open/copy repo URLs and open Issues/PRs.
- **Dependencies**: Task 9.3
- **Acceptance Criteria**:
  - Actions: Open in browser, Copy HTTPS, Copy SSH
  - Actions: Open Issues, Open Pull Requests
  - Clipboard actions use host API
- **Complexity**: Small

### Task 9.5: System Preferences manifest + command
- **Description**: Add manifest and command registration for System Preferences extension.
- **Dependencies**: Task 9.1
- **Acceptance Criteria**:
  - Manifest includes `com.photoncast.settings`
  - Command registered with mode `view`
  - No permissions required
- **Complexity**: Small

### Task 9.6: System Preferences list view + deep links
- **Description**: Render settings list and open `x-apple.systempreferences:` links.
- **Dependencies**: Task 9.5
- **Acceptance Criteria**:
  - List includes common panes (Wi‑Fi, Bluetooth, Privacy, Sound)
  - Selecting an item opens the correct deep link
  - View uses extension view schema
- **Complexity**: Medium

### Task 9.7: Screenshot Browser manifest + command
- **Description**: Add manifest and command registration for Screenshot Browser extension.
- **Dependencies**: Task 9.1
- **Acceptance Criteria**:
  - Manifest includes `com.photoncast.screenshots`
  - Permissions include clipboard, filesystem
  - Preferences include `screenshots_folder` (directory type)
  - Command `Browse Screenshots` registered with mode `view`
- **Complexity**: Small

### Task 9.8: Screenshot Browser file scanning + indexing
- **Description**: Implement folder scanning for screenshot files with metadata extraction.
- **Dependencies**: Task 9.7
- **Acceptance Criteria**:
  - Scans configured folder (default: ~/Desktop) for image files
  - Extracts metadata: filename, date modified, file size
  - Generates thumbnails for list display
  - Supports png, jpg, jpeg, gif, webp extensions
  - Optional subfolder scanning based on preference
- **Complexity**: Medium

### Task 9.9: Screenshot Browser search + sort
- **Description**: Implement search filtering and date-based sorting.
- **Dependencies**: Task 9.8
- **Acceptance Criteria**:
  - Search bar filters by filename (fuzzy match)
  - Results sorted by date (newest first)
  - Search is responsive (<50ms)
- **Complexity**: Small

### Task 9.10: Screenshot Browser split-view UI
- **Description**: Render list with thumbnails and large preview panel.
- **Dependencies**: Task 9.9
- **Acceptance Criteria**:
  - List items show thumbnail, filename, date, size
  - Right panel shows large preview of selected screenshot
  - Preview updates on selection change
  - Uses extension view schema
- **Complexity**: Medium

### Task 9.11: Screenshot Browser actions
- **Description**: Implement actions for screenshot items.
- **Dependencies**: Task 9.10
- **Acceptance Criteria**:
  - Enter copies image to clipboard
  - Open in Preview action
  - Reveal in Finder action
  - Delete with confirmation dialog
  - Quick Look (Cmd+Y) support
- **Complexity**: Medium

### Task 9.12: Screenshot Browser preferences UI
- **Description**: Add preferences for folder path and options.
- **Dependencies**: Task 9.7
- **Acceptance Criteria**:
  - Folder picker for screenshots directory
  - Toggle for including subfolders
  - Multi-select for file extensions to include
  - Preferences persist via extension storage
- **Complexity**: Small

### Task 9.13: Package reference extensions for local install
- **Description**: Ensure install layout (extension.toml, lib/, assets/) for each extension.
- **Dependencies**: Tasks 9.2–9.9
- **Acceptance Criteria**:
  - Each extension produces a dylib in `lib/`
  - Assets and manifest are included in install folder
  - Extensions load via host discovery
- **Complexity**: Small

---

## Task Group 10: Testing & Performance

### Task 10.1: Unit tests for manifest parsing & validation
- **Description**: Add unit tests for parse errors and validation rules.
- **Dependencies**: Tasks 2.1–2.2
- **Acceptance Criteria**:
  - Tests cover invalid IDs, versions, api_version, and entry paths
  - Valid manifests parse successfully
  - Tests run in CI
- **Complexity**: Medium

### Task 10.2: Unit tests for custom command placeholders
- **Description**: Verify `{query}`, `{selection}`, `{clipboard}`, `{env:VAR}` expansion.
- **Dependencies**: Task 7.3
- **Acceptance Criteria**:
  - Each placeholder has tests for happy and missing cases
  - Env var lookup respects missing vars
  - Clipboard/selection fall back to empty string
- **Complexity**: Small

### Task 10.3: Unit tests for ABI/version checks
- **Description**: Test loader compatibility checks with mock root modules.
- **Dependencies**: Task 4.2
- **Acceptance Criteria**:
  - Incompatible API version fails
  - ABI mismatch fails
  - Compatible modules load successfully
- **Complexity**: Medium

### Task 10.4: Integration test for lifecycle state transitions
- **Description**: Load, activate, deactivate, unload a mock extension and assert states.
- **Dependencies**: Tasks 4.3–4.4
- **Acceptance Criteria**:
  - State transitions follow spec
  - Errors set state to Failed
  - Providers/commands are registered/unregistered
- **Complexity**: Medium

### Task 10.5: Integration test for search provider integration
- **Description**: Validate provider results flow and error isolation.
- **Dependencies**: Task 6.2
- **Acceptance Criteria**:
  - Results appear in search list
  - Provider errors do not break other results
  - Result limits enforced
- **Complexity**: Medium

### Task 10.6: Integration test for hot reload
- **Description**: Simulate dylib replacement and verify reload pipeline.
- **Dependencies**: Task 8.4
- **Acceptance Criteria**:
  - Reload triggers on file change
  - New behavior is reflected without restart
  - Failures mark extension as Failed
- **Complexity**: Medium

### Task 10.7: Performance benchmark for extension load and search
- **Description**: Benchmark load (<50ms) and provider search (<20ms) targets.
- **Dependencies**: Tasks 4.3, 6.2
- **Acceptance Criteria**:
  - Benchmarks report load and search durations
  - Results are logged with thresholds
  - Failing thresholds are flagged in test output
- **Complexity**: Medium

### Task 10.8: Performance benchmark for hot reload
- **Description**: Benchmark hot reload cycle (<250ms) in dev mode.
- **Dependencies**: Task 8.4
- **Acceptance Criteria**:
  - Reload timing recorded end-to-end
  - Threshold check (<250ms) reported
  - Benchmarks run deterministically
- **Complexity**: Small
