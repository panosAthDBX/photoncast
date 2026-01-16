# Implementation: 2.4 Search Engine

**Specialist:** backend-specialist  
**Phase:** 6 (Search Setup - Can run parallel with 2.3)  
**Dependencies:** 2.1.3 (IndexedApp), 1.1.4 (Core dependencies)

---

## Task Assignment

### 2.4 Search Engine

- [ ] **2.4.1 Add nucleo dependency** (1h) ⭐ CRITICAL
  - Add `nucleo` crate (or `nucleo-matcher`)
  - Configure for Unicode normalization and smart case
  - Dependencies: 1.1.4

- [ ] **2.4.2 Implement FuzzyMatcher wrapper** (2h) ⭐ CRITICAL
  - Create `src/search/fuzzy.rs`
  - Wrap nucleo `Matcher` with PhotonCast config
  - Implement `score(query, target) -> Option<(u32, Vec<u32>)>`
  - Return match indices for highlighting
  - Dependencies: 2.4.1

- [ ] **2.4.3 Define SearchProvider trait** (1h)
  - Create `src/search/providers/mod.rs`
  - Define `trait SearchProvider: Send + Sync`
  - Methods: `name()`, `search()`, `result_type()`
  - Dependencies: 1.1.4

- [ ] **2.4.4 Implement AppProvider** (3h) ⭐ CRITICAL
  - Create `src/search/providers/apps.rs`
  - Query indexed apps from memory/cache
  - Apply fuzzy matching to app names
  - Return `Vec<RawSearchResult>` with scores
  - Dependencies: 2.4.2, 2.4.3, 2.1.3

- [ ] **2.4.5 Create SearchEngine orchestrator** (3h) ⭐ CRITICAL
  - Create `src/search/engine.rs`
  - Hold vector of `Box<dyn SearchProvider>`
  - Dispatch queries to all providers in parallel
  - Collect and merge results
  - Dependencies: 2.4.3, 2.4.4

- [ ] **2.4.6 Implement search result types** (2h)
  - Define `SearchResult`, `SearchAction`, `ResultType` in `src/search/mod.rs`
  - `SearchAction` enum: `LaunchApp`, `ExecuteCommand`, `OpenFile`, `RevealInFinder`
  - Add `SearchResultId` newtype
  - Dependencies: 2.4.5

- [ ] **2.4.7 Wire search to UI** (2h)
  - Connect SearchBar `on_change` to SearchEngine
  - Spawn async search task with `cx.spawn()`
  - Update ResultsList on completion
  - Dependencies: 2.4.5, 1.4.1, 1.4.4

- [ ] **2.4.8 Write search unit tests** (2h)
  - Test fuzzy matching accuracy
  - Test score consistency
  - Test match indices correctness
  - Dependencies: 2.4.2

- [ ] **2.4.9 Write search integration tests** (2h)
  - Test full search workflow
  - Test result merging from multiple providers
  - Target: <30ms search latency
  - Dependencies: 2.4.5

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 4.1: Search Engine)
- **Backend Standard:** `droidz/standards/backend/api.md`

---

## Instructions

1. Read spec.md Section 4.1 for search architecture
2. Use `nucleo` crate for fuzzy matching (crate-first!)
3. Create `src/search/` module structure
4. Implement SearchProvider trait pattern
5. Target <30ms search latency
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use `nucleo` for fuzzy matching (same as Helix)
- SearchProvider trait for extensibility
- Parallel provider execution with futures::join_all
- Return match indices for UI highlighting
