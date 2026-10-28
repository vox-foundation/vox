---
phase: 91-measurement-harness-advisory-closure
verified: 2026-10-28T03:40:00Z
status: passed
score: 7/7 must-haves verified
covered_files: [".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-01-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-01-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-02-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-02-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-03-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-03-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-04-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-04-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-05-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-05-SUMMARY.md", "contracts/eval/diagram-authoring/manifest.v1.yaml", "contracts/eval/diagram-authoring/runs/2026-10-27/attempts.jsonl", "contracts/eval/diagram-authoring/runs/2026-10-27/tables.md", "contracts/eval/diagram-authoring/runs/2026-10-27/tallies.json", "crates/vox-cli/src/commands/model/eval_diagram/live.rs", "crates/vox-cli/src/commands/model/eval_diagram/mod.rs", "crates/vox-cli/src/commands/model/eval_diagram/oracle.rs", "crates/vox-cli/src/commands/model/eval_diagram/prereg.rs", "crates/vox-cli/src/commands/model/eval_diagram/prompt.rs", "crates/vox-cli/src/commands/model/eval_diagram/render.rs", "crates/vox-cli/src/commands/model/eval_diagram/run.rs", "crates/vox-cli/src/commands/model/eval_diagram/score.rs", "crates/vox-cli/src/commands/model/eval_diagram/stats.rs", "crates/vox-cli/src/commands/model/eval_diagram/tally.rs", "crates/vox-cli/tests/it/eval_diagram_frozen.rs", "crates/vox-cli/tests/it/eval_diagram_live.rs", "crates/vox-cli/tests/it/eval_diagram_rescore.rs", "crates/vox-cli/tests/it/eval_diagram_tasks.rs", "crates/vox-eval/src/paired_ni.rs", "docs-astro/tests/lib/mermaid-batch.mjs", "docs-astro/tests/unit/diagram-authoring-rescore.test.mjs", "docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md", "docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md"]
covered_digest: "v3:sha256:7dd31b56f76558c887cad547cb267c7365acb385f0e2d240180c3f235556c8a3"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 91: Measurement Harness & Advisory Closure Verification Report

**Phase Goal:** The claim that agents author VoxDiagram at least as reliably as Mermaid is measured and reported honestly, and the advisory's unsourced numbers are no longer presented as fact.
**Verified:** 2026-10-28T03:40:00Z
**Status:** passed
**Re-verification:** No (initial verification)

