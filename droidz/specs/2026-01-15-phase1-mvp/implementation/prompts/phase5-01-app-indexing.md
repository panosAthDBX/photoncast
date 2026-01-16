# Implementation: 2.1 Application Indexing

**Specialist:** backend-specialist  
**Phase:** 5 (Backend Foundation)  
**Dependencies:** 1.1.4 (Core dependencies)

---

## Task Assignment

### 2.1 Application Indexing

- [ ] **2.1.1 Create app scanner module** (3h) ⭐ CRITICAL
  - Create `src/indexer/scanner.rs`
  - Scan `/Applications`, `/System/Applications`, `~/Applications`
  - Filter to `.app` bundles only
  - Exclude patterns: `.prefPane`, `*Uninstaller*`, nested apps
  - Dependencies: 1.1.4

- [ ] **2.1.2 Implement Info.plist parser** (3h) ⭐ CRITICAL
  - Create `src/indexer/metadata.rs`
  - Add `plist` crate dependency
  - Parse `CFBundleDisplayName` or `CFBundleName`
  - Extract `CFBundleIdentifier` (required)
  - Extract `LSApplicationCategoryType` (optional)
  - Dependencies: 2.1.1

- [ ] **2.1.3 Create IndexedApp data structure** (1h)
  - Define struct with name, bundle_id, path, icon, keywords, category
  - Implement `last_modified` timestamp tracking
  - Add `Clone`, `Debug`, `PartialEq` derives
  - Dependencies: 2.1.2

- [ ] **2.1.4 Implement async directory scanning** (2h)
  - Use `tokio::fs::read_dir` for non-blocking I/O
  - Spawn concurrent tasks per directory
  - Collect results with timeout handling
  - Target: <2s for full scan of ~200 apps
  - Dependencies: 2.1.1, 2.1.2

- [ ] **2.1.5 Implement icon extraction** (4h)
  - Create `src/indexer/icons.rs`
  - Read `CFBundleIconFile` from Info.plist
  - Load `.icns` files from `Contents/Resources/`
  - Add `icns` crate for parsing
  - Cache extracted icons to disk
  - Dependencies: 2.1.2

- [ ] **2.1.6 Create icon cache system** (2h)
  - LRU cache with 100 icon limit
  - Store in `~/Library/Caches/PhotonCast/icons/`
  - Lazy loading with `OnceCell`
  - Dependencies: 2.1.5

- [ ] **2.1.7 Write indexer unit tests** (2h)
  - Test plist parsing with fixture files
  - Test app bundle discovery
  - Test icon extraction
  - Dependencies: 2.1.2, 2.1.5

- [ ] **2.1.8 Write indexer integration tests** (2h)
  - Create mock app bundles in temp directory
  - Test full scan workflow
  - Verify metadata extraction accuracy
  - Dependencies: 2.1.4

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 4.3: App Indexer)
- **Backend Standard:** `droidz/standards/backend/api.md`
- **Crate-First:** `droidz/standards/global/crate-first.md`

---

## Instructions

1. Read spec.md Section 4.3 for indexer architecture
2. Search crates.io for `plist` and `icns` crates (crate-first!)
3. Create `src/indexer/` module structure
4. Implement async scanning with tokio
5. Target <2s indexing time for ~200 apps
6. Run tests and mark complete in tasks.md

---

## Key Standards

- Use `plist` crate for Info.plist parsing
- Use `tokio::fs` for async file I/O
- Cache icons in ~/Library/Caches/PhotonCast/
- All I/O operations must be async
