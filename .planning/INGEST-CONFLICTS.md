## Conflict Detection Report

Full-corpus re-run: 430 classifications (49 ADR, 96 SPEC, 1 PRD, 284 DOC). This report supersedes the
prior full-corpus run's conflicts report — that run's single BLOCKER and single WARNING were both
resolved in the source tree before this run and are verified resolved below.

### BLOCKERS (0)

None. Independent verification of the two fixes described in the ingest prompt:

- **ADR-037 (Tauri Convergence) vs adr-NNN (Scope Tauri to desktop only):** ADR-037's own Status line
  now reads "Partially superseded (2026-06-12) by adr-NNN" and its classification is `locked: false`.
  adr-NNN remains `locked: true`, Accepted 2026-06-12, and explicitly supersedes ADR-037 for the
  mobile-target clause only. This is no longer LOCKED-vs-LOCKED — confirmed resolved. See INFO entry
  below for how the still-valid desktop-convergence content of ADR-037 is represented.
- **ADR-024 (Dashboard as Axum SPA) vs ADR-045 (Tauri GUI Replaces Axum Dashboard):** ADR-024's Status
  line now reads "Superseded (2026-05-11) by ADR-045" and its classification is `locked: false`.
  `crates/vox-dashboard` is absent from the current tree (confirmed via filesystem: only
  `crates/vox-gui` and `crates/vox-runtime-rn` exist), corroborating the supersession claim. Confirmed
  resolved. See WARNING entry below for the residual oddity this uncovers (ADR-045 itself is not
  locked/Accepted).

No LOCKED-vs-LOCKED ADR contradiction exists anywhere else in the 49-ADR set (heuristic scope-overlap
scan run across all ADR/SPEC/PRD pairs; every overlap ≥2 shared scope terms was inspected by hand —
all are companion/building-block relationships, not contradictions). No UNKNOWN or low-confidence
classifications exist in the 430-doc set (all high or medium confidence, all correctly typed). Cross-ref
cycle detection (DFS, max depth 50, no cap exceeded) found 75 cycles — see INFO entry — none of which
are supersession loops or otherwise gate synthesis.

### WARNINGS (2)

[WARNING] ADR-045 is the de facto current dashboard/GUI decision but carries no locked/Accepted status
  Found: docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md has frontmatter `status: "current"` and no
  explicit "Status: Accepted" line in the body; its classification is `locked: false`, `confidence: medium`.
  Impact: Now that ADR-024 (its predecessor) is marked Superseded, ADR-045 is the only non-contradicted
  source for dashboard/GUI architecture — but a non-locked, medium-confidence document is standing in as
  the authoritative decision. A future locked ADR could in principle override it without ceremony, since
  nothing currently locks the vox-gui/Tauri decision in place.
  → Recommend formally accepting ADR-045 (add "Status: Accepted" + `locked: true`-equivalent frontmatter)
  in a follow-up doc edit, or explicitly ratify it as part of routing this intel to ROADMAP.md/PROJECT.md.

[WARNING] Locked ADRs 030 and 031 still name the decommissioned "vox-dashboard" surface
  Found: docs/src/adr/030-state-machine-ssot.md (locked, Accepted) scopes itself partly to "vox-dashboard"
  as the reactive-UI host; docs/src/adr/031-deprecate-vox-vscode.md (locked, Accepted) explicitly names
  "`vox-dashboard` is the primary user surface" in its own title/decision. Per the ingest instruction,
  these were deliberately left unedited because their core decisions (state_machine as reactive-state SSoT;
  vox-vscode deprecated in favor of a GUI surface) are not themselves contradicted by the dashboard
  architecture change — only their named successor-surface is stale (vox-dashboard doesn't exist;
  vox-gui does, per ADR-045).
  Impact: A downstream reader of decisions.md taking ADR-031's title literally would conclude
  vox-dashboard is still the primary user surface, which is false as of ADR-024/045's resolution.
  → decisions.md entries for ADR-030/031 carry an explicit `note:` flagging the stale reference; no source
  edit is required unless/until someone revises those ADRs directly. Downstream consumers (gsd-roadmapper)
  should treat "vox-gui" as the current primary surface, not "vox-dashboard," regardless of ADR-030/031's
  literal wording.

