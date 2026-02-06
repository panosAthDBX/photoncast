# Submitting PhotonCast to Homebrew Cask

This document outlines the process for submitting PhotonCast to the official Homebrew/homebrew-cask repository.

## Prerequisites

Before submitting, ensure PhotonCast meets Homebrew Cask requirements:

### Required

- [x] **Signed macOS Application**: App is signed with Developer ID
- [ ] **Notarized**: App passes Apple notarization (for Gatekeeper)
- [ ] **Stable Version**: Release is not marked as pre-release on GitHub
- [ ] **Public Repository**: Source code is publicly available
- [ ] **Downloadable DMG**: Release includes downloadable DMG

### Recommended

- [ ] **50+ GitHub Stars**: Homebrew typically requires 50+ stars for new casks
- [ ] **Established Project**: Some history/releases beyond initial version
- [ ] **Active Maintenance**: Regular updates and responsive maintainers

## Current Status

| Requirement | Status | Notes |
|-------------|--------|-------|
| Signed App | ✅ Ready | Code signing in place |
| Notarized | ⚠️ In Progress | Working on notarization workflow |
| Stable Release | ⚠️ Pending | Currently v0.1.0-alpha |
| 50+ Stars | ❌ Pending | Need to build community |
| Public Repo | ✅ Ready | Open source on GitHub |

## Submission Process

### Step 1: Fork homebrew-cask

```bash
# Fork via GitHub UI: https://github.com/Homebrew/homebrew-cask/fork

# Clone your fork
git clone https://github.com/YOUR_USERNAME/homebrew-cask.git
cd homebrew-cask

# Add upstream remote
git remote add upstream https://github.com/Homebrew/homebrew-cask.git

# Create feature branch
git checkout -b photoncast-cask
```

### Step 2: Prepare the Formula

```bash
# Create the cask file in the correct location
# Casks are organized by first letter
mkdir -p Casks/p

# Copy and adapt the formula
cp /path/to/photoncast/homebrew/photoncast.rb Casks/p/photoncast.rb

# Edit to ensure it follows Homebrew conventions
# - Verify version format
# - Ensure SHA256 is correct (not :no_check)
# - Check all URLs are valid
```

### Step 3: Validate the Formula

```bash
# Install Homebrew development tools
brew install brew-gem

# Run audit (must pass before submission)
brew audit --cask Casks/p/photoncast.rb

# Check style (auto-fix with --fix)
brew style --fix Casks/p/photoncast.rb

# Test installation locally
brew install --cask Casks/p/photoncast.rb

# Verify app launches
open /Applications/PhotonCast.app

# Uninstall test
brew uninstall --cask photoncast
```

### Step 4: Submit Pull Request

```bash
# Stage changes
git add Casks/p/photoncast.rb

# Commit following Homebrew conventions
git commit -m "Add PhotonCast 0.1.0"

# Push to your fork
git push origin photoncast-cask
```

Create PR via GitHub: https://github.com/Homebrew/homebrew-cask/compare

#### PR Template

```markdown
**Important:** *Do not tick a checkbox if you haven't performed its action.* Honesty is crucial for an efficient review process.

After making all changes to a cask, verify:

- [x] The submission is for [a stable version](https://docs.brew.sh/Acceptable-Casks#stable-versions) or [documented exception](https://docs.brew.sh/Acceptable-Casks#but-there-is-no-stable-version).
- [x] `brew audit --cask photoncast.rb` is error-free.
- [x] `brew style --fix photoncast.rb` reports no offenses.

Additionally, **if adding a new cask**:

- [x] Named the cask according to the [token reference](https://docs.brew.sh/Cask-Cookbook#token-conventions).
- [x] Checked the cask was not already refused in [closed PRs](https://github.com/Homebrew/homebrew-cask/pulls?q=is%3Apr+is%3Aclosed).
- [x] Checked there are no [open PRs](https://github.com/Homebrew/homebrew-cask/pulls) for the same cask.
- [x] Checked the cask is submitted to [the correct repo](https://docs.brew.sh/Acceptable-Casks#finding-a-home-for-your-cask).

PhotonCast is a fast, extensible macOS launcher built with Rust and GPUI.
- GitHub: https://github.com/photoncast/photoncast
- Homepage: https://github.com/photoncast/photoncast

Notes:
- Requires macOS 12.0+ (Monterey)
- Includes auto-update capability via Sparkle
- Signed and notarized for Gatekeeper
```

