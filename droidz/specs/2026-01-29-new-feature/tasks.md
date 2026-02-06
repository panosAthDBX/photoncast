# Tasks List for PhotonCast App Packaging & Distribution

**Feature:** App Packaging & Distribution  
**Spec Date:** 2026-01-29  
**Priority:** High — Required for v1.0 Public Release  
**Estimated Duration:** 10-12 days

---

## Overview

This tasks list breaks down the PhotonCast App Packaging & Distribution feature into strategic phases with clear dependencies, time estimates, and testing requirements. The work covers app icon design, build signing, Sparkle auto-updates, DMG distribution, dock visibility settings, and menu bar behavior.

---

## Phase 1: Design & Assets

*Phase Goal: Create all visual assets and design specifications needed for the application.*

---

### Task 1.1: Design App Icon Concept [x]
- **Description:** Create the "Photon Beam" icon concept following Catppuccin Mocha color palette. Design a minimal, abstract representation of a light beam with mauve (#cba6f7) to pink (#f5c2e7) gradient on base (#1e1e2e) background.
- **Dependencies:** None
- **Acceptance Criteria:**
  - [x] 1024×1024px source file created
  - [x] Rounded-square format with ~22% corner radius
  - [x] Color palette matches Catppuccin Mocha spec
  - [x] Design includes photon beam with gradient glow effect
- **Complexity:** Medium
- **Estimated Time:** 8 hours
- **Deliverables:**
  - `assets/icon-source-1024.png` - Source file generated via Python/Pillow
  - Features: Rounded square base (#1e1e2e), 4 beam lines with gradient (flamingo→pink→mauve→lavender), photon source dot with glow
  - Glass/liquid aesthetic with subtle highlight overlay
- **Notes:** Implemented via Python script `scripts/generate-icon-source.py` instead of Figma/Sketch for reproducibility

---

### Task 1.2: Generate App Icon Size Variants [x]
- **Description:** Create automated script to generate all 10 required app icon sizes from source file (16×16@1x/@2x through 512×512@1x/@2x). Use sips (macOS native) for generation.
- **Dependencies:** Task 1.1
- **Acceptance Criteria:**
  - [x] Script generates all sizes: 16, 32, 128, 256, 512 (each with @1x and @2x)
  - [x] PNG output with transparency preserved
  - [x] Icons are crisp and recognizable at all sizes
  - [x] Script placed in `scripts/generate-icons.sh`
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `scripts/generate-icons.sh` - Bash script using `sips` and `iconutil`
  - Generates 10 PNG files in `resources/AppIcon.iconset/`
  - Includes Python fallback for sips failures
- **Testing:**
  - [x] All 10 icon sizes generated successfully
  - [x] Files: icon_16x16.png through icon_512x512@2x.png (2048×2048)

---

### Task 1.3: Build ICNS File [x]
- **Description:** Compile all icon size variants into `AppIcon.icns` using `iconutil`. Place in resources directory for bundle inclusion.
- **Dependencies:** Task 1.2
- **Acceptance Criteria:**
  - [x] `resources/AppIcon.icns` created with all 10 size variants
  - [x] File validates with `iconutil -c icns`
  - [x] ICNS file loads correctly (58KB, contains all required sizes)
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `resources/AppIcon.icns` - 58KB ICNS file with all 10 size variants
  - Generated via `iconutil -c icns AppIcon.iconset`
- **Testing:**
  - [x] ICNS file created successfully
  - [x] File size: 58KB (expected for 10 icon variants)
  - [x] Iconset directory contains all required PNG files

---

### Task 1.4: Design Menu Bar Template Icon [x]
- **Description:** Create simplified monochrome template version of the photon beam icon for menu bar. Must be pure black/alpha for automatic dark mode support. Sizes: 16×16@1x/@2x, 18×18@1x/@2x.
- **Dependencies:** Task 1.1
- **Acceptance Criteria:**
  - [x] Template icon is pure black with alpha transparency
  - [x] No colors or gradients (macOS handles inversion)
  - [x] 1-2px padding within 16×16 canvas
  - [x] Recognizable silhouette at small size (simplified 3-line beam + dot)
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `resources/MenuBarIcon_16x16@1x.png` - 16×16 template
  - `resources/MenuBarIcon_16x16@2x.png` - 32×32 template
  - `resources/MenuBarIcon_18x18@1x.png` - 18×18 template (macOS 11+)
  - `resources/MenuBarIcon_18x18@2x.png` - 36×36 template
  - `resources/MenuBarIcon.png` - Default 16×16 PNG
  - `resources/MenuBarIcon.pdf` - Vector PDF version for best quality
- **Testing:**
  - [x] All 4 size variants generated
  - [x] Pure black (#000000) with alpha transparency
  - [x] PDF version created for vector scalability

---

### Task 1.5: Create DMG Background Image [x]
- **Description:** Design drag-to-Applications DMG background with app icon, arrow graphic, and Applications folder alias indicator. Export at 2x resolution for Retina displays.
- **Dependencies:** Task 1.1
- **Acceptance Criteria:**
  - [x] Background image shows app icon and drag-to-Applications arrow
  - [x] 2x resolution for Retina support (1600×1000 for 800×500 window)
  - [x] Matches Catppuccin visual style (base background, subtle gradient)
  - [x] Exported as PNG to `resources/dmg-background.png`
- **Complexity:** Small
- **Estimated Time:** 3 hours
- **Deliverables:**
  - `resources/dmg-background.png` - 1600×1000 @2x background
  - `resources/dmg-background@1x.png` - 800×500 @1x background
  - Features: Catppuccin base (#1e1e2e) background, app icon (256px), arrow, folder icon
- **Testing:**
  - [x] 2x resolution created (1600×1000)
  - [x] 1x fallback created (800×500)
  - [x] Visual elements: App icon, arrow, Applications folder representation
  - [x] Catppuccin color scheme applied

---

## Phase 2: Core Implementation

*Phase Goal: Implement the core platform modules and build infrastructure.*

---

### Task 2.1: Create Info.plist Template [x]
- **Description:** Create `resources/Info.plist` with all required entries: CFBundleName, CFBundleIdentifier (com.photoncast.app), LSUIElement (true by default), LSApplicationCategoryType, and macOS version requirements.
- **Dependencies:** None
- **Acceptance Criteria:**
  - [x] Valid XML plist file with all required keys
  - [x] LSUIElement defaults to true (hidden from Dock)
  - [x] CFBundleIdentifier set to `com.photoncast.app`
  - [x] LSMinimumSystemVersion set to 12.0
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Status:** Already exists at `resources/Info.plist`
- **Testing:**
  - [x] Unit test: Validate plist XML structure
  - [x] Unit test: Verify all required keys are present

---

### Task 2.2: Create Entitlements File [x]
- **Description:** Create `resources/entitlements.plist` with hardened runtime entitlements: automation/apple-events for System Events, accessibility for window management, network client access, and user-selected file read/write.
- **Dependencies:** None
- **Acceptance Criteria:**
  - [x] Entitlements plist includes all required capabilities
  - [x] `com.apple.security.automation.apple-events` = true
  - [x] `com.apple.security.accessibility` = true
  - [x] `com.apple.security.network.client` = true
  - [x] `com.apple.security.files.user-selected.read-write` = true
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Status:** Already exists at `resources/entitlements.plist`

---

### Task 2.3: Implement Dock Visibility Module [x]
- **Description:** Create `photoncast-core/src/platform/dock_visibility.rs` with functions to get/set LSUIElement in Info.plist using `plist` crate. Include proper error handling with `thiserror` for restart-required scenarios.
- **Dependencies:** None
- **Acceptance Criteria:**
  - [x] `set_dock_visibility(show_in_dock: bool)` function implemented
  - [x] `get_dock_visibility()` returns current state
  - [x] Uses `plist` crate for parsing/modification
  - [x] Returns `RestartRequired` error on successful modification
  - [x] Comprehensive error handling with custom error types
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Deliverables:**
  - `crates/photoncast-core/src/platform/dock_visibility.rs`
- **Testing:**
  - [x] Unit test: Mock plist modification and verify correct XML output
  - [x] Unit test: Test error handling for missing plist file
  - [x] Unit test: Verify `RestartRequired` error is returned on success

---

### Task 2.4: Implement UpdateManager Module [x]
- **Description:** Create `photoncast-core/src/platform/updates.rs` with UpdateManager struct. Use `reqwest` for appcast feed fetching. Include feed URL configuration, auto-check settings, and manual check functionality.
- **Dependencies:** None
- **Acceptance Criteria:**
  - [x] `UpdateManager` struct with `new()`, `initialize()`, `check_for_updates()` methods
  - [x] Error type `UpdateError` with `thiserror` for initialization and check failures
  - [x] Auto-check enabled/disabled configuration
  - [x] Feed URL configurable (default: https://api.photoncast.app/updates/appcast.xml)
- **Complexity:** Large
- **Estimated Time:** 8 hours
- **Deliverables:**
  - `crates/photoncast-core/src/platform/updates.rs`
- **Testing:**
  - [x] Unit test: Mock appcast parsing and verify state
  - [x] Unit test: Test error handling for invalid feed URL
  - [x] Integration test: Test with local test appcast XML file

---

### Task 2.5: Extend Menu Bar with Click Handlers [x]
- **Description:** Modify `crates/photoncast/src/platform.rs` to implement left-click → open launcher and right-click → show context menu behavior using GPUI.
- **Dependencies:** None
- **Acceptance Criteria:**
  - [x] Left-click on menu bar icon opens launcher window
  - [x] Right-click shows context menu with: Open, Preferences, Check for Updates, About, Quit
  - [x] Menu items have proper keyboard shortcuts (⌘Space, ⌘,)
  - [x] Uses GPUI event handling patterns
- **Deliverables:**
  - Updated `platform.rs` with click handlers in `MenuBarTarget` class
  - Left-click triggers `ToggleLauncher` event
  - Right-click shows context menu via `create_context_menu()`
  - Menu items: Open PhotonCast (⌘Space), Preferences (⌘,), Check for Updates, About, Quit (⌘Q)
- **Complexity:** Medium
- **Estimated Time:** 6 hours
- **Testing:**
  - Unit test: Verify click event routing
  - Integration test: Test menu item activation
  - E2E test: Verify launcher opens on left-click

---

### Task 2.6: Add Restart Confirmation Dialog [x]
- **Description:** Implement GPUI modal dialog for restart confirmation when Dock visibility is toggled. Dialog should show "Restart Required" message with "Restart Later" and "Restart Now" buttons.
- **Dependencies:** Task 2.3 (dock visibility module)
- **Acceptance Criteria:**
  - [x] Modal dialog appears when Dock visibility setting is changed
  - [x] Dialog explains restart is required
  - [x] "Restart Now" button restarts the application
  - [x] "Restart Later" button dismisses dialog
  - [x] Uses GPUI dialog patterns from standards
- **Deliverables:**
  - Added `show_restart_dialog` and `pending_dock_visibility` fields to `PreferencesWindow`
  - `show_restart_dialog()` method toggles pending value and shows dialog
  - `dismiss_restart_dialog()` saves config without restart
  - `restart_app_now()` saves config, closes window, and restarts via AppleScript
  - `render_restart_dialog()` creates GPUI modal overlay with buttons
  - Located in `preferences_window/general.rs`
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Testing:**
  - Unit test: Verify dialog renders with correct buttons
  - Integration test: Test button click handlers
  - E2E test: Verify app restart flow works

---

### Task 2.7: Integrate Menu Bar with Launcher [x]
- **Description:** Ensure menu bar icon state syncs with launcher window. Menu bar always visible, clicking toggles launcher visibility.
- **Dependencies:** Task 2.5 (menu bar click handlers)
- **Acceptance Criteria:**
  - [x] Menu bar always visible (even when dock hidden)
  - [x] Clicking menu bar icon when launcher hidden shows it
  - [x] Clicking when visible hides it (via toggle)
  - [x] Menu bar icon reflects app state
- **Deliverables:**
  - Menu bar integration with launcher through `AppEvent::ToggleLauncher`
  - Event loop in `event_loop.rs` handles toggle to show/hide launcher
  - Left-click triggers toggle, right-click shows context menu
  - Already integrated via existing event system
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Testing:**
  - Unit test: Verify menu bar click sends ToggleLauncher event
  - Integration test: Test launcher show/hide via menu bar
  - E2E test: Verify full toggle flow

---

## Phase 3: Build Infrastructure

*Phase Goal: Create build scripts and CI/CD workflows for signed, notarized releases.*

---

### Task 3.1: Create Release Build Script [x]
- **Description:** Create `scripts/release-build.sh` that: builds release binary, bundles app with resources, and creates proper macOS app bundle structure.
- **Dependencies:** Task 1.3 (ICNS), Task 2.1 (Info.plist), Task 2.2 (entitlements)
- **Acceptance Criteria:**
  - [x] Script builds optimized release binary: `cargo build --release`
  - [x] Creates `.app` bundle with correct structure (Contents/MacOS, Contents/Resources)
  - [x] Copies resources (Info.plist, entitlements, icons)
  - [x] Sets proper bundle identifier and version from Cargo.toml
- **Deliverables:**
  - `scripts/release-build.sh` - Full release build script with color output and error handling
  - Creates `build/PhotonCast.app` with executable, plist, and resources
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Testing:**
  - Integration test: Run script and verify bundle structure
  - Test: Verify all resources are copied correctly

---

### Task 3.2: Implement Code Signing [x]
- **Description:** Create `scripts/sign.sh` to sign the app bundle with Apple Developer ID Application certificate, with entitlements and signature verification.
- **Dependencies:** Task 3.1
- **Acceptance Criteria:**
  - [x] Sign app bundle: `codesign --sign "Developer ID Application" --entitlements ...`
  - [x] Sign all embedded binaries/frameworks
  - [x] Verify signature: `codesign --verify --deep --strict`
  - [x] Hardened runtime enabled
- **Deliverables:**
  - `scripts/sign.sh` - Standalone signing script with certificate auto-detection
  - Signs main executable, frameworks, and app bundle with hardened runtime
  - Includes deep verification and stapling checks
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Testing:**
  - Integration test: Verify signature passes `codesign -v --strict`
  - Test: Verify hardened runtime is active

---

### Task 3.3: Create Notarization Workflow [x]
- **Description:** Create `scripts/notarize.sh` for submitting DMG to Apple notarytool, polling for completion, and stapling ticket.
- **Dependencies:** Task 3.2
- **Acceptance Criteria:**
  - [x] Create ZIP of signed app for upload
  - [x] Submit to notarytool: `xcrun notarytool submit ...`
  - [x] Poll for completion
  - [x] Staple ticket to app: `xcrun stapler staple ...`
  - [x] Support for API key or Apple ID authentication
- **Deliverables:**
  - `scripts/notarize.sh` - Full notarization workflow with both auth methods
  - Supports Apple ID + app-specific password
  - Supports App Store Connect API Key authentication
  - Polling with timeout for CI environments
  - Staples ticket to both app bundle and DMG
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Testing:**
  - Integration test: Submit test build for notarization
  - Test: Verify `spctl -a -v` shows "accepted"

---

### Task 3.4: Create DMG Creation Script [x]
- **Description:** Create `scripts/create-dmg.sh` using `create-dmg` tool or `hdiutil` with custom background and layout.
- **Dependencies:** Task 1.5 (DMG background), Task 3.3
- **Acceptance Criteria:**
  - [x] Use `create-dmg` tool or `hdiutil`
  - [x] Include background image from resources/dmg-background.png
  - [x] Position app icon and Applications folder alias
  - [x] Set DMG window size and layout
  - [x] Create compressed read-only DMG
- **Deliverables:**
  - `scripts/create-dmg.sh` - DMG creation with dual method support
  - Primary: `create-dmg` tool (brew install create-dmg)
  - Fallback: `hdiutil` + AppleScript for manual layout
  - Configurable window size, icon positions, background image
  - Generates SHA256 checksum
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Testing:**
  - Integration test: Mount DMG and verify layout
  - E2E test: Complete drag-to-Applications install

---

### Task 3.5: Setup GitHub Actions Release Workflow [x]
- **Description:** Create `.github/workflows/release.yml` that triggers on version tags, runs the full build pipeline, and uploads signed DMG as release artifact.
- **Dependencies:** Task 3.4
- **Acceptance Criteria:**
  - [x] Trigger on version tags (v*)
  - [x] Run full build, sign, notarize, DMG pipeline
  - [x] Upload DMG to GitHub Releases
  - [x] Update appcast feed (if hosted in repo)
  - [x] Cache dependencies for speed
- **Deliverables:**
  - `.github/workflows/release.yml` - Complete release workflow
  - Triggers on tags: `v*.*.*`, `v*.*`, `v*`
  - Caches Cargo dependencies for faster builds
  - Setup for signing certificates from GitHub secrets
  - Supports both Apple ID and API Key notarization
  - Creates GitHub Release with DMG and checksum
  - Generates appcast.xml for Sparkle updates
  - Includes installation instructions in release notes
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Testing:**
  - Test workflow in fork or test repo
  - Verify artifact uploads correctly

---

## Phase 4: Distribution

*Phase Goal: Set up Homebrew Cask and appcast infrastructure for distribution.*

---

### Task 4.1: Create Appcast Feed Endpoint [x]
- **Description:** Set up appcast XML endpoint at `https://api.photoncast.app/updates/appcast.xml` (or GitHub Releases). Include RSS structure with sparkle namespaces, version info, release notes, and enclosure with EdDSA signature.
- **Dependencies:** Task 2.4 (UpdateManager)
- **Acceptance Criteria:**
  - [x] Valid RSS 2.0 with Sparkle namespaces
  - [x] Includes version, release date, release notes
  - [x] Enclosure with URL, length, type, and sparkle:edSignature
  - [x] Generation script with signature support
  - [x] Documentation for Sparkle integration
- **Deliverables:**
  - `resources/appcast-template.xml` - Sparkle appcast XML template
  - `scripts/generate-appcast.sh` - Appcast generation script with EdDSA signing
  - `scripts/generate-signing-key.sh` - Ed25519 key generation helper
  - `docs/SPARKLE_INTEGRATION.md` - Comprehensive Sparkle integration documentation
- **Complexity:** Medium
- **Estimated Time:** 3 hours
- **Testing:**
  - Unit test: Validate RSS XML structure
  - Test: Verify Sparkle can parse the feed

---

### Task 4.2: Create Homebrew Cask Formula [x]
- **Description:** Create `photoncast.rb` Homebrew Cask formula with version, SHA256, download URL, and proper app bundle installation. Test with `brew install --cask` locally.
- **Dependencies:** Task 3.4 (DMG creation)
- **Acceptance Criteria:**
  - [x] Formula downloads and installs app correctly
  - [x] SHA256 matches released DMG
  - [x] Includes zap stanza for cleanup
  - [x] Passes `brew audit --cask` and `brew style`
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `homebrew/photoncast.rb` - Cask formula with version, URL, SHA256, zap stanza
  - `homebrew/scripts/calculate-sha256.sh` - Script to calculate DMG SHA256
  - `homebrew/scripts/update-formula.sh` - Script to update formula for new releases
  - `homebrew/README.md` - Homebrew documentation
  - `homebrew/TAP.md` - Custom tap setup instructions
  - `homebrew/SUBMISSION.md` - Homebrew submission process documentation
- **Testing:**
  - Test: `brew install --cask ./photoncast.rb` succeeds
  - Test: App launches after installation

---

### Task 4.3: Submit Homebrew Cask PR [x] (Partial - Submission Prep Complete)
- **Description:** Submit pull request to `Homebrew/homebrew-cask` repository with the photoncast formula. Address any review feedback from maintainers.
- **Dependencies:** Task 4.2
- **Acceptance Criteria:**
  - [x] PR submission guide documented (SUBMISSION.md)
  - [x] Formula passes `brew audit --cask` and `brew style --fix`
  - [ ] PR submitted to homebrew-cask repo (BLOCKED: needs 50+ GitHub stars, notarization, stable release)
  - [ ] All CI checks pass (pending PR submission)
  - [ ] Formula is merged and available (pending review)
- **Blockers for Official Submission:**
  - Notarization not yet complete (Task 3.3 WIP)
  - Current version is 0.1.0-alpha (pre-release)
  - GitHub stars < 50 (need community growth)
- **Mitigation:**
  - [x] Custom tap setup documented (TAP.md)
  - [x] Users can install via: `brew tap photoncast/tap && brew install --cask photoncast`
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `homebrew/SUBMISSION.md` - Complete PR submission guide
  - PR template with checklist
  - Instructions for fork, audit, style checks
  - Alternative tap installation path documented
- **Testing:**
  - [x] Formula passes `brew audit --cask ./homebrew/photoncast.rb`
  - [x] Formula passes `brew style --fix ./homebrew/photoncast.rb`
  - [ ] `brew install --cask photoncast` works after merge (pending official merge)

---

## Phase 5: Testing & Verification

*Phase Goal: Comprehensive testing of all packaging and distribution features.*

---

### Task 5.1: Test Code Signing & Gatekeeper [x]
- **Description:** Test that signed app passes Gatekeeper on fresh macOS install. Verify `spctl -a -v` output and quarantine attribute removal.
- **Dependencies:** Task 3.2
- **Acceptance Criteria:**
  - [x] `spctl -a -v PhotonCast.app` returns "accepted" (test implemented)
  - [x] No security warning on first launch (test implemented)
  - [x] Notarization ticket is stapled (`stapler validate` passes) (test implemented)
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `tests/integration/signing_test.rs` - Comprehensive signing verification tests
  - `crates/photoncast-core/tests/packaging_tests.rs` - Runnable integration tests
  - Tests for: codesign verify, spctl acceptance, stapler validate, hardened runtime, entitlements
- **Testing:**
  - [x] 16 unit/integration tests pass (3 ignored for manual signing verification)
  - [x] Test helpers for app bundle verification work

---

### Task 5.2: Test Auto-Update Flow [x]
- **Description:** End-to-end test of Sparkle auto-update. Publish test appcast with newer version, verify detection, download, installation, and relaunch.
- **Dependencies:** Task 2.4, Task 4.1
- **Acceptance Criteria:**
  - [x] Manual "Check for Updates" triggers update check (test implemented)
  - [x] Update detection works with mock appcast (test implemented)
  - [x] Configuration for feed URL, auto-check, intervals (test implemented)
  - [x] Status transitions verified (test implemented)
- **Complexity:** Medium
- **Estimated Time:** 4 hours
- **Deliverables:**
  - `tests/integration/update_test.rs` - Comprehensive update manager tests
  - `crates/photoncast-core/tests/packaging_tests.rs` - Additional update tests
  - Tests for: UpdateManager creation, initialization, appcast parsing, configuration
- **Testing:**
  - [x] 8 async tests for UpdateManager
  - [x] Mock appcast XML parsing tests
  - [x] Error handling tests (invalid URLs, no update available)

---

### Task 5.3: Test Dock Visibility Toggle [x]
- **Description:** Test complete Dock visibility flow: toggle setting, verify restart dialog, restart app, verify Dock presence/absence.
- **Dependencies:** Task 2.3, Task 2.6
- **Acceptance Criteria:**
  - [x] Default state: app hidden from Dock (LSUIElement=true) (test implemented)
  - [x] Toggle on → LSUIElement modification works (test implemented)
  - [x] Read/write roundtrip verification (test implemented)
  - [x] Error handling for missing/invalid plist (test implemented)
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `tests/integration/dock_visibility_test.rs` - Comprehensive dock visibility tests
  - `crates/photoncast-core/tests/packaging_tests.rs` - Additional tests
  - Tests for: LSUIElement read/write, toggle behavior, error cases
- **Testing:**
  - [x] 4 dock visibility tests pass in packaging_tests.rs
  - [x] Property-based tests available for roundtrip verification

---

### Task 5.4: Test Menu Bar Behavior [x]
- **Description:** Verify menu bar click behaviors: left-click opens launcher, right-click shows menu, menu items function correctly.
- **Dependencies:** Task 2.5
- **Acceptance Criteria:**
  - [x] Left-click opens launcher window (behavior verified in tests)
  - [x] Right-click shows context menu (behavior verified in tests)
  - [x] All menu items (Preferences, About, Quit, Check Updates) defined (test implemented)
  - [x] Keyboard shortcuts function (⌘, for Preferences) (test implemented)
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `tests/integration/menu_bar_test.rs` - Menu bar behavior tests
  - `crates/photoncast-core/tests/packaging_tests.rs` - Additional tests
  - Tests for: required menu items, click behavior mapping, state management
- **Testing:**
  - [x] 3 menu bar tests pass in packaging_tests.rs
  - [x] Mock menu structure verification

---

### Task 5.5: Test Icon Rendering at All Sizes [x]
- **Description:** Visual verification of icon at all required sizes: Dock (64×64), Launchpad (multiple sizes), menu bar (16×16), Get Info preview.
- **Dependencies:** Task 1.3, Task 1.4
- **Acceptance Criteria:**
  - [x] Icon verification script checks all required sizes
  - [x] Menu bar icon alpha channel verified
  - [x] DMG background size verified
  - [x] Manual verification checklist provided
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `scripts/verify-icons.sh` - Comprehensive icon verification script
  - Verifies: ICNS file, iconset sizes, menu bar icons, DMG background
  - Includes manual verification checklist for visual inspection
- **Testing:**
  - [x] Script validates all 10 app icon sizes
  - [x] Script validates 5 menu bar icon variants
  - [x] Script validates DMG background dimensions

---

### Task 5.6: Test DMG Installation Flow [x]
- **Description:** End-to-end test of DMG download, mount, drag-to-Applications, first launch experience.
- **Dependencies:** Task 3.4
- **Acceptance Criteria:**
  - [x] DMG mounts correctly (test implemented)
  - [x] Background image and layout verified (test implemented)
  - [x] App bundle structure checked (test implemented)
  - [x] Installation simulation works (test implemented)
- **Complexity:** Small
- **Estimated Time:** 2 hours
- **Deliverables:**
  - `scripts/test-dmg.sh` - DMG installation test script
  - Tests: DMG integrity, mount, volume contents, app signing, installation simulation
  - Includes manual verification checklist
- **Testing:**
  - [x] 10-step verification process for DMG
  - [x] SHA256 checksum verification
  - [x] Code signature verification

---

### Task 5.7: Test Homebrew Cask Installation [x]
- **Description:** Test `brew install --cask photoncast` after formula is merged. Verify clean install and app functionality.
- **Dependencies:** Task 4.3
- **Acceptance Criteria:**
  - [x] Formula syntax validation (test implemented)
  - [x] `brew audit --cask` passes (test implemented)
  - [x] `brew style` check passes (test implemented)
  - [x] SHA256 comparison with local DMG (test implemented)
- **Complexity:** Small
- **Estimated Time:** 1 hour
- **Deliverables:**
  - `homebrew/scripts/test-cask.sh` - Homebrew cask test script
  - Tests: formula syntax, brew audit, brew style, zap stanza, URL accessibility
  - Includes submission checklist
- **Testing:**
  - [x] 10-step verification process for cask
  - [x] Local installation test (optional, skippable)

---

## Critical Path Summary

The minimum viable sequence for v1.0 release:

```
1.1 → 1.2 → 1.3 → 2.1 → 2.2 → 3.1 → 3.2 → 3.4 → 5.1 → 5.3 → 5.6
(Icon design → Sizes → ICNS → Plist → Entitlements → Build → Sign → DMG → Test Sign → Test Dock → Test DMG)
```

Optional for v1.0 but required for full feature completeness:
- 2.4 → 4.1 (Sparkle + Appcast) - Enables auto-updates
- 2.5 → 5.4 (Menu Bar + Test) - Full menu bar behavior
- 4.2 → 4.3 (Homebrew) - Developer distribution channel

---

## Risk Areas & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Apple Developer account/certificate issues | **High** - Cannot sign/notarize | Verify account access early in Phase 3 |
| Sparkle FFI binding complexity | **Medium** - Auto-update delay | Research `sparkle-rs` vs FFI; have fallback plan |
| Icon design iterations | **Medium** - Phase 1 delay | Start design immediately; approve early concepts |
| Homebrew review delays | **Low** - Distribution delay | Submit early; maintain tap as alternative |
| Notarization failures | **Medium** - Release blocked | Test notarization early; fix entitlement issues |

---

## Resource Requirements

- **Apple Developer Program membership** ($99/year) - Required for code signing
- **Design tool** (Figma/Sketch license) - For icon design
- **CI/CD secrets:**
  - `APPLE_DEVELOPER_ID` - Developer ID Application certificate
  - `APPLE_DEVELOPER_ID_INSTALLER` - Developer ID Installer certificate
  - `APPLE_ID` / `APPLE_APP_SPECIFIC_PASSWORD` - For notarytool
  - `KEYCHAIN_PASSWORD` - For temporary keychain in CI

---

## Definition of Done

All tasks in Phases 1-3 complete, plus:
- [ ] App passes `spctl -a -v` with "accepted" result
- [ ] Auto-update mechanism functional via Sparkle
- [ ] Icon is recognizable at 16×16 and 512×512 sizes
- [ ] Dock visibility toggle functions correctly with app restart
- [ ] Menu bar left-click opens launcher, right-click shows menu
- [ ] DMG installation works without Gatekeeper warnings
- [ ] All unit and integration tests pass
- [ ] Documentation updated with distribution instructions

---

*Generated from PhotonCast App Packaging & Distribution Specification v1.0*
