# Implementation: 2.5 Ranking Algorithm

**Specialist:** backend-specialist  
**Phase:** 7 (Search Advanced)  
**Dependencies:** 2.4.5 (SearchEngine), 2.2.5 (Database operations)

---

## Task Assignment

### 2.5 Ranking Algorithm

- [ ] **2.5.1 Implement pure match quality ranking** (2h) ⭐ CRITICAL
  - Create `src/search/ranking.rs`
  - Sort results by nucleo score (higher is better)
  - Dependencies: 2.4.5

- [ ] **2.5.2 Implement frecency calculation** (3h) ⭐ CRITICAL
  - Define `FrecencyScore` struct
  - Calculate: `frequency * recency_decay`
  - Recency decay: half-life of 72 hours
  - Query usage data from database
  - Dependencies: 2.2.5, 2.5.1

- [ ] **2.5.3 Implement boost factors** (2h)
  - Create `BoostConfig` with configurable multipliers
  - Path boosts: 1.2x for `/System/Applications`, 1.1x for `/Applications`
  - Match boosts: 2.0x for exact match, 1.5x for prefix match
  - Dependencies: 2.5.1

- [ ] **2.5.4 Implement combined ranking** (2h)
  - Formula: `final_score = match_score + (frecency * 10.0)`
  - Apply boosts after combination
  - Dependencies: 2.5.1, 2.5.2, 2.5.3

- [ ] **2.5.5 Implement tiebreaker logic** (1h)
  - Order: usage count → recency → alphabetical
  - Ensure deterministic ordering
  - Dependencies: 2.5.4

- [ ] **2.5.6 Write ranking unit tests** (2h)
  - Test frecency calculation with known values
  - Test boost application
  - Test tiebreaker ordering
  - Dependencies: 2.5.2, 2.5.3, 2.5.5

- [ ] **2.5.7 Write ranking property tests** (2h)
  - Test ranking is deterministic (same input → same output)
  - Test exact matches always rank higher than partial
  - Dependencies: 2.5.4

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 4.2: Ranking Algorithm)

---

## Instructions

1. Read spec.md Section 4.2 for ranking formulas
2. Implement frecency with 72-hour half-life
3. Apply boosts in correct order
4. Use proptest for determinism testing
5. Mark tasks complete in tasks.md

---

## Key Standards

- Frecency half-life: 72 hours
- Tiebreaker: usage count → recency → alphabetical
- Ranking must be deterministic
