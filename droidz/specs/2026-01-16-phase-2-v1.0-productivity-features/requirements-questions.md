# Phase 2: Version 1.0 - Productivity Features
## Clarifying Questions

> Generated: 2026-01-16
> Phase: 2 (Sprints 4-6, Weeks 13-24)
> Goal: Feature parity with basic Raycast/Alfred use cases

---

## Sprint 4: Productivity Features

### 4.1 Clipboard History

#### Core Functionality

1. **What should the default global hotkey be for Clipboard History?**
   Context: Raycast uses a configurable hotkey. Common choices are `Cmd+Shift+V` or `Ctrl+Shift+V`.
   Options: A) `Cmd+Shift+V` (matches Raycast), B) `Ctrl+V` (double-tap), C) User-defined only, D) Alias-based only (no default hotkey)
   Suggested default: A) `Cmd+Shift+V`

2. **What should the default history retention period be?**
   Context: Raycast offers up to 3 months for Pro users. More history = more disk space.
   Options: A) 7 days, B) 30 days, C) 90 days, D) Unlimited (until manual clear)
   Suggested default: B) 30 days

3. **What should the maximum number of clipboard items stored?**
   Context: Raw idea suggests 1000 items. More items = more memory on load.
   Options: A) 500 items, B) 1000 items, C) 5000 items, D) Configurable with default of 1000
   Suggested default: D) Configurable with default of 1000

4. **Should clipboard history persist across app restarts?**
   Context: Users expect history to survive restarts and system reboots.
   Options: A) Yes (SQLite), B) Yes (encrypted SQLite), C) Session-only (memory)
   Suggested default: B) Yes (encrypted SQLite) - matches Raycast's security approach

5. **What encryption should be used for stored clipboard data?**
   Context: Raycast encrypts clipboard history locally. Sensitive data may be copied.
   Options: A) No encryption, B) AES-256 with machine key, C) Keychain-backed encryption, D) User-provided passphrase
   Suggested default: B) AES-256 with machine-derived key

#### Content Types

6. **Should we support images in clipboard history?**
   Context: Raw idea says yes. Increases storage requirements significantly.
   Options: A) Yes (store full images), B) Yes (thumbnail + path reference), C) No (text only)
   Suggested default: A) Yes (store full images with configurable toggle)

7. **What maximum image size should be stored?**
   Context: Very large screenshots/images could bloat the database.
   Options: A) 5MB, B) 10MB, C) 25MB, D) No limit
   Suggested default: B) 10MB (with option to increase)

8. **Should we support file references in clipboard history?**
   Context: When files are copied (Cmd+C on file), store the path reference.
   Options: A) Yes (store path + icon + metadata), B) Yes (path only), C) No
   Suggested default: A) Yes (store path + icon + metadata)

9. **Should we detect and display colors (Hex/RGB) specially?**
   Context: Designers often copy color codes. Special display could show color preview.
   Options: A) Yes (with color swatch preview), B) Yes (basic text with tag), C) No
   Suggested default: A) Yes (with color swatch preview)

10. **Should rich text (HTML/RTF) be preserved or stripped?**
    Context: Copying from web/docs includes formatting. Users may want plain text.
    Options: A) Preserve formatting (offer "paste as plain text" action), B) Strip by default, C) Configurable
    Suggested default: A) Preserve formatting with paste options

11. **Should we support URL detection and preview?**
    Context: URLs could show favicons and page titles like Raycast.
    Options: A) Yes (fetch favicon + title in background), B) Yes (basic detection, no fetch), C) No
    Suggested default: A) Yes (fetch in background with caching)

#### Privacy & Security

12. **Which apps should be excluded from clipboard monitoring by default?**
    Context: Password managers should never have items captured.
    Options: List specific bundle IDs
    Suggested defaults: 
    - `com.1password.1password` (1Password)
    - `com.agilebits.onepassword7` (1Password 7)
    - `com.bitwarden.desktop` (Bitwarden)
    - `com.lastpass.LastPass` (LastPass)
    - `com.apple.keychainaccess` (Keychain Access)
    - `com.dashlane.Dashlane` (Dashlane)
    
    Should users be able to add/remove from this list? **Yes**

13. **Should we respect "transient" pasteboard items (marked as sensitive)?**
    Context: Some apps mark clipboard items as transient (password managers do this).
    Options: A) Yes (never store transient items), B) Configurable, C) No (store everything)
    Suggested default: A) Yes (never store transient items)

