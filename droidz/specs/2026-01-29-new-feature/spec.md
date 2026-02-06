# PhotonCast App Packaging & Distribution Specification

**Date:** 2026-01-29  
**Status:** Draft  
**Feature Area:** macOS App Distribution, Dock/Menu Bar Behavior, App Icon, Auto-Updates  
**Priority:** High — Required for v1.0 Public Release

---

## 1. Overview

### 1.1 Purpose

This specification defines the app packaging, distribution, and system integration requirements for PhotonCast's public release. It covers how users will discover, install, and interact with the application on macOS — including the app icon design, dock visibility preferences, menu bar behavior, and update mechanisms.

### 1.2 Goals

1. **Professional Distribution**: Provide a polished, trustworthy installation experience via DMG with automatic updates
2. **System Integration**: Seamlessly integrate with macOS Dock and menu bar following platform conventions
3. **Developer-Friendly**: Support Homebrew Cask for developer adoption and CI/CD workflows
4. **Security Compliance**: Signed and notarized for macOS Gatekeeper compliance
5. **Brand Identity**: Establish a distinctive visual identity aligned with Catppuccin design language

### 1.3 Target Audience

- **Primary**: macOS power users seeking a Raycast/Alfred alternative
- **Secondary**: Developers who prefer Rust-based tools and Homebrew installation
- **Tertiary**: General macOS users discovering via word-of-mouth or tech communities

### 1.4 Success Criteria

- App passes macOS Gatekeeper without user intervention
- Auto-update mechanism works reliably via Sparkle
- Icon is recognizable at 16×16 (menu bar) and 512×512 (Dock/App Store) sizes
- Dock visibility toggle functions correctly with app restart
- Homebrew Cask formula is accepted and functional

---

## 2. User Stories

### 2.1 Installation & First Launch

**Story 1: DMG Installation**
> As a new user, I want to download PhotonCast as a DMG, drag it to Applications, and launch it without security warnings, so that I can start using it immediately.

**Story 2: Homebrew Installation**
> As a developer, I want to install PhotonCast via `brew install --cask photoncast`, so that I can manage it alongside my other development tools.

**Story 3: Auto-Updates**
> As a user, I want PhotonCast to notify me when updates are available and install them automatically, so that I always have the latest features and security fixes.

### 2.2 Dock & Menu Bar Integration

**Story 4: Menu Bar Access**
> As a user, I want to access PhotonCast from the menu bar at all times, so that I can open the launcher or access settings even if I forget the hotkey.

**Story 5: Dock Visibility Preference**
> As a user, I want to choose whether PhotonCast appears in my Dock, so that I can keep my Dock clean while still having quick access when needed.

**Story 6: Menu Bar Behavior**
> As a user, I want left-clicking the menu bar icon to open the launcher and right-clicking to show a menu, so that I have intuitive access to all app functions.

### 2.3 Visual Identity

**Story 7: Icon Recognition**
> As a user, I want to easily identify PhotonCast in my Dock and menu bar through a distinctive, professional icon, so that it feels like a native macOS application.

---

## 3. Technical Requirements

### 3.1 Distribution Formats

| Format | Priority | Purpose |
|--------|----------|---------|
| **DMG** | P0 | Primary distribution for direct download |
| **Sparkle Auto-Update** | P0 | Seamless in-app updates |
| **Homebrew Cask** | P1 | Developer/CLI installation |
| **Mac App Store** | P2 (Future) | Broader distribution (requires sandboxing review) |

### 3.2 Code Signing & Notarization

**Requirements:**
- Apple Developer ID Application certificate for signing
- Apple Developer ID Installer certificate for PKG (if needed)
- Notarization via `notarytool` for all distributed binaries
- Hardened runtime with appropriate entitlements

