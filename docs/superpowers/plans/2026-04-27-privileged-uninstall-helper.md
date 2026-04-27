# Privileged Uninstall Helper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a proper long-term SMJobBless-based privileged uninstall path for root-owned `/Applications/*.app` bundles that cannot be moved to Trash by the unprivileged PhotonCast app.

**Architecture:** Keep PhotonCast unprivileged and preserve the existing `NSFileManager::trashItemAtURL` path as the default. Add a separately signed privileged helper installed by `SMJobBless`; Rust calls a bundled Swift bridge binary that handles SMJobBless/XPC and talks to the root helper over a narrow, signed, validated interface.

**Tech Stack:** Rust workspace crates, Swift command-line bridge/helper, ServiceManagement `SMJobBless`, XPC, launchd plist, codesign, existing shell release/sign/install scripts.

---

## File Structure

- Create: `native/privileged-uninstall/PhotonCastPrivilegedClient.swift`
  - Unprivileged bridge executable launched by Rust.
  - Commands: `status`, `bless`, `uninstall`.
  - Performs SMJobBless and XPC calls; emits one JSON response on stdout.
- Create: `native/privileged-uninstall/PhotonCastPrivilegedHelper.swift`
  - Root launchd helper.
  - Exposes XPC service `com.photoncast.privileged-uninstall.helper`.
  - Validates client code requirement and target app bundle before mutation.
- Create: `native/privileged-uninstall/PhotonCastPrivilegedShared.swift`
  - Shared Codable request/response structs and validation constants.
- Create: `resources/privileged-helper/com.photoncast.privileged-uninstall.helper.plist`
  - launchd plist for SMJobBless helper.
- Create: `resources/privileged-helper/helper-info.plist`
  - Helper signing metadata, including `SMAuthorizedClients`.
- Modify: `resources/Info.plist`
  - Add `SMPrivilegedExecutables` entry for the helper.
- Create: `crates/photoncast-apps/src/privileged.rs`
  - Rust client abstraction that invokes the Swift bridge and parses JSON responses.
- Modify: `crates/photoncast-apps/src/lib.rs`
  - Export privileged uninstall API.
- Modify: `crates/photoncast-apps/src/error.rs`
  - Add structured privileged uninstall errors.
- Modify: `crates/photoncast-apps/src/uninstaller.rs`
  - Detect permission-denied Trash failures and preserve enough context for admin fallback.
- Modify: `crates/photoncast/src/launcher/uninstall.rs`
  - Add admin fallback flow after permission-denied uninstall failure.
- Modify: `crates/photoncast/src/launcher/mod.rs`
  - Extend `UninstallState` and initialization.
- Modify: `crates/photoncast/src/launcher/render.rs`
  - Add persistent privileged uninstall confirmation/error UI.
- Modify: `crates/photoncast/src/launcher/search.rs`
  - Preserve modal/search behavior when privileged uninstall confirmation is visible.
- Modify: `scripts/release-build.sh`
  - Build Swift bridge/helper and place them in the app bundle.
- Modify: `scripts/sign.sh`
  - Sign helper and bridge before signing app bundle; verify SMJobBless metadata.
- Create: `scripts/verify-privileged-helper.sh`
  - Verifies helper files, signatures, plist requirements, and launchd labels.

Commit steps are intentionally omitted from this plan because repository policy says to commit only when the user explicitly asks.

---

## Task 1: Shared Rust error model for privileged fallback

**Files:**
- Modify: `crates/photoncast-apps/src/error.rs`
- Modify: `crates/photoncast-apps/src/uninstaller.rs`

- [ ] **Step 1: Write failing tests for permission classification**

Add tests in `crates/photoncast-apps/src/uninstaller.rs` test module:

```rust
#[test]
fn test_trash_error_detects_permission_denied_text() {
    assert!(super::is_trash_permission_denied(
        "“Brave Browser” couldn’t be moved to the trash because you don’t have permission to access it."
    ));
}

#[test]
fn test_trash_error_does_not_treat_missing_file_as_permission_denied() {
    assert!(!super::is_trash_permission_denied(
        "The file doesn’t exist."
    ));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
rtk cargo test -p photoncast-apps test_trash_error_detects_permission_denied_text test_trash_error_does_not_treat_missing_file_as_permission_denied
```

Expected: fail because `is_trash_permission_denied` does not exist.

