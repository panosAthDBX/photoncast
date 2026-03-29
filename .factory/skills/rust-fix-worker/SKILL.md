---
name: rust-fix-worker
description: Rust code quality fix worker — fixes bugs, removes unwraps, improves error handling, stabilizes tests, and refactors code in the PhotonCast codebase.
---

# Rust Fix Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Use for features that involve:
- Fixing code quality issues (unwraps, error handling, clippy warnings)
- Refactoring patterns (async migration, process management)
- Stabilizing flaky tests
- Implementing small missing functionality (accessibility checks, regex optimization)
- TODO cleanup and documentation

## Required Skills

None — all work is done through file editing, cargo commands, and ripgrep verification.

## Work Procedure

1. **Read the feature description carefully.** Understand what files need to change and what the expected behavior is. Read `AGENTS.md` for coding conventions and boundaries.

2. **Read the affected files.** Before making any changes, read the full context of the files you'll modify. Understand the surrounding code patterns, imports, and conventions.

3. **Write failing tests first (TDD red phase).** For each behavioral change:
   - Write a test that demonstrates the current buggy/missing behavior
   - Run `cargo test -p <crate>` to confirm it fails (or add a new test for new behavior)
   - If the fix is purely structural (e.g., removing unwrap, changing signature), write a test that verifies the new pattern works

4. **Implement the fix (TDD green phase).** Make the minimal code change to fix the issue:
   - Follow existing code style and patterns in the file
   - Use `parking_lot` if already a dependency, `std::sync` otherwise
   - Use `tracing::warn!` or `tracing::error!` for error logging (never `println!`)
   - Prefer `?` operator over `.unwrap()` in library code
   - When converting `block_on` to async, use `cx.background_executor().spawn()` or `cx.spawn()` for GPUI contexts, `tokio::spawn` for pure async

5. **Run verification commands:**
   - `cargo test -p <affected_crate>` — all tests pass
   - `cargo clippy -p <affected_crate> -- -D warnings` — no warnings
   - `cargo fmt --check` — properly formatted

6. **Verify with grep patterns.** For each assertion in the feature's `fulfills`, run the grep check described in the validation contract to confirm the fix is complete.

7. **Run full workspace checks before finishing:**
   - `cargo test --workspace -- --test-threads=6` — all tests pass
   - `cargo clippy --workspace -- -D warnings` — zero warnings

## Example Handoff

```json
{
  "salientSummary": "Removed 5 block_on() calls from event_loop.rs, replacing them with cx.background_executor().spawn() + callback pattern for async quicklinks loading. All 1428+ tests pass, clippy clean.",
  "whatWasImplemented": "Converted synchronous quicklinks loading in event_loop.rs to async pattern using GPUI background executor. Added load_quicklinks_async() helper that spawns the SQLite read on a background thread and updates the view state via cx.notify() callback. Updated all 5 call sites (lines 201, 221, 370, 407, 457) to use the new async pattern.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      { "command": "cargo test --workspace -- --test-threads=6", "exitCode": 0, "observation": "1432 passed, 0 failed, 39 ignored" },
      { "command": "cargo clippy --workspace -- -D warnings", "exitCode": 0, "observation": "No warnings" },
      { "command": "rg 'block_on.*quicklinks' crates/photoncast/src/event_loop.rs", "exitCode": 1, "observation": "No matches — all block_on calls removed" }
    ],
    "interactiveChecks": []
  },
  "tests": {
    "added": [
      { "file": "crates/photoncast/src/event_loop.rs", "cases": [
        { "name": "test_quicklinks_load_does_not_block", "verifies": "Loading quicklinks uses async dispatch, not block_on" }
      ]}
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The fix requires changing a public API that other crates depend on and you're unsure about compatibility
- A test failure reveals a deeper architectural issue beyond the scope of the feature
- The code pattern is significantly different from what the feature description expected
- You need to add a new crate dependency (not allowed without orchestrator approval)
