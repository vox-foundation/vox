# Research Summary: v1.1 Research Trial Flywheel

**Researched:** 2026-10-01
**Inputs:** [TRIAL-DESIGN.md](TRIAL-DESIGN.md), [QUALITY-EVAL.md](QUALITY-EVAL.md), [OBSERVABILITY.md](OBSERVABILITY.md), [KNOWLEDGE.md](KNOWLEDGE.md)

## Bottom line

The building blocks exist (preregistration signing, the claim verifier and citation audit, `pareto.rs` dominance, `vox-eval` statistics, the LLM response metrics, and the search stack), but none of them is wired into a trial loop, and several are measurably wrong today. A baseline captured on the current tree would be untrustworthy, so Phase 7 must fix identity, metering, and evaluator correctness *before* recording the baseline.

## Defects that change scope

| Defect | Evidence | Requirement |
|--------|----------|-------------|
| Prereg gate has no production caller and accepts any embedded signing key | TRIAL-DESIGN §1 | TRIAL-05 |
| `trusty_uri.rs` imports `sha2` directly; canonicalization sorts only top-level keys | TRIAL-DESIGN §1 | TRIAL-05 |
| Session IDs collide (`INSERT OR IGNORE` + `last_insert_rowid()`); fresh `trace_id` per LLM call; `anon-session` turn records | OBSERVABILITY §2, TRIAL-DESIGN §2 | TELEM-02 |
| No telemetry key allowlist; bridge contract allows extra properties | OBSERVABILITY §1 | TELEM-01 |
| Tokens/cost/TTFT/tool calls dropped in research stages; run duration undercounts | OBSERVABILITY §3, QUALITY-EVAL §5 | METER-01 |
| No-evidence answers score as abstentions (precision 1.0); zero-citation answers score 1.0; groundedness negation-blind; schema-violating output | QUALITY-EVAL §1 | SCORE-04 |
| Result cache, verdict cache, and learned search policy leak across arms | TRIAL-DESIGN §2, OBSERVABILITY §4 | RACE-02 |
| `BudgetManager` is fail-open, tokens-only, in-memory, double-counts | TRIAL-DESIGN §3 | RACE-03 |
| `persist_fact` is fire-and-forget; lookup scans 500 rows; campaign keys collide at cache cap | KNOWLEDGE §1 | KNOW-02 |
| `ingest_markdown_tree` would ingest `docs/src/archive/`; chunk search has no scope filter | KNOWLEDGE §2 | KNOW-03, KNOW-04 |
| `vox-spool` and a Tier C blob store do not exist; the only CAS stores blobs inline | OBSERVABILITY §4 | STORE-03 |

## Recommended approach

- **Statistics:** paired sign-flip test per query, Lan–DeMets incremental alpha spending, Holm across arms, fixed-sequence across metrics, Wilson and seeded (`rand_chacha`) bootstrap intervals; failed attempts count as worst score. About 300 LoC, no heavy dependencies.
- **Scoring:** `research-quality-metrics.v1` (18 directed metrics, explicit NotApplicable/Missing/Abstained), then validity, hard gates, a CI-conservative N-axis Pareto frontier, and Holm-corrected confirmatory tests. Weighted views are display-only.
- **Spend:** durable vox-db reservation ledger with one conditional update inside the run-row transaction; worst cases multiplied by cascade fan-out.
- **Knowledge:** authoritative scientia findings, citations, append-only status events, a projection-state ledger, and signed per-arm manifests with allowlist-scoped retrieval.
- **Hermetic tests:** frozen archetype outputs scored with recorded judge verdicts; about 45 red tests across the four files.

## Decisions needed from the user

1. **Tier A placement:** new active domain (ruling G22) or scientia extension (KNOWLEDGE recommendation), at baseline 95 with a shared-DB stamping guard.
2. **Tier C evidence storage:** owner, hash, retention/tombstones, encryption/access, reference shape, and whether trial events use the metrics table now or wait for `vox-spool`.
3. **New crate:** a dedicated trial-engine crate (`vox-research-shim` is ~11.4k LoC against an 8k budget) needs a layer assignment and crate-edge exceptions.
4. **Crate edge:** `vox-cli-research` → `vox-eval` for the statistics and gate template.

Items 3 and 4 are USER-AUTHORIZED-ONLY under AGENTS.md §Dependency Discipline.
