# Implementation: 3.8 Spotlight File Search

**Specialist:** backend-specialist  
**Phase:** 10 (Settings - Can run parallel with 3.5)  
**Dependencies:** 1.1.4 (Core dependencies)

---

## Task Assignment

### 3.8 Spotlight File Search

- [ ] **3.8.1 Implement NSMetadataQuery wrapper** (4h) ⭐ CRITICAL
  - Create `src/platform/spotlight.rs`
  - Use `objc2-foundation` for NSMetadataQuery
  - Build predicate for name search: `kMDItemDisplayName LIKE[cd] '*query*'`
  - Set search scopes: user home, local computer
  - Dependencies: 1.1.4

- [ ] **3.8.2 Implement async query execution** (2h)
  - Use `tokio::task::spawn_blocking` for query
  - Set result limit (configurable, default 5)
  - Add 500ms timeout
  - Dependencies: 3.8.1

- [ ] **3.8.3 Create FileResult type** (1h)
  - Define struct: path, name, kind (file/folder), size, modified
  - Implement `FileKind::from_path()` helper
  - Dependencies: 3.8.1

- [ ] **3.8.4 Implement FileProvider** (3h) ⭐ CRITICAL
  - Create `src/search/providers/files.rs`
  - Implement `SearchProvider` trait
  - Wrap Spotlight queries
  - Convert `FileResult` to `SearchResult`
  - Dependencies: 2.4.3, 3.8.1, 3.8.3

- [ ] **3.8.5 Implement file opening** (2h)
  - Use NSWorkspace to open files
  - Handle `SearchAction::OpenFile` variant
  - Add "Reveal in Finder" alternative action
  - Dependencies: 3.8.4

- [ ] **3.8.6 Add file usage tracking** (1h)
  - Track file opens in database
  - Update frecency for files
  - Dependencies: 3.8.5, 2.2.5

- [ ] **3.8.7 Write Spotlight integration tests** (2h)
  - Test query execution returns results
  - Test timeout handling
  - Test result conversion
  - Dependencies: 3.8.1, 3.8.4

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 5.4: Spotlight Integration)
- **Platform Standard:** `droidz/standards/backend/platform.md`

---

## Instructions

1. Read spec.md Section 5.4 for NSMetadataQuery usage
2. Use `objc2-foundation` for Objective-C interop
3. Create `src/platform/spotlight.rs`
4. Implement 500ms timeout for queries
5. Test with real file searches
6. Mark tasks complete in tasks.md

---

## Key Standards

- Use NSMetadataQuery for Spotlight access
- 500ms timeout for queries
- Default 5 file results
- Track file usage for frecency
