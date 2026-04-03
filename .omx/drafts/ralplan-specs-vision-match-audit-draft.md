# Consensus Plan Draft — Mission-First Specs/Vision Match Audit

## Requirements Summary
- Produce a **detailed gap audit** for **currently implemented/currently shipped** PhotonCast areas only.
- Judge alignment **mission-first**, with `droidz/product/mission.md` as the primary authority when older docs/specs/README claims diverge.
- Prevent aspirational/future-facing product claims from being mis-scored as failures of the shipped product.
- Evaluate shipped behavior against the repo's stated taste/quality bar: **speed**, **privacy/local-first behavior**, **simplicity/no AI gimmicks**, **native reliability**, and **sustainable maintainability**.
- Ground claims in repo evidence from the implementation and shipped-area specs, not assumptions.

### Evidence anchors
- Mission + principles: `droidz/product/mission.md:7-28`, `droidz/product/mission.md:152-178`
- Roadmap phase boundaries: `droidz/product/roadmap.md:1-24`, `droidz/product/roadmap.md:29-78`
- Claimed shipped surface and module map: `README.md:5-20`, `README.md:66-91`
- Architecture and dependency boundaries: `ARCHITECTURE.md:1-82`
- Shipped Phase 2/v1.0 goals and non-goals: `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:31-58`, `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md:74-105`
- Current shipped implementation roots:
  - `crates/photoncast/src/main.rs`
  - `crates/photoncast-core/src/lib.rs`
  - `crates/photoncast-apps/src/lib.rs`
  - `crates/photoncast-calculator/src/lib.rs`
  - `crates/photoncast-calendar/src/lib.rs`
  - `crates/photoncast-clipboard/src/lib.rs`
  - `crates/photoncast-quicklinks/src/lib.rs`
  - `crates/photoncast-timer/src/lib.rs`
  - `crates/photoncast-window/src/lib.rs`
  - `crates/photoncast-theme/src/lib.rs`
  - shipped extension crates under `crates/photoncast-ext-*/src/lib.rs`

## Acceptance Criteria
1. Write a PRD at `.omx/plans/prd-specs-vision-match-audit.md` describing audit scope, authority order, promise classification rules, audit dimensions, evidence standards, and output structure.
2. Write a test spec at `.omx/plans/test-spec-specs-vision-match-audit.md` defining what must be inspected and how findings are verified.
3. Before substantive auditing, classify every surfaced promise as exactly one of: `shipped`, `claimed-but-unshipped`, `future/draft`.
4. Audit only the `shipped` class for scored findings; move `claimed-but-unshipped` items into a separate appendix.
5. Every substantive finding is categorized as `matches`, `drifts`, `contradictions`, or `unknown/unverified`.
6. Every substantive finding cites repo evidence; principle-level findings require at least one governing-doc anchor, one implementation anchor, and one corroborating proof type (test, benchmark, config, permission path, encryption path, or explicit downgrade to `unknown/unverified`).
7. The plan makes the **union inventory + per-promise traceability table** a required artifact, not just a planning idea.
8. The plan covers all currently shipped areas claimed in `README.md:7-20`, including the theme surface, or clearly justifies exclusions.
9. The plan freezes the authority matrix and promise classification before any parallel execution lanes begin.
10. The final audit output is concrete enough that a follow-up execution agent can produce the audit without reopening requirements.

## Implementation Steps
0. **Build and freeze the authority + promise-classification matrix**
   - Read `droidz/product/mission.md`, `droidz/product/roadmap.md`, `README.md`, and `ARCHITECTURE.md` first.
   - Classify each surfaced promise as:
     - `shipped`
     - `claimed-but-unshipped`
     - `future/draft`
   - Use authority ordering: `mission/principles` > `shipped-area spec` > `README/architecture notes`.
   - Freeze this matrix before any parallel lane starts.
   - Output: authoritative audit scope table, per-promise traceability table seed, and appendix seed.

1. **Establish the shipped-area audit map**
   - Use `README.md:5-20` and `README.md:66-91` to enumerate claimed shipped surfaces and map them to crates.
   - Use `ARCHITECTURE.md:1-82` to validate module boundaries and shared subsystems.
   - Cross-check against Phase 2 shipped goals/non-goals.
   - Explicitly separate:
     - **shipped native extension surface** (`crates/photoncast-extension-api`, `crates/photoncast-ext-*`)
     - **future/aspirational Raycast ecosystem promises** (appendix only unless credible shipped behavior exists)
   - Output: matrix of `feature → promise class → mission principles → governing docs → implementation modules`.

