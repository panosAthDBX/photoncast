# User Testing

## Validation Surface

PhotonCast is a native macOS desktop application. There is **no web UI, no API, and no CLI** to test against remotely.

**Primary testing surface:** Automated checks only
- `cargo test --workspace` — unit and integration tests
- `cargo clippy --workspace -- -D warnings` — lint
- `cargo fmt --check` — formatting
- `rg` (ripgrep) — code pattern verification

**No interactive testing surface** — the app requires a macOS display, Accessibility permissions, and manual interaction. All validation for this mission is through automated compilation, test, and grep checks.

## Validation Concurrency

All validation for this mission is automated (cargo commands + ripgrep). These are lightweight and can run serially. Max concurrent validators: **1** (no parallelism needed since there's no interactive testing surface).
