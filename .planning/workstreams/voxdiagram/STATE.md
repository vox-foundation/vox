---
gsd_state_version: "1.0"
milestone: v1.7
milestone_name: VoxDiagram
current_phase: 91
current_phase_name: Measurement Harness & Advisory Closure
current_plan: 5
status: verifying
stopped_at: Completed 91-05-PLAN.md
last_updated: "2026-10-27T23:51:58.311Z"
last_activity: 2026-10-27
last_activity_desc: Phase 91 execution started
state_head: bfafe4912153dc3e1cfaf467a045078d63686b23
progress:
  total_phases: 9
  completed_phases: 8
  total_plans: 42
  completed_plans: 42
---

# Project State: VoxDiagram (`voxdiagram`)

## Project Reference

See: `.planning/PROJECT.md` (shared, owned by the `unified` workstream, not edited here) and `.planning/workstreams/voxdiagram/PROJECT-MILESTONE.md`.

**Core value:** For this milestone, agents and humans author diagrams that cannot render-crash, and every live docs diagram renders deterministically.
**Current focus:** Phase 91 — Measurement Harness & Advisory Closure

## Current Position

Phase: 91 (Measurement Harness & Advisory Closure) — EXECUTING
Plan: 5 of 5
Status: Phase complete — ready for verification
Last activity: 2026-10-27 — Phase 91 execution started

Progress: [█████████░] 89% (0/9 phases)

## Progress

**Phases Complete:** 0
**Current Plan:** 5

## Key Decisions (milestone-level)

- **Numbering:** Phases 83-91; phases 77-82 belong to the parallel v1.6 milestone in workstream `unified`.
- **Crate edges (pre-authorized by the user 2026-10-16, chat):** exactly `vox-doc-pipeline -> vox-diagram` (lands in Phase 87) and `vox-orchestrator-mcp -> vox-diagram` (lands in Phase 88). The CLI uses a `vox-doc-pipeline` re-export. Any other edge needs fresh approval; never regenerate a baseline to admit an edge.
- **No standalone authorization phase:** font policy folded into Phase 85, Mermaid CDN loader coordination into Phase 89, `profile-parity` into Phase 88, non-inferiority rule into Phase 91.
- **Corpus baseline:** 41 mermaid fences / 26 files under `docs/src` (excluding `archive/`): 37 graph, 2 sequence, 1 state, 1 gantt. Gantt stays Mermaid on the allowlist.
- **Parallelism:** after Phase 83, Phases 84, 85, 87 can run in parallel; 88 and 89 in parallel; 91 alongside 89 and 90.

## Accumulated Context

### Pending Todos

None yet.

### Blockers/Concerns

- An uncommitted jsdelivr `mermaid@11` loader exists in another session's checkout (`docs-astro/src/components/PageSidebar.astro`); coordinate in Phase 89, do not assume removal.
- `dagre` 0.1.1 is single-maintainer with no commits since May 2026; keep the one-file adapter and `dugong` fallback (Phase 85 go/no-go).
- Phase 88 touches the operations catalog and generated artifacts that the `unified` milestone also edits; regenerate through `ssot-autoregen`.
- Any allowlist entry beyond the Gantt (Phase 90) requires user sign-off.

## Session Continuity

**Last session:** 2026-10-27T23:51:58.228Z

**Stopped At:** Completed 91-05-PLAN.md
**Resume File:** None

