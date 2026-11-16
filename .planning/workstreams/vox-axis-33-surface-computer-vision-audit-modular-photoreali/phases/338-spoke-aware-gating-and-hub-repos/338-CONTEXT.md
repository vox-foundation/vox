# Phase 338: Spoke-aware gating and Hub repos - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning
**Source:** Orchestrator brief (no interactive discuss-phase). The user asked for the plan now; where
discuss-phase would have asked a question, the recommended default was chosen and is marked
`[ASSUMED]` below. Items marked `[USER]` come from the user's own brief.

<domain>
## Phase Boundary

Gate each MENS adapter against its spoke's role, not only the ladder gate. Delivers:

1. Per-spoke gate files (vox-lang, rust, tool-selection, argument-generation, plus the ladder gate)
   wired into promotion, with an honest inventory of which parts are wired today and which are paper.
2. The spoke recorded for every published adapter (Hub card metadata and receipt).
3. Eval and data dataset repos so held-out and collateral items are published and versioned.
4. Receipts that hash their inputs and the gate policy, so a pass under a retired rule renders as
   "needs re-gate" instead of "passed".
5. Behavioral spoke suites (top priority): harness/agent spokes run multi-turn episodes against the
   real Vox harness and its live tool schemas; vox-lang and rust gate on compile-and-test pass rates.

Out of scope: card redesign (phase 339), the local selector/pipeline (phase 340), eval throughput
work (phase 337), and the debt sweep (phase 341). 338 produces the gate states and receipts those
phases consume.

</domain>

<decisions>
## Implementation Decisions

### Spoke identity on the Hub
- **D-01 [ASSUMED]:** The spoke is recorded in card metadata (HF YAML front matter / `vox` metadata
  block) and in the receipt, NOT in the repo id. Existing ids `<ns>/vox-mens-qwen3-<size>`
  (phase 324) are kept unchanged. Reason: renaming published repos is one-way, and the current
  ladder adapters are trained on mixed data (`target/dogfood/train_mixed.jsonl`), so their honest
  spoke value is a "mixed"/multi-spoke value, not one spoke. A spoke-specific repo id is only needed
  when a single-spoke adapter is first published; that naming is deferred to that moment (see
  Deferred) and must be raised with the user before any such repo is created.
- **D-02 [ASSUMED]:** A mixed-data adapter is gated against every spoke suite whose gate file applies
  to it, and its card shows one gate state per spoke. It is never shown as "passed" for a spoke whose
  suite it did not run.

### Gate policy and promotion
- **D-03 [USER]:** Per-spoke gate files (`mens/config/eval-gates.yaml`, `eval-gates-rust.yaml`,
  `eval-gates-agents.yaml`, `eval-gates-ladder.yaml`) are the policy source of truth, and promotion
  consults the gate(s) mapped to the adapter's spoke via `mens/config/domain-profiles.yaml`. Plans
  start from a wired-vs-paper inventory of each gate file and wire or delete each paper item.
- **D-04 [USER]:** GSM8K and MATH remain collateral (no-regression) checks only. They are never the
  evidence that a spoke works.

### Receipts
- **D-05 [USER]:** Receipts hash their inputs (eval item files, adapter weights/revision, base
  snapshot, decode settings) and the gate policy (a canonical hash of the effective gate rule), in
  addition to the existing `source_commit` (commit 2e9f45caf). A receipt whose policy hash differs
  from the current policy renders as "needs re-gate".
- **D-06 [ASSUMED]:** The 1.7B/4B/8B receipts that passed under the retired 5%-relative/n=32 rule are
  rendered "needs re-gate" by this phase; re-running those sizes is scheduled but optional and gated
  on the stated wall-clock budget, not required to close the phase. Proof of the behavioral suites
  and their known-bad controls is done on the smallest size (0.6B) first.

