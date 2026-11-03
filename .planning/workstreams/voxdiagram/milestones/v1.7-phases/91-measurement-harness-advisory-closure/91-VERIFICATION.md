---
phase: 91-measurement-harness-advisory-closure
verified: 2026-10-28T04:10:00Z
status: passed
score: 7/7 must-haves verified
covered_files: [".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-01-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-01-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-02-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-02-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-03-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-03-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-04-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-04-SUMMARY.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-05-PLAN.md", ".planning/workstreams/voxdiagram/phases/91-measurement-harness-advisory-closure/91-05-SUMMARY.md", "contracts/eval/diagram-authoring/manifest.v1.yaml", "contracts/eval/diagram-authoring/runs/2026-10-27/attempts.jsonl", "contracts/eval/diagram-authoring/runs/2026-10-27/tables.md", "contracts/eval/diagram-authoring/runs/2026-10-27/tallies.json", "crates/vox-cli/src/commands/model/eval_diagram/live.rs", "crates/vox-cli/src/commands/model/eval_diagram/mod.rs", "crates/vox-cli/src/commands/model/eval_diagram/oracle.rs", "crates/vox-cli/src/commands/model/eval_diagram/prereg.rs", "crates/vox-cli/src/commands/model/eval_diagram/prompt.rs", "crates/vox-cli/src/commands/model/eval_diagram/render.rs", "crates/vox-cli/src/commands/model/eval_diagram/run.rs", "crates/vox-cli/src/commands/model/eval_diagram/score.rs", "crates/vox-cli/src/commands/model/eval_diagram/stats.rs", "crates/vox-cli/src/commands/model/eval_diagram/tally.rs", "crates/vox-cli/tests/it/eval_diagram_frozen.rs", "crates/vox-cli/tests/it/eval_diagram_live.rs", "crates/vox-cli/tests/it/eval_diagram_rescore.rs", "crates/vox-cli/tests/it/eval_diagram_tasks.rs", "crates/vox-eval/src/paired_ni.rs", "docs-astro/tests/lib/mermaid-batch.mjs", "docs-astro/tests/unit/diagram-authoring-rescore.test.mjs", "docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md", "docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md"]
covered_digest: "v3:sha256:168a2785f914d05f2c3703500eab2b8882c501021888e4ccc39939eb5151e292"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 7/7
  reason: "stale digest after 9a5dc6bf7 (MAX_TOKENS 16384, replay diagnosis in the report) and 3ff94db58 (advisory note states the contradiction)"
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 91: Measurement Harness & Advisory Closure Verification Report

**Phase Goal:** The claim that agents author VoxDiagram at least as reliably as Mermaid is measured and reported honestly, and the advisory's unsourced numbers are no longer presented as fact.
**Verified:** 2026-10-28T04:10:00Z
**Status:** passed
**Re-verification:** Yes. The previous report went stale after two commits landed: `9a5dc6bf7` and `3ff94db58`. It had no gaps.

