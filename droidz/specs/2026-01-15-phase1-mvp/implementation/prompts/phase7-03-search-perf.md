# Implementation: 2.7 Search Performance Optimization

**Specialist:** backend-specialist  
**Phase:** 7 (Search Advanced - Can run parallel)  
**Dependencies:** 2.4.5 (SearchEngine), 2.5.4 (Ranking)

---

## Task Assignment

### 2.7 Search Performance Optimization

- [ ] **2.7.1 Implement search index pre-computation** (2h)
  - Pre-lowercase app names for case-insensitive matching
  - Pre-sort by frequency for early termination
  - Store in memory for fast access
  - Dependencies: 2.4.4

- [ ] **2.7.2 Implement early termination** (2h)
  - Stop search when enough high-quality matches found
  - Use threshold: `max_results * 2`
  - Dependencies: 2.7.1

- [ ] **2.7.3 Add search benchmarks** (2h)
  - Create `benches/search_bench.rs`
  - Benchmark fuzzy match on 200 apps
  - Benchmark ranking on 100 results
  - Target: <30ms end-to-end
  - Dependencies: 2.4.5, 2.5.4

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 8.3: Optimization Strategies)

---

## Instructions

1. Read spec.md Section 8.3 for optimization patterns
2. Pre-compute lowercase names at index time
3. Implement early termination strategy
4. Create criterion benchmarks
5. Target <30ms search latency
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use criterion for benchmarks
- Pre-compute search data at index time
- Early termination when quality threshold met
- Target: <30ms end-to-end search