14. **Should we implement "concealed" mode for sensitive pastes?**
    Context: Raycast can paste without adding to history (for one-time sensitive data).
    Options: A) Yes (action: "Paste and Don't Save"), B) No
    Suggested default: A) Yes

15. **Should clipboard history be clearable with a confirmation?**
    Context: Destructive action, should require confirmation.
    Options: A) Yes (with confirmation dialog), B) Yes (no confirmation), C) Command-only
    Suggested default: A) Yes (with confirmation dialog)

#### Features

16. **Should users be able to pin clipboard items?**
    Context: Frequently-used snippets can be pinned to top of history.
    Options: A) Yes (pinned items shown first), B) Yes (separate "Pinned" section), C) No
    Suggested default: B) Yes (separate "Pinned" section)

17. **Should pinned items count against the history limit?**
    Context: If limit is 1000, should 10 pinned items leave 990 regular slots?
    Options: A) Yes (count against limit), B) No (separate allocation)
    Suggested default: B) No (pinned items are separate from limit)

18. **Should we support merging multiple clipboard items?**
    Context: Select 3 items and combine them into one text block.
    Options: A) Yes (merge action), B) No (v1.0), C) Phase 3+
    Suggested default: C) Phase 3+ (focus on core features first)

19. **Should we add OCR for images to make text searchable?**
    Context: Raycast Pro has OCR for finding text inside screenshot images.
    Options: A) Yes (using macOS Vision framework), B) No (v1.0), C) Phase 3+
    Suggested default: C) Phase 3+ (complex feature, defer)

20. **What primary action should "Enter" perform on a clipboard item?**
    Context: Raycast defaults to "Paste" but some prefer "Copy to clipboard".
    Options: A) Paste directly, B) Copy to clipboard, C) Configurable
    Suggested default: C) Configurable with default of "Paste directly"

#### Sync & Multi-device

21. **Should clipboard history sync across devices?**
    Context: Raycast doesn't sync clipboard. iCloud sync would be complex.
    Options: A) No (local only), B) Yes (iCloud), C) Yes (opt-in iCloud), D) Phase 3+
    Suggested default: A) No (local only) - privacy-first approach

---

### 4.2 Built-in Calculator

#### Math Expressions

22. **What precision should be used for decimal calculations?**
    Context: Financial calculations need high precision. Display vs internal precision.
    Options: A) f64 (standard), B) Decimal128 (high precision), C) Arbitrary precision (rug/bigdecimal)
    Suggested default: A) f64 for most, B) Decimal128 for currency

23. **Should we use a specific crate for expression parsing?**
    Context: Raw idea mentions `evalexpr` or `meval`.
    Options: A) evalexpr (feature-rich), B) meval (simpler), C) Custom parser (more control)
    Suggested default: A) evalexpr (well-maintained, supports variables/functions)

24. **What mathematical functions should be supported?**
    Context: Raw idea lists several. Need to confirm completeness.
    Current list: `sqrt`, `sin`, `cos`, `tan`, `log`, `ln`, `abs`, `floor`, `ceil`, `round`
    Additional options:
    - Inverse trig: `asin`, `acos`, `atan`
    - Hyperbolic: `sinh`, `cosh`, `tanh`
    - Other: `exp`, `pow`, `mod`, `min`, `max`, `factorial`
    
    **Which additional functions should be included?** Please specify or approve all above.

25. **Should we support variable assignment for multi-step calculations?**
    Context: `x = 5`, then `x * 3` → `15`
    Options: A) Yes (session variables), B) No (v1.0), C) Phase 3+
    Suggested default: B) No (v1.0) - keep calculator simple initially

#### Currency Conversions

26. **Which currency API should we use?**
    Context: Need reliable, free/affordable API with good update frequency.
    Options:
    - A) exchangerate-api.com (free tier: 1500 req/month, updates daily)
    - B) openexchangerates.org (free tier: 1000 req/month)
    - C) frankfurter.app (free, open-source, ECB data)
    - D) Fallback chain: Try A, then B, then C
    
    Suggested default: C) frankfurter.app (free, no API key, ECB official rates)

27. **How often should currency rates be updated?**
    Context: Raw idea says every 6 hours. More frequent = more API calls.
    Options: A) Every hour, B) Every 6 hours, C) Every 24 hours, D) On-demand only
    Suggested default: B) Every 6 hours (balance of freshness and API limits)

