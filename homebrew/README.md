# PhotonCast Homebrew Cask

Homebrew Cask formula for [PhotonCast](https://github.com/photoncast/photoncast) - a fast, extensible macOS launcher built with Rust and GPUI.

## Quick Install

### Option 1: Official Homebrew Cask (Recommended - once available)

```bash
brew install --cask photoncast
```

### Option 2: Custom Tap (Current)

```bash
brew tap photoncast/tap
brew install --cask photoncast
```

### Option 3: Direct from Repository

```bash
brew install --cask https://raw.githubusercontent.com/photoncast/photoncast/main/homebrew/photoncast.rb
```

## Files

| File | Purpose |
|------|---------|
| `photoncast.rb` | Homebrew Cask formula |
| `scripts/calculate-sha256.sh` | Calculate SHA256 for DMG releases |
| `scripts/update-formula.sh` | Update formula for new releases |
| `TAP.md` | Custom tap setup instructions |
| `SUBMISSION.md` | Guide for submitting to Homebrew |

## Updating the Formula

When releasing a new version:

```bash
# 1. Build the release DMG
./scripts/build-release.sh

# 2. Update the formula
./homebrew/scripts/update-formula.sh 0.2.0 ./dist/PhotonCast-0.2.0.dmg

# 3. Test locally
brew install --cask ./homebrew/photoncast.rb

# 4. Commit changes
git add homebrew/
git commit -m "chore(homebrew): update formula to v0.2.0"
```

## Development

### Testing the Formula

```bash
# Audit the formula
brew audit --cask ./homebrew/photoncast.rb

# Check style
brew style --fix ./homebrew/photoncast.rb

# Install locally
brew install --cask ./homebrew/photoncast.rb

# Uninstall
brew uninstall --cask photoncast
```

### Calculating SHA256

```bash
# For a specific DMG
./homebrew/scripts/calculate-sha256.sh ./dist/PhotonCast-0.1.0-alpha.dmg

# Or let it search automatically
./homebrew/scripts/calculate-sha256.sh
```

## Requirements

- macOS 12.0 (Monterey) or later
- 64-bit Intel or Apple Silicon Mac

## Cask Details

- **Bundle ID**: `com.photoncast.app`
- **App Name**: PhotonCast.app
- **Install Location**: `/Applications/PhotonCast.app`
- **Config Location**: `~/Library/Application Support/PhotonCast`

## Uninstall

```bash
brew uninstall --cask photoncast
```

This will:
- Remove the application
- Clean up user preferences (via `zap` stanza)

## Contributing

See [SUBMISSION.md](./SUBMISSION.md) for details on submitting to the official Homebrew Cask repository.

## License

The formula follows the same license as PhotonCast (MIT).
