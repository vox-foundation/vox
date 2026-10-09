# Requirements: Vox v1.1 Research Trial Flywheel

**Defined:** 2026-10-01
**Core Value:** The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout.
**Milestone value:** Each research run produces reproducible evidence, comparable quality/latency/cost metrics, searchable verified knowledge, and a safe proposal for the next experiment.

**Source:** Research Trial Flywheel plan (grill rulings G1–G25, review rulings R1–R9) and `.planning/research/`. v1.0 requirements are archived in `.planning/milestones/v1.0-REQUIREMENTS.md`.

## v1.1 Requirements

### Trial Contract

- [ ] **TRIAL-01**: Every campaign has one signed `campaign_id`; every arm attempt has a unique `run_id` plus `replicate_id`, `attempt_index`, and `retry_of_run_id`, propagated through the research pipeline, the eval runner, telemetry, and Tier A rows.
- [ ] **TRIAL-02**: A versioned, immutable expanded campaign preregistration carries hypotheses, arms, metrics, hard gates, validity rules, budgets, statistical tests, stopping rules, and decisions; signing plus immediate verification freezes it, tampering is rejected, and supersession creates a new linked campaign. `PreregistrationV1` is a conversion input only.
- [ ] **TRIAL-03**: Campaign lifecycle Draft → Signed → Running → Completed/Failed/Stopped is enforced; illegal transitions are rejected.
- [ ] **TRIAL-04**: An interrupted attempt is always terminalized and retried under a new linked `run_id`; same-`run_id` resume is not possible in v1.1.
- [ ] **TRIAL-05**: The campaign preregistration gate is called by the production trial runner (proven by a mutation test) and accepts only signers from an allowlisted key set; canonicalization sorts keys at every depth and rejects NaN/non-finite numbers; hashing and signing go through `vox-crypto` (no direct `sha2` import).

### Persistence & Telemetry

- [ ] **STORE-01**: Campaign/run/sample data lives in an active Tier A fragment (new domain per G22, or a scientia extension, as decided at human schema approval) defined by a canonical contract and approved before migration; it migrates at the next free DB baseline (95 as of 2026-10-01), with a guard that fails loudly if a shared DB already carries that version without these tables; quarantined `scientia_prereg` is untouched.
- [ ] **STORE-02**: Each campaign/run/sample batch persists atomically; a forced mid-write failure rolls back the whole unit.
- [ ] **STORE-03**: Tier C evidence-artifact ownership, retention, deletion/tombstones, hashing, encryption/access, and Tier A references are decided and implemented; until then live campaigns are observational only and never replay- or promotion-eligible.
- [ ] **TELEM-01**: Research-trial telemetry is structural only (IDs, hashes, versions, numeric metrics, bounded statuses, stage/tool identifiers, reason codes) and enforced by an allowlist that rejects unknown and content-bearing keys; the allowlist lands before any new producer.
- [ ] **TELEM-02**: One trace context spans every LLM/tool call in a run; LLM turn records carry the real session/run ID (no `anon-session`); research session IDs are unique per attempt and never resolved via `INSERT OR IGNORE` + `last_insert_rowid()`.
- [ ] **METER-01**: Tokens (in/out), provider-reported or estimated cost, latency, real TTFT (or explicit `NotApplicable` for non-streaming), and tool-call/step counts flow from `LlmResponse` through research stages into run results; run duration covers verification, citation audit, and persistence.
- [ ] **SCORE-04**: Evaluator correctness: a no-evidence non-abstaining answer is not counted as abstention; zero-citation answers do not score citation precision 1.0; groundedness is negation-aware and does not auto-pass very short answers; failed queries count as worst score rather than being dropped; no 0.5 imputation for missing gold; `latency_p50_ms` is a true median; harness output validates against its results schema.
- [ ] **TEST-01**: Default tests use deterministic research/executor fixtures and perform no model or external HTTP access; live Crossref/OpenAlex coverage runs only in an explicit ignored/live lane.
- [ ] **BASE-01**: An observational baseline with no policy changes records answer quality, groundedness, citation precision/diversity, retrieval hit rate, latency/TTFT, tokens, estimated cost, tool calls/steps, completion rate, reproducibility, and knowledge-reuse rate.

