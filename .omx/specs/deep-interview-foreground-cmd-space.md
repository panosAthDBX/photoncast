# Deep Interview Spec — foreground-cmd-space

## Metadata
- Profile: standard
- Rounds: 3
- Final ambiguity: 13%
- Threshold: 20%
- Context type: brownfield
- Context snapshot: `.omx/context/foreground-cmd-space-20260403T164336Z.md`

## Clarity Breakdown
| Dimension | Score |
|---|---:|
| Intent | 0.86 |
| Outcome | 0.95 |
| Scope | 0.90 |
| Constraints | 0.82 |
| Success Criteria | 0.88 |
| Brownfield Context | 0.92 |

## Intent
Restore PhotonCast launcher behavior so Cmd+Space works like a true launcher invocation again: PhotonCast should come to the foreground, not merely become active in the background.

## Desired Outcome
Whenever Cmd+Space triggers the launcher, PhotonCast becomes the true frontmost app and the launcher window is key/focused and visibly on top of all normal apps. This must hold even when `Show in Dock` is off.

## In Scope
- Cmd+Space / launcher foregrounding behavior
- Activation/focus/order-front behavior on launcher toggle/open
- Any minimal launcher-only activation-policy or window-presentation adjustments strictly required to make Cmd+Space foreground correctly

## Out of Scope / Non-goals
- Dock visibility behavior beyond what is strictly required for Cmd+Space launcher foregrounding
- Menu bar visibility behavior
- Startup flow changes
- Secondary windows (preferences, clipboard, quicklinks, etc.) unless strictly required for the launcher fix

## Decision Boundaries
- OMX may change launcher activation/foregrounding semantics so Cmd+Space always brings PhotonCast truly frontmost, even in dock-hidden mode
- OMX should not broaden the fix into unrelated Dock/menu bar/startup behavior without a new user decision

## Constraints
- Preserve the user’s desired Dock-hidden mode as a product capability
- Fix must remain launcher-specific and avoid broad regressions
- Brownfield evidence indicates the current existing-window toggle path lacks explicit app/window activation calls, while other windows use them

## Testable Acceptance Criteria
1. Pressing Cmd+Space makes PhotonCast the frontmost app every time.
2. The launcher window becomes key/focused and visibly appears on top of all normal apps.
3. This works even when `show_in_dock = false`.
4. The fix does not intentionally alter menu bar behavior, startup behavior, or unrelated secondary window behavior.

## Assumptions Exposed + Resolutions
- Assumption: Dock-hidden mode might intentionally avoid full foreground behavior.
  - Resolution: Rejected; user explicitly wants true foreground behavior even when Show in Dock is off.
- Assumption: Mere activation without visible frontmost/key window behavior might be acceptable.
  - Resolution: Rejected; user requires both frontmost app state and visibly top/key launcher window.

## Pressure-pass Findings
A pressure pass revisited the first answer by forcing a concrete success definition. The clarified requirement tightened from “foreground app” to “foreground app plus key/focused launcher window visibly on top of all normal apps.”

## Brownfield Evidence vs Inference
- Evidence: `AppEvent::ToggleLauncher` is sent from `crates/photoncast/src/main.rs`.
- Evidence: Existing launcher toggle handling in `crates/photoncast/src/event_loop.rs` only calls `view.toggle(cx)` and `cx.focus_self()` and does not call `cx.activate(true)` / `cx.activate_window()`.
- Evidence: Launcher windows are opened in `crates/photoncast/src/main.rs` with `focus: true`, `show: true`, and `WindowKind::Normal`.
- Evidence: Recent local changes added activation-policy syncing at startup in `crates/photoncast/src/platform.rs`.
- Inference: The regression likely sits in launcher-specific app/window activation semantics rather than hotkey dispatch itself.

## Technical Context Findings
- Hotkey path: `main.rs` -> `AppEvent::ToggleLauncher`
- Event handling: `event_loop.rs`
- Launcher view visibility logic: `launcher/mod.rs`
- Activation-policy handling: `platform.rs`

## Recommended Handoff
- `$ralplan .omx/specs/deep-interview-foreground-cmd-space.md`
- `$ralph .omx/specs/deep-interview-foreground-cmd-space.md` if you want immediate persistent execution