28. **Should we cache rates for offline use?**
    Context: Calculator should work offline with last-known rates.
    Options: A) Yes (SQLite cache with timestamp), B) Yes (file cache), C) No
    Suggested default: A) Yes (SQLite cache with timestamp and "rates as of X" display)

29. **Which cryptocurrencies should be supported?**
    Context: Raw idea lists: BTC, ETH, USDT, BNB, XRP, ADA, DOGE, SOL
    Options: A) Above list only, B) Top 20 by market cap, C) User-configurable list
    **Should we include any others by default?** 
    Suggested: Add USDC, MATIC, AVAX, DOT, LINK to make top ~15

30. **Which API should we use for cryptocurrency rates?**
    Context: Need reliable crypto price data.
    Options:
    - A) CoinGecko (free, comprehensive)
    - B) CoinMarketCap (requires API key, more accurate)
    - C) Binance public API (real-time, trading focused)
    
    Suggested default: A) CoinGecko (free, no API key required)

#### Unit Conversions

31. **Should unit conversion be case-insensitive?**
    Context: `5 km` vs `5 KM` vs `5 Km`
    Options: A) Case-insensitive (all work), B) Case-sensitive (standard units only)
    Suggested default: A) Case-insensitive for better UX

32. **Should we support unit abbreviation variations?**
    Context: "kilometers", "km", "kms", "kilometre" all mean the same
    Options: A) Yes (comprehensive aliases), B) Strict abbreviations only
    Suggested default: A) Yes (comprehensive aliases)

33. **Which speed units should be included?**
    Context: Raw idea lists: m/s, km/h, mph, knots
    **Should we add:** ft/s, Mach (speed of sound)?
    Suggested: Add ft/s, defer Mach to later

#### Date & Time Calculations

34. **What date parsing library should we use?**
    Context: Need natural language parsing ("monday in 3 weeks")
    Options:
    - A) chrono-english (NL parsing for chrono)
    - B) dateparser (comprehensive NL support)
    - C) Custom implementation with chrono
    
    Suggested default: B) dateparser or A) chrono-english (evaluate both)

35. **How should timezone abbreviations be resolved?**
    Context: "PST" could mean Pacific Standard Time or Philippine Standard Time
    Options: A) Use common US/EU conventions, B) Prompt for clarification, C) Use system locale preference
    Suggested default: A) Use common conventions with fallback to system locale

36. **Should we support city-based timezone lookups?**
    Context: "time in tokyo", "5pm london in dubai"
    Options: A) Yes (with city database), B) No (timezone codes only)
    Suggested default: A) Yes - matches Raycast behavior

37. **What city database should we use for timezone lookups?**
    Context: Need mapping of city names to IANA timezone identifiers
    Options: A) Bundled minimal database (~500 cities), B) Comprehensive database (~5000 cities), C) Online lookup
    Suggested default: A) Bundled minimal database (top 500 global cities)

#### Calculator UX

38. **Should calculator results show immediately as you type?**
    Context: Real-time evaluation vs explicit "=" or Enter
    Options: A) Real-time (Raycast style), B) Require Enter, C) Debounced (200ms delay)
    Suggested default: A) Real-time with debounce for expensive calculations

39. **What should happen when copying a currency result?**
    Context: `100 usd in eur` → `€92.50`. Copy raw number or formatted?
    Options: A) Formatted (€92.50), B) Raw (92.50), C) Both actions available
    Suggested default: C) Both actions (Enter = formatted, Cmd+Enter = raw)

40. **Should calculator maintain a history of calculations?**
    Context: Raycast has "Calculator History" to recall previous calculations.
    Options: A) Yes (separate command), B) Yes (in main calculator view), C) No
    Suggested default: A) Yes (separate "Calculator History" command)

---

## Sprint 5: Window Management & Productivity

### 5.1 Window Management

#### Layouts & Positions

41. **Should window management require a dedicated hotkey per layout?**
    Context: Raycast encourages hotkeys like `Ctrl+Opt+←` for Left Half.
    Options: A) Yes (recommend hotkeys), B) Command-based only, C) Both with suggestions
    Suggested default: C) Both - commands work, but suggest common hotkeys in onboarding

