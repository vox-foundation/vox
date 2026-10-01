# Phase 4: GUI/Dashboard Architecture Consolidation - Pattern Map

**Mapped:** 2026-09-22
**Files analyzed:** 6 (all modified, 0 new)
**Analogs found:** 6 / 6

This is a documentation-ratification-and-audit phase (per RESEARCH.md) — every target file
is an edit to an existing tracked doc/metadata file, not a new source file. "Role" below is
therefore documentation-taxonomy (ADR, machine-metadata, contract, audit-report) rather than
code-architecture role, and "analog" means the closest sibling doc that already carries the
target pattern (an already-ratified ADR, an already-locked classification record, an
already-shaped contract entry).

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` | ADR (governance doc) | transform (status ratification, body+frontmatter edit) | `docs/src/adr/037-tauri-convergence.md` | exact (same doc type, same "Status" convention, same domain — GUI/Tauri) |
| `.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json` | machine metadata (GSD ingest snapshot) | transform (field flip: confidence/locked/notes) | `.planning/intel/classifications/041-durable-functions-completion-2026-6dcf43cf.json` | exact (same schema, same locked-ADR end-state) |
| `.planning/intel/decisions.md` | machine-readable index (per-ADR summary block) | transform (status line + note edit, in place) | ADR 041 / ADR 047 entries in the same file (lines ~247-251, ~283-287) | exact (same file, same per-entry block shape) |
| `docs/src/adr/037-tauri-convergence.md` | ADR (governance doc) | transform (append verification annotation to existing Consequences section) | same file's own `## Status` line convention (self-referential — extending, not replacing) | exact |
| `contracts/frontend/surface-ownership.v1.yaml` and/or `docs/src/architecture/external-frontend-interop-plan-2026.md` | contract / architecture-plan doc | transform (new field on existing entry / new prose subsection) | existing `vox-gui` surface entry in the same YAML file; existing "Cross-cutting concerns" section in the same plan doc | exact (both are in-file precedent, not cross-file) |
| GUI-02 audit finding (no file edit expected — write-up lands wherever the phase plan records audit results, e.g. a completion note in `.planning/phases/04-.../04-SUMMARY.md` or equivalent, not a source file) | audit report | request-response (read-only verification) | N/A — see "No Analog Found" | n/a |

## Pattern Assignments

### `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` (ADR ratification)

**Analog:** `docs/src/adr/037-tauri-convergence.md`

**CRITICAL CORRECTION to CONTEXT.md D-01:** D-01 literally says "updates its frontmatter
`status: "current"` → `status: "accepted"`". **Do not do this** — `"accepted"` is not in
`vox-doc-pipeline`'s enforced `VALID_STATUS` enum (`current|experimental|legacy|research|
roadmap|deprecated`, `crates/vox-doc-pipeline/src/pipeline/lint.rs:37-44`) and will hard-fail
the lint gate. Every existing locked ADR (037, 041, 047) keeps frontmatter `status: "current"`
and signals ratification only via a body line. Follow that precedent exactly.

**Current frontmatter** (lines 1-6) — leave unchanged:
```markdown
---
title: "ADR 045 — Tauri GUI Replaces Axum Dashboard"
date: "2026-05-11"
status: "current"
category: "Architecture Decisions (ADRs)"
---
```

**Analog Status-line pattern** (`docs/src/adr/037-tauri-convergence.md` lines 9-13 — inline
bold form, immediately after the H1, before the first `##` section):
```markdown
# ADR 037 — Tauri Convergence

**Status**: Partially superseded (2026-06-12) by [adr-NNN](../architecture/adr-NNN-scope-tauri-desktop-only.md) — the mobile-target clause only (Tauri no longer covers `--target mobile-android`/`--target mobile-ios`; React Native + Expo does). The desktop-convergence decision below is unaffected and remains Accepted.
**Date**: 2026-05-11
```