**Entitlements Required:**
```xml
<!-- entitlements.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" 
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Core functionality -->
    <key>com.apple.security.automation.apple-events</key>
    <true/>
    <key>com.apple.security.temporary-exception.apple-events</key>
    <array>
        <string>com.apple.systemevents</string>
    </array>
    
    <!-- Accessibility for window management -->
    <key>com.apple.security.accessibility</key>
    <true/>
    
    <!-- File access -->
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
    
    <!-- Network for updates and extensions -->
    <key>com.apple.security.network.client</key>
    <true/>
    
    <!-- Keychain for clipboard encryption keys -->
    <key>keychain-access-groups</key>
    <array/>
</dict>
</plist>
```

### 3.3 Info.plist Configuration

**Key Entries:**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Bundle Info -->
    <key>CFBundleName</key>
    <string>PhotonCast</string>
    <key>CFBundleDisplayName</key>
    <string>PhotonCast</string>
    <key>CFBundleIdentifier</key>
    <string>com.photoncast.app</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    
    <!-- Dock Visibility (default: hidden) -->
    <key>LSUIElement</key>
    <true/>
    
    <!-- App Category -->
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.productivity</string>
    
    <!-- High Resolution Support -->
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSRequiresAquaSystemAppearance</key>
    <false/>
    
    <!-- Security -->
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    
    <!-- Localization -->
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleLocalizations</key>
    <array>
        <string>en</string>
    </array>
</dict>
</plist>
```

**Note:** `LSUIElement = true` hides the app from Dock and Cmd+Tab by default. The user preference toggle requires an app restart because `LSUIElement` is read at launch time and cannot be changed dynamically.

---

## 4. Implementation Details

### 4.1 Sparkle Auto-Update Integration

**Crate Selection:**
Use the `sparkle-rs` crate (bindings to the native Sparkle framework) or invoke Sparkle via FFI. Alternative: embed Sparkle.framework and trigger via Objective-C interop.

**Implementation:**
```rust
// In photoncast-core/src/platform/updates.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Sparkle initialization failed: {0}")]
    InitFailed(String),
    #[error("Update check failed: {0}")]
    CheckFailed(String),
}

pub struct UpdateManager {
    feed_url: String,
    auto_check_enabled: bool,
}

impl UpdateManager {
    pub fn new(feed_url: &str) -> Self {
        Self {
            feed_url: feed_url.to_string(),
            auto_check_enabled: true,
        }
    }
    
    /// Initialize Sparkle with the appcast feed URL
    pub fn initialize(&self) -> Result<(), UpdateError> {
        // Initialize Sparkle framework
        // Set feed URL to: https://api.photoncast.app/updates/appcast.xml
        Ok(())
    }
    
    /// Manually check for updates
    pub fn check_for_updates(&self) -> Result<(), UpdateError> {
        // Trigger Sparkle update check UI
        Ok(())
    }
    
    /// Set automatic update checking
    pub fn set_automatic_checks(&mut self, enabled: bool) {
        self.auto_check_enabled = enabled;
        // Update Sparkle settings
    }
}
```

**Appcast Feed Structure:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
    <channel>
        <title>PhotonCast Changelog</title>
        <item>
            <title>Version 1.1.0</title>
            <pubDate>Mon, 15 Feb 2026 12:00:00 +0000</pubDate>
            <sparkle:version>1.1.0</sparkle:version>
            <sparkle:shortVersionString>1.1.0</sparkle:shortVersionString>
            <description><![CDATA[
                <h2>What's New</h2>
                <ul>
                    <li>New calculator features</li>
                    <li>Performance improvements</li>
                </ul>
            ]]></description>
            <enclosure url="https://api.photoncast.app/releases/1.1.0/PhotonCast.dmg"
                       sparkle:edSignature="..."
                       length="15240000"
                       type="application/octet-stream"/>
        </item>
    </channel>
</rss>
```

### 4.2 Dock Visibility Toggle Implementation

**Config Extension:**
The `show_in_dock` field already exists in `GeneralConfig` (default: `false`). The implementation requires:

1. **UI Toggle** (already exists in `preferences_window/general.rs`)
2. **Info.plist Modification Script**
3. **Restart Prompt**

