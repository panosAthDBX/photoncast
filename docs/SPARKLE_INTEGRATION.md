# Sparkle Auto-Update Integration

This document describes the Sparkle auto-update system for PhotonCast, including appcast feed generation, signing, and integration with the application.

## Overview

[Sparkle](https://sparkle-project.org/) is an open-source software update framework for macOS that enables automatic updates. PhotonCast uses Sparkle to:

- Check for updates automatically or on demand
- Download and install updates securely
- Verify update authenticity via EdDSA signatures
- Present release notes to users

## Current shipped status (as of 2026-04-02)

PhotonCast currently has **core update-checking library support**, but **app-level wiring is still partial**:

- **Implemented in `photoncast-core`**:
  - `UpdateManager` construction/configuration
  - async initialization
  - manual `check_for_updates()` against the appcast feed
  - `auto_check_if_needed()` helper logic for launch-time checks
  - update feed parsing and configuration tests
- **Not yet wired in the app shell**:
  - no confirmed startup call path that creates an `UpdateManager` and runs `auto_check_if_needed()`
  - the menu-bar **Check for Updates** action currently logs a note instead of invoking the update manager (`crates/photoncast/src/main.rs:307-312`)
  - update installation remains `NotImplemented` in the current core implementation

So the shipped behavior today is best described as: **update subsystem implemented in core, but no verified end-to-end startup/manual UI integration yet**.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  PhotonCast App │────▶│  Appcast Feed   │◀────│  GitHub/Server  │
│                 │     │  (XML/RSS)      │     │  (DMG + XML)    │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │
         │ Sparkle Framework
         ▼
┌─────────────────┐
│  Update Check   │
│  Download       │
│  Verify + Install│
└─────────────────┘
```

## Appcast Feed

The appcast is an RSS 2.0 feed with Sparkle namespace extensions that contains update information.

### Feed URL

- **Production**: `https://api.photoncast.app/updates/appcast.xml`
- **GitHub Alternative**: `https://github.com/photoncast/photoncast/releases.atom`

### Latest Build Download Endpoints

- **Latest DMG**: `https://github.com/panosAthDBX/photoncast/releases/latest/download/PhotonCast.dmg`
- **Latest checksum**: `https://github.com/panosAthDBX/photoncast/releases/latest/download/PhotonCast.dmg.sha256`
- **Latest release page**: `https://github.com/panosAthDBX/photoncast/releases/latest`

### Appcast Format

```xml
<?xml version="1.0" encoding="utf-8"?>
<rss xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle" version="2.0">
    <channel>
        <title>PhotonCast Changelog</title>
        <item>
            <title>Version 1.1.0</title>
            <pubDate>Mon, 29 Jan 2026 12:00:00 +0000</pubDate>
            <sparkle:version>110</sparkle:version>
            <sparkle:shortVersionString>1.1.0</sparkle:shortVersionString>
            <description><![CDATA[...]]></description>
            <enclosure
                url="https://api.photoncast.app/releases/1.1.0/PhotonCast.dmg"
                sparkle:edSignature="base64signature"
                length="15240000"
                type="application/octet-stream"/>
        </item>
    </channel>
</rss>
```

### Key Elements

| Element | Description |
|---------|-------------|
| `sparkle:version` | Build number for comparison (must increase) |
| `sparkle:shortVersionString` | Human-readable version string |
| `sparkle:edSignature` | EdDSA signature for security verification |
| `sparkle:minimumSystemVersion` | Minimum macOS version required |
| `enclosure` | DMG download with URL, size, and signature |

## Security: Code Signing

### EdDSA Signature Generation

Updates are signed using Ed25519 to prevent tampering:

1. **Private Key**: Signs the DMG file (kept secret, used in CI/CD)
2. **Public Key**: Embedded in the app for verification

### Generating Signing Keys

```bash
# Generate a new Ed25519 key pair
./scripts/generate-signing-key.sh

# Output:
#   certs/sparkle_signing.key      (private - keep secret)
#   certs/sparkle_signing.pub      (public - text format)
#   certs/sparkle_signing_pub.der  (public - DER format for Sparkle)
```

### Embedding the Public Key

The DER-format public key must be embedded in your app bundle:

1. Add to Xcode project resources: `sparkle_signing_pub.der`
2. Or configure in code: `SPUUpdater.setPublicKeyData()`

## Generating the Appcast

### Manual Generation

```bash
# Generate appcast for a release
./scripts/generate-appcast.sh \
    1.1.0 \
    110 \
    "https://api.photoncast.app/releases/1.1.0/PhotonCast.dmg" \
    ./dist/PhotonCast.dmg

# With release notes
RELEASE_NOTES="Major feature release with new extensions" \
    ./scripts/generate-appcast.sh 1.1.0 110 ...
```

### Arguments

| Argument | Description | Example |
|----------|-------------|---------|
| `version` | Semantic version | `1.1.0` |
| `build_number` | Numeric build for comparison | `110` |
| `dmg_url` | Public download URL | `https://.../PhotonCast.dmg` |
| `dmg_path` | Local path for signing | `./dist/PhotonCast.dmg` |

### GitHub Actions Integration

```yaml
# .github/workflows/release.yml (excerpt)
- name: Generate Appcast
  run: |
    ./scripts/generate-appcast.sh \
      "${{ github.ref_name }}" \
      "${{ github.run_number }}" \
      "https://github.com/photoncast/photoncast/releases/download/${{ github.ref_name }}/PhotonCast.dmg" \
      ./dist/PhotonCast.dmg
  env:
    SPARKLE_SIGNING_KEY: certs/sparkle_signing.key

- name: Upload Appcast
  uses: actions/upload-release-asset@v1
  with:
    upload_url: ${{ steps.create_release.outputs.upload_url }}
    asset_path: ./dist/appcast-${{ github.ref_name }}.xml
    asset_name: appcast.xml
```

## Hosting Options

### Option 1: GitHub Releases (Recommended for Open Source)

Use GitHub Releases as your appcast host:

1. Upload `appcast.xml` with each release
2. Use GitHub's Releases feed: `https://github.com/photoncast/photoncast/releases.atom`
3. Configure Sparkle feed URL to point to your hosted XML

**Pros**: Free, versioned, integrated with release workflow  
**Cons**: Less control over feed structure

### Option 2: Self-Hosted API

Host appcast on your own infrastructure:

```
https://api.photoncast.app/updates/appcast.xml
```

**Pros**: Full control, can implement channels (stable/beta)  
**Cons**: Requires infrastructure, CDN for global performance

### Option 3: Static Hosting (S3, Cloudflare R2, etc.)

Upload appcast to static storage:

```bash
# After generation
aws s3 cp ./dist/appcast-1.1.0.xml s3://photoncast-updates/appcast.xml
aws s3 cp ./dist/PhotonCast.dmg s3://photoncast-updates/releases/1.1.0/
```

**Pros**: Fast, reliable, low cost  
**Cons**: Requires separate upload step

## Rust Integration

### Using sparkle-rs

Add to `Cargo.toml`:

```toml
[dependencies]
sparkle-rs = "0.2"
```

Basic implementation:

```rust
use sparkle_rs::{Updater, UpdateError};

pub struct UpdateManager {
    updater: Updater,
}

impl UpdateManager {
    pub fn new(feed_url: &str) -> Result<Self, UpdateError> {
        let updater = Updater::new(feed_url)?;
        Ok(Self { updater })
    }

    pub fn check_for_updates(&self) -> Result<(), UpdateError> {
        self.updater.check_for_updates()
    }

    pub fn set_automatic_checks(&self, enabled: bool) {
        self.updater.set_automatically_checks_for_updates(enabled);
    }
}
```

### Using FFI with Sparkle.framework

For direct Sparkle integration:

```rust
use std::ffi::CString;
use std::os::raw::c_char;

#[link(name = "Sparkle", kind = "framework")]
extern "C" {
    fn SPUUpdater_new(feed_url: *const c_char) -> *mut c_void;
    fn SPUUpdater_check_for_updates(updater: *mut c_void);
}

pub struct SparkleUpdater {
    ptr: *mut c_void,
}

impl SparkleUpdater {
    pub fn new(feed_url: &str) -> Self {
        let url = CString::new(feed_url).unwrap();
        let ptr = unsafe { SPUUpdater_new(url.as_ptr()) };
        Self { ptr }
    }

    pub fn check_for_updates(&self) {
        unsafe { SPUUpdater_check_for_updates(self.ptr) };
    }
}
```

## Menu Integration

Add "Check for Updates" to the menu bar:

```rust
// In menu_bar.rs
impl MenuBarManager {
    fn build_menu(&self) -> Menu {
        Menu::new()
            .entry(MenuItem::action("Open PhotonCast", OpenAction))
            .separator()
            .entry(MenuItem::action("Check for Updates...", CheckForUpdatesAction))
            .separator()
            .entry(MenuItem::action("Preferences...", OpenPreferencesAction))
            .entry(MenuItem::action("About PhotonCast", AboutAction))
            .separator()
            .entry(MenuItem::action("Quit", QuitAction))
    }
}

// Action handler
impl Action for CheckForUpdatesAction {
    fn perform(&self, cx: &mut App) {
        if let Some(updater) = cx.global::<UpdateManager>() {
            updater.check_for_updates().ok();
        }
    }
}
```

## Testing Updates

### Local Testing

1. Build two versions of the app (e.g., 1.0.0 and 1.0.1)
2. Generate appcast pointing to 1.0.1
3. Host locally:
   ```bash
   python3 -m http.server 8000 --directory ./dist
   ```
4. Point test app to `http://localhost:8000/appcast.xml`
5. Launch 1.0.0 and check for updates

### Test Channels

Support beta/nightly channels:

```xml
<!-- Stable channel: appcast.xml -->
<item>
    <sparkle:version>100</sparkle:version>
    <enclosure url=".../PhotonCast-stable.dmg" ... />
</item>

<!-- Beta channel: appcast-beta.xml -->
<item>
    <sparkle:version>101</sparkle:version>
    <enclosure url=".../PhotonCast-beta.dmg" ... />
</item>
```

Configure based on user preference:

```rust
let feed_url = if config.beta_channel {
    "https://api.photoncast.app/updates/appcast-beta.xml"
} else {
    "https://api.photoncast.app/updates/appcast.xml"
};
```

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| "Update check failed" | Feed URL unreachable | Verify URL and network |
| "Signature invalid" | Wrong public key or corrupted DMG | Regenerate keys, re-sign DMG |
| "No updates available" | Build number not increased | Ensure sparkle:version increments |
| "Update incompatible" | Minimum version mismatch | Check sparkle:minimumSystemVersion |

### Verifying Signatures

```bash
# Verify DMG signature manually
openssl dgst -sha256 -verify public_key.pem -signature signature.bin PhotonCast.dmg

# Check Sparkle signature in appcast
grep sparkle:edSignature appcast.xml
```

### Debugging Feed

```bash
# Validate RSS structure
curl -s https://api.photoncast.app/updates/appcast.xml | xmllint --format -

# Check enclosure URL is accessible
curl -I $(grep -o 'url="[^"]*"' appcast.xml | head -1 | sed 's/url="//;s/"$//')
```

## References

- [Sparkle Project](https://sparkle-project.org/)
- [Sparkle Documentation](https://sparkle-project.org/documentation/)
- [Publishing Updates](https://sparkle-project.org/documentation/publishing/)
- [Security & Signing](https://sparkle-project.org/documentation/#3-segue-for-security-concerns)
- [Ed25519](https://ed25519.cr.yp.to/)

## Related Files

- `resources/appcast-template.xml` - Appcast XML template
- `scripts/generate-appcast.sh` - Appcast generation script
- `scripts/generate-signing-key.sh` - Signing key generation
- `photoncast-core/src/platform/updates.rs` - UpdateManager implementation
