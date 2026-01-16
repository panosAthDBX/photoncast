# Implementation: 1.1 Project Setup & Infrastructure

**Specialist:** devops-specialist  
**Phase:** 1 (Foundation - MUST RUN FIRST)  
**Dependencies:** None

---

## Task Assignment

### 1.1 Project Setup & Infrastructure

- [ ] **1.1.1 Initialize Cargo workspace** (2h) ⭐ CRITICAL
  - Create `Cargo.toml` with workspace configuration
  - Set up `photoncast` binary crate and `photoncast-core` library crate
  - Configure MSRV 1.75+, Rust 2021 edition
  - Add `rust-toolchain.toml` for version pinning
  - Dependencies: None

- [ ] **1.1.2 Configure linting and formatting** (1h)
  - Add `rustfmt.toml` with project formatting rules
  - Configure `clippy.toml` with pedantic + nursery lints
  - Add `.editorconfig` for editor consistency
  - Dependencies: 1.1.1

- [ ] **1.1.3 Set up GitHub Actions CI pipeline** (2h) ⭐ CRITICAL
  - Create `.github/workflows/ci.yml`
  - Add jobs: `cargo fmt --check`, `cargo clippy`, `cargo test`
  - Configure macOS runner for platform-specific tests
  - Add caching for Cargo dependencies
  - Dependencies: 1.1.1, 1.1.2

- [ ] **1.1.4 Add core dependencies to Cargo.toml** (1h) ⭐ CRITICAL
  - Add GPUI (`gpui` crate from Zed)
  - Add `gpui-component` for UI components
  - Add `tokio` for async runtime
  - Add `thiserror` + `anyhow` for error handling
  - Add `tracing` + `tracing-subscriber` for logging
  - Add `serde` + `toml` for configuration
  - Dependencies: 1.1.1

- [ ] **1.1.5 Create module structure** (1h)
  - Set up `src/` directory structure per spec (app, ui, search, etc.)
  - Create `mod.rs` files with proper re-exports
  - Add placeholder modules for each component
  - Dependencies: 1.1.1

- [ ] **1.1.6 Set up integration test infrastructure** (1h)
  - Create `tests/` directory structure
  - Add `tests/common/mod.rs` for shared test utilities
  - Configure `proptest` for property-based testing
  - Dependencies: 1.1.1, 1.1.4

---

## Context Files

Read these for requirements and patterns:
- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 2: Architecture)
- **Requirements:** `droidz/specs/2026-01-15-phase1-mvp/requirements.md`
- **Tech Stack:** `droidz/standards/global/tech-stack.md`
- **Coding Style:** `droidz/standards/global/coding-style.md`

---

## Instructions

1. Read spec.md Section 2 (Architecture) for module structure
2. Initialize Cargo workspace at project root `/Users/panos.athanasiou/code/photoncast/`
3. Follow tech-stack.md for exact dependencies and versions
4. Run `cargo build` to verify setup compiles
5. Run `cargo fmt` and `cargo clippy` to verify CI checks pass
6. Mark tasks complete with [x] in `droidz/specs/2026-01-15-phase1-mvp/tasks.md`

---

## Key Standards

- Use Rust 2021 edition, MSRV 1.75+
- Follow `crate-first.md` - search crates.io before implementing
- Use `thiserror` for error types, `anyhow` for application errors
- Configure clippy with pedantic + nursery lints
