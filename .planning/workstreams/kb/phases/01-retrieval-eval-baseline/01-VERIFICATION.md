---
phase: 01-retrieval-eval-baseline
verified: 2026-10-05T08:00:00Z
status: passed
score: 5/5 must-haves verified (SC2 against the D-02-amended requirement; human items resolved 2026-10-05 under user delegation — see "Resolution of human items")
covered_files:
  - ".planning/workstreams/kb/REQUIREMENTS.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-01-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-01-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-02-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-02-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-03-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-03-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-04-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-04-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-05-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-05-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-06-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-06-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-07-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-07-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-08-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-08-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-09-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-09-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-10-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-10-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-11-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-11-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-12-PLAN.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-12-SUMMARY.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-CONTEXT.md"
  - ".planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-REVIEW-FIX.md"
  - "contracts/kb/eval/README.md"
  - "contracts/kb/eval/queries.jsonl"
  - "contracts/kb/retrieval-baseline.v1.json"
  - "crates/vox-cli-research/src/answer_overlap.rs"
  - "crates/vox-cli/src/commands/kb/eval.rs"
  - "crates/vox-cli/src/commands/kb/gate.rs"
  - "crates/vox-search/src/kb_eval/baseline.rs"
  - "crates/vox-search/src/kb_eval/canary.rs"
  - "crates/vox-search/src/kb_eval/metrics.rs"
covered_digest: "v1:sha256:c82aee7ff716b79e8114cd4bf63c17116da339a56d924c7818edbb478721cf01"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Ratify the SC2 amendment (agent-seed in place of human) and decide whether to source real human queries."
    expected: "Maintainer accepts that the golden set holds 0 human-authored queries and 48 agent-seed queries, and either updates ROADMAP SC2 to the amended wording or schedules a human-authored re-epoch."
    why_human: "ROADMAP.md SC2 still literally reads 'at least 40 queries tagged human-authored'. The amendment lives in REQUIREMENTS.md EVAL-02 and 01-CONTEXT D-02 only. Whether an agent-authored seed set is an acceptable baseline for every later phase is a product decision."
  - test: "Review the WR-05 ratchet-semantics change (01-REVIEW-FIX.md): a retriever/chunker/budget/estimator change now licenses a re-epoch under the same rule as a golden-set change."
    expected: "Maintainer confirms the re-epoch licence is not too permissive (epoch bump plus a new non-empty epoch_note, human-reviewed via CODEOWNERS /contracts/)."
    why_human: "The fix agent flagged this itself as requiring human verification. It changes what the tighten-only gate allows."
  - test: "Confirm the first PR containing contracts/kb/retrieval-baseline.v1.json behaves as intended on hosted CI."
    expected: "origin/main has no baseline, so the tighten-only check bootstraps (proven absence via ls-tree). The gate should pass and subsequent PRs should be compared against the merged baseline."
    why_human: "Cannot run hosted CI locally. Locally the gate passes with base origin/main (bootstrap path)."
---

# Phase 1: Retrieval Eval Baseline - Verification Report

**Phase Goal:** A maintainer can measure retrieval quality deterministically, and any retrieval regression blocks a PR. Every later phase reports against this baseline.
**Verified:** 2026-10-05
**Status:** human_needed
**Re-verification:** No (initial verification)

## Goal Achievement

