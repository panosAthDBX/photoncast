# PhotonCast

A lightning-fast macOS launcher built in pure Rust using [GPUI](https://github.com/zed-industries/zed).

## Features

- **Fuzzy search** across applications, commands, files, and extensions
- **Frecency-based ranking** (frequency + recency) for personalized results
- **Catppuccin theming** with auto light/dark mode sync
- **Smooth animations** with reduce-motion support
- **Global hotkey** activation (CGEventTap)
- **Clipboard history** with AES-256-GCM encryption and full-text search
- **Calculator & unit converter** with currency conversion and datetime math
- **Window management** — halves, quarters, thirds, centering, maximize, restore, multi-display support, layout cycling, and visual overlay feedback
- **Calendar integration** — next meeting display, conference link detection (EventKit)
- **Native extension system** — ABI-stable dynamic library extensions with code signing
- **Quick links** — bookmarks with placeholder-based URL expansion
- **File search** — Spotlight-powered live file index with browsing
- **App management** — auto-quit, force quit, uninstaller with leftover detection
- **Sleep timer** — configurable timer with system actions (sleep, shutdown, etc.)

## Download Latest Build

- **Latest DMG**: [PhotonCast.dmg](https://github.com/panosAthDBX/photoncast/releases/latest/download/PhotonCast.dmg)
- **Latest checksum**: [PhotonCast.dmg.sha256](https://github.com/panosAthDBX/photoncast/releases/latest/download/PhotonCast.dmg.sha256)
- **Latest release notes**: [GitHub Releases](https://github.com/panosAthDBX/photoncast/releases/latest)
- **Latest main-branch CI artifacts**: [CI workflow runs](https://github.com/panosAthDBX/photoncast/actions/workflows/ci.yml?query=branch%3Amain) (download `photoncast-dmg` from the latest successful push run)

## Requirements

- macOS 12.0+
- Rust stable toolchain (MSRV: 1.75)

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run
cargo run
```

## Local Install

```bash
# Create a stable local code-signing identity once
./scripts/create-dev-signing-identity.sh

# Build a signed-or-ad-hoc app bundle in build/PhotonCast.app
./scripts/release-build.sh

# Replace the installed app in /Applications and relaunch it
./scripts/install-app.sh
```

If PhotonCast is only ad-hoc signed, macOS tracks it by `cdhash`, so Accessibility
and Calendar permissions may need to be re-granted after rebuilds. A stable signing
identity avoids that churn. `create-dev-signing-identity.sh` provisions a local
`PhotonCast Local Dev` identity, and `release-build.sh` / `sign.sh` automatically
use it when `~/.config/photoncast/dev-signing.env` is present.

## Project Structure

```
photoncast/
├── crates/
│   ├── photoncast/                        # Main binary (launcher UI, event loop)
│   ├── photoncast-core/                   # Core library (search, indexing, extensions, platform)
│   ├── photoncast-apps/                   # App management (uninstaller, force quit, auto quit, sleep)
│   ├── photoncast-calculator/             # Calculator (math, currency, units, datetime)
│   ├── photoncast-calendar/               # Calendar integration (EventKit, conference links)
│   ├── photoncast-clipboard/              # Clipboard history (encrypted storage, monitoring)
│   ├── photoncast-ext-github/             # GitHub search extension
│   ├── photoncast-ext-screenshots/        # Screenshot browser extension
│   ├── photoncast-ext-system-preferences/ # System preferences extension
│   ├── photoncast-extension-api/          # ABI-stable extension API
│   ├── photoncast-quicklinks/             # Quick links management
│   ├── photoncast-theme/                  # Catppuccin theming
│   ├── photoncast-timer/                  # Sleep timer
│   └── photoncast-window/                 # Window management (layouts, multi-display, Accessibility API)
├── tests/                                 # Integration tests
└── droidz/                                # Product specs and standards
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full architecture overview including crate dependency graph, data flow, and key design decisions.

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench
```

## License

MIT
