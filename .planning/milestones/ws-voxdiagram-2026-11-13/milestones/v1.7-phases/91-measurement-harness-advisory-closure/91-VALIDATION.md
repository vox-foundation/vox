---
phase: "91"
slug: "measurement-harness-advisory-closure"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-27"
---

# Phase 91 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: cargo-nextest (vox-eval unit tests; vox-cli lib unit tests under `commands/model/eval_diagram/`; vox-cli merged integration binary `tests/it` modules `eval_diagram_rescore`, `eval_diagram_tasks`, `eval_diagram_live` (ignored)). Node: `node:test` in docs-astro (`tests/unit/diagram-authoring-rescore.test.mjs`, `tests/unit/mermaid-oracle.test.mjs`) over the pinned mermaid 11.17.2 + jsdom. Doc gates: `vox-doc-pipeline --lint-only`, `vox ci check-links`. No new dependency |
| **Config file** | `.config/nextest.toml` (default profile kills a test after 180 s, so the live run uses `cargo test`); no new config |
| **Quick run command** | `timeout 3600 cargo nextest run -p vox-cli --lib --test it --retries 0 -E 'test(/eval_diagram/)'` ; `timeout 1800 cargo nextest run -p vox-eval --retries 0 -E 'test(/paired_ni/)'` ; `timeout 600 pnpm --dir docs-astro exec node --test tests/unit/diagram-authoring-rescore.test.mjs` |
| **Full suite command** | the quick commands plus `timeout 600 pnpm --dir docs-astro exec node --test tests/unit/mermaid-oracle.test.mjs`, `timeout 1200 cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture/voxdiagram-authoring-reliability-findings-2026.md,architecture/diagramming-and-mermaid-evaluation-advisory-2026.md` and `timeout 900 target/debug/vox ci check-links` |
| **Estimated runtime** | vox-eval filter seconds once built; vox-cli `it` + lib filter seconds once built (first build of the vox-cli test binaries can take tens of minutes on this machine: background it); Node tests ~2-5 s (0.8 s mermaid load + ~1.2 ms per parse); live run 1-4 h (manual, 91-05 only) |

Command rules: every cargo and pnpm command runs with `timeout` and is started with `run_in_background: true` (never a
foreground wait), writing a log under `target/eval-diagram/` and printing `exit=<code>` (91-01 Execution rule 9);
per-crate cargo only; `cmp` and `node -e` are blocked (use `diff -q`). The live run never runs in CI; it is the
ignored test `eval_diagram_live::live_run`, invoked once in 91-05.

CI homes of the offline re-score: Rust half in `ci.yml` job `tests` (nextest on affected crates; any vox-diagram or
contracts change selects vox-cli); Node half in `.github/workflows/docs-quality.yml` job `docs-quality`, step
"Docs unit tests" (`node --test "tests/unit/*.test.mjs"`, triggered by `contracts/**`, `docs-astro/**`,
`crates/vox-cli/**`).

---

## Sampling Rate

- **After every task commit:** the task's own `<automated>` commands (the eval_diagram filter, the paired_ni filter, the Node re-score test, as the task lists).
- **After every plan wave:** the quick run commands (all three).
- **Before `/gsd-verify-work`:** the full suite command green locally for the touched crates, then hosted CI (`tests` and `docs-quality`) green on the milestone PR.
- **Max feedback latency:** about 60 s once the test binaries are built.

---

## Requirement → Automated Command Map

