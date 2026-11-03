---
phase: 82-operational-gui-surfaces-cli-side-effect-dedicated-floor-ratchet
verified: 2026-10-16T17:05:00Z
status: passed
score: 5/5 must-haves verified
---

# Phase 82: Operational GUI Surfaces & CLI Side-Effect / Dedicated-Floor Ratchet — Verification

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `SettingsView.tsx` (`Clavis` secrets, OAuth, `Oratio` STT, server connection) executes live round-trips (`OPS-01`) | passed | `crates/vox-gui/tests/real_backend_harness.rs::real_backend_operational_surfaces_settings_models_approvals_policies_subagents_roundtrip` + `SettingsView.test.tsx` (16/16 passed) |
| 2 | `ModelsView.tsx` (`MensServePanel`, `MensTrainingView`, `RoutingExplainer`) reflects live model catalog state (`OPS-02`) | passed | `real_backend_operational_surfaces_settings_models_approvals_policies_subagents_roundtrip` (`/models/preferences`, `/models/snapshot`, `/models/routing-summary`, `models_preference_set`) + `ModelsView.test.tsx` (9/9), `MensServePanel.test.tsx` (3/3), `MensTrainingView.test.tsx` (5/5), `RoutingExplainer.test.tsx` (7/7) passed |
| 3 | `ApprovalsView`, `PoliciesView`, `NeedsYouSurface`, and `SubAgentsView` execute real control round-trips (`OPS-03`) | passed | `real_backend_operational_surfaces_settings_models_approvals_policies_subagents_roundtrip` (`hopper_submit` -> `hopper_reprioritize` -> `hopper_mark_done` -> `/hopper/list`, `/orchestrator/approvals`, `load_policy_registry`) + `PoliciesView.test.tsx` (6/6), `NeedsYouSurface.test.tsx` (8/8), `ApprovalsWidget.test.tsx` (2/2), `SubAgentsView.test.tsx` (3/3), `HarnessHealthView.test.tsx` (4/4) passed |
| 4 | At least 150 additional CLI rows in `contracts/operations/catalog.v1.yaml` receive handler-evidenced `side_effect_class`, ratcheting `side-effect-ratchet.v1.json` to `<= 756` (`OPS-04`) | passed | 257 CLI rows classified (`906 -> 649`); `contracts/capabilities/side-effect-ratchet.v1.json` `max_unresolved` tightened to `649` (`<= 756`); `vox ci capability-index` exits 0 |
| 5 | `audit`, `db`, `doctor`, and `secrets` promoted to dedicated GUI bindings in `contracts/gui/surface-registry.v1.yaml`, raising `dedicated-floor-ratchet.v1.json` from `12` to `>= 16` (`OPS-05`) | passed | `contracts/gui/dedicated-floor-ratchet.v1.json` has 16 members (`audit`, `commands`, `config`, `db`, `doctor`, `gamify`, `memory`, `mens`, `model`, `populi`, `repo`, `research`, `scientia`, `secrets`, `skill`, `speech`); `vox ci gui-surface-registry` and `vox ci gui-surface-coverage` exit 0 |

## Requirements Coverage

| Requirement | Status | Verification Evidence |
|-------------|--------|-----------------------|
| `OPS-01` | passed | `real_backend_harness.rs` + `SettingsView.test.tsx` (16/16 passed) |
| `OPS-02` | passed | `real_backend_harness.rs` + `ModelsView.test.tsx`, `MensServePanel.test.tsx`, `MensTrainingView.test.tsx`, `RoutingExplainer.test.tsx` (24/24 passed) |
| `OPS-03` | passed | `real_backend_harness.rs` + `PoliciesView.test.tsx`, `NeedsYouSurface.test.tsx`, `ApprovalsWidget.test.tsx`, `SubAgentsView.test.tsx`, `HarnessHealthView.test.tsx` (23/23 passed) |
| `OPS-04` | passed | `contracts/capabilities/side-effect-ratchet.v1.json` (`max_unresolved: 649`), `vox ci capability-index` |
| `OPS-05` | passed | `contracts/gui/dedicated-floor-ratchet.v1.json` (16 members), `vox ci gui-surface-registry` |
