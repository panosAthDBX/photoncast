# PhotonCast - Development Roadmap

> Phased approach from MVP to full-featured launcher

## Roadmap Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Phase 1: MVP          │  Phase 2: v1.0       │  Phase 3: Extensions       │
│  (Months 1-3)          │  (Months 4-6)        │  (Months 7-9)              │
│                        │                       │                            │
│  • App launcher        │  • Clipboard history │  • Raycast API shim        │
│  • Global hotkey       │  • Window management │  • Extension sidecar       │
│  • Basic file search   │  • Calculator        │  • Store integration       │
│  • System commands     │  • Custom commands   │  • 80%+ compatibility      │
│                        │  • Native extensions │                            │
├─────────────────────────────────────────────────────────────────────────────┤
│  Phase 4: Ecosystem    │  Phase 5+: Future                                 │
│  (Months 10-12)        │  (Months 13+)                                     │
│                        │                                                    │
│  • Extension store UI  │  • Themes & customization                         │
│  • Update management   │  • Workflow automation                            │
│  • 90%+ compatibility  │  • Sync (opt-in)                                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Minimum Viable Product (MVP)

**Timeline:** Months 1-3  
**Goal:** Core launcher that's faster than alternatives  
**Release:** v0.1.0-alpha

### Sprint 1: Foundation (Weeks 1-4)

#### 1.1 Project Setup ✓ Priority: Critical
- [x] Initialize Cargo workspace
- [x] Configure GPUI with gpui-component
- [x] Set up development tooling (CI, linting, formatting)
- [x] Create base application structure

#### 1.2 Core UI Framework
- [ ] Implement main launcher window (overlay mode)
- [ ] Create search bar component with real-time input
- [ ] Build results list component with keyboard navigation
- [ ] Design and implement theme system (dark mode first)
- [ ] Add smooth animations and transitions

**Acceptance Criteria:**
- Window appears/disappears in under 50ms
- Keyboard navigation is intuitive (↑↓, Enter, Esc)
- UI renders at consistent 120 FPS

### Sprint 2: App Launcher (Weeks 5-8)

#### 2.1 Application Indexing
- [ ] Discover apps from /Applications, ~/Applications, /System/Applications
- [ ] Parse app metadata (name, icon, bundle ID)
- [ ] Build initial search index using nucleo fuzzy matcher
- [ ] Implement background re-indexing on file system changes

#### 2.2 Search & Launch
- [ ] Implement fuzzy search with intelligent ranking
- [ ] Track usage frequency for result ordering
- [ ] Launch apps via NSWorkspace APIs
- [ ] Handle app aliases and symlinks

#### 2.3 Theming System
- [ ] Implement Catppuccin color palette (4 flavors)
  - [ ] Latte (light mode)
  - [ ] Frappé (dark - low contrast)
  - [ ] Macchiato (dark - medium contrast)
  - [ ] Mocha (dark - high contrast)
- [ ] System theme detection and auto-switching
- [ ] Customizable accent color (14 options)
- [ ] Semantic color mapping for UI components

**Acceptance Criteria:**
- Index 200+ apps in under 2 seconds
- Search results appear in under 30ms
- Correct app launches on Enter
- All themes pass WCAG AA contrast standards

### Sprint 3: Global Hotkey & System (Weeks 9-12)

#### 3.1 Global Hotkey
- [ ] Register global hotkey (default: Cmd+Space)
- [ ] Handle hotkey conflicts gracefully
- [ ] Support customizable key combinations
- [ ] Request accessibility permissions properly

#### 3.2 System Commands
- [ ] Implement built-in commands:
  - `sleep` - Put Mac to sleep
  - `lock` - Lock screen
  - `restart` - Restart Mac
  - `shutdown` - Shut down Mac
  - `logout` - Log out current user
  - `empty trash` - Empty Trash
  - `screen saver` - Start screen saver

#### 3.3 Basic File Search
- [ ] Query Spotlight index via NSMetadataQuery
- [ ] Display file results with icons and paths
- [ ] Open files with default application
- [ ] Reveal files in Finder

**Acceptance Criteria:**
- Hotkey responds within 50ms
- System commands execute correctly
- File search returns results in under 100ms

### MVP Success Metrics
| Metric | Target |
|--------|--------|
| Cold start time | < 100ms |
| Hotkey response | < 50ms |
| Search latency | < 30ms |
| Memory usage | < 50MB |
| Crash rate | < 1% |

---

## Phase 2: Version 1.0

