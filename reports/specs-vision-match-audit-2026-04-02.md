# PhotonCast Specs/Vision Match Audit — 2026-04-02

## Executive summary

**Verdict:** PhotonCast’s **currently shipped product is now well aligned** with its mission and shipped-area specs after the follow-up doc/spec/runtime fixes applied on **2026-04-02**.

### High-level call
- **Strong alignment:** native Rust/GPUI launcher, search/frecency, file search, system commands, theming, clipboard privacy posture, calculator scope, quick links, app management, sleep timer, native extension infrastructure, and now real window overlay feedback.
- **Resolved during follow-up fixes:**
  1. README hotkey implementation detail now matches the shipped Carbon-based implementation.
  2. Window overlay feedback is now implemented in `crates/photoncast-window/src/overlay.rs`, and the README no longer overclaims beyond shipped behavior.
  3. The Phase 2 calendar spec now matches the shipped Join Meeting behavior.
  4. Mission/positioning language now separates **shipped native extensions** from **planned Raycast compatibility/store work**.
- **Remaining open areas:**
  - several hard performance claims are still more strongly asserted than directly proven by the inspected tests/benchmarks;
  - shipped startup/update-check behavior is now traced: the core update subsystem exists, but startup auto-check and menu-bar manual check are not yet wired end-to-end in the app shell;
  - overlay lifecycle polish is acceptable but could still be refined.

## Source-of-truth order used
1. `droidz/product/mission.md`
2. relevant shipped-area specs under `droidz/specs/**/spec.md`
3. `droidz/product/roadmap.md`
4. `README.md`
5. `ARCHITECTURE.md`

## Scope classification matrix

