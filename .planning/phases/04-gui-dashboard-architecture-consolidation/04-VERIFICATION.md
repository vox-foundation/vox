---
phase: 04-gui-dashboard-architecture-consolidation
verified: 2026-09-22T23:30:00Z
status: gaps_found
score: 4/5 must-haves verified
covered_files: [".planning/REQUIREMENTS.md", ".planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json", ".planning/intel/decisions.md", ".planning/phases/04-gui-dashboard-architecture-consolidation/04-01-PLAN.md", ".planning/phases/04-gui-dashboard-architecture-consolidation/04-01-SUMMARY.md", ".planning/phases/04-gui-dashboard-architecture-consolidation/04-02-PLAN.md", ".planning/phases/04-gui-dashboard-architecture-consolidation/04-02-SUMMARY.md", "contracts/frontend/surface-ownership.v1.yaml", "docs/src/adr/037-tauri-convergence.md", "docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md", "docs/src/architecture/external-frontend-interop-plan-2026.md", "docs/src/architecture/where-things-live.md", "docs/src/reference/frontend-surface-ownership.md", "docs/src/reference/vox-web-stack.md"]
covered_digest: "v1:sha256:32ff4760c3a172f3e3fe195472d98b85e97ed3bce7e6b74daa40e63a25c48a5f"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "No reference doc still names the deleted dashboard crate as the canonical primary GUI (phase goal: 'sole orchestration surface'; ROADMAP SC-1; 04-02-PLAN.md must_have D3)"
    status: failed
    reason: >
      04-02-PLAN.md's must-have D3 states "No page under docs/src/reference/ still names the
      deleted dashboard crate as the canonical primary GUI" — plural "page", not one file. Only
      docs/src/reference/frontend-surface-ownership.md was audited and repaired. A sibling page
      in the SAME docs/src/reference/ directory, docs/src/reference/vox-web-stack.md, still
      asserts (as of this verification): "`vox-dashboard` is the Single Source of Truth for the
      Vox user-facing frontend experience"; "The orchestration dashboard (`crates/vox-dashboard/`)
      is the primary Vox user surface. It is served by the Axum backend (`vox dashboard`
      command)..."; a dashboard entry point at `crates/vox-dashboard/app/src/app.vox`; and a CI
      gate at `scripts/check_dashboard_ssot.vox` that does not exist in the tree. This page is
      not deprecated (frontmatter `training_eligible: true`, category "Language Reference", no
      superseded/retired marker) and is actively linked from other current docs. It directly
      contradicts ADR-045 (ratified in 04-01) and the phase goal that the Tauri GUI is the sole
      orchestration surface. Neither 04-01 nor 04-02 touched this file.
    artifacts:
      - path: "docs/src/reference/vox-web-stack.md"
        issue: "Lines 33-41 name the deleted vox-dashboard crate (removed 2026-05-12) as the canonical, Axum-served, primary Vox user surface; cites a non-existent enforcement script scripts/check_dashboard_ssot.vox"
    missing:
      - "Repoint or annotate the '## Canonical Frontend' section of docs/src/reference/vox-web-stack.md so it no longer names crates/vox-dashboard as the SSOT/primary surface, consistent with the repair already made to docs/src/reference/frontend-surface-ownership.md"
      - "Remove or correct the reference to scripts/check_dashboard_ssot.vox, which does not exist"
deferred: []
advisory: []
---

# Phase 4: GUI/Dashboard Architecture Consolidation Verification Report

