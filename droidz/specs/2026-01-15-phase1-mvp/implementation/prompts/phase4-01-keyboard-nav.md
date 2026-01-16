# Implementation: 1.6 Keyboard Navigation

**Specialist:** frontend-specialist  
**Phase:** 4 (Interactions)  
**Dependencies:** 1.4.4 (ResultsList), 1.2.4 (GPUI Actions)

---

## Task Assignment

### 1.6 Keyboard Navigation

- [ ] **1.6.1 Implement selection state management** (2h) ⭐ CRITICAL
  - Track `selected_index: usize` in state
  - Clamp to valid range on results update
  - Reset to 0 on new search
  - Dependencies: 1.4.4

- [ ] **1.6.2 Implement ↑/↓ navigation** (2h) ⭐ CRITICAL
  - `SelectNext`: increment with bounds check
  - `SelectPrevious`: decrement with bounds check
  - Also support `Ctrl+N/P` alternatives
  - Dependencies: 1.6.1, 1.2.4

- [ ] **1.6.3 Implement Enter activation** (2h)
  - Dispatch `Activate` action on Enter
  - Get selected result and trigger action
  - Close launcher after activation
  - Dependencies: 1.6.1, 1.2.4

- [ ] **1.6.4 Implement Escape handling** (1h)
  - If query present: clear query
  - If query empty: close launcher
  - Dependencies: 1.2.4

- [ ] **1.6.5 Implement ⌘1-9 quick selection** (2h)
  - Map `⌘1` to first result, `⌘9` to ninth
  - Immediately activate selected result
  - Show shortcut badges in ResultItem
  - Dependencies: 1.2.4, 1.4.5

- [ ] **1.6.6 Implement Tab group cycling** (1h)
  - Tab: move to next group's first item
  - Shift+Tab: move to previous group
  - Dependencies: 1.6.1, 1.4.8

- [ ] **1.6.7 Implement scroll-to-selection** (2h)
  - Auto-scroll to keep selected item visible
  - Smooth scrolling within viewport
  - Dependencies: 1.6.1, 1.4.4

- [ ] **1.6.8 Write keyboard navigation tests** (2h)
  - Test ↑/↓ bounds checking
  - Test Enter activates correct item
  - Test ⌘1-9 quick selection
  - Dependencies: 1.6.1, 1.6.2, 1.6.3, 1.6.5

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 3.6: Keyboard Shortcuts)
- **Components Standard:** `droidz/standards/frontend/components.md`

---

## Instructions

1. Read spec.md Section 3.6 for keyboard shortcut table
2. Implement action handlers in LauncherWindow
3. Wire actions to GPUI key bindings from 1.2.5
4. Ensure bounds checking prevents crashes
5. Test all keyboard interactions
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use GPUI `actions!` macro for all keyboard actions
- Bounds check all index operations
- Call `cx.notify()` after selection changes