- [ ] **Step 3: Add structured error fields**

Change `AppError::Trash` in `crates/photoncast-apps/src/error.rs` to include `permission_denied`:

```rust
#[error("failed to move {path} to Trash: {message}. {hint}")]
Trash {
    path: String,
    message: String,
    hint: String,
    permission_denied: bool,
},
```

Update all construction sites to set the new field.

- [ ] **Step 4: Add permission classifier**

Add near `move_to_trash` in `crates/photoncast-apps/src/uninstaller.rs`:

```rust
fn is_trash_permission_denied(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("permission")
        || lower.contains("not permitted")
        || lower.contains("not allowed")
        || lower.contains("access")
}
```

In `move_to_trash`, set:

```rust
let permission_denied = is_trash_permission_denied(&error_msg);
let hint = if permission_denied {
    "This app requires administrator permission. Use privileged uninstall to remove it.".to_string()
} else {
    "Quit the app and any helper processes, then try uninstalling again.".to_string()
};
```

- [ ] **Step 5: Run package tests**

Run:

```bash
rtk cargo test -p photoncast-apps
```

Expected: all tests pass.

---

## Task 2: Swift shared protocol and target validation

**Files:**
- Create: `native/privileged-uninstall/PhotonCastPrivilegedShared.swift`

- [ ] **Step 1: Create shared request/response types and validation constants**

Create `native/privileged-uninstall/PhotonCastPrivilegedShared.swift`:

```swift
import Foundation

let photonCastAppBundleID = "com.photoncast.app"
let helperBundleID = "com.photoncast.privileged-uninstall.helper"
let helperMachServiceName = "com.photoncast.privileged-uninstall.helper"

enum PrivilegedCommand: String, Codable {
    case status
    case bless
    case uninstall
}

enum UninstallMode: String, Codable {
    case trashFirst
    case deleteConfirmed
}

struct PrivilegedRequest: Codable {
    let command: PrivilegedCommand
    let path: String?
    let mode: UninstallMode?
    let requestID: String
}

struct PrivilegedResponse: Codable {
    let ok: Bool
    let requestID: String
    let code: String
    let message: String
    let operation: String?
}

enum ValidationError: Error, CustomStringConvertible {
    case missingPath
    case relativePath(String)
    case cannotCanonicalize(String)
    case notDirectory(String)
    case notAppBundle(String)
    case outsideApplications(String)
    case protectedSystemPath(String)
    case photonCastSelf(String)
    case missingInfoPlist(String)
    case missingBundleIdentifier(String)
    case appleBundle(String)

    var description: String {
        switch self {
        case .missingPath: return "missing app path"
        case .relativePath(let path): return "path is not absolute: \(path)"
        case .cannotCanonicalize(let path): return "cannot canonicalize path: \(path)"
        case .notDirectory(let path): return "path is not a directory: \(path)"
        case .notAppBundle(let path): return "path is not an .app bundle: \(path)"
        case .outsideApplications(let path): return "path is outside /Applications: \(path)"
        case .protectedSystemPath(let path): return "path is protected: \(path)"
        case .photonCastSelf(let path): return "refusing to uninstall PhotonCast: \(path)"
        case .missingInfoPlist(let path): return "missing Contents/Info.plist: \(path)"
        case .missingBundleIdentifier(let path): return "missing CFBundleIdentifier: \(path)"
        case .appleBundle(let bundleID): return "refusing to uninstall Apple bundle: \(bundleID)"
        }
    }
}

func validateApplicationBundle(_ rawPath: String?) throws -> URL {
    guard let rawPath, !rawPath.isEmpty else { throw ValidationError.missingPath }
    guard rawPath.hasPrefix("/") else { throw ValidationError.relativePath(rawPath) }

    let url = URL(fileURLWithPath: rawPath).standardizedFileURL
    let canonical = url.resolvingSymlinksInPath()
    let path = canonical.path

    guard FileManager.default.fileExists(atPath: path) else {
        throw ValidationError.cannotCanonicalize(rawPath)
    }

    var isDirectory: ObjCBool = false
    guard FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory), isDirectory.boolValue else {
        throw ValidationError.notDirectory(path)
    }

    guard canonical.pathExtension == "app" else { throw ValidationError.notAppBundle(path) }
    guard canonical.deletingLastPathComponent().path == "/Applications" else {
        throw ValidationError.outsideApplications(path)
    }
    guard !path.hasPrefix("/System/") else { throw ValidationError.protectedSystemPath(path) }
    guard canonical.lastPathComponent != "PhotonCast.app" else { throw ValidationError.photonCastSelf(path) }

    let infoURL = canonical.appendingPathComponent("Contents/Info.plist")
    guard FileManager.default.fileExists(atPath: infoURL.path) else {
        throw ValidationError.missingInfoPlist(path)
    }
    guard let info = NSDictionary(contentsOf: infoURL),
          let bundleID = info["CFBundleIdentifier"] as? String,
          !bundleID.isEmpty else {
        throw ValidationError.missingBundleIdentifier(path)
    }
    guard !bundleID.hasPrefix("com.apple.") else { throw ValidationError.appleBundle(bundleID) }
    guard bundleID != photonCastAppBundleID else { throw ValidationError.photonCastSelf(path) }

    return canonical
}
```