**Platform Module Addition:**
```rust
// photoncast-core/src/platform/dock_visibility.rs

use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockVisibilityError {
    #[error("Failed to modify Info.plist: {0}")]
    PlistError(String),
    #[error("App restart required for changes to take effect")]
    RestartRequired,
}

/// Sets the LSUIElement value in Info.plist
/// 
/// Note: Changes require app restart to take effect
pub fn set_dock_visibility(show_in_dock: bool) -> Result<(), DockVisibilityError> {
    let plist_path = get_info_plist_path()?;
    
    // Use PlistBuddy or defaults command to modify LSUIElement
    let value = if show_in_dock { "false" } else { "true" };
    
    Command::new("/usr/libexec/PlistBuddy")
        .args(&[
            "-c",
            &format!("Set :LSUIElement {}", value),
            &plist_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| DockVisibilityError::PlistError(e.to_string()))?;
    
    Err(DockVisibilityError::RestartRequired)
}

/// Reads current LSUIElement value from Info.plist
pub fn get_dock_visibility() -> Result<bool, DockVisibilityError> {
    // Read current value and return inverse (LSUIElement=true means hidden)
    Ok(false) // Placeholder
}

fn get_info_plist_path() -> Result<std::path::PathBuf, DockVisibilityError> {
    // Return path to Contents/Info.plist within app bundle
    Ok(std::path::PathBuf::from(
        "/Applications/PhotonCast.app/Contents/Info.plist"
    ))
}
```

**Preferences UI Update:**
The existing toggle in `general.rs` already includes "(requires restart)" text. Need to:
1. Add restart confirmation dialog when toggle is changed
2. Provide "Restart Now" / "Later" options

### 4.3 Menu Bar Icon Behavior

The menu bar implementation exists in `photoncast-core/src/platform/menu_bar.rs`. Need to extend for click behavior:

```rust
// Extend MenuBarManager with click handlers

pub enum MenuBarClickBehavior {
    /// Left click opens launcher, right click shows menu
    RaycastStyle,
    /// Both clicks show menu
    MenuOnly,
}

impl MenuBarManager {
    /// Handle left-click on menu bar icon
    pub fn on_left_click(&self, cx: &mut App) {
        // Open launcher window
    }
    
    /// Handle right-click on menu bar icon  
    pub fn on_right_click(&self, cx: &mut App) {
        // Show context menu: Settings, About, Quit
    }
}
```

**Menu Structure:**
```
┌─────────────────────────────┐
│  Open PhotonCast      ⌘Space │
├─────────────────────────────┤
│  Preferences...           ⌘, │
├─────────────────────────────┤
│  Check for Updates           │
├─────────────────────────────┤
│  About PhotonCast            │
├─────────────────────────────┤
│  Quit PhotonCast          ⌘Q │
└─────────────────────────────┘
```

---

## 5. Icon Specifications

### 5.1 Design Concept

**Theme:** "Photon Beam" — A minimal, abstract representation of a light beam or photon particle emanating from a central source.

**Design Language:**
- macOS Big Sur/Monterey rounded-square format
- Liquid/glass aesthetic with subtle gradients
- Clean geometric construction
- Distinctive at small sizes

**Color Palette (Catppuccin Mocha):**
| Element | Color | Hex |
|---------|-------|-----|
| Primary gradient start | Mauve | `#cba6f7` |
| Primary gradient end | Pink | `#f5c2e7` |
| Accent highlight | Flamingo | `#f2cdcd` |
| Background | Base | `#1e1e2e` |
| Inner glow | Surface0 | `#313244` |

### 5.2 Icon Sizes & Formats

**App Icon (Dock/App):**

| Size | Scale | Purpose | Format |
|------|-------|---------|--------|
| 16×16 | @1x | Small lists | PNG |
| 16×16 | @2x | Retina lists | PNG |
| 32×32 | @1x | Standard | PNG |
| 32×32 | @2x | Retina | PNG |
| 128×128 | @1x | Finder | PNG |
| 128×128 | @2x | Retina Finder | PNG |
| 256×256 | @1x | Quick Look | PNG |
| 256×256 | @2x | Retina Quick Look | PNG |
| 512×512 | @1x | App Store | PNG |
| 512×512 | @2x | Retina App Store | PNG |