## Performance Metrics

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 83 P01 | 70min | 2 tasks | 21 files |
| Phase 83 P02 | 60min | 3 tasks | 14 files |
| Phase 83 P03 | 20min | 2 tasks | 5 files |
| Phase 83 P04 | 75min | 3 tasks | 30 files |
| Phase 83 P05 | 45m | 3 tasks | 9 files |
| Phase 83 P06 | 80m | 3 tasks | 41 files |
| Phase 83 P07 | 3h | 3 tasks | 37 files |
| Phase 84 P01 | 50min | 3 tasks | 55 files |
| Phase 84 P02 | 45min | 3 tasks | 15 files |
| Phase 84 P03 | 50min | 3 tasks | 10 files |
| Phase 84 P04 | 45min | 2 tasks | 10 files |
| Phase 84 P05 | ~1h | 3 tasks | 60 files |
| Phase 85 P01 | 45min | 2 tasks | 23 files |
| Phase 85 P02 | 40min | 2 tasks | 14 files |
| Phase 85 P03 | 75min | 2 tasks | 12 files |
| Phase 85 P04 | 75min | 3 tasks | 9 files |
| Phase 85 P05 | 70min | 3 tasks | 18 files |
| Phase 86 P01 | 1 session | 2 tasks | 38 files |
| Phase 86 P02 | one session | 3 tasks | 19 files |
| Phase 86 P03 | 45min | 3 tasks | 15 files |
| Phase 86 P04 | 55min | 3 tasks | 22 files |
| Phase 87 P01 | 50min | 3 tasks | 13 files |
| Phase 87 P02 | 35min | 2 tasks | 7 files |
| Phase 87 P03 | 40min | 2 tasks | 8 files |
| Phase 88 P01 | 75min | 3 tasks | 40 files |
| Phase 88 P02 | 70min | 2 tasks | 22 files |
| Phase 88 P03 | 50min | 3 tasks | 8 files |
| Phase 89 P01 | 35min | 3 tasks | 12 files |
| Phase 89 P02 | 40min | 2 tasks | 7 files |
| Phase 89 P03 | resumed | 3 tasks | 8 files |
| Phase 89 P04 | 1 session | 3 tasks | 7 files |
| Phase 91 P02 | 189min | 2 tasks | 15 files |
| Phase 91 P03 | 48min | 2 tasks | 8 files |
| Phase 91 P04 | 55min | 3 tasks | 13 files |
| Phase 91 P05 | 79min | 2 tasks | 8 files |

## Decisions

