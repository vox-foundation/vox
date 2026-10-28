---
phase: "1"
slug: "retrieval-eval-baseline"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-03"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: 01-RESEARCH.md "## Validation Architecture", mapped onto plans 01-01 … 01-12. Revision 1 split 01-12 (CLI SSOT registration + docs) out of 01-06 and trimmed the 01-01 fixture to the dev split.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo nextest 0.9.143 over built-in `#[test]` / `#[tokio::test]` (Rust 1.98.1, via the build broker: plain `cargo` on PATH) |
| **Config file** | `.config/nextest.toml` (existing) |
| **Quick run command** | `cargo nextest run -p vox-search --lib kb_` |
| **Full suite command** | `cargo nextest run -p vox-search -p vox-cli-research -p vox-db` then `cargo nextest run -p vox-cli --lib commands::kb pre_push::tests` |
| **Estimated runtime** | ~60-90 s quick (incremental vox-search); ~6-10 min full (vox-cli test binary compile dominates) |

Local verification is per crate only. Never run workspace-wide clippy or tests (AGENTS.md "Hosted CI is the gate"). Never run `cargo fmt --all`; use `vox run scripts/fmt.vox`.

---

## Sampling Rate

- **After every task commit:** run that task's `<automated>` command(s), which are per-crate and filtered (see map). Every filter names tests that exist; "0 tests run" is a failure.
- **After every plan wave:** run the full suite command, plus `./target/debug/vox kb eval --gate` once the golden set and baseline exist (wave 5).
- **Before `/gsd-verify-work`:** full suite green; the two-process `cmp` (text and `--json`) identical; `vox ci pre-push --dry-run` lists `vox kb eval --gate`; median gate time ≤ 10 s in debug, or a recorded deviation.
- **Max feedback latency:** 120 s for vox-search/vox-db/vox-cli-research tasks. vox-cli tasks accept the test-binary compile cost.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | EVAL-01 | T-01-02, T-01-05 | Hand fixture exact (0.333333/0.666667/0.380952/0.364507); no GUID literal; ordered collections only | unit + integration (tracer) | `cargo nextest run -p vox-search --lib kb_` ; `cargo nextest run -p vox-search --test kb_eval_determinism` | ❌ W0 (created test-first in task) | ⬜ pending |
| 01-01-02 | 01 | 1 | EVAL-01 | T-01-05 | answer_in_budget prefix semantics; deepest-ancestor chunk resolution; dedupe; byte-identical JSON | unit + integration | `cargo nextest run -p vox-search --lib kb_eval` ; `--test kb_eval_determinism` | ❌ W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | EVAL-01 (EVAL-02 support) | T-01-01, T-01-03, T-01-04 | Strict BEIR validation; Dev never opens test qrels; validate_splits (tempdir with both splits; the fixture is dev-only); path filter | unit | `cargo nextest run -p vox-search --lib kb_eval::golden` | ❌ W0 | ⬜ pending |
| 01-02-01 | 02 | 1 | EVAL-05 | T-01-06 | Word-overlap helpers gone from vox-search; crate-private in vox-cli-research | unit + grep | `cargo nextest run -p vox-cli-research answer_overlap` ; `bash -c '! grep -rnE "calculate_recall_at_5\|calculate_groundedness" crates/'` | ✅ (tests moved) | ⬜ pending |
| 01-02-02 | 02 | 1 | EVAL-05 | T-01-07 | Honest report key | grep + json | `jq empty contracts/reports/research-eval/results.v1.schema.json contracts/reports/research-eval/baseline.v1.json` | ✅ | ⬜ pending |
| 01-03-01 | 03 | 1 | DOC-01 (S1 → IDX-07) | T-01-08 | Turso FTS behaviour pinned; red on drift | integration (characterization) | `cargo nextest run -p vox-db --test turso_fts_probe` | ❌ W0 | ⬜ pending |
| 01-03-02 | 03 | 1 | DOC-01 | T-01-09 | Spike record lint-clean | doc lint | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture/knowledge-base-practices-research-2026.md` | ✅ | ⬜ pending |
| 01-04-01 | 04 | 1 | EVAL-02 | T-01-11 | One-way snapshot form chosen by the user | checkpoint:decision | — (SUMMARY `snapshot_decision:` line) | n/a | ⬜ pending |
| 01-04-02 | 04 | 1 | EVAL-03 | T-01-02 | Exactly one lowercase canary line; GUID absent from draft | shell | canary-line count = 1; GUID-absence `grep -qF` (see plan) | ❌ W0 | ⬜ pending |
| 01-04-03 | 04 | 1 | EVAL-02 | T-01-10 | ≥40 human-written queries | checkpoint:human-action | `grep -cE '^- \[(where\|how-to\|…)\] .{10,}' golden-draft/queries.md` ≥ 40 | n/a | ⬜ pending |
| 01-05-01 | 05 | 2 | EVAL-04 | T-01-12 | Tighten-only matrix; ratchet_split = Dev; mutation-proven | unit | `cargo nextest run -p vox-search --lib kb_eval::baseline` | ❌ W0 | ⬜ pending |
| 01-05-02 | 05 | 2 | EVAL-03 | T-01-02 | Planted GUID detected then cleared (before/after presence); root overlap rejected | unit (mutation) | `cargo nextest run -p vox-search --lib kb_eval::canary` | ❌ W0 | ⬜ pending |
| 01-05-03 | 05 | 2 | EVAL-04 | T-01-12 | Degraded retriever fails regression on fixture | integration | `cargo nextest run -p vox-search --test kb_eval_gate` | ❌ W0 | ⬜ pending |
| 01-06-01 | 06 | 2 | EVAL-01 | T-01-14 | No raw git; report/query/headings/snapshot modes | unit (clap + fixture) | `cargo nextest run -p vox-cli --lib commands::kb` | ❌ W0 | ⬜ pending |
| 01-06-02 | 06 | 2 | EVAL-02 | T-01-02, T-01-15 | Snapshot deterministic, GUID-free, ignored for AI context | shell | `wc -l`/`jq -e ._id` on corpus.jsonl; GUID-absence; attribute/ignore greps | ❌ (created) | ⬜ pending |
| 01-12-01 | 12 | 3 | EVAL-01 | T-01-23 | CLI SSOT chain green; generated docs untouched | CI command | `cargo run -q -p vox-cli -- ci command-compliance` ; `cargo run -q -p vox-cli -- ci operations-verify` | ✅ | ⬜ pending |
| 01-12-02 | 12 | 3 | EVAL-01 | — | where-things-live row; docs lint-clean | doc lint | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths reference/cli.md,architecture/where-things-live.md` | ✅ | ⬜ pending |
| 01-07-01 | 07 | 4 | EVAL-04 | T-01-12, T-01-02 | Ratchet modes refuse `--split`; tighten never lowers; index build rejects GUID corpus | unit | `cargo nextest run -p vox-cli --lib commands::kb` | ❌ W0 | ⬜ pending |
| 01-07-02 | 07 | 4 | EVAL-03, EVAL-04 | T-01-02, T-01-12, T-01-16 | MENS extractor planted test; gate fails closed on unresolvable base; gate flags documented | unit (mutation) + CLI + doc lint | `cargo nextest run -p vox-cli --lib commands::kb::gate` ; fail-closed `--base refs/heads/definitely-missing-kb-eval` must exit ≠ 0 with "cannot resolve base ref" ; `cargo run -q -p vox-doc-pipeline -- --lint-only --paths reference/cli.md` | ❌ W0 | ⬜ pending |
| 01-08-01 | 08 | 3 | EVAL-02 | T-01-18 | Pooled proposals; all chunk ids valid | shell | block count = ceil(n/2); `--headings` validation loop prints no `BAD` | ❌ (created) | ⬜ pending |
| 01-08-02 | 08 | 3 | EVAL-02 | T-01-19 | No pre-approved `keep` | shell | ≥15 paraphrase blocks; `--headings` loop | ❌ (created) | ⬜ pending |
| 01-09-01 | 09 | 3 | EVAL-02 | T-01-18 | Pooled proposals; no id overlap with half A | shell | block count = n − ceil(n/2); `--headings` loop | ❌ (created) | ⬜ pending |
| 01-09-02 | 09 | 3 | EVAL-02 | T-01-19, T-01-20 | Gap questions tagged, no pre-approved `keep` | shell | ≥20 candidate blocks; `--headings` loop | ❌ (created) | ⬜ pending |
| 01-10-01 | 10 | 4 | EVAL-02 | T-01-18 | Only signed, approved rows | checkpoint:human-action | `approved_by:` non-empty ×4; ≥40 human q blocks | n/a | ⬜ pending |
| 01-10-02 | 10 | 4 | EVAL-02 | T-01-02, T-01-03 | BEIR set contract: ≥40 human, ≥1 expansion, disjoint/covering splits, 20-35% test, canary on every line; drafts deleted | integration | `cargo nextest run -p vox-search --test kb_eval_golden_contract` | ❌ W0 | ⬜ pending |
| 01-11-01 | 11 | 5 | EVAL-01, EVAL-04 | T-01-12 | Real retriever meets baseline; degraded fails; two-process byte-identical | integration + smoke | `cargo nextest run -p vox-search --test kb_eval_golden_contract` ; two-process `cmp` (text + `--json`) ; `./target/debug/vox kb eval --gate` | ❌ W0 | ⬜ pending |
| 01-11-02 | 11 | 5 | EVAL-03, EVAL-04 | T-01-02, T-01-21, T-01-13 | Gate in fast tier; mutations a-d red then green | unit + smoke + manual mutation | `cargo nextest run -p vox-cli --lib pre_push::tests::prepush_base_ref` ; `./target/debug/vox ci pre-push --dry-run` lists the step ; `./target/debug/vox kb eval --gate` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Test files are created test-first inside their owning task (the failing test is committed with or before the implementation), so the phase has no separate Wave-0 plan. They are:

