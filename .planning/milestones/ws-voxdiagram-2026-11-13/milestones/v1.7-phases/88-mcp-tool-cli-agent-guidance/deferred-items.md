# Deferred items (phase 88)

- `vox-orchestrator-mcp registry::profile_backcompat_tests::{snapshot_is_todays_default_feature_registry,dev_profile_advertises_the_snapshot}`
  fail at the phase base independent of plan 88-01: `tests/fixtures/dev-advertised-tools.default-features.txt` lacks 14 tools that the
  committed registry already advertises (`vox_gamify_{companions,discovery_record,due_actions,leaderboard,profile,record_event}`,
  `vox_gui_run_{start,finish}`, `vox_hopper_{cancel,mark_done,reprioritize,submit}`, `vox_plan_node_upsert`, `vox_plan_nodes_approve`).
  Plan 88-01 added exactly `vox_validate_diagram` to both sides; the remaining 14-tool diff is unrelated. Out of scope; not fixed.

- `vox ci profile-parity` (plan 88-03 Task 3): `lean: forbidden crate vox-gamify present in lean build`. vox-gamify reaches the lean
  `vox-cli` tree through `vox-cli-core` (optional `db` feature, unchanged since `82b24d2d4`), `vox-lsp` and `vox-orchestrator`;
  `cargo tree -p vox-diagram` contains no gamify, and the phase's only manifest change is `vox-orchestrator-mcp -> vox-diagram`
  (a leaf crate). Not caused by phase 88; not fixed.
  status: acknowledged
