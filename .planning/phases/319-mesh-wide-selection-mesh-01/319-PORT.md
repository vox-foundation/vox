# Phase 319: Port Note

**Ported:** 2026-10-29, from `cursor/t3-deeper-extraction-v55-510f` (close `38001f8dc`, fixes `ceead59d7` CR-01 and `f2ab6c443` WR-01) onto `gsd/mens-compare-316`.
**Decision:** keep this branch's phases 317 and 318 (`size_select.rs`, measured floors). Port only MESH-01. Cursor's 317, 318 and 320 work is superseded and was not ported.

The other `319-*.md` files are Cursor's record, copied verbatim. They cite Cursor's phase 318 selector and placeholder floors. On this branch, read them through this table.

## Selector mapping

| Cursor's record says | This branch has |
|---|---|
| `select_qwen3_code_qlora_for_free_mb(u32)` (automatic pick) | `size_select::select_size(overlay, Some(&FreeMemory), None, false)` |
| `explicit_qwen3_code_qlora_for_free_mb` / `explicit_qwen3_size_for_free_mb` / `resolve_qwen3_size` spelling | `size_select::select_size(overlay, Some(&FreeMemory), Some("Qwen3-4B"), false)`. An explicit size is refused, never shrunk. A non-ladder base (`gpt2`) is `NotALadderSize`, a new error variant, because it has no floor to route on. |
| `read_accelerator_free_mb()` (heartbeat writer) | `size_select::read_free_memory().map(\|f\| f.mb)`. Metal live working set, or nvidia-smi free x0.88. |
| `Qwen3LivePick { size_token, preset, floor_mb, hf_id }` | `SizeRung { size, preset, floor_mb, hf_id }`. `Qwen3MeshSelection` carries `rung: SizeRung`. `Qwen3Placement::Local` carries a `SizeRung`. |
| `place_qwen3_code_qlora(.., local_free_mb: Option<u32>)` | `place_qwen3_code_qlora(overlay, .., local_free: Option<FreeMemory>)`. The overlay is injected, so the plugin needs no `vox_corpus` call. |
| `accelerator_free_mb_from_reading` helper, plus its test | Not ported. It was `Option::map`. |
| `explicit_qwen3_code_qlora_for_free_mb` in `spoke_base_resolver.rs`, plus its test | Not ported. `select_size` covers it. |
| "Unmeasured 1.7B and 4B rows stay `floor_measured: false`" | Wrong here. 0.6B, 1.7B and 4B are measured. 8B and larger are estimates. |

## Floor mapping (`mens/config/gpu-specs.yaml` `qwen3_code` QLoRA)

| Size | Preset | Cursor test floor | This branch |
|---|---|---|---|
| 0.6B | qwen3_dev_cpu | 2000 | 15319 measured |
| 1.7B | qwen3_6g | n/a | 27007 measured |
| 4B | qwen3_12g | n/a | 33037 measured |
| 8B | qwen3_16g | 12000 | 122876 estimate |
| 14B | qwen3_24g | 20000 | 195382 estimate |
| 32B | qwen3_96g | 60000 | 295822 estimate |

Cursor's tests used 70000 (expecting 32B) and 25000 (expecting 14B). The ported tests use only measured rungs: 34000 free picks 4B (33037), and 28000 free picks 1.7B (27007). The test cases and their assertions are otherwise the same, one for one.

## Unchanged from Cursor

- `NodeRecord.accelerator_free_mb`
- Both heartbeat merges, which clear the field on omission
- `node_passes_dispatch_policy`, shared with `select_best_node`
- The stale window, with the one-hour default
- The u32-overflow drop before the max
- The lowest-CPU tie-break
- The `mens` feature on the existing `vox-plugin-populi-mesh` -> `vox-populi` edge (319-CONTEXT allows this; no new edge)
