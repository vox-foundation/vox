# Phase 320: Train the Sizes (`SIZE-03`) - Context

**Gathered:** 2026-10-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Every measured `qwen3_code` QLoRA size whose floor fits one free-memory reading gets its own record. A Ready snapshot is trained through the existing lane-prefix QLoRA path, and the adapter manifest names that pin. A size that does not fit, is unmeasured, or has no Ready snapshot gets a not-trained record and no adapter. This phase does not download weights, invent scores, or measure the 1.7B or 4B floors.

The operator accepted every recommended answer in advance, including the areas below that were not asked one at a time.

</domain>

<decisions>
## Implementation Decisions

### Which sizes are attempted
- Candidates are measured `qwen3_code` QLoRA rows whose `floor_mb` is less than or equal to one free-memory reading. 0.6B is 2000 MB, 8B is 12000, 14B is 20000, 32B is 60000. 1.7B and 4B stay `floor_measured: false` and are not trained. Each of those two gets a not-trained record that says the floor was not measured and names no adapter. The 14B LoRA row is not a candidate.
- A base snapshot that is not Ready is `not_trained` with reason `base_snapshot_absent`, the pin, and the free-memory reading. Do not download weights. Ready means `config.json`, `tokenizer.json`, and a weight file are present for that pin, the same completeness rule phase 316 uses.
- "Fits" uses one reading from the phase 318 free-memory adapter, taken once at the start. Tests inject that reading. Total VRAM is not the budget.
- Write records smallest floor first. Every candidate gets its own record, including the ones that are not trained.

### What a record holds
- One JSON file per size under `mens/runs/size_train/<size>-<rev12>.json`. `mens/runs/` is already gitignored. Schema `size_train.v1`.
- Fields: `status` of `trained` or `not_trained`, `hf_id`, `size_token`, `free_mb`, and either `floor_mb` or `floor_measured: false`. A not-trained record has a `reason`. It has no adapter path.
- Do not write accuracy, think-initiation, or intelligence keys. Phase 321 owns scores.

### When a snapshot is Ready
- Call the existing schola QLoRA train with the phase 313 lane prefix. The adapter manifest names the pinned `hf_id`.
- Tests inject a Ready fixture and an absent fixture. They do not start a train, hit the network, or require a GPU. The Ready fixture's record is `trained` and points at a manifest that names the pin. The absent fixture's record is `base_snapshot_absent`.
- On this machine the 0.6B pin `c1899de289a0` is the only measured QLoRA snapshot in the Hugging Face cache. 8B, 14B, and 32B are not cached. Do not start a live multi-hour train in this phase. The live record command classifies each candidate and, for a Ready snapshot, writes `not_trained` with reason `train_not_run` unless a test fixture supplies the adapter. No invented manifest.

### One record per candidate
- Unmeasured rows use reason `floor_unmeasured`. A floor above the reading uses reason `does_not_fit` and includes `floor_mb` and `free_mb`. A missing snapshot uses `base_snapshot_absent`. A Ready snapshot that this phase does not train uses `train_not_run`.
- Follow the `reasoning_compare` command shape for the writer. Register a new subcommand through the existing command catalog if one is required. Do not add a crate edge.

### Claude's Discretion

None. The operator accepted the recommended answers for this phase and for later grey areas.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `pick_base` and `read_accelerator_free_mb` in `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`. Unmeasured rows cannot win a pick. This phase still lists them as not-trained records.
- Phase 316 writes `reasoning_before_after.v1` with `status: not_compared` and `reason: base_snapshot_absent` from `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`. Copy that absence shape, not its score fields.
- `run_train` in `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` is the existing train entry. Adapter manifests are parsed in `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs`.

### Established Patterns
- `mens/runs/` is gitignored. Default outputs stay there.
- A `cargo test` that runs 0 tests is not a pass. `vox-ml-cli` train modules need `--features gpu` when the test is inside a gpu-gated module. The record writer should live where the reasoning-compare tests already compile.
- This cache has `models--Qwen--Qwen3-0.6B` snapshot `c1899de289a0` with `config.json`, `tokenizer.json`, and `model.safetensors`. It also has 1.7B and 4B, which stay unmeasured.

### Integration Points
- The record writer takes an injected free-memory reading and an injected snapshot probe. The live command may call `read_accelerator_free_mb` once. Tests never call it.
- Phase 321 reads these records. A `train_not_run` or `base_snapshot_absent` row must not grow score fields here.

</code_context>

<specifics>
## Specific Ideas

Records live at `mens/runs/size_train/<size>-<rev12>.json`. Reasons are `floor_unmeasured`, `does_not_fit`, `base_snapshot_absent`, and `train_not_run`.

</specifics>

<deferred>
## Deferred Ideas

Live QLoRA training of the cached 0.6B snapshot, weight downloads, 1.7B and 4B floor measurements, and before/after scores stay out of this phase. Scores are phase 321.

</deferred>