**Scope note.** No cargo build was run, because two builds were already in flight. The evidence comes from reading the code, from querying the committed recording with `jq`, from an independent Node re-implementation of Newcombe and McNemar (the reviewer's `newcombe.mjs`, re-run here), and from the test logs on disk. The working tree is clean for every covered source file at `3ff94db58`, and the digest above was computed there.

## What changed since the previous verification

| Commit | Change | Criteria affected | Result |
|---|---|---|---|
| `9a5dc6bf7` | `live.rs` `MAX_TOKENS` 4096 -> 16384 (future runs only). The report gains "Later runs allow 16384 …" in Method and a "Diagnosis by replay" sub-bullet in the observed limitations. | SC1 (harness config), SC2 (report) | No regression, details below |
| `3ff94db58` | The advisory's "Unsourced row" note now says the measurement contradicts the 18-35% figure. | SC3 | Strengthened, details below |

**`9a5dc6bf7`, checked:**
- **Recorded run unaffected.** The header of `runs/2026-10-27` still records `max_tokens: 4096`. `MAX_TOKENS` feeds only `slot_config` and the header of new runs, so the recorded run and its re-score do not change.
- **Pinned blocks unaffected.** No prereg block, manifest or scorer changed. The prereg block was compared again with `f1ef3bb50` and the diff was empty. The new report text lies outside both the prereg markers and the results markers, so `diagram_authoring_report_matches_run` still pins the results block.
- **Replay claims check out.** I checked each one against the replay log (scratchpad `replay.log`, test `replay_empty_replies` ok, "total replay spend $0.024459"):
  - 17 calls were replayed. 2 were model B decode errors (g055:B:mermaid turns 0 and 1), which leaves 15 answered.
  - 8 of the 15 now returned content: g050, g053, g062, g096:0, g128, g029:1, g096:1 and g055:B:2.
  - 7 came back empty again:
    - 6 stopped with `finish=length` at 4096 tokens: g029:0, g055:0/1/2 and g056:1/2, all Mermaid.
    - 1 stopped with `finish=stop` at 596 completion tokens, all of them reasoning, with the fenced answer only in the reasoning: g059 VoxDiagram.
  - The report's figures ($0.024, 8 of 15, 7 empty, 6 at `length`, g059 at 596 tokens, 2 decode failures) all match. The paragraph says the replay is "not part of the measurement" and leaves the verdict alone, which is correct.

**`3ff94db58`, checked.** The note now quotes first-pass failure rates. They match `tables.md`:

| Model | Mermaid | VoxDiagram |
|---|---|---|
| A | 8/200 = 4.0% | 4/200 = 2.0% |
| B | 4/199 = 2.0% | 5/199 = 2.5% |

The note sets these against the claimed 18-35%. This closes the earlier open item (the annotation did not say that the measurement contradicts the row).

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | SC1a: the paired NL-to-diagram harness runs at least 60 tasks over both arms | VERIFIED (regression check) | `tasks.v1.jsonl` has 200 lines (184 graph, 16 sequence), unchanged since the freeze. `run_slot` sends every task in both arms. The recording holds 827 attempts. |
| 2 | SC1b: models are selected through `vox_actor_runtime::llm`, with no pin and no vendor hostname | VERIFIED | Calls: `chat_once` -> `llm_chat` is the only call path. Selection: `decide()` with `CandidateScope::CloudOnly`, and `refuse_override` accepts only `SelectionReason::Scored`. The `9a5dc6bf7` diff touches only `MAX_TOKENS` and a doc comment. A grep for hostnames in `eval_diagram/` and the it tests still finds nothing. The recorded run's selection reason is Scored. |
| 3 | SC1c: the deterministic parsers are the oracle (no LLM judge), and validity is reported per arm at first pass and after bounded repair, with Wilson 95% CIs | VERIFIED (regression check) | The oracles (`vox_diagram::check`, `mermaid-batch.mjs` pinned at mermaid 11.17.2), `MAX_REPAIR_TURNS` = 2 and `wilson_interval(.., Z_95)` are all unchanged. The report adds "an answer found only in the reasoning still scores invalid; the rule reads the reply", which matches `oracle_text`, since it reads `raw_response` only. |
| 4 | SC2a: the report states the verdict the data supports under a decision rule fixed before the run | VERIFIED | The freeze commit `f1ef3bb50` (22:27Z) is an ancestor of the run commit `ea0af22a4`, and the run started at 22:33Z. The prereg block at the freeze commit and at HEAD is identical (re-checked after `9a5dc6bf7`). Tasks, cards and the template are unchanged since the freeze. `tallies.json` gives `non_inferior`, and the report says "Primary verdict: non-inferior." once. An independent jq tally agrees: A Mermaid 192/200, A VoxDiagram 196/200, B 195/199 and 194/199. `newcombe.mjs` reproduces the four paired rows (A first pass [-0.01655, +0.05923], p 0.3877). |
| 5 | SC2b: the report is committed under `docs/src/architecture/` with valid frontmatter | VERIFIED | `9a5dc6bf7` touched no frontmatter. The kb:summary is still 95 words, and the lint log for 91-05 is clean. The added text uses no new fence, and it adds no `mermaid` fence and no task prompt. |
| 6 | SC2c: re-scoring the recorded model outputs reproduces the tallies deterministically | VERIFIED | Neither new commit touches the recording, the derived files, the scorer or the oracles. The Rust re-score (oracle-inputs, `tallies.json`, `tables.md`, results block of the report) and the Node re-score (oracle-outputs) are unchanged and run in CI. Mutations M1 and M2 were proven in 91-05. Latest green logs: `green-91fix.log` (it: 11 passed, 1 ignored), `s7-it.log`, `cli.log` (lib: 58 passed). The new `Attempt` fields use `serde(default)` plus `skip_serializing_if`, so the recording parses and re-serialises unchanged. |
| 7 | SC3: the advisory's error-rate table is annotated as unsourced and links to the measured result | VERIFIED | Line 141 labels the row "(unsourced, see note)". Line 147 is a note that links `voxdiagram-authoring-reliability-findings-2026.md` and, since `3ff94db58`, states that the measurement does not support the figure (rates checked above). The 18-35% figure appears nowhere else in `docs/src` (archive excluded). |

**Score:** 7/7 truths verified (0 present but behaviour-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/vox-eval/src/paired_ni.rs` | Newcombe method-10 interval and NI verdict | VERIFIED | Unchanged. Independently reproduced. |
| `crates/vox-cli/src/commands/model/eval_diagram/` | scorer, tally, render, run loop, live wiring, prereg | VERIFIED | The only change since the last run is `MAX_TOKENS`, a constant. |
| `crates/vox-cli/tests/it/eval_diagram_*.rs` | task, frozen, re-score and live tests | VERIFIED | Unchanged and registered in `tests/it/main.rs`. |
| `contracts/eval/diagram-authoring/` | 200 tasks, cards, template, manifest with the rule, probe ledger | VERIFIED | Unchanged. |
| `contracts/eval/diagram-authoring/runs/2026-10-27/` | raw recording and derived files | VERIFIED | Unchanged. Header `max_tokens` 4096, `freeze_commit` f1ef3bb. |
| `docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md` | findings with the verdict | VERIFIED | The additions are outside the pinned blocks, and their claims were checked against the replay log. |
| `docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md` | annotated row | VERIFIED | Lines 141 and 147. The rates in the note match `tables.md`. |

### Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| `live.rs` | `vox_actor_runtime::llm::llm_chat` | `chat_once` | WIRED |
| `live.rs` | `vox_orchestrator::models::decide` | CodeGen, CloudOnly, Scored-only | WIRED |
| `run.rs` | `mermaid-batch.mjs` | `LiveBackend::mermaid_batch` | WIRED |
| `eval_diagram_rescore.rs` | findings report | results block == `tables.md` | WIRED |
| `runs/2026-10-27` header | pre-registration commit | `freeze_commit` | WIRED |
| advisory | findings report | markdown link (line 147) | WIRED |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|---|---|---|---|---|
| report results block | tables | `render_tables(tally(recording, oracle-outputs))` | yes | FLOWING |
| advisory note's rates | first-pass failure counts | `tables.md` of `runs/2026-10-27` | yes: they match | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Newcombe/McNemar for the run's four tables | `node scratchpad/newcombe.mjs` | All four rows match `tables.md`. | PASS |
| First-pass counts from the raw oracle outputs | jq over `oracle-outputs.jsonl` | A mm 192/8, A vd 196, B mm 195/5, B vd 195 | PASS |
| Replay claims in the report | grep over the replay log | 15 answered, 8 with content, 6 at `length`, g059 at 596 tokens, 2 decode errors, $0.024459 | PASS |
| Prereg block at the freeze commit vs HEAD | `git show f1ef3bb50:<report> \| awk` + `diff` | empty diff | PASS |
| cargo tests | not run, by instruction | the logs above stand in | SKIP |

### Probe Execution

None declared. The phase's "probes" are model probes in `probe-spend.jsonl`.

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|---|---|---|---|
| EVAL-01 | 91-01..05 | SATISFIED | Truths 1-6 |
| EVAL-02 | 91-04 | SATISFIED | Truth 7 |

No orphaned requirements and no new crate edge.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| (phase files) | none | TBD/FIXME/XXX | none | none |
| `run.rs` | 240 | `ponytail:` first call admitted on headroom alone | Info | Documented ceiling. Not in the success criteria. |

### Open items (non-blocking)

- **No hosted CI evidence.** The branch has no remote, so the green results are local logs only.
- **A future live run will be refused by the freeze check,** because the report changed after `f1ef3bb50`. This is by design: a new run needs a new pre-registration commit.
- **Optional review findings still open:** IN-01, IN-02, IN-03, IN-05 and IN-06.

### Human Verification Required

None.

### Gaps Summary

None. All three roadmap success criteria and both requirements are met at `3ff94db58`. Neither commit since the previous verification changed the recorded run, the decision rule, the scorer or the verdict. They only updated the harness configuration for future runs and added correct statements to the report and the advisory.

---

_Verified: 2026-10-28T04:10:00Z_
_Verifier: Claude (gsd-verifier)_
