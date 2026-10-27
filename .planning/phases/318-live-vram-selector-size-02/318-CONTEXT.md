# Phase 318: Live VRAM Selector - Context

**Gathered:** 2026-10-27
**Status:** Ready for planning

<domain>
## Phase Boundary

One selector reads current free accelerator memory, picks the largest Qwen3 ladder size (Phase 317 `train_bases.qwen3_code`) whose footprint fits, and the hub and every spoke in an invocation follow that one size. Training, local serve, and chat use it; the fixed `auto_select.rs` tiers go. Mesh-wide selection is Phase 319; measuring floors by training is Phase 320.

</domain>

<decisions>
## Implementation Decisions (all accepted 2026-10-27)

- **D-01 Free memory.** Metal: `accel_budget::query_accel_budget().working_set_bytes` (min of recommendedMaxWorkingSetSize and vm_stat free+inactive+speculative minus max(15%, 2 GiB); `accel_budget.rs:64-78`, `macos_metal.rs:143-156`) — no extra margin on top. CUDA: live free VRAM (nvidia-smi `memory.free` in vox-populi; NVML plugin `vram_free_mb` where registered) × 0.88 (`memory_model.rs` `SAFETY_FRACTION`). No live reading → refuse; never fall back to a default size. Do not use `vram_autodetect` total-as-free or `probe_gpu().vram_mb` (static ceiling).
- **D-02 Ladder + location.** The selector picks from the six-size `qwen3_code` ladder only, judging fit by each size's QLoRA rung `floor_mb` (the 14B/32B LoRA rungs are method variants, not sizes). It lives in vox-populi `mens::tensor` (layer 3); vox-orchestrator, vox-cli and vox-ml-cli reach it through existing edges. No new crate edges. Spoke `base.model` tags stop choosing the size.
- **D-03 Presets follow size.** 0.6B→qwen3_dev_cpu, 8B→qwen3_16g, 14B→qwen3_24g, 32B→qwen3_96g; add new presets for 1.7B and 4B (update `training-presets.v1.yaml`, `KNOWN_PRESETS`, and `detect_qwen_size_class` classes).
- **D-04 Explicit size.** Via existing inputs: `--model`, `--base Qwen3-<size>`, `VOX_MENS_DEFAULT_MODEL`. Size-name → pin resolution moves from vox-ml-cli `reasoning_compare/detect.rs::resolve_base_pin` into vox-populi (vox-ml-cli then calls it). No new flag. An explicit ladder size that doesn't fit is refused (not shrunk); training's existing force override still applies. A concrete id not on the ladder passes through unchanged (no floor to check).
- **D-05 Step-down.** Re-read free memory once right before download/load (training: `run_train.rs` before the HF download; serve: before `worker.rs` load). If the auto pick no longer fits, step down to the largest fitting size and log why. Never step down an explicit size. The post-download refusal in `gpu.rs` stays as a backstop.
- **D-06 Hub + spokes.** One pick per pipeline/train invocation; every spoke in it inherits that size (and its preset). Not persisted across runs.
- **D-07 Inference consumers.** Chat (`auto-local`), serve and the GUI use `floor_mb` as footprint (conservative). The six fake `auto_select.rs` tiers and their duplicate registry entries (`registry.rs:777-785` + test) are deleted; `AutoModelSelection` changes shape with its TS types and mocks (`ui/src/transport.ts`, `ui/e2e/lib/tauriMock.ts`).
- **D-08 Cloud.** `train_arm.rs` fixed 24 GB cloud path is left for Phase 319.

### Claude's Discretion
- Selector API shape (e.g. `select_size(free_mb, explicit) -> Result<Pick>` plus a `FreeMemory` reading type), log wording, how the GUI renders the pick, and plan splitting.

</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-orchestrator/src/models/auto_select.rs:53-100` (fake tiers), `:104-185` total-memory detection; callers `vox-cli/src/commands/chat.rs:571-576`, `vox-gui/src/commands/models_host.rs:33-38`; re-exports `models/mod.rs:34`, `models/select.rs:29`.
- `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`: `TrainBase`, `pick_base` (:126, fails closed), `resolve_base_model` (:303, uses total), `resolve_metal_default_base` (:331, `agentic_default`).
- Resolver callers: `schola/train/run_train.rs:192-219`, `mens/training_selection.rs:61`, `populi/train_arm.rs:739/785/827`, `mens/pipeline.rs:466`.
- Spokes: `mens/config/domain-profiles.yaml` (all `qwen3_16g`); `HubConfig` (`domain_profiles.rs:9`) has only `embedder`.
- Serve: `vox mens serve` requires `--model` (`populi/dispatch.rs:351`), loads at `ai/serve/worker.rs:90`.
- Crate edges: vox-gui→vox-populi and vox-cli→vox-ml-cli are NOT allowed; go through vox-orchestrator / vox-populi.
- vox-cli does not compile on this branch (uncommitted vox-orchestrator-mcp work in another tree); verify with scoped crates, not `vox`.

</code_context>

<deferred>
## Deferred Ideas

- Per-rung inference footprint (`infer_mb`) instead of `floor_mb` for chat/serve.
- Persisting the hub size across separate runs (`hub.base`).
- Measuring the CUDA context/allocator overhead to tune the 0.88 margin; vm_stat reliability on macOS 26 (purgeable/compressed memory).
- Cloud 24 GB fixed path (Phase 319).

</deferred>
