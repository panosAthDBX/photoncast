# Implementation: 1.3 Theme System

**Specialist:** frontend-specialist  
**Phase:** 2 (Core Setup - Can run parallel with 1.2)  
**Dependencies:** 1.1.4 (Core dependencies)

---

## Task Assignment

### 1.3 Theme System

- [ ] **1.3.1 Implement Catppuccin palette definitions** (2h) ⭐ CRITICAL
  - Create `src/theme/catppuccin.rs` with all 4 flavors
  - Define all 14 accent colors + 12 surface colors per flavor
  - Use `gpui::Hsla` for all color values
  - Match exact hex values from Catppuccin spec
  - Dependencies: 1.1.4

- [ ] **1.3.2 Create semantic color mapping** (2h) ⭐ CRITICAL
  - Implement `ThemeColors` struct with semantic roles
  - Map: background, surface, text, border, accent, status colors
  - Include hover/selected/focus states
  - Dependencies: 1.3.1

- [ ] **1.3.3 Implement theme provider** (2h) ⭐ CRITICAL
  - Create `PhotonTheme` struct implementing GPUI `Global`
  - Add `theme(cx: &App)` accessor function
  - Support runtime theme switching
  - Dependencies: 1.3.1, 1.3.2

- [ ] **1.3.4 Add accent color customization** (1h)
  - Implement `AccentColor` enum with all 14 options
  - Add `with_accent()` builder method
  - Default to Mauve
  - Dependencies: 1.3.3

- [ ] **1.3.5 Implement system theme detection** (2h)
  - Use `cocoa` crate to detect macOS appearance
  - Call `NSApp.effectiveAppearance()` for dark/light detection
  - Map system dark → Mocha, system light → Latte
  - Dependencies: 1.3.3

- [ ] **1.3.6 Add system theme change observer** (2h)
  - Use `cx.observe_system_appearance()` for live updates
  - Implement auto-sync option (enabled by default)
  - Trigger `cx.refresh()` on theme change
  - Dependencies: 1.3.5

- [ ] **1.3.7 Write theme unit tests** (1h)
  - Test all 4 flavors load correctly
  - Test semantic mapping produces valid colors
  - Test accent color override works
  - Dependencies: 1.3.1, 1.3.2, 1.3.3

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 4.4: Theme System)
- **Theming Standard:** `droidz/standards/frontend/theming.md`
- **Catppuccin Colors:** All hex values in theming.md

---

## Instructions

1. Read `droidz/standards/frontend/theming.md` for exact Catppuccin color values
2. Create `src/theme/` module with catppuccin.rs, colors.rs, provider.rs
3. Implement all 4 flavors: Latte, Frappé, Macchiato, Mocha
4. Use HSLA color format throughout
5. Test each flavor renders correctly
6. Mark tasks complete with [x] in tasks.md

---

## Key Standards

- Use exact Catppuccin hex values from theming.md
- Implement `Global` trait for theme provider
- Use `cx.observe_system_appearance()` for live theme sync
- All text must meet WCAG AA contrast standards
