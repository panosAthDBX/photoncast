# PhotonCast Phase 1 MVP - Requirements Clarification

> Clarifying questions to finalize the Phase 1 MVP specification

---

## Context

Based on the raw idea and project standards, PhotonCast Phase 1 MVP will deliver:
- **Core UI Framework**: Launcher window, search bar, results list, Catppuccin theming, 120 FPS animations
- **App Launcher**: Application indexing, nucleo fuzzy search, usage tracking, NSWorkspace launching
- **Global Hotkey & System**: Hotkey registration, system commands (sleep/lock/restart/etc.), Spotlight file search

**Performance targets**: Cold start <100ms, hotkey <50ms, search <30ms, memory <50MB, 120 FPS

---

## Clarifying Questions

### 1. Window Dimensions & Positioning

The launcher window is the primary UI surface. Key decisions needed:

- **Window size**: What fixed dimensions should the launcher use? Raycast is ~680×400px, Spotlight is ~680×variable. Should PhotonCast have:
  - Fixed dimensions (e.g., 600×400px as suggested)?
  - Fixed width with dynamic height based on result count?
  - User-configurable dimensions?

- **Screen positioning**: Where should the window appear?
  - Centered horizontally, offset vertically from top (like Spotlight)?
  - Exact center of screen?
  - User-configurable position?
  - Should it remember last position?

- **Multi-monitor behavior**: On which display should the window appear?
  - Display with mouse cursor?
  - Display with currently focused window?
  - Primary display only?
  - User preference?

---

### 2. Search Results Display & Limits

- **Maximum results shown**: How many results should be visible at once?
  - Should there be a hard limit (e.g., 10 items visible, scroll for more)?
  - Or virtual scrolling through all matches?
  - For Phase 1, is 10-15 visible results sufficient?

- **Result grouping**: In Phase 1, we have apps and system commands. Should results be:
  - Mixed and ranked purely by relevance score?
  - Grouped by type (Apps section, Commands section)?
  - Prioritize certain types (always show matching apps above commands)?

- **Result item information**: What should each result item display?
  - Icon + Name + Subtitle (path or description)?
  - Should we show keyboard shortcut hints (e.g., "⌘1" for first result)?
  - Match highlighting in the name?

---

### 3. Search Ranking Algorithm

nucleo provides fuzzy matching scores, but final ranking needs consideration:

- **Usage frequency weight**: How heavily should past usage influence ranking?
  - Simple: Most recently used at top?
  - Frecency: Balance frequency + recency (like Firefox)?
  - Pure relevance: Ignore usage, rank purely by match quality?

- **Tie-breaking**: When two items have similar match scores, what's the tiebreaker?
  - Alphabetical?
  - Usage count?
  - Recency of last use?

- **Boost factors**: Should certain apps get ranking boosts?
  - System apps (Finder, Safari) boosted?
  - Apps in /Applications ranked above ~/Applications?

---

### 4. Hotkey Conflict Handling

`Cmd+Space` is the default, but conflicts with Spotlight:

- **Conflict detection**: Should PhotonCast detect existing hotkey bindings?
  - Warn user if Cmd+Space is taken?
  - Automatically suggest alternative (e.g., `Opt+Space`)?

- **Fallback behavior**: If the hotkey fails to register:
  - Show error notification?
  - Silently retry?
  - Prompt user to disable Spotlight first?

- **Hotkey customization**: How much flexibility in Phase 1?
  - Full customization (any key + any modifier combo)?
  - Limited preset options (Cmd+Space, Opt+Space, Ctrl+Space)?
  - Double-tap modifier support (e.g., double-tap Cmd)?

---

### 5. Accessibility Permission Flow

Global hotkeys require Accessibility permission:

- **First-run experience**: When the app first launches without permission:
  - Show in-app explainer modal with "Open System Preferences" button?
  - Direct user to Privacy settings automatically?
  - Allow app to function without hotkey (click menu bar icon instead)?

- **Permission denied state**: If user denies or revokes permission:
  - Show persistent banner in the app?
  - Disable global hotkey silently?
  - Check permission state on each launch?

- **Permission granted state**: How to notify user that everything is working?
  - Success toast notification?
  - Visual indicator in settings?

---

### 6. Edge Cases & Error States

- **No results found**: When search yields zero matches:
  - Show "No results" placeholder with suggestions?
  - Offer fallback action (e.g., "Search in Finder")?
  - Empty state with keyboard shortcut hints?

- **Slow indexing**: If initial app indexing takes >2 seconds:
  - Show loading indicator in search bar?
  - Allow searching partial index?
  - Show "Indexing..." status message?

- **App launch failures**: If an app fails to launch (corrupted, permission denied):
  - Show error notification?
  - Offer to reveal app in Finder instead?
  - Log error for debugging?

- **System command failures**: If a system command fails (e.g., user lacks admin rights for restart):
  - Show error message explaining why?
  - Request authentication via macOS prompt?

---

### 7. Theme & Visual Customization Scope

Catppuccin with 4 flavors (Latte, Frappé, Macchiato, Mocha) is planned. Questions:

- **System theme sync**: Should PhotonCast automatically switch between Latte (light) and Mocha (dark) based on macOS appearance?
  - Or always respect user's explicit theme choice?
  - Show theme selector only, or also "Auto" option?