42. **What default hotkey scheme should we suggest (if any)?**
    Context: Common schemes use Ctrl+Opt or Hyper key
    Options: 
    - A) Ctrl+Opt+Arrow (Raycast style)
    - B) Ctrl+Cmd+Arrow
    - C) No defaults (user assigns)
    
    Suggested default: C) No defaults (avoid conflicts), but provide suggested mappings in preferences

43. **Should layouts support "cycling" behavior?**
    Context: Press "Left Half" twice → cycles between 50% → 33% → 66% width
    Options: A) Yes (cycling), B) No (fixed positions), C) Configurable
    Suggested default: A) Yes (cycling) - matches Magnet/Rectangle behavior

44. **Should we support custom grid layouts?**
    Context: Power users may want arbitrary X×Y grids (e.g., 3×2 for 6 zones)
    Options: A) Yes (v1.0), B) No (preset layouts only), C) Phase 3+
    Suggested default: B) No (v1.0) - focus on standard layouts

45. **Should we include a "snap to edges" feature?**
    Context: Drag window near edge and it snaps to half/quarter
    Options: A) Yes, B) No (command-only window management)
    Suggested default: B) No - keep launcher-focused, avoid daemon complexity

#### Animation & Performance

46. **Should window movements be animated?**
    Context: Smooth animations look polished but may affect performance.
    Options: A) Yes (always), B) Yes (respecting Reduce Motion), C) No (instant)
    Suggested default: B) Yes (respecting Reduce Motion accessibility setting)

47. **What animation duration should be used?**
    Context: Balance between snappy and smooth
    Options: A) 100ms (fast), B) 200ms (balanced), C) 300ms (smooth)
    Suggested default: B) 200ms

48. **Should animation be configurable?**
    Context: Some users prefer instant, others prefer smooth
    Options: A) Yes (in preferences), B) No (fixed), C) Speed slider
    Suggested default: A) Yes (on/off toggle in preferences)

#### Multi-Monitor

49. **How should "Next Display" work with 3+ monitors?**
    Context: Cycle through all? Only adjacent? User-defined order?
    Options: A) Cycle by macOS arrangement, B) Cycle by physical position, C) User-defined order
    Suggested default: A) Cycle by macOS arrangement (System Preferences order)

50. **Should moving to another display preserve window position?**
    Context: Move "Left Half" on Display 1 → Should it be "Left Half" on Display 2?
    Options: A) Preserve relative position, B) Maintain absolute pixel position, C) Center on new display
    Suggested default: A) Preserve relative position (Left Half → Left Half)

51. **Should we detect display disconnection and restore windows?**
    Context: Unplug external monitor → windows move to main. Replug → restore?
    Options: A) Yes (remember positions), B) No (macOS handles it), C) Phase 3+
    Suggested default: B) No (v1.0) - let macOS handle this complexity

#### Permissions

52. **How should we handle missing Accessibility permissions?**
    Context: Window management requires Accessibility access.
    Options: A) Prompt once on first use, B) Show inline in results, C) Both
    Suggested default: C) Both - initial prompt + show status in command results

---

### 5.2 Quick Links

53. **Where should Quick Links configuration be stored?**
    Context: Raw idea mentions TOML file
    Options: A) TOML file only, B) SQLite + TOML export, C) GUI-only with SQLite
    Suggested default: B) SQLite (for UI editing) + TOML export (for backup/sharing)

54. **Should Quick Links support dynamic URL parameters?**
    Context: `https://github.com/search?q={query}` with placeholder
    Options: A) Yes (with input prompt), B) No (static URLs only), C) Phase 3+
    Suggested default: A) Yes - matches Raycast Quicklinks feature

55. **Should we support importing bookmarks from browsers?**
    Context: Raw idea mentions Safari, Chrome, Firefox
    Options: A) Yes (all three), B) Safari + Chrome only, C) Manual only (v1.0)
    Suggested default: A) Yes (all three) - one-time import feature

56. **Should Quick Links support folders/organization?**
    Context: Group links by category (Work, Personal, Dev)
    Options: A) Yes (folders), B) Yes (tags), C) Flat list only
    Suggested default: B) Yes (tags) - more flexible than rigid folders

---

### 5.3 Calendar Integration

#### Permissions & Setup

57. **How should we handle Calendar permission requests?**
    Context: EventKit requires user authorization
    Options: A) Request on first calendar command, B) Request during onboarding, C) Just-in-time per action
    Suggested default: A) Request on first calendar command (least intrusive)

