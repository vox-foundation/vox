# Phase 4: GUI/Dashboard Architecture Consolidation - Context

**Gathered:** 2026-09-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Ratify ADR-045 as the accepted GUI/dashboard architecture decision, verify `vox-gui`'s command surface stays fully sourced from `vox-cli`'s `CommandCatalog` SSOT with no orphaned/duplicated definitions, establish and audit the Vox-native/React-interop UI boundary against the *current* governing doc, and confirm ADR-037's desktop-convergence clause is actually implemented (not just decided). Mobile-target work (adr-NNN, React Native/Expo) is explicitly out of boundary.

</domain>

<decisions>
## Implementation Decisions

User delegated all four discussion areas to Claude's judgment ("check and verify, use your best judgment... include all") rather than a per-area Q&A. Each decision below is grounded in a direct check of the current source/doc state, not assumption.

### ADR-045 ratification (GUI-01)
- **D-01:** The plan edits `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` directly — adds an explicit `**Status**: Accepted (2026-09-22)` body line (matching ADR-037's own convention) and updates its frontmatter `status: "current"` → `status: "accepted"`. No separate human sign-off gate: ADR-045 is already the sole non-contradicted dashboard/GUI decision (ADR-024 confirmed superseded, `vox-dashboard` confirmed absent from the tree), and PROJECT.md already explicitly names ratifying it as this milestone's Active scope — this phase's job IS that ratification. — **Reversibility:** costly — reverting to non-accepted status later would need another explicit doc edit/supersession, not a code change, but is not itself a breaking migration.

### ADR-027 is stale — corrected boundary source (GUI-03)
- **D-02:** VERIFIED: `docs/src/adr/027-dual-track-ui-surfaces.md` (the doc GUI-03 cites) is itself `status: "deprecated"`, superseded 2026-05-03 by `docs/src/architecture/external-frontend-interop-plan-2026.md`. The retired ADR-027 model (Track A/Track B, `@island` as the interop bridge, compile-time-error on mixing) no longer reflects reality: `@island` is retired entirely, replaced by bidirectional Vox↔React component-level interop (a Vox component can import/render a React component and vice versa, no island primitive). **The plan must establish/audit the Vox-native vs. React/TanStack boundary against `external-frontend-interop-plan-2026.md` and `contracts/frontend/*` (its own stated normative-rules location), not against ADR-027's retired Track A/B split.** REQUIREMENTS.md's GUI-03 citation of "ADR-027" should be treated as historical context only when planning, not as the current rule source. — **Reversibility:** reversible — this is a documentation/audit-target correction, not a structural code decision.

### CommandCatalog SSOT audit scope (GUI-02)
- **D-03:** One-time audit + fix, not new permanent CI tooling, is in scope for this phase. VERIFIED: `vox-gui` already sources its catalog live at runtime — `crates/vox-gui/src/commands/catalog.rs::get_command_catalog()` calls `vox_cli::command_catalog::build_catalog()` directly (no static/stale copy). The audit is: grep `crates/vox-gui/ui/src/` for any hardcoded navigation/command definitions that bypass `get_command_catalog()`, per ADR-045's own enforced rule 1 ("No manual navigation entries in TypeScript"), and fix any found. Adding a permanent lint/CI check for this is left to implementer discretion as a nice-to-have, not required scope.

### ADR-037 desktop-clause evidence (GUI-04)
- **D-04:** Evidence that closes GUI-04 is ADR-037's own Decision list, desktop-scoped items only (its point 1's mobile clause is adr-NNN's territory, already superseded): (1) `vox compile --target desktop` produces a real Tauri project via actual Tauri build tooling, not only `tauri-packaging/` hints; (2) the generated desktop app shell has no `axum`/`rust-embed` dependency; (3) `docs/src/architecture/layers.toml` lists the forbidden Axum-as-app-shell pattern, with any current violations tracked in `exempt_files` rather than silently passing. No new acceptance criteria invented beyond what ADR-037 itself already states.

### Claude's Discretion
- Whether to add a permanent CommandCatalog-bypass lint/CI check beyond the one-time audit (D-03).
- Exact wording/placement of the ADR-045 "Status: Accepted" line (D-01) — mirror ADR-037's format.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### GUI/dashboard architecture (GUI-01, GUI-02)
- `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` — the ADR being ratified; currently `status: "current"`, no body "Status: Accepted" line. Decision text: CLI-as-SSOT, no manual TS nav entries, no fetch()/WebSocket in the webview.
- `crates/vox-cli/src/command_catalog.rs` — `build_catalog()`, `CommandCatalogEntry`, `CommandCatalog` — the actual SSOT struct/builder.
- `crates/vox-gui/src/commands/catalog.rs` — `get_command_catalog()`, the Tauri command that calls `build_catalog()` directly; confirms no static duplication today.
- `crates/vox-gui/src/commands/action_manifest.rs`, `crates/vox-gui/src/commands/discovery.rs` — other consumers of `vox_cli::command_catalog::*`; check these too for the GUI-02 audit, not just `catalog.rs`.

### UI boundary (GUI-03) — corrected source, see D-02
- `docs/src/architecture/external-frontend-interop-plan-2026.md` — the CURRENT governing doc for Vox-native vs. React interop (bidirectional component interop model; `@island` retired 2026-05-03). Read in full, not just the excerpt seen during discussion.
- `contracts/frontend/` — per external-frontend-interop-plan-2026.md's own "Layering" note, this is where normative machine rules for the boundary should live (mirrors how ADR-027 described its own now-superseded rule location).
- `docs/src/adr/027-dual-track-ui-surfaces.md` — historical context only; `status: "deprecated"`, superseded 2026-05-03. Do not plan against its Track A/Track B/`@island` model as if current.

### Desktop convergence (GUI-04)
- `docs/src/adr/037-tauri-convergence.md` — desktop-convergence clause (Decision points 1 [desktop only], 2, 3, 4, 6) remains Accepted per the ADR's own Status line; mobile clause superseded by adr-NNN.
- `docs/src/architecture/adr-NNN-scope-tauri-desktop-only.md` — the superseding ADR for the mobile clause; confirms desktop clause is unaffected.
- `docs/src/architecture/layers.toml` — should carry the forbidden-pattern enforcement ADR-037 point 6 requires; verify it's actually present, not just decided.
- `docs/src/architecture/tauri-convergence-migration-plan-2026.md` — referenced migration plan; check for any still-open desktop-scoped items.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox_cli::command_catalog::build_catalog()` — already the live SSOT function; GUI-02's audit is verification, not new plumbing.
- `crates/vox-gui/ui/src/components/` (React/TypeScript) — where any hardcoded nav-entry violations (D-03) would be found.

### Established Patterns
- SSOT-driven codegen is pervasive in this repo (contracts/**/*.yaml → generated Rust/docs/CLI, drift-gated via `vox ci ssot-drift`) — if a permanent CommandCatalog-bypass check is added (implementer discretion, D-03), this is the idiom to follow rather than inventing a new mechanism.
- ADRs in this repo use an explicit `**Status**: <value> (<date>)` body line as the ratification convention (see ADR-037) — D-01 follows this exactly.

### Integration Points
- `crates/vox-gui/src/main.rs` — Tauri `generate_handler!` registration; where `get_command_catalog` is wired in.

</code_context>

<specifics>
## Specific Ideas

No specific implementation-style requirements beyond the four verified decisions above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 4-GUI/Dashboard Architecture Consolidation*
*Context gathered: 2026-09-22*
