# Phase 317: Size Ladder - Context

**Gathered:** 2026-10-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Every Qwen3 dense size (0.6B, 1.7B, 4B, 8B, 14B, 32B) is a pinned, licensed rung that the trainer and the harness can name. A rung whose memory floor has not been measured on hardware says so. Selection by live free memory is Phase 318; mesh is 319; measuring floors by training is 320.

</domain>

<decisions>
## Implementation Decisions

### Size ladder shape
- The ladder is the existing `train_bases.qwen3_code` list in `mens/config/gpu-specs.yaml`, extended with `Qwen/Qwen3-1.7B` and `Qwen/Qwen3-4B` rungs (no new table). Phase 316's `--base Qwen3-<size>` resolution reads it already.
- Candidate pins (from the HF API, must be re-confirmed with `git ls-remote https://huggingface.co/Qwen/<repo> main` before committing): Qwen3-1.7B `70d244cc86ccca08cf5af4e1e306ecf908b1ad5e`, Qwen3-4B `1cfa9a7208912126459214e8b04321603b3df60c`; both Apache-2.0, `attribution_required: false`. Re-confirm the existing 0.6B/8B/14B/32B pins the same way and record the evidence.
- Each rung carries `floor_measured: true|false`; when true, also `measured_on` (host/accelerator) and `peak_mb`. All current floors are estimates, so every rung is `floor_measured: false` until Phase 320 measures it. The resolver/harness surfaces "floor unmeasured" wherever a rung is reported.
- New floor_mb estimates for 1.7B and 4B follow the file's existing method (bare NF4 weight size × the ~2.25–2.96× multiplier the other rungs carry), with the derivation noted in a comment.
- A hermetic contract test asserts all six dense sizes are present, each with a 40-lowercase-hex SHA and a license, and that the size names resolve (e.g. via the Phase 316 `resolve_base_pin`). No network in CI.
- `Qwen/Qwen3.8-27B` (not a real Qwen3 dense size, no license) and review finding WR-03 (vision/MTP-only shard rule) stay out of the dense ladder; both are recorded as deferred.

### Claude's Discretion
- Exact struct/field plumbing for `floor_measured` through `spoke_base_resolver.rs` (`BaseCandidate`), `vox-plugin-mens-candle-core/src/model_card.rs`, and the 316 detect code; ordering of rungs; whether the 14B LoRA rung keeps its separate entry.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `mens/config/gpu-specs.yaml` `train_bases` (`qwen3_code`, `small_code_default`, `strong_code_default`, …) with `hf_id@sha`, `floor_mb`, `methods`, `license`, `attribution_required`.
- `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`: `BaseCandidate { floor_mb, … }`, largest-fitting-rung pick, `resolve_license_class`.
- `crates/vox-populi/tests/training_presets_yaml_contract.rs` and the integration test `gpu_specs_agentic_default_qwen3_hf_ids_have_real_sha_format` — pin-format guards to extend.
- `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` `resolve_base_pin` (Phase 316) — size name → pinned base.
- `ensure_not_placeholder()` rejects unpinned Qwen3 revisions at train/dispatch time.

### Established Patterns
- Comments in gpu-specs.yaml document floor derivations and license decisions with dates.
- Contract tests read the YAML hermetically.

### Integration Points
- Trainer base pick (`crates/vox-ml-cli/src/commands/schola/train/gpu.rs`), harness/model card (`model_card.rs`), Phase 316 compare `--base`.

</code_context>

<specifics>
## Specific Ideas

- The user wants real adapters trained in Phase 320 on this Mac; the ladder must be ready for that (pins real, names resolvable, floors honestly marked unmeasured).

</specifics>

<deferred>
## Deferred Ideas

- WR-03 (316 review): `snapshot_complete` requires every indexed shard while `vox mens download` skips vision/MTP-only shards — affects non-dense rungs only.
- Removing or relicensing the `Qwen3.8-27B` rung in `strong_code_default`.

</deferred>
