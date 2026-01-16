# Implementation: 1.5 UI States

**Specialist:** frontend-specialist  
**Phase:** 3 (UI Components - Can run parallel with 1.4)  
**Dependencies:** 1.3.3 (Theme Provider)

---

## Task Assignment

### 1.5 UI States

- [ ] **1.5.1 Implement EmptyState component** (2h)
  - Create `src/ui/empty_state.rs`
  - No query: "Type to search apps, commands, and files"
  - No results: 'No results for "query"'
  - Include keyboard hints
  - Dependencies: 1.3.3

- [ ] **1.5.2 Implement LoadingState component** (2h)
  - Create loading spinner animation
  - Display "Indexing applications..." with progress
  - Show found count "Found 142 of ~200 apps"
  - Dependencies: 1.3.3

- [ ] **1.5.3 Implement ErrorState component** (2h)
  - Display error icon and message
  - Include action buttons (Retry, Open Folder)
  - Style with warning/error theme colors
  - Dependencies: 1.3.3

- [ ] **1.5.4 Wire up state management** (2h)
  - Create `LauncherState` enum (Empty, Loading, Results, Error)
  - Connect states to ResultsList display
  - Handle state transitions
  - Dependencies: 1.5.1, 1.5.2, 1.5.3

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 3.5: UI States)
- **Components Standard:** `droidz/standards/frontend/components.md`

---

## Instructions

1. Read spec.md Section 3.5 for state wireframes
2. Create state components in `src/ui/`
3. Implement loading spinner with CSS animation
4. Wire state enum to LauncherWindow
5. Test all state transitions
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use `LauncherState` enum for type-safe state management
- Match state wireframes from spec exactly
- Error states must have actionable recovery options
