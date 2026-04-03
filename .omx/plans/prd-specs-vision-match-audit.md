# PRD — Mission-First Specs/Vision Match Audit

## Objective
Produce a detailed, evidence-based audit of whether **currently implemented/currently shipped** PhotonCast surfaces match the project's stated **vision, principles, and shipped-area specs**, using a **mission-first** authority order.

## Source of Truth / Authority Order
1. `droidz/product/mission.md`
2. Relevant shipped-area specs under `droidz/specs/**/spec.md`
3. `droidz/product/roadmap.md`
4. `README.md`
5. `ARCHITECTURE.md`

### Decision Boundary
If mission/principles conflict with older README/spec claims, classify the mismatch as **drift** unless implementation evidence clearly supports the stronger claim.

## In Scope
- Only currently implemented/currently shipped surfaces.
- Cross-cutting audit dimensions:
  - speed / responsiveness
  - privacy / local-first behavior
  - simplicity / no-AI posture
  - reliability / native feel
  - maintainability / sustainable simplicity
- Primary implementation roots:
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
  - current shipped `crates/photoncast-ext-*/`

## Out of Scope
- Future-only or draft-only roadmap work without credible shipped behavior.
- Direct fixes or implementation changes.
- Full roadmap compliance review for unbuilt features.

## Deliverables
1. Final audit report
2. Canonical promise inventory + shipped-scope matrix
3. Per-promise traceability table
4. Per-surface findings table
5. Cross-cutting principle synthesis
6. `claimed-but-unshipped` appendix

## Promise Classification Gate
Before substantive auditing, create a canonical promise inventory from the union of:
- `droidz/product/mission.md`
- relevant shipped-area specs
- `droidz/product/roadmap.md`
- `README.md`
- `ARCHITECTURE.md`

Classify every surfaced promise as exactly one of:
- `shipped`
- `claimed-but-unshipped`
- `future/draft`

Only `shipped` items receive scored findings in the main audit. The inventory and per-promise traceability table are required final artifacts, not optional working notes.

## Finding Taxonomy
Every substantive finding must be one of:
- `matches`
- `drifts`
- `contradictions`
- `unknown/unverified`

## Evidence Standard
For each substantive finding, capture:
- promise text
- authority source
- shipped-status decision
- implementation root
- implementation evidence
- corroborating proof artifact (test, benchmark, config, permission path, report) or explicit `unknown/unverified` downgrade
- verdict
- confidence
- rationale

## Required Audit Shape
### 1. Authority + Scope Matrix
A table of:
`promise → authority source → shipped-status decision → implementation root`

### 2. Per-Promise Traceability Table
A table of:
`promise text → authority source → shipped-status decision → implementation evidence → corroborating proof → final finding`

### 3. Per-Surface Audit
Audit each shipped surface with:
- governing promise(s)
- implementation area(s)
- taste/quality observations
- verdict + confidence
- citations

### 4. Cross-Cutting Principle Review
Synthesize findings across:
- speed
- privacy
- simplicity
- reliability
- sustainable simplicity

### 5. Claimed-but-Unshipped Appendix
Separate all promises that are still visible in docs but should not count against shipped scoring.

## Success Criteria
- Every surfaced promise is classified exactly once.
- Every shipped surface has either a scored audit section or a justified exclusion.
- Every major finding has doc evidence + implementation evidence + corroborating proof, or is downgraded to `unknown/unverified`.
- Mission-first conflicts are called out explicitly.
- Product-taste issues are separated from implementation-quality issues where useful.
- The report is reusable by downstream execution without reopening scope.

## Risks / Mitigations
- **Mission language overreach** → use promise classification gate before scoring.
- **README-centric incompleteness** → use union-of-promises inventory, not README alone.
- **Subjective quality judgments** → anchor each judgment to mission principles + evidence.
- **Raycast/ecosystem ambition contaminates scoring** → isolate into `claimed-but-unshipped` appendix unless clearly shipped.
- **Parallel audit drift** → freeze the authority/classification matrix before multi-lane work.

## Execution Steps
1. Build and freeze the authority + promise inventory.
2. Classify all surfaced promises.
3. Map shipped promises to implementation roots.
4. Audit shipped surfaces using the evidence standard.
5. Synthesize cross-cutting principle findings.
6. Publish final report + appendix.

## Verification
- PRD and test spec content explicitly require the union inventory and per-promise traceability table sourced from mission + shipped specs + roadmap + README + architecture.
- Scope matrix covers the union of mission/spec/roadmap/README/architecture promises.
- All shipped surfaces are accounted for, including the theming surface claimed in README and implemented in `crates/photoncast-theme/`.
- Main audit contains only `shipped` items.
- Evidence standard is applied consistently.

## ADR
- **Decision:** Use a mission-first, principle-led audit with a mandatory promise-classification gate.
- **Drivers:** detailed gap audit request; shipped-only scope; mission-first authority; need for defensible completeness.
- **Alternatives considered:** README/spec checklist audit; principle-only narrative review; architecture-first quality review.
- **Why chosen:** best balance of traceability, quality judgment, and scope control.
- **Consequences:** more up-front scope normalization, higher final confidence.
- **Follow-ups:** if drift is material, convert findings into a remediation roadmap later.

## Available-Agent-Types Roster
- `explore`
- `analyst`
- `planner`
- `architect`
- `critic`
- `executor`
- `verifier`
- `writer`
- `team-executor`

## Follow-up Staffing Guidance
### `$ralph`
- `executor` / high — perform classification + sequential audit
- `verifier` / high — validate evidence quality and coverage
- `writer` / medium — normalize final report if needed

### `$team`
- Leader: `team-executor`
- Lane A: `executor` / high — launcher/core/search/theme/file-search
- Lane B: `executor` / high — productivity crates + shipped extension surface
- Lane C: `verifier` / high — evidence quality + mission-first consistency
- Lane D: `writer` / medium — report normalization

## Launch Hints
- `$ralph .omx/plans/prd-specs-vision-match-audit.md`
- `$team .omx/plans/prd-specs-vision-match-audit.md`

## Team Verification Path
1. Freeze and cite the authority/promise-classification matrix.
2. Confirm every shipped surface has a section or explicit exclusion.
3. Confirm every major finding has evidence or explicit downgrade.
4. Confirm `claimed-but-unshipped` items are separated from shipped scoring.
5. Run final verifier pass for mission-first consistency.

## Applied Improvements
- Widened scope from README-first to a union-of-promises inventory.
- Added a mandatory promise-classification gate.
- Added a concrete evidence schema for each finding.