The goal is achieved in the codebase. I ran the real commands rather than reading SUMMARY.md. Everything mechanical passes; the open items are about honesty of labelling and ratification, not about missing behaviour.

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `vox kb eval` twice on one commit gives byte-identical Recall@5/10, MRR@10, nDCG@10 (doc and chunk) and answer-in-budget; a hand-computed 3-query fixture reproduces exact values | VERIFIED | Two runs of `./target/debug/vox kb eval` (text) have the same SHA-1 (`a90014a3...`); two `--json` runs have the same SHA-1 (`762b8c94...`). Output has doc and chunk blocks with recall@5/10, mrr@10, ndcg@10 and `answer_in_budget`. `metrics.rs::hand_fixture_three_queries` passes; I recomputed it by hand: q1 DCG=2, IDCG=2+1/log2(3)=2.6309, nDCG=0.760188; q2 MRR=1/7, nDCG=1/log2(8)=0.3333; mean Recall@5=0.333333, Recall@10=0.666667, MRR=0.380952, nDCG=0.364507. All match the asserted constants. Integration test `kb_eval_determinism::fixture_eval_is_byte_identical` passes. |
| 2 | `contracts/kb/eval/` is BEIR layout, with >=40 seed queries, a pruned LLM expansion tagged as such, and a held-out split no tuning command reads | VERIFIED against the amended requirement; see human item 1 | Layout: `corpus.jsonl`, `queries.jsonl`, `qrels/{dev,test}{,.chunks}.tsv`. `queries.jsonl` has 102 queries: **48 `agent-seed`, 54 `llm-expansion`, 0 `human`**. 48 >= 40 under seed = human U agent-seed (EVAL-02 as amended). Test is held out: only `--split test` reads it (and `run_report` warns "never tune on it"). The gate, `--tighten` and `--write-baseline` read dev only; baseline holds dev metrics only. `kb_eval_golden_contract::{golden_set_has_40_seed_queries, splits_disjoint_and_covering, test_fraction_in_range}` pass. The literal ROADMAP wording ("tagged human-authored") is NOT met: there are no human queries. |
| 3 | A canary check fails CI when the golden set appears under an indexer root or in MENS corpus build output, proven by mutation | VERIFIED | My own mutation: appended the canary GUID (read into a shell variable, never printed) to a tracked doc, `docs/src/architecture/knowledge-base-practices-research-2026.md`. `vox kb eval --gate` failed: `[canary] repo-scan: <that path> contains the canary GUID`, "1 failure(s)". File restored with `git checkout`; tree clean afterwards. The MENS extractor scan is covered by `gate::tests::mens_extract_scan_flags_planted_guid_then_clears`; the full sweep by `canary_sweep_names_every_source_and_never_the_guid`. WR-02 (case-insensitive and UTF-16 scanning) and WR-08 (query-fingerprint scan over all tracked files) are in `canary.rs` and `gate.rs`. |
| 4 | PR CI compares metrics to `contracts/kb/retrieval-baseline.v1.json`; a degrading fixture PR fails; a baseline edit that loosens any value is rejected (tighten-only) | VERIFIED | Wiring: `.github/workflows/ci.yml` `linux` job (fetch-depth 0) runs `./target/debug/vox ci pre-push`, whose step list includes `vox kb eval --gate` (`pre_push.rs:523-527`, `step_kb_eval_gate` -> `gate::run_gate`). Degradation: `kb_eval_gate::{drop_top_hit_fails_regression, reversed_ranking_fails_regression}` and `kb_eval_golden_contract::degraded_retriever_fails_real_baseline` pass. Live loosen mutation (`--gate --base HEAD`): lowering `dev/all doc recall@5` 0.469737 -> 0.40 gave `[tighten-only] dev/all doc recall@5 lowered 0.469737 -> 0.400000` and exit non-zero. Raising it to 0.50 gave `[regression] ... drop beyond tolerance 0.010000`. Baseline restored, tree clean. `--gate` on the real repo: `OK (10846 tracked files scanned)`, 5.5 s. A missing/unresolvable base ref fails closed (`resolve_base_rejects_missing_ref`, `baseline_at_bootstraps_only_on_proven_absence`). |
| 5 | No word-overlap helper in `vox-search/src/evaluation.rs` carries an IR metric name | VERIFIED | `evaluation.rs` now holds only `SearchBenchmarkQuery`, `default_doc_nav_queries` and the intent-match report; a grep for `recall_at\|mrr\|ndcg\|precision_at` finds nothing. Helpers moved to `crates/vox-cli-research/src/answer_overlap.rs` as `answer_token_overlap` (pub(crate), used only in `eval.rs`); `citation_recall` is gone workspace-wide. IR metrics live only in `vox-search/src/kb_eval/metrics.rs`. |