**Target edit for ADR-045** (current body opens directly at `## Context`, line 8-10 — insert
a Status line between the H1 and `## Context`, mirroring the pattern above but using the
simpler unconditional form since ADR-045 has no partial-supersession caveat):
```markdown
# ADR 045 — Tauri GUI Replaces Axum Dashboard

**Status**: Accepted (2026-09-22)

## Context
```

Note: ADR-041 uses a slightly different variant (`## Status` as its own H2 section, body
`**Accepted (date).**`) — that is a valid alternative precedent but RESEARCH.md Pattern 1
explicitly recommends mirroring ADR-037's inline-bold-no-heading form since ADR-045's own
"Consequences" section already follows ADR-037's section-naming convention. Use the ADR-037
form for consistency within this ratification.

---

### `.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json` (metadata sync)

**Analog:** `.planning/intel/classifications/041-durable-functions-completion-2026-6dcf43cf.json`

**Current content** (full file, single line):
```json
{"source_path": "/Users/brbrainerd/dev/vox/docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md", "type": "ADR", "confidence": "medium", "manifest_override": true, "title": "ADR 045 — Tauri GUI Replaces Axum Dashboard", "summary": "Decommission legacy Axum dashboard and unify into Tauri 2 GUI with CLI as SSOT.", "scope": ["Tauri 2", "vox-gui", "vox-cli", "CommandCatalog", "dashboard", "UI"], "cross_refs": [], "locked": false, "precedence": null, "notes": "No explicit 'Status: Accepted' line in document body. Frontmatter has status='current' which is non-standard for ADRs."}
```