**Combined:** `AppIcon.icns` (Icon Set format)

**Menu Bar Icon:**

| Size | Scale | Format | Notes |
|------|-------|--------|-------|
| 16×16 | @1x | Template PNG | Monochrome, auto-inverts |
| 16×16 | @2x | Template PNG | Retina support |
| 18×18 | @1x | Template PNG | macOS 11+ standard |
| 18×18 | @2x | Template PNG | Retina |

**Design Requirements:**
- Menu bar icon must be **template images** (pure black/alpha) for automatic dark mode support
- No colors in menu bar icon — macOS handles inversion
- Simple, recognizable silhouette of the app icon
- 1-2px padding within the 16×16 canvas

### 5.3 Icon Construction Guidelines

**App Icon Geometry:**
```
┌──────────────────────────────┐
│    Rounded Rectangle         │
│    Corner radius: ~22%       │
│                              │
│     ╭──────────────╮         │
│    ╱   Gradient    ╲        │
│   │   Background    │       │
│   │                 │       │
│   │    ━━━━☆━━━━    │       │  ← Photon beam motif
│   │   (centered)    │       │
│   │                 │       │
│    ╲                ╱        │
│     ╰──────────────╯         │
│                              │
└──────────────────────────────┘
```

**Menu Bar Icon:**
- Simplified version without gradient
- Photon beam as 2-3 horizontal lines or a single dot with emission
- Maximum 16×16px, centered
- 1px padding on all sides

### 5.4 Asset Generation Workflow

1. **Source File:** Create in Figma/Sketch at 1024×1024px
2. **Export:** Automated script to generate all sizes
3. **Compilation:** `iconutil -c icns` to create .icns file
4. **Placement:** Copy to `Resources/AppIcon.icns` in bundle

**Build Script:**
```bash
#!/bin/bash
# scripts/generate-icons.sh

SOURCE="assets/icon-source.svg"
OUTPUT_DIR="resources/icons"

# Generate all app icon sizes
for size in 16 32 128 256 512; do
    # @1x
    convert -background none -resize ${size}x${size} $SOURCE $OUTPUT_DIR/icon_${size}x${size}.png
    # @2x
    convert -background none -resize $((size*2))x$((size*2)) $SOURCE $OUTPUT_DIR/icon_${size}x${size}@2x.png
done

# Create ICNS
iconutil -c icns -o $OUTPUT_DIR/AppIcon.icns $OUTPUT_DIR/iconset

# Generate menu bar template
convert -background none -resize 16x16 -colorspace gray $SOURCE $OUTPUT_DIR/MenuBarIcon.pdf
```

---

## 6. Settings UI for Dock Visibility

### 6.1 Current Implementation Status

The Dock visibility toggle already exists in `preferences_window/general.rs` with:
- Label: "Show in Dock"
- Sublabel: "Display PhotonCast icon in the Dock (requires restart)"
- Default value: `false` (hidden from Dock)

### 6.2 Required Enhancements

**Restart Confirmation Flow:**

When user toggles "Show in Dock", present a modal:

```
┌─────────────────────────────────────┐
│  Restart Required                   │
│                                     │
│  This change requires a restart to  │
│  take effect.                       │
│                                     │
│  [Restart Later]    [Restart Now]   │
└─────────────────────────────────────┘
```

**Implementation in GPUI:**
```rust
// In preferences_window/general.rs

fn toggle_show_in_dock(&mut self, cx: &mut ViewContext<Self>) {
    let new_value = !self.config.general.show_in_dock;
    
    // Update platform setting
    if let Err(e) = platform::dock_visibility::set_dock_visibility(new_value) {
        // Show restart required dialog
        self.show_restart_dialog(cx);
    }
    
    // Update config (persisted immediately)
    self.config.general.show_in_dock = new_value;
    self.save_config(cx);
    cx.notify();
}

fn show_restart_dialog(&self, cx: &mut ViewContext<Self>) {
    // Render modal dialog with restart options
}
```

