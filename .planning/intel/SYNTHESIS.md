---
title: "Doc ingest synthesis summary (2026-09-22 full-corpus re-run)"
description: "Entry point for downstream consumers (gsd-roadmapper) into the synthesized intel from the 430-doc classification corpus."
category: "Architecture SSOTs"
---

# Synthesis Summary

Full-corpus re-run over 430 classifications, after the operator resolved the prior run's single
BLOCKER (ADR-037 vs adr-NNN, LOCKED-vs-LOCKED on Tauri mobile scope) and its related WARNING
(ADR-024 vs ADR-045 on dashboard architecture) directly in the source tree. This run's intel files
and conflicts report **replace** the prior full-corpus run's output.

## Doc counts by type

| Type | Count |
|---|---|
| ADR  | 49 |
| SPEC | 96 |
| PRD  | 1 |
| DOC  | 284 |
| **Total** | **430** |

All 430 classifications were high or medium confidence — zero UNKNOWN or low-confidence docs in this
corpus (no re-tagging needed).

## Decisions (`decisions.md`)

49 ADR entries, one per ADR. 34 locked (Accepted, cannot be auto-overridden), 15 non-locked
(proposed/superseded/evaluation-only). Two entries carry special handling from this run's conflict
resolution:

- **ADR-037** (Tauri Convergence) — now `proposed (partially superseded)`: its mobile-target clause is
  superseded by locked **adr-NNN**; its desktop-convergence clause remains Accepted/active.
- **ADR-024** (Dashboard as Axum SPA) — now `proposed (superseded)` by **ADR-045** (Tauri GUI Replaces
  Axum Dashboard), itself not locked — see `INGEST-CONFLICTS.md` WARNINGS.

Three further auto-resolutions (ADR-047 > ADR-020/008/009 on mesh transport; ADR-041 > ADR-028 on
durable functions) are logged as INFO in the conflicts report, not repeated here.

Two locked ADRs (030, 031) still name the decommissioned "vox-dashboard" surface in their own text;
both carry an explanatory `note:` in `decisions.md` so downstream consumers don't read that as current.

## Requirements (`requirements.md`)

1 PRD in the corpus: `dead-crate-fate-plan-2026-05-08.md`. Extracted as 6 requirements grouped by
crate-disposition category (DELETE, WIRE-UP-AS-IS, EXTRACT-TO-PLUGIN, KEEP-FROZEN, MISPLACED,
catalog-cleanup) rather than one requirement per crate, to avoid ~20 near-duplicate entries while
preserving every crate-level acceptance criterion verbatim in the `acceptance:` field: REQ-dead-crate-
delete, REQ-dead-crate-wire-up, REQ-dead-crate-extract-to-plugin, REQ-dead-crate-keep-frozen,
REQ-dead-crate-misplaced, REQ-dead-crate-catalog-cleanup.

## Constraints (`constraints.md`)

96 SPEC entries. `type` bucketing (api-contract | schema | nfr | protocol) is a best-effort heuristic
over title/summary/scope keywords — many source SPECs are broad architecture SSOTs rather than a
single-bucket artifact, so the bucket is a routing aid, not an exclusive classification:

| type | count |
|---|---|
| nfr | 37 |
| api-contract | 30 |
| protocol | 16 |
| schema | 13 |

## Context (`context.md`)

284 DOC entries, topic-keyed by document title (alphabetical), each with source attribution and the
classifier-extracted summary. Nearly the entire DOC set lives under `docs/src/architecture/`
(research, audits, plans, SSOTs not themselves ADR/SPEC/PRD-typed).

## Conflicts

See `.planning/INGEST-CONFLICTS.md` for full detail.

- **0 BLOCKERS** — both fixes from the prior run independently verified resolved (source Status lines
  and classification `locked` fields checked directly, not assumed from the prompt).
- **2 WARNINGS** — ADR-045 stands as the current dashboard/GUI decision without itself being locked;
  ADR-030/031 still name the decommissioned "vox-dashboard" surface (left uncorrected per operator
  instruction, flagged so synthesis doesn't propagate the stale name).
- **5 INFO** — 4 precedence auto-resolutions (ADR-047 mesh transport, ADR-041 durable functions,
  ADR-045 dashboard, adr-NNN/ADR-037 split) + 1 cycle-detection note (75 cross-ref cycles found, all
  companion-document mutual references, none blocking).

No LOCKED-vs-LOCKED contradictions remain anywhere in the 49-ADR set (verified by heuristic
scope-overlap scan across all ADR/SPEC/PRD pairs, every overlap ≥2 shared scope terms inspected by
hand). No competing PRD acceptance-criteria variants exist (only one PRD in corpus).

## Status

**READY — safe to route to `gsd-roadmapper`**, with the two WARNINGS above surfaced for awareness
(neither blocks routing; both are informational quality notes about a non-locked current decision and
stale terminology in unedited-but-still-valid locked ADRs).
