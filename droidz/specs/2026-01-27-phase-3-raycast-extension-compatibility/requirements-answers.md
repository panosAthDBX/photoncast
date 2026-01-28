# Phase 3: Raycast Extension Compatibility — Requirements Answers

> Date: 2026-01-27

## A. Scope & Compatibility Level
1. **Compatibility target**: Comprehensive — 80%+ extension compatibility
2. **Spec structure**: Separate spec per sub-phase (B)
3. **Target extensions**: Brew, Kill Process, System Monitor, Set Audio Device, Clean Keyboard, Home Assistant

## B. Runtime Architecture
4. **Runtime**: Per-extension Node.js sidecar (A)
5. **Node.js management**: Bundle with PhotonCast app (A)
6. **Sidecar approach**: Node.js + JS bootstrap script, no separate binary (B)

## C. API Shim Design
7. **React serialization**: Custom React reconciler (A)
8. **Data hooks**: Use open-source `@raycast/utils` (C)
9. **Callback actions**: Callback registry with IDs across IPC (A)

## D. Extension Installation & Discovery
10. **Store**: In-app store browser with search and one-click install (C)
11. **Discovery**: Bridge wrapping Raycast extensions as native Extension trait objects (C)
12. **Build**: Hybrid — pre-built if available, build from source otherwise (C)
13. **esbuild**: Bundle with app (A)

## F. Security & Sandboxing
14. **Security model**: Full macOS sandbox profile (C)
15. **Permission consent**: Translate Raycast permissions, show consent dialog (A)

## G. Performance & Resource Management
16. **Performance targets**: Moderate — <1s cold start, <200ms warm (B)
17. **Spawn strategy**: Lazy spawn (A)

## H. Migration & Developer Experience
18. **Dev CLI**: Basic `photoncast dev <path>` (B)
19. **Error reporting**: Both toast + error view (C)
20. **Source maps**: Dev mode only (B)

## I. Crate & Module Structure
21. **Organization**: Separate bridge + IPC protocol crates (C)
22. **Node.js shim location**: `packages/raycast-compat/` in-repo (A)

## Follow-up Answers

F1. **macOS sandbox phasing**: B — Phase 3a uses process isolation only, full sandbox in 3b/3c
F2. **In-app store phasing**: B — Phase 3a uses manual install + CLI, store in 3c/3d
F3. **Target extension API needs**: Basic network + shell + filesystem should be sufficient. Evaluate and implement accordingly.
F4. **Store API source**: A — Use Raycast's public store API. If not available, fall back to B (GitHub monorepo).

## Visual Assets
- None provided
- Instruction: Research Raycast's visual patterns, mimic their approach, update/create docs as needed
