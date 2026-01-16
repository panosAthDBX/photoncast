# Implementation: 3.2 Global Hotkey Registration

**Specialist:** backend-specialist  
**Phase:** 9 (Hotkey)  
**Dependencies:** 3.1.1 (Permission check)

---

## Task Assignment

### 3.2 Global Hotkey Registration

- [ ] **3.2.1 Create HotkeyBinding type** (1h)
  - Define `HotkeyBinding { key: KeyCode, modifiers: Modifiers }`
  - Default: `Cmd+Space`
  - Implement `Default` trait
  - Dependencies: 1.1.4

- [ ] **3.2.2 Implement CGEventTap hotkey handler** (4h) ⭐ CRITICAL
  - Create `src/platform/hotkey.rs`
  - Create event tap for `KeyDown` and `FlagsChanged` events
  - Match events against registered binding
  - Consume matched events (return None)
  - Dependencies: 3.2.1

- [ ] **3.2.3 Create HotkeyManager** (3h) ⭐ CRITICAL
  - Implement `register()`, `unregister()` methods
  - Check accessibility permission before registration
  - Enable tap and add to CFRunLoop
  - Store registration state
  - Dependencies: 3.2.2, 3.1.1

- [ ] **3.2.4 Wire hotkey to window toggle** (2h)
  - On hotkey press, call `launcher.toggle()`
  - Ensure thread-safe communication to main thread
  - Target: <50ms hotkey response
  - Dependencies: 3.2.3, 1.2.3

- [ ] **3.2.5 Create HotkeyError type** (1h)
  - Define variants: `PermissionDenied`, `ConflictDetected`, `RegistrationFailed`, `InvalidBinding`
  - Implement user-friendly messages with action hints
  - Dependencies: 3.2.3

- [ ] **3.2.6 Write hotkey integration tests** (2h)
  - Test registration succeeds with permission
  - Test registration fails without permission
  - Test event matching
  - Dependencies: 3.2.3

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.1: Global Hotkey Registration)
- **Platform Standard:** `droidz/standards/backend/platform.md`

---

## Instructions

1. Read spec.md Section 5.1 for CGEventTap implementation
2. Use `core-graphics` crate for CGEventTap
3. Create `src/platform/hotkey.rs`
4. Target <50ms hotkey response time
5. Test registration with and without permission
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use CGEventTap for global hotkey capture
- Check accessibility permission before registration
- Consume matching events (return None)
- Target: <50ms hotkey response
