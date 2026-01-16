# Implementation: 3.3 Hotkey Conflict Detection

**Specialist:** backend-specialist  
**Phase:** 9 (Hotkey - Can run parallel with 3.2)  
**Dependencies:** 1.1.4 (Core dependencies)

---

## Task Assignment

### 3.3 Hotkey Conflict Detection

- [ ] **3.3.1 Read Spotlight shortcut status** (2h)
  - Read `~/Library/Preferences/com.apple.symbolichotkeys.plist`
  - Check key 64 (Spotlight) enabled status
  - Parse plist with `plist` crate
  - Dependencies: 1.1.4

- [ ] **3.3.2 Implement conflict detection** (2h) ⭐ CRITICAL
  - Create `detect_hotkey_conflict(binding) -> Option<String>`
  - Check Spotlight (Cmd+Space)
  - Return conflicting app name if found
  - Dependencies: 3.3.1

- [ ] **3.3.3 Handle conflicts in registration** (2h)
  - Return `HotkeyError::ConflictDetected` with app name
  - Show user-friendly conflict message
  - Suggest changing PhotonCast hotkey
  - Dependencies: 3.3.2, 3.2.5

- [ ] **3.3.4 Write conflict detection tests** (1h)
  - Test Spotlight detection
  - Test conflict error creation
  - Dependencies: 3.3.1, 3.3.2

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.1: Conflict Detection)

---

## Instructions

1. Read spec.md for conflict detection implementation
2. Parse com.apple.symbolichotkeys.plist for Spotlight
3. Detect Cmd+Space conflict with Spotlight
4. Provide clear error message to user
5. Mark tasks complete in tasks.md

---

## Key Standards

- Read Spotlight status from system plist
- Detect conflict before registration attempt
- Provide actionable error message
