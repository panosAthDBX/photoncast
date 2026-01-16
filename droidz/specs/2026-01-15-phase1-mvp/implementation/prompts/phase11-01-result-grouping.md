# Implementation: 3.9 Result Grouping

**Specialist:** full-stack-specialist  
**Phase:** 11 (Integration)  
**Dependencies:** 2.4.6 (Search result types), 1.4.8 (ResultGroup component)

---

## Task Assignment

### 3.9 Result Grouping

- [ ] **3.9.1 Implement result grouping logic** (2h) ⭐ CRITICAL
  - Create `SearchResults::grouped()` method
  - Group results by `ResultType`
  - Sort groups: Apps → Commands → Files
  - Dependencies: 2.4.6

- [ ] **3.9.2 Update ResultsList for grouped display** (2h)
  - Render `ResultGroup` headers between sections
  - Display group name and shortcut range
  - Dependencies: 3.9.1, 1.4.8

- [ ] **3.9.3 Implement inter-group navigation** (2h)
  - Tab moves to next group
  - Update ⌘1-9 to work across groups
  - Dependencies: 3.9.2, 1.6.6

- [ ] **3.9.4 Write grouping tests** (1h)
  - Test correct grouping order
  - Test navigation across groups
  - Dependencies: 3.9.1, 3.9.3

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 3.1: Window Layout)

---

## Instructions

1. Implement grouping in SearchResults
2. Order: Apps → Commands → Files
3. Update ResultsList to show headers
4. Fix ⌘1-9 shortcuts for grouped results
5. Test navigation across groups
6. Mark tasks complete in tasks.md

---

## Key Standards

- Group order: Apps → Commands → Files
- Show section headers with names
- ⌘1-9 shortcuts work across groups
