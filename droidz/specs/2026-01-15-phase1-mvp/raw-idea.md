# PhotonCast Phase 1 MVP

## Overview
PhotonCast Phase 1 MVP - A Rust/GPUI-based macOS launcher application that serves as a fast, native alternative to Raycast/Alfred without AI features or subscriptions.

## Core Features

### 1. Core UI Framework (Sprint 1, Weeks 1-4)
- Main launcher window (overlay mode)
- Search bar component with real-time input
- Results list with keyboard navigation (↑↓, Enter, Esc)
- Theme system with Catppuccin colors (4 flavors: Latte, Frappé, Macchiato, Mocha)
- Smooth animations targeting 120 FPS

### 2. App Launcher (Sprint 2, Weeks 5-8)
- Application indexing from /Applications, ~/Applications, /System/Applications
- Parse app metadata (name, icon, bundle ID)
- Fuzzy search using nucleo matcher
- Usage frequency tracking for intelligent ranking
- App launching via NSWorkspace APIs
- Handle app aliases and symlinks
- Background re-indexing on file system changes

### 3. Global Hotkey & System Commands (Sprint 3, Weeks 9-12)
- Global hotkey registration (Cmd+Space default)
- Customizable key combinations
- Accessibility permissions handling
- System commands:
  - `sleep` - Put Mac to sleep
  - `lock` - Lock screen
  - `restart` - Restart Mac
  - `shutdown` - Shut down Mac
  - `logout` - Log out current user
  - `empty trash` - Empty Trash
  - `screen saver` - Start screen saver
- Basic file search via Spotlight/NSMetadataQuery
- Open files with default application
- Reveal files in Finder

## Tech Stack
- **Language:** Rust (Edition 2021, MSRV 1.75+)
- **GUI:** GPUI + gpui-component
- **Async:** Tokio
- **Fuzzy Search:** nucleo
- **Storage:** rusqlite
- **macOS APIs:** objc2, cocoa, core-foundation

## Performance Targets
| Metric | Target |
|--------|--------|
| Cold start time | < 100ms |
| Hotkey response | < 50ms |
| Search latency | < 30ms |
| Memory usage | < 50MB |
| UI rendering | 120 FPS |

## Acceptance Criteria
- Window appears/disappears in under 50ms
- Keyboard navigation is intuitive
- Index 200+ apps in under 2 seconds
- Search results appear in under 30ms
- Correct app launches on Enter
- All themes pass WCAG AA contrast standards
- Hotkey responds within 50ms
- System commands execute correctly
- File search returns results in under 100ms

## Timeline
- **Duration:** 3 months (12 weeks)
- **Release:** v0.1.0-alpha
