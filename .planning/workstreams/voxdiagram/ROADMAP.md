# Roadmap: Vox

## Milestones

- 🚧 **v1.7 VoxDiagram** - Phases 83-91 (in progress; workstream `voxdiagram`)

## Overview

v1.7 retires Mermaid as the primary diagram *authoring* format. A new pure L2 crate, `vox-diagram`, owns a typed text DSL (`graph` and `sequence` kinds) with an all-errors-at-once checker, machine-applicable fixes, a canonical formatter, deterministic and injection-safe SVG, and sanitized Mermaid export plus a strict fail-closed Mermaid importer. Consumers wire it in through exactly the two crate edges the user pre-authorized on 2026-10-16 (`vox-doc-pipeline -> vox-diagram`, `vox-orchestrator-mcp -> vox-diagram`; the CLI reaches it through a `vox-doc-pipeline` re-export). The docs site gets its first committed diagram renderer (Rust pre-render, then a thin remark plugin), the 41 live Mermaid fences in 26 files under `docs/src` migrate behind a shrink-only ratchet, and a paired harness measures whether agents author VoxDiagram at least as reliably as Mermaid.

**Numbering:** Phases start at 83. Phases 77-82 belong to the parallel v1.6 milestone in workstream `unified`. Sequential IDs (`phase_id_convention` unset).

**Granularity:** `.planning/config.json` sets no `granularity`; the orchestrator directed a ~9-phase shape (Fine band, 6-10). Each phase is a distinct delivery boundary with its own risk profile (parser, Mermaid oracle, layout spike gate, sequence, governance edge, agent surface, Astro integration, migration, LLM measurement).

**Dependencies and parallelism**

```text
83 -> 84, 85, 87            (parallel after 83)
84 + 85 -> 86
84 + 85 + 86 + 87 -> 88
85 + 86 + 87 -> 89          (parallel with 88)
84 + 86 + 87 + 88 + 89 -> 90
83 + 84 + 88 -> 91          (parallel with 89 and 90)
```

- **Serial gate:** 83 blocks everything (parser, IR, `check`, `fence` API).
- **Parallel after 83:** 84 (Mermaid emit/import, `vox-diagram` mermaid modules), 85 (layout/SVG, `vox-diagram` layout/svg/theme modules), and 87 (doc lint + ratchet, `vox-doc-pipeline`) touch disjoint modules and can run in separate worktrees. Expect trivial `lib.rs` merge conflicts; on merge, re-verify each item is defined exactly once.
- **86** needs 85 (shared SVG writer/theme/escape primitives) and 84 (sequence Mermaid round-trip). Its DSL parser/printer half may start after 83.
- **88 and 89 are parallel** once 84-87 land (88 is Rust/MCP/CLI; 89 is `docs-astro` + workflows).
- **91 is off the critical path:** it needs only 83, 84 and 88 and runs parallel with 89 and 90.
- **Critical path:** 83 -> 85 -> 86 -> 88/89 -> 90.
- **Shared-file caution (phase 88):** `contracts/operations/catalog.v1.yaml` and its generated artifacts are also touched by the parallel `unified` milestone. Regenerate through the `ssot-autoregen` flow; never hand-regenerate after a merge.

## Phases

**Phase Numbering:**
- Integer phases (83, 84, ...): Planned milestone work
- Decimal phases (e.g. 85.1): Urgent insertions (marked INSERTED)

- [x] **Phase 83: Core Crate - Parser, IR & Diagnostics** - Registered L2 `vox-diagram` crate: graph DSL, recovering parser, typed IR, all-errors diagnostics with fixes, canonical `fmt`, input caps (completed 2026-10-16)
- [ ] **Phase 84: Mermaid Emitter & Strict Importer** - Sanitized Mermaid export that real Mermaid accepts; fail-closed importer verified by corpus round trip
- [ ] **Phase 85: Graph Layout & Deterministic SVG** - `dagre` seam behind a go/no-go spike; byte-identical, injection-safe, themeable SVG
- [ ] **Phase 86: Sequence Kind** - `sequence` DSL, width-aware lane SVG, and DSL round trip for the corpus sequence blocks
- [ ] **Phase 87: Doc-Pipeline Consumer & Legacy-Mermaid Ratchet** - Doc lint validates `vox-diagram` fences and rejects new Mermaid fences via a shrink-only baseline; first authorized crate edge
- [ ] **Phase 88: MCP Tool, CLI & Agent Guidance** - `vox_validate_diagram`, `vox diagram` CLI, and the grammar card agents author from; second authorized crate edge
- [ ] **Phase 89: Docs-Astro Integration** - Pre-render to manifest, remark inline, Playwright proof of ids, label fit and contrast
- [ ] **Phase 90: Corpus Migration & Gate Tightening** - Migrate all convertible live fences in small reviewed batches, then tighten the baseline to the allowlist
- [ ] **Phase 91: Measurement Harness & Advisory Closure** - Paired NL-to-diagram harness with CIs; annotate the advisory's unsourced table