**Phase Goal:** The Tauri GUI is the ratified, sole orchestration surface, with a clear, enforced boundary between Vox-native and React/TanStack interop UI code.
**Verified:** 2026-09-22T23:30:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | ADR-045 carries an explicit "Status: Accepted" body line; classification changes from `locked:false`/medium to `locked:true`/high (GUI-01, ROADMAP SC-1) | ✓ VERIFIED | `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` line 9 reads `**Status**: Accepted (2026-09-22)`; frontmatter `status: "current"` byte-identical; `.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json` has `"confidence": "high"`, `"locked": true`, no `notes` key; `.planning/intel/decisions.md`'s `## ADR 045` block reads `- status: locked`. `vox-doc-pipeline --lint-only` exits 0 on the ADR. |
| 2 | Every command `vox-gui` exposes traces back to `vox-cli`'s CommandCatalog SSOT; audit finds no orphaned/duplicated surface (GUI-02, ROADMAP SC-2) | ✓ VERIFIED | 4 files under `crates/vox-gui/src/` reference `build_catalog` (`catalog.rs`, `action_manifest.rs`, `discovery.rs`, `coderabbit.rs`); `transport.ts:408` invokes `get_command_catalog`; `Catalog.tsx` sources `entries: skills ?? []` from that invoke with no hardcoded fallback array; the audit finding and the `coderabbit.rs` test-only exemption are recorded as `notes:` on the `vox-gui` entry in `contracts/frontend/surface-ownership.v1.yaml`. |
| 3 | A documented rule states which UI primitives are Vox-native vs React/TanStack interop; existing components checked against it, no undocumented crossovers (GUI-03, ROADMAP SC-3) | ✓ VERIFIED | `contracts/frontend/surface-ownership.v1.yaml` carries `authoring_track: react-hand-authored` (vox-gui) and `authoring_track: vox-native-interop-reference` (marquee-app); `docs/src/architecture/external-frontend-interop-plan-2026.md` line 182 states the matching prose rule naming both the YAML field and the file; `find crates/vox-gui/ui/src -type f -name '*.vox'` returns 0 files. Roadmap SC-3 literally says "per ADR-027" but ADR-027 is itself `status: "deprecated"` (superseded 2026-05-03); 04-CONTEXT.md D-02 explicitly verifies this and redirects the rule to `external-frontend-interop-plan-2026.md` — a documented, evidence-grounded correction, not a silent scope reduction. |
| 4 | ADR-037's desktop-convergence clause is confirmed complete, its own status reflects Accepted for that clause (GUI-04, ROADMAP SC-4) | ✓ VERIFIED | ADR-037's `## Consequences` gained a 7th bullet citing `generate_tauri_workspace` (`crates/vox-codegen/src/codegen_rust/emit/mod.rs:295`), `build_single_binary`/`build_tauri_app` (`crates/vox-cli/src/commands/bundle.rs:235,289`), and `no-capacitor-in-app-codegen`/`no-axum-in-generated-app-emit`/`no-rust-embed-in-generated-cargo` (`docs/src/architecture/layers.toml:453,467,477`) — all five symbols/rules resolve in the tree. Line-11 `**Status**` and `**Date**` lines are byte-identical to pre-edit state. |
| 5 | No reference doc still names the deleted dashboard crate as the canonical primary GUI (phase goal: "sole orchestration surface"; 04-02-PLAN.md must_have D3) | ✗ FAILED | `docs/src/reference/vox-web-stack.md` (same `docs/src/reference/` directory as the page that WAS repaired) still states "`vox-dashboard` is the Single Source of Truth for the Vox user-facing frontend experience" and "the orchestration dashboard (`crates/vox-dashboard/`) is the primary Vox user surface. It is served by the Axum backend..." — the crate was deleted 2026-05-12. See Gaps Summary. |

**Score:** 4/5 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` | Body Status line added; frontmatter unchanged | ✓ VERIFIED | Confirmed via direct read; lint passes |
| `.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json` | `locked: true`, `confidence: high`, `notes` removed | ✓ VERIFIED | JSON content matches exactly |
| `.planning/intel/decisions.md` | ADR-045 block status `locked` | ✓ VERIFIED | Block content confirmed |
| `docs/src/adr/037-tauri-convergence.md` | New Consequences bullet, cited symbols resolve | ✓ VERIFIED | 7 bullets; all cited symbols found |
| `contracts/frontend/surface-ownership.v1.yaml` | GUI-02 audit notes + `authoring_track` fields | ✓ VERIFIED | 6 surface entries, both `authoring_track` keys present, audit notes present |
| `docs/src/architecture/external-frontend-interop-plan-2026.md` | New Cross-cutting concerns bullet | ✓ VERIFIED | Bullet present, names YAML field |
| `docs/src/reference/frontend-surface-ownership.md` | 4 refs repointed, 0 `vox-dashboard` mentions | ✓ VERIFIED | 0 `vox-dashboard`, 4 `crates/vox-gui` |
| `docs/src/reference/vox-web-stack.md` | (not in phase's `files_modified` list, but covered by 04-02's must_have D3 as an implicit "no page under docs/src/reference/" scope) | ✗ STALE / UNADDRESSED | Still names `crates/vox-dashboard` as canonical SSOT; not touched by either plan |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| ADR-045 body Status line | classification JSON `locked` flag | shared ratification date/claim | ✓ WIRED | Both assert Accepted/locked/high-confidence consistently |
| ADR-045 body Status line | `.planning/intel/decisions.md` ADR-045 block | shared ratification date/claim | ✓ WIRED | `status: locked`, note cites the body line |
| `crates/vox-gui/src/commands/{catalog,discovery,action_manifest,coderabbit}.rs` | `vox_cli::command_catalog::build_catalog()` | direct call | ✓ WIRED | All 4 files call the live SSOT builder |
| `crates/vox-gui/ui/src/transport.ts` | `get_command_catalog` Tauri command | `safeInvoke` | ✓ WIRED | Line 408 |
| `contracts/frontend/surface-ownership.v1.yaml` `authoring_track` | `external-frontend-interop-plan-2026.md` prose bullet | named cross-reference | ✓ WIRED | Bullet names both the field and the file |
| `docs/src/reference/frontend-surface-ownership.md` | ADR-045 / YAML `retired` status | consistency | ✓ WIRED | Repaired; now agrees |
| `docs/src/reference/vox-web-stack.md` | ADR-045 / YAML `retired` status | consistency | ✗ NOT_WIRED | Still contradicts; asserts deleted crate is SSOT/primary |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| GUI-01 | 04-01 | ADR-045 ratified — explicit Accepted marker, locked classification | ✓ SATISFIED | See Truth 1 |
| GUI-02 | 04-02 | vox-gui command surface verified fully sourced from CommandCatalog SSOT | ✓ SATISFIED | See Truth 2 |
| GUI-03 | 04-02 | Vox-native/React interop boundary documented and enforced | ✓ SATISFIED | See Truth 3 (with documented ADR-027→interop-plan redirection) |
| GUI-04 | 04-01 | ADR-037 desktop-convergence clause confirmed complete | ✓ SATISFIED | See Truth 4 |

No orphaned requirements — all four GUI-0x IDs declared in `04-01-PLAN.md`/`04-02-PLAN.md` frontmatter are also listed under `### GUI/Dashboard Architecture` in `.planning/REQUIREMENTS.md`, and the Traceability table maps all four to Phase 4 (checkbox/status column still reads "Pending" — bookkeeping only, not auto-updated by this workflow, not itself evidence of a gap).

