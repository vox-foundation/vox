# Project Research Summary

**Project:** Vox — milestone v1.7 VoxDiagram (GSD workstream `voxdiagram`)
**Domain:** Native text-diagram DSL → typed IR → deterministic SVG + sanitized Mermaid, with Mermaid importer, agent validate tool, and docs-site pre-rendering, inside a layered Rust workspace
**Researched:** 2026-10-16
**Confidence:** MEDIUM-HIGH

> Note: the four researcher files say "v1.6"; the milestone is **v1.7** (v1.6 is the separate Production Stability milestone, Phases 77–82). VoxDiagram phases start at **83**.

## Executive Summary

VoxDiagram is a new pure **L2** crate, `vox-diagram`: hand-written lexer + recursive-descent parser with error recovery, typed AST/IR, canonical printer, layout seam, deterministic SVG emitter, Mermaid emitter, and a strict Mermaid importer. Docs get diagrams via a Rust pre-render step (`vox-doc-pipeline --render-diagrams`) writing a gitignored manifest that a thin remark plugin inlines as SVG; agents get `vox_validate_diagram`, returning all diagnostics in one pass with machine-applicable fixes. The design centers on always-quoted strings, byte-deterministic output, and respecting governance (crate-edge exceptions are user-authorized only).

Resolved scope facts:
- **Corpus:** 41 ```mermaid fences in 26 files under `docs/src` excluding `archive/` — 37 flowchart/graph, 2 sequenceDiagram, 1 stateDiagram-v2, 1 gantt. (STACK/PITFALLS' 40/25 is a counting-method off-by-one.) Archive (29 blocks / 28 files) is tombstoned and out of scope; `docs/superpowers` (~10) is out of gate scope.
- **Current rendering:** committed `main` has **no** Mermaid rendering on voxlang.org. A jsdelivr `mermaid@11` loader in `docs-astro/src/components/PageSidebar.astro` exists only as an *uncommitted* change in another session's checkout — possible concurrent work, not shipped. VoxDiagram is the site's first committed diagram renderer; coordinate, don't assume removal.
- **Diagram kinds:** `graph` and `sequence`. State diagrams fold into `graph` (start/end shapes + self-loops). Gantt stays Mermaid on an allowlist or becomes a table (user decision).
- **Layout:** `dagre` 0.1.1 (pure Rust) behind a one-file adapter, verified deterministic in the STACK spike (40/40 byte-identical runs); fallback `dugong`. Edges to containers are rejected with a span-precise error in v1.7.
- **Governance:** `vox-doc-pipeline → vox-diagram` and `vox-orchestrator-mcp → vox-diagram` need a user-authorization checkpoint; the CLI goes through a re-export from `vox-doc-pipeline` (no third edge).
- **LLM claims:** the advisory's LLM error-rate table (18–35% vs 4–8% etc.) is unsourced and refuted (PITFALLS). Do not carry its numbers into requirements. Any LLM-authoring success claim must be measured by a paired harness with the deterministic parser as oracle.

Main risks: layout quality and determinism (cross-platform floats, HashMap order), governance stalls on crate edges, and silent migration loss. Mitigations: time-boxed layout spike on the 10 largest corpus diagrams; HashMap-free crate, no transcendental floats, golden snapshots; authorization checkpoint first; strict fail-closed importer with corpus round-trip; shrink-only legacy-Mermaid ratchet landed before migration.

## Key Findings

### Recommended Stack

Almost entirely first-party code. New deps: `dagre` and a promoted `unicode-width` (already in `Cargo.lock`). No new npm dep for rendering (`hast-util-from-html` possibly for the remark plugin).

- **`dagre` 0.1.1** behind `layout/dagre_impl.rs` — compound clusters, edge labels, variable node sizes; wrap with `catch_unwind` + node/edge caps. Fallbacks: `dugong` 0.9.0 → `elkrs` (after licence/provenance review).
- **Hand-written lexer/parser** — small grammar, span-precise recovery; no chumsky/winnow/logos.
- **Plain `String` SVG writer** with fixed attribute order and one `xml_escape` (`svg` crate stores attrs in a `HashMap` → non-deterministic; rejected).
- **Static advance-width table + `unicode-width`** — no font crate, bit-deterministic, ~8% slack.
- **Hand-written flowchart/graph importer** — no `merman-core` dep (~20 transitive crates).
- **Own serde `Diagnostic`** with stable `diagram/*` codes; no `miette` in the library.
- **Dev:** `insta`, `proptest`; existing `mermaid.rs` validator is a differential oracle for the Mermaid emitter.

Constraints: no direct `blake3`/`sha2` in `vox-diagram` (sequence-index ids, content-keyed manifests); no cmake/nasm/Go/perl/libclang; `deny.toml` excludes EPL-2.0 (`merman-elk-layered` out); run `cargo hakari generate` after manifest edits.

### Expected Features

**Table stakes:**
- `graph` kind: `direction`; quoted `node` labels with escapes; `container` with brace nesting; `a -> b` and `a -> b: "label"`; dotted refs; shapes `rect`/`rounded`/`diamond`/`cylinder` + `start`/`end`; self-loops; `//` comments.
- Typed AST + resolved IR (serde); span-precise diagnostics with error recovery (all errors per pass); canonical formatter.
- `sequence` kind: actors, ordered messages, dashed return, `note over`, width-aware lanes (messages reach 80+ chars).
- Sanitized Mermaid emitter passing `validate_and_repair_mermaid` with `was_repaired == false`.
- Deterministic SVG on a layered layout: container boxes, edge labels, cycles, self-loops.
- Strict fail-closed Mermaid importer (unquoted-label leniency, `&` fan-out and chain expansion, `<br>` → `\n`, unsupported constructs reported by line).
- Docs gate rejecting new ```mermaid fences (excluding `archive/`) and validating ```vox-diagram fences.
- Migration command (`import --write`, `--check`).
- `vox_validate_diagram`: stable codes, `fixes[]`, `normalized_source`, `stats`, `from_mermaid`.

**Differentiators:** machine-applicable fixes + all-errors-at-once (protect under schedule pressure); IR-generated `<title>`/`<desc>`; one inline SVG themed by CSS variables for both themes; layout stability (small edit → small diff); hairball-size warnings.

**Deferred / anti-features:** ER/class/pie/Gantt kinds; styling language, per-diagram themes, extra edge styles/shapes; chain/fan-out sugar in the DSL, HTML in labels, `|md` blocks, layers/animation/hints; multiple layout engines, client-side rendering, committed SVG/Mermaid fallbacks; compiler keyword, LSP, Axis GUI; LLM repair inside the tool; removing the existing Mermaid lint or `vox_validate_mermaid`.

### Architecture Approach

`vox-diagram` is L2 with no workspace deps — one crate, modules under the 500-line file cap. All I/O lives in consumers: `vox-doc-pipeline` (L3: fence lint, legacy-Mermaid ratchet, `--render-diagrams`), `vox-orchestrator-mcp` (L3: tool), `vox-cli` (`vox diagram check|fmt|render|export|import`, thin, via a `vox_doc_pipeline::diagram` re-export). Docs: Rust pre-render → gitignored manifest keyed by normalized fence source → remark plugin replaces fences with inline SVG via hast nodes (`hName`/`hChildren`, MDX-safe), failing the build on a miss. Rejected: plugin spawning `vox`, WASM, committed SVG. The legacy-Mermaid ratchet copies `facet_backlog.rs`, seeded at 41/26, tightened to the allowlist after migration.

**Major components:**
1. `vox-diagram` — lexer, parser, AST, validate, IR, printer, layout seam, SVG + Mermaid emitters, importer, `fence` (single fence scanner).
2. `vox-doc-pipeline` — `diagrams.rs`, `mermaid_gate.rs`, `LintKind::{InvalidDiagram, LegacyMermaid}`, `--render-diagrams`.
3. `vox-orchestrator-mcp` `diagram_tools.rs` — `vox_validate_diagram` via the operations catalog (~10 artifact fan-out).
4. `vox-cli` `commands/diagram.rs` — thin, no new crate edge.
5. `docs-astro` — `render-diagrams.mjs`, `remark-vox-diagram.mjs`, `--vd-*` → `--sl-*` CSS mapping, workflow path filters for `crates/vox-diagram/**`.

### Critical Pitfalls

1. **Unsourced advisory numbers** — never cite them; success = measured non-inferiority on a paired harness with parser oracle; annotate the advisory table.
2. **Novel DSL with zero training data** — conservative syntax, ≤~600-token grammar card with examples, lenient parse with canonical-rewrite diagnostics, bounded repair loop via the LLM facade; measure with ≥60 tasks, Wilson CIs, no LLM judge.
3. **Non-determinism** — no `HashMap`, stable sorts with declaration-index ties, no transcendental floats, quantized coordinates, no versions/timestamps embedded; `insta` goldens on macOS + Linux and repeat-run tests.
4. **Layout rabbit hole** — time-boxed spike on the 10 largest diagrams; labels as first-class layout objects; reject container edges; cap nesting; proptest geometric invariants.
5. **Governance** — batch-request edge exceptions up front; never regenerate a baseline to admit an edge; register layer + where-things-live rows.
6. **Injection/escaping** — labels untrusted; single escaping writer; generated ids only; no `<style>`/`foreignObject`/user-derived classes or hrefs; input caps; mutation-verified escape test.
7. **Migration loss** — fail-closed importer; node/edge-set equality through emitter round-trip; small batches with contact sheet; never touch `archive/`.
8. **Other traps** — validate Mermaid export against a real Mermaid parser (`end`, `o`/`x` ids, quoting); per-diagram id prefixes for inline SVG; info string exactly `vox-diagram` + Shiki alias; verify text measurement in a real browser under two font stacks.

## Implications for Roadmap

Phases start at **83**. Human gate first, parser/IR before everything, ratchet before migration; after the doc-pipeline consumer, gate/MCP/CLI/importer tracks can run in parallel worktrees.

- **Phase 83 — Authorization & decision checkpoint:** one batch approval for the two edge exceptions, CLI re-export vs direct edge, `vox-diagram` L2 row + `max_loc`, single-crate decision; decisions on Gantt (allowlist vs table), the concurrent Mermaid CDN loader, in-scope tree, `profile-parity`; advisory annotation; decision record for the measured-non-inferiority metric.
- **Phase 84 — Core crate (parser, IR, diagnostics):** scaffold + all registrations; lexer, recovering parser, AST, validate, IR, canonical printer, `fence` module, stable `diagram/*` diagnostics with fixes, size/depth caps; grammar card; sequence IR types. Tests first.
- **Phase 85 — Mermaid emitter + strict importer:** sanitized emitter; strict flowchart/graph importer with `unsupported` channel; round-trip/equality tests over 37 graph blocks + the state diagram; zero-repair validation; Node-side real-Mermaid parse oracle with adversarial corpus.
- **Phase 86 — Layout + SVG emitter (graph):** `dagre` seam with `catch_unwind` + caps; go/no-go spike (≤2 of 10 need DSL tweaks, else `dugong`); text metrics + wrapping; fixed-order SVG writer, CSS-variable theming, id prefixes, `<title>`/`<desc>`, no `<style>`; self-loops/back-edges; proptests + `insta` goldens on macOS and Linux; container-edge rejection.
- **Phase 87 — Sequence kind:** parser additions, width-aware lane layout, Mermaid emit, importer for the 2 blocks. Can slip without stranding migration.
- **Phase 88 — Doc-pipeline consumer + legacy-Mermaid ratchet:** `diagrams.rs`, `LintKind::InvalidDiagram`, `--render-diagrams`, re-export, self-heal-bot check; `mermaid_gate.rs` seeded 41/26, shrink-only tighten + pre-push base-ref compare; exact-`vox` doctest matching verified; tests for prose mentions and 4-backtick examples.
- **Phase 89 — MCP tool + CLI:** `vox_validate_diagram` via the operations catalog with full artifact checklist (dispatch, schema, fixtures, `command-sync`, GUI coverage — no new exemption); `vox diagram check|fmt|render|export|import` with catalog rows, needles, cli.md.
- **Phase 90 — Docs-astro integration:** pre-render script, remark plugin (hast), lockfile + path filters, CSS mapping with forced-colors, Shiki alias; build-output specs incl. `llms-full.txt` retention; Playwright for duplicate ids, `<text>` bbox fit under two font stacks, axe contrast both themes. Spikes: hChildren on `index.mdx`, llms-txt retention, Expressive Code ordering.
- **Phase 91 — Migration + gate tightening:** `import --write` over 26 files in small batches with contact sheet + fidelity report; home-page fence first; baseline tightened to the allowlist; archive diff empty.
- **Phase 92 — Measurement + hardening:** paired NL→diagram harness (≥60 tasks, Wilson CIs, parser oracle, models via the facade, none pinned); DSL reference page (dogfoods renderer); AGENTS.md + advisory updates; optional openwiki generators emitting `vox-diagram`.

### Research Flags

Need deeper research at plan time: **84** (DSL surface, grammar card), **86** (layout spike, text metrics, float determinism), **90** (Astro 6/MDX hast, llms-txt, Expressive Code), **92** (harness design, power). Standard patterns: 83 (human checkpoint), 85, 87, 88, 89, 91.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | `dagre` built and run (40/40 identical); rejections verified from source; single-maintainer risk mitigated by seam + `dugong`. `elkrs`/`merman-core` licence LOW, off critical path. |
| Features | HIGH / MEDIUM | Corpus measured (41/26 reconciled); D2 sequence + Mermaid a11y fetched; rest from training knowledge. |
| Architecture | HIGH / MEDIUM | Repo integration read from source; Astro/MDX/llms-txt/Expressive Code behavior flagged as spikes. |
| Pitfalls | MEDIUM-HIGH | Advisory refutation + repo traps HIGH; layout/text/novel-DSL guidance MEDIUM; OmniDiagram LOW, unread. |

### Gaps to Address

- Concurrent Mermaid CDN loader — confirm status with its owner (Phase 83), coordinate in Phase 90.
- `dagre` maintenance (single maintainer, no commits since May 2026) — keep the seam; re-spike on bumps.
- Font policy — static table with slack vs pinned subset font; decide in 86, verify in browser.
- Gantt / non-flowchart residue — allowlist entries are user-authorized only.
- `profile-parity` for new public MCP tools — confirm in 89.
- `lint.rs` splits info strings on `,`, `doctest.rs` compares whole string — verify in 88.
- `astro dev` needs manual re-render after fence edits — accept or fix.
- Published IR JSON Schema — defer.
- `vox-cli` `max_workspace_deps = 60` vs 66 baseline edges — re-export sidesteps it.
- Confirm nothing else claims phases ≥83 (phase-numbering index).

## Sources

**Primary (HIGH):** repo source/contracts (`layers.toml`, `where-things-live.md`, `crate_edges.rs`, `crate-edges.allow.v1.json`, `vox-doc-pipeline` mermaid/lint/doctest/facet_backlog, `vox-orchestrator-mcp` dispatch/input_schemas/mermaid_repair, operations catalog, docs workflows, docs-astro plugins); fence-aware corpus census; scratch crate running `dagre =0.1.1`; `svg-0.18.0` and `dagre-0.1.1` sources; crates.io/npm checks (2026-10-16); arXiv 2511.14967 (MermaidSeqBench), arXiv 2510.25761 (DiagramEval); d2lang sequence docs; Mermaid accessibility/flowchart docs; Rust `f64` docs.

**Secondary (MEDIUM):** Grammar Prompting (arXiv 2305.19234); Synthetic Programming Elicitation; DIAGRAM-FAIL-BENCH poster; Mermaid CVE-2026-50159 / CVE-2026-41159 summaries; Graphviz/ELK compound-layout notes; Astro/MDX hast contract.

**Tertiary (LOW):** OmniDiagram (arXiv 2604.05514, search summary only); `elkrs` provenance/licence; `merman-core` licence closure; the advisory's error-rate table (unsourced, refuted — never cite).

---
*Research completed: 2026-10-16 — Ready for roadmap: yes*