58. **Should we request read-only or read-write access?**
    Context: Read-only is safer, but creating events needs write access
    Options: A) Read-only (display only), B) Read-write (full features), C) Configurable
    Suggested default: A) Read-only for v1.0 (defer event creation to Phase 3+)

#### Event Display

59. **How many days of upcoming events should be shown by default?**
    Context: "My Schedule" command - how far ahead?
    Options: A) 24 hours, B) 7 days, C) 14 days, D) Configurable
    Suggested default: B) 7 days

60. **Should all-day events be displayed differently?**
    Context: All-day events don't have specific times
    Options: A) Yes (grouped at top), B) Yes (inline with date), C) No special handling
    Suggested default: A) Yes (grouped at top of each day)

61. **Should recurring events show individual instances or series?**
    Context: Daily standup - show each occurrence or collapse?
    Options: A) Individual instances, B) Collapsed with "daily" indicator, C) Configurable
    Suggested default: A) Individual instances (matching macOS Calendar)

62. **How should timezone differences be handled for events?**
    Context: Event in different timezone than local
    Options: A) Convert all to local time, B) Show original + local, C) Show original only
    Suggested default: A) Convert all to local time with timezone indicator if different

#### Conference Calls

63. **Which conference providers should we detect?**
    Context: Raw idea mentions Zoom, Meet, Teams
    Options: A) Zoom + Meet + Teams only, B) Add Webex, BlueJeans, C) Configurable detection patterns
    Suggested default: A) Zoom + Meet + Teams (add more in minor releases)

64. **Where should we look for conference links?**
    Context: Links can be in location, notes, or structured conference data
    Options: A) Location field only, B) Location + Notes, C) All fields + structured data
    Suggested default: C) All fields + structured data (most comprehensive)

65. **Should we show a "Join" button for meetings starting soon?**
    Context: Raycast shows prominent "Join Meeting" when meeting is about to start
    Options: A) Yes (within 5 minutes), B) Yes (within 15 minutes), C) Always show join option
    Suggested default: B) Yes (within 15 minutes of start time)

66. **Should we integrate with Focus/Do Not Disturb for meetings?**
    Context: Auto-enable Focus mode during meetings
    Options: A) Yes (automatic), B) Yes (opt-in), C) No (v1.0)
    Suggested default: C) No (v1.0) - complex feature, defer

---

### 5.4 App Management

#### Uninstaller

67. **Should app uninstall require confirmation?**
    Context: Destructive action
    Options: A) Yes (always), B) Yes (with preview of files to delete), C) No
    Suggested default: B) Yes (with preview) - show what will be removed

68. **Should we protect system apps from uninstall?**
    Context: Safari, Finder, System Preferences should never be uninstallable
    Options: A) Yes (hardcoded protection), B) Yes (warn but allow), C) No protection
    Suggested default: A) Yes (hardcoded protection for /System/Applications)

69. **Should we show the space to be freed before uninstall?**
    Context: Helpful to know disk space recovered
    Options: A) Yes (calculate and display), B) No (just uninstall)
    Suggested default: A) Yes (calculate and display)

70. **How thorough should related file cleanup be?**
    Context: Raw idea lists many locations (Application Support, Preferences, Caches, etc.)
    Options: A) Conservative (only exact bundle ID matches), B) Aggressive (name + bundle ID), C) Configurable
    Suggested default: A) Conservative - avoid deleting unrelated files

71. **Should we use AppCleaner's approach (reverse-engineer app relationships)?**
    Context: AppCleaner does deep scanning for related files
    Options: A) Yes (comprehensive), B) No (standard locations only), C) Optional deep scan
    Suggested default: C) Optional deep scan (default off, user can enable)

72. **Should uninstalled app data be recoverable (Trash) or permanently deleted?**
    Context: Trash allows recovery, permanent delete frees space immediately
    Options: A) Move to Trash, B) Permanent delete, C) User choice per uninstall
    Suggested default: A) Move to Trash (safer, macOS convention)

#### Force Quit

73. **Should Force Quit require confirmation?**
    Context: Force quitting may cause data loss
    Options: A) Yes (always), B) Yes (for non-frozen apps), C) No
    Suggested default: B) Yes (for non-frozen apps, skip confirmation if app is unresponsive)

74. **Should we show a "Not Responding" indicator for hung apps?**
    Context: Help users identify which apps need force quitting
    Options: A) Yes, B) No
    Suggested default: A) Yes

---

