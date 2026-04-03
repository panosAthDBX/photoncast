# Draft Plan — Foreground Cmd+Space Launcher Behavior

## RALPLAN-DR Summary
### Principles
1. Restore true launcher foregrounding on Cmd+Space before touching broader app behavior.
2. Keep the fix launcher-specific; do not widen into Dock/menu bar/startup changes.
3. Prefer the narrowest change that matches existing working window-activation patterns.
4. Prove behavior on the installed app, not just in unit-level abstractions.
5. Separate hotkey-permission confounders from launcher-foreground semantics during verification.

### Decision Drivers
1. The user explicitly wants PhotonCast to become the true frontmost app on every Cmd+Space press, even with `show_in_dock = false`.
2. Brownfield evidence shows launcher toggle handling lacks explicit `cx.activate(true)` / `cx.activate_window()` calls while other windows use them.
3. Recent Dock-hidden/activation-policy changes likely altered launcher foreground semantics, so the plan must isolate launcher-specific activation from broader Dock behavior.

### Viable Options
1. **Event-loop activation fix (preferred):** add explicit app/window activation to both launcher branches in `handle_toggle_launcher` (existing-window toggle and new-window open), matching other windows' activation behavior.
   - Pros: narrow, consistent with existing patterns, likely addresses the regression directly.
   - Cons: may be insufficient if Dock-hidden agent-mode needs an extra macOS/AppKit foreground call.
2. **Launcher-specific platform foreground helper:** add a small macOS helper to explicitly foreground the app/window before or during launcher toggle.
   - Pros: stronger control if GPUI activation alone is not enough in UIElement mode.
   - Cons: more platform-specific complexity; greater regression risk.
3. **Broader activation-policy redesign:** revisit the startup/UIElement activation model.
   - Rejected: exceeds scope and risks changing Dock/menu bar/startup semantics beyond the clarified brief.

## Objective
Restore Cmd+Space so PhotonCast becomes the true frontmost app and the launcher window is key/focused and visibly on top of all normal apps, including when `show_in_dock = false`.

## Source of Truth / Constraints
- Primary requirements source: `.omx/specs/deep-interview-foreground-cmd-space.md`
- Scope is limited to Cmd+Space / launcher foregrounding only.
- Do not intentionally change Dock presence, menu bar visibility, startup flow, or unrelated secondary windows.
- Preserve Dock-hidden mode as a supported capability.

## In Scope
- Hotkey-triggered launcher foregrounding path
- Existing-window launcher toggle activation/focus behavior
- New-window launcher open activation/focus behavior if needed for consistency
- Minimal platform-specific launcher foreground helper only if GPUI activation alone is insufficient
- Narrow codepaths:
  - `crates/photoncast/src/event_loop.rs`
  - `crates/photoncast/src/main.rs`
  - `crates/photoncast/src/platform.rs` (only if required)
  - optional narrow launcher-facing tests/harnesses under existing crates/tests

## Out of Scope
- Dock visibility preference behavior except where strictly required to allow launcher foregrounding
- Menu bar preference behavior
- Startup sequencing/performance work
- Preferences, clipboard, quicklinks, and other non-launcher window behavior changes
- Docs-only substitutes for runtime behavior

## Current Evidence Snapshot
- `AppEvent::ToggleLauncher` is emitted from the hotkey path.
- Existing launcher-toggle handling updates the launcher view and focuses self, but unlike other windows does not call `cx.activate(true)` / `cx.activate_window()`.
- `handle_toggle_launcher` has two behavior branches: existing launcher window vs creating a new launcher window, so both need explicit foreground proof.
- Launcher windows already open with `focus: true`, `show: true`, and `WindowKind::Normal`.
- The app now runs as a `UIElement` when Dock-hidden, so foregrounding may require stricter launcher-specific activation semantics.

## Acceptance Criteria
1. Pressing Cmd+Space from a state with no visible launcher window makes PhotonCast the frontmost app.
2. The launcher window becomes key/focused and visibly appears on top of all normal apps in both:
   - the existing-window toggle path
   - the new-window open path