### Race Harness

- [ ] **RACE-01**: `vox research eval` runs a campaign matrix with bounded concurrency (zero/invalid concurrency rejected), seeds and config snapshots, stage-model attribution, and warm/cold-cache modes; it remains the single CLI surface (run/report/replay modes).
- [ ] **RACE-02**: Arms are isolated from ambient state: campaign/arm/config/model/threshold/context hashes participate in cache identity, and search-policy feedback/domain penalties are disabled or frozen per signed arm.
- [ ] **RACE-03**: Live calls are admitted only through atomic worst-case token/call/cost reservations recorded in a durable vox-db ledger (single conditional update in the run-row transaction; the in-memory fail-open `BudgetManager` is not reused), with worst cases multiplied by cascade fan-out, against frozen campaign pricing; providers without a defensible pre-dispatch liability bound are ineligible; concurrent reservations cannot overspend.
- [ ] **RACE-04**: Preregistered group-sequential batches evaluate stopping boundaries only after admitted batches finish; safety, spend, and wall-time stops are immediate and fail-closed; cancellation and partial failure leave consistent state.

### Scoring & Reports

- [ ] **SCORE-01**: Metrics and evaluator are versioned with defined direction, normalization, missing-data/abstention behavior, cold-start/division-by-zero guards, and weighted views that must sum to 1.00; every scoring-policy change carries a fixed-corpus snapshot.
- [ ] **SCORE-02**: Arms are scored with hard gates plus a Pareto frontier, never a single opaque number; the legacy aggregate `quality_score` is display-only; hallucinated no-evidence answers fail hard gates.
- [ ] **SCORE-03**: Confirmatory metric families apply multiplicity/sequential adjustment; exploratory outcomes may nominate later trials but cannot make the current arm promotion-eligible.
- [ ] **REPORT-01**: Each campaign produces a machine-readable JSON report and a concise CLI summary comparing arms against baseline and the current champion.

### Knowledge Loop

- [ ] **KNOW-01**: Verified findings persist authoritatively in Tier A with provenance, citations, content hashes, campaign/config IDs, confidence, supersession, and expiry/revalidation state; failed arms and negative findings remain searchable.
- [ ] **KNOW-02**: Memory (`MemoryManager::persist_fact`) and search-index writes are acknowledged projections with status, retry, and reconciliation, never proof of the authoritative write; a reconciler compares projected and authoritative content hashes, and campaign fact keys cannot collide at the memory cache cap.
- [ ] **KNOW-03**: Approved authored research is mirrored into `search_documents`/chunks and verified via `vox db retrieval-status` (extended to count documents, chunks, and projection states) and research queries; `docs/src/archive/` is never ingested.
- [ ] **KNOW-04**: Each arm uses a content-addressed knowledge manifest frozen at signing and retrieval scoped to that manifest's allowlist (knowledge-graph FTS lane, `MEMORY.md`, KB entries, and post-signing web cache disabled inside arms); no post-signing finding can enter that campaign except via a preregistered treatment snapshot.
- [ ] **KNOW-05**: Knowledge-reuse impact is measured with four ablations: empty, current, stale/conflicting, and curated verified memory.

### Promotion & Replay

- [ ] **REPLAY-01**: Replay deterministically re-evaluates a campaign offline from governed, content-addressed captured evidence with provider access disabled; a fresh live fetch is a new replication campaign.
- [ ] **PROMO-01**: After each campaign the system generates candidate policy changes from failure clusters and Pareto results and runs them as shadow challengers.
- [ ] **PROMO-02**: Promotion eligibility requires preregistered metrics, no hard-gate regression, concordant adjusted evidence from a nomination campaign and a separately signed holdout confirmation campaign, provenance completeness, and replay success.
- [ ] **PROMO-03**: Champion designation requires two distinct, cryptographically authenticated, policy-authorized humans independent of campaign signing, authorship, execution, and evidence curation; designation never mutates active runtime policy.
- [ ] **PROMO-04**: Champions are scoped by a signed, versioned workload-domain key; exact eligible scope wins over broader champions; changing precedence requires a new signed governance-policy version.
- [ ] **PROMO-05**: Proposal states AwaitingConfirmation → AwaitingApproval → Designated → Suspended/Revoked/EvidenceExpired are enforced; one conflict-free safety principal may suspend immediately; revocation, reinstatement, extension, or scope change requires the dual-control quorum; evidence expiry suspends eligibility without rewriting history.