**Timeline:** Months 4-6  
**Goal:** Feature parity with basic Raycast/Alfred use cases  
**Release:** v1.0.0

### Sprint 4: Productivity Features (Weeks 13-16)

#### 4.1 Clipboard History
- [ ] Monitor pasteboard changes
- [ ] Store clipboard history in SQLite
- [ ] Support text, images, and file references
- [ ] Configurable history limit (default: 1000 items)
- [ ] Quick paste via keyboard shortcut
- [ ] Search through clipboard history

#### 4.2 Built-in Calculator
- [ ] Parse and evaluate natural language math expressions
- [ ] Support basic operations (+, -, *, /, ^, %)
- [ ] Support parentheses and order of operations
- [ ] Add common functions (sqrt, sin, cos, tan, log, ln, abs)
- [ ] Unit conversions:
  - [ ] Length (mm, cm, m, km, in, ft, yd, mi)
  - [ ] Weight (mg, g, kg, oz, lb, ton)
  - [ ] Volume (ml, l, tsp, tbsp, cup, pt, qt, gal)
  - [ ] Temperature (C, F, K)
  - [ ] Data (B, KB, MB, GB, TB, PB)
  - [ ] Speed (m/s, km/h, mph, knots)
- [ ] Currency conversions:
  - [ ] Major fiat currencies (USD, EUR, GBP, JPY, etc.)
  - [ ] Cryptocurrency (BTC, ETH, USDT, etc.)
  - [ ] Background rate updates
- [ ] Date & time calculations:
  - [ ] Relative dates ("monday in 3 weeks", "35 days ago")
  - [ ] Days until/since calculations
  - [ ] Timezone conversions ("5pm ldn in sf")
- [ ] Copy result to clipboard

**Acceptance Criteria:**
- Clipboard captures all copy events
- Calculator evaluates in under 5ms
- History is searchable
- Currency rates updated every 6 hours

### Sprint 5: Window Management (Weeks 17-20)

#### 5.1 Window Commands
- [ ] Implement window positioning commands:
  - `left half` / `right half`
  - `top half` / `bottom half`
  - `maximize` / `center`
  - `quarters` (top-left, top-right, etc.)
- [ ] Use Accessibility APIs for window control
- [ ] Smooth animation during resize
- [ ] Multi-monitor support

#### 5.2 Quick Links
- [ ] User-defined URL bookmarks
- [ ] Configurable keywords/aliases
- [ ] Open in default browser
- [ ] Import from browsers (Safari, Chrome, Firefox)

#### 5.3 Calendar Integration
- [ ] Connect to macOS native calendar (EventKit)
- [ ] Display upcoming events with color coding
- [ ] Conference call detection (Zoom, Meet, Teams)
- [ ] One-click join meeting button
- [ ] Commands: My Schedule, Today's Events, This Week
- [ ] Quick actions: Join call, Copy details, Email attendees

#### 5.4 App Management
- [ ] App uninstaller with related file cleanup
  - [ ] Scan ~/Library for app-related files
  - [ ] Show space to be freed
  - [ ] Optional: remove preferences, caches, logs
- [ ] Force quit applications
- [ ] Show app info (version, size, location)

#### 5.5 Sleep Timer
- [ ] Schedule delayed system actions
  - [ ] Sleep in X minutes/hours
  - [ ] Shut down at specific time
  - [ ] Lock after delay
- [ ] Show remaining time
- [ ] Cancel scheduled timer
- [ ] Natural language parsing ("sleep in 30 min")

**Acceptance Criteria:**
- Windows resize smoothly
- Multi-monitor detection works
- Quick links open instantly
- Calendar events load in under 500ms
- App uninstall cleans up 90%+ of related files

### Sprint 6: Native Extension System (Weeks 21-24)

#### 6.1 Native Extension Architecture
- [ ] Define native extension manifest format (TOML)
- [ ] Implement Rust extension loading and lifecycle
- [ ] Create extension API (search, UI, storage)
- [ ] Hot-reload support for development

#### 6.2 Custom Commands
- [ ] User-defined command shortcuts
- [ ] Shell script execution
- [ ] Environment variable support
- [ ] Output capture and display
- [ ] Error handling and notifications

#### 6.3 First-Party Native Extensions
- [ ] Create example extension templates
- [ ] Build 2-3 reference extensions:
  - GitHub repositories browser
  - System preferences shortcuts
  - Color picker

**Acceptance Criteria:**
- Native extensions load in under 50ms
- API is documented with examples
- Custom commands execute reliably

