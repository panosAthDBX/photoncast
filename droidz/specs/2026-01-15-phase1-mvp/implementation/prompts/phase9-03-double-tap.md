# Implementation: 3.4 Double-Tap Modifier Support

**Specialist:** backend-specialist  
**Phase:** 9 (Hotkey)  
**Dependencies:** 3.2.2 (CGEventTap handler)

---

## Task Assignment

### 3.4 Double-Tap Modifier Support

- [ ] **3.4.1 Implement DoubleTapDetector** (3h)
  - Create struct tracking `last_modifier_press: Option<Instant>`
  - Configure threshold (300ms default)
  - Track target modifier (e.g., Command)
  - Dependencies: 3.2.2

- [ ] **3.4.2 Implement modifier event handling** (2h)
  - On modifier press: record timestamp
  - On second press within threshold: trigger
  - Reset state after trigger or timeout
  - Dependencies: 3.4.1

- [ ] **3.4.3 Add double-tap to HotkeyManager** (2h)
  - Support `double_tap_modifier: Option<Modifier>` config
  - Wire to existing hotkey callback
  - Dependencies: 3.4.1, 3.2.3

- [ ] **3.4.4 Write double-tap tests** (1h)
  - Test detection within threshold
  - Test no detection outside threshold
  - Dependencies: 3.4.1, 3.4.2

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.1: Double-Tap Support)

---

## Instructions

1. Read spec.md for double-tap implementation
2. Use 300ms threshold for detection
3. Track modifier press timestamps
4. Reset state after trigger or timeout
5. Mark tasks complete in tasks.md

---

## Key Standards

- 300ms default threshold
- Track modifier key press timestamps
- Reset after trigger or timeout
