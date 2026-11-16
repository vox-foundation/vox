# Phase 318: Live VRAM Selector (`SIZE-02`) - Context

**Gathered:** 2026-10-29
**Status:** Ready for planning

<domain>
## Phase Boundary

One selector reads current free memory and picks the largest measured Qwen3 dense size whose checked-in floor fits. Training, local serve, and chat use that pick. The hub and every spoke follow the same size. An explicit size overrides the pick and is refused when it does not fit. This phase does not measure 1.7B or 4B, download weights, train, or change mesh routing.

</domain>

<decisions>
## Implementation Decisions

### Which rungs can be picked
- `1.7B` and `4B` cannot win a pick. A row with `floor_measured: false` stays ineligible. This phase does not invent a `floor_mb`.
- CUDA free memory is `memory.free` from `nvidia-smi`. Metal free memory is `monitor_metal`'s `memory_free_mb`: `recommendedMaxWorkingSetSize` minus `currentAllocatedSize`. Do not use `hw.memsize` times the wired-limit percent, and do not use the path in `get_system_vram_info` that copies the Metal budget into both total and free.
- The comparison is the checked-in `floor_mb` of measured `qwen3_code` QLoRA rows. The largest floor that fits wins.
- If nothing fits, the pick fails closed. An explicit size that does not fit is refused. There is no fallthrough to `Qwen3-0.6B`.

### One pick for the hub and the spokes
- Extend `pick_base` on `spoke_base_resolver`. `select_optimal_local_model` calls that function. Do not keep a second ladder in `auto_select.rs`.
- One dense size. Every spoke's base and preset use that size. An 8B spoke preset is not left in place when the hub pick is 14B.
- Call sites in this phase: training's `resolve_base_model` when no size was named, chat's auto-select in `crates/vox-cli/src/commands/chat.rs`, and the GUI models host. Mesh routing stays in phase 319.
- Remove the `mens/runs/qwen3_27b_metal_check/*` and `vox-mens-8b-v0.6` directory map in `select_tier_for_vram`. Unreadable free memory fails closed.
- `vox-orchestrator` already depends on `vox-populi` behind the `runtime` feature (`vox-populi/mens`). Call the selector through that existing feature. Do not add a new workspace crate edge.

### Memory drops before load
- Read free memory again immediately before load, with the same free-memory reading. If the chosen `floor_mb` no longer fits, take the next smaller measured `qwen3_code` QLoRA row that does.
- The result names the original size, the size actually loaded, and the free-memory reading that forced the change.
- If the smallest measured row no longer fits, fail closed and do not load. `1.7B` and `4B` stay ineligible, so they are not the last step.
- Tests pass two free-memory readings, one for the pick and one for the load. They do not probe the GPU. A second reading that still fits the original size does not step down.

### An explicit size
- Override names are the ones `resolve_qwen3_size` already accepts: `0.6B`, `1.7B`, `4B`, `8B`, `14B`, `32B`, plus `Qwen3-<size>` and `Qwen/Qwen3-<size>`. That size is the candidate. It is not stepped down.
- Unmeasured, or a `floor_mb` above the current free memory, is an error. The pre-load re-read uses the same rule: if free memory dropped below that floor, refuse. Do not substitute a smaller size.
- A caller that passes `strong_code_default` or `small_code_default` keeps `pick_base` on that tag. Only a missing size uses the new selector. This phase does not retune those tags.
- Tests inject the free-memory readings. No GPU probe, no Hugging Face, no weight download, no training run, and no mesh heartbeat changes. `1.7B` and `4B` stay unmeasured.

### Claude's Discretion
None. Every grey area was accepted as recommended.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `spoke_base_resolver::pick_base` already skips `floor_measured: false` and selects the largest `floor_mb` that fits.
- `spoke_base_resolver::resolve_qwen3_size` names the six dense sizes and the `Qwen3-<size>` / `Qwen/Qwen3-<size>` aliases.
- `spoke_base_resolver::resolve_base_model` picks by capability tag plus a VRAM override. A concrete id containing `/` passes through.
- `macos_metal::monitor_metal` reports `memory_free_mb` as the recommended working set minus `currentAllocatedSize`.
- `vram_autodetect::query_nvidia_smi_vram` already parses `memory.free`.

### Established Patterns
- `select_tier_for_vram` in `crates/vox-orchestrator/src/models/auto_select.rs` maps a gigabyte total onto `mens/runs/qwen3_27b_metal_check/*` and falls back to `vox-mens-8b-v0.6`, including for zero and NaN. Those directories are the fixed tiers this phase removes.
- Chat (`crates/vox-cli/src/commands/chat.rs`) and the GUI (`crates/vox-gui/src/commands/models_host.rs`) both call `select_optimal_local_model`. They should keep that call. The function's body changes.
- `live_pressure_budget_bytes` is a `vm_stat` estimate and its own comment says it does not replace the Metal driver advisory. The accepted Metal reading is `monitor_metal`, not that estimate and not `hw.memsize`.

### Integration Points
- Training with no named size goes through `resolve_base_model`. Named capability tags stay on `pick_base` for that tag.
- The pre-load re-read has to sit on the path that actually loads, not only inside the chat auto-select return value.
- Phase 319 consumes this selector for mesh routing. It is not part of this phase.

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the accepted answers. The Metal number is the existing `monitor_metal` free-memory field. The CUDA number is the existing `nvidia-smi` `memory.free` field.

</specifics>

<deferred>
## Deferred Ideas

- Measuring floors for `1.7B` and `4B`.
- Mesh heartbeats and mesh-wide selection — phase 319.
- Training any size — phase 320.
- Retuning `small_code_default` and `strong_code_default`.
- Downloading weights.

</deferred>