### v1.0 Success Metrics
| Metric | Target |
|--------|--------|
| Daily active users | 500+ |
| GitHub stars | 1,000+ |
| Native extensions | 5+ |
| User-reported bugs | < 20 open |
| NPS score | 50+ |

---

## Phase 3: Raycast Extension Compatibility

**Timeline:** Months 7-9  
**Goal:** Run Raycast extensions natively  
**Release:** v1.1.0 - v1.3.0

### Sprint 7: Extension Runtime Foundation (Weeks 25-28)

#### 7.1 Node.js Sidecar Process
- [ ] Implement Node.js sidecar binary bundling
- [ ] Create IPC protocol (JSON-RPC over stdio)
- [ ] Build process lifecycle management (spawn, monitor, restart)
- [ ] Implement sandboxed execution environment
- [ ] Add resource limits and timeout handling

#### 7.2 Raycast API Shim
- [ ] Implement `@raycast/api` compatibility layer
- [ ] Core UI components:
  - [ ] `List` component with sections and items
  - [ ] `Grid` component with columns
  - [ ] `Detail` component with markdown
  - [ ] `Form` component with inputs
  - [ ] `Action` and `ActionPanel` components
- [ ] Core hooks:
  - [ ] `useNavigation` - push/pop views
  - [ ] `useCachedPromise` - async data loading
  - [ ] `useFetch` - HTTP requests
- [ ] Storage APIs:
  - [ ] `LocalStorage` - key-value storage
  - [ ] `Cache` - temporary caching

**Acceptance Criteria:**
- Sidecar process starts in under 200ms
- Basic Raycast extensions load and render
- IPC latency under 10ms

### Sprint 8: API Completeness (Weeks 29-32)

#### 8.1 Raycast API - Utilities
- [ ] `Clipboard` - copy/paste operations
- [ ] `showToast` - notifications
- [ ] `showHUD` - brief messages
- [ ] `open` / `openExtensionPreferences`
- [ ] `getPreferenceValues` - extension preferences
- [ ] `environment` - extension context

#### 8.2 Raycast API - Advanced
- [ ] `OAuth` - basic OAuth flows
- [ ] `Keyboard` - keyboard shortcuts
- [ ] `AI` stub - graceful "not supported" message
- [ ] `Browser Extension` stub - not supported

#### 8.3 macOS-specific API Stubs
- [ ] `runAppleScript` - log warning, throw error
- [ ] `Application` APIs - partial support for installed apps
- [ ] `System Utilities` - platform-appropriate equivalents

**Acceptance Criteria:**
- 80%+ of non-AI Raycast extensions work
- Clear error messages for unsupported features
- Extension preferences UI working

### Sprint 9: Store Integration (Weeks 33-36)

#### 9.1 Raycast Store Browser
- [ ] Implement Raycast Store API client
- [ ] Build extension search and discovery UI
- [ ] Display extension metadata, screenshots, ratings
- [ ] Show compatibility status for each extension

#### 9.2 Extension Installation
- [ ] Download extension packages from Store
- [ ] Extract and install to local directory
- [ ] Install npm dependencies automatically
- [ ] Build/bundle extension code
- [ ] Register extension in local database

#### 9.3 Extension Management
- [ ] List installed extensions
- [ ] Enable/disable extensions
- [ ] Uninstall extensions
- [ ] Check for and apply updates

**Acceptance Criteria:**
- Can browse and search Raycast Store
- Install extensions with one click
- Compatibility warnings shown before install

### Phase 3 Success Metrics
| Metric | Target |
|--------|--------|
| Raycast API coverage | 80%+ |
| Extension compatibility | 80%+ of top 100 |
| Store browsing | Fully functional |
| Install success rate | 90%+ |

---

## Phase 4: Extension Ecosystem Polish

**Timeline:** Months 10-12  
**Goal:** Production-ready extension support  
**Release:** v1.4.0 - v2.0.0

### Planned Work

- [ ] Extension auto-updates
- [ ] Version pinning and rollback
- [ ] Extension ratings and reviews display
- [ ] Performance optimizations for extension loading
- [ ] Better error handling and recovery
- [ ] Extension development tools (CLI, docs)
- [ ] Compatibility database with known issues

---

## Phase 5: Future Development

**Timeline:** Month 13+  
**Goal:** Advanced features and ecosystem growth  
**Release:** v2.x

### Planned Features (Prioritized)

#### 5.1 Enhanced Search
- [ ] File content search (using tantivy)
- [ ] Email search integration
- [ ] Calendar events search
- [ ] Contacts search
- [ ] Notes search

