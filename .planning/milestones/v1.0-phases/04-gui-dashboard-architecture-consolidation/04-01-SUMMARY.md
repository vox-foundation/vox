---
phase: 04-gui-dashboard-architecture-consolidation
plan: 01
subsystem: infra
tags: [adr, docs, vox-doc-pipeline, gsd-intel]

requires: []
provides:
  - "ADR-045 ratified as Accepted, agreeing across doc body, GSD classification JSON, and decisions.md"
  - "ADR-037's desktop-convergence clause recorded as implemented, with three verified code citations"
affects: [04-02]

actuals:
  tokens: 3800
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns: ["ADR ratification lives in a body `**Status**: Accepted (date)` line, never the frontmatter `status:` key -- vox-doc-pipeline's VALID_STATUS enum has no 'accepted' value"]

key-files:
  created: []
  modified:
    - "docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md"
    - ".planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json"
    - ".planning/intel/decisions.md"
    - "docs/src/adr/037-tauri-convergence.md"

key-decisions:
  - "Deliberately did NOT follow CONTEXT.md D-01's literal instruction to set ADR-045 frontmatter status to 'accepted' -- that value isn't in vox-doc-pipeline's VALID_STATUS enum and would be a hard lint failure. Used the body Status-line convention every other locked ADR in this repo (037, 041, 047) actually uses instead. Documented as a visible correction, not applied silently."

requirements-completed: [GUI-01, GUI-04]

coverage:
  - id: D1
    description: "ADR-045 carries an explicit Accepted marker and its GSD intel classification agrees (ROADMAP SC-1, GUI-01)"
    requirement: "GUI-01"
    verification:
      - kind: other
        ref: "grep for body Status line -- present; frontmatter status unchanged; classification JSON locked=true/confidence=high, notes key removed; decisions.md ADR-045 block status: locked"
        status: pass
      - kind: other
        ref: "cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md -- exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "ADR-037's desktop-convergence clause is recorded as implemented, citing 3 verified code paths (ROADMAP SC-4, GUI-04)"
    requirement: "GUI-04"
    verification:
      - kind: other
        ref: "grep for the new Consequences bullet -- present; generate_tauri_workspace, build_tauri_app, and both layers.toml forbidden-pattern rules all resolve in the tree; existing Status/Date lines byte-identical"
        status: pass
      - kind: other
        ref: "cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/adr/037-tauri-convergence.md -- exit 0"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-09-22
status: complete
---

# Phase 4 Plan 01: ADR-045 Ratification & ADR-037 Desktop-Clause Closeout Summary

**Ratified ADR-045 as Accepted across all three GSD artifacts (doc body, intel classification, decisions index) and recorded ADR-037's desktop-convergence clause as implemented with three cited, verified code paths — zero code changes, pure documentation ratification.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-09-22T22:05:00Z
- **Completed:** 2026-09-22T22:30:00Z
- **Tasks:** 2 of 2
- **Files modified:** 4

## Accomplishments
- Added ADR-045's body `**Status**: Accepted (2026-09-22)` line, matching ADR-037/041/047's existing convention, while leaving its frontmatter `status: "current"` untouched (required — see Decisions Made)
- Synced `.planning/intel/classifications/045-*.json` to `locked: true`, `confidence: "high"`, removing the now-superseded `notes` key
- Synced `.planning/intel/decisions.md`'s ADR-045 block to `status: locked` with a ratification note
- Verified `cargo run -p vox-doc-pipeline -- --lint-only` passes on ADR-045 with the frontmatter unchanged
- Appended one `## Consequences` bullet to ADR-037 citing `generate_tauri_workspace`, `build_single_binary`/`build_tauri_app`, and layers.toml's three forbidden-pattern rules — all independently confirmed present in the tree before writing the citation
- Verified ADR-037's existing `**Status**`/`**Date**` lines are byte-identical post-edit, and the doc-pipeline lint passes

## Task Commits

1. **Task 1: Ratify ADR-045 (body Status line, intel classification, decisions index)** — `4f26e5deb` (docs)
2. **Task 2: Close out GUI-04 — annotate ADR-037's Consequences** — `287b93b5b` (docs)

**Plan-level verification:** `git diff --name-only 6c7fba76c..HEAD` lists exactly the 4 files in `files_modified` — no code, no `.toml`, no `contracts/` file touched.

## Files Created/Modified
- `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` — one body Status line inserted; frontmatter byte-identical
- `.planning/intel/classifications/045-tauri-gui-replaces-axum-dashboard-8c1d9e4b.json` — locked/confidence updated, stale `notes` key removed
- `.planning/intel/decisions.md` — ADR-045 block status + note updated; ADR-044/046 blocks untouched
- `docs/src/adr/037-tauri-convergence.md` — one Consequences bullet appended; existing 6 bullets and Status/Date lines byte-identical

## Decisions Made
- **Corrected CONTEXT.md D-01's literal instruction rather than applying it silently.** D-01 said to set ADR-045's frontmatter `status:` to `accepted`. `vox-doc-pipeline`'s `VALID_STATUS` enum (`crates/vox-doc-pipeline/src/pipeline/lint.rs:37-44`) only permits `current|experimental|legacy|research|roadmap|deprecated` — zero of the repo's 24 frontmatter-`status:`-bearing ADRs use any other value, including locked ADRs 037/041/047. The body `**Status**: Accepted (date)` line is this repo's actual ratification mechanism. D-01's intent (an unambiguous Accepted marker) is fully satisfied; its literal mechanism was not, and that correction is recorded here rather than hidden.
- Used the inline-bold `**Status**:` form (ADR-037's convention) rather than ADR-041's `## Status` H2 heading, since ADR-045's own section style already follows ADR-037.

## Deviations from Plan

None beyond the documented D-01 correction, which the plan itself specified in advance (see `<planner_correction_to_d01>` in 04-01-PLAN.md) — not an in-flight deviation.

## Issues Encountered

None.

## Next Phase Readiness

**Complete.** GUI-01 and GUI-04 are both closed with verified evidence. Plan 04-02 (GUI-02 CommandCatalog audit, GUI-03 UI boundary documentation) is next and depends on this plan only nominally (wave ordering, no functional dependency).

---
*Phase: 04-gui-dashboard-architecture-consolidation*
*Completed: 2026-09-22*