## Phase Details

### Phase 83: Core Crate - Parser, IR & Diagnostics

**Goal**: Authors and agents can write a `graph` diagram in the VoxDiagram DSL and get every mistake reported at once, with machine-applicable fixes and a canonical form, from a registered and governance-clean L2 `vox-diagram` crate.
**Depends on**: Nothing (first phase)
**Requirements**: DGM-01, DGM-03, DGM-04, DGM-05, DGM-06
**Success Criteria** (what must be TRUE):
  1. A `graph` diagram using every DGM-01 construct (`direction`, quoted labels with escapes, nested `container` braces, `a -> b` and `a -> b: "label"`, dotted refs, the six shapes, self-loops, `//` comments) parses to a typed IR whose serde output is identical across repeated runs.
  2. One `check` call over a diagram with several independent mistakes returns every error, not just the first, each with a stable `diagram/*` code, severity, and exact source span; diagnostics with a mechanical remedy carry `fixes[]` of `{span, replacement}`, and applying them to the seeded broken-input set yields a diagram that passes `check`.
  3. `fmt` is canonical and idempotent: `fmt(fmt(x)) == fmt(x)`, and `parse(fmt(x))` equals `parse(x)` (property-tested).
  4. Oversized input, over-nested containers, and an edge whose endpoint is a container are rejected with span-precise errors; adversarial inputs terminate within a bounded time with no panic and no silent mis-layout.
  5. `vox-diagram` is registered at L2 (single crate, no `-core` split) with `layers.toml` row, `where-things-live.md` rows, hakari entry and crate docs, and hosted CI gates (`vox-arch-check`, `vox ci crate-edges`, `ssot-drift`) pass with no new consumer edge.

**Plans**: 7/7 plans complete in 6 waves

