# Implementation: 3.7 Command Search Provider

**Specialist:** backend-specialist  
**Phase:** 9 (Hotkey - Can run parallel)  
**Dependencies:** 3.6.1 (SystemCommand enum), 2.4.3 (SearchProvider trait)

---

## Task Assignment

### 3.7 Command Search Provider

- [ ] **3.7.1 Implement CommandProvider** (3h) ⭐ CRITICAL
  - Create `src/search/providers/commands.rs`
  - Implement `SearchProvider` trait
  - Match query against command names and aliases
  - Return command results with icons
  - Dependencies: 2.4.3, 3.6.1

- [ ] **3.7.2 Wire command activation** (2h)
  - Handle `SearchAction::ExecuteCommand` variant
  - Execute command with confirmation if required
  - Close launcher after execution
  - Dependencies: 3.7.1, 3.6.8

- [ ] **3.7.3 Add command usage tracking** (1h)
  - Track command executions in database
  - Update frecency for commands
  - Dependencies: 3.7.2, 2.2.5

- [ ] **3.7.4 Write command provider tests** (1h)
  - Test search returns correct commands
  - Test alias matching
  - Dependencies: 3.7.1

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.3)
- **Built-in Commands:** `droidz/standards/backend/builtin-commands.md`

---

## Instructions

1. Implement SearchProvider trait for commands
2. Match against both names and aliases
3. Execute with confirmation for destructive commands
4. Track usage for frecency
5. Mark tasks complete in tasks.md

---

## Key Standards

- Match query against names AND aliases
- Confirmation dialog for destructive commands
- Track usage for frecency ranking