- [ ] `crates/vox-search/src/kb_eval/metrics.rs` `hand_fixture_three_queries` (01-01 Task 1, before the metric impls)
- [ ] `crates/vox-search/tests/kb_eval_determinism.rs` with `mod common;` + `tests/fixtures/kb_eval/` (01-01)
- [ ] `crates/vox-db/tests/turso_fts_probe.rs` (01-03)
- [ ] `kb_eval::baseline::tests` tighten-only matrix and `kb_eval::canary::tests` planted-GUID tests (01-05)
- [ ] `crates/vox-search/tests/kb_eval_gate.rs` (01-05)
- [ ] `commands::kb::tests` / `commands::kb::eval::tests` / `commands::kb::gate::tests` (01-06, 01-07)
- [ ] `crates/vox-search/tests/kb_eval_golden_contract.rs` (01-10, extended in 01-11)

Framework already installed (nextest 0.9.143); no installs.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Snapshot form chosen (frozen vs live) | EVAL-02 | One-way decision (git-history blob), the user's call | 01-04 Task 1 checkpoint:decision; SUMMARY `snapshot_decision:` |
| ≥40 seed questions are human-written | EVAL-02 | Authorship cannot be automated by definition (AP-02) | 01-04 Task 3; agent counts lines afterwards |
| Qrels approved, expansion pruned | EVAL-02 | Ground truth requires human judgment | 01-10 Task 1; agent checks signatures and row completeness |
| Docs-only PR runs the gate in hosted CI | EVAL-04 | Needs a real PR run | After push, confirm the `linux` job log of a docs-only PR shows the "vox kb eval --gate" step line |
| Gate median ≤ 10 s in debug | EVAL-04 (AP-05) | Machine-dependent timing | 01-11 Task 2: `/usr/bin/time -p ./target/debug/vox kb eval --gate` ×3, record the median; >10 s means stop and surface |