Plans:
**Wave 1**
- [x] 83-01-PLAN.md — Tracer `check()` through every layer + L2 crate registration (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 83-02-PLAN.md — Full graph grammar to resolved IR: canonical example, lexer, trivia, determinism guard (wave 2)
- [x] 83-03-PLAN.md — CommonMark fence scanner + sequence IR types (wave 2, parallel with 02)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 83-04-PLAN.md — All-errors recovery, diagnostic catalogue, caps and adversarial corpus (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 83-05-PLAN.md — Canonical idempotent `fmt` + shared proptest generators (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 83-06-PLAN.md — Fix engine (`apply_fixes`/`repair`), native fixes, seeded repair set, hostile properties (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*
- [x] 83-07-PLAN.md — Mermaid-ism leniency + fixes, grammar card, mutate-and-repair, governance closure (wave 6)

**Research flag**: DEEP. DSL surface (conservative, always-quoted strings), error-recovery strategy, and a draft of the <=~600-token grammar card (finalized in Phase 88).
**Folded decisions**: single-crate decision and `max_loc` baseline; `orphan_exempt = true` until Phase 87 wires the first consumer; `sequence` IR types are defined here (consumed by Phases 84 and 86); no direct `blake3`/`sha2` in the crate; tests first per the Test-First Policy.

### Phase 84: Mermaid Emitter & Strict Importer

**Goal**: Any diagram can be exported to Mermaid that real Mermaid parses, and a maintainer can import existing Mermaid blocks without anything being silently lost.
**Depends on**: Phase 83
**Requirements**: REND-06, MIG-01, MIG-02
**Success Criteria** (what must be TRUE):
  1. Mermaid exported from any graph or sequence IR (including hostile labels and ids such as `end`, `o`, `x`, quotes, `<br>`) is accepted by a pinned real Mermaid parser (the sole oracle; no dependency on the uncommitted `mermaid.rs` validator).
  2. Importing a Mermaid flowchart/graph/state/sequence block produces a diagram, or fails closed with a by-line `unsupported` list (e.g. `classDef`, `click`, Gantt); the documented leniencies (unquoted labels, `&` fan-out and chain expansion, `<br>` to newline) work and nothing else is dropped without being listed.
  3. Over the frozen live corpus (all `docs/src` fences at committed HEAD, archive excluded — 33 fences / 21 files on 2026-10-16), import then Mermaid export gives node/edge-set equality against the original block, with the original's sets extracted by an independent path (the pinned Mermaid parser); the Gantt block and the markdown-string block (`how-to-rust-crate-imports.md:53`, rejected by real Mermaid) are reported unsupported, not mangled.

**Plans**: TBD
**Research flag**: standard patterns. Node-side parse oracle lives with the existing `docs-astro` `node --test` suites.
**Parallelism**: can run in parallel with Phases 85 and 87.

### Phase 85: Graph Layout & Deterministic SVG

**Goal**: A `graph` diagram renders to a deterministic, injection-safe, themeable inline SVG that holds up on the largest real diagrams, with layout risk retired by an explicit go/no-go.
**Depends on**: Phase 83
**Requirements**: REND-01, REND-02, REND-03, REND-04
**Success Criteria** (what must be TRUE):
  1. A recorded go/no-go on the 10 largest corpus diagrams (authored as DSL, or imported once Phase 84 lands): `dagre` is adopted only if at most 2 need DSL tweaks, otherwise `dugong` is adopted behind the same one-file adapter; the `dagre`/`dugong` call is wrapped in `catch_unwind` with node and edge caps.
  2. A graph with containers, edge labels, cycles, and self-loops renders to SVG whose geometry satisfies the property-tested invariants (no overlapping nodes, children inside their container, labels inside their boxes), reviewed on a contact sheet of the 10 spike diagrams.
  3. SVG output is byte-identical across repeated runs and across macOS and Linux (`insta` goldens green on both; no `HashMap` order, transcendental floats, versions, or timestamps in output).
  4. Hostile labels (`<script>`, quotes, `&`, `]]>`) are escaped by the single SVG writer; output has no `<style>`, `foreignObject`, or user-derived attributes, per-diagram generated id prefixes, IR-derived `<title>`/`<desc>`, and text as `<text>`; removing the escape makes the guard test fail (mutation-proven).
  5. The same single inline SVG is legible when mounted under light and dark Starlight token sets via CSS variables (`--vd-*` mapped to `--sl-*`), with no duplicated per-theme markup.

**Plans**: TBD
**Research flag**: DEEP. Layout go/no-go spike, label-aware sizing, float determinism across platforms.
**Folded decisions**: font policy (static advance-width table + `unicode-width` with slack vs a pinned subset font) is decided here and verified in a real browser in Phase 89; container-edge rejection already lives in Phase 83.
**Parallelism**: can run in parallel with Phases 84 and 87.
**UI hint**: yes

### Phase 86: Sequence Kind

**Goal**: Authors can write, check, format, and render `sequence` diagrams, and the corpus's sequence blocks round-trip through the DSL.
**Depends on**: Phase 83, Phase 84, Phase 85
**Requirements**: DGM-02, REND-05
**Success Criteria** (what must be TRUE):
  1. An author can write a `sequence` diagram with actors, ordered messages, dashed returns, and `note over`; `check` and `fmt` behave exactly as for `graph` (all errors at once with codes, spans and fixes; idempotent canonical form).
  2. A `sequence` diagram renders to SVG with lane widths driven by the longest message label, so 80+ character messages neither overlap nor clip; output goes through the same safe writer and is byte-identical across runs and platforms.
  3. The 2 corpus sequence blocks import (Phase 84), print as DSL, re-parse to the same IR, render, and re-export to Mermaid that passes the zero-repair check.

**Plans**: TBD
**Research flag**: standard patterns.
**Parallelism**: DSL parser/printer half may start after Phase 83; the render half needs Phase 85's writer. If this phase slips, the 2 sequence blocks stay on the Phase 90 allowlist (needs user sign-off).
**UI hint**: yes

### Phase 87: Doc-Pipeline Consumer & Legacy-Mermaid Ratchet

**Goal**: The docs lint validates every `vox-diagram` fence and stops the legacy Mermaid count from ever growing, before any migration begins.
**Depends on**: Phase 83
**Requirements**: DOCS-01, DOCS-02
**Success Criteria** (what must be TRUE):
  1. Adding a new ```` ```mermaid ```` fence anywhere under `docs/src` (excluding `archive/`) fails doc lint with a clear message; the baseline is seeded at 41 fences / 26 files, can only be tightened (shrink-only), and the pre-push base-ref compare fails if it grows; prose mentions and 4-backtick example blocks are not counted.
  2. A malformed ```` ```vox-diagram ```` fence fails doc lint with the `diagram/*` diagnostic and file:line span, and a valid one passes; the `vox` doctest runner never compiles it (exact info-string matching verified against the `lint.rs` comma-split vs `doctest.rs` whole-string behavior).
  3. `vox-doc-pipeline` re-exports `vox_diagram` (so the CLI needs no third edge), the self-heal-bot pre-PR check also validates `vox-diagram` fences, and `vox ci crate-edges` passes with exactly one new edge, `vox-doc-pipeline -> vox-diagram`, recorded as an exception citing the 2026-10-16 chat authorization; `orphan_exempt` is removed.

**Plans**: TBD
**Research flag**: standard patterns (ratchet copies `facet_backlog.rs`; keep `pipeline/mod.rs` edits small, it is already over the 500-line limit).
**Folded decisions**: no other crate edge is added without fresh user approval; the baseline is never regenerated to admit anything.
**Parallelism**: can run in parallel with Phases 84 and 85.

### Phase 88: MCP Tool, CLI & Agent Guidance

**Goal**: Agents and developers can validate, format, render, export, and import diagrams through supported surfaces, and agents are told to author VoxDiagram.
**Depends on**: Phase 84, Phase 85, Phase 86, Phase 87
**Requirements**: TOOL-01, TOOL-02, TOOL-03
**Success Criteria** (what must be TRUE):
  1. An agent calling `vox_validate_diagram` on a broken diagram receives every diagnostic, `fixes[]`, `normalized_source`, and `stats`; with `from_mermaid` it receives a conversion or the by-line `unsupported` list; the tool is deterministic and performs no LLM calls.
  2. The tool is registered through the operations catalog with dispatch, input schema, fixtures, all generated artifacts, and GUI coverage (no new exemption), the `profile-parity` question is resolved, and `vox ci command-sync`/`ssot-drift` pass; `vox-orchestrator-mcp -> vox-diagram` is the only new edge, recorded citing the 2026-10-16 chat authorization.
  3. A developer can run `vox diagram check|fmt|render|export|import` on files or stdin with exit codes that reflect diagnostics, with catalog rows, needles, and `cli.md` updated and no `vox-cli -> vox-diagram` edge.
  4. The grammar card (<=~600 tokens by measurement, with examples that each pass `check`) is in agent authoring guidance, and `AGENTS.md` directs new docs diagrams to ```` ```vox-diagram ````.

**Plans**: TBD
**Research flag**: standard patterns (follow the ~10-artifact catalog fan-out checklist).
**Folded decisions**: `profile-parity` for new public MCP tools; coordinate catalog regeneration with the `unified` milestone, which also edits the catalog.
**Parallelism**: can run in parallel with Phase 89.

### Phase 89: Docs-Astro Integration

**Goal**: voxlang.org builds and shows every `vox-diagram` fence as an inline, accessible, themed SVG, and the build breaks rather than shipping a missing render.
**Depends on**: Phase 85, Phase 86, Phase 87
**Requirements**: DOCS-03, DOCS-04
**Success Criteria** (what must be TRUE):
  1. `pnpm build` in `docs-astro` pre-renders every ```` ```vox-diagram ```` fence through `vox-doc-pipeline --render-diagrams` into a gitignored manifest and inlines each SVG via the remark plugin in both `.md` and `.mdx`; a fence with no render fails the build with a message naming the file and the exact command.
  2. Playwright against the built site shows no duplicate ids on a page with several diagrams, every `<text>` within its box under two font stacks, and contrast passing in both light and dark themes (forced-colors stays legible), exercised on fixture diagrams covering both kinds.
  3. The docs deploy and quality workflows run the pre-render step before `pnpm build` and trigger on `crates/vox-diagram/**` changes.

**Plans**: TBD
**Research flag**: DEEP. Astro 6/MDX hast `hChildren` on `index.mdx`, `llms-full.txt` retention, Expressive Code/Shiki ordering and `vox-diagram` alias, `astro dev` needing a manual re-render after fence edits.
**Folded decisions**: coordinate with the owner of the uncommitted jsdelivr `mermaid@11` loader in `docs-astro/src/components/PageSidebar.astro` (another session's checkout) so it never processes `vox-diagram` figures; confirm the font-policy decision from Phase 85 in a real browser.
**Parallelism**: can run in parallel with Phase 88.
**UI hint**: yes

### Phase 90: Corpus Migration & Gate Tightening

**Goal**: The live docs corpus uses VoxDiagram, with only an explicit allowlist left as Mermaid and the gate enforcing exactly that.
**Depends on**: Phase 84, Phase 86, Phase 87, Phase 88, Phase 89
**Requirements**: MIG-03, MIG-04
**Success Criteria** (what must be TRUE):
  1. Every convertible ```` ```mermaid ```` fence in `docs/src` (excluding `archive/`) is migrated to ```` ```vox-diagram ```` in small batches (home-page fence first), each batch reviewed on a before/after contact sheet with a fidelity report showing node/edge-set equality.
  2. The legacy-Mermaid baseline is tightened to exactly the allowlist (the Gantt plus any block MIG-01 cannot convert, each listed with a reason), so doc lint now fails if a non-allowlisted fence appears or the list grows.
  3. `git diff` over `docs/src/archive/` is empty, every migrated diagram renders on the built site, and the Phase 89 Playwright suite passes over the full migrated corpus.

**Plans**: TBD
**Research flag**: standard patterns.
**Folded decisions**: the Gantt allowlist entry is already user-decided; any further allowlist entry needs user sign-off.

### Phase 91: Measurement Harness & Advisory Closure

**Goal**: The claim that agents author VoxDiagram at least as reliably as Mermaid is measured and reported honestly, and the advisory's unsourced numbers are no longer presented as fact.
**Depends on**: Phase 83, Phase 84, Phase 88
**Requirements**: EVAL-01, EVAL-02
**Success Criteria** (what must be TRUE):
  1. A paired NL-to-diagram harness runs at least 60 tasks over VoxDiagram and Mermaid arms, with models selected via `vox_actor_runtime::llm` (none pinned, no vendor hostnames), the deterministic parsers as oracle (no LLM judge), and reports first-pass validity and post-bounded-repair validity per arm with Wilson 95% CIs.
  2. The report states which of non-inferior, inconclusive, or inferior the data supports under a decision rule fixed before the run, is committed under `docs/src/architecture/` with valid frontmatter, and re-scoring the recorded model outputs reproduces the tallies deterministically.
  3. The advisory's error-rate table is annotated as unsourced and links to the measured result.

**Plans**: TBD
**Research flag**: DEEP. Harness design, task-set construction, power for the CIs, and the non-inferiority rule.
**Folded decisions**: the harness must not add a crate edge beyond the two authorized (`vox-orchestrator-mcp` already depends on `vox-actor-runtime` and, after Phase 88, `vox-diagram`; a `.vox` script is the alternative per VoxScript-first glue); the result is reported, not a go/no-go on migration.
**Parallelism**: can run in parallel with Phases 89 and 90.

## Progress

**Execution Order:**
Phases execute in numeric order, with the parallel opportunities noted under Overview: 83 -> (84 | 85 | 87) -> 86 -> (88 | 89) -> 90, with 91 alongside 89 and 90.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 83. Core Crate - Parser, IR & Diagnostics | 7/7 | Complete    | 2026-10-16 |
| 84. Mermaid Emitter & Strict Importer | 0/TBD | Not started | - |
| 85. Graph Layout & Deterministic SVG | 0/TBD | Not started | - |
| 86. Sequence Kind | 0/TBD | Not started | - |
| 87. Doc-Pipeline Consumer & Legacy-Mermaid Ratchet | 0/TBD | Not started | - |
| 88. MCP Tool, CLI & Agent Guidance | 0/TBD | Not started | - |
| 89. Docs-Astro Integration | 0/TBD | Not started | - |
| 90. Corpus Migration & Gate Tightening | 0/TBD | Not started | - |
| 91. Measurement Harness & Advisory Closure | 0/TBD | Not started | - |

---
*Roadmap created: 2026-10-16*
