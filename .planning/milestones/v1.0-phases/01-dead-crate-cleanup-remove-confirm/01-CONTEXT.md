# Phase 1: Dead Crate Cleanup — Remove & Confirm - Context

**Gathered:** 2026-09-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Dispose of the DELETE, KEEP-FROZEN, and catalog-cleanup portions of the 20-crate dead-crate-fate audit: delete zero-consumer crates (migrating any worthwhile types to their true owning crate first), confirm the three frozen crates need no action, and verify `catalog.toml` accurately reflects already-removed ghost entries. WIRE-UP, EXTRACT-TO-PLUGIN, and MISPLACED dispositions are out of this phase's boundary — they belong to Phase 2 and Phase 3.

</domain>

<decisions>
## Implementation Decisions

### vox-scientia-ingest disposition
- **D-01:** After removing vox-scientia-ingest's mandatory `vox-cli` dependency, whether its scholarly-external-jobs functionality is inlined directly into vox-publisher or gated behind a feature flag is left to whoever implements this task — decide based on what reads cleanest once the actual call sites are visible. — **Reversibility:** reversible — either approach lives entirely inside vox-publisher; switching from inline to flag-gated (or back) later is a local, single-crate change.

### Frozen-crate documentation
- **D-02:** Document the three KEEP-FROZEN crates' (vox-workflow-runtime, vox-integration-tests, vox-test-harness) "intentionally inactive" status as entries in `catalog.toml` — the same mechanism REQ-dead-crate-catalog-cleanup already uses for ghost-entry bookkeeping, so frozen-crate status and catalog hygiene live in one place rather than split across crate-level comments or `where-things-live.md`.

### Verification tooling
- **D-03:** Before deleting any crate, re-verify zero-consumer status with this repo's own `vox graph coverage` (dead-surface/OrphanBackend detector) rather than relying solely on the 2026-05-08 PRD audit plus `cargo tree`/`cargo metadata` spot-checks. Run `vox graph status` first and `vox graph refresh --auto` if stale (per AGENTS.md §Code Intelligence — Graphify) before trusting its output. `cargo tree -p vox-cli` / `cargo metadata` checks from the acceptance criteria still run as the final ground-truth confirmation after deletion.

### Commit granularity
- **D-04:** Commit deletions/migrations batched by sub-group rather than one-crate-per-commit or one-atomic-commit: (1) the vox-scientia-* trio (vox-scientia-core, vox-scientia-social, vox-scientia-ingest) together since they share the vox-publisher migration target, (2) standalone zero-consumer deletes (vox-schola, vox-socrates-policy, vox-spool, vox-tools, vox-mcp-meta, vox-browser, vox-audio-ingress) together, (3) the KEEP-FROZEN documentation + catalog-cleanup verification as its own commit.

### Claude's Discretion
- Exact inline-vs-flag implementation shape for vox-scientia-ingest (D-01).
- Sub-grouping order within each batched commit (D-04) — e.g. which standalone crate to delete first — left to the implementer.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements source
- `docs/src/architecture/dead-crate-fate-plan-2026-05-08.md` — the PRD this phase's requirements (REQ-dead-crate-delete, REQ-dead-crate-catalog-cleanup, REQ-dead-crate-keep-frozen) are extracted from verbatim; has the full per-crate rationale and acceptance criteria beyond what's summarized in REQUIREMENTS.md.
- `.planning/REQUIREMENTS.md` — locked acceptance criteria for this phase's three requirements.
- `.planning/PROJECT.md` — milestone-level constraints (build toolchain, crate dependency edges, plugin isolation, test-first policy) that bind this phase's work.

### Dependency/edge policy
- `contracts/ci/crate-edges.allow.v1.json` — the crate-edge ratchet; deleting crates and migrating types (e.g. into vox-orchestrator-types) changes the resolved edge set and must stay consistent with this contract per AGENTS.md §Dependency Discipline.
- `docs/src/architecture/layers.toml` — must be updated (row removed) for each deleted crate; `vox-arch-check` fails otherwise.
- `docs/src/architecture/where-things-live.md` — concept→crate lookup table; remove rows for deleted crates in the same PR per AGENTS.md.

### Verification tooling
- `docs/src/architecture/terminal-exec-policy-ssot.md` — not directly relevant to this phase but adjacent; the actually load-bearing doc is AGENTS.md §Code Intelligence — Graphify (`vox graph status` / `vox graph refresh --auto` / `vox graph coverage --kind <kind>`), which this phase's D-03 decision depends on.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-orchestrator-types/` — the L0 pure-types crate that receives the migrated `ConfidencePolicy`/`ComplexityBand`/`RiskBand` types from vox-socrates-policy, and the `A2A_MESSAGE_TYPES` constants from vox-mcp-meta (that second migration is actually Phase 2's REQ-dead-crate-wire-up work, but both land in the same destination crate — worth sequencing awareness).
- `catalog.toml` — already the bookkeeping mechanism for ghost/removed crate entries (per REQ-dead-crate-catalog-cleanup); D-02 extends its use to frozen-crate documentation too.

### Established Patterns
- Crate naming/layering convention (`docs/src/architecture/where-things-live.md`, `layers.toml`) — every crate removal needs a corresponding row removal in both files, mechanically enforced by `vox-arch-check` and `vox ci crate-edges`.
- Test-first policy (AGENTS.md) — any new `pub fn` created during the vox-scientia-ingest migration into vox-publisher needs an adjacent test per the `skeleton/untested-pub-api` detector.

### Integration Points
- `vox-publisher` — receiving crate for vox-scientia-ingest's scholarly-external-jobs functionality (D-01).
- `vox-orchestrator-types` — receiving crate for socrates-policy's types.

</code_context>

<specifics>
## Specific Ideas

No specific implementation-style requirements beyond the four decisions above — the PRD's acceptance criteria are already prescriptive per-crate; this discussion resolved only the genuinely open choices (ingest disposition, frozen documentation location, verification depth, commit shape).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope. WIRE-UP, EXTRACT-TO-PLUGIN, and MISPLACED dispositions were explicitly identified as out-of-boundary (Phase 2/Phase 3) rather than discussed here.

</deferred>

---

*Phase: 1-Dead Crate Cleanup — Remove & Confirm*
*Context gathered: 2026-09-22*