| Promise / surface | Authority source | Class | Implementation root(s) | Notes |
|---|---|---|---|---|
| Native Rust/GPUI launcher | `droidz/product/mission.md:7,68-72` | shipped | `crates/photoncast/` | Core shell is real and native. |
| Fuzzy search + frecency ranking | `README.md:7-8`, `droidz/specs/2026-01-15-phase1-mvp/spec.md:31-41` | shipped | `crates/photoncast-core/`, `crates/photoncast/` | Strongly evidenced and tested. |
| Global hotkey launcher activation | `README.md:11`, `droidz/product/roadmap.md:82-85` | shipped | `crates/photoncast/`, `crates/photoncast-core/src/platform/` | Shipped and now documented accurately. |
| Catppuccin theming + auto sync | `README.md:9`, `droidz/product/roadmap.md:48-49` | shipped | `crates/photoncast-theme/`, `crates/photoncast/` | Implemented. |
| Reduce-motion animation support | `README.md:10`, `droidz/specs/2026-01-15-phase1-mvp/spec.md:41` | shipped | `crates/photoncast/src/launcher/animation.rs`, `crates/photoncast-core/src/ui/animations.rs`, config | Implemented. |
| File search | `README.md:18`, `droidz/product/mission.md:72-75` | shipped | `crates/photoncast/`, `crates/photoncast-core/src/platform/spotlight.rs` | Implemented and tested. |
| System commands | `droidz/product/mission.md:82`, `droidz/product/roadmap.md:87-99` | shipped | `crates/photoncast-core/src/commands/` | Implemented and tested. |
| Custom commands | `droidz/product/mission.md:90`, `droidz/product/roadmap.md:252-258`, `ARCHITECTURE.md:97` | shipped | `crates/photoncast-core/src/custom_commands/`, `crates/photoncast/src/launcher/search.rs` | Implemented even though not called out in README feature list. |
| Clipboard history | `README.md:12`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:113-205` | shipped | `crates/photoncast-clipboard/` | Strong privacy-aligned implementation. |
| Calculator + units + currency + datetime | `README.md:13`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:209-297` | shipped | `crates/photoncast-calculator/` | Implemented. |
| Window management | `README.md:14`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:302-363` | shipped | `crates/photoncast-window/` | Core feature exists and now includes real overlay feedback. |
| Calendar integration | `README.md:15`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:412-468` | shipped | `crates/photoncast-calendar/`, `crates/photoncast/src/launcher/` | Implemented; spec wording now matches shipped join behavior. |
| Native extension system | `README.md:16`, `droidz/product/mission.md:88`, `droidz/product/roadmap.md:247-267` | shipped | `crates/photoncast-extension-api/`, `crates/photoncast-core/src/extensions/`, `crates/photoncast-ext-*/` | Real shipped surface. |
| Quick links | `README.md:17`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:367-409` | shipped | `crates/photoncast-quicklinks/`, `crates/photoncast/src/event_loop.rs` | Implemented. |
| App management | `README.md:19`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:472-543` | shipped | `crates/photoncast-apps/`, `crates/photoncast/src/launcher/` | Implemented. |
| Sleep timer | `README.md:20`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:545-572` | shipped | `crates/photoncast-timer/`, `crates/photoncast/` | Implemented. |
| Raycast extension compatibility | `droidz/product/mission.md:88-90`, `droidz/product/roadmap.md:282-317` | future/draft | `crates/photoncast-extension-ipc/`, `crates/photoncast-extension-runner/` | Planned / partial groundwork exists, but it is no longer presented as shipped. |
| Raycast Store integration | `droidz/product/mission.md:90`, `droidz/product/roadmap.md:346-367` | future/draft | no shipped UI/root proving it | Explicitly planned, not scored as shipped. |
| Future workflow automation / broader future ecosystem claims | `droidz/product/roadmap.md:18-19` | future/draft | n/a | Out of scope for scored findings. |

## Per-promise traceability table

| Promise | Authority | Shipped-status | Implementation evidence | Corroborating proof | Final finding |
|---|---|---|---|---|---|
| Native launcher built in Rust/GPUI | `droidz/product/mission.md:7,68-72` | shipped | `crates/photoncast/src/main.rs`, `crates/photoncast/src/launcher/mod.rs` | `ARCHITECTURE.md:1-18,56-64` | matches |
| Search with frecency | `README.md:7-8` | shipped | `crates/photoncast-core/src/search/engine.rs`, `.../search/ranking.rs`, `.../storage/usage.rs` | `tests/integration/search_test.rs`, `crates/photoncast-core/benches/search_bench.rs` | matches |
| Global hotkey activation | `README.md:11` | shipped | `crates/photoncast/src/platform.rs:300-372` | `tests/integration/hotkey_test.rs` | matches |
| Catppuccin theming + auto sync | `README.md:9`, `droidz/product/roadmap.md:48-49` | shipped | `crates/photoncast-theme/src/provider.rs`, `crates/photoncast-theme/src/lib.rs` | provider unit tests | matches |
| Reduce-motion animation support | `README.md:10`, `droidz/specs/2026-01-15-phase1-mvp/spec.md:41` | shipped | `crates/photoncast/src/launcher/animation.rs`, `crates/photoncast-core/src/ui/animations.rs`, `crates/photoncast-core/src/app/config.rs` | reduced-motion config and animation code paths | matches |
| File search | `README.md:18` | shipped | `crates/photoncast/src/file_search_helper.rs`, `crates/photoncast-core/src/platform/spotlight.rs` | `tests/integration/file_search_test.rs`, `crates/photoncast-core/tests/spotlight_integration.rs` | matches |
| System commands | `droidz/product/mission.md:82` | shipped | `crates/photoncast-core/src/commands/definitions.rs`, `.../commands/mod.rs`, `.../commands/system.rs` | `crates/photoncast-core/src/commands/tests.rs`, `tests/integration/e2e_test.rs` | matches |
| Custom commands | `droidz/product/mission.md:90`, `droidz/product/roadmap.md:252-258` | shipped | `crates/photoncast-core/src/custom_commands/mod.rs`, `.../executor.rs`, `.../store.rs`, `crates/photoncast/src/launcher/search.rs:997-1058` | placeholder/executor tests in `custom_commands/` plus provider integration in `search/providers/custom_commands.rs` | matches |
| Clipboard history privacy posture | `README.md:12`, `spec.md:171-205` | shipped | `crates/photoncast-clipboard/src/encryption.rs`, `.../storage.rs`, `.../monitor.rs`, `.../config.rs` | clipboard benches + encryption/config tests | matches |
| Calculator with currency/datetime | `README.md:13`, `spec.md:219-297` | shipped | `crates/photoncast-calculator/src/lib.rs`, `.../currency.rs`, `.../datetime.rs` | `crates/photoncast-calculator/benches/calculator_bench.rs` | matches |
| Window management with visual overlay feedback | `README.md:14` | shipped | `crates/photoncast-window/src/lib.rs`, `.../commands.rs`, `.../overlay.rs` | window crate tests plus implemented overlay code path | matches |
| Calendar join flow | `spec.md:459-462` | shipped | `crates/photoncast-calendar/src/eventkit.rs`, `.../conference.rs`, `crates/photoncast/src/launcher/render_actions.rs:11-18,121-126,188-199` | conference tests + launcher action rendering | matches |
| Native extension system | `README.md:16`, `droidz/product/roadmap.md:247-267` | shipped | `crates/photoncast-extension-api/src/lib.rs`, `crates/photoncast-core/src/extensions/`, `crates/photoncast-extension-runner/src/main.rs`, shipped ext crates | signing tests + shipped extension crates | matches |
| Quick links | `README.md:17` | shipped | `crates/photoncast-quicklinks/src/browser_import.rs`, `.../storage.rs`, `.../placeholder.rs`, `crates/photoncast/src/event_loop.rs` | storage/import/placeholder coverage in crate | matches |
| App management | `README.md:19` | shipped | `crates/photoncast-apps/src/uninstaller.rs`, `.../process.rs`, `.../auto_quit.rs`, launcher action flows | `tests/integration/app_management_test.rs` | matches |
| Sleep timer | `README.md:20` | shipped | `crates/photoncast-timer/src/parser.rs`, `.../scheduler.rs`, `.../ui.rs`, `crates/photoncast/src/main.rs:407-433` | scheduler tests | matches |
| Raycast compatibility | `droidz/product/mission.md:88-90` | future/draft | `crates/photoncast-extension-ipc/`, `crates/photoncast-extension-runner/` | roadmap still marks Phase 3 planned | future/draft |
| Raycast Store integration | `droidz/product/mission.md:90` | future/draft | no shipped store UI or install flow proven in shipped roots | roadmap Phase 3 store section is planned | future/draft |

## Per-surface findings

| Surface | Verdict | Confidence | Governing promise(s) | Implementation area(s) | Key citations | Why |
|---|---|---:|---|---|---|---|
| Launcher shell / native feel | matches | medium | Native Rust/GPUI launcher; macOS-first native experience | `crates/photoncast/`, `crates/photoncast/src/launcher/` | `droidz/product/mission.md:7,68-72`; `crates/photoncast/src/main.rs`; `crates/photoncast/src/launcher/mod.rs`; `ARCHITECTURE.md:1-18,56-64` | Native Rust/GPUI + macOS-specific integration are clearly real. |
| Search + frecency | matches | high | Fuzzy search and personalized ranking | `crates/photoncast-core/src/search/`, `crates/photoncast-core/src/storage/usage.rs` | `README.md:7-8`; `crates/photoncast-core/src/search/engine.rs`; `crates/photoncast-core/src/search/ranking.rs`; `tests/integration/search_test.rs`; `crates/photoncast-core/benches/search_bench.rs` | Strong implementation and direct test/bench evidence. |
| Global hotkey | matches | high | Global hotkey activation | `crates/photoncast/src/platform.rs`, `crates/photoncast-core/src/platform/hotkey.rs` | `README.md:11`; `droidz/product/roadmap.md:82-85`; `crates/photoncast/src/platform.rs:300-372`; `tests/integration/hotkey_test.rs` | Capability exists and the docs now describe it accurately. |
| Theming | matches | high | Catppuccin theming with auto light/dark sync | `crates/photoncast-theme/`, theme registration in `crates/photoncast/` | `README.md:9`; `droidz/product/roadmap.md:48-49`; `crates/photoncast-theme/src/provider.rs`; `crates/photoncast-theme/src/lib.rs` | Theme crate implements Catppuccin flavors and auto-sync. |
| Reduce-motion accessibility | matches | medium | Smooth animations with reduce-motion support | `crates/photoncast/src/launcher/animation.rs`, `crates/photoncast-core/src/ui/animations.rs`, config | `README.md:10`; `crates/photoncast/src/launcher/animation.rs`; `crates/photoncast-core/src/ui/animations.rs`; `crates/photoncast-core/src/app/config.rs:621-662` | Animation logic respects reduced-motion configuration. |
| File search | matches | high | Spotlight-powered live file search | `crates/photoncast/src/file_search_helper.rs`, `crates/photoncast-core/src/platform/spotlight.rs` | `README.md:18`; `droidz/product/mission.md:72-75`; `tests/integration/file_search_test.rs`; `crates/photoncast-core/tests/spotlight_integration.rs` | Spotlight-backed implementation and integration tests are present. |
| System commands | matches | high | Quick access to sleep/restart/lock/empty trash/etc. | `crates/photoncast-core/src/commands/`, integration through app layer | `droidz/product/mission.md:82`; `droidz/product/roadmap.md:87-99`; `crates/photoncast-core/src/commands/definitions.rs`; `crates/photoncast-core/src/commands/tests.rs`; `tests/integration/e2e_test.rs` | Broad built-in command surface is implemented and tested. |
| Custom commands | matches | medium | User-defined shortcuts and script execution | `crates/photoncast-core/src/custom_commands/`, launcher execution path | `droidz/product/mission.md:90`; `droidz/product/roadmap.md:252-258`; `ARCHITECTURE.md:97`; `crates/photoncast-core/src/custom_commands/mod.rs`; `crates/photoncast-core/src/custom_commands/executor.rs`; `crates/photoncast/src/launcher/search.rs:997-1058` | Real user-defined command/search/execution pipeline exists. |
| Clipboard | matches | high | Encrypted clipboard history with search | `crates/photoncast-clipboard/` | `README.md:12`; `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:113-205`; `crates/photoncast-clipboard/src/encryption.rs`; `crates/photoncast-clipboard/src/storage.rs`; clipboard benches/tests | Strongest privacy-aligned implementation in the repo. |
| Calculator | matches | high | Calculator, unit conversion, currency conversion, datetime math | `crates/photoncast-calculator/` | `README.md:13`; `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:209-297`; `crates/photoncast-calculator/src/lib.rs`; `crates/photoncast-calculator/src/currency.rs`; `crates/photoncast-calculator/benches/calculator_bench.rs` | Broad feature surface aligns with spec and has perf benches. |
| Calendar | matches | medium | Read-only EventKit calendar with conference detection / join flow | `crates/photoncast-calendar/`, `crates/photoncast/src/launcher/` | `README.md:15`; `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:412-468`; `crates/photoncast-calendar/src/eventkit.rs`; `crates/photoncast/src/launcher/render_actions.rs:11-18,121-126,188-199` | Core feature exists and the spec now matches the shipped Join Meeting behavior. |
| Quick links | matches | high | Quick links with placeholders/import | `crates/photoncast-quicklinks/`, `crates/photoncast/src/event_loop.rs` | `README.md:17`; `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:367-409`; `crates/photoncast-quicklinks/src/browser_import.rs`; `.../placeholder.rs`; `.../storage.rs` | Import, placeholders, management flows, and storage all exist. |
| App management | matches | medium | Uninstall / quit / force quit / auto-quit | `crates/photoncast-apps/`, launcher action flows | `README.md:19`; `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:472-543`; `crates/photoncast-apps/src/uninstaller.rs`; `.../process.rs`; `tests/integration/app_management_test.rs` | Uninstall/quit/auto-quit flows are real and tested. |
| Sleep timer | matches | medium | Configurable timer with system actions | `crates/photoncast-timer/`, polling in `crates/photoncast/` | `README.md:20`; `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:545-572`; `crates/photoncast-timer/src/parser.rs`; `.../scheduler.rs`; `.../ui.rs` | Scheduling/persistence/parser/UI are present; some secondary UX is less proven. |
| Window management | matches | medium | Layouts, cycling, multi-display support, visual overlay feedback | `crates/photoncast-window/` | `README.md:14`; `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:302-363`; `crates/photoncast-window/src/commands.rs`; `.../accessibility.rs`; `.../overlay.rs` | Core layouts exist and visual overlay feedback is now implemented. |
| Native extension system | matches | medium | ABI-stable native extension system with code signing | `crates/photoncast-extension-api/`, `crates/photoncast-core/src/extensions/`, `crates/photoncast-ext-*/` | `README.md:16`; `droidz/product/roadmap.md:247-267`; `crates/photoncast-extension-api/src/lib.rs`; `crates/photoncast-core/src/extensions/manager.rs`; signing tests; shipped extension crates | ABI-stable, signed, permissioned host/extension system is real. |

## Cross-cutting principle synthesis

### 1. Speed is a feature
**Mostly aligned, but the proof is uneven.**
- Strongest evidence: search performance has explicit targets and direct test/bench coverage: `tests/integration/search_test.rs` checks `<30ms`, and `crates/photoncast-core/benches/search_bench.rs` encodes the same target.
- Weaker evidence: mission/roadmap claims like **sub-50ms activation**, **<50ms hotkey response**, and **120 FPS** are not equally proven by the inspected evidence. That makes these claims **unknown/unverified**, not clearly false.

### 2. Privacy by default
**Mostly aligned, with important nuance.**
- Strongest alignment comes from clipboard: AES-256-GCM, keychain-backed salt, excluded password-manager apps, transient-pasteboard skips, and local storage.
- I found **no telemetry/account/cloud-sync surface** in the shipped core product.
- Nuance: the codebase does include legitimate network features — e.g. calculator currency refresh via `frankfurter.app` / `coingecko` and an update manager with an HTTP feed in `crates/photoncast-core/src/platform/updates.rs`. These are **not telemetry**, but they do mean the privacy story is best phrased as **no telemetry / no cloud account dependence**, not “literally zero network traffic under all conditions.”

### 3. Simplicity over features / no AI posture
**Aligned.**
- I found no shipped AI/LLM surface.
- The product is broad, but the shipped breadth is still coherent with launcher/productivity use cases rather than obvious gimmick creep.
- The main simplicity risk is now more about future scope discipline than current messaging; the most misleading Raycast/store overclaims were reduced in the follow-up doc pass.

### 4. Reliability / native feel
**Aligned overall.**
- The app leans heavily on macOS-native APIs: Spotlight, EventKit, Carbon hotkeys, accessibility APIs, Keychain.
- Several modules include fallbacks or defensive behavior.
- The main remaining reliability questions are no longer obvious feature gaps, but proof/observability depth and small lifecycle polish details such as the retained overlay window handle.

### 5. Sustainable simplicity / maintainability
**Moderately aligned.**
- The crate split is a strength: `photoncast-core` foundation plus separate feature crates is consistent with the architecture document.
- Test and bench presence is materially better than average for a launcher project.
- The biggest maintainability risk is now reduced but not gone: the recent doc/spec refresh improved alignment substantially, but future changes should keep shipped/planned labels synchronized across README, mission, roadmap, and specs.

## Claimed-but-unshipped appendix

After the follow-up doc cleanup, there are **no major top-level shipped overclaims left in the audited documents**.

What remains is explicitly labeled as planned/future work rather than shipped functionality:

1. **Raycast extension compatibility** — planned path, not current shipped capability
2. **Raycast Store integration** — planned path, not current shipped capability
3. **Phase 3 roadmap compatibility/store milestones** — still planned as roadmap work

## Unknown / unverified items

1. **Hard performance claims**: sub-50ms activation, <50ms hotkey response, 120 FPS. Search is directly evidenced; the rest were not equivalently proven in the inspected artifacts.
2. **Exact shipped breadth of update behavior**: the core update subsystem is implemented, but app-shell startup auto-check and menu-bar manual check are still not wired end-to-end.
3. **Overlay lifecycle polish**: the runtime behavior is correct and approved, but the last hidden overlay can remain retained until the next overlay or explicit close.

## Bottom line

If I judge **mission-first** and **shipped-only**, PhotonCast is now a **stronger and more honest match** for its current vision than it was at the start of this audit cycle.

The most important corrections have already been applied:
- misleading doc claims were tightened,
- planned extension/store work is now labeled as planned,
- calendar spec wording now matches shipped behavior, and
- real window overlay feedback is now implemented.

The main remaining issues are no longer obvious shipped-product drifts; they are mostly about **evidence depth** for some performance/startup claims and a few smaller polish/observability questions.