### 6.3 Settings Layout

The General section currently shows (in order):
1. Launch at Login
2. Show in Menu Bar
3. **Show in Dock** ← Already implemented
4. Max Results

---

## 7. Tasks Breakdown

### Phase 1: Icon Design & Assets (2-3 days)

| Task | Priority | Assignee | Est. Time |
|------|----------|----------|-----------|
| Create icon concept sketches | P0 | Designer | 4h |
| Design 1024×1024 source file | P0 | Designer | 8h |
| Generate all required sizes | P0 | Dev | 2h |
| Create menu bar template version | P0 | Dev | 2h |
| Build ICNS file and integrate | P0 | Dev | 2h |
| Test icon at all sizes | P1 | QA | 2h |

**Deliverables:**
- `resources/AppIcon.icns`
- `resources/MenuBarIcon.pdf` (template)
- Source file in repo (`assets/icon-source.fig` or `.sketch`)

### Phase 2: Build & Signing Infrastructure (2-3 days)

| Task | Priority | Assignee | Est. Time |
|------|----------|----------|-----------|
| Create release build script | P0 | Dev | 4h |
| Set up code signing in CI | P0 | Dev | 4h |
| Implement notarization workflow | P0 | Dev | 4h |
| Create DMG layout with background | P0 | Dev | 4h |
| Test signed/notarized build | P0 | Dev | 2h |

**Deliverables:**
- `scripts/build-release.sh`
- `.github/workflows/release.yml` (updated)
- DMG background image
- Signed, notarized DMG

### Phase 3: Sparkle Integration (2 days)

| Task | Priority | Assignee | Est. Time |
|------|----------|----------|-----------|
| Add Sparkle framework dependency | P0 | Dev | 2h |
| Implement UpdateManager module | P0 | Dev | 4h |
| Create appcast feed endpoint | P0 | Dev | 2h |
| Add "Check for Updates" menu item | P0 | Dev | 2h |
| Test update flow | P0 | QA | 2h |

**Deliverables:**
- `photoncast-core/src/platform/updates.rs`
- Appcast XML endpoint
- Working auto-update in test builds

### Phase 4: Homebrew Cask (1 day)

| Task | Priority | Assignee | Est. Time |
|------|----------|----------|-----------|
| Create Cask formula | P1 | Dev | 2h |
| Submit PR to homebrew-cask | P1 | Dev | 2h |
| Address review feedback | P1 | Dev | 2h |

**Deliverables:**
- `photoncast.rb` formula
- Accepted PR to `Homebrew/homebrew-cask`

### Phase 5: Menu Bar Behavior (1-2 days)

| Task | Priority | Assignee | Est. Time |
|------|----------|----------|-----------|
| Implement left-click → open launcher | P0 | Dev | 4h |
| Implement right-click context menu | P0 | Dev | 4h |
| Add menu items: Settings, About, Quit | P0 | Dev | 2h |
| Test menu behavior | P0 | QA | 2h |

**Deliverables:**
- Updated `menu_bar.rs` with click handlers
- Working context menu

### Phase 6: Dock Visibility Polish (1 day)

| Task | Priority | Assignee | Est. Time |
|------|----------|----------|-----------|
| Implement restart dialog | P0 | Dev | 4h |
| Test Info.plist modification | P0 | Dev | 2h |
| Test restart flow | P0 | QA | 2h |

**Deliverables:**
- Restart confirmation dialog
- Working Dock visibility toggle with restart

---

## 8. Testing Plan

### 8.1 Code Signing & Notarization

| Test Case | Steps | Expected Result |
|-----------|-------|-----------------|
| Gatekeeper acceptance | Download DMG, double-click app | No security warning, app opens |
| Quarantine removal | Check `xattr -l PhotonCast.app` | No `com.apple.quarantine` flag |
| Notarization valid | `spctl -a -v PhotonCast.app` | "accepted" and notarization ticket present |

### 8.2 Auto-Update (Sparkle)

