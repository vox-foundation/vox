---
phase: 433
phase_name: "rust-mcp-seam-closure-single-voxtransport-funnel-honest-ipc"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 433 Learnings: rust-mcp-seam-closure-single-voxtransport-funnel-honest-ipc

## Decisions

### Implemented GuestShareService::list_invites and list_pending_messages in...
Implemented `GuestShareService::list_invites` and `list_pending_messages` in `crates/vox-orchestrator/src/guest_share.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 433-01-SUMMARY.md

---

### Wired vox_guest_share_list (guest.share.list) across crates/vox-orchestr...
Wired `vox_guest_share_list` (`guest.share.list`) across `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`, `input_schemas.rs`, `input_schemas/browser_and_gui.rs`, `dispatch_domains/compiler_analysis.rs`, and `contracts/operations/catalog.v1.yaml` (+ `vox ci operations-sync --target all --write`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 433-01-SUMMARY.md

---

### Aligned wire DTO fields for vox_guest_share_mint (token, session_id, per...
Aligned wire DTO fields for `vox_guest_share_mint` (`token`, `session_id`, `permission`, `scope`, `revoked`), `vox_guest_share_redeem`, `vox_guest_share_submit`, `vox_guest_share_moderate`, and `vox_muse_pool_adopt` (`adopted_variant_id`, `adopted_branch_name`, `pruned_variant_ids`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 433-01-SUMMARY.md

---

### Added std::fs::create_dir_all(parent) before std::fs::write in crates/vo...
Added `std::fs::create_dir_all(parent)` before `std::fs::write` in `crates/vox-gui/src/commands/mercatus.rs` (`save_mercatus_config_to_path`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 433-01-SUMMARY.md

---

### Added "vox_consensus_run" => Duration::from_secs(120) in crates/vox-gui/...
Added `"vox_consensus_run" => Duration::from_secs(120)` in `crates/vox-gui/src/remote/calls.rs` (`deadline_for_tool`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 433-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