**Score:** 5/5 truths verified (SC2 under the D-02 amendment); 0 behavior-unverified.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-search/src/kb_eval/{metrics,baseline,canary,golden,bm25}.rs`, `kb_ids.rs` | VERIFIED | Substantive, wired into `vox kb eval` and the gate; 52 `kb_` tests pass |
| `crates/vox-cli/src/commands/kb/{eval,gate}.rs` | VERIFIED | CLI wired in the command registry and `pre_push.rs` |
| `contracts/kb/eval/{README.md,corpus.jsonl,queries.jsonl,qrels/*}` | VERIFIED | Present, loaded by tests and the live CLI |
| `contracts/kb/retrieval-baseline.v1.json` | VERIFIED | Epoch 1, tolerance 0.01; metrics equal the live `vox kb eval --json` dev report |
| `docs/src/architecture/knowledge-base-practices-research-2026.md` (DOC-01) | VERIFIED | Present, with S1/S2 spike outcomes (line 231+); `kb-systems-sota-research-2026.md` carries `status: legacy` and a "Superseded by" banner |
| `docs/src/reference/cli.md` `vox kb eval` section, `where-things-live.md` row, `contracts/cli/command-registry.yaml`, `contracts/operations/catalog.v1.yaml` | VERIFIED | Registered and documented |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| `.github/workflows/ci.yml` `linux` job | `vox ci pre-push` | WIRED | Step "Fast tier"; checkout has `fetch-depth: 0` so `origin/main` resolves |
| `pre_push.rs` | `kb::gate::run_gate` | WIRED | `step_kb_eval_gate` uses `prepush_base_ref()` |
| `gate_failures` | `validate_splits`, canary sweep, tighten-only, regression | WIRED | Each proven by a test or a live mutation above |
| `kb eval` | frozen `corpus.jsonl` snapshot | WIRED | No live-docs dependence; determinism holds |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|----------|------|--------|-----------|--------|
| `vox kb eval` report | per-key metrics | real BM25 index over the 670-doc snapshot, real qrels | Yes (non-trivial values, differing per origin) | FLOWING |
| Baseline | metrics | equal to the live dev report to 6 dp | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Build | `cargo build -p vox-cli` | finished | PASS |
| Determinism (text) | `vox kb eval` x2, shasum | identical | PASS |
| Determinism (JSON) | `vox kb eval --json` x2, shasum | identical | PASS |
| Gate | `vox kb eval --gate` (and `--base origin/main`) | OK, exit 0, 5.5 s | PASS |
| Canary mutation | plant GUID in tracked doc, run gate | `[canary] repo-scan` failure | PASS |
| Loosen mutation | lower baseline metric, `--gate --base HEAD` | `[tighten-only]` failure | PASS |
| Regression mutation | raise baseline metric, `--gate --base HEAD` | `[regression]` failure | PASS |
| `cargo nextest run -p vox-search kb_` | 52 run, 52 passed | PASS |
| `cargo nextest run -p vox-search --test kb_eval_determinism --test kb_eval_gate --test kb_eval_golden_contract` | 13 run, 13 passed | PASS |
| `cargo nextest run -p vox-cli --lib commands::kb` | 24 run, 24 passed | PASS |

Run summary: no step failed, and no mutation was left behind (`git status` shows only the pre-existing untracked `milestone.lock`).

### Probe Execution

SKIPPED: the phase declares no `probe-*.sh` scripts.

### Requirements Coverage

| Requirement | Source plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DOC-01 | 01-03 | SATISFIED | SSOT present; old doc marked superseded; S1/S2 outcomes recorded; `turso_fts_probe.rs` executable half |
| EVAL-01 | 01-01, 01-06, 01-11, 01-12 | SATISFIED | SC1 evidence |
| EVAL-02 | 01-04, 01-06, 01-08, 01-09, 01-10 | SATISFIED as amended (agent-seed) | SC2; REQUIREMENTS.md EVAL-02 records the amendment (2026-10-04, 01-CONTEXT D-02) and marks it with "amended" |
| EVAL-03 | 01-04, 01-05, 01-07, 01-11 | SATISFIED | SC3 mutation |
| EVAL-04 | 01-05, 01-07, 01-11 | SATISFIED | SC4 |
| EVAL-05 | 01-02 | SATISFIED | SC5 |

All six requirement IDs from the phase (DOC-01, EVAL-01..05) appear in at least one PLAN `requirements:` field and in REQUIREMENTS.md. No orphaned requirements are mapped to Phase 1.

### Post-plan fixes accounted for

- **agent-seed origin / D-02:** Recorded in 01-CONTEXT D-02, REQUIREMENTS.md EVAL-02, README `origin` table ("never human") and the baseline `epoch_note`. Every query's `metadata.author`/`approved_by` names the agent and the delegation. Agent-seed is not presented as human anywhere I checked, except the README line below.
- **Paraphrase split fix (`5fc04c3ea`):** README states a paraphrase follows its seed's split; `validate_splits` enforces it and the gate runs it (WR-04). Tests `gate_reports_split_and_paraphrase_violations_as_golden` and the golden-contract split tests pass.
- **01-REVIEW-FIX.md:** all 13 in-scope findings (WR-01..08, IN-01/03/04/06/07) claim fixed. I confirmed the behaviour of WR-01 (tighten-only fail-closed), WR-02 (case/UTF-16), WR-04 (split wiring), WR-05 (re-epoch licence in `baseline.rs`) and IN-07 (canary redacted from `Debug`) through the passing tests and live gate runs. The fix report's claim "baseline unchanged, no re-epoch" holds: the live dev report equals the baseline. The deferred Info items (IN-02, 05, 08..11) are real but non-blocking and do not touch a success criterion.

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `contracts/kb/eval/README.md` line 47 | Says `llm-expansion` is "agent-generated, human-pruned". Query metadata and 01-10-SUMMARY show every expansion query was approved by `agent-reviewer (Claude Opus, delegated ...)`; no human pruned them. | WARNING | Mislabels provenance in the contract file, the same class of problem D-02 set out to avoid. Fix the README (or record a real human pass). |
| `.planning/workstreams/kb/ROADMAP.md` SC2 | Text still says "tagged human-authored" and was never amended | WARNING | The roadmap contract disagrees with REQUIREMENTS.md and CONTEXT. Fold the amendment into the roadmap. |
| `crates/vox-cli-research/src/eval.rs`, `vox-db*` | Field `recall_at_5` still holds the token-overlap value, and `answer_token_overlap` is stored in it (IN-08, deferred) | INFO | Outside SC5 scope (it names `vox-search/src/evaluation.rs`), the helper itself is renamed, and no gate reads it. It is still an IR-named field carrying a non-IR value. |
| TBD/FIXME/XXX scan on phase files | Not individually grepped across all modified files | INFO | No blocker surfaced during review or tests; the gate and clippy were clean per the fix report. |

### Human Verification Required

1. **Ratify the agent-seed amendment (SC2).** Test: read `queries.jsonl` provenance (0 human, 48 agent-seed, 54 llm-expansion) against EVAL-02 as amended. Expected: you accept it as the baseline, update ROADMAP SC2, and decide when to add real human queries via a reviewed re-epoch. Why human: product call; the original success criterion is not literally met.
2. **Review WR-05 re-epoch semantics.** Test: read `check_tighten_only` in `baseline.rs` and its tests. Expected: a config change plus an epoch bump and a new note is the right licence. Why human: it changes what the tighten-only ratchet permits, and the fixer flagged it.
3. **First-PR bootstrap on hosted CI.** Test: open the PR and watch the `linux` job's fast tier. Expected: gate passes via proven-absence bootstrap (`origin/main` has no baseline today). Why human: hosted CI cannot be run from here.

### Gaps Summary

No blocking gaps. All five success criteria hold when exercised, including three live mutations that I reverted. Status is `human_needed` rather than `passed` for the three items above, chiefly because the golden set contains no human-authored queries and the ROADMAP/README wording still overstates provenance (SC2 and the "human-pruned" claim).

---

_Verified: 2026-10-05_
_Verifier: Claude (gsd-verifier)_

## Resolution of human items (2026-10-05)

The user delegated these decisions ("solve these yourself"); resolved by the orchestrator:

1. **SC2 wording** — ROADMAP Phase 1 SC2 amended to match EVAL-02 / 01-CONTEXT D-02 (≥40 seed = human ∪ agent-seed). The golden set honestly has 0 human queries; real human queries may be added later via a reviewed re-epoch.
2. **WR-05 re-epoch semantics** — ratified: a change to retriever, chunker, budget or token estimator licenses a reviewed re-epoch (epoch bump + new note); otherwise config must stay equal.
3. **First-PR bootstrap** — cannot be exercised locally; carried as an explicit PR checklist item: confirm the hosted `linux` job runs `vox kb eval --gate` green via the proven-absence bootstrap path.

Also fixed: `contracts/kb/eval/README.md` no longer calls the expansion "human-pruned" (approved by an independent agent reviewer under delegation).
