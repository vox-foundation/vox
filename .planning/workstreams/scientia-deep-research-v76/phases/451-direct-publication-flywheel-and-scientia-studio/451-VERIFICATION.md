---
phase: 451-direct-publication-flywheel-and-scientia-studio
verified: 2026-11-15
verifier: rust-verifier
status: passed
requirements: [PUBL-01, PUBL-02, PUBL-03, PUBL-04, STUD-01, STUD-02]
---

# Phase 451 Verification Matrix

## Requirement Coverage
| Requirement | Plan | Verification Evidence | Status |
|---|---|---|---|
| **PUBL-01** | `451-01` | 1-click candidate promotion & 4-layer `ExternalEmissionGate` verified in `vox-scientia --lib` (`flywheel_gate`, `flywheel_exec`, `no_network_publish_symbol_in_review_*`), `vox-research-shim --lib` (`discovery_bridge`), and `vox-orchestrator-mcp` (`gui_ops::scientia::tests::nanopublish_requires_prior_approval`, `scientia` `18 passed`) | Passed |
| **PUBL-02** | `451-01` | `artifact_json_wiped_on_doc_publish: false` & `doc_publish_artifact_retention_rate: 1.0` in `scientia-v76-after-closure.v1.json`; `write_research_doc_preserves_existing_artifact_json` (`gui_ops` `156 passed`); `doc_synthesizer` & `architecture_ssot` tests (`vox-research-shim` `171 passed`, `vox-scientia` `376 passed`) | Passed |
| **PUBL-03** | `451-01` | Idempotent `flywheel_advance_from_db`, hardened LaTeX (`escape.rs`, `render.rs`), deterministic `flate2` tar/gzip (`bundle.rs`), and nanopub hydration (`review_flow.rs`, `scientia_nanopub`) verified in `vox-scientia --lib` (`376 passed`) and `vox-cli-research --lib scientia` (`30 passed`) | Passed |
| **PUBL-04** | `451-01` | `normalize_replay_report_score_maps_pass_partial_and_fail`, `replay_status_event_detail_serializes_measured_score`, and queue/cost USD attribution verified in `vox-scientia --lib` (`376 passed`) and `vox-cli-research --lib scientia` (`30 passed`) | Passed |
| **STUD-01** | `451-01` | `gui_cli_db_spawn_count: 3 -> 0`; zero `'execute_command'` spawns in `PublicationsView`/`ClaimsView` (`rg` + Vitest); 9 `BackendCalls` Tauri commands verified in `vox-gui --bins scientia` (`9 passed`) and `gui_ops::scientia_studio` (`4 passed`) | Passed |
| **STUD-02** | `451-01` | `WorthinessRadarPanel`, `DirectPublicationDrawer`, `DiscoveryInbox`, `DiscoveryReview`, `ClaimsView`, `PublicationsView`, `ScientiaSurface` + UI guards verified via `tsc --noEmit` (`exit 0`) and Vitest (`17` files, `101 passed; 0 failed`) | Passed |

## Key Wave F Command Receipts
- `vox-orchestrator-mcp`: `gui_ops` (`156 passed`), `scientia` (`18 passed`), `scientia_studio` (`4 passed`)
- `vox-cli-research --lib scientia`: `30 passed; 0 failed`
- `vox-gui --bins scientia`: `9 passed; 0 failed`
- `pnpm --dir crates/vox-gui/ui exec tsc --noEmit`: `exit 0`
- Combined Vitest (`17` suites): `17 passed (17)`, `101 passed (101)`