- [ ] **Step 2: Compile shared Swift file alone**

Run:

```bash
xcrun swiftc -parse native/privileged-uninstall/PhotonCastPrivilegedShared.swift
```

Expected: no output and exit 0.

---

## Task 3: Privileged helper executable skeleton

**Files:**
- Create: `native/privileged-uninstall/PhotonCastPrivilegedHelper.swift`

- [ ] **Step 1: Create helper XPC service skeleton**

Create `native/privileged-uninstall/PhotonCastPrivilegedHelper.swift`:

```swift
import Foundation

@objc protocol PhotonCastPrivilegedUninstallProtocol {
    func handle(_ data: Data, withReply reply: @escaping (Data) -> Void)
}

final class HelperDelegate: NSObject, NSXPCListenerDelegate {
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        connection.exportedInterface = NSXPCInterface(with: PhotonCastPrivilegedUninstallProtocol.self)
        connection.exportedObject = PrivilegedUninstallService()
        connection.resume()
        return true
    }
}

final class PrivilegedUninstallService: NSObject, PhotonCastPrivilegedUninstallProtocol {
    func handle(_ data: Data, withReply reply: @escaping (Data) -> Void) {
        let response: PrivilegedResponse
        do {
            let request = try JSONDecoder().decode(PrivilegedRequest.self, from: data)
            response = try handleRequest(request)
        } catch {
            response = PrivilegedResponse(
                ok: false,
                requestID: "unknown",
                code: "invalid-request",
                message: String(describing: error),
                operation: nil
            )
        }
        let encoded = (try? JSONEncoder().encode(response)) ?? Data()
        reply(encoded)
    }

    private func handleRequest(_ request: PrivilegedRequest) throws -> PrivilegedResponse {
        switch request.command {
        case .status:
            return PrivilegedResponse(ok: true, requestID: request.requestID, code: "ok", message: "helper available", operation: nil)
        case .bless:
            return PrivilegedResponse(ok: true, requestID: request.requestID, code: "ok", message: "helper already installed", operation: nil)
        case .uninstall:
            let target = try validateApplicationBundle(request.path)
            return performUninstall(target: target, mode: request.mode ?? .trashFirst, requestID: request.requestID)
        }
    }

    private func performUninstall(target: URL, mode: UninstallMode, requestID: String) -> PrivilegedResponse {
        switch mode {
        case .trashFirst:
            do {
                var resultingURL: NSURL?
                try FileManager.default.trashItem(at: target, resultingItemURL: &resultingURL)
                return PrivilegedResponse(ok: true, requestID: requestID, code: "trashed", message: "moved to Trash", operation: "trash")
            } catch {
                return PrivilegedResponse(
                    ok: false,
                    requestID: requestID,
                    code: "needs-delete-confirmation",
                    message: "Privileged Trash move failed: \(error.localizedDescription)",
                    operation: nil
                )
            }
        case .deleteConfirmed:
            do {
                try FileManager.default.removeItem(at: target)
                return PrivilegedResponse(ok: true, requestID: requestID, code: "deleted", message: "deleted app bundle", operation: "delete")
            } catch {
                return PrivilegedResponse(ok: false, requestID: requestID, code: "delete-failed", message: error.localizedDescription, operation: nil)
            }
        }
    }
}

let delegate = HelperDelegate()
let listener = NSXPCListener(machServiceName: helperMachServiceName)
listener.delegate = delegate
listener.resume()
RunLoop.main.run()
```