### Audit & Hardening

- [ ] **AUDIT-01**: Adversarial trials cover hallucinated answers, stale/conflicting memory, provider failure, budget exhaustion, reservation races, and cancellation.
- [ ] **AUDIT-02**: Security/reliability gates (signing, reservations, quorum, frozen manifest, telemetry allowlist) are mutation-tested; each guard's test fails when the guard is removed.
- [ ] **AUDIT-03**: Every campaign emits an evidence bundle: config snapshot, trace/span linkage, samples, scores, confidence, promotion decision, and replay command.
- [ ] **AUDIT-04**: Docs and gates are current: `vox ci data-storage-guard`, telemetry/contract drift checks, docs lint, and `vox ci pre-push --complete` pass.

### Discoverability (inserted phases 06.2–06.6)

- [x] **DISC-01**: Contributor can open one generated capability index (`contracts/capabilities/capability-index.v1.json`) that joins every operations-catalog entry (CLI path + MCP tool) to its GUI surface, doc page, owning crate/layer, skill, and `vox graph` coverage kind.
- [x] **DISC-02**: The index is regenerated by a generator wired into `vox ci ssot-drift`, so a new command, tool, surface, or crate cannot land without appearing in it.
- [ ] **DISC-03**: User can run `vox explain <command|tool|surface|crate>` and `vox features [--domain <d>]` to see what a thing is, its layer, CLI/GUI/MCP equivalents, and docs link — answered from the index.
- [ ] **DISC-04**: `llms.txt` / `vox-docs.json` capability sections are generated from the index (including MCP tool reference and GUI navigation), not hand-kept.
- [ ] **DISC-05**: GUI user can find any index entry from the ⌘K Omnibar — CLI verbs, MCP tools, skills, surfaces, crates, docs, settings, and keybinds — with the index as the palette's single source.
- [ ] **DISC-06**: GUI user can run a found CLI verb or action directly from the palette (opening a prefilled Catalog form when arguments are required).
- [ ] **DISC-07**: GUI user can press `?` to open a shortcut sheet generated from `lib/keybinds.ts`, and every keybind is also a palette action.
- [ ] **DISC-08**: GUI user can browse a Capability Explorer grouped by domain (compiler, orchestrator, mesh, ML, research, secrets, …) showing each capability's status: dedicated surface / generic form / CLI-only / orphan backend.
- [ ] **DISC-09**: The surface classifier credits Catalog generic forms as a `generic_form` tier, so coverage reflects what is actually reachable.
- [ ] **DISC-10**: CI fails when a CLI group or capability has no GUI representation (dedicated or generic) and no exemption with owner + reason.
- [ ] **DISC-11**: Every `vox graph coverage` OrphanBackend / DeadEnd node is dispositioned (surfaced, deleted, or exempted with reason); the undispositioned count is 0 and ratcheted in CI.
- [ ] **DISC-12**: GUI user can navigate an Architecture Atlas of layers L0–L5 and their crates from `layers.toml`, with dependency edges.
- [ ] **DISC-13**: GUI user can select a crate to see its blast radius / community (from `vox graph crate-map`), the capabilities it backs, and links to source and docs.
- [ ] **DISC-14**: Atlas and Explorer cross-link: from a capability to its crate/layer and back.
- [ ] **DISC-15**: Scripted Playwright time-to-find journeys (e.g. "find and run mesh status", "find which crate owns the parser", "find the secrets doctor") each complete from cold start within a recorded interaction budget, capturing review-bundle screenshots.
- [ ] **DISC-16**: The journey suite runs in hosted CI as part of the `ui` lane and fails on budget regression.

## v2 Requirements

Deferred. Tracked but not in the current roadmap.

- **TRIAL-RESUME-01**: Same-`run_id` checkpoint resume, once durable idempotent checkpoints and side-effect accounting are specified and tested.
- **PROMO-APPLY-01**: Governed application of a designated champion to active runtime policy.
- **REPORT-UI-01**: GUI/dashboard presentation of campaigns and scorecards.

