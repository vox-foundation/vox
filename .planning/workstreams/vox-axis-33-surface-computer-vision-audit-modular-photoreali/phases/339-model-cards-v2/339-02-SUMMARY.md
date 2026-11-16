---
phase: 339-model-cards-v2
plan: "02"
subsystem: docs
tags: [mens, hub-cards, docs, vox-diagram]
requires: []
provides:
  - "docs/src/reference/mens-models.md with the 15 fixed anchored headings (contract for 339-05/06/07/08/09)"
affects: [339-05, 339-06, 339-07, 339-08, 339-09]
key-files:
  created: [docs/src/reference/mens-models.md]
  modified: []
key-decisions:
  - "Rank and alpha are not stated as shared: the run records disagree (see Findings)"
requirements-completed: [CARD-08]
status: complete
commits: 1
plan_head_before: 52b8e9fe659675bfd6e43633a70eb0a4d6fe5a5d
plan_head_after: f639df32f7d8986d716a8d745fb1f6dedfcd7a57
actuals:
  tokens: 4107
  tasks: 2
  commits: 1
completed: 2026-11-06
---

# Phase 339 Plan 02: Shared MENS models page Summary

One voxlang.org page (`docs/src/reference/mens-models.md`) now holds all shared card explanation under 15 fixed anchors, with a validated hub-and-spoke diagram, the six-phrase verdict list, the serving-path rule and the full duplicate-sentence rule.

## Commits

- `f639df32f` docs(339-02): add the shared MENS models page the Hub cards link to

(`commits:` above is the count of task commits between `plan_head_before` and `plan_head_after`; this plan's docs commit is the only one on that range from this executor, and the range also contains no other plan's commits at the time it was measured.)

## Verification

- Doc lint: `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/mens-models.md` printed `linting ./docs/src/reference/mens-models.md` and `no hard errors`, exit 0.
- Diagram: the installed `vox` has no `diagram` subcommand and building `vox-cli` was not attempted at load average ~66. The fence body was checked through the same function the CLI calls, `vox_diagram::check` (crates/vox-diagram/src/check.rs), from a throwaway crate outside the repo. Result: `"valid": true`, `"diagnostics": []`, 6 nodes, 1 container, 5 edges. `fixes[]` was empty, so none applied. The fence is the research draft unchanged. The docs build also pre-renders the fence (`--render-diagrams`) and succeeded.
- Acceptance greps: topic, domain, authorship, `authorship_model` once each; 0 `last_updated`; summary 82 words; 15 headings once each at the right level; 1 `vox-diagram` fence, 0 mermaid, 0 `vox` fences; all six verdict phrases in backticks; `four or more words`, `footer`, `adapter_serving` and `ADAPTER_SERVING` present; `kb-facet-backlog.v1.yaml` untouched.

## Anchors (research assumption A7 confirmed)

Build command: `pnpm --dir docs-astro install --frozen-lockfile` then `pnpm --dir docs-astro build` (866 pages, exit 0). Built page: `docs-astro/dist/reference/mens-models/index.html` (gitignored; `git status` shows nothing for it). Every slug is a real `id="<slug>"`:

what-mens-is, hub-and-spoke, the-harness, gating, held-out-pass-rate, collateral-damage, why-mcnemar, pinned-slices, state-legend, examples-on-the-cards, prompt-protocol-and-training-method, shared-training-settings, why-peft-cannot-load-the-adapters, how-the-cards-are-made, links (15 of 15). github-slugger output equals the table slugs, so 339-09's slugger needs no change.

## Findings for later plans

- **Rank and alpha are not shared across sizes in the records.** `adapter_card.json` says `lora_rank 16`, `lora_alpha 32` for every trained size, but `training_manifest.json` says rank 8 / alpha 16 for 0.6B and rank 32 / alpha 64 for 14B (1.7B, 4B, 8B: 16 / 32). The page therefore describes the fields and where they are recorded and states no value. Method (QLoRA via `candle_qlora`), seed 42, sequence length 3072, 1 epoch and the `train_data_sha256` are identical across all five trained sizes (checked with `jq`), so the page states those without numbers per size. 339-05 should decide which record the Training section reads for rank.
- **Existing checkpoints predate the serving fix.** `mens/runs/qwen3-4b/eval_local_report.ckpt.jsonl` has a header with no `adapter_serving`, so under the rule on the page its adapter outputs are not current evidence.
- The `vox-mens-index` link is described by repo name and the `VOX_MENS_HF_NAMESPACE` variable, not a URL, because the namespace is runtime configuration.

## Deviations from Plan

- Task 1's diagram check used `vox_diagram::check` from a scratch crate instead of `cargo run -p vox-cli -- --json diagram check -` (same checker, avoids a full `vox-cli` build). The JSON envelope fields differ from the CLI's; `valid` and `diagnostics` do not.
- `authorship_model` is `claude-sonnet-5-5` (the executing model). The commit trailer is `Claude Opus 5.5` as the caller required.
- The harness section describes the harness from `mens/config/domain-profiles.yaml` and the phase 338 roadmap goal; there is no dedicated harness doc to cite.

## Known Stubs

None.

## Self-Check: PASSED

- docs/src/reference/mens-models.md exists and is tracked in `f639df32f`.