2. **Define audit heuristics by mission principle**
   - **Speed**: latency-sensitive architecture, search/indexing/ranking design, benchmarks, perf tests, startup/hotkey claims.
   - **Privacy**: local-only storage, encryption, permission handling, telemetry/account/cloud absence.
   - **Simplicity**: absence of AI/account/subscription surfaces; avoidance of unnecessary shipped surface sprawl.
   - **Reliability/native feel**: macOS-native APIs, fallbacks, permission flows, failure handling.
   - **Sustainable simplicity**: crate/module boundaries, test coverage shape, standards alignment, complexity hotspots.
   - Output: explicit judgment rubric so “taste and quality” stays reproducible.

3. **Plan the evidence-gathering pass**
   - Inspect governing docs and key implementation files first.
   - Inspect representative tests, benchmarks, reports, config, or sensitive code paths where relevant.
   - For each finding, capture:
     - governing source
     - implementation source
     - corroborating proof type (or `unknown/unverified` downgrade)
     - observed alignment status
     - confidence level
     - rationale
   - Output: repeatable evidence template for the audit artifact.

4. **Define the final audit deliverable**
   - Executive summary with mission-first verdict.
   - Per-surface sections with `matches / drifts / contradictions / unknowns`.
   - Cross-cutting findings split into:
     - **product-taste issues**
     - **implementation-quality issues**
   - `claimed-but-unshipped` appendix separated from shipped scoring.
   - Prioritized remediation opportunities only as follow-up suggestions, not implementation.
   - Output: final report shape and evidence checklist.

5. **Write planning artifacts**
   - Save PRD and test-spec under `.omx/plans/`.
   - Include execution handoff guidance for either a sequential (`$ralph`) or parallel (`$team`) audit run.

## Risks and Mitigations
- **Risk: future-facing mission language is mis-scored as shipped drift.**  
  **Mitigation:** enforce Stage 0 promise classification and isolate `claimed-but-unshipped` in a separate appendix.
- **Risk: doc-only conclusions overstate certainty.**  
  **Mitigation:** require corroborating proof for principle-level findings or downgrade to `unknown/unverified`.
- **Risk: README overstates shipped status.**  
  **Mitigation:** require implementation evidence before counting a surface as matched.
- **Risk: “taste and quality” becomes subjective.**  
  **Mitigation:** anchor each judgment to a mission principle plus explicit evidence tier.
- **Risk: extension/Raycast ambition contaminates shipped-surface scoring.**  
  **Mitigation:** split shipped native extensions from Raycast ecosystem promises.
- **Risk: parallel lanes diverge on what counts as drift.**  
  **Mitigation:** freeze the authority/classification matrix before parallel execution.

## Verification Steps
1. Confirm the PRD and test-spec content explicitly require a canonical union inventory and per-promise traceability table sourced from mission + shipped specs + roadmap + README + architecture.
2. Confirm every surfaced promise has exactly one class: `shipped`, `claimed-but-unshipped`, or `future/draft`.
3. Confirm every shipped area claimed in `README.md:7-20`, including theming, has either an audit section or an explicit exclusion.
4. Confirm the plan includes mission-first authority ordering and a frozen classification gate before parallel work.
5. Confirm the test spec requires governing-doc evidence + implementation evidence + corroborating proof (or explicit downgrade) for principle-level findings.
6. Confirm the final deliverable separates `claimed-but-unshipped` items from shipped scoring.
7. Confirm the handoff includes both sequential and parallel execution options without direct implementation.

## RALPLAN-DR Summary
### Principles
1. **Mission-first authority** over stale doc/spec drift.
2. **Evidence over impression** for every quality claim.
3. **Shipped surfaces only** for scored findings.
4. **Promise classification before auditing** to keep future ambition out of shipped scoring.
5. **Repeatable output structure** so the audit can be rerun later.

### Decision Drivers
1. The user explicitly wants a **detailed gap audit**, not a lightweight verdict.
2. The user explicitly wants a **mission-first** judgment standard.
3. The repo contains both shipped surfaces and aspirational product claims, so scope normalization must happen first.

### Viable Options
#### Option A — Principle-led audit matrix with a shipped-scope gate **(recommended)**
- **Approach:** Classify promises first, then audit only shipped surfaces against mission principles, governing docs, and implementation evidence.
- **Pros:** Best fit for “taste and quality”; prevents future ambition from polluting shipped scoring; reusable for future audits.
- **Cons:** Slightly more setup before execution; requires stricter evidence discipline.

