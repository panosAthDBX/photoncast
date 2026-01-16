# Implementation: 1.2 GPUI Integration

**Specialist:** frontend-specialist  
**Phase:** 2 (Core Setup)  
**Dependencies:** Phase 1 complete (1.1 Project Setup)

---

## Task Assignment

### 1.2 GPUI Integration

- [ ] **1.2.1 Create GPUI application bootstrap** (3h) ⭐ CRITICAL
  - Implement `main.rs` with GPUI application initialization
  - Set up `App` and run loop
  - Configure 120 FPS rendering target
  - Add graceful shutdown handling
  - Dependencies: 1.1.4

- [ ] **1.2.2 Create main launcher window** (3h) ⭐ CRITICAL
  - Implement `LauncherWindow` struct with `Render` trait
  - Set fixed width (680px), dynamic height (72-500px)
  - Configure border radius (12px), shadow, centered-top position
  - Handle multi-monitor cursor-based positioning
  - Dependencies: 1.2.1

- [ ] **1.2.3 Implement window show/hide logic** (2h)
  - Add `toggle()`, `show()`, `hide()` methods
  - Configure window as panel (no dock icon by default)
  - Set up window focus handling
  - Target: <50ms window appear time
  - Dependencies: 1.2.2

- [ ] **1.2.4 Register GPUI actions** (2h)
  - Define actions in `src/app/actions.rs` using `actions!` macro
  - Register `SelectNext`, `SelectPrevious`, `Activate`, `Cancel`
  - Add `QuickSelect1-9` actions for ⌘1-9 shortcuts
  - Dependencies: 1.2.1

- [ ] **1.2.5 Configure key bindings** (1h)
  - Set up `↑/↓` for navigation, `Enter` for activate, `Esc` for cancel
  - Add `Ctrl+N/P` alternatives for navigation
  - Add `⌘1-9` for quick selection
  - Add `Tab` for group cycling
  - Dependencies: 1.2.4

- [ ] **1.2.6 Write GPUI integration tests** (2h)
  - Test window creation and rendering
  - Test action dispatch and key binding
  - Verify 120 FPS baseline rendering
  - Dependencies: 1.2.2, 1.2.4

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 3: UI/UX Specification)
- **Components Standard:** `droidz/standards/frontend/components.md`
- **Tech Stack:** `droidz/standards/global/tech-stack.md`

---

## Instructions

1. Read spec.md Section 3 for window dimensions and layout
2. Study `droidz/standards/frontend/components.md` for GPUI patterns
3. Create `src/ui/launcher.rs` with `Render` trait implementation
4. Create `src/app/actions.rs` with GPUI actions
5. Run `cargo build` and `cargo test` to verify
6. Mark tasks complete with [x] in tasks.md

---

## Key Standards

- Use GPUI `Render` trait for components
- Use `actions!` macro for action definitions
- Call `cx.notify()` after state changes
- Target 120 FPS, measure with profiling