- **D-18 [USER, coordinator 2026-11-05]:** First wave of 338, ahead of everything else (but still
  after phase 335 lands): close the stale-pass publish hole in hub-sync. Today
  `verify_publish_gates` accepts any collateral report whose status is `"pass"`, including the
  32-item-era reports for 1.7B/4B/8B that carry no `slice_blake3`; the receipt gate checks slice pins
  only when eval-gate is re-run, so a sync of those sizes could publish weights on a stale pass. The
  publish path must refuse when (a) the collateral report lacks the current slice pins, (b) its rule
  or alpha differs from `mens/config/eval-gates-ladder.yaml`, or (c) its `grader` field (added in
  f51c569c0, currently "grade_final_answer v2") differs from the current grader. Each of the three
  refusals is mutation-proven (break the guard, watch its test fail, restore).

### Behavioral suites
- **D-07 [USER]:** Harness/agent spokes (tool-selection, argument-generation) are gated on
  multi-turn episodes against the real Vox harness with its live tool schemas, checking that the
  model: picks the right tool; emits schema-valid arguments; consumes tool results; recovers from a
  tool error; stops when done; refuses out-of-scope calls.
- **D-08 [USER]:** Episode scoring comes from episode outcome and schema validation, never from text
  matching a reference.
- **D-09 [USER]:** vox-lang and rust spokes are gated on compile-and-test pass rates.
- **D-10 [USER]:** Every suite ships a known-bad control (e.g. the base model, or a shuffled-tool
  adapter) that must fail it. A suite whose control passes is treated as non-discriminating and
  cannot gate.
- **D-11 [ASSUMED]:** Tool schemas are read from the live registry at eval time (the same source the
  harness serves), not from a hand-copied snapshot; the receipt records the hash of the schema set
  used, so a schema change also renders "needs re-gate".

### Dataset repos
- **D-12 [ASSUMED]:** Eval and data items are published as Hub dataset repos, versioned by revision,
  and receipts reference the dataset revision they used. New dataset repo ids are new names, not
  renames; the planner picks ids consistent with the phase-324 naming config and lists them
  explicitly. Every actual push to the Hub is a human-approved checkpoint (no autonomous push).
- **D-13 [ASSUMED]:** Third-party collateral items (GSM8K, MATH) are only republished if their
  licences allow redistribution; otherwise the dataset repo publishes item ids plus hashes and a
  pointer to the upstream source.

### Constraints on execution
- **D-14 [USER]:** Reuse existing eval code (`crates/vox-eval`, `crates/vox-ml-cli` `mens eval-*`,
  the harness in vox-orchestrator / vox-actor-runtime) rather than a new framework. Test-first; no
  new crate edges without a proposed exception in the PR description; all LLM calls through
  `vox_actor_runtime::llm`.
- **D-15 [USER]:** Phase 335 is actively editing `crates/vox-eval` and
  `crates/vox-ml-cli/src/commands/mens/eval_*`. Plans that touch those files are ordered after
  phase 335 lands and must re-read those files at execution time rather than assume their current
  shape. Phase 337 (eval throughput) also precedes 338.
- **D-16 [USER]:** Every plan that runs an eval states the expected wall-clock per size and avoids
  redundant runs (reuse base-side generations and existing records where the protocol matches).
- **D-17 [USER]:** 338's output contract for phases 339 (cards v2) and 340 (local selector) is
  explicit: a versioned, machine-readable per-spoke gate state and receipt schema that those phases
  read, with "passed", "failed", "needs re-gate" and "not run" as distinct states.

### Post-research defaults (2026-11-05, from 338-RESEARCH.md open questions)
- **D-19 [ASSUMED, corrects D-01/D-02]:** Research found `train_mixed.jsonl` is 1166 rows from lanes
  `vox_codegen` and `vox_docs_qa` only, matching `mix-vox-lang.yaml` `include_lanes`. The current
  ladder adapters are therefore `vox-lang` spoke adapters. The spoke is derived from the training-lane
  histogram via `include_lanes`, not hand-set. D-01 (metadata, not repo id) stands; D-02 becomes: an
  adapter is gated on its derived spoke's suite, and every other spoke shows `not_run`.
- **D-20 [ASSUMED]:** Local-weight episodes and evals run in-process through the existing `MlBackend`
  path (as every existing local-adapter eval does), plus one parity test that routes the same turn
  through the `vox_actor_runtime::llm` facade against `vox mens serve`. No new direct vendor calls.