3. The behavior works when `show_in_dock = false`.
4. The fix remains limited to launcher foregrounding and does not intentionally alter menu bar, startup, or unrelated secondary-window behavior.
5. Verification is performed against the installed `/Applications/PhotonCast.app`, not only local dev/test binaries.

## Implementation Plan
1. Isolate verification confounders first: ensure Accessibility permission is granted and stale debug/test binaries are not participating in validation.
2. Inspect both launcher branches in `handle_toggle_launcher` and align them with the explicit activation/focus pattern already used by other windows.
3. Implement the narrowest launcher-specific change first in `event_loop.rs` (and `main.rs` only if needed for the new-window branch).
4. If GPUI-level activation still does not foreground reliably in Dock-hidden mode, add a minimal launcher-specific macOS helper in `platform.rs` to explicitly foreground the app/window without revisiting startup-wide activation policy.
5. Rebuild and reinstall the release app to `/Applications/PhotonCast.app`.
6. Verify true foreground behavior manually with Cmd+Space in Dock-hidden mode for both cold/open and warm/re-toggle launcher states, and ensure no intentional regressions in Dock/menu bar/startup behavior.

## Risks / Mitigations
- **GPUI activation may be insufficient in UIElement mode** → stage a platform helper only as a fallback, not the first move.
- **Fix could accidentally broaden to all windows** → keep changes launcher-path-local unless a shared helper is truly required.
- **Background debug/test binaries can confuse verification** → verify against the installed app and clear stale test processes before manual validation.
- **macOS Accessibility/TCC can masquerade as foreground bugs** → treat Accessibility as a verification precondition and document if it blocks proof.

## Verification
- `cargo build -p photoncast -q`
- `cargo test -p photoncast --no-run`
- Diagnostics on touched launcher/activation files
- Release rebuild + local install via existing scripts
- Manual Cmd+Space validation against `/Applications/PhotonCast.app` with `show_in_dock = false`
- Confirm PhotonCast becomes frontmost and launcher is visibly key/on top in both new-window and existing-window paths

## ADR
- **Decision:** pursue a launcher-path-local activation/foreground fix, escalating to a minimal macOS launcher foreground helper only if GPUI activation alone is insufficient.
- **Drivers:** explicit user requirement for true frontmost behavior, evidence of missing activation calls in launcher toggle path, scope constraint against broad Dock/menu bar/startup changes.
- **Alternatives considered:** platform helper first; broader activation-policy redesign.
- **Why chosen:** matches the narrowest likely root cause while preserving the clarified scope.
- **Consequences:** may require one fallback iteration if UIElement mode needs lower-level AppKit foregrounding.
- **Follow-ups:** if the fallback helper is needed, document why GPUI activation was insufficient and keep it launcher-scoped.

## Available-Agent-Types Roster
- `explore`
- `planner`
- `architect`
- `critic`
- `executor`
- `verifier`
- `writer`
- `team-executor`

## Follow-up Staffing Guidance
### `$ralph`
- `executor` / high — implement the launcher foreground fix
- `verifier` / high — validate build/install/manual Cmd+Space behavior on the installed app
- `writer` / medium — refresh any concise evidence notes if needed

### `$team`
- Leader: `team-executor`
- Lane A: executor / high — launcher activation/focus implementation
- Lane B: verifier / high — release install + manual foreground validation
- Lane C: writer / medium — capture evidence/risk notes if the fix changes fallback strategy

## Launch Hints
- `$ralph .omx/plans/prd-foreground-cmd-space.md`
- `$team .omx/plans/prd-foreground-cmd-space.md`

## Team Verification Path
1. Build/test compile the touched codepaths.
2. Install the rebuilt release to `/Applications/PhotonCast.app`.
3. Validate Cmd+Space with `show_in_dock = false`.
4. Confirm PhotonCast is the frontmost app and the launcher is key/on top.
5. Confirm no intentional regressions to menu bar/startup/unrelated windows were introduced.