### Step 5: Address Review Feedback

Homebrew maintainers may request changes:

1. **Common Requests**:
   - Adjust description formatting
   - Fix URL patterns
   - Update zap stanza
   - Add `depends_on` constraints

2. **Respond Promptly**:
   ```bash
   # Make requested changes
   vim Casks/p/photoncast.rb

   # Re-verify
   brew audit --cask Casks/p/photoncast.rb
   brew style --fix Casks/p/photoncast.rb

   # Amend commit
   git add Casks/p/photoncast.rb
   git commit --amend -m "Add PhotonCast 0.1.0"
   git push --force-with-lease origin photoncast-cask
   ```

### Step 6: Post-Merge

Once merged:

1. **Update documentation**:
   - Add `brew install --cask photoncast` to README
   - Update installation instructions on website

2. **Archive custom tap** (if used):
   - Update tap README with migration notice
   - Consider deprecating after transition period

3. **Monitor for issues**:
   - Watch Homebrew/homebrew-cask issues for reports
   - Respond to installation problems

## Alternative: Custom Tap

If the app doesn't meet official cask requirements (stars, stability):

1. **Create `photoncast/homebrew-tap`** repository
2. **Users install via**: `brew tap photoncast/tap && brew install --cask photoncast`
3. **Document in TAP.md** (see TAP.md for details)
4. **Migrate later** when requirements are met

## Maintenance

### Updating the Cask After Merge

```bash
# Fork and clone (one-time setup)
git clone https://github.com/YOUR_USERNAME/homebrew-cask.git
cd homebrew-cask
git remote add upstream https://github.com/Homebrew/homebrew-cask.git

# Sync with upstream
git checkout main
git pull upstream main
git push origin main

# Create update branch
git checkout -b photoncast-0.2.0

# Update the cask
brew bump-cask-pr --version 0.2.0 photoncast
# Or manually edit Casks/p/photoncast.rb

# Submit PR (same process as initial submission)
```

### Automated Updates with bump-cask-pr

For maintainers with Homebrew write access:

```bash
# Automatically create PR with version bump
brew bump-cask-pr photoncast --version 0.2.0
```

This:
1. Calculates new SHA256
2. Updates version
3. Creates branch
4. Commits changes
5. Pushes to fork
6. Opens PR

## Troubleshooting Submission Issues

### Audit Failures

```bash
# Run verbose audit
brew audit --cask --strict --online Casks/p/photoncast.rb

# Common fixes:
# - URL not reachable: Check GitHub release URL
# - SHA256 mismatch: Re-run update script
# - Homepage differs: Ensure homepage matches cask homepage
```

### Style Failures

```bash
# Auto-fix most style issues
brew style --fix Casks/p/photoncast.rb

# For manual fixes, follow Ruby style guide
# https://rubystyle.guide/
```

### Test Failures

```bash
# Verbose installation test
brew install --cask --verbose --debug Casks/p/photoncast.rb

# Check app behavior
ls -la /Applications/PhotonCast.app
/Applications/PhotonCast.app/Contents/MacOS/photoncast --version
```

## References

- [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook)
- [Acceptable Casks](https://docs.brew.sh/Acceptable-Casks)
- [Adding Software to Homebrew](https://docs.brew.sh/Adding-Software-to-Homebrew)
- [Homebrew Cask Token Reference](https://docs.brew.sh/Cask-Cookbook#token-conventions)
- [Common Issues](https://docs.brew.sh/Common-Issues)
