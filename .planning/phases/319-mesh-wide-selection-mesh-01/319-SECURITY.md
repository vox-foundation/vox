---
phase: "319"
slug: "mesh-wide-selection-mesh-01"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-29"
---

> **Port note (2026-10-29):** This is Cursor's record, copied verbatim. On `gsd/mens-compare-316`, the selector names and floors it cites are re-targeted onto `size_select::select_size` and the measured floors. See `319-PORT.md`.

# Phase 319 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

ASVS L1. `workflow.security_block_on` is `high`. The plan-time register has five mitigations (T-319-02 through T-319-06) and two accepted rows (T-319-01, and T-319-SC copied in all three plans). The security auditor checked committed `a5d60a1e8..f2ab6c443` (tip `f2ab6c443`) on 2026-10-29, including review fixes `ceead59d7` (CR-01) and `f2ab6c443` (WR-01), and closed all seven unique rows. No implementation files were changed for this audit. `319-01-SUMMARY.md`, `319-02-SUMMARY.md`, and `319-03-SUMMARY.md` have no Threat Flags section.

Checked after the review fixes: an explicit size on an empty registry or on `NoFreshReading` goes through `explicit_qwen3_code_qlora_for_free_mb` and is not stepped (CR-01). A single `accelerator_free_mb` that does not fit in `u32` is dropped before the max, so a usable sibling is still routed and that error does not abort placement (WR-01). `ReadingExceedsU32` is returned only when every fresh reading overflows; that result does not use `local_free_mb`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Worker heartbeat JSON → `NodeRecord` | A joined worker supplies `accelerator_free_mb`. Omission must erase the previous number. Total VRAM stays copy-if-present. | Free MiB |
| `select_qwen3_mesh` → vox-populi mens | The plugin calls the existing picker. The floor comes from `accelerator_free_mb` only. | Free MiB, size token |
| `select_qwen3_mesh` clock | Callers supply now and the window. The function does not read the wall clock or a new secret. | Unix ms |
| `place_qwen3_code_qlora` `local_free_mb` | The caller supplies this machine's reading. The function does not probe the GPU. | Free MiB |
| `select_best_node` versus the size pick | Script dispatch keeps the existing policy and CPU sort. | Node policy fields |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-319-01 | Spoofing | `select_qwen3_mesh` | medium | accept | `accelerator_free_mb` is an optional field on the same `NodeRecord` heartbeat as `gpu_vram_total_mb` (`node_record.rs:72-77`). Heartbeat still requires `auth_allows_worker_plane` (`vox-populi` `nodes.rs` heartbeat; plugin `nodes.rs` heartbeat). This phase adds no new trust root. A lied-about free-memory reading is the same advertisement trust as total VRAM. Recorded as AR-319-01. | closed |
| T-319-02 | Tampering | `select_qwen3_mesh` | medium | mitigate | `select_qwen3_mesh` (`dispatch.rs:466-519`) builds the floor from `accelerator_free_mb` only, then one call to `explicit_qwen3_code_qlora_for_free_mb` or `select_qwen3_code_qlora_for_free_mb`. The function body does not read `gpu_vram_total_mb`, `memory_free_bytes`, or `capabilities.min_vram_mb`. `route_pick_to_holder` (`:522-529`) filters holders on `accelerator_free_mb` against `pick.floor_mb`. Test `mesh_pick_ignores_total_vram_system_ram_and_min_vram` (`:729-735`) keeps the 14B node. WR-01: readings that fail `u32::try_from` are removed before the max (`:483-509`); `route_pick_to_holder` receives `usable`. `ReadingExceedsU32` is returned only when `usable` is empty and some fresh reading remains (`:496-500`). Test `mesh_pick_drops_overflowing_free_memory_reading` (`:810-833`) picks the 25000 MB node's `14B` beside `u64::MAX`. | closed |
| T-319-03 | Tampering | `merge_optional_node_fields` | high | mitigate | Both copies assign `target.accelerator_free_mb = src.accelerator_free_mb` on every merge, including `None` (`vox-populi` `nodes.rs:216`; plugin `nodes.rs:179`). `gpu_vram_total_mb` stays inside `is_some` in vox-populi (`:212-213`). The plugin copy does not assign `gpu_vram_total_mb`. Tests `merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` in both crates expect `None` after an omit, total VRAM and `memory_free_bytes` unchanged, then `Some(8192)` on the next merge. | closed |
| T-319-04 | Denial of service | `select_qwen3_mesh` | medium | mitigate | Candidates require `accelerator_free_mb` and `now.saturating_sub(last_seen_unix_ms) <= window` (`dispatch.rs:474-480`). `effective_stale_window_ms` (`:460-463`) uses the argument when it is `Some` and `> 0`, otherwise `3_600_000`. Tests `mesh_pick_ignores_stale_and_missing_free_memory` (`:757-767`) and `mesh_pick_unset_window_uses_dispatch_hour` cover `None` and `Some(0)`. The range does not edit `list_nodes` and adds no prune. | closed |
| T-319-05 | Elevation of privilege | `place_qwen3_code_qlora` | medium | mitigate | `place_qwen3_code_qlora` (`dispatch.rs:563-590`) returns `Ok(Mesh)` when `select_qwen3_mesh` succeeds, including a one-node slice (`place_qwen3_only_this_node_is_a_mesh_pick`). `ExplicitDoesNotFit` and `NothingFits` are `Err(other)` and return without calling the local selector (`place_qwen3_explicit_miss_does_not_fall_back_local`). Only an empty registry or `NoFreshReading` reaches the local arm, and only with the supplied `local_free_mb`. `local_free_mb == None` returns `NoFreshReading` (`:579`). `dispatch.rs` does not call `read_accelerator_free_mb` (`place_qwen3_without_local_reading_does_not_probe`). CR-01: the local explicit arm calls `explicit_qwen3_code_qlora_for_free_mb` (`:582-585`) and maps failure to `ExplicitDoesNotFit`. Only a missing explicit size calls `select_qwen3_code_qlora_for_free_mb` (`:587`). Tests `place_qwen3_empty_explicit_32b_does_not_step_down` (`32B` at 25000 MB is `ExplicitDoesNotFit`), `place_qwen3_empty_explicit_14b_does_not_step_up` (`14B` at 70000 MB stays `14B`), and `place_qwen3_stale_or_missing_explicit_size_uses_local_rule`. `explicit_qwen3_code_qlora_for_free_mb` does not step down (`spoke_base_resolver.rs:523-533`; test `:1525-1527`). WR-01: a single overflowing sibling does not return `ReadingExceedsU32`, so `place_qwen3_code_qlora` is not forced onto the fatal arm while a usable node remains. | closed |
| T-319-06 | Tampering | `select_best_node` | medium | mitigate | `select_best_node` (`dispatch.rs:169-198`) filters with `node_passes_dispatch_policy`, applies `req.min_vram_mb` against `capabilities.min_vram_mb`, and sorts by CPU. It does not call `select_qwen3_mesh` or `select_qwen3_code_qlora_for_free_mb`. Test `select_best_node_ignores_accelerator_free_mb` (`:856`) returns the idle node with no free reading while the mesh pick names the busy 70000 MB node. | closed |
| T-319-SC | Tampering | cargo feature enablement | high | accept | Plans 01, 02, and 03 each declare this row. The range adds no crate, npm, or pip package and does not change `Cargo.lock`. The only manifest edit is `features = ["mens"]` on the plugin's existing `vox-populi` dependency (`crates/vox-plugin-populi-mesh/Cargo.toml`). Recorded as AR-319-SC. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-319-01 | T-319-01 | Workers already advertise `gpu_vram_total_mb` on an authenticated heartbeat. `accelerator_free_mb` rides the same `NodeRecord`. This phase does not add a trust root. A heartbeat that lies about free memory has the same advertisement trust as a lied-about total. | phase 319 plan 01 | 2026-10-29 |
| AR-319-SC | T-319-SC | This phase installs nothing. It enables the existing `mens` feature on the workspace `vox-populi` dependency the plugin already has, and calls functions already in that crate. | phase 319 plans 01–03 | 2026-10-29 |

*Accepted risks do not resurface in future audit runs.*

---

## Unregistered Flags

None. The phase summaries have no `## Threat Flags` section. CR-01 and WR-01 are review fixes of T-319-05 and T-319-02; both are present at `f2ab6c443`.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-29 | 7 | 7 | 0 | gsd-security-auditor, ASVS L1, `block_on: high`, range `a5d60a1e8..f2ab6c443` |

The three plan rows named T-319-SC are one accepted supply-chain row here.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-29
