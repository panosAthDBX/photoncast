# Implementation: 3.6 System Commands

**Specialist:** backend-specialist  
**Phase:** 8 (Platform Foundation - Can run parallel with 3.1)  
**Dependencies:** 1.1.4 (Core dependencies)

---

## Task Assignment

### 3.6 System Commands

- [ ] **3.6.1 Define SystemCommand enum** (2h) ⭐ CRITICAL
  - Create `src/commands/definitions.rs`
  - Variants: Sleep, SleepDisplays, LockScreen, Restart, ShutDown, LogOut, EmptyTrash, ScreenSaver
  - Add metadata: name, aliases, description, icon, requires_confirmation
  - Dependencies: 1.1.4

- [ ] **3.6.2 Implement Sleep command** (1h)
  - Execute `pmset sleepnow`
  - No confirmation required
  - Dependencies: 3.6.1

- [ ] **3.6.3 Implement SleepDisplays command** (1h)
  - Execute `pmset displaysleepnow`
  - No confirmation required
  - Dependencies: 3.6.1

- [ ] **3.6.4 Implement LockScreen command** (1h)
  - Use Quartz Display Services `CGSession::lock()`
  - No confirmation required
  - Dependencies: 3.6.1

- [ ] **3.6.5 Implement Restart/ShutDown/LogOut commands** (2h)
  - Use AppleScript via `osascript`
  - Require confirmation dialog
  - Dependencies: 3.6.1

- [ ] **3.6.6 Implement EmptyTrash command** (1h)
  - Use Finder AppleScript
  - Require confirmation dialog
  - Dependencies: 3.6.1

- [ ] **3.6.7 Implement ScreenSaver command** (1h)
  - Execute `open -a ScreenSaverEngine`
  - No confirmation required
  - Dependencies: 3.6.1

- [ ] **3.6.8 Create confirmation dialog** (2h)
  - Show dialog for destructive commands
  - Include action name and description
  - Cancel and Confirm buttons
  - Dependencies: 1.3.3

- [ ] **3.6.9 Create AppleScript executor** (2h)
  - Implement `run_applescript(script) -> Result<()>`
  - Handle script errors
  - Log execution at debug level
  - Dependencies: 3.6.5

- [ ] **3.6.10 Create CommandError type** (1h)
  - Define variants: `ExecutionFailed`, `AuthorizationRequired`, `NotAvailable`
  - Implement user-friendly messages
  - Dependencies: 3.6.1

- [ ] **3.6.11 Write command unit tests** (2h)
  - Test command metadata
  - Test execution (mock system calls)
  - Dependencies: 3.6.1

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.3: System Commands)
- **Built-in Commands:** `droidz/standards/backend/builtin-commands.md`

---

## Instructions

1. Read spec.md Section 5.3 for command implementations
2. Read `builtin-commands.md` for full command reference
3. Create `src/commands/` module
4. Implement all 7 commands
5. Add confirmation dialog for destructive commands
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use AppleScript via osascript for system events
- Confirmation required for: Restart, ShutDown, LogOut, EmptyTrash
- Use `pmset` for sleep commands
- Handle all execution errors gracefully