#### Option B — Feature-by-feature spec compliance audit
- **Approach:** Start from README/spec features and check whether each shipped feature appears implemented as described.
- **Pros:** Faster to execute; straightforward checklist.
- **Cons:** Too literal; may miss mission drift and cross-cutting quality problems.

#### Option C — Architecture-first codebase quality audit
- **Approach:** Focus on crate boundaries, tests, performance/privacy design, then infer product alignment secondarily.
- **Pros:** Strong for maintainability and technical taste.
- **Cons:** Too indirect for the user's product/vision-alignment brief.

### Recommendation
Choose **Option A** because it preserves the user's explicit mission-first, shipped-only, detailed-gap-audit brief while adding the scope gate needed to keep future-facing claims from being misclassified.

## ADR
- **Decision:** Plan and execute a mission-first, principle-led audit matrix for currently shipped PhotonCast areas, with a mandatory promise-classification gate before auditing.
- **Drivers:** User-selected detailed gap audit; mission-first authority order; need to separate shipped reality from aspirational docs; need for repeatable, evidence-backed findings.
- **Alternatives considered:**
  - Feature checklist/spec compliance only — rejected as too shallow for taste/quality.
  - Architecture-only audit — rejected as too indirect for product/vision alignment.
- **Why chosen:** It balances product principles, shipped behavior, and concrete implementation evidence without letting future ambition distort current-product scoring.
- **Consequences:** Execution will spend more up front on scope normalization, but the findings will be more defensible and less noisy.
- **Follow-ups:** If the audit reveals material drift, a later planning step can convert findings into a remediation roadmap.

## Available-Agent-Types Roster
Relevant roles from the current agent catalog:
- `explore` — fast repo mapping and file/symbol discovery
- `analyst` — maintain finding taxonomy, severity, and confidence structure
- `planner` — audit-plan refinement / sequencing
- `architect` — authority-order and audit-structure review
- `critic` — quality gate for the plan and final audit report shape
- `executor` — conduct the actual repo audit and write the report
- `verifier` — validate evidence quality and coverage of shipped areas
- `writer` — polish the final audit document for clarity
- `team-executor` — coordinate conservative multi-lane audit execution

## Follow-up Staffing Guidance
### If using `$ralph`
- **Lane 1: executor (high)** — perform Stage 0 classification, then audit the repo sequentially surface by surface.
- **Lane 2: verifier (high)** — validate that every major finding has adequate evidence and that shipped-area coverage is complete.
- **Optional Lane 3: writer (medium)** — tighten the final report structure without altering conclusions.
- **Why this laneing:** sequential ownership fits an evidence-heavy audit where later claims depend on the frozen classification matrix.

### If using `$team`
- **Leader:** `team-executor` or leader context with this plan.
- **Prerequisite gate:** leader completes and freezes the authority/promise-classification matrix before worker lanes start substantive auditing.
- **Worker A: executor (high)** — core launcher/search/theme/hotkey/file-search audit.
- **Worker B: executor (high)** — productivity feature crates audit (clipboard/calculator/calendar/quicklinks/timer/apps/window).
- **Worker C: verifier (high)** — cross-check evidence quality, shipped-area coverage, and mission-principle consistency.
- **Worker D: writer (medium)** — normalize report structure and evidence tables after findings stabilize.
- **Why this split:** write scopes can be separated by audit sections while verifier remains independent.

## Launch Hints
### Ralph
- `$ralph .omx/plans/prd-specs-vision-match-audit.md`
- Provide `.omx/plans/test-spec-specs-vision-match-audit.md` as the verification target during execution.

### Team
- `$team .omx/plans/prd-specs-vision-match-audit.md`
- Or equivalent OMX team launch with lanes aligned to the staffing guidance above.

## Team Verification Path
Before shutdown, the team should prove:
1. The frozen authority/promise-classification matrix is complete and cited.
2. Every shipped area in scope has an audit section or explicit exclusion.
3. Every major finding contains governing-doc evidence, implementation evidence, and corroborating proof or explicit downgrade.
4. The final report distinguishes `matches`, `drifts`, `contradictions`, and `unknown/unverified`.
5. `claimed-but-unshipped` items are separated from shipped scoring.
6. A verifier pass confirms mission-first consistency across all findings.
7. If handed back to Ralph, Ralph performs the final cross-section sanity check and produces the user-facing summary.


## Applied Improvements
- Widened the scope gate from README-first coverage to a union-of-promises inventory across mission, roadmap, README, architecture, and shipped specs.
- Added an explicit per-promise traceability table schema to strengthen defensibility.
- Reframed README coverage as a minimum check instead of the sole shipped-surface oracle.