| Test Case | Steps | Expected Result |
|-----------|-------|-----------------|
| Manual check | Click "Check for Updates" | Sparkle window shows current version |
| Update available | Publish test appcast with newer version | Sparkle prompts to update |
| Update install | Accept update | New version downloads, installs, relaunches |
| Background check | Launch app, wait for interval | Automatic check happens silently |

### 8.3 Dock Visibility

| Test Case | Steps | Expected Result |
|-----------|-------|-----------------|
| Default hidden | Fresh install launch | No Dock icon, menu bar icon visible |
| Enable Dock | Toggle on in Settings, restart | Dock icon appears on relaunch |
| Disable Dock | Toggle off in Settings, restart | Dock icon removed on relaunch |
| Cmd+Tab behavior | App hidden from Dock | Does not appear in Cmd+Tab switcher |

### 8.4 Menu Bar Behavior

| Test Case | Steps | Expected Result |
|-----------|-------|-----------------|
| Left-click open | Left-click menu bar icon | Launcher window opens |
| Right-click menu | Right-click menu bar icon | Context menu appears |
| Settings access | Right-click → Preferences | Settings window opens |
| Quit from menu | Right-click → Quit | App quits cleanly |

### 8.5 Icon Rendering

| Test Case | Steps | Expected Result |
|-----------|-------|-----------------|
| Dock icon | View app in Dock | Icon clear, recognizable at 64×64 |
| Menu bar icon | View in menu bar | Template image, inverts for dark mode |
| Small size | View in Launchpad small | Icon distinct, not muddy |
| App Store | View in App Store (future) | Icon attractive at all displayed sizes |

---

## 9. Acceptance Criteria

### 9.1 Must Have (P0)

- [ ] App is signed with valid Apple Developer ID certificate
- [ ] App passes notarization without warnings
- [ ] DMG is created with drag-to-Applications workflow
- [ ] Sparkle auto-update is functional
- [ ] Menu bar icon is visible and interactive
- [ ] Left-click opens launcher, right-click shows menu
- [ ] Dock visibility toggle works with app restart
- [ ] App icon is distinctive at all required sizes
- [ ] Menu bar icon follows macOS template image conventions

### 9.2 Should Have (P1)

- [ ] Homebrew Cask formula is available
- [ ] DMG has custom background with app icon and arrow
- [ ] Update release notes are displayed in Sparkle UI
- [ ] Code signing is automated in CI/CD pipeline

### 9.3 Nice to Have (P2)

- [ ] Delta updates (smaller download sizes)
- [ ] Mac App Store version (requires sandboxing review)
- [ ] In-app onboarding for first launch

---

## 10. Open Questions

1. **Sparkle Hosting**: Where will the appcast feed be hosted? (GitHub Releases, S3, dedicated API)
2. **Update Frequency**: How often should automatic checks happen? (Daily, weekly)
3. **Beta Channel**: Should there be a beta update channel for testers?
4. **Icon Designer**: Do we have a designer, or should we commission one?
5. **Paid vs Free**: Will this be a free app, or do we need payment processing?

---

## 11. References

### Apple Documentation
- [Code Signing Guide](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/)
- [Notarization Guide](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Information Property List](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/)

### Sparkle Documentation
- [Sparkle Project](https://sparkle-project.org/)
- [Appcast Documentation](https://sparkle-project.org/documentation/publishing/)

### Homebrew Cask
- [Cask Cookbook](https://docs.brew.sh/Cask-Cookbook)
- [Adding a Cask](https://docs.brew.sh/Adding-Software-to-Homebrew#casks)

### Existing PhotonCast Specs
- [Phase 1 MVP Spec](../2026-01-15-phase1-mvp/spec.md)
- [Phase 2 Productivity Spec](../2026-01-16-phase-2-v1.0-productivity-features/spec.md)
- [Phase 3 Extension Spec](../2026-01-23-native-extension-system/spec.md)

---

## 12. Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-01-29 | 1.0 | Spec Writer | Initial specification covering DMG, Sparkle, Homebrew, Dock visibility, menu bar behavior, and icon design |

---

*End of Specification*
