---
phase: 04-gui-dashboard-architecture-consolidation
plan: 02
subsystem: ui
tags: [contracts, gui, command-catalog, boundary-rule]

requires:
  - phase: 04-01
    provides: "ADR-045 ratified as Accepted (repair in this plan depends on that ratification existing)"
provides:
  - "GUI-02 audit recorded durably in contracts/frontend/surface-ownership.v1.yaml"
  - "GUI-03 Vox-native/React boundary rule in two agreeing halves"
  - "Removed the last live contradiction of ADR-045 in docs/src/reference/"
affects: []

actuals:
  tokens: 5200
  tasks: 2
  commits: 1

tech-stack:
  added: []
  patterns: ["Per-surface authoring_track field in contracts/frontend/surface-ownership.v1.yaml, paired with a prose bullet naming it -- two agreeing halves of one rule, not a new enforcement mechanism"]

key-files:
  created: []
  modified:
    - "contracts/frontend/surface-ownership.v1.yaml"
    - "docs/src/architecture/external-frontend-interop-plan-2026.md"
    - "docs/src/reference/frontend-surface-ownership.md"

key-decisions:
  - "Added the 'crates/' prefix to one previously-bare 'vox-dashboard' reference (Worked Examples section) when repointing it to vox-gui, for consistency with the other 3 references and to meet the acceptance criterion's literal 4-occurrence count -- a cosmetic normalization, not a prose-meaning change"

requirements-completed: [GUI-02, GUI-03]

coverage:
  - id: D1
    description: "vox-gui command surface audited against CommandCatalog SSOT; no orphaned/duplicated commands found; finding recorded in a tracked contract file (ROADMAP SC-2, GUI-02)"
    requirement: "GUI-02"
    verification:
      - kind: other
        ref: "grep -rl build_catalog crates/vox-gui/src -- exactly 4 files (catalog.rs, action_manifest.rs, discovery.rs, coderabbit.rs); transport.ts routes through get_command_catalog invoke; Catalog.tsx has no hardcoded fallback (skills ?? []); audit notes present in surface-ownership.v1.yaml vox-gui entry"
        status: pass
      - kind: other
        ref: "git diff on the 7 audit-relevant GUI files -- empty (no code changed, confirming-only audit)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Vox-native vs. hand-authored-React boundary documented in machine-readable + prose form, against the current governing doc (ROADMAP SC-3, GUI-03)"
    requirement: "GUI-03"
    verification:
      - kind: other
        ref: "authoring_track: react-hand-authored on vox-gui, vox-native-interop-reference on marquee-app; prose bullet in external-frontend-interop-plan-2026.md Cross-cutting concerns naming both; find crates/vox-gui/ui/src -name '*.vox' -- 0 results"
        status: pass
      - kind: other
        ref: "cargo run -p vox-doc-pipeline -- --lint-only (both docs) -- exit 0; cargo run -p vox-cli -- ci contracts-index -- exit 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "No reference page under docs/src/reference/ still names the deleted dashboard crate as canonical primary GUI (phase goal: sole orchestration surface)"
    requirement: "GUI-03"
    verification:
      - kind: other
        ref: "frontend-surface-ownership.md: 0 occurrences of vox-dashboard, 4 occurrences of crates/vox-gui, frontmatter and file length (53 lines) unchanged"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-09-22
status: complete
---

# Phase 4 Plan 02: CommandCatalog Audit & UI Boundary Documentation Summary

**Audited vox-gui's command surface (clean — no violations), recorded the finding and one documented exemption in the surface-ownership contract, wrote the Vox-native/React authoring boundary in a machine-readable field plus matching prose, and fixed the one reference page still naming a deleted crate as the canonical GUI.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-09-22T22:35:00Z
- **Completed:** 2026-09-22T23:05:00Z
- **Tasks:** 2 of 2
- **Files modified:** 3

## Accomplishments
- Enumerated all 4 `crates/vox-gui/src` files referencing `build_catalog()` (`catalog.rs`, `action_manifest.rs`, `discovery.rs`, `coderabbit.rs`) — one more than RESEARCH.md's stale 3-consumer count
- Confirmed `crates/vox-gui/ui/src`'s sole command-surface consumer (`Catalog.tsx`) has zero hardcoded fallback, sourcing entries entirely from the `get_command_catalog` Tauri invoke
- Grounded the `navigation.ts` `PARENT_CHILD_MAP` non-violation verdict in `CommandCatalogEntry`'s actual field list (no section/parent/group field exists, so it couldn't be catalog-derived)
- Recorded the audit finding and the `coderabbit.rs` exemption as durable notes on `contracts/frontend/surface-ownership.v1.yaml`'s `vox-gui` entry — declined a permanent CI check per D-03 (would duplicate `gui_catalog_parity.rs` for a zero-instance violation class)
- Added `authoring_track` field (`react-hand-authored` / `vox-native-interop-reference`) to the `vox-gui` and `marquee-app` surface entries, plus a matching prose bullet in `external-frontend-interop-plan-2026.md`
- Repointed 4 stale `vox-dashboard` references in `docs/src/reference/frontend-surface-ownership.md` to `crates/vox-gui` — this page contradicted ADR-045 (ratified in 04-01) and the YAML registry's own `retired` status for that surface

## Task Commits

1. **Task 1 + Task 2 (combined — both edit the same YAML file's `vox-gui`/`marquee-app` entries)** — `e7b0bcf01` (docs)

**Plan-level verification:** `git diff --name-only 6c7fba76c..HEAD` across both of Phase 4's plans lists exactly the 7 files in the phase-wide `artifacts_this_phase_produces` table.

## Files Created/Modified
- `contracts/frontend/surface-ownership.v1.yaml` — 3 new notes + `authoring_track` on `vox-gui`; `authoring_track` on `marquee-app`; other 4 surface entries untouched
- `docs/src/architecture/external-frontend-interop-plan-2026.md` — 1 new `## Cross-cutting concerns` bullet
- `docs/src/reference/frontend-surface-ownership.md` — 4 references repointed; frontmatter and 53-line length unchanged

## Decisions Made
- Combined Task 1 and Task 2's edits to `surface-ownership.v1.yaml` into single passes per entry (audit notes + `authoring_track` both land on `vox-gui` in one edit) rather than two separate touches of the same lines, and committed both tasks together since splitting the YAML file's history mid-entry would be artificial.
- Normalized one bare `vox-dashboard` reference (no `crates/` prefix, in the Worked Examples prose) to `crates/vox-gui` when repointing it, matching the other three references' prefix style — a consistency fix, not a meaning change, and needed to satisfy the acceptance criterion's literal 4-occurrence count for `crates/vox-gui`.

## Deviations from Plan

None of substance. One verify command (`git diff --quiet -- crates/vox-gui`) initially reported "modified" due to three pre-existing, unrelated local changes in the shared working tree (`research.rs`, `search_probe.rs`, `research_doc_publish_test.rs` — present before this session started, part of separate in-progress work, not touched by this task). Confirmed via targeted `git diff --stat` on the 7 audit-relevant files specifically, which was empty — the task's actual constraint (no GUI source touched) held; the blanket check was a false positive from the shared dirty tree, not a real violation.

## Issues Encountered

None beyond the verify false-positive above.

## Next Phase Readiness

**Complete.** All four of Phase 4's requirements (GUI-01 through GUI-04) are closed with verified evidence across both plans. Phase 4 is ready to close.

---
*Phase: 04-gui-dashboard-architecture-consolidation*
*Completed: 2026-09-22*