- [Phase 83]: 83-01: all phase deps declared in the tracer plan so Cargo.toml/Cargo.lock have one owner; later plans must not touch them
- [Phase 83]: 83-01: resolver keeps first declaration on duplicate ids with no diagnostic until plan 03
- [Phase 83]: 83-02: lexer emits its own diagnostics and best-effort tokens (Str terminated:false, BadIdent) that the parser accepts, so one mistake yields one diagnostic
- [Phase 83]: 83-02: direction: is a statement only directly inside graph { }; in a container it is an unexpected-token
- [Phase 83]: 83-04: Collector tracks has_error apart from stored diagnostics so dropped Errors still invalidate the report
- [Phase 83]: 83-04: items past the node/edge cap stay in the AST (IR withheld by the Error) so edges to them do not cascade into undeclared-node
- [Phase 83]: 83-04: skip_to_stmt_end is brace-aware and stops at a line-leading node/container; unquoted labels are recovered as the label
- [Phase 83]: 83-05: generator ids unique by DFS counter suffix; edges are indices into declared nodes; noise is a shrinkable die stream
- [Phase 83]: 83-06: apply_fixes applies each diagnostic's fixes as one atomic group, rejecting out-of-range, off-boundary or overlapping groups; equal-offset inserts keep diagnostic order and are never deduplicated
- [Phase 83]: 83-06: undeclared-node declare-fix only for a single-segment ref or the last segment of a dotted ref; scopes without both braces get no fix until closed (repair converges in a second round)
- [Phase 83]: Mermaid free-text forms (links with labels, id[Label] brackets, styling lines, %% comments) are single lexer tokens so their text never raises lexical noise
- [Phase 83]: M3 id[Label] is reported by the resolver (needs scope braces for Rule R); chains and fan-out get one whole-statement fix
- [Phase 83]: Lexer caches the line end so a line of links or brackets is scanned once (quadratic scan found by adversarial_mermaid_tokens)
- [Phase 84]: 84-01: importer id grammar rejects ids ending in '-' so a--->b fails closed; unit tests stay inline in new mermaid src files
- [Phase 84]: 84-02: Mermaid ids assigned nodes, then containers, then dangling endpoints in one namespace; blank message text prints bare, blank actor and note text prints #32;
- [Phase 84]: 84-03: unquoted -- text --> edge text allows all but the double quote (measured), the pipe form keeps the full special set
- [Phase 84]: 84-03: a refused subgraph header still opens a frame so its end balances; limit-* constructs are reported once on the crossing line
- [Phase 84]: stateDiagram-v2 imports into the graph kind: [*] as source/target becomes start/end terminals, printed via flow::to_dsl
- [Phase 84]: sequenceDiagram imports into ImportReport.sequence with LineIndex spans; participant-after-use and redeclared-participant fail closed instead of last-wins
- [Phase 84]: Free-text semicolon check is entity-aware (#59; is text); HTML entities in state/sequence text report semicolon
- [Phase 84]: 84-05: F2 sequence fixpoint draws text from DSL-legal characters; the importer refuses forbidden chars even as numeric entities by design
- [Phase 85]: 85-01: dagre confined to layout/dagre_impl.rs behind the Engine seam; all coordinates quantized to integer half-pixels at the seam
- [Phase 85]: 85-01: desc/endpoint-label truncation counts the ellipsis inside the limit (24 / 300); cylinder drawn as one rect plus a vd-node-rim path
- [Phase 85]: 85-02: verify padding rule w>=tw+4*PAD_X, h>=th+4*PAD_Y (diamond half-extent); vd-edge-labels group only emitted when an edge has a label
- [Phase 85]: 85-03: container titles use a 16 px line pitch (band = 10 px + 16 px/line); shapes::label takes a (class, fill) pair and a step; vd-containers group emitted only when containers exist
- [Phase 85]: 85-03: parent cycle and over-deep nesting share one InvalidIr (depth walk stops after MAX_DEPTH hops); labels over MAX_LABEL_CHARS are InvalidIr for nodes, containers and edges
- [Phase 85]: 85-04: SVG oracle is an independent quick-xml allowlist checker; label centring asserted within one half-pixel (odd-size boxes have no representable centre); REND-03 ticked, REND-01/02/04 left for 85-05
- [Phase 85]: GO on dagre 0.1.1 (exact pin, one-file adapter): 30/30 importable corpus graphs render, 0 of 10 need a DSL tweak; edge-through-container-title and ~4% generated-graph Geometry fail-closed recorded as residual risks
- [Phase 86]: 86-01: actor, reply, note and over are contextual keywords (plain identifiers unless the next token says otherwise); RESERVED stays the only reserved list
- [Phase 86]: 86-01: sequence geometry runs in i64 half-pixels and fails closed past a 100000 px canvas (RenderError::Geometry)
- [Phase 86]: 86-02: undeclared sequence actors are declared after the last actor line (insert_decl when the closing brace shares it), once per id, keeping first-mention lane order
- [Phase 86]: 86-02: Mermaid --> in a sequence rewrites to 'reply a -> b' like -->>; other Mermaid links and side notes are reported without a fix
- [Phase 86]: 86-03: note text soft-wraps at 60 cells like messages; REND-05 left open for 86-04 (corpus block + browser review)
- [Phase 86]: 86-04: ImportReport.sequence kept; source.is_some() == unsupported.is_empty() for every kind; empty Note text refused as empty-label (DSL has no empty note)
- [Phase 87]: 87-01: one exceptions entry admits vox-doc-pipeline -> vox-diagram (2026-10-16 authorization); near-miss vox-diagram info strings are hard diagram-fence-info errors
- [Phase 87]: 87-02: legacy-Mermaid baseline seeded 33 fences/21 files from committed tree; missing baseline admits nothing, invalid is one hard error; tighten refuses before writing on growth
- [Phase 87]: 87-03: capability-index staleness (no crate.vox-diagram row) predates the plan; left to hosted-CI ssot-autoregen, not hand-committed
- [Phase 88]: 88-01: vox_validate_diagram envelope is vox_diagram::validate serialized as-is; Mermaid input validates the converted DSL; only edge vox-orchestrator-mcp -> vox-diagram (user-authorized 2026-10-16)
- [Phase 88]: 88-02: vox diagram CLI rows carry side_effect_class (writes_files / read_only) so the side-effect ratchet stays untouched
- [Phase 88]: 88-03: grammar card covers graph and sequence (2101 bytes); reference page publishes it verbatim, guarded by reference_page_publishes_the_card_verbatim
- [Phase 89]: 89-01: dedupe tracks tried keys so a failing diagram source is reported once, from the first file holding it
- [Phase 89]: 89-01: built-page source block is verified via data-language=vox-diagram; the language-vox-diagram class only appears in llms-full (plan 89-03)
- [Phase 89]: 89-02: horizontal padding floor pad_x (box >= 112% of measured width) shared by every padded text box; table and 8% slack unchanged
- [Phase 89]: 89-03: dev content refresh via refreshContent() on astro:server:setup instead of deleting Astro's data store (store is .astro/data-store.json; deleting it under a running dev server 404s every page)
- [Phase 89]: 89-04: browser checks discover figure pages from dist/; review bundle holds shipped and natural-size PNGs
- [Phase 91]: 91-02: every quoted span of a task prompt must be a text of the expected sets; banned syntax words match whole words and plurals
- [Phase 91]: 91-02: Rust rescore parses the decision rule only once a run exists (the rule lands in the dataset manifest in 91-04)
- [Phase 91]: Hard-case quotas are measured per task from the oracle's canonical sets (reference-outputs.jsonl), never from tags; card/template/manifest-hash gates live in eval_diagram_frozen.rs
- [Phase 91]: 91-04: the spend cap is checked before every model call including each transport retry and every probe; spend = attempt costs under runs/ + probe-spend.jsonl
- [Phase 91]: 91-04: a negative, non-finite or missing reply cost stops the run (cost_unknown); the unpriced reply is still recorded
- [Phase 91]: 91-04: pre-registration commit f1ef3bb50; the live run refuses unless its freeze commit is that commit, an ancestor of HEAD, with dataset and report unchanged (vox_git::read_only)
- [Phase 91]: 91-04: oracle-outputs.jsonl is written with Node's own bytes; the Backend's Mermaid batch is JSONL in, JSONL out
- [Phase 91]: EVAL-01 primary verdict (pre-registered rule, run 2026-10-27): non-inferior; model A deepseek/deepseek-v4.1-flash-1017 first-pass delta +2.0 points, Newcombe 95% [-1.7, +5.9]; model B xiaomi/mimo-v2.6-pro -0.5 [-4.0, +2.9]; spend 0.285 USD of 25

## Pending Todos (voxdiagram)

- At the next merge of local `main` (`ceb1b5d1a`, other session's v1.5/v1.6) into `milestone/v1.7-voxdiagram`: (1) call `vox_doc_pipeline::pipeline::fence_issues(md)` from `self_heal_bot::validate_bot_patch_scope_and_gates` + one test; (2) re-seed `contracts/documentation/legacy-mermaid-baseline.v1.yaml` (user re-seeds; main has 40 fences / 25 files) — decided 2026-10-17.
- Before opening the milestone PR: `vox-orchestrator-mcp` `profile_backcompat_tests` (2) fail at the phase base because the advertised-tools fixture lacks 14 already-registered tools; this PR touches the crate, so hosted `tests` will run them — fix the fixture (or confirm they are quarantined) in PR prep. Also expect `cargo clippy -p vox-doc-pipeline` to flag 26 pre-existing `claims/` findings on the affected-crate clippy step. — noted 2026-10-17.
