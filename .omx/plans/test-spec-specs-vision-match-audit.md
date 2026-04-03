# Test Spec — Mission-First Specs/Vision Match Audit

## Purpose
Define what the audit execution must inspect and what evidence is required before any finding can be considered complete.

## Preconditions
1. `.omx/plans/prd-specs-vision-match-audit.md` exists.
2. Audit scope is limited to currently shipped/currently implemented surfaces.
3. Authority order is frozen before parallelization.

## Stage 0 — Authority + Promise Classification
### Must inspect
- `droidz/product/mission.md`
- relevant shipped-area specs in `droidz/specs/**/spec.md`
- `droidz/product/roadmap.md`
- `README.md`
- `ARCHITECTURE.md`

### Must produce
A canonical inventory table with these columns:
- promise text
- authority source
- shipped-status decision (`shipped` / `claimed-but-unshipped` / `future/draft`)
- implementation root(s)
- notes

A per-promise traceability table with these columns:
- promise text
- authority source
- shipped-status decision
- implementation evidence
- corroborating proof
- final finding

### Pass conditions
- Every surfaced promise is classified exactly once.
- README-listed shipped features are included in the inventory.
- Architecture/mission-only promises are either classified or explicitly ruled out.
- The per-promise traceability table exists and is sourced from the same union inventory.

## Stage 1 — Shipped Surface Coverage
### Must inspect
At minimum, the shipped implementation roots relevant to the inventory:
- `crates/photoncast/`
- `crates/photoncast-core/`
- `crates/photoncast-apps/`
- `crates/photoncast-calculator/`
- `crates/photoncast-calendar/`
- `crates/photoncast-clipboard/`
- `crates/photoncast-quicklinks/`
- `crates/photoncast-timer/`
- `crates/photoncast-window/`
- `crates/photoncast-theme/`
- `crates/photoncast-extension-api/`
- shipped `crates/photoncast-ext-*/`
- representative tests / benches / reports where available

### Pass conditions
- Every `shipped` promise has a mapped implementation root or an explicit justified exclusion.
- No `future/draft` or `claimed-but-unshipped` item is scored in the main audit.

## Stage 2 — Finding Evidence Rules
For every substantive finding, the audit must record:
- governing promise text
- governing authority source
- implementation evidence
- corroborating proof artifact from one of:
  - test
  - benchmark
  - config / runtime setting
  - permission / security path
  - report / verification artifact
  - if none available, explicit `unknown/unverified` downgrade
- verdict (`matches` / `drifts` / `contradictions` / `unknown/unverified`)
- confidence (`high` / `medium` / `low`)
- rationale

### Pass conditions
- No major finding lacks both doc evidence and implementation evidence.
- Principle-level claims without corroborating proof are downgraded to `unknown/unverified`.
- Mission-first conflicts are labeled explicitly as drift or contradiction.

## Stage 3 — Principle Coverage
The audit must evaluate shipped surfaces against these dimensions:
1. **Speed / responsiveness**
2. **Privacy / local-first behavior**
3. **Simplicity / no-AI posture**
4. **Reliability / native feel**
5. **Sustainable simplicity / maintainability**

### Pass conditions
- Each dimension has either concrete findings or an explicit explanation of limited visibility.
- Cross-cutting findings are separate from per-surface findings.

## Stage 4 — Final Output Contract
### Required sections
1. Executive summary
2. Authority + scope matrix
3. Per-promise traceability table
4. Per-surface audit findings
5. Cross-cutting principle synthesis
6. `claimed-but-unshipped` appendix
7. Unknown/unverified items

### Pass conditions
- Every shipped surface has a verdict or justified exclusion.
- `claimed-but-unshipped` items are separated from shipped scoring.
- No uncategorized shipped surfaces remain.

## Negative Tests
The audit fails if any of these occur:
- README is used as the sole scope oracle.
- Future-only promises are counted as shipped failures.
- A major finding has no implementation evidence.
- A major finding has no governing-doc anchor.
- Taste/quality claims are asserted without principle linkage.
- Parallel lanes begin before authority/classification is frozen.

## Verification Checklist
- [ ] PRD exists
- [ ] Inventory table exists
- [ ] All surfaced promises classified once
- [ ] All shipped surfaces mapped or explicitly excluded
- [ ] All major findings include evidence or downgrade
- [ ] Cross-cutting principle review complete
- [ ] Claimed-but-unshipped appendix present
- [ ] Unknown/unverified items present where needed