- **Accent color**: Should users be able to customize the accent color in Phase 1?
  - 14 Catppuccin accent options?
  - Or ship with Mauve as default, add customization later?

- **Animation preferences**: Should there be an option to reduce motion?
  - Respect macOS "Reduce motion" accessibility setting?
  - Explicit toggle in PhotonCast settings?

---

### 8. File Search Scope (via Spotlight)

Basic file search is planned for Phase 1:

- **Search trigger**: How does file search activate?
  - Automatic when query looks like a filename?
  - Explicit prefix (e.g., `file:` or `f:` prefix)?
  - Search files alongside apps in unified results?

- **Result limits**: How many file results to show?
  - Fixed limit (e.g., 5 files max)?
  - Separate "Files" section with expandable results?

- **File actions**: What can users do with file results?
  - Open with default app only?
  - Reveal in Finder?
  - Quick Look preview (Phase 1 or later)?

---

## Visual References Request

To ensure the UI/UX meets expectations, it would be helpful to see:

1. **Screenshots or mockups** of any specific visual style you have in mind
2. **Reference apps**: Which launcher's visual design do you prefer?
   - Raycast (modern, colorful icons, grouped results)
   - Alfred (minimal, text-focused)
   - Spotlight (clean macOS native feel)
   - Other launchers?
3. **Animation style**: Do you have examples of the animation feel you want?
   - Quick snap-in appearance?
   - Smooth fade + scale?
   - Spring physics?

---

---

## Decisions (User Responses)

### 1. Window Dimensions & Positioning
**Decision: Follow Raycast**
- Window size: ~680px wide, dynamic height based on results
- Centered horizontally, offset from top (~20% from screen top)
- Multi-monitor: Display with mouse cursor
- Smooth appearance animation

### 2. Search Results Display & Limits
**Decision: Follow Raycast**
- Mixed results ranked by relevance with type grouping (Apps, Commands, Files sections)
- ~8-10 visible results with scroll
- Keyboard shortcut hints (⌘1, ⌘2, etc.) for quick selection
- Match highlighting in result names
- Icon + Name + Subtitle layout

### 3. Search Ranking Algorithm
**Decision: Implement all three in phases**
- **Phase 1a**: Pure match quality from nucleo
- **Phase 1b**: Add frecency (frequency + recency weighting)
- **Phase 1c**: Add boost factors for system apps and /Applications priority
- Tiebreaker: Usage count → Recency → Alphabetical

### 4. Hotkey Conflict Handling
**Decision: Yes to all**
- Detect existing hotkey bindings (especially Spotlight's Cmd+Space)
- Warn user if conflict detected
- Auto-suggest alternatives (Opt+Space, Ctrl+Space)
- Full customization support (any key + modifier combo)
- Double-tap modifier support (e.g., double-tap Cmd)
- Error notification + guided resolution if registration fails

### 5. Accessibility Permission Flow
**Decision: Robust guided workflow**
- In-app explainer modal on first launch
- Step-by-step visual guide to grant permissions
- Direct "Open System Preferences" button
- Real-time permission status checking
- Clear success/failure indicators
- Fallback: Menu bar icon activation if hotkey unavailable
- Re-prompt if permission revoked with helpful guidance

### 6. Edge Cases & Error States
**Decision: Fail fast**
- Clear, immediate error messages
- No silent failures - always inform user
- Actionable error states with recovery options
- Logging for debugging
- Graceful degradation where possible (e.g., partial index search)

### 7. Theme & Visual Customization
**Decision: Full implementation as specified**
- System theme sync: Auto-switch Latte (light) / Mocha (dark) with macOS
- Manual override option available
- All 14 Catppuccin accent colors from launch
- Respect macOS "Reduce motion" accessibility setting
- Explicit toggle for reduced animations in PhotonCast settings

### 8. File Search Scope
**Decision: Follow Raycast pattern**
- Unified search: Files appear alongside apps in results
- Automatic detection (no prefix required)
- Separate "Files" section in results
- ~5 file results initially, expandable
- Actions: Open with default app, Reveal in Finder
- Quick Look preview deferred to Phase 2

---

## Visual Reference Task

**Action Required**: Study current Raycast UI and create detailed mockups covering:
- Main launcher window (search bar, results list, detail panel)
- Result item states (normal, hover, selected)
- Empty states and loading states
- Permission request flow screens
- Error state displays
- Theme variations (all 4 Catppuccin flavors)
- Animation transitions

---

## Summary - Final Decisions

| # | Topic | Decision |
|---|-------|----------|
| 1 | Window | Follow Raycast (~680px, centered-top, cursor display) |
| 2 | Results | Grouped sections, shortcuts, highlighting |
| 3 | Ranking | Phased: match quality → frecency → boosts |
| 4 | Hotkey | Full detection, warnings, customization |
| 5 | Permissions | Robust guided in-app workflow |
| 6 | Edge cases | Fail fast with clear messaging |
| 7 | Themes | Full Catppuccin + system sync + reduce motion |
| 8 | Files | Unified search, Raycast-style sections |

---

*Questions generated: 2026-01-15*
*Decisions recorded: 2026-01-15*
