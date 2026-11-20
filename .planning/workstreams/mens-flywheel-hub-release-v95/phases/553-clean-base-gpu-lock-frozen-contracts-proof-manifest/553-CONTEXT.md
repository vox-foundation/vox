# Phase 553: Clean Base, GPU Lock, Frozen Contracts & Proof Manifest - Context

**Gathered:** 2026-11-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Every later comparison starts from one reviewed base SHA on `main`, only one GPU job can run at a time, the JSON shapes
and proof inputs that parallel agents share are frozen, and the failed v4 27B adapter is preserved durably.

</domain>

<decisions>
## Implementation Decisions

### Landing the base (BASE-01) — owner decision 2026-11-20, overrides the roadmap's "~157 MENS paths" wording
- No other agent is running in the repo. Land **all** dirty and untracked work in the main checkout (162 paths on
  2026-11-20, several workstreams: MENS v9.4 phases 547–550, OpenWiki/doc-pipeline claims v9.1, frontpage tour v9.3,
  plus smaller compiler/orchestrator/search/GUI/CLI edits and the two local-only v9.5 planning commits), not only MENS.
- Preserve everything. Group commits by workstream (path-scoped `git add`, never `-A`); finish or fix whatever does not
  compile or fails its own tests, in parallel by group; drop nothing silently. Anything judged not useful is listed with a
  reason, not deleted.
- Land on `main` through a PR (work reaches `main` only via PR). Push and PR are authorized by the owner for this work.
  If hosted CI is unavailable (billing), run local CI and merge on green, saying so in the PR body.
- The base SHA recorded in the proof manifest is the merge commit on `main`.
- Also fold in the Space fix branch `fix/hf-space-live-base-demo` (`dda2c1d6f`).

### GPU lock (BASE-02)
- Reuse the file-lock pattern in `crates/vox-build-queue/src/global.rs`; machine-wide lock file `~/.vox/locks/gpu.lock`.
- When busy: refuse and name the holder (PID, command, start time); waiting is opt-in (`--wait`).
- Also treat a running `mens train` / `mens eval*` process that did not take the lock (e.g. another agent's) as busy.
- Every eval and train report header records whether another GPU job was running.

### v4 preservation and output-dir guard
- Copy the failed v4 27B adapter, collateral report, evidence and train record from the Antigravity scratch folder
  `qwen38_27b_v4_failed_gate` to `mens/runs/qwen3.8-27b-v4-failed-gate/` (gitignored); record BLAKE3 hashes in the proof
  manifest.
- Training refuses `--force-restart` on an output dir holding a finished adapter; resuming the same run stays allowed;
  a test fails when the refusal is removed.

### Claude's Discretion
- Contract file names and schema format under `contracts/mens/`, proof-manifest layout under `mens/config/`, CPU
  re-score driver shape (`.vox` driver per VoxLang-first policy).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-build-queue/src/global.rs` — machine-wide file-lock semaphore (build broker).
- `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` — exact McNemar, collateral report writer.
- `contracts/reports/mens-execution-eval-rescore-v2.json` — existing CPU re-score of 10 checkpoints.

### Established Patterns
- Reports live in `contracts/reports/*.json`; configs in `mens/config/*.yaml`; `mens/runs/` is gitignored.
- New metric fields are `Option` (absent = unknown), never default 0.

### Integration Points
- `crates/vox-ml-cli/src/commands/mens/mod.rs` — pre-register this milestone's new module lines.
- Train entry: `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` (output-dir guard, GPU lock).

</code_context>

<specifics>
## Specific Ideas

- Two orphaned `cargo test -p vox-terminal-core` processes (PIDs 31909, 32894, two days old) exist; do not pattern-kill
  them. They are reported to the owner, not touched.

</specifics>

<deferred>
## Deferred Ideas

- Uploading the v4 adapter to a private HF repo (declined; local durable copy only).

</deferred>
