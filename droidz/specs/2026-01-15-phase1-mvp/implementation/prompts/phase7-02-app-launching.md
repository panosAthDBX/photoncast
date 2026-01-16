# Implementation: 2.6 App Launching

**Specialist:** backend-specialist  
**Phase:** 7 (Search Advanced - Can run parallel with 2.5)  
**Dependencies:** 2.2.5 (Database operations), 1.1.4 (Core dependencies)

---

## Task Assignment

### 2.6 App Launching

- [ ] **2.6.1 Implement NSWorkspace launcher** (3h) ⭐ CRITICAL
  - Create `src/platform/launch.rs`
  - Use `objc2-app-kit` for NSWorkspace access
  - Implement `launch_app(bundle_id: &str) -> Result<()>`
  - Handle app not found, damaged app errors
  - Dependencies: 1.1.4

- [ ] **2.6.2 Create LaunchError type** (1h)
  - Define error variants: `NotFound`, `LaunchFailed`, `Damaged`
  - Implement user-friendly error messages
  - Implement `Display` trait
  - Dependencies: 2.6.1

- [ ] **2.6.3 Implement usage tracking** (2h)
  - On successful launch, increment `launch_count`
  - Update `last_launched_at` timestamp
  - Use database operations from 2.2.5
  - Dependencies: 2.6.1, 2.2.5

- [ ] **2.6.4 Wire launch to activation** (2h)
  - Connect `Activate` action to launch handler
  - Match `SearchAction::LaunchApp` variant
  - Close launcher window after launch
  - Dependencies: 2.6.1, 1.6.3

- [ ] **2.6.5 Handle launch errors gracefully** (2h)
  - Show toast notification for errors
  - Remove not-found apps from index
  - Offer "Reveal in Finder" for damaged apps
  - Dependencies: 2.6.2, 1.5.3

- [ ] **2.6.6 Write launch tests** (1h)
  - Test successful launch (mock NSWorkspace)
  - Test error handling for missing apps
  - Dependencies: 2.6.1, 2.6.2

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.3: System Commands)
- **Platform Standard:** `droidz/standards/backend/platform.md`

---

## Instructions

1. Read `droidz/standards/backend/platform.md` for macOS FFI patterns
2. Use `objc2` for safe Objective-C interop
3. Create `src/platform/launch.rs`
4. Handle all launch error cases
5. Update usage stats on successful launch
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use `objc2`/`objc2-app-kit` for safe FFI
- Handle all NSWorkspace error cases
- Update frecency on every launch
- Close launcher after successful activation