- **D-21 [ASSUMED]:** The live tool catalog is sourced by Option A: a hash-pinned catalog export
  produced by the real registry (`vox-orchestrator-mcp`) with a drift test, consumed by
  `vox-ml-cli` as data. No new `vox-ml-cli -> vox-orchestrator-mcp` crate edge (Option B would need a
  user-authorised exception).
- **D-22 [ASSUMED]:** No tool-selection, argument-generation or rust adapter exists, so those suites
  ship as "suite + controls proven" (scripted oracle 100%, every named control fails, 0.6B base
  control fails) and their gate floors are provisional; current adapters show `not_run` for them.
  The rust spoke gets a small held-out compile-and-test bench with hidden tests in this phase.
- **D-23 [ASSUMED]:** The episode runner presents tool parameter schemas to the model itself. Changing
  the `vox mens serve` chat route to show parameter schemas (it shows only `name: description` today)
  is recorded as a finding and deferred, not changed in 338.
- **D-24 [ASSUMED]:** Requirement IDs SPOKE-01..SPOKE-08 are minted from 338-RESEARCH.md
  "Phase Requirements" and carried in plan frontmatter.
- **D-25 [DECIDED by user 2026-11-05: "Yes, private"]:** `<ns>/vox-mens-eval` and `<ns>/vox-mens-data`,
  both private; MATH-500 as ids + hashes + upstream pointer only, GSM8K republished. Still open at
  plan 09 task 2: whether to re-sync the stale 1.7B/4B/8B Hub cards. Original assumption: Dataset repo ids (proposed `<ns>/vox-mens-eval`,
  `<ns>/vox-mens-data`) and visibility (private recommended, so held-out gold answers do not become
  training data) are a one-way choice: plans build the dataset bundles and manifests locally and stop
  at a human checkpoint before any push. MATH-500 is published as ids + hashes + upstream pointer
  (no declared licence); GSM8K (MIT) may be republished.

### Claude's Discretion
- Exact file layout of new suites, episode fixture format, and which existing eval module hosts the
  episode runner, provided D-14 holds.
- The canonical-hash algorithm and where the hash helper lives, provided it goes through
  `vox-crypto` (repo cryptography policy).
- Plan granularity and wave layout.

</decisions>

<specifics>
## Specific Ideas

- The collateral rule changed from a 5%-relative drop on n=32 to an exact one-sided McNemar test on
  128 items per slice (commit 7ccb4adaa). That is the concrete example of a "retired rule".
- Batched decoding exists (commit 7ccb4adaa, `VOX_REASONING_EVAL_BATCH_SIZE`), but 4B+ evals are
  still hours long.
- Known-bad control examples from the roadmap: base model; shuffled-tool adapter.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope
- `.planning/ROADMAP.md` §"Phase 338", §"Phase 339", §"Phase 340" — goal and consumer phases
- `.planning/phases/335-0-6b-collateral-gate-pass-and-hub-sync/335-RESEARCH.md`, `335-01-PLAN.md` —
  the in-flight collateral gate work 338 must sequence after
- `.planning/phases/336-model-cards-and-hub-index-from-the-sync-pipeline/336-01-SUMMARY.md` —
  record-driven cards that will render gate state

### Gate policy and spokes
- `mens/config/eval-gates.yaml`, `mens/config/eval-gates-rust.yaml`,
  `mens/config/eval-gates-agents.yaml`, `mens/config/eval-gates-ladder.yaml`
- `mens/config/domain-profiles.yaml` — spoke definitions

### Harness and tool schemas
- `docs/src/reference/mcp-tools.generated.md` — generated tool surface (read the generator input, not
  the output, for structure)
- `contracts/` — tool and schema contracts

### Repo policy
- `AGENTS.md` — test-first, crate-edge discipline, LLM facade, cryptography policy

</canonical_refs>

<deferred>
## Deferred Ideas

- Repo-id scheme for a future single-spoke adapter (e.g. a spoke suffix). Raise with the user before
  the first such repo is created; renaming is one-way.
- `vox mens serve` chat route showing tool parameter schemas to local models (D-23).
- Re-gating 4B/8B under the new rule if the measured wall-clock does not fit the budget; they stay
  "needs re-gate" until run.

</deferred>
