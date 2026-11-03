# Phase 1: Dead Crate Cleanup — Remove & Confirm - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-22
**Phase:** 1-Dead Crate Cleanup — Remove & Confirm
**Areas discussed:** vox-scientia-ingest disposition, Frozen-crate documentation form, Verification tooling, Commit granularity

---

## vox-scientia-ingest disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Inline directly | Fold scholarly-external-jobs functionality straight into vox-publisher, no flag surface — simplest, matches that it's mandatory functionality anyway | |
| Gate behind a flag | Keep it toggleable/optional in vox-publisher — more flexible if this ever needs to be disabled or extracted again later | |
| You decide | Let the planner/executor pick based on what's cleanest when they're actually looking at the code | ✓ |

**User's choice:** You decide
**Notes:** Left as implementer discretion — recorded in CONTEXT.md's "Claude's Discretion" section.

---

## Frozen-crate documentation form

| Option | Description | Selected |
|--------|-------------|----------|
| catalog.toml entry | One central place — matches the mechanism REQ-dead-crate-catalog-cleanup already uses for ghost-entry bookkeeping | ✓ |
| Doc comment in each crate | A comment in each crate's Cargo.toml/lib.rs — visible right where someone would look when opening the crate | |
| where-things-live.md note | Add to the canonical concept→crate lookup table — visible to anyone navigating the architecture docs | |

**User's choice:** catalog.toml entry
**Notes:** Consolidates frozen-crate status with existing catalog-hygiene bookkeeping rather than splitting documentation across multiple locations.

---

## Verification tooling

| Option | Description | Selected |
|--------|-------------|----------|
| Re-verify with vox graph | Catches any drift since the ~4-month-old audit — needs `vox graph refresh --auto` first per AGENTS.md; more thorough | ✓ |
| Trust PRD + cargo checks | Faster — audit is recent, cargo tree/metadata already required by the acceptance criteria as the ground-truth check | |
| You decide | Let the planner weigh it — e.g. use vox graph only if it's already fresh, skip refresh if stale | |

**User's choice:** Re-verify with vox graph
**Notes:** Uses this repo's own dead-surface detector (`vox graph coverage`) as an independent check before deletion, on top of the acceptance criteria's cargo tree/metadata verification.

---

## Commit granularity

| Option | Description | Selected |
|--------|-------------|----------|
| One commit per crate | Most reviewable and bisectable/revertable — 10 deletions become 10 small commits, matches this repo's atomic-commit culture | |
| Batched by sub-group | e.g. one commit for the vox-scientia-* trio, one for standalone zero-consumer deletes, one for type migrations — fewer commits, still logically separable | ✓ |
| One atomic commit | The whole phase lands as a single commit — simplest history, harder to bisect if something breaks | |

**User's choice:** Batched by sub-group
**Notes:** Three groups identified: (1) vox-scientia-* trio sharing the vox-publisher migration target, (2) standalone zero-consumer deletes, (3) frozen-crate documentation + catalog-cleanup verification.

---

## Claude's Discretion

- vox-scientia-ingest: inline vs. feature-flag implementation shape (user explicitly deferred).
- Sub-grouping order within each batched commit — which standalone crate to delete first, etc.

## Deferred Ideas

None — discussion stayed within Phase 1's boundary. WIRE-UP, EXTRACT-TO-PLUGIN, and MISPLACED dispositions were identified as belonging to Phase 2/Phase 3, not discussed here.
