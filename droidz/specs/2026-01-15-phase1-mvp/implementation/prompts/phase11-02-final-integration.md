# Implementation: 3.10 Final Integration & Polish

**Specialist:** full-stack-specialist  
**Phase:** 11 (Integration - FINAL)  
**Dependencies:** All previous phases complete

---

## Task Assignment

### 3.10 Final Integration & Polish

- [ ] **3.10.1 Register all search providers** (1h)
  - Add AppProvider, CommandProvider, FileProvider to SearchEngine
  - Configure provider priorities
  - Dependencies: 2.4.4, 3.7.1, 3.8.4

- [ ] **3.10.2 Implement search timeout handling** (1h)
  - Return partial results if timeout exceeded
  - Show "Search took too long" toast
  - Dependencies: 2.4.5

- [ ] **3.10.3 Add menu bar icon** (2h)
  - Create status item in menu bar
  - Show PhotonCast icon
  - Click to toggle launcher
  - Dependencies: 1.2.3

- [ ] **3.10.4 Implement launch at login** (2h)
  - Use `SMAppService` for login item registration
  - Add toggle in settings
  - Dependencies: 3.5.2

- [ ] **3.10.5 Add preferences shortcut** (1h)
  - ⌘, opens preferences window
  - Dependencies: 3.5.2, 1.2.4

- [ ] **3.10.6 Create config file loading** (2h)
  - Load from `~/.config/photoncast/config.toml`
  - Create default config if missing
  - Dependencies: 2.2.2

- [ ] **3.10.7 Implement config file saving** (1h)
  - Save settings changes to config file
  - Use atomic write
  - Dependencies: 3.10.6

- [ ] **3.10.8 Final performance profiling** (2h)
  - Profile cold start time
  - Profile hotkey response time
  - Profile search latency
  - Identify and fix bottlenecks
  - Dependencies: All

- [ ] **3.10.9 Write end-to-end tests** (3h)
  - Test full app lifecycle
  - Test search → activate workflow
  - Test hotkey → search → launch workflow
  - Dependencies: All

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (All sections)
- **Requirements:** `droidz/specs/2026-01-15-phase1-mvp/requirements.md`

---

## Instructions

1. Register all 3 search providers
2. Add menu bar status item
3. Implement config file loading/saving
4. Profile performance against targets
5. Write comprehensive E2E tests
6. Mark ALL tasks complete in tasks.md

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Cold start | < 100ms |
| Hotkey response | < 50ms |
| Search latency | < 30ms |
| Memory (idle) | < 50MB |
| UI rendering | 120 FPS |

---

## Final Checklist

- [ ] All search providers registered
- [ ] Menu bar icon shows and works
- [ ] Launch at login option works
- [ ] Config persists between launches
- [ ] All performance targets met
- [ ] E2E tests pass
