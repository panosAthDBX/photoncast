# Implementation: 3.1 Accessibility Permissions

**Specialist:** backend-specialist  
**Phase:** 8 (Platform Foundation)  
**Dependencies:** 1.1.4 (Core dependencies)

---

## Task Assignment

### 3.1 Accessibility Permissions

- [ ] **3.1.1 Implement permission status check** (2h) ⭐ CRITICAL
  - Create `src/platform/accessibility.rs`
  - Call `AXIsProcessTrusted()` from accessibility_sys
  - Return `PermissionStatus` enum: Granted, Denied, Unknown
  - Dependencies: 1.1.4

- [ ] **3.1.2 Implement permission request with prompt** (2h)
  - Call `AXIsProcessTrustedWithOptions` with prompt flag
  - Trigger macOS permission dialog
  - Dependencies: 3.1.1

- [ ] **3.1.3 Create PermissionDialog UI component** (3h) ⭐ CRITICAL
  - Create `src/ui/permission_dialog.rs`
  - Display explanation of why permission is needed
  - Bullet points: "Register global shortcuts", "Respond to hotkey"
  - Add "Open System Settings" and "Skip for Now" buttons
  - Dependencies: 1.3.3, 3.1.1

- [ ] **3.1.4 Implement "Open System Settings" action** (1h)
  - Open Privacy & Security → Accessibility pane
  - Use `open` command with URL scheme
  - Dependencies: 3.1.3

- [ ] **3.1.5 Implement real-time permission polling** (2h)
  - Poll permission status every 1 second
  - When granted, show success toast and dismiss dialog
  - Stop polling when granted or dialog dismissed
  - Dependencies: 3.1.1, 3.1.3

- [ ] **3.1.6 Wire permission flow to app startup** (2h)
  - Check permission on launch
  - If not granted, show dialog before hotkey registration
  - Allow launcher to work from menu bar without permission
  - Dependencies: 3.1.1, 3.1.3

- [ ] **3.1.7 Write permission tests** (1h)
  - Test status check function
  - Test dialog rendering
  - Dependencies: 3.1.1, 3.1.3

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.2: Accessibility Permission Flow)
- **Platform Standard:** `droidz/standards/backend/platform.md`

---

## Instructions

1. Read spec.md Section 5.2 for permission flow
2. Use `accessibility-sys` crate for AX* APIs
3. Create guided permission dialog
4. Poll every 1s to detect when permission granted
5. Test full permission flow
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use `accessibility-sys` crate
- Poll permission status every 1 second
- Provide clear explanation to user
- Fallback: menu bar activation without permission