### Anti-Patterns Found

None. Scanned every file this phase modified (`docs/src/adr/045-*.md`, `docs/src/adr/037-*.md`, `contracts/frontend/surface-ownership.v1.yaml`, `docs/src/architecture/external-frontend-interop-plan-2026.md`, `docs/src/reference/frontend-surface-ownership.md`, `.planning/intel/decisions.md`, the classification JSON) for `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER` — zero matches.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Doc-pipeline lint passes on all 4 phase-touched docs | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths adr/045-...,adr/037-...,architecture/external-frontend-interop-plan-2026.md,reference/frontend-surface-ownership.md` | "lint complete — no hard errors" | ✓ PASS |
| `contracts-index` accepts the edited YAML contract | `cargo run -q -p vox-cli -- ci contracts-index` | exit 0 | ✓ PASS |
| `build_catalog` consumers in `vox-gui/src` | `grep -rl build_catalog crates/vox-gui/src/` | 4 files: `action_manifest.rs`, `catalog.rs`, `coderabbit.rs`, `discovery.rs` | ✓ PASS |
| Cited ADR-037 symbols resolve | `grep -n 'fn generate_tauri_workspace\|fn build_tauri_app\|no-axum-in-generated-app-emit\|no-rust-embed-in-generated-cargo'` | all found | ✓ PASS |

### Human Verification Required

None. All must-haves are statically verifiable against tracked files and this phase produced zero behavior-dependent (runtime state-transition) truths — it is a documentation/ratification phase.

## Gaps Summary

Four of five must-haves are solidly verified with direct evidence: ADR-045's ratification, the ADR-037 desktop-clause citation, the GUI-02 CommandCatalog audit, and the GUI-03 authoring-track boundary rule all check out against the actual tree, not just the SUMMARY narrative.

The one gap: 04-02-PLAN.md's own must-have D3 promises **"No page under `docs/src/reference/` still names the deleted dashboard crate as the canonical primary GUI"** — but only one of at least two offending pages in that directory was repaired. `docs/src/reference/vox-web-stack.md` (a live, non-deprecated "Language Reference" page, `training_eligible: true`) still declares, in a section literally titled "## Canonical Frontend": *"`vox-dashboard` is the Single Source of Truth for the Vox user-facing frontend experience... The orchestration dashboard (`crates/vox-dashboard/`) is the primary Vox user surface. It is served by the Axum backend (`vox dashboard` command)..."* — plus a dashboard entry point path (`crates/vox-dashboard/app/src/app.vox`) and an enforcement script (`scripts/check_dashboard_ssot.vox`) that no longer exist in the tree (crate deleted 2026-05-12, per AGENTS.md's own Retired Surfaces table).

This is not a peripheral nit: the phase's stated goal is "The Tauri GUI is the ratified, sole orchestration surface." A live reference page asserting the opposite, in the same "reference material" category the plan's own must-have specifically targeted, is a direct contradiction of that goal that the plan's scope (correctly) identified as in-bounds for one file but did not extend to its sibling. Neither 04-01 nor 04-02 touched this file, and it is not deferred to a later phase in ROADMAP.md.

(Separately and outside this phase's stated `docs/src/reference/` scope, `docs/src/architecture/where-things-live.md` also lists `vox-dashboard` — "Local Axum-served orchestration dashboard (SPA host)" — without a retired marker; noted for awareness but not counted as a gap since the phase's must-have was scoped to `docs/src/reference/`.)

**Recommended fix:** repoint or annotate `docs/src/reference/vox-web-stack.md`'s "## Canonical Frontend" section the same way `frontend-surface-ownership.md` was repaired in 04-02 — name `crates/vox-gui` as the canonical/primary surface, remove or correct the `scripts/check_dashboard_ssot.vox` citation, and keep the surrounding prose/frontmatter otherwise intact. This is a small, mechanical follow-up plan, not a re-scope of the phase.

---

*Verified: 2026-09-22T23:30:00Z*
*Verifier: Claude (gsd-verifier)*