---

## Mutation Proofs Required (AGENTS.md "verify guards by mutation")

Each proof has a before/after presence check, and each is restored before any commit.

| Guard | Mutation | Expected | Plan |
|-------|----------|----------|------|
| nDCG metric | discount → `1/log2(rank+2)` | hand fixture red | 01-01 |
| R4 absence gate | re-add a stub with the old name | grep gate red | 01-02 |
| Turso FTS probe | flip an expectation to Ok | test red | 01-03 |
| tighten-only | invert comparison | `tighten_only_rejects_lowered_metric` red | 01-05 |
| repo scan | stub scan to empty | planted test red | 01-05 |
| MENS extractor scan | stub hits to empty | planted test red | 01-07 |
| canary (real repo) | plant README canary line in a tracked prose doc | gate red ([canary] repo scan + MENS) | 01-11 |
| regression (real repo) | raise a baseline value +0.05 | gate red ([regression]) | 01-11 |
| tighten-only (real repo) | lower a value −0.05, `--base HEAD` | gate red ([tighten-only]) | 01-11 |
| fast-tier wiring | remove the OwnedStep | `--dry-run` lacks the label | 01-11 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (checkpoint tasks verified by the agent's post-checks listed above)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (test files created test-first within tasks; confirm during execution)
- [x] No watch-mode flags
- [x] Feedback latency < 120 s for non-vox-cli tasks
- [ ] `nyquist_compliant: true` set in frontmatter (set by validate-phase)

**Approval:** pending