**Analog locked-ADR shape** (041's JSON — the target field values to mirror):
```json
{"source_path": "...", "type": "ADR", "confidence": "high", "manifest_override": true, "title": "...", "summary": "...", "scope": [...], "cross_refs": [...], "locked": true, "precedence": null}
```
Note 041 and 047 both *omit* the `notes` field entirely once locked (rather than setting it
to an empty string) — only 037 (still `locked: false`, partially superseded) retains a
`notes` field explaining the caveat.

**Target edit for 045's JSON:** flip `"confidence": "medium"` → `"high"`, `"locked": false` →
`true`, and remove the stale `"notes"` field entirely (it described exactly the gap this
ratification closes — keeping it verbatim would contradict the new locked state). Leave
`"cross_refs": []` as-is unless the plan also decides to backfill it (optional, not required
by any decision in CONTEXT.md).

---

### `.planning/intel/decisions.md` (index entry sync)

**Analog:** the ADR 041 and ADR 047 entries in the same file (same file, same block shape —
this is an in-file pattern, not a cross-file one).

**Current ADR-045 entry:**
```markdown
## ADR 045 — Tauri GUI Replaces Axum Dashboard
- source: docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md
- status: proposed
- decision: Decommission legacy Axum dashboard and unify into Tauri 2 GUI with CLI as SSOT.
- scope: Tauri 2, vox-gui, vox-cli, CommandCatalog, dashboard, UI
- note: Not marked locked/Accepted in its own frontmatter (status: "current", no explicit Accepted line) — medium confidence. It is nonetheless the current, non-contradicted decision for dashboard/GUI architecture now that ADR-024 (its predecessor) has been marked Superseded and no other locked ADR contradicts it. Treat as authoritative pending a formal Accepted status line.
```

**Analog locked-entry shape** (ADR 041, no caveat needed since it's cleanly locked):
```markdown
## ADR 041: Durable functions completion (workflow, activity, actor, @scheduled)
- source: docs/src/adr/041-durable-functions-completion-2026.md
- status: locked
- decision: Records closure of parse-only stub gap; features now backed by working runtime, codegen, journal-backed replay, and scheduler.
- scope: durable functions, workflow, activity, actor, @scheduled, HIR interpreter, codegen
```
(no `note:` line — locked entries with no caveat omit it entirely, as seen on 041/044/047/048)

**Target edit for the ADR-045 entry:** `status: proposed` → `status: locked`; replace the
`note:` line with one recording the ratification (or drop it, matching 041's no-note
precedent) — e.g. `- note: Ratified 2026-09-22 — Status: Accepted line added to ADR body; see docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md.`

---

### `docs/src/adr/037-tauri-convergence.md` (GUI-04 desktop-clause verification annotation)

**Analog:** self — extend the existing `## Consequences` section (lines 51-58) with one more
bullet, following the exact bullet style already used there.

**Current Consequences section** (lines 51-58):
```markdown
## Consequences

- `vox compile --target desktop|mobile-*` becomes a heavier build path. CI must explicitly account for Tauri, Android, and iOS toolchain costs rather than hiding them behind hint generation.
- `native-binary` remains available for the Axum + embedded SPA shape where a local server binary is the desired artifact.
- The generated app codegen split becomes clearer: server targets emit Axum, application targets emit Tauri.
- Capability projection moves from "merge hints for a downstream shell" toward generated Tauri config/capability files.
- `apps/vox-mental-tracker` becomes the acceptance fixture for proving Tauri mobile can carry real Vox app features, including on-device ASR through the Sherpa plugin port.
- Future contributors and coding agents get hard guardrails: adding new `@capacitor/*`, `npx cap sync`, or Axum-as-app generation outside the active migration allowlist fails architecture checks.
```

**Target edit:** append one bullet closing the loop per RESEARCH.md Open Question 2 — cite
the three verified code paths (no new ADR needed, this is evidence-citation prose):
```markdown
- **Desktop-clause implementation verified complete (2026-09-22):** `vox compile --target desktop` emits a real Tauri workspace (`crates/vox-codegen/src/codegen_rust/emit/mod.rs::generate_tauri_workspace`, no Axum/rust-embed), and `vox bundle` detects the generated `src-tauri/Cargo.toml` and runs real `cargo tauri build --no-bundle` (`crates/vox-cli/src/commands/bundle.rs::build_single_binary` → `build_tauri_app`). `docs/src/architecture/layers.toml`'s `no-axum-in-generated-app-emit` / `no-rust-embed-in-generated-cargo` forbidden patterns are already active with tracked `exempt_files`. Decision points 1-3 and 6 above are implemented, not just decided.
```

---

### `contracts/frontend/surface-ownership.v1.yaml` (GUI-03 boundary rule — YAML half)

**Analog:** the existing `vox-gui` surface entry in the same file (lines 4-17) — extend its
shape in place rather than inventing a new top-level key.

**Current `vox-gui` entry** (lines 4-17):
```yaml
  - id: vox-gui
    path: crates/vox-gui
    status: canonical
    role: Primary user-facing native operator GUI surface for Vox.
    source_of_truth:
      - crates/vox-gui/src/main.rs
      - crates/vox-gui/ui/src/App.tsx
      - docs/src/architecture/vox-gui-capability-audit-2026.md
    ownership:
      team: platform-gui
      review_label: gui-canonical
    notes:
      - Tauri IPC handlers in crates/vox-gui/src/commands define backend capability.
      - UI panels in crates/vox-gui/ui/src/components/surfaces must map to real handlers.
      - Browser surface (preview iframe + agent CDP live view) lives in ui/src/components/surfaces/Browser/; backend in src/commands/browser.rs.
```

**Target edit:** add an `authoring_track: react-hand-authored` field to `vox-gui`'s entry
(and `authoring_track: vox-native` or `hybrid` to the `marquee-app` entry, lines 30-42, since
that surface is the interop-reference target per RESEARCH.md's diagram) plus a verified-fact
note, e.g.:
```yaml
    authoring_track: react-hand-authored
    notes:
      - ...(existing notes unchanged)...
      - "Verified 2026-09-22: 0 of 319 UI source files under crates/vox-gui/ui/src are .vox-compiled component output; the surface is 100% hand-authored TSX. Any .vox-derived component appearing here would violate this boundary."
```

### `docs/src/architecture/external-frontend-interop-plan-2026.md` (GUI-03 boundary rule — prose half)

**Analog:** the existing `## Cross-cutting concerns` section (lines 174-181) — same file,
same bullet-list convention, append one bullet.

**Current section end** (line 181):
```markdown
- **Emitted component code is generated, not authored.** Per the project's "auto-generated docs" policy, emitted `.tsx` files should not be hand-edited; the `.vox` source is canonical. Any escape-hatch user-edit zones must be explicitly delimited so the compiler can preserve them across re-emits.
```

**Target edit:** append a new bullet stating the surface-level boundary rule GUI-03 requires,
citing `contracts/frontend/surface-ownership.v1.yaml` as the machine-checkable half:
```markdown
- **Vox-native vs. hand-authored React boundary (per-surface, not per-file):** `crates/vox-gui` is a fully hand-authored React/TSX surface (0 `.vox`-compiled components as of 2026-09-22) and stays that way — it is the canonical operator GUI, not an interop demo. `apps/interop/marquee_app` is the interop-reference surface exercising the bidirectional `import_react` bridge above. The per-surface `authoring_track` field in `contracts/frontend/surface-ownership.v1.yaml` records this split; a surface changing tracks requires updating both that file and this note.
```

---

## Shared Patterns

### ADR ratification convention
**Source:** `docs/src/adr/037-tauri-convergence.md` lines 9-13 (also `041`, `047`)
**Apply to:** `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md`
**Rule:** frontmatter `status:` stays in `{current, experimental, legacy, research, roadmap, deprecated}` always — ratification is signaled only by an inline body `**Status**: <value> (<date>)` line directly under the H1, never by a frontmatter value change to `"accepted"`.
```markdown
# ADR NNN — Title

**Status**: Accepted (YYYY-MM-DD)

## Context
```

### `.planning/intel/` dual-artifact sync (doc edit + machine metadata)
**Source:** `.planning/intel/classifications/*.json` + `.planning/intel/decisions.md` (paired for every ADR)
**Apply to:** every ADR-045 edit in this phase
**Rule:** an ADR body/status edit is incomplete until its paired classification JSON (`locked`, `confidence`, `notes`) and its `decisions.md` block (`status:`, `note:`) are updated in the same change — these are read by future `/gsd-plan-phase`/`/gsd-progress` runs, not by any Vox-repo runtime tool, but drift here re-flags a ratified ADR as still-unratified.

### Doc-pipeline lint compliance gate
**Source:** `crates/vox-doc-pipeline/src/pipeline/lint.rs:37-44` (`VALID_STATUS` enum)
**Apply to:** any frontmatter `status:` edit under `docs/src/`
**Verification command:** `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md`

## No Analog Found

| File / Deliverable | Role | Data Flow | Reason |
|---|---|---|---|
| GUI-02 CommandCatalog-bypass audit write-up | audit report | request-response (read-only grep + confirm) | No prior "audit-only, confirm-clean" deliverable exists as a standalone file pattern in this repo — RESEARCH.md confirms the audit finds a clean state (0 hardcoded nav entries; all three Rust consumers and the one TSX consumer already route through `build_catalog()`/`get_command_catalog()`). The planner should record this as a phase-completion note (e.g. in the phase's SUMMARY or PROGRESS artifact per the GSD flow) rather than an edit to any source file. If Claude's discretion (per CONTEXT.md) adds a permanent CI check, the analog for *that* optional artifact is `crates/vox-cli/src/commands/ci/gui_catalog_parity.rs` (existing SSOT-drift-style checker) — cited in RESEARCH.md's Don't Hand-Roll table — but this is optional, not required scope. |

## Metadata

**Analog search scope:** `docs/src/adr/*.md`, `.planning/intel/classifications/*.json`, `.planning/intel/decisions.md`, `contracts/frontend/*.yaml`, `docs/src/architecture/external-frontend-interop-plan-2026.md`
**Files scanned:** ADR-037, ADR-041, ADR-045, ADR-047 (full or partial reads); 3 classification JSONs; `decisions.md` ADR-023–048 block; `surface-ownership.v1.yaml` (full); `external-frontend-interop-plan-2026.md` (headers + Cross-cutting concerns section)
**Tracked-source gate:** all 6 target paths confirmed via `git ls-files` — none are gitignored mirrors.
**Pattern extraction date:** 2026-09-22
