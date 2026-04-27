# Privileged Uninstall Helper Design

Date: 2026-04-27
Status: Approved design, not yet implemented

## Problem

PhotonCast can uninstall user-writable applications by moving them to Trash with `NSFileManager::trashItemAtURL`, but that call fails for some `/Applications` bundles owned by `root:wheel`, such as `/Applications/Brave Browser.app`:

> “Brave Browser” couldn’t be moved to the trash because you don’t have permission to access it.

Finder can handle this class of operation by using macOS authorization paths. PhotonCast currently has no privileged component, so it cannot perform administrator-authorized uninstall operations safely or correctly.

## Goals

- Provide a long-term, Apple-aligned privileged uninstall path for root-owned app bundles.
- Keep the main PhotonCast process unprivileged.
- Require explicit user/admin authorization before any privileged operation.
- Avoid shell execution, `sudo`, AppleScript elevation, and arbitrary file operations.
- Preserve normal Trash-based uninstall behavior for apps that do not need elevated privileges.
- Make privileged failures debuggable through persistent app-visible errors and logs.

## Non-goals

- Do not build a generic privileged file manager.
- Do not support arbitrary paths outside approved application directories.
- Do not silently elevate uninstall operations.
- Do not uninstall system apps or PhotonCast itself.
- Do not implement App Store compatibility in this phase; this is for signed/notarized non-App-Store distribution.

## Recommended Architecture

Use an `SMJobBless` privileged helper installed into launchd after administrator authorization.

PhotonCast remains a normal user-space app. It embeds a separately signed helper executable and the required launchd metadata. When a user chooses an elevated uninstall action for a root-owned app, PhotonCast ensures the helper is installed and authorized through `SMJobBless`, then sends a narrow uninstall request to the helper over IPC.

The helper performs strict validation and then executes the privileged operation. The helper exposes only one bounded capability: uninstalling validated third-party application bundles from `/Applications`.

## Components

### Main app: `photoncast`

Responsibilities:

- Continue using the existing normal uninstall path first.
- Detect permission-denied Trash failures from `photoncast-apps`.
- Offer a clearly labeled admin action, such as “Uninstall with administrator privileges.”
- Install or update the helper with `SMJobBless` when needed.
- Send an uninstall request to the helper.
- Display persistent error details when privileged uninstall fails.

### App management crate: `photoncast-apps`

Responsibilities:

- Keep normal uninstall logic and preview scanning.
- Add a privileged-uninstall client abstraction that the main app can call.
- Map helper responses into structured app errors.
- Keep validation helpers shared where possible, but treat helper-side validation as authoritative.

### Privileged helper

Responsibilities:

- Run as root under launchd only after user authorization.
- Accept IPC requests from the authorized PhotonCast app only.
- Validate caller identity using code-signing requirements.
- Validate target paths before any filesystem mutation.
- Move or delete the app bundle according to the approved policy.
- Emit auditable logs for all accepted and rejected privileged requests.

## Helper API

Initial API surface:

```text
uninstall_application(path: String, mode: UninstallMode) -> UninstallResult
```

`UninstallMode`:

- `TrashFirst`: attempt safe Trash semantics first; if unavailable, return a typed failure requiring explicit delete confirmation.
- `DeleteConfirmed`: delete directly after the main app has obtained explicit user confirmation.

`UninstallResult`:

- `Succeeded { removed_path, operation }`
- `NeedsDeleteConfirmation { reason }`
- `Rejected { reason }`
- `Failed { reason, underlying_error }`

The helper must not accept shell snippets, glob patterns, relative paths, or arbitrary operation names.

## Validation Rules

The helper must canonicalize and validate the target immediately before mutation.

Required checks:

- Path must be absolute.
- Path must canonicalize successfully.
- Path must be a directory.
- Path must have `.app` extension.
- Path must be directly under `/Applications` for the first implementation.
- Path must not be under `/System`, `/System/Applications`, `/Library/Apple`, or other protected system locations.
- Path must not be `/Applications/PhotonCast.app`.
- Bundle must contain `Contents/Info.plist`.
- Bundle identifier must not start with `com.apple.`.
- Bundle identifier must not be `com.photoncast.app`.

The main app should perform the same checks for UX, but helper-side checks are the security boundary.

## Uninstall Behavior

Default behavior remains unchanged for user-writable applications:

1. Main app creates uninstall preview.
2. Main app calls normal Trash move.
3. Success returns immediately.

Permission-denied behavior:

