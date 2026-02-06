# App Packaging & Distribution — Clarifying Questions

**Date:** 2026-01-29  
**Feature Area:** macOS App Distribution, Dock/Menu Bar Behavior, App Icon  
**Status:** Awaiting user input

---

## Overview

This feature covers how users will discover, install, and interact with PhotonCast on macOS — including the app icon design, dock visibility preferences, and menu bar behavior.

---

## Clarifying Questions

### 1. Installation & Distribution Method

**How should users obtain and install PhotonCast?**

Context: The distribution method affects code signing requirements, sandboxing, update mechanisms, and user acquisition channels.

| Option | Pros | Cons |
|--------|------|------|
| **DMG with drag-to-Applications** | Simple, no installer, common for utility apps | Manual updates, no auto-update |
| **PKG installer** | Can install helper tools/launch agents, more "official" feel | More complex, user sees installer UI |
| **Homebrew Cask** | CLI-friendly, popular with developers, easy updates | Requires `brew` knowledge, formula maintenance |
| **Mac App Store** | Wide reach, automatic updates, trusted by users | Sandboxing restrictions, 30% fee, review process |
| **Direct download + Sparkle auto-updates** | Full control, seamless updates, no sandboxing | Need to host + implement update mechanism |
| **Multiple methods** (e.g., Homebrew + DMG + App Store) | Maximum reach | More maintenance overhead |

**Follow-ups:**
- Do you need auto-update functionality, or are users okay with manual updates?
- Are you planning a free app, paid one-time, or subscription model?

**Suggested default:** DMG with drag-to-Applications for MVP, with Sparkle auto-updater. Add Homebrew Cask for developer adoption.

---

### 2. Dock Visibility Preferences

**Should PhotonCast appear in the Dock, and should this be configurable?**

Context: Launcher utilities like Alfred and Raycast typically hide from the Dock (running as `LSUIElement`) because they're invoked via hotkey, not clicked from the Dock. However, some users prefer seeing the app in the Dock for quick access or force-quit purposes.

| Approach | Behavior | Configuration |
|----------|----------|---------------|
| **Always hidden from Dock** (`LSUIElement = true`) | Cleanest experience, icon only in menu bar | Hardcoded in Info.plist |
| **Preference toggle in Settings** | User chooses: "Show in Dock: On/Off" | App writes to plist, requires restart to take effect |
| **Context menu on menu bar icon** | Right-click menu bar icon → "Show/Hide in Dock" | Runtime toggle (more complex to implement) |
| **Never hidden (normal app)** | Always visible in Dock and Cmd+Tab | Standard app behavior |

**Reference apps:**
- **Raycast:** Hidden from Dock (menu bar only)
- **Alfred:** Hidden from Dock (menu bar only)
- **Spotlight:** Hidden (system service, no icon)

**Follow-ups:**
- Should the app still appear in Cmd+Tab when hidden from Dock? (macOS behavior: LSUIElement apps don't appear in Cmd+Tab)
- If there's a Settings window, should that appear in the Dock when open?

**Suggested default:** Hidden from Dock by default (`LSUIElement = true`) with a preference toggle in Settings that requires app restart.

---

### 3. App Icon Design

**What visual identity should PhotonCast have?**

Context: The icon needs to work at multiple sizes (menu bar: ~16-22px, Dock: 128-512px) and follow macOS design conventions. It should feel "at home" next to other Apple and third-party utility apps.

**Design Direction Questions:**

1. **Do you have any existing brand assets, logo, or color scheme?** Or is this a blank slate?

2. **Reference apps:** Which macOS app icons do you like? (e.g., Raycast's minimal purple gradient, Things' blue checkmark, Bear's gradient icon, Apple's fluid "liquid" icons in Monterey+)

3. **Conceptual direction:** What should the icon evoke?
   - Light/beam metaphor ("Photon" in the name)
   - Search/magnifying glass
   - Command/terminal prompt
   - Lightning bolt / speed
   - Minimal geometric shape

4. **Color palette:** Should it align with Catppuccin (the app's current theme)? Or use a distinct brand color?

5. **Do you have a designer, or should we:**
   - Create a simple placeholder icon
   - Commission a designer
   - Use an AI icon generator
   - Adapt an existing open-source icon

**Technical Requirements:**
- Menu bar icon: Template image (monochrome, ~16x16px, supports dark mode)
- App icon: Full color icon in ICNS format (1024x1024px source, includes all sizes down to 16x16)
- Both should look good in Big Sur/Monterey rounded-square style

**Suggested default:** Minimal icon concept playing on "light beam" or "spotlight" metaphor, using Catppuccin palette (mauve/pink gradient), with a clean geometric shape that works at all sizes.

---

### 4. Menu Bar Icon Behavior

**What should happen when the user interacts with the menu bar icon?**

Context: Since PhotonCast is a launcher, the primary interaction is typically a global hotkey (e.g., Cmd+Space or Option+Space). The menu bar icon serves as a secondary access point and status indicator.

**Click Behavior:**

| Action | Behavior | Common In |
|--------|----------|-----------|
| **Left-click opens launcher** | Same as hotkey | Raycast |
| **Left-click shows menu** | Dropdown with: Open, Settings, Quit | Alfred, many menu bar apps |
| **Right-click shows menu** | Same menu on right-click | Standard macOS behavior |
| **Double-click opens launcher** | Less common | — |

**Menu Contents (if showing a menu):**
- Open PhotonCast (or "Search...")
- Settings/Preferences...
- Check for Updates
- About PhotonCast
- Quit PhotonCast

**Additional Behaviors:**
- Should the menu bar icon show a "dot" or badge when updates are available?
- Should it change appearance when the launcher is "active" vs. hidden?
- Should there be a "Start at Login" toggle in the menu?

**Reference apps:**
- **Raycast:** Menu bar icon has no left-click action (only right-click menu)
- **Alfred:** Menu bar icon shows menu on left-click
- **Dropover:** Left-click opens main interface

**Suggested default:** Left-click opens the launcher (same as hotkey) for fastest access. Right-click shows a menu with Settings, About, Quit.

---

## Summary of Suggested Defaults

| Area | Suggested Approach |
|------|-------------------|
| **Distribution** | DMG + Sparkle auto-updater, add Homebrew Cask later |
| **Dock Visibility** | Hidden by default (`LSUIElement`), preference toggle in Settings |
| **Icon Design** | Minimal "light beam" concept, Catppuccin mauve/pink palette |
| **Menu Bar Click** | Left-click opens launcher, right-click shows menu |

---

## Next Steps

Once you've answered these questions:

1. I'll update the specification with your decisions
2. Create technical implementation tasks
3. If icon design is needed, we can explore concepts or find design resources
4. Define the exact plist configuration and entitlements needed

Please respond with your preferences, or let me know if you'd like to discuss any of these options!
