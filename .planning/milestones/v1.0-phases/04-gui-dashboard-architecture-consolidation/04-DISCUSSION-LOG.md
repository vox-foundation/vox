# Phase 4: GUI/Dashboard Architecture Consolidation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-22
**Phase:** 4-GUI/Dashboard Architecture Consolidation
**Areas discussed:** ADR-045 ratification approval, ADR-027 boundary enforcement, CommandCatalog SSOT audit scope, ADR-037 desktop-clause evidence

---

## All four areas (delegated)

User response to the area-selection question was freeform: "check and verify, use your best judgment for all options of phase 4, include all." Interpreted as: select all four areas, but skip per-area interactive Q&A — instead verify current source/doc state directly and record the resulting decision, since the user explicitly asked for verification-backed judgment rather than a menu of options.

| Area | Resolution |
|------|-----------|
| ADR-045 ratification approval | Edit ADR-045 directly (add Status: Accepted line); no separate sign-off gate — see CONTEXT.md D-01 |
| ADR-027 boundary enforcement | **Material finding**: ADR-027 is itself superseded (2026-05-03) by external-frontend-interop-plan-2026.md — boundary must be planned against the current doc, not the retired one. See CONTEXT.md D-02. |
| CommandCatalog SSOT audit scope | One-time audit + fix; CI tooling left as implementer discretion, not required. See CONTEXT.md D-03. |
| ADR-037 desktop-clause evidence | Use ADR-037's own Decision list (desktop-scoped items) as the acceptance evidence; no new criteria invented. See CONTEXT.md D-04. |

**Verification performed before recording decisions:**
- Read `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` frontmatter/body — confirmed `status: "current"`, no explicit Accepted line.
- Read `docs/src/adr/037-tauri-convergence.md` full Decision section — confirmed desktop clause explicitly stated as "remains Accepted" in the ADR's own Status line.
- Read `docs/src/adr/027-dual-track-ui-surfaces.md` — discovered `status: "deprecated"`, superseded by external-frontend-interop-plan-2026.md.
- Read `docs/src/architecture/external-frontend-interop-plan-2026.md` premise/decisions — confirmed it replaces ADR-027's Track A/Track B + `@island` model with bidirectional Vox↔React component interop.
- Searched `crates/vox-cli/src/command_catalog.rs` and `crates/vox-gui/src/commands/*.rs` — confirmed `vox-gui` already calls `vox_cli::command_catalog::build_catalog()` live, no static duplication found in the files searched.

## Claude's Discretion

- Whether to add a permanent CommandCatalog-bypass lint/CI check beyond the one-time audit.
- Exact wording/placement of the ADR-045 "Status: Accepted" line.

## Deferred Ideas

None — discussion stayed within Phase 4's boundary.
