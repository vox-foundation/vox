# Phase 333: Gated training for every model size (`REL-01..04`) - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Each published Qwen3 size stays an unfinished trained add-on until it clears one shared release review. After that receipt passes, `vox mens merge-qlora` writes a complete checkpoint from the local Qwen base plus the trained adapter, that checkpoint is stored and distributed, and the Hugging Face card updates in the same step.

This phase does not retrain 4B to fix a smoke preset. It does not flip Hugging Face access gating. It does not call any size ready before both the receipt and the merged file exist.

Depends on nothing in Phases 324–331.

</domain>

<decisions>
## Implementation Decisions

### What is already trained

- **D-01:** The published 4B run is a full shared-recipe run (rank 16, alpha 32, gradient accumulation 8, `global_step` 1108, 138 optimizer updates). It was uploaded before the release checklist passed. It is not a smoke run. The short token and micro-step counts on that resume are the last process only. — **Reversibility:** reversible — a later retrain can replace the artifact, but planning must not schedule a smoke correction for 4B.
- **D-02:** The smoke preset is the published 0.6B run only (`qwen3_dev_cpu`, rank 8, alpha 16, accumulation 1). 1.7B and 8B already used the shared recipe. — **Reversibility:** costly — replacing the published 0.6B adapter requires a new training run.

### What gets distributed

- **D-03:** The file on the Hub today is the trained add-on (`candle_qlora_adapter.safetensors`). The complete model is the checkpoint `vox mens merge-qlora` writes from the local Qwen base plus that add-on. That merged file is what gets stored and distributed, and only after `overall_passed: true`. — **Reversibility:** reversible — the adapter can stay published beside the merge, but the card's download target becomes the merge.
- **D-04:** Original Qwen bases are already in the local Hugging Face cache for 0.6B (1.4 GB), 1.7B (3.8 GB), 4B (7.5 GB), 8B (15 GB), and 14B (28 GB). 32B is not downloaded. 14B has no trained adapter. Neither 14B nor 32B is distributed until it has an adapter and the same passing receipt. — **Reversibility:** reversible — a 32B download and a 14B training run are later steps inside this phase, not a new phase.

### When the card changes

- **D-05:** Until a size is ready, its card and the size index say the page is the trained add-on and the review is unfinished. They do not use the word "ungated". When the merged checkpoint is the distributed file, those pages update in the same step and describe that complete model. A reader can take it from the Files tab or with one `vox mens hub download` command. — **Reversibility:** reversible — card text follows the artifact.

### Claude's Discretion

- The user asked for this to be recorded in planning. D-01 through D-05 are those decisions.
- Do not claim the release review has passed. None of the six sizes has `overall_passed: true`.
- Do not present a training-memory peak as the inference requirement.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Planning
- `.planning/ROADMAP.md` — Phase 333 success criteria
- `.planning/REQUIREMENTS.md` — `REL-01`, `REL-02`, `REL-03`, `REL-04`

### Training and merge
- `crates/vox-populi/src/mens/tensor/preset_schema.rs` — `qwen3_dev_cpu` is the smoke preset; `qwen3_16g` is the shared rank-16 recipe
- `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` — `vox mens merge-qlora --base-shard <base> --adapter <adapter>`
- `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/mod.rs` — `total_tokens` and `micro_steps_executed` start at zero each process; `global_step` is restored on resume

### Cards and serve
- `crates/vox-ml-cli/src/commands/mens/hub.rs` — `vox mens hub download --repo <repo> --output-dir <dir>`
- `docs/src/reference/mens-serving-ssot.md` — serve is `vox mens serve`, port 11435, not an Ollama route

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox mens merge-qlora` already folds a Candle safetensors adapter into a base shard. This phase stores that output. It does not add a second merge implementation.
- Collateral and the release receipt already exist. 4B and 8B collateral checks passed. Their receipts are `overall_passed: false` because `eval_local_report`, `benchmark_passatk`, and `review_metrics` are missing, not because the short samples got worse.

### Established Patterns
- The Hub repos `brbrainerd/vox-mens-qwen3-*` currently contain the adapter, the manifest, and eval evidence. They do not contain `model-*-of-*.safetensors`.
- Card copy lives on the Hub README. `mens/config/hf_readme_template.md` is an older 27B template and is not the ladder card.

### Integration Points
- A resumed run must persist whole-run token and step counts before a card or a receipt treats the summary as the full run.
- Distribution waits on `REL-02`. The merge does not run for a size whose receipt has not passed.

</code_context>

<specifics>
## Specific Ideas

No size is ready today. The cards stay on the add-on wording until a merged checkpoint for that size is the file being uploaded.

</specifics>

<deferred>
## Deferred Ideas

- Retraining 4B. Its published run is already the shared recipe.
- Changing Hugging Face access from public to gated. Readiness here is the release receipt plus the merged file.

</deferred>