### 5.5 Sleep Timer

75. **Should timers persist across PhotonCast restarts?**
    Context: If user sets "sleep in 30 min" and restarts PhotonCast
    Options: A) Yes (persist to disk), B) No (timer lost on restart)
    Suggested default: A) Yes (persist to disk)

76. **Should there be a confirmation before scheduled shutdown/restart?**
    Context: Give user chance to cancel if they forgot about timer
    Options: A) Yes (1 minute warning), B) Yes (5 minute warning), C) No
    Suggested default: A) Yes (1 minute warning with cancel option)

77. **Should we support multiple timers simultaneously?**
    Context: "Sleep in 30 min" AND "lock in 15 min"
    Options: A) Yes (multiple), B) No (one timer at a time)
    Suggested default: B) No (one timer at a time - simpler, less confusing)

78. **Should we show a countdown indicator in the UI?**
    Context: Persistent indicator showing "Sleep in 15:32"
    Options: A) Yes (in menu bar), B) Yes (in launcher when open), C) Both
    Suggested default: C) Both

79. **Should the timer cancel automatically if user is active?**
    Context: "Sleep in 30 min" but user keeps working
    Options: A) Yes (activity resets/cancels timer), B) No (strict timer), C) Configurable
    Suggested default: B) No (strict timer) - user explicitly set it, respect that

---

### 5.6 Preferences & Settings

#### Storage Format

80. **Where should preferences be stored?**
    Context: Need persistent, editable configuration
    Options: A) ~/Library/Preferences (plist), B) ~/.config/photoncast (TOML/JSON), C) SQLite
    Suggested default: B) ~/.config/photoncast/config.toml (user-editable, portable)

81. **Should preferences support import/export?**
    Context: Migrate settings to new machine
    Options: A) Yes (JSON export), B) Yes (TOML file is the export), C) No
    Suggested default: B) Yes (TOML file is directly shareable/importable)

#### Theme System

82. **Should we support all four Catppuccin variants in v1.0?**
    Context: Raw idea lists Latte, Frappé, Macchiato, Mocha
    Options: A) Yes (all four), B) Mocha + Latte only (dark + light), C) Mocha only (current)
    Suggested default: A) Yes (all four Catppuccin variants)

83. **Should theme follow system appearance automatically?**
    Context: Switch light/dark based on macOS appearance
    Options: A) Yes (auto option), B) No (manual selection only)
    Suggested default: A) Yes (auto option that follows system)

84. **What should the 14 accent color options be?**
    Context: Raw idea mentions "14 options" without specifying
    Suggested: Use Catppuccin's named colors:
    - Rosewater, Flamingo, Pink, Mauve, Red, Maroon, Peach
    - Yellow, Green, Teal, Sky, Sapphire, Blue, Lavender
    
    **Is this the intended set of 14?**

#### Keyboard Shortcuts

85. **Should users be able to customize all keyboard shortcuts?**
    Context: Actions like Cmd+K, Cmd+C, etc.
    Options: A) Yes (fully customizable), B) Partial (some locked), C) No (fixed shortcuts)
    Suggested default: A) Yes (fully customizable)

86. **Should we support "hyper key" combinations?**
    Context: Caps Lock remapped to Cmd+Ctrl+Opt+Shift
    Options: A) Yes (detect hyper key), B) No (standard modifiers only)
    Suggested default: A) Yes (detect hyper key combinations)

---

## Sprint 6: Native Extension System

### 6.1 Extension Architecture

#### Manifest & Loading

87. **What format should extension manifests use?**
    Context: Raw idea mentions TOML
    Options: A) TOML (matches Rust ecosystem), B) JSON (broader tooling), C) Both supported
    Suggested default: A) TOML (Rust-native, human-readable)

88. **Where should extensions be installed?**
    Context: User-installed vs system extensions
    Options: 
    - A) ~/.config/photoncast/extensions/
    - B) ~/Library/Application Support/PhotonCast/Extensions/
    - C) Both (user + system paths)
    
    Suggested default: B) ~/Library/Application Support/PhotonCast/Extensions/

89. **Should extensions be sandboxed?**
    Context: Security vs functionality tradeoff
    Options: A) Yes (strict sandbox), B) Yes (permissioned sandbox), C) No (full access)
    Suggested default: B) Yes (permissioned sandbox) - extension declares needed permissions