- [ ] **Step 2: Compile helper skeleton**

Run:

```bash
xcrun swiftc native/privileged-uninstall/PhotonCastPrivilegedShared.swift native/privileged-uninstall/PhotonCastPrivilegedHelper.swift -o /tmp/photoncast-privileged-helper
```

Expected: helper binary exists at `/tmp/photoncast-privileged-helper`.

---

## Task 4: Client bridge skeleton

**Files:**
- Create: `native/privileged-uninstall/PhotonCastPrivilegedClient.swift`

- [ ] **Step 1: Create JSON-speaking client bridge**

Create `native/privileged-uninstall/PhotonCastPrivilegedClient.swift`:

```swift
import Foundation
import ServiceManagement

func emit(_ response: PrivilegedResponse) -> Never {
    let data = try! JSONEncoder().encode(response)
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write("\n".data(using: .utf8)!)
    exit(response.ok ? 0 : 1)
}

func requestID() -> String { UUID().uuidString }

let args = CommandLine.arguments.dropFirst()
guard let commandName = args.first, let command = PrivilegedCommand(rawValue: commandName) else {
    emit(PrivilegedResponse(ok: false, requestID: requestID(), code: "usage", message: "usage: photoncast-privileged-client status|bless|uninstall <path> [trashFirst|deleteConfirmed]", operation: nil))
}

let path = args.dropFirst().first.map(String.init)
let mode = args.dropFirst().dropFirst().first.flatMap { UninstallMode(rawValue: String($0)) }
let req = PrivilegedRequest(command: command, path: path, mode: mode, requestID: requestID())

if command == .bless {
    var error: Unmanaged<CFError>?
    let ok = SMJobBless(kSMDomainSystemLaunchd, helperBundleID as CFString, nil, &error)
    if ok {
        emit(PrivilegedResponse(ok: true, requestID: req.requestID, code: "blessed", message: "helper installed", operation: nil))
    }
    let message = error?.takeRetainedValue().localizedDescription ?? "SMJobBless failed"
    emit(PrivilegedResponse(ok: false, requestID: req.requestID, code: "bless-failed", message: message, operation: nil))
}

let connection = NSXPCConnection(machServiceName: helperMachServiceName, options: .privileged)
connection.remoteObjectInterface = NSXPCInterface(with: PhotonCastPrivilegedUninstallProtocol.self)
connection.resume()

let semaphore = DispatchSemaphore(value: 0)
var finalResponse: PrivilegedResponse?

let proxy = connection.remoteObjectProxyWithErrorHandler { error in
    finalResponse = PrivilegedResponse(ok: false, requestID: req.requestID, code: "xpc-error", message: error.localizedDescription, operation: nil)
    semaphore.signal()
} as! PhotonCastPrivilegedUninstallProtocol

let data = try JSONEncoder().encode(req)
proxy.handle(data) { responseData in
    finalResponse = try? JSONDecoder().decode(PrivilegedResponse.self, from: responseData)
    semaphore.signal()
}

_ = semaphore.wait(timeout: .now() + 30)
connection.invalidate()

emit(finalResponse ?? PrivilegedResponse(ok: false, requestID: req.requestID, code: "timeout", message: "helper did not respond", operation: nil))
```

- [ ] **Step 2: Compile client skeleton**

Run:

```bash
xcrun swiftc native/privileged-uninstall/PhotonCastPrivilegedShared.swift native/privileged-uninstall/PhotonCastPrivilegedHelper.swift native/privileged-uninstall/PhotonCastPrivilegedClient.swift -o /tmp/photoncast-privileged-client
```

Expected: this may fail because the helper file contains a process entrypoint. If it fails, split the protocol declaration into `PhotonCastPrivilegedShared.swift` and remove protocol duplication from helper/client so each executable compiles with shared types only.

---

## Task 5: SMJobBless plist metadata

**Files:**
- Create: `resources/privileged-helper/com.photoncast.privileged-uninstall.helper.plist`
- Create: `resources/privileged-helper/helper-info.plist`
- Modify: `resources/Info.plist`

- [ ] **Step 1: Add launchd plist**

