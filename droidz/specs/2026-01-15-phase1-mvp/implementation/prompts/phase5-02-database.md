# Implementation: 2.2 Database & Persistence

**Specialist:** database-specialist  
**Phase:** 5 (Backend Foundation - Can run parallel with 2.1)  
**Dependencies:** 1.1.4 (Core dependencies)

---

## Task Assignment

### 2.2 Database & Persistence

- [ ] **2.2.1 Add rusqlite dependency** (1h)
  - Add `rusqlite` with `bundled` feature
  - Configure async wrapper or use `tokio::task::spawn_blocking`
  - Dependencies: 1.1.4

- [ ] **2.2.2 Create database schema** (2h) ⭐ CRITICAL
  - Create `src/storage/database.rs`
  - Define `app_usage` table (bundle_id, launch_count, last_launched)
  - Define `command_usage` table
  - Define `file_usage` table
  - Define `app_cache` table for indexed apps
  - Dependencies: 2.2.1

- [ ] **2.2.3 Implement database migrations** (2h)
  - Create migration system for schema versioning
  - Add migration 001: initial schema
  - Auto-run migrations on startup
  - Dependencies: 2.2.2

- [ ] **2.2.4 Create database wrapper** (2h)
  - Implement `Database` struct with connection pool
  - Add async query methods
  - Handle connection errors gracefully
  - Dependencies: 2.2.2

- [ ] **2.2.5 Implement app cache operations** (2h)
  - `insert_app()`, `get_all_apps()`, `remove_app()`
  - `update_app()` for incremental updates
  - Batch insert support for full re-index
  - Dependencies: 2.2.4

- [ ] **2.2.6 Write database tests** (2h)
  - Test migrations run correctly
  - Test CRUD operations
  - Test concurrent access
  - Dependencies: 2.2.2, 2.2.4, 2.2.5

---

## Context Files

- **Spec:** `droidz/specs/2026-01-15-phase1-mvp/spec.md` (Section 6.4: Database Schema)
- **Database Standard:** `droidz/standards/backend/queries.md`

---

## Instructions

1. Read spec.md Section 6.4 for exact schema SQL
2. Use `rusqlite` with bundled SQLite
3. Create `src/storage/` module
4. Implement migration system with version tracking
5. Store database in `~/Library/Application Support/PhotonCast/`
6. Run tests and mark complete in tasks.md

---

## Key Standards

- Use `rusqlite` with bundled feature
- Wrap blocking DB calls with `spawn_blocking`
- Use prepared statements for all queries
- Implement proper migration versioning