90. **What permissions should extensions be able to request?**
    Context: Define the permission model
    Suggested permissions:
    - `clipboard_read` - Read clipboard
    - `clipboard_write` - Write to clipboard
    - `network` - Make HTTP requests
    - `filesystem_read` - Read files (scoped to user directory)
    - `filesystem_write` - Write files (scoped to extension directory)
    - `notifications` - Show system notifications
    - `storage` - Per-extension persistent storage
    
    **Are these permissions appropriate? Should any be added/removed?**

#### API Design

91. **Should the extension API be synchronous or async?**
    Context: Rust is async-native, but sync may be easier for extension authors
    Options: A) Async-first, B) Sync with async utilities, C) Both available
    Suggested default: A) Async-first (modern Rust patterns)

92. **What UI components should be available to extensions?**
    Context: Mirror Raycast's component model
    Suggested: List, Detail, Form, Grid, Action, ActionPanel
    **Should we add any additional components?**

93. **Should extensions have access to PhotonCast's search infrastructure?**
    Context: Extensions could be search providers
    Options: A) Yes (extensions can provide search results), B) No (UI only)
    Suggested default: A) Yes (extensions can provide search results)

94. **How should extension errors be handled?**
    Context: Crashing extension shouldn't crash PhotonCast
    Options: A) Isolated process per extension, B) Error boundaries with recovery, C) Both
    Suggested default: C) Both - isolated if possible, error boundaries as fallback

#### Hot Reload & Development

95. **Should extensions support hot reload during development?**
    Context: Raw idea mentions hot-reload support
    Options: A) Yes (file watcher), B) Yes (manual refresh), C) No
    Suggested default: A) Yes (file watcher for .rs files in dev mode)

96. **Should we provide an extension development CLI?**
    Context: `photoncast extension new`, `photoncast extension dev`, etc.
    Options: A) Yes (full CLI), B) No (use cargo directly), C) Minimal scaffolding only
    Suggested default: A) Yes (full CLI with scaffolding, validation, packaging)

---

### 6.2 Custom Commands

97. **What shell should custom commands execute in?**
    Context: User's default shell vs specific shell
    Options: A) User's default ($SHELL), B) /bin/zsh (macOS default), C) Configurable per command
    Suggested default: C) Configurable per command, defaulting to user's $SHELL

98. **Should command output be streamed or buffered?**
    Context: Long-running commands should show progress
    Options: A) Streamed (real-time), B) Buffered (show after completion), C) Configurable
    Suggested default: A) Streamed (real-time output)

99. **What should the default command timeout be?**
    Context: Prevent hung commands from blocking
    Options: A) 30 seconds, B) 60 seconds, C) No timeout, D) Configurable per command
    Suggested default: D) Configurable per command with 60 second default

100. **Should commands support environment variable injection?**
     Context: Pass secrets or config to commands
     Options: A) Yes (define in command config), B) Yes (inherit from system), C) Both
     Suggested default: C) Both - inherit system + allow additional per-command

101. **Should we support interactive commands (stdin)?**
     Context: Commands that prompt for input
     Options: A) Yes, B) No (output only), C) Phase 3+
     Suggested default: B) No (v1.0) - complex UX, defer

102. **What notification should be shown when command completes?**
     Context: Success/failure feedback
     Options: A) Toast notification, B) HUD (brief overlay), C) Both based on result, D) Configurable
     Suggested default: C) HUD for success, Toast for failure

---

### 6.3 First-Party Extensions

103. **Which first-party extensions should ship with v1.0?**
     Context: Raw idea mentions GitHub, System Preferences, Color Picker
     Options:
     - GitHub Repositories Browser
     - System Preferences Shortcuts
     - Color Picker (eyedropper + format conversion)
     
     **Should all three be included? Any others?**
     Suggested: All three + potentially:
     - SSH Connections (quick connect to saved hosts)
     - Port Scanner (see what's running locally)

104. **Should first-party extensions be bundled or downloaded?**
     Context: Bundled = larger app size, downloaded = requires internet on first use
     Options: A) Bundled, B) Downloaded on first use, C) Mix (core bundled, others downloaded)
     Suggested default: A) Bundled (better first-run experience)

---

## Cross-Cutting Concerns

### Testing & Quality

105. **What test coverage target should we aim for in Phase 2?**
     Context: Raw idea mentions 80% for v1.0 (in roadmap)
     Options: A) 60%, B) 70%, C) 80%
     Suggested default: C) 80% (matches roadmap)