1. Normal Trash move returns a structured permission failure.
2. PhotonCast shows an admin-uninstall action.
3. User chooses admin uninstall.
4. PhotonCast obtains/updates privileged helper authorization with `SMJobBless`.
5. PhotonCast sends the app path to the helper.
6. Helper validates the caller and target.
7. Helper attempts Trash-first behavior where feasible.
8. If reliable Trash semantics are not feasible as root, helper returns `NeedsDeleteConfirmation`.
9. PhotonCast asks for explicit delete confirmation.
10. Helper performs direct removal only after `DeleteConfirmed`.

This keeps data-loss risk lower than immediately deleting root-owned bundles, while still supporting cases where macOS cannot provide correct Trash semantics from a privileged helper.

## IPC and Trust Boundary

The preferred IPC mechanism is XPC because `SMJobBless` helpers are commonly paired with launchd/XPC services on macOS. If Rust-native XPC support is insufficient, the project can introduce a small Objective-C or Swift bridge for helper installation and IPC while keeping PhotonCast’s higher-level logic in Rust.

The helper must verify the connecting client’s code signature requirement, not merely trust the socket path or process name. The app bundle and helper must mutually declare code-signing requirements through `SMPrivilegedExecutables` and `SMAuthorizedClients`.

## Signing, Packaging, and Lifecycle

The release bundle must include:

- Helper executable in the expected bundle location.
- Helper launchd plist.
- Main app `SMPrivilegedExecutables` entry.
- Helper `SMAuthorizedClients` entry.
- Consistent signing identities and requirements for app/helper validation.

Build scripts must sign the helper before signing the containing app. Verification should check the helper signature, app signature, launchd plist presence, and blessing metadata.

Helper update policy:

- On app launch or first privileged uninstall, PhotonCast compares installed helper version with bundled helper version.
- If missing or outdated, PhotonCast requests authorization and blesses the bundled helper.

Helper removal policy:

- A future maintenance command may remove the helper with `SMJobRemove`.
- This is not required for the first privileged-uninstall implementation.

## Error Handling and Logging

The current transient toast is not enough for privileged operations.

Required UX changes:

- Privileged uninstall failures must remain visible until dismissed.
- Error details should include the target path, helper phase, and actionable next step.
- Admin-auth cancellation should be reported as cancellation, not failure.

Required logging changes:

- Add durable logging for app-side uninstall attempts.
- Add helper-side logs for accepted/rejected privileged requests.
- Include request IDs so app and helper logs can be correlated.

macOS Unified Logging is preferred for the helper and app-side privileged uninstall flow. If that is not implemented in the same phase, write a local log file under `~/Library/Logs/PhotonCast/` for the main app and a root-readable helper log location suitable for support diagnostics.

## Security Considerations

The privileged helper is a security boundary and must be intentionally small.

Rules:

- No arbitrary command execution.
- No recursive delete until all validation passes.
- No operations outside the approved app-bundle scope.
- No trust in main-app validation alone.
- No stringly typed shell commands.
- No broad “delete path” API.
- No following user-controlled symlink tricks into protected locations.
- No deleting system apps, Apple apps, or PhotonCast.

## Testing Strategy

Unit tests:

- Path validation accepts `/Applications/Example.app`.
- Path validation rejects relative paths.
- Path validation rejects non-`.app` paths.
- Path validation rejects `/System/Applications/*.app`.
- Path validation rejects `com.apple.*` bundle IDs.
- Path validation rejects PhotonCast.
- Helper request parser rejects malformed requests.

Integration tests:

- Normal user-writable app still uninstalls through existing Trash path.
- Root-owned fixture app triggers privileged fallback.
- Authorization cancellation is reported cleanly.
- Helper rejects invalid paths even if main app sends them.
- Helper version mismatch triggers update flow.

Manual QA:

- Install release bundle locally.
- Attempt Brave uninstall.
- Confirm admin authorization prompt appears.
- Confirm helper validates and performs the expected operation.
- Confirm error details persist if the operation fails.
- Confirm app/helper logs contain correlated request IDs.

## Implementation Phases

1. Introduce shared validation model and persistent uninstall error UI.
2. Add helper target, launchd plist, signing metadata, and build-script integration.
3. Add app-side helper install/update flow with `SMJobBless`.
4. Add app/helper IPC and request validation.
5. Implement privileged uninstall operation with Trash-first / delete-confirmed behavior.
6. Add tests and release verification.

## Confirmed Policy

Trash-first remains the desired default for privileged uninstall. Direct delete requires explicit confirmation because root-owned app deletion bypasses user Trash recovery semantics.