Create `resources/privileged-helper/com.photoncast.privileged-uninstall.helper.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.photoncast.privileged-uninstall.helper</string>
    <key>MachServices</key>
    <dict>
        <key>com.photoncast.privileged-uninstall.helper</key>
        <true/>
    </dict>
    <key>ProgramArguments</key>
    <array>
        <string>/Library/PrivilegedHelperTools/com.photoncast.privileged-uninstall.helper</string>
    </array>
    <key>RunAtLoad</key>
    <false/>
</dict>
</plist>
```

- [ ] **Step 2: Add helper info plist**

Create `resources/privileged-helper/helper-info.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.photoncast.privileged-uninstall.helper</string>
    <key>CFBundleName</key>
    <string>PhotonCastPrivilegedUninstallHelper</string>
    <key>CFBundleVersion</key>
    <string>0.1.0-beta</string>
    <key>SMAuthorizedClients</key>
    <array>
        <string>identifier com.photoncast.app and anchor trusted</string>
    </array>
</dict>
</plist>
```

- [ ] **Step 3: Add main app SMPrivilegedExecutables entry**

Add before the closing `</dict>` in `resources/Info.plist`:

```xml
    <key>SMPrivilegedExecutables</key>
    <dict>
        <key>com.photoncast.privileged-uninstall.helper</key>
        <string>identifier com.photoncast.privileged-uninstall.helper and anchor trusted</string>
    </dict>
```

- [ ] **Step 4: Validate plists**

Run:

```bash
plutil -lint resources/Info.plist resources/privileged-helper/com.photoncast.privileged-uninstall.helper.plist resources/privileged-helper/helper-info.plist
```

Expected: all files report `OK`.

---

## Task 6: Build and signing integration

**Files:**
- Modify: `scripts/release-build.sh`
- Modify: `scripts/sign.sh`
- Create: `scripts/verify-privileged-helper.sh`

- [ ] **Step 1: Add Swift build section to release script**

In `scripts/release-build.sh`, after line 86 where the extension runner is copied, add a section that creates `Contents/Library/LaunchServices`, compiles the Swift helper/client, and copies plists:

```bash
PRIVILEGED_DIR="${CONTENTS_DIR}/Library/LaunchServices"
mkdir -p "${PRIVILEGED_DIR}"

echo -e "${BLUE}Building privileged uninstall helper...${NC}"
xcrun swiftc \
    "${PROJECT_ROOT}/native/privileged-uninstall/PhotonCastPrivilegedShared.swift" \
    "${PROJECT_ROOT}/native/privileged-uninstall/PhotonCastPrivilegedHelper.swift" \
    -o "${PRIVILEGED_DIR}/com.photoncast.privileged-uninstall.helper"

echo -e "${BLUE}Building privileged uninstall client...${NC}"
xcrun swiftc \
    "${PROJECT_ROOT}/native/privileged-uninstall/PhotonCastPrivilegedShared.swift" \
    "${PROJECT_ROOT}/native/privileged-uninstall/PhotonCastPrivilegedClient.swift" \
    -o "${MACOS_DIR}/photoncast-privileged-client"

cp "${PROJECT_ROOT}/resources/privileged-helper/com.photoncast.privileged-uninstall.helper.plist" \
   "${PRIVILEGED_DIR}/com.photoncast.privileged-uninstall.helper.plist"
chmod +x "${PRIVILEGED_DIR}/com.photoncast.privileged-uninstall.helper"
chmod +x "${MACOS_DIR}/photoncast-privileged-client"
```

- [ ] **Step 2: Sign helper before app bundle**

In `scripts/sign.sh`, before signing every executable in `Contents/MacOS`, add signing for `Contents/Library/LaunchServices` using helper info plist:

```bash
HELPER_INFO_PLIST="${PROJECT_ROOT}/resources/privileged-helper/helper-info.plist"
LAUNCH_SERVICES_DIR="${APP_BUNDLE}/Contents/Library/LaunchServices"
if [[ -d "$LAUNCH_SERVICES_DIR" ]]; then
    find "$LAUNCH_SERVICES_DIR" -type f ! -name "*.plist" | while read -r file; do
        echo -e "${BLUE}Signing privileged helper: ${file}${NC}"
        codesign --sign "${SIGNING_REF}" --force --timestamp \
            ${SIGNING_KEYCHAIN:+--keychain "${SIGNING_KEYCHAIN}"} \
            --info "${HELPER_INFO_PLIST}" \
            "${file}"
    done
fi
```