| Requirement | Success criterion | Automated command | Fails when |
|-------------|-------------------|-------------------|------------|
| EVAL-01 | SC1: paired harness, >= 60 tasks (200), unpinned registry selection through the facade, deterministic oracles, first-pass and post-repair validity with Wilson 95% CIs | `timeout 3600 cargo nextest run -p vox-cli --lib --test it --retries 0 -E 'test(/eval_diagram/)'` (task-set gates: 200 tasks, quotas; scorer; stub run; spend cap and probe ledger; freeze verification; preconditions) and the 91-04 acceptance greps (no vendor hostname, no model-shaped literal, no direct git spawn, `CandidateScope::CloudOnly`, `llm_chat`, `vox_git::read_only`) | exit other than 0, no tests run, a grep count above 0 |
| EVAL-01 | SC1: paired interval and verdict computed correctly | `timeout 1800 cargo nextest run -p vox-eval --retries 0 -E 'test(/paired_ni/)'` | exit other than 0 or no tests run |
| EVAL-01 | SC2: verdict under a rule fixed before the run | In code (91-04): the live run refuses unless the freeze commit is the pre-registration commit (subject prefix), an ancestor of HEAD, and the dataset and report are unchanged since it (unit tests per refusal; three fail-closed runs). After the run (91-05 Task 2): prereg block at the pre-registration commit equals the current block (`diff -q`); header `freeze_commit` equals that SHA | a refusal missing, any difference, empty block, or SHA mismatch |
| EVAL-01 | SC2: report committed under docs/src/architecture with valid frontmatter | `timeout 1200 cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture/voxdiagram-authoring-reliability-findings-2026.md` | exit other than 0 or any ERROR line |
| EVAL-01 | SC2: re-scoring reproduces the tallies deterministically | Rust: `timeout 3600 cargo nextest run -p vox-cli --test it --retries 0 -E 'test(/eval_diagram_rescore/)'`; Node: `timeout 600 pnpm --dir docs-astro exec node --test tests/unit/diagram-authoring-rescore.test.mjs` | exit other than 0, no tests run, `# fail` above 0 |
| EVAL-02 | SC3: advisory row annotated unsourced and linked to the measured result | 91-04 Task 3 acceptance greps (row first cell, blockquote) + `git diff --numstat 5a631bb22 -- <advisory>` = 3 added / 1 deleted + `timeout 900 target/debug/vox ci check-links` | a grep prints 0, another numstat, or a broken link |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 91-01-01 | 01 | 1 | EVAL-01 | T-91-01, T-91-02, T-91-03 | Byte-equal re-derivation in both runtimes; hostile text only on stdin; synthetic fixture labelled | integration (both runtimes) | eval_diagram filter ; Node re-score + existing oracle tests | ❌ W0 (created test-first in the task) | ⬜ pending |
| 91-01-02 | 01 | 1 | EVAL-01 | — | N/A (pure statistics) | unit | paired_ni filter | ❌ W0 | ⬜ pending |
| 91-01-03 | 01 | 1 | EVAL-01 | T-91-01 | Recording checks reject deleted turns and edited feedback (mutations) | unit + integration | eval_diagram filter ; Node re-score ; scoped lint of where-things-live | ✅ (from 91-01-01) | ⬜ pending |
| 91-02-01 | 02 | 2 | EVAL-01 | T-91-04, T-91-05, T-91-06, T-91-07 | Slot-only template differences; card size gate; banned syntax words; manifest hashes | unit + integration | eval_diagram filter ; Node re-score | ❌ W0 (`eval_diagram_tasks.rs`) | ⬜ pending |
| 91-02-02 | 02 | 2 | EVAL-01 | T-91-05 | Expected sets only from the real parser | integration | eval_diagram filter (it) ; Node re-score ; dependency diff | ✅ | ⬜ pending |
| 91-03-01 | 03 | 3 | EVAL-01 | T-91-20 | Quotas measured from canonical sets, not tags | integration | eval_diagram filter (it, --no-capture totals) ; Node re-score | ✅ (quota_counts created test-first) | ⬜ pending |
| 91-03-02 | 03 | 3 | EVAL-01 | T-91-20, T-91-21, T-91-SC | Count 184/16 and quota thresholds (red-first); final manifest hashes | integration | eval_diagram filter (it) ; Node re-score ; dependency diff | ✅ | ⬜ pending |
| 91-04-01 | 04 | 4 | EVAL-01 | T-91-08, T-91-09, T-91-12 | Spend cap before every call, counting earlier runs and the probe ledger; stop lines; no fallback model | unit (stub backend) | eval_diagram filter | ❌ W0 (run.rs tests) | ⬜ pending |
| 91-04-02 | 04 | 4 | EVAL-01 | T-91-08, T-91-09, T-91-10, T-91-11, T-91-13, T-91-14 | Freeze verified in code via read-only git (subject, ancestry, unchanged); probe ledger appended per probe; override refused; no vendor hostname, model literal or direct git spawn | unit + ignored-test negative runs | eval_diagram filter ; three fail-closed `cargo test --ignored` runs (no variable, HEAD, random SHA) ; acceptance greps | ❌ W0 (live.rs tests, `eval_diagram_live.rs`) | ⬜ pending |
| 91-04-03 | 04 | 4 | EVAL-01, EVAL-02 | T-91-14 | Rule committed before any call, separate commit | doc gates + git | scoped lint ; check-links ; `git show --stat` of the pre-registration commit | ✅ | ⬜ pending |
| 91-05-01 | 05 | 5 | EVAL-01 | T-91-15, T-91-17, T-91-18 | Key-prefix scan of run files and probe ledger; one run; stop for a human on no provider (probe ledger committed) or incomplete run | live (manual trigger) + offline re-score | header checks ; Rust re-score ; Node re-score ; corruption mutation | ✅ | ⬜ pending |
| 91-05-02 | 05 | 5 | EVAL-01 | T-91-16, T-91-19 | Report tables equal the re-scored run; prereg block unchanged; no task prompt in the report | integration + doc gates | eval_diagram filter (it) ; prereg diff ; scoped lint ; check-links | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Test-first inside the tasks that need them (no separate scaffold plan):

- [ ] `crates/vox-eval/src/paired_ni.rs` tests: six worked vectors, properties, verdict mapping (91-01 Task 2; phi mutation)
- [ ] `crates/vox-cli/tests/it/eval_diagram_rescore.rs` + `docs-astro/tests/unit/diagram-authoring-rescore.test.mjs` with its `parseError` and `oracleLine` tests (91-01 Task 1; verdict-flip mutation)
- [ ] `score.rs` / `tally.rs` unit tests and the deleted-turn and edited-feedback mutations (91-01 Task 3)
- [ ] `crates/vox-cli/tests/it/eval_diagram_tasks.rs` (91-02 Task 1; count and quota gates in 91-03)
- [ ] `run.rs` stub-backend and `prior_spend` tests; `live.rs` freeze-verification, probe-ledger and precondition tests (91-04 Tasks 1 and 2)
- [ ] No framework install: node, pnpm, mermaid 11.17.2 and jsdom are already installed

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The live run itself | EVAL-01 | Spends real money (cap $25, probes included) and calls external providers; never in CI (CONTEXT: Reproducibility) | 91-05 Task 1 step 2, once; a no-provider or incomplete outcome stops for a human |
| Reading the verdict and limitations for honesty | EVAL-01 | Prose judgement over numbers the test already pins | Verify-work: read the report's Verdict and Limitations against `tallies.json` |
| Hosted CI over the `contracts/**` change (full Rust suite, docs-quality incl. biome lint) | EVAL-01, EVAL-02 | Only runs on hosted CI | Milestone PR checks `tests` and `docs-quality` green |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120 s once built
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
