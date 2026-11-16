# Phase 82: Operational GUI Surfaces & CLI Side-Effect / Dedicated-Floor Ratchet - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning & verification

<domain>
## Phase Boundary

Deliver Milestone v1.6 Phase 82 (`OPS-01..05`):
- **OPS-01**: Wire `SettingsView` and `ModelsView` end-to-end against real `vox_gui_settings_save`, `vox_gui_settings_models_save`, `vox_gui_settings_secret_set`, `vox_gui_settings_secret_clear`, `vox_gui_models_list`, and `vox_gui_models_select` with persistent storage and doctor verification.
- **OPS-02**: Wire `ApprovalsView`, `PoliciesView`, and `NeedsYouSurface` against real `vox_gui_approvals_decide`, `vox_gui_policies_save`, and `vox_gui_attention_resolve` with optimistic state reconciliation and live counter updates.
- **OPS-03**: Wire `SubAgentsView` and `HarnessView` against real `vox_gui_subagents_spawn`, `vox_gui_subagents_message`, `vox_gui_subagents_terminate`, and `vox_gui_harness_snapshot` with live transcript drill-down.
- **OPS-04**: Classify ≥ 200 additional CLI operations in [`contracts/operations/catalog.v1.yaml`](file:///Users/brbrainerd/dev/vox/contracts/operations/catalog.v1.yaml) with explicit `side_effect_class` (`read_only | writes_files | spawns_process | network | destructive`), tightening [`contracts/capabilities/side-effect-ratchet.v1.json`](file:///Users/brbrainerd/dev/vox/contracts/capabilities/side-effect-ratchet.v1.json) `max_unresolved` from `956` to `≤ 756`.
- **OPS-05**: Promote ≥ 4 high-value CLI command groups (`audit`, `db`, `doctor`, `secrets`) into dedicated or live-backend GUI surface bindings in [`contracts/gui/surface-registry.v1.yaml`](file:///Users/brbrainerd/dev/vox/contracts/gui/surface-registry.v1.yaml) and ratchet [`contracts/gui/dedicated-floor-ratchet.v1.json`](file:///Users/brbrainerd/dev/vox/contracts/gui/dedicated-floor-ratchet.v1.json) from `12` to `≥ 16`.
</domain>

<decisions>
## Implementation Decisions

### Operational Surface Wiring (`OPS-01..03`)
- `SettingsView` and `ModelsView` use `guiInvoke('vox_gui_settings_save' | 'vox_gui_settings_models_save' | 'vox_gui_settings_secret_set' | 'vox_gui_settings_secret_clear' | 'vox_gui_models_list' | 'vox_gui_models_select')` backed by `crates/vox-gui/src/gateway_ipc.rs` and `crates/vox-gui/src/workspace_snapshot.rs`.
- `ApprovalsView`, `PoliciesView`, and `NeedsYouSurface` use `vox_gui_approvals_decide`, `vox_gui_policies_save`, and `vox_gui_attention_resolve` with optimistic reconciliation and live pending counter updates.
- `SubAgentsView` and `HarnessView` use `vox_gui_subagents_spawn`, `vox_gui_subagents_message`, `vox_gui_subagents_terminate`, and `vox_gui_harness_snapshot` with live subagent transcript drill-down.

### CLI Side-Effect Classification & Ratchet (`OPS-04`)
- Classify every handler in `vox ci *` (`crates/vox-cli/src/commands/ci/run_body.rs`, 160 CLI operations) and `vox db *` (`crates/vox-cli/src/commands/db_cli/mod.rs`, 55 CLI operations) in `contracts/operations/catalog.v1.yaml`, totaling 215 newly classified CLI operations.
- Run `vox ci operations-sync --write`, `vox ci capability-index --tighten`, and `vox ci capability-index --write` to update the capability index and tighten `side-effect-ratchet.v1.json` below `756`.

### Dedicated-Floor Promotion & Ratchet (`OPS-05`)
- Bind `audit` → `harness-health` (`curated_decorator`), `db` → `memory` (`live_backend`), `doctor` → `settings` (`curated_decorator`), and `secrets` → `settings` (`live_backend`) in `contracts/gui/surface-registry.v1.yaml` with `nav_label: null` so `PrimaryNavViewKey` parity remains untouched while promoting 4 groups out of `generic_ fallback`.
- Run `vox ci gui-surface-registry --write` and `vox ci gui-surface-registry --tighten` to ratchet `contracts/gui/dedicated-floor-ratchet.v1.json` from `12` to `16`.
</decisions>
