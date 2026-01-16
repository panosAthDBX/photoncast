# Implementation: 3.5 Hotkey Customization

**Specialist:** full-stack-specialist  
**Phase:** 10 (Settings)  
**Dependencies:** 3.2.3 (HotkeyManager), 2.2.2 (Config schema)

---

## Task Assignment

### 3.5 Hotkey Customization

- [ ] **3.5.1 Add hotkey to config file** (1h)
  - Add `[hotkey]` section to config schema
  - Support `key` and `modifiers` fields
  - Support `double_tap_modifier` optional field
  - Dependencies: 2.2.2

- [ ] **3.5.2 Implement hotkey settings UI** (3h)
  - Create settings panel for hotkey configuration
  - Show current binding
  - Allow key capture for new binding
  - Dependencies: 1.3.3, 3.5.1

- [ ] **3.5.3 Implement key capture** (2h)
  - Listen for next keypress in capture mode
  - Validate binding (no single modifier, no reserved keys)
  - Dependencies: 3.5.2

- [ ] **3.5.4 Implement hotkey change with re-registration** (2h)
  - Unregister old hotkey
  - Validate new binding
  - Register new hotkey
  - Persist to config file
  - Dependencies: 3.5.2, 3.2.3

- [ ] **3.5.5 Write hotkey settings tests** (1h)
  - Test config parsing
  - Test binding validation
  - Dependencies: 3.5.1, 3.5.3

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 6.3: Configuration Types)

---

## Instructions

1. Add hotkey config section
2. Create settings UI with key capture
3. Validate new bindings before applying
4. Re-register hotkey after change
5. Persist to config file
6. Mark tasks complete in tasks.md

---

## Key Standards

- Validate binding before registration
- Unregister old before registering new
- Persist changes atomically