**Scope note.** No cargo build was run, because two builds were already in flight. The evidence comes from reading the code, from querying the committed recording with `jq`, from an independent Node re-implementation of Newcombe and McNemar (the reviewer's `newcombe.mjs`, re-run here), and from the test logs on disk. The uncommitted change (`live.rs` MAX_TOKENS 4096 -> 16384, and the replay-diagnosis paragraph in the report) was treated as in flight and was not judged. The fingerprint covers the working tree, including that change.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | SC1a: the paired NL-to-diagram harness runs at least 60 tasks over both arms | VERIFIED | `tasks.v1.jsonl` has 200 lines (184 graph, 16 sequence). `run_slot` (run.rs:329) sends every task in both arms (`Arm::BOTH`). The recording holds 827 attempts covering all 200 tasks for A and 199 scored for B. |
| 2 | SC1b: models are selected through `vox_actor_runtime::llm`, with no pin and no vendor hostname | VERIFIED | Calls: `chat_once` -> `llm_chat` (live.rs:128-146) is the only call path. Selection: `decide()` with `CandidateScope::CloudOnly`, and `refuse_override` accepts only `SelectionReason::Scored` (live.rs:57-63). A grep for `https?://`, openrouter, anthropic, openai.com or googleapis in `eval_diagram/` and the it tests found nothing. The run header records reason "Chosen by the model scorer" (Scored). |
| 3 | SC1c: the deterministic parsers are the oracle (no LLM judge), and validity is reported per arm at first pass and after bounded repair, with Wilson 95% CIs | VERIFIED | VoxDiagram is judged by `vox_diagram::check`/`validate`, Mermaid by `mermaid-batch.mjs` pinned at mermaid 11.17.2 (`docs-astro/package.json`). Repair is capped at `MAX_REPAIR_TURNS` = 2, enforced in run.rs:340 and score.rs:24. `stats.rs:25` uses `wilson_interval(.., Z_95)`. `tables.md` shows first pass and after repair with a 95% CI for each arm and model. |
| 4 | SC2a: the report states the verdict the data supports under a decision rule fixed before the run | VERIFIED | The freeze commit `f1ef3bb50` (22:27Z) is an ancestor of the run commit `ea0af22a4`, and the run started at 22:33Z. The prereg block is identical at the freeze commit and in the working tree (diff empty). `manifest.v1.yaml`, tasks, cards and the template have no diff since the freeze. No harness or oracle file changed between the freeze and the run. `tallies.json` gives primary verdict `non_inferior`, and the report says "Primary verdict: non-inferior." once. An independent tally from the oracle outputs agrees: A Mermaid 192/200, A VoxDiagram 196/200, B 195/199 and 194/199 after g056. `newcombe.mjs` reproduces all four paired rows to 1e-5 (A first pass [-0.01655, +0.05923], p 0.3877). |
| 5 | SC2b: the report is committed under `docs/src/architecture/` with valid frontmatter | VERIFIED | `docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md` has title, description, category "Architecture SSOTs", topic `doc-pipeline` and domain `documentation` (both taxonomy prefLabels), status `research`, authorship plus authorship_model, and a 95-word kb:summary. `target/eval-diagram/91-05-lint1.log` reads "no hard errors, exit=0". |
| 6 | SC2c: re-scoring the recorded model outputs reproduces the tallies deterministically | VERIFIED | Rust: `eval_diagram_rescore.rs` re-derives oracle-inputs, `tallies.json` and `tables.md` byte for byte, and `diagram_authoring_report_matches_run` pins the report's results block to `tables.md`. Node: `diagram-authoring-rescore.test.mjs` re-derives oracle-outputs. Mutations M1 (one byte of oracle-outputs) and M2 (report cell) failed the tests (91-05 logs). Latest green: `green-91fix.log` (11 passed, 1 ignored), `s7-it.log`, and `cli.log` (lib 58 passed). The scorer and oracles are unchanged since the run. `Attempt`'s new `finish_reason`/`reasoning_tokens` fields use `serde(default)` plus `skip_serializing_if`, so the recording parses unchanged. Neither half is quarantined: the Rust half runs in `ci.yml tests`, the Node half in the `docs-quality` "Docs unit tests" step. |
| 7 | SC3: the advisory's error-rate table is annotated as unsourced and links to the measured result | VERIFIED | `diagramming-and-mermaid-evaluation-advisory-2026.md:141` labels the row "(unsourced, see note)". Line 147 is a "Unsourced row" note linking `voxdiagram-authoring-reliability-findings-2026.md`. A grep of `docs/src` (archive excluded) finds the 18-35% / 4-8% figures nowhere else. |

**Score:** 7/7 truths verified (0 present but behaviour-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/vox-eval/src/paired_ni.rs` | Newcombe method-10 interval and NI verdict | VERIFIED | Independently reproduced. The reviewer's run of `cargo test -p vox-eval --lib paired_ni` passed 4/4. |
| `crates/vox-cli/src/commands/model/eval_diagram/` | scorer, tally, render, run loop, live wiring, prereg | VERIFIED | 3959 lines, no stub. Wired from the it tests and the ignored live test. |
| `crates/vox-cli/tests/it/eval_diagram_*.rs` | task, frozen, re-score and live tests | VERIFIED | Registered in `tests/it/main.rs:22-25`. |
| `contracts/eval/diagram-authoring/` | 200 tasks, cards, template, manifest with the rule, probe ledger | VERIFIED | The manifest is unchanged since the freeze. The ledger holds 2 priced probes. |
| `contracts/eval/diagram-authoring/runs/2026-10-27/` | raw recording and derived files | VERIFIED | Header: `synthetic: false`, `freeze_commit` f1ef3bb, `max_tokens` 4096, CloudOnly. The 827 attempts cost $0.28490 in total, with 8 retries. |
| `docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md` | findings with the verdict | VERIFIED | Results block equals `tables.md` (tested). Limitations both known in advance and observed. |
| `docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md` | annotated row | VERIFIED | Lines 141 and 147. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `live.rs` | `vox_actor_runtime::llm::llm_chat` | `chat_once` | WIRED | This is the only LLM call path. |
| `live.rs` | `vox_orchestrator::models::decide` | CodeGen intent, CloudOnly, Scored-only | WIRED | `run_live` lines 323-331. |
| `run.rs` | `mermaid-batch.mjs` | `LiveBackend::mermaid_batch` (node, stdin/stdout) | WIRED | live.rs:175-204. |
| `eval_diagram_rescore.rs` | findings report | results block == `tables.md` | WIRED | Test at lines 113-192. |
| `runs/2026-10-27` header | pre-registration commit | `freeze_commit` | WIRED | Equals f1ef3bb50d0e…, whose subject is `docs(91-04): pre-register …`. |
| advisory | findings report | markdown link | WIRED | Line 147. The 91-05 `check-links` run passed. |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|---|---|---|---|---|
| report results block | tables | `render_tables(tally(recording, oracle-outputs))` | yes: live recording of 835 calls | FLOWING |
| `tallies.json` primary verdict | paired cells 188/8/4/0 | the recording's turn-0 judgements | yes: an independent jq tally agrees | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Newcombe/McNemar for the run's four tables | `node scratchpad/newcombe.mjs` | 188,8,4,0 -> [-0.01655, 0.05923], p 0.3877. All four rows match `tables.md`. g056 sensitivity: [-0.0355, 0.0355]. | PASS |
| First-pass counts from the raw oracle outputs | jq over `oracle-outputs.jsonl` | A mm 192 ok / 8 not; A vd 196; B mm 195/5; B vd 195 | PASS |
| Report's run claims | jq over `attempts.jsonl` | 17 empty replies (15 mm, 2 vd), 10 at 4096 tokens, 8 retries, 1 transport failure (g056 B mm 1), cost $0.28490, max per-call $0.0037, 0 model drift | PASS |
| Mermaid "valid" kind leak (IN-04) | jq over the kinds of valid Mermaid replies | 364 graph and 32 sequence, no other kind | PASS: no effect on this run |
| cargo tests | not run, by instruction | the logs above stand in | SKIP |

### Probe Execution

None declared. The phase's "probes" are model probes in `probe-spend.jsonl`, not `scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| EVAL-01 | 91-01..05 | Paired harness of at least 60 tasks with Wilson CIs, parser oracle, facade selection and no pin; reports first-pass and post-repair validity | SATISFIED | Truths 1-6 |
| EVAL-02 | 91-04 | The advisory table is annotated as unsourced and linked to the EVAL-01 result | SATISFIED | Truth 7 |

No orphaned requirements: REQUIREMENTS.md maps only EVAL-01 and EVAL-02 to Phase 91. No new crate edge (no diff to `vox-cli`/`vox-eval` `Cargo.toml` or `crate-edges.allow.v1.json` since `efbf294b9`).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| (phase files) | none | TBD/FIXME/XXX | none | none |
| `run.rs` | 240 | `ponytail:` first call admitted on headroom alone | Info | Documented ceiling. Not in the success criteria. |

### Open items (non-blocking)

- **In-flight change not yet committed.** The working tree changes `live.rs` (MAX_TOKENS 16384) and adds a replay diagnosis to the report. Neither is inside the prereg or results markers, and the header of the recorded run still says 4096. When this lands, the report test still pins the results block. The replay claims ($0.024, 8 of 15 answered) were not verified here.
- **No hosted CI evidence.** The branch has no remote, so the green results are local logs only. `it2.log` (the post-`422609e5a` integration run) was still compiling during verification.
- **A future live run will be refused by the freeze check,** because the report changed after `f1ef3bb50`. This is by design: a new run needs a new pre-registration commit.
- **Review findings not addressed:** IN-01 (freeze pathspec omits harness and oracle code; verified by hand for this run), IN-02, IN-03, IN-05 and IN-06 are optional.
- **Advisory annotation:** the advisory's note does not say that the measurement contradicts the row. The row claims 18-35% for Mermaid; the measured first-pass invalid rate was 4% for model A. The success criterion requires only the annotation and the link.

### Human Verification Required

None.

### Gaps Summary

None. All three roadmap success criteria and both requirements are met in the codebase.

The report states its limitations plainly:
- empty replies dominate the discordant cells;
- the g056 exclusion removed a pair that favoured VoxDiagram;
- the fidelity comparison is biased toward VoxDiagram by construction;
- failed calls were charged $0 against the cap.

None of these limitations changes the pre-registered verdict.

---

_Verified: 2026-10-28T03:40:00Z_
_Verifier: Claude (gsd-verifier)_