### INFO (5)

[INFO] Auto-resolved: ADR-047 (locked) > ADR-020 (superseded) > ADR-008/ADR-009 (non-locked) on populi mesh transport
  Note: ADR-047 (iroh QUIC replaces the bespoke populi mesh transport, locked, Accepted) is the current
  transport decision. ADR-020 (Populi mesh scaling default transport posture) was itself Superseded
  (2026-09-04) by ADR-047 per its own Status line and is now `locked: false`; its "keep HTTP as default"
  posture is historical only. ADR-008 (Mens transport: in-tree HTTP/Axum control plane, non-locked) and
  ADR-009 (Hosted mens/BaaS future scope, non-locked) predate this and are superseded in practice by the
  same shift, though neither carries an explicit supersession marker in its own text. decisions.md
  entries for ADR-008 and ADR-020 carry explanatory notes; ADR-047 is the entry synthesis treats as
  current.

[INFO] Auto-resolved: ADR-041 (locked) > ADR-028 (non-locked proposal) on durable functions / stub grammar
  Note: ADR-028 proposed removing `@scheduled`/`@durable`/`workflow`/`activity` from the public grammar
  after an audit found zero runtime implementation (non-locked, proposal status). ADR-041 (locked,
  Accepted) later closed that implementation gap with a working runtime, codegen, and journal-backed
  replay, and explicitly supersedes ADR-028's reservation-gate approach (confirmed independently by
  AGENTS.md §Grammar Unification: "ADR-041 supersedes the old ADR-028 reservation gate"). ADR-041 wins;
  ADR-028's removal proposal is moot.

[INFO] Auto-resolved: ADR-045 (current, non-locked) > ADR-024 (superseded, non-locked) on dashboard architecture
  Note: With the former BLOCKER cleared (ADR-024 marked Superseded, `locked: false`), ADR-045 stands as
  the sole non-contradicted decision for dashboard/GUI architecture: decommission `vox-dashboard`, unify
  on Tauri 2 `vox-gui` with the CLI's `CommandCatalog` as SSOT. See WARNING above — this is a valid but
  fragile resolution since ADR-045 is not itself locked.

[INFO] Auto-resolved: adr-NNN (locked) partially supersedes ADR-037 (now non-locked) — split by clause
  Note: adr-NNN (Scope Tauri to desktop only; React Native + Expo + uniffi for mobile, locked, Accepted
  2026-06-12) supersedes ADR-037's mobile-target clause specifically. ADR-037's desktop-convergence
  clause (Tauri 2 as the desktop application shell) is unaffected and remains Accepted/active — it was
  never contradicted by adr-NNN, which only claims the mobile scope. decisions.md represents ADR-037 as
  `status: proposed (partially superseded)` with a note explaining the split, rather than dropping its
  still-valid desktop content or promoting it back to `locked` (its own source Status line no longer
  reads a bare "Accepted," so classification correctly reflects `locked: false` for the document as a
  whole while the note preserves which clause is still live).

[INFO] Cross-reference cycle detection: 75 cycles found, all companion-document "see also" pairs, none blocking
  Note: DFS cycle detection over the `cross_refs` graph (max depth 50, cap never hit) found 75 cycles.
  Every cycle inspected is a mutual cross-link between companion documents that legitimately reference
  each other (e.g. a design spec and its implementation plan, an audit and its follow-up plan, an ADR and
  its own migration-plan/audit trail — including the already-resolved 020↔047, 024↔dashboard-migration-
  research, and 037↔adr-NNN↔mobile-target-evaluation↔tauri-convergence-migration-plan clusters). None of
  the 75 cycles is a supersession loop (no case where A supersedes B supersedes A) or otherwise makes
  precedence resolution undefined. All docs in the corpus, including every doc inside a detected cycle,
  were synthesized normally.
