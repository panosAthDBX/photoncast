# Implementation: 2.3 File System Watcher

**Specialist:** backend-specialist  
**Phase:** 6 (Search Setup)  
**Dependencies:** 2.1.1 (App scanner)

---

## Task Assignment

### 2.3 File System Watcher

- [ ] **2.3.1 Add notify crate dependency** (1h)
  - Add `notify` crate for cross-platform FS events
  - Configure for macOS FSEvents backend
  - Dependencies: 1.1.4

- [ ] **2.3.2 Implement FS watcher setup** (3h) ⭐ CRITICAL
  - Create `src/indexer/watcher.rs`
  - Watch all 3 scan paths non-recursively
  - Handle `Create`, `Modify`, `Remove` events
  - Filter to `.app` bundles only
  - Dependencies: 2.3.1, 2.1.1

- [ ] **2.3.3 Implement debounced updates** (2h)
  - 500ms debounce for batch operations
  - Coalesce multiple events for same path
  - Trigger incremental re-index after debounce
  - Dependencies: 2.3.2

- [ ] **2.3.4 Wire up watcher to indexer** (2h)
  - Start watcher on app launch
  - Connect events to `AppIndexer.rescan_directory()`
  - Log watcher events at debug level
  - Dependencies: 2.3.2, 2.1.4

- [ ] **2.3.5 Write watcher integration tests** (2h)
  - Test app install detection
  - Test app removal detection
  - Test debouncing behavior
  - Dependencies: 2.3.2, 2.3.3

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 4.3: Background Re-indexing)
- **Crate-First:** `droidz/standards/global/crate-first.md`

---

## Instructions

1. Read spec.md Section 4.3 for watcher architecture
2. Use `notify` crate (crate-first!)
3. Watch /Applications, ~/Applications, /System/Applications
4. Implement 500ms debounce for batch updates
5. Test with actual app install/uninstall
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use `notify` crate for FS watching
- 500ms debounce to prevent thrashing
- Non-recursive watching (apps are at top level)
