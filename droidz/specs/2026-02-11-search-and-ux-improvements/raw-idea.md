# Search & UX Improvements - 4 Enhancements

## Status
**Requirements finalized** — All decisions resolved, ready for spec writing.

## Context
- **Project**: PhotonCast (Rust-based macOS launcher)
- **Date**: 2026-02-11
- **Branch**: TBD

## Description

Four improvements to PhotonCast's search, indexing, and app management, in priority order:

### 1. Smarter App Indexing (Highest Priority)
Fix `/System/Library/CoreServices` indexing that pollutes search with system helper apps. Filter CoreServices apps by `LSUIElement=false` in Info.plist to keep only user-facing apps (like Finder). Add user-configurable search scope in preferences so users can add/remove scan directories.

### 2. Frecency-Based Result Sorting (High Priority)
Heavily-used apps should always dominate search results. Bump `FRECENCY_MULTIPLIER` from 10.0 to 25-50x. Add per-query frecency tracking (like Raycast) so selecting "Shortwave" for "sh" boosts it specifically for future "sh" queries. Keep 72-hour half-life. No pinning needed.

### 3. Better Fuzzy Matching (Medium Priority)
Add explicit word-boundary/acronym bonus scoring on top of nucleo's base matching. "ss" → System Settings, "vsc" → Visual Studio Code should get bonuses when characters match word starts. Keep spread factor at 1.5x.

### 4. Faster App Quitting (Lower Priority)
Make quit fire-and-forget: send `terminate()`, immediately dismiss launcher, no polling loop, no toast. Removes the 5-second blocking timeout that freezes the UI.

## Current Implementation Notes

### Search & Ranking
- **Fuzzy matching**: `nucleo` crate with smart case, Unicode normalization, prefix bonus, spread factor 1.5x
- **Frecency**: 72-hour half-life exponential decay, `frequency * recency` formula
- **Ranking formula**: `(match_score + frecency * 10.0) * path_boost * match_boost`
- **Boosts**: System apps 1.2x, /Applications 1.1x, exact match 2.0x, prefix match 1.5x
- **Usage tracking**: SQLite-backed, tracks app launches, commands, file opens

### App Indexing
- **Scan paths**: `/Applications`, `/Applications/Utilities`, `/System/Applications`, `/System/Applications/Utilities`, `/System/Library/CoreServices`, `~/Applications`
- **Exclusions**: `*.prefPane`, `*Uninstaller*.app`, `*.app/Contents/*`
- **Scanner**: Async with 10s timeout, 20 concurrent metadata parsers, deduplication via canonical paths

### App Quit Flow
- `quit_app_by_bundle_id()`: Finds app by bundle ID → sends `NSRunningApplication.terminate()` → polls every 100ms for 5s
- `force_quit_app_action()`: Uses `NSRunningApplication.forceTerminate()` or SIGKILL fallback
- UI handler calls `photoncast_apps::quit_app_by_bundle_id()` synchronously, then `self.hide(cx)`

## Key Decisions Made
- **Frecency multiplier**: Bump to 25-50x (Option A)
- **Per-query frecency**: Yes, track per query prefix (Option B)
- **Half-life**: Keep 72 hours
- **Word boundary matching**: Add explicit bonus scoring (Option B)
- **CoreServices**: Filter by LSUIElement=false (Option C)
- **Search scope**: User-configurable in preferences (Option B)
- **Quit behavior**: Fire-and-forget, silent dismiss (Option A)
- **No pinning**: Frecency handles it
- **No learning period**: Match quality handles new apps
- **No quit toast**: Silent dismiss
