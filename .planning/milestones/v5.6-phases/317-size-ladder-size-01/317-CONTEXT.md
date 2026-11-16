# Phase 317: Size Ladder (`SIZE-01`) - Context

**Gathered:** 2026-10-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Every Qwen3 dense size (0.6B, 1.7B, 4B, 8B, 14B, 32B) is a pinned, licensed rung that the trainer and the harness can name. This phase adds the two missing sizes to `train_bases` and a size-name lookup. It does not download weights, train, remeasure floors, or change which measured rung `pick_base` returns.

</domain>

<decisions>
## Implementation Decisions

### Where the six rungs live
- The canonical ladder stays in `train_bases` in `mens/config/gpu-specs.yaml`. The trainer and harness already read it through `spoke_base_resolver::load_overlay`.
- Keep the existing commits and Apache-2.0 license for 0.6B, 8B, 14B, and 32B. Add 1.7B and 4B beside them.
- One QLoRA rung per size. Leave the existing 14B and 32B LoRA rows alone.
- A resolver returns the pinned `hf_id` for each of the six sizes.

### Pins for 1.7B and 4B
- Repositories are `Qwen/Qwen3-1.7B` and `Qwen/Qwen3-4B`, the same base-model shape as `Qwen/Qwen3-0.6B` and `Qwen/Qwen3-8B`. Not the `*-Instruct` repositories and not the `*-Base` repositories.
- Pins are the Hugging Face `main` commits looked up on 2026-10-28. Do not download weights. Do not float these rows on `main`.
  - `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e`
  - `Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c`
- Both model cards report `license: apache-2.0`. Record that, with `attribution_required: false`, matching the sibling Qwen3 rows.
- New rows go under `qwen3_code` only. `small_code_default` and `strong_code_default` keep today's picks.

### Memory floors
- Keep the `floor_mb` numbers already written for 0.6B, 8B, 14B, and 32B. This phase does not remeasure them.
- Do not invent a `floor_mb` for 1.7B or 4B. `floor_mb` is a required `u32` today and `pick_base` treats every number as a real floor, so an unmeasured rung must not carry a guessed number.
- Add `floor_measured` (serde default `true`, so existing rows stay measured). Set `floor_measured: false` on the 1.7B and 4B rows. `pick_base` skips a row with `floor_measured: false`.
- A size lookup still returns the pin and reports the floor as unmeasured. Measuring those floors belongs to phase 318.

### What the harness calls
- One function on `spoke_base_resolver` reads the `qwen3_code` QLoRA rows and returns the pinned id, revision, license, and whether the floor was measured. Phase 316's size-name path calls that function instead of growing a second table in `reasoning_compare.rs`.
- Accepted spellings are `0.6B`, `1.7B`, `4B`, `8B`, `14B`, and `32B`. `Qwen3-1.7B` is the same size, because phase 316 already passes that form. Anything else is an error. Capability tags such as `qwen3_16g` stay on the memory picker.
- Tests are offline. They read the checked-in overlay. They do not call Hugging Face.
- This phase does not download weights, train, change how a measured row is picked, or add mesh routing.

### Claude's Discretion
None. Every grey area was accepted as recommended.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` — `TrainBase` (`hf_id`, `floor_mb`, `methods`, `license`, `attribution_required`), `load_overlay`, `pick_base`, `resolve_license_class`.
- `mens/config/gpu-specs.yaml` `train_bases.qwen3_code` — existing pinned Qwen3 rows. 0.6B, 8B, 14B, and 32B are already there. 1.7B and 4B are not.
- Phase 316 `reasoning_compare` already loads the `train_bases` overlay and accepts a size name such as `Qwen3-0.6B`.

### Established Patterns
- A Qwen3 row is `hf_id: "Qwen/Qwen3-<size>@<40 lowercase hex>"` with `methods: [qlora]`, `license: apache-2.0`, `attribution_required: false`.
- `ensure_not_placeholder()` rejects an unpinned Qwen3 rung. The guard runs late; entries must be pinned in the file.
- `pick_base` selects the largest `floor_mb` that fits and fails closed when nothing fits. The Qwen3.8-27B row's "UNMEASURED" note is a comment beside a real number, which still participates in selection. Do not copy that pattern for 1.7B and 4B.

### Integration Points
- `load_overlay` is what spoke validation, training dispatch, and phase 316's compare command read.
- `pick_base` callers for `qwen3_code` must ignore `floor_measured: false` rows so 1.7B and 4B cannot win a memory pick.
- Phase 318 (live VRAM selector) and phase 320 (training) consume this ladder. They are not part of this phase.

</code_context>

<specifics>
## Specific Ideas

Checked-in pins from the 2026-10-28 Hugging Face model API (`sha` plus `cardData.license`), no weight files fetched:

- `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` — apache-2.0
- `Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c` — apache-2.0

</specifics>

<deferred>
## Deferred Ideas

- Measuring 1.7B and 4B memory floors, and switching training, serve, and chat onto one live selector — phase 318.
- Training any size — phase 320.
- Mesh routing — phase 319.
- Retuning `small_code_default` and `strong_code_default`.
- Downloading model weights.

</deferred>
