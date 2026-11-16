# Phase 319: Mesh-Wide Selection (`MESH-01`) - Context

**Gathered:** 2026-10-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Node heartbeats carry current free accelerator memory. One mesh pick chooses the largest measured `qwen3_code` QLoRA size that fits some eligible node's free memory, routes that job to a node that can hold it, and titrates only the automatic pick. A stale or missing reading cannot win. When no node is eligible, the caller uses the phase 318 selector on this machine.

This phase does not train, score, download weights, measure the 1.7B or 4B floors, or change phase 322.

</domain>

<decisions>
## Implementation Decisions

### What a heartbeat reports
- Add `accelerator_free_mb` beside `gpu_vram_total_mb` on the node record. Fill it with the phase 318 CUDA-then-Metal reading (`query_nvidia_smi_vram` memory.free, otherwise `monitor_metal` memory_free_mb). Leave `memory_free_bytes` as system RAM.
- Store megabytes, the same unit as `floor_mb`, so the mesh pick can call `pick_base` without a conversion.
- When the probe fails, omit `accelerator_free_mb`. A missing reading is not zero and is not the total.
- Keep `gpu_vram_total_mb` as information only. Selection uses the free field.

### Which node runs the job
- One automatic pick: the largest measured `qwen3_code` QLoRA floor that fits some eligible node's `accelerator_free_mb`, through the same `pick_base` path as phase 318. Route to a node whose free memory holds that floor.
- When several nodes fit that floor, keep today's tie-break: lowest `cpu_usage_pct`. Quarantine, maintenance, required labels, and donation policy still apply.
- The fit test is `accelerator_free_mb` against the chosen floor. `gpu_vram_total_mb`, system `memory_free_bytes`, and `capabilities.min_vram_mb` do not decide the size.
- An explicit `resolve_qwen3_size` spelling that no eligible node can hold is an error. Only the automatic pick titrates to the next smaller measured row.

### Stale or missing free memory
- A node is a size candidate only when `accelerator_free_mb` is present and `last_seen_unix_ms` is inside `server_stale_prune_ms`. Tests inject the clock and the window. When that window is unset, use the one-hour bound already used for dispatch results (`3_600_000` ms in `dispatch.rs`). Do not add a new env var.
- A live node that omitted `accelerator_free_mb` cannot win. Total VRAM, system `memory_free_bytes`, and zero are not substitutes.
- A later heartbeat that omits the free field clears `accelerator_free_mb` on that node. The old number does not stay.
- When every candidate is stale or missing, the mesh size pick returns an error. It does not choose a node or a size.

### No mesh
- An empty registry, or a mesh size pick that errors because no node has a fresh `accelerator_free_mb`, uses the phase 318 selector on this machine. `select_best_node` does not call that selector itself.
- The local path is the phase 318 pick: this machine's free-memory reading, one measured `qwen3_code` QLoRA size and preset, step-down only on the automatic pick, explicit sizes refused.
- A registry that contains only this node, with a fresh free reading, is a mesh pick of this node. "No mesh" means no eligible node.
- Tests inject node records, clocks, and free-memory readings. Cover two nodes, an explicit size that no node can hold, a stale or missing reading, and an empty registry whose caller still resolves locally from an injected reading. No GPU probe, no Hugging Face, no weight download.

### Claude's Discretion

None. All four areas were accepted as recommended.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `select_qwen3_code_qlora_for_free_mb`, `pick_base`, `resolve_qwen3_size`, and `read_accelerator_free_mb` in `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`. Unmeasured 1.7B and 4B rows stay `floor_measured: false` and cannot win.
- `NodeRecord` in `crates/vox-populi-types/src/node_record.rs` already has `gpu_vram_total_mb`, `last_seen_unix_ms`, and system-RAM `memory_free_bytes`.
- `filter_registry_by_max_stale_ms` and `server_stale_prune_ms` (`VoxMeshServerStalePruneMs`) already age nodes by `last_seen_unix_ms`. An unset window does not prune the registry listing.
- `select_best_node` in `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs` filters quarantine, maintenance, labels, `capabilities.min_vram_mb`, and donation policy, then sorts by `cpu_usage_pct`. Dispatch results use a one-hour TTL.

### Established Patterns
- Phase 318 tests inject free-memory readings. A `cargo test` that runs 0 tests is not a pass; the module that holds the picker compiles with `--features mens`.
- Heartbeat merge in `crates/vox-populi/src/transport/handlers/nodes.rs` copies `gpu_vram_total_mb` only when the source sets it. The new free field must clear on omission instead of sticking.
- `vox-plugin-populi-mesh` already depends on `vox-populi`. Do not add a workspace crate edge. Call `pick_base` through that dependency. Enabling the existing `mens` feature on that dependency is allowed. Do not copy the ladder into the plugin.

### Integration Points
- Heartbeat writer fills `accelerator_free_mb` from the phase 318 reading. Tests inject the value and do not call the probe.
- Mesh dispatch returns the chosen node, the size, and the preset. The caller that sees an empty registry or a mesh error uses `select_qwen3_code_qlora_for_free_mb` on an injected or live local reading.
- Training's phase 318 path in `crates/vox-ml-cli/src/commands/mens/training_selection.rs` stays the local selector. This phase does not retune `small_code_default` or `strong_code_default`.

</code_context>

<specifics>
## Specific Ideas

The fit number is `accelerator_free_mb` in megabytes. The one-hour fallback bound is the `3_600_000` ms already written next to dispatch results. `memory_free_bytes` stays system RAM.

</specifics>

<deferred>
## Deferred Ideas

None. Training runs, score reports, weight downloads, 1.7B and 4B floor measurements, and phase 322 stay out of this phase. Registry listing keeps today's behavior when `server_stale_prune_ms` is unset; only the size pick applies the one-hour bound in that case.

</deferred>
