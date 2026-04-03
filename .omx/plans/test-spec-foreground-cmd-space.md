# Test Spec — Foreground Cmd+Space Launcher Behavior

## Purpose
Define how execution proves that Cmd+Space restores true foreground launcher behavior without widening the fix beyond launcher foregrounding.

## Preconditions
1. `.omx/plans/prd-foreground-cmd-space.md` exists.
2. `.omx/specs/deep-interview-foreground-cmd-space.md` remains the requirements source of truth.
3. Verification targets the installed `/Applications/PhotonCast.app`, not just local test binaries.
4. Accessibility permission is granted, or the verification report explicitly records that hotkey proof is blocked by TCC rather than launcher foreground logic.

## Stage 0 — Scope Guard
### Must inspect
- changed codepaths remain primarily within:
  - `crates/photoncast/src/event_loop.rs`
  - `crates/photoncast/src/main.rs`
  - `crates/photoncast/src/platform.rs` only if required

### Pass conditions
- No intentional changes to menu bar behavior, startup sequencing, or unrelated secondary windows.
- Dock behavior changes occur only if strictly required for launcher foregrounding.

## Stage 1 — Buildability
### Must run
- `cargo build -p photoncast -q`
- `cargo test -p photoncast --no-run`

### Pass conditions
- Build succeeds.
- Test binaries compile successfully.

## Stage 2 — Diagnostic Cleanliness
### Must run
- diagnostics on touched files

### Pass conditions
- No new diagnostics in changed launcher/activation files.

## Stage 3 — Release Validation Target
### Must run
- release rebuild via existing script
- local install via existing script to `/Applications/PhotonCast.app`

### Pass conditions
- Installed app bundle verifies successfully.
- Running process path is `/Applications/PhotonCast.app/Contents/MacOS/photoncast`.

## Stage 4 — Foreground Behavior Proof
### Must validate manually
With `show_in_dock = false`:
1. Ensure another normal app is frontmost, then press Cmd+Space when no launcher window is currently visible.
2. PhotonCast becomes the frontmost app.
3. The launcher window is key/focused and visibly on top of all normal apps.
4. Make another normal app frontmost again, dismiss/hide the launcher, then press Cmd+Space again.
5. Repeating Cmd+Space continues to foreground the launcher reliably in the existing-window path.

### Pass conditions
- All five checks above hold.

## Stage 5 — Regression Guard
### Must confirm
- No intentional changes to menu bar visibility behavior.
- No intentional changes to startup flow.
- No intentional changes to preferences/clipboard/quicklinks foreground behavior unless unavoidable and documented.

### Pass conditions
- Any broadened effect is either absent or explicitly documented as unavoidable.

## Rejection Rule
The pass fails if:
- PhotonCast still only becomes active without becoming truly frontmost.
- The launcher is not visibly key/on top after Cmd+Space.
- Only one launcher branch works (new-window open vs existing-window re-toggle).
- Hotkey proof is reported without separating TCC/accessibility blockers from launcher foreground behavior.
- The fix relies on broader Dock/menu bar/startup changes not justified by the PRD.