106. **Should we add integration tests for each feature?**
     Context: Beyond unit tests
     Options: A) Yes (all features), B) Yes (critical paths only), C) Unit tests only
     Suggested default: A) Yes (all features have integration tests)

107. **Should we add UI tests with GPUI's test framework?**
     Context: Test component rendering and interactions
     Options: A) Yes, B) No (manual testing), C) Critical components only
     Suggested default: C) Critical components only (search, results, actions)

### Performance

108. **What are the performance targets for Phase 2 features?**
     Context: Define acceptable latencies
     Suggested targets:
     - Calculator evaluation: < 5ms
     - Clipboard history load: < 100ms
     - Window resize: < 50ms (animation separate)
     - Calendar events load: < 500ms
     - Extension load: < 50ms
     
     **Are these targets appropriate?**

109. **Should we add benchmarks for new features?**
     Context: Prevent performance regressions
     Options: A) Yes (all features), B) Yes (performance-critical only), C) No
     Suggested default: B) Yes (performance-critical only)

### Documentation

110. **What documentation is needed for v1.0?**
     Context: User docs, developer docs, API reference
     Options:
     - A) README + inline help only
     - B) Full user guide + API reference
     - C) User guide + API reference + tutorials
     
     Suggested default: B) Full user guide + API reference

111. **Should we create extension development tutorials?**
     Context: Help community build extensions
     Options: A) Yes (v1.0), B) No (Phase 3+)
     Suggested default: A) Yes - crucial for ecosystem growth

112. **Should extensions require API stability guarantees?**
     Context: Breaking API changes would break extensions
     Options: A) Yes (semver, no breaks in minor), B) No (early days, expect changes), C) Warn about unstable APIs
     Suggested default: C) Warn about unstable APIs (mark as `#[unstable]`)

---

## Visual Design & References

113. **Do you have any mockups or wireframes for Phase 2 features?**
     Context: Visual designs help clarify requirements
     Please share any:
     - Figma/Sketch files
     - Screenshots from Raycast/Alfred you want to emulate
     - Hand-drawn sketches
     - Specific design preferences

114. **Should we match Raycast's visual patterns or differentiate?**
     Context: Similar UX = easier transition for users, different = unique identity
     Options: A) Match closely, B) Similar but distinct, C) Unique PhotonCast identity
     Suggested default: B) Similar but distinct (familiar patterns, Catppuccin aesthetic)

115. **Are there specific Raycast features/behaviors you want to exactly replicate?**
     Context: Knowing priorities helps implementation
     Please list any must-match behaviors:
     - [ ] Clipboard history keyboard shortcuts
     - [ ] Calculator natural language parsing
     - [ ] Window management layouts
     - [ ] Calendar event display
     - [ ] Other: _________

116. **Are there Raycast behaviors you explicitly want to avoid?**
     Context: Some Raycast patterns may not fit PhotonCast's vision
     Please list any avoid-patterns:
     - [ ] _______________

---

## Prioritization

117. **If timeline is constrained, which features are must-have vs nice-to-have?**
     Context: Help prioritize development effort
     
     Please rank (1 = must-have, 2 = important, 3 = nice-to-have):
     
     **Sprint 4:**
     - [ ] Clipboard History: ___
     - [ ] Calculator (basic math): ___
     - [ ] Calculator (currency): ___
     - [ ] Calculator (units): ___
     - [ ] Calculator (dates/timezones): ___
     
     **Sprint 5:**
     - [ ] Window Management: ___
     - [ ] Quick Links: ___
     - [ ] Calendar Integration: ___
     - [ ] App Uninstaller: ___
     - [ ] Sleep Timer: ___
     - [ ] Preferences UI: ___
     
     **Sprint 6:**
     - [ ] Native Extension System: ___
     - [ ] Custom Commands: ___
     - [ ] First-Party Extensions: ___

118. **Are there any features in the raw idea that should be deferred to Phase 3+?**
     Context: Scope management
     Please list any to defer: _____________

---

## Final Notes

119. **Is there anything not covered in these questions that's important for Phase 2?**
     Please add any additional requirements or considerations.

120. **What is your availability for follow-up questions during implementation?**
     Context: Quick clarifications help maintain velocity
     Options: A) Daily async, B) Weekly sync, C) As-needed async

---

*Total questions: 120*
*Organized by: Feature area → Specific topic → Implementation detail*
