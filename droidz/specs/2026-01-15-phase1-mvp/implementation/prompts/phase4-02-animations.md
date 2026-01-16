# Implementation: 1.7 Animations

**Specialist:** frontend-specialist  
**Phase:** 4 (Interactions - Can run parallel with 1.6)  
**Dependencies:** 1.2.3 (Window show/hide), 1.4.6 (ResultItem states)

---

## Task Assignment

### 1.7 Animations

- [ ] **1.7.1 Implement window appear animation** (2h)
  - 150ms ease-out animation
  - Fade in + slight scale up
  - Dependencies: 1.2.3

- [ ] **1.7.2 Implement window dismiss animation** (1h)
  - 100ms ease-in animation
  - Fade out + slight scale down
  - Dependencies: 1.2.3

- [ ] **1.7.3 Implement selection change animation** (2h)
  - 80ms ease-in-out transition
  - Smooth background color transition
  - Dependencies: 1.4.6

- [ ] **1.7.4 Implement hover highlight animation** (1h)
  - 60ms linear transition
  - Subtle background color change
  - Dependencies: 1.4.6

- [ ] **1.7.5 Implement reduce motion support** (2h) ⭐ CRITICAL
  - Detect `NSWorkspace.accessibilityDisplayShouldReduceMotion`
  - Create `animation_duration()` helper function
  - When enabled: instant transitions, no physics
  - Support PhotonCast settings override
  - Dependencies: 1.7.1, 1.7.2, 1.7.3, 1.7.4

- [ ] **1.7.6 Write animation tests** (1h)
  - Test reduce motion detection
  - Test animation duration helper
  - Dependencies: 1.7.5

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 3.7: Animations)
- **Accessibility Standard:** `droidz/standards/frontend/accessibility.md`

---

## Instructions

1. Read spec.md Section 3.7 for animation timings
2. Use GPUI animation primitives
3. Implement reduce motion check from cocoa
4. Create helper function for duration with reduce motion fallback
5. Test animations render smoothly at 120 FPS
6. Mark tasks complete in tasks.md

---

## Key Standards

- All animations respect reduce motion preference
- Use ease curves specified in spec
- Target 120 FPS during all animations
