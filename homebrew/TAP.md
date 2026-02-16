# PhotonCast Homebrew Tap

This document explains how to set up and use a custom Homebrew Tap for PhotonCast, which is an alternative to submitting to the official Homebrew/homebrew-cask repository.

## Why a Custom Tap?

Homebrew Cask has specific requirements that PhotonCast may not yet meet:

- **GitHub Stars**: Homebrew typically requires 50+ stars for new casks
- **Notarization**: App must be signed and notarized for Gatekeeper
- **Stable Releases**: Prefer stable versions over pre-releases

While we work toward meeting these requirements, users can install PhotonCast via our custom tap.

## Latest Build Downloads

- **Latest DMG**: [PhotonCast.dmg](https://github.com/panosAthDBX/photoncast/releases/latest/download/PhotonCast.dmg)
- **Latest checksum**: [PhotonCast.dmg.sha256](https://github.com/panosAthDBX/photoncast/releases/latest/download/PhotonCast.dmg.sha256)
- **Latest release notes**: [GitHub Releases](https://github.com/panosAthDBX/photoncast/releases/latest)

## Quick Start for Users

### Add the Tap and Install

```bash
# Add the PhotonCast tap
brew tap photoncast/tap

# Install PhotonCast
brew install --cask photoncast

# Or install directly without tapping
brew install --cask photoncast/tap/photoncast
```

### Update

```bash
# Update Homebrew
brew update

# Upgrade PhotonCast
brew upgrade --cask photoncast
```

### Uninstall

```bash
brew uninstall --cask photoncast

# Optional: Remove the tap
brew untap photoncast/tap
```

## Setting Up the Tap Repository

### For Maintainers

To create the official `photoncast/homebrew-tap` repository:

1. **Create the Repository**
   ```bash
   # Create on GitHub as photoncast/homebrew-tap
   # Must be named homebrew-tap for auto-tap naming
   ```

2. **Initialize with Formula**
   ```bash
   # Clone the tap repository
   git clone https://github.com/photoncast/homebrew-tap.git
   cd homebrew-tap

   # Create Casks directory
   mkdir -p Casks

   # Copy the formula
   cp ../photoncast/homebrew/photoncast.rb Casks/photoncast.rb

   # Commit and push
   git add Casks/
   git commit -m "feat: add PhotonCast cask v0.1.0-alpha"
   git push origin main
   ```

### Repository Structure

```
homebrew-tap/
├── Casks/
│   └── photoncast.rb       # Main cask formula
├── README.md               # Tap documentation
└── .github/
    └── workflows/
        └── ci.yml          # CI to validate formula
```

### Automated Updates

Use the update script to push new versions:

```bash
# Build the release DMG first
./scripts/build-release.sh

# Update the formula
./homebrew/scripts/update-formula.sh 0.2.0 ./dist/PhotonCast-0.2.0.dmg

# Copy to tap repo and push
cp homebrew/photoncast.rb ../homebrew-tap/Casks/photoncast.rb
cd ../homebrew-tap
git add Casks/
git commit -m "chore: update PhotonCast to v0.2.0"
git push origin main
```

## CI/CD for Tap

Create `.github/workflows/ci.yml` in the tap repository:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  audit:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Homebrew
        uses: Homebrew/actions/setup-homebrew@master

      - name: Audit formula
        run: brew audit --cask Casks/photoncast.rb

      - name: Check style
        run: brew style --fix Casks/photoncast.rb

  install-test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Homebrew
        uses: Homebrew/actions/setup-homebrew@master

      - name: Install from local formula
        run: brew install --cask ./Casks/photoncast.rb

      - name: Verify installation
        run: |
          ls -la /Applications/PhotonCast.app
          /Applications/PhotonCast.app/Contents/MacOS/photoncast --version
```

## Migration to Official Cask

Once PhotonCast meets Homebrew requirements:

1. **Submit to homebrew-cask** (see SUBMISSION.md)
2. **Update tap README** with migration notice:
   ```markdown
   ## Migration Notice

   PhotonCast is now available in the official Homebrew Cask repository!

   ### For existing users:
   ```bash
   # Uninstall from tap
   brew uninstall --cask photoncast
   brew untap photoncast/tap

   # Install from official cask
   brew install --cask photoncast
   ```
   ```
3. **Archive the tap** or keep it for beta/development versions

## Troubleshooting

### "Formula is unreadable" Error

```bash
# Reset Homebrew's cache
brew cleanup -s photoncast
rm -rf ~/Library/Caches/Homebrew/Cask/photoncast
```

### Tap Not Found

```bash
# Verify tap URL
brew tap photoncast/tap https://github.com/photoncast/homebrew-tap

# Or install directly
brew install --cask https://raw.githubusercontent.com/photoncast/homebrew-tap/main/Casks/photoncast.rb
```

### Permission Issues

```bash
# Fix ownership
sudo chown -R $(whoami) /usr/local/Caskroom/photoncast
sudo chown -R $(whoami) /Applications/PhotonCast.app
```

## References

- [Homebrew Tap Documentation](https://docs.brew.sh/Taps)
- [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook)
- [Homebrew Cask Token Reference](https://docs.brew.sh/Cask-Cookbook#token-conventions)
