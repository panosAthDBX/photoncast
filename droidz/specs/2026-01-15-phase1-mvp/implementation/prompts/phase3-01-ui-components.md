# Implementation: 1.4 Core UI Components

**Specialist:** frontend-specialist  
**Phase:** 3 (UI Components)  
**Dependencies:** 1.2.2 (LauncherWindow), 1.3.3 (Theme Provider)

---

## Task Assignment

### 1.4 Core UI Components

- [ ] **1.4.1 Implement SearchBar component** (3h) ⭐ CRITICAL
  - Create `src/ui/search_bar.rs` with `Render` trait
  - Add search icon (20px), text input (16px font)
  - Fixed height 48px, horizontal padding
  - Implement placeholder "Search PhotonCast..."
  - Dependencies: 1.3.3, 1.2.2

- [ ] **1.4.2 Add SearchBar focus handling** (2h)
  - Auto-focus on window show
  - Visual focus indicator (border color change)
  - Clear on Escape key
  - Dependencies: 1.4.1

- [ ] **1.4.3 Implement input debouncing** (1h)
  - 16ms debounce (single frame at 60 FPS)
  - Emit `on_change` event after debounce
  - Prevent excessive re-renders
  - Dependencies: 1.4.1

- [ ] **1.4.4 Implement ResultsList component** (3h) ⭐ CRITICAL
  - Create `src/ui/results_list.rs` with scrollable container
  - Implement virtual scrolling for performance
  - Calculate visible range, render only visible items
  - Add spacers for off-screen items
  - Dependencies: 1.3.3, 1.2.2

- [ ] **1.4.5 Implement ResultItem component** (3h) ⭐ CRITICAL
  - Create `src/ui/result_item.rs` with `RenderOnce` trait
  - Layout: icon (32px), title, subtitle, shortcut badge
  - Fixed height 56px, horizontal padding 16px
  - Dependencies: 1.3.3

- [ ] **1.4.6 Add ResultItem selection states** (2h)
  - Normal: default background
  - Hover: `surface_hover` background
  - Selected: `surface_selected` with accent border
  - Use theme colors for all states
  - Dependencies: 1.4.5

- [ ] **1.4.7 Implement match highlighting** (2h)
  - Accept `match_ranges: Vec<Range<usize>>` prop
  - Apply accent color to matched characters in title
  - Handle multi-range highlighting
  - Dependencies: 1.4.5

- [ ] **1.4.8 Implement ResultGroup component** (2h)
  - Create `src/ui/result_group.rs` for section headers
  - Display group name (Apps, Commands, Files)
  - Include shortcut hint (⌘1-5)
  - Dependencies: 1.3.3

- [ ] **1.4.9 Write component unit tests** (2h)
  - Test SearchBar renders correctly
  - Test ResultItem states (normal, hover, selected)
  - Test ResultsList virtual scrolling calculations
  - Dependencies: 1.4.1, 1.4.4, 1.4.5

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 3.3-3.4: Components)
- **Components Standard:** `droidz/standards/frontend/components.md`

---

## Instructions

1. Read spec.md Section 3.3-3.4 for component specifications
2. Create components in `src/ui/` directory
3. Use `RenderOnce` for stateless items, `Render` for stateful
4. Use theme colors from `theme(cx)` accessor
5. Implement virtual scrolling for ResultsList
6. Run tests and mark tasks complete in tasks.md

---

## Key Standards

- Use `RenderOnce` trait for ResultItem (stateless)
- Use builder pattern for component props
- All colors from theme, no hardcoded values
- Virtual scrolling for lists >10 items