If shellcheck or quoting fails on `${SIGNING_KEYCHAIN:+...}`, replace it with an array-based `codesign_args` implementation instead of using `eval`.

- [ ] **Step 3: Add verifier script**

Create `scripts/verify-privileged-helper.sh`:

```bash
#!/bin/bash
set -euo pipefail

APP_BUNDLE="${1:-build/PhotonCast.app}"
HELPER="${APP_BUNDLE}/Contents/Library/LaunchServices/com.photoncast.privileged-uninstall.helper"
HELPER_PLIST="${APP_BUNDLE}/Contents/Library/LaunchServices/com.photoncast.privileged-uninstall.helper.plist"
CLIENT="${APP_BUNDLE}/Contents/MacOS/photoncast-privileged-client"

test -x "$HELPER"
test -f "$HELPER_PLIST"
test -x "$CLIENT"
codesign --verify --strict --verbose=2 "$HELPER"
codesign --verify --strict --verbose=2 "$CLIENT"
plutil -lint "$HELPER_PLIST"
codesign -d --entitlements - "$APP_BUNDLE" >/dev/null 2>&1 || true
echo "privileged helper bundle verification passed"
```

Make it executable:

```bash
chmod +x scripts/verify-privileged-helper.sh
```

- [ ] **Step 4: Build release bundle**

Run:

```bash
./scripts/release-build.sh
./scripts/verify-privileged-helper.sh build/PhotonCast.app
```

Expected: release build succeeds, helper/client exist, signatures verify.

---

## Task 7: Rust privileged client wrapper

**Files:**
- Create: `crates/photoncast-apps/src/privileged.rs`
- Modify: `crates/photoncast-apps/src/lib.rs`
- Modify: `crates/photoncast-apps/src/error.rs`

- [ ] **Step 1: Add privileged error variants**

In `crates/photoncast-apps/src/error.rs`, add:

```rust
#[error("privileged uninstall unavailable: {0}")]
PrivilegedUnavailable(String),

#[error("privileged uninstall failed: {0}")]
PrivilegedFailed(String),
```

- [ ] **Step 2: Create Rust bridge wrapper**

Create `crates/photoncast-apps/src/privileged.rs`:

```rust
use crate::error::{AppError, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct PrivilegedResponse {
    pub ok: bool,
    pub requestID: String,
    pub code: String,
    pub message: String,
    pub operation: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum PrivilegedUninstallMode {
    TrashFirst,
    DeleteConfirmed,
}

impl PrivilegedUninstallMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::TrashFirst => "trashFirst",
            Self::DeleteConfirmed => "deleteConfirmed",
        }
    }
}

pub fn uninstall_with_privileges(path: &Path, mode: PrivilegedUninstallMode) -> Result<PrivilegedResponse> {
    let client = privileged_client_path()?;
    let output = Command::new(&client)
        .arg("uninstall")
        .arg(path)
        .arg(mode.as_str())
        .output()
        .map_err(|e| AppError::PrivilegedUnavailable(format!("failed to run {}: {}", client.display(), e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: PrivilegedResponse = serde_json::from_str(stdout.trim())
        .map_err(|e| AppError::PrivilegedFailed(format!("invalid helper response: {}; stdout={}", e, stdout)))?;

    if response.ok {
        Ok(response)
    } else {
        Err(AppError::PrivilegedFailed(format!("{}: {}", response.code, response.message)))
    }
}

fn privileged_client_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::PrivilegedUnavailable(format!("cannot locate current executable: {}", e)))?;
    let macos_dir = exe.parent()
        .ok_or_else(|| AppError::PrivilegedUnavailable("current executable has no parent directory".to_string()))?;
    let client = macos_dir.join("photoncast-privileged-client");
    if client.is_file() {
        Ok(client)
    } else {
        Err(AppError::PrivilegedUnavailable(format!("missing privileged client at {}", client.display())))
    }
}
```

- [ ] **Step 3: Export module**

In `crates/photoncast-apps/src/lib.rs`, add:

```rust
pub mod privileged;
pub use privileged::{uninstall_with_privileges, PrivilegedResponse, PrivilegedUninstallMode};
```

- [ ] **Step 4: Run checks**

Run:

```bash
rtk cargo test -p photoncast-apps
rtk cargo clippy -p photoncast-apps --all-targets -- -D warnings
```

Expected: tests pass and clippy reports no issues.

---