#### 5.2 Themes & Customization
- [ ] Multiple theme support
- [ ] Theme editor / custom CSS-like syntax
- [ ] Icon pack support
- [ ] Font customization
- [ ] Compact/comfortable view modes

#### 5.3 Workflow Automation
- [ ] Multi-step workflows (like Alfred)
- [ ] Conditional logic
- [ ] Variable passing between steps
- [ ] Visual workflow builder
- [ ] Workflow sharing

#### 5.4 Native Extension Marketplace
- [ ] Central extension registry
- [ ] Extension discovery and installation
- [ ] Version management and updates
- [ ] Extension ratings and reviews
- [ ] Developer tools and documentation

#### 5.5 Optional Sync (Privacy-First)
- [ ] Opt-in settings sync
- [ ] iCloud integration (no third-party servers)
- [ ] Export/import configuration
- [ ] Selective sync (what to sync)

#### 5.6 Additional Built-in Features
- [ ] Snippets / Text Expansion
  - [ ] Static text snippets with keywords
  - [ ] Dynamic placeholders ({date}, {time}, {clipboard})
  - [ ] Cursor positioning after expansion
  - [ ] Import from TextExpander/Alfred
- [ ] Emoji Picker
  - [ ] Search by name and keywords
  - [ ] Recently used section
  - [ ] Skin tone variants
  - [ ] Categories and favorites
- [ ] Color Picker
  - [ ] Screen eyedropper tool
  - [ ] Format conversion (HEX, RGB, HSL, Swift, etc.)
  - [ ] Color palette storage
  - [ ] Recent colors history
- [ ] System Preferences Shortcuts
  - [ ] Quick access to all macOS settings panels
  - [ ] Display, Sound, Network, Bluetooth, Privacy
  - [ ] Keyboard, Trackpad, Accessibility
  - [ ] Battery, Users, Date & Time

### Future Exploration (Not Committed)

| Feature | Rationale | Priority |
|---------|-----------|----------|
| **Password manager integration** | 1Password/Bitwarden lookup | Medium |
| **Translation** | Quick inline translation | Low |
| **Menu bar mode** | Alternative access method | Low |
| **Keyboard shortcut viewer** | Show app shortcuts | Low |
| **Contacts search** | Find and call/email contacts | Medium |
| **Notes search** | Search Apple Notes | Low |

---

## Release Schedule

| Version | Milestone | Target Date | Status |
|---------|-----------|-------------|--------|
| v0.1.0-alpha | MVP complete | Month 3 | 🔄 In Progress |
| v0.1.0-beta | Public beta | Month 3.5 | ⏳ Planned |
| v0.2.0 | Stability fixes | Month 4 | ⏳ Planned |
| v1.0.0 | Full release | Month 6 | ⏳ Planned |
| v1.1.0 | Raycast extension runtime | Month 7 | ⏳ Planned |
| v1.2.0 | Raycast API completeness | Month 8 | ⏳ Planned |
| v1.3.0 | Raycast Store integration | Month 9 | ⏳ Planned |
| v1.4.0 | Extension ecosystem polish | Month 10 | ⏳ Planned |
| v2.0.0 | Advanced features | Month 12+ | 📋 Roadmap |

---

## Development Approach

### Iteration Principles

1. **Ship Early, Iterate Fast**  
   Get alpha releases out quickly for feedback

2. **Performance Regression Prevention**  
   Every PR must not regress key metrics

3. **User-Driven Prioritization**  
   Feature priority informed by community feedback

4. **Documentation First**  
   Features aren't done until documented

### Quality Gates

| Gate | MVP | v1.0 | Future |
|------|-----|------|--------|
| Unit test coverage | 60% | 80% | 80% |
| Integration tests | Core paths | All features | All features |
| Performance benchmarks | Key metrics | Full suite | Full suite |
| Accessibility audit | Basic | WCAG 2.1 AA | WCAG 2.1 AA |
| Security review | Basic | External audit | Continuous |

---

## Community Involvement

### How to Contribute

- **Bug Reports:** GitHub Issues with reproduction steps
- **Feature Requests:** GitHub Discussions
- **Code Contributions:** PRs welcome (see CONTRIBUTING.md)
- **Extensions:** Build and share via extension registry
- **Documentation:** Help improve docs

### Governance

- Core team makes final decisions
- RFCs for major changes
- Community voting on feature priorities
- Transparent roadmap updates

---

*Roadmap last updated: January 2026*  
*Next review: March 2026*
