---
phase: 82
phase_name: "operational-gui-surfaces-cli-side-effect-dedicated-floor-ratchet"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 0
  patterns: 5
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 82 Learnings: operational-gui-surfaces-cli-side-effect-dedicated-floor-ratchet

## Decisions

### Verified SettingsView (16/16), ModelsView + MensServePanel + MensTrainin...
Verified SettingsView (16/16), ModelsView + MensServePanel + MensTrainingView + RoutingExplainer (24/24), PoliciesView (6/6), ApprovalsWidget + NeedsYouSurface (10/10), SubAgentsView (3/3), and HarnessHealthView (4/4).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 82-01-SUMMARY.md

---

### Classified 215 CLI operations across vox ci * and vox db * by inspecting...
Classified 215 CLI operations across vox ci * and vox db * by inspecting crates/vox-cli/src/commands/ci/run_body.rs and crates/vox-cli/src/commands/db_cli/mod.rs, ratcheting max_unresolved to 649.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 82-02-SUMMARY.md

---

### Bound audit -> harness-health, db -> vox-search, doctor -> explorer, and...
Bound audit -> harness-health, db -> vox-search, doctor -> explorer, and secrets -> approvals in contracts/gui/surface-registry.v1.yaml, ratcheting dedicated-floor-ratchet.v1.json from 12 to 16.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 82-02-SUMMARY.md

## Lessons

## Patterns

### Operational control surfaces execute live IPC round-trips with optimisti...
Operational control surfaces execute live IPC round-trips with optimistic reconciliation.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 82-01-SUMMARY.md

---

### live-operational-surfaces
live-operational-surfaces

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 82-01-SUMMARY.md

---

### 1-to-1 unique view_key <-> cli_group binding in contracts/gui/surface-re...
1-to-1 unique view_key <-> cli_group binding in contracts/gui/surface-registry.v1.yaml.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 82-02-SUMMARY.md

---

### handler-evidenced-side-effects
handler-evidenced-side-effects

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 82-02-SUMMARY.md

---

### dedicated-surface-ratchet
dedicated-surface-ratchet

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 82-02-SUMMARY.md

## Surprises