## Task 8: Launcher privileged uninstall UX

**Files:**
- Modify: `crates/photoncast/src/launcher/uninstall.rs`
- Modify: `crates/photoncast/src/launcher/mod.rs`
- Modify: `crates/photoncast/src/launcher/render.rs`
- Modify: `crates/photoncast/src/launcher/search.rs`

- [ ] **Step 1: Confirm uninstall render/state fields**

Run:

```bash
grep -R "uninstall\.preview\|show_toast\|UninstallState\|files_selected_index" crates/photoncast/src/launcher
```

Expected: see `UninstallState` in `launcher/mod.rs`, uninstall modal rendering in `launcher/render.rs`, search gating in `launcher/search.rs`, and actions in `launcher/uninstall.rs`.

- [ ] **Step 2: Add state for privileged fallback**

Add fields to the uninstall state struct:

```rust
pub privileged_error: Option<String>,
pub privileged_target: Option<std::path::PathBuf>,
pub awaiting_delete_confirmation: bool,
```

Initialize them to `None`, `None`, and `false` wherever uninstall state is constructed.

- [ ] **Step 3: Preserve preview on permission-denied failure**

Change `perform_uninstall` in `crates/photoncast/src/launcher/uninstall.rs` so permission-denied `AppError::Trash` does not discard the preview permanently:

```rust
Err(photoncast_apps::AppError::Trash { permission_denied: true, path, message, .. }) => {
    self.uninstall.privileged_target = Some(std::path::PathBuf::from(path));
    self.uninstall.privileged_error = Some(message);
    self.uninstall.preview = Some(preview);
    cx.notify();
}
```

For non-permission errors, keep the existing toast behavior.

- [ ] **Step 4: Add admin uninstall action**

Add a method next to `perform_uninstall`:

```rust
pub(super) fn perform_privileged_uninstall(&mut self, cx: &mut ViewContext<Self>) {
    let Some(path) = self.uninstall.privileged_target.clone() else {
        self.show_toast("No privileged uninstall target".to_string(), cx);
        return;
    };

    match photoncast_apps::uninstall_with_privileges(&path, photoncast_apps::PrivilegedUninstallMode::TrashFirst) {
        Ok(response) => {
            tracing::info!("Privileged uninstall succeeded: {} {}", response.code, path.display());
            self.uninstall.preview = None;
            self.uninstall.privileged_target = None;
            self.uninstall.privileged_error = None;
            self.show_toast(format!("{}", response.message), cx);
            self.hide(cx);
        },
        Err(e) => {
            tracing::error!("Privileged uninstall failed for {}: {}", path.display(), e);
            self.uninstall.privileged_error = Some(e.to_string());
            cx.notify();
        },
    }
}
```

- [ ] **Step 5: Render persistent error with admin button**

In the uninstall preview render file, add a persistent error block when `privileged_target` is present:

```rust
div()
    .child("Administrator permission is required to uninstall this app.")
    .child("PhotonCast will ask macOS to authorize its privileged helper.")
    .child(button("Uninstall with administrator privileges"))
```

Wire the button to `perform_privileged_uninstall`. Use the project’s existing button/action style; do not use a transient toast for this error.

- [ ] **Step 6: Run app crate checks**

Run:

```bash
rtk cargo test -p photoncast
rtk cargo clippy -p photoncast --all-targets -- -D warnings
```

Expected: tests pass and clippy reports no issues.

---

## Task 9: Helper installation and manual QA

**Files:**
- Modify: `native/privileged-uninstall/PhotonCastPrivilegedClient.swift`
- Modify: `native/privileged-uninstall/PhotonCastPrivilegedHelper.swift`
- Modify: `scripts/verify-privileged-helper.sh`

- [ ] **Step 1: Add status command behavior**

Ensure the client `status` command connects to XPC and returns `ok=true` when the helper is available. If it receives `xpc-error`, the main app can run `bless`.

- [ ] **Step 2: Add bless-before-uninstall behavior in Rust wrapper**

In `crates/photoncast-apps/src/privileged.rs`, when uninstall returns `xpc-error`, run the client with `bless`, then retry uninstall once:

```rust
fn bless_helper(client: &Path) -> Result<()> {
    let output = Command::new(client)
        .arg("bless")
        .output()
        .map_err(|e| AppError::PrivilegedUnavailable(format!("failed to run bless: {}", e)))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(AppError::PrivilegedFailed(String::from_utf8_lossy(&output.stdout).to_string()))
    }
}
```