### Discoverability (deferred)

- **DISC-v2-01**: Contextual help on every panel (what is this / CLI equivalent / docs) and a first-run feature tour.
- **DISC-v2-02**: Semantic "which capability fits this task" search via the `vox-search` hybrid stack.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Automatic runtime policy mutation | v1.1 promotion is a human-authorized designation only (G16). |
| A second research CLI or parallel trial stack | `vox research eval` is the single surface; reuse existing pipeline, telemetry, eval, routing, retrieval (R3). |
| Reviving `scientia_prereg` | Quarantined; a new Tier A domain replaces it (G22). |
| Content in default telemetry | Queries, answers, URLs, titles, snippets live only in governed evidence storage (G24). |
| Research-trial GUI/dashboard work | Deferred to REPORT-UI-01 (discoverability phases 06.2–06.6 are the only GUI scope this milestone). |
| Physical-GPU runtime validation | Not required by this milestone. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| TRIAL-01 | Phase 7 | Pending |
| TRIAL-02 | Phase 7 | Pending |
| TRIAL-03 | Phase 7 | Pending |
| TRIAL-04 | Phase 7 | Pending |
| STORE-01 | Phase 7 | Pending |
| STORE-02 | Phase 7 | Pending |
| TRIAL-05 | Phase 7 | Pending |
| TELEM-01 | Phase 7 | Pending |
| TELEM-02 | Phase 7 | Pending |
| METER-01 | Phase 7 | Pending |
| SCORE-04 | Phase 7 | Pending |
| TEST-01 | Phase 7 | Pending |
| BASE-01 | Phase 7 | Pending |
| RACE-01 | Phase 8 | Pending |
| RACE-02 | Phase 8 | Pending |
| RACE-03 | Phase 8 | Pending |
| RACE-04 | Phase 8 | Pending |
| SCORE-01 | Phase 8 | Pending |
| SCORE-02 | Phase 8 | Pending |
| SCORE-03 | Phase 8 | Pending |
| REPORT-01 | Phase 8 | Pending |
| STORE-03 | Phase 9 | Pending |
| KNOW-01 | Phase 9 | Pending |
| KNOW-02 | Phase 9 | Pending |
| KNOW-03 | Phase 9 | Pending |
| KNOW-04 | Phase 9 | Pending |
| KNOW-05 | Phase 9 | Pending |
| REPLAY-01 | Phase 10 | Pending |
| PROMO-01 | Phase 10 | Pending |
| PROMO-02 | Phase 10 | Pending |
| PROMO-03 | Phase 10 | Pending |
| PROMO-04 | Phase 10 | Pending |
| PROMO-05 | Phase 10 | Pending |
| AUDIT-01 | Phase 11 | Pending |
| AUDIT-02 | Phase 11 | Pending |
| AUDIT-03 | Phase 11 | Pending |
| AUDIT-04 | Phase 11 | Pending |
| DISC-01 | Phase 06.2 | Complete |
| DISC-02 | Phase 06.2 | Complete |
| DISC-03 | Phase 06.2 | Pending |
| DISC-04 | Phase 06.2 | Pending |
| DISC-05 | Phase 06.3 | Pending |
| DISC-06 | Phase 06.3 | Pending |
| DISC-07 | Phase 06.3 | Pending |
| DISC-08 | Phase 06.4 | Pending |
| DISC-09 | Phase 06.4 | Pending |
| DISC-10 | Phase 06.4 | Pending |
| DISC-11 | Phase 06.4 | Pending |
| DISC-12 | Phase 06.5 | Pending |
| DISC-13 | Phase 06.5 | Pending |
| DISC-14 | Phase 06.5 | Pending |
| DISC-15 | Phase 06.6 | Pending |
| DISC-16 | Phase 06.6 | Pending |

**Coverage:**

- v1.1 requirements: 53 total (37 flywheel + 16 discoverability)
- Mapped to phases: 53
- Unmapped: 0 ✓

---
*Requirements defined: 2026-10-01; reconciled with .planning/research/ the same day*
*Last updated: 2026-10-08 after inserting discoverability phases 06.2–06.6*