- [ ] **Step 3: Build and install release app**

Run:

```bash
./scripts/release-build.sh
./scripts/install-app.sh
```

Expected: bundle builds, signs, installs, and launches.

- [ ] **Step 4: Verify helper status before blessing**

Run:

```bash
"/Applications/PhotonCast.app/Contents/MacOS/photoncast-privileged-client" status
```

Expected before blessing: JSON response with `ok=false` and `code=xpc-error`, or `ok=true` if already installed.

- [ ] **Step 5: Trigger privileged uninstall manually with a fixture app**

Create a root-owned fixture app, not Brave:

```bash
mkdir -p /tmp/RootOwnedFixture.app/Contents
/usr/libexec/PlistBuddy -c 'Add :CFBundleIdentifier string com.example.RootOwnedFixture' /tmp/RootOwnedFixture.app/Contents/Info.plist
sudo rm -rf /Applications/RootOwnedFixture.app
sudo mv /tmp/RootOwnedFixture.app /Applications/RootOwnedFixture.app
sudo chown -R root:wheel /Applications/RootOwnedFixture.app
"/Applications/PhotonCast.app/Contents/MacOS/photoncast-privileged-client" bless
"/Applications/PhotonCast.app/Contents/MacOS/photoncast-privileged-client" uninstall "/Applications/RootOwnedFixture.app" trashFirst
```

Expected: admin prompt appears during bless; uninstall returns JSON. If trash fails, retry with `deleteConfirmed` only after confirming this fixture is safe to remove.

- [ ] **Step 6: Verify rejected paths**

Run:

```bash
"/Applications/PhotonCast.app/Contents/MacOS/photoncast-privileged-client" uninstall "/System/Applications/Calculator.app" trashFirst
"/Applications/PhotonCast.app/Contents/MacOS/photoncast-privileged-client" uninstall "/Applications/PhotonCast.app" trashFirst
```

Expected: both return JSON with `ok=false` and `code` describing rejection.

---

## Task 10: Final verification

**Files:**
- All modified files from prior tasks.

- [ ] **Step 1: Run Rust diagnostics**

Run LSP diagnostics on changed Rust files:

```text
crates/photoncast-apps/src/error.rs
crates/photoncast-apps/src/uninstaller.rs
crates/photoncast-apps/src/privileged.rs
crates/photoncast-apps/src/lib.rs
crates/photoncast/src/launcher/uninstall.rs
```

Expected: no diagnostics.

- [ ] **Step 2: Run format and tests**

Run:

```bash
rtk cargo fmt --check
rtk cargo test -p photoncast-apps
rtk cargo test -p photoncast
rtk cargo clippy -p photoncast-apps --all-targets -- -D warnings
rtk cargo clippy -p photoncast --all-targets -- -D warnings
```

Expected: all commands exit 0.

- [ ] **Step 3: Run release verification**

Run:

```bash
./scripts/release-build.sh
./scripts/verify-privileged-helper.sh build/PhotonCast.app
codesign --verify --deep --strict --verbose=2 build/PhotonCast.app
```

Expected: app, client, helper, and nested signatures verify.

- [ ] **Step 4: Run manual Brave QA only after fixture QA passes**

Launch installed PhotonCast and attempt Brave uninstall. Expected behavior:

1. Normal Trash path fails with permission denied.
2. PhotonCast shows persistent admin uninstall UI.
3. Admin authorization prompt appears.
4. Helper validates `/Applications/Brave Browser.app`.
5. Helper either moves the app to Trash or requests explicit delete confirmation.
6. No transient-only error is the sole diagnostic.

---

## Self-Review

- Spec coverage: covers SMJobBless helper, narrow API, validation, signing, helper lifecycle, logging/error UX, tests, and Brave manual QA.
- Placeholder scan: no unresolved placeholder tokens or intentionally vague implementation steps remain.
- Type consistency: Rust names are `PrivilegedResponse`, `PrivilegedUninstallMode`, and `uninstall_with_privileges`; Swift request/response names match across client/helper/shared files.
- Known implementation risk: Swift client/helper compilation may require splitting the protocol declaration into a fourth shared file if entrypoint code conflicts during compilation. The plan includes the expected failure and the exact corrective split.
