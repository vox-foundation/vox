# Feature Landscape

**Domain:** Text diagram DSL + deterministic SVG/Mermaid emitters + Mermaid importer + agent validate tool + docs-build integration (VoxDiagram, `vox-diagram` crate)
**Milestone:** v1.6 VoxDiagram (companion track)
**Researched:** 2026-10-16
**Overall confidence:** HIGH for corpus inventory (measured by script over the repo), HIGH for D2 sequence semantics and Mermaid a11y (official docs fetched), MEDIUM for the remaining D2/Mermaid feature lists (training knowledge, not re-fetched).

---

## 0. Corpus Inventory (the deciding evidence)

### 0.1 Scope correction: the "~71 blocks in ~55 docs" figure includes the archive

Counted with a fence-aware script (`` ``` `` and `~~~`, `.md` and `.mdx`, indented fences included), cross-checked with `grep`:

| Scope | Blocks | Files |
|-------|--------|-------|
| `docs/src/` including `archive/` | 70 | 54 |
| `docs/src/archive/` only (tombstoned) | 29 | 28 |
| **`docs/src/` excluding `archive/` (the real migration scope)** | **41** | **26** |
| `docs/superpowers/` (outside `docs/src`, not in stated scope) | 10 | 10 |
| `README.md`, `docs/plans`, `docs/agents`, `docs/news`, `docs/ci` | 0 | 0 |

The milestone brief's "~71 blocks / ~55 docs, excluding archive" matches the count **including** the archive. Excluding it, the migration is **41 blocks in 26 files**. AGENTS.md's Archival Protocol says agents must not ingest or modify `docs/src/archive/`, so the 29 archive blocks should stay Mermaid forever. The roadmap should say "41 blocks / 26 files" and put "docs gate ignores `archive/`" in the acceptance criteria. The 10 `docs/superpowers/` blocks are out of scope but would trip a naive repo-wide gate, so scope the gate to `docs/src/` explicitly.

Whether `docs/src/` is the only tree that matters for the site: `docs-astro/src/content/docs/` also holds a generated mirror (for example `repo/planning-workstreams-unified-roadmap-md.md`). It is derived, so do not count or migrate it.

### 0.2 Diagram kinds, `docs/src/` excluding `archive/` (41 blocks)

| Kind | Blocks | Share | Notes |
|------|--------|-------|-------|
| `flowchart` | 32 | 78% | 17 `LR`, 14 `TD`, 1 `TB` |
| `graph` (legacy header) | 5 | 12% | all `TD`; same semantics as flowchart |
| **flowchart + graph total** | **37** | **90%** | |
| `sequenceDiagram` | 2 | 5% | |
| `stateDiagram-v2` | 1 | 2% | `reference/populi.md` |
| `gantt` | 1 | 2% | `architecture/deep-research-full-surface-audit-and-roadmap-2026.md:485` |
| `erDiagram` | 0 | 0% | |
| `classDiagram` | 0 | 0% | |
| any other (pie, journey, gitGraph, mindmap, timeline, block, ...) | 0 | 0% | |

Decision this forces: **v1.6 must support two diagram kinds, `graph` (covers flowchart, graph, and state) and `sequence`.** ER and class diagrams have zero uses, so the "kinds" extension point should exist in the IR but nothing is built for them. The single Gantt is an explicit exemption (see 3.3).

Largest block sizes: flowchart max 31 non-blank lines, max 21 edges. All fit a simple layered layout.

### 0.3 Features actually used, flowchart/graph (37 blocks, 279 edges)

| Feature | Count | Implication |
|---------|-------|-------------|
| Edge operator `-->` | 279 of 279 edges | **Only one edge style.** No dotted (`-.->`), thick (`==>`), bidirectional, or `--o`/`--x` anywhere in scope. One arrow style suffices. |
| Blocks with `subgraph` | 11 blocks, 25 subgraphs | Containers are table stakes |
| Subgraph nesting depth | max 1, 0 nested blocks | Nesting is allowed by the grammar but never exercised. Single-level layout must be perfect; deep nesting can be best-effort. |
| Subgraph form | 14 `id["title"]`, 11 `id[title]` | Always `id + title`. Importer must handle both. |
| Edges whose endpoint is a subgraph id | 0 confirmed. One false positive was `P52 --> P53 --> P54`, where `P52..P54` also name subgraphs, so confirm in the importer test. | Edges-to-container is a D2 feature but not needed to migrate; keep it cheap or defer. |
| Edge labels | 26 total: 10 `-- "x" -->`, 10 `-- x -->` (unquoted), 6 `-->|x|` | Labels are common. Three Mermaid spellings, one output form. The 10 unquoted ones are exactly what the existing repair tool targets. |
| Node shape: rectangle `A[..]` | 324 (244 quoted, 80 unquoted) | Default shape |
| Node shape: diamond `A{..}` | 9 nodes in 6 blocks | Decision shape is required |
| Node shape: rounded `A(..)` | 6 nodes | Needed for fidelity, cheap |
| Node shape: cylinder `A[(..)]` | 1 | Needed (database) |
| Circle, stadium, subroutine, hexagon, `@{shape}` | 0 | Do not build |
| Fan-out `A & B --> C` | 3 blocks (4 lines) | **Importer expands to separate edges.** The DSL needs no sugar. |
| Chained edges `A --> B --> C` | 5 blocks (8 lines) | Importer expands. DSL chain syntax is optional, defer. |
| `<br>` in labels | 1 block | Map to `\n` in quoted strings, not HTML |
| Inner `direction` inside a subgraph | 1 block | Rare. Parse it, apply or warn; do not build per-container direction in layout v1. |
| `style` / `classDef` / `class` / `linkStyle` / `:::` | **0** | **No styling DSL needed.** Theming is CSS-only. |
| `click` / links / tooltips | 0 | Do not build |
| `%%` comments, HTML entities | 0 | Importer may ignore |
| Unquoted node labels (`A[text]`) | 11 of 37 blocks | Importer must accept lenient input, then emit always-quoted. |
| Unquoted labels containing punctuation (`:` `(` `/` `.`) | 6 blocks | These are the exact fragility cases from the advisory. Importer must not mis-split them. |

(Counts exclude `archive/`. For reference, archive is 27 flowchart + 2 graph, 26 of 29 using unquoted labels, 15 subgraphs, still 0 style or click.)

### 0.4 Features actually used, other kinds

| Kind | Features present | Features absent (do not build) |
|------|------------------|--------------------------------|
| `sequenceDiagram` (2 blocks, 14 and 18 lines) | `participant X as Alias` (both blocks, 7 and 8 participants); `->>` sync message (both); `-->>` dashed return (1 block); `Note over A,B: text` (1 block); labels with parens, quotes, `#`, `@`, `>` (1 block, the fragile case) | `alt`/`opt`/`loop`/`par`/`critical`, `activate`/`deactivate`, `autonumber`, `box`, `rect`, `actor` keyword |
| `stateDiagram-v2` (1 block, 7 lines) | `[*]` start and end, labelled transitions, a **self-loop** (`Leased --> Leased: lease-renew`) | composite states, `<<choice>>`/`<<fork>>`, notes, concurrency |
| `gantt` (1 block, 18 lines) | `title`, `dateFormat`, 4 `section`s, 11 tasks with ids, start dates, durations | n/a |

### 0.5 Corpus-derived acceptance test

The 40 non-Gantt blocks in `docs/src/` (excluding `archive/`) are the golden fixture set for the whole milestone. Acceptance is: import, validate with 0 errors, emit Mermaid that the existing `validate_and_repair_mermaid` accepts unchanged, and emit SVG deterministically (byte-identical on two runs). The 29 archive blocks are an optional stress corpus (mostly unquoted labels) for the importer's leniency, never a migration target.

---

## 1. How these DSLs work (design reference)

### 1.1 D2 (the closest design ancestor)

Source: d2lang.com tour (sequence page fetched 2026-10-16, HIGH), remaining items from training knowledge (MEDIUM).

- **Shapes and keys.** `id: "Label"` declares an object. `id.shape: cylinder` or a block `id: { shape: cylinder }` sets attributes. A fixed shape vocabulary (rectangle, cylinder, diamond, circle, person, ...).
- **Containers by nesting.** `a: { b; c }` or dotted paths `a.b`. Dotted keys can create nested objects without braces, and `_` references the parent. Cross-container edges use dotted paths (`client -> backend.gateway`). This is exactly the dotted-ref design VoxDiagram already specifies.
- **Connections.** `a -> b: label`, `<-`, `<->`, `--`. Chains (`a -> b -> c`) are allowed. Labels are plain text after the colon.
- **Sequence diagrams.** `shape: sequence_diagram` on a container. Actors are optional to declare (declare them to fix left-to-right order). Messages are ordinary connections and **definition order is visual order** (an explicit exception to the rest of D2). Notes are a nested object with no connections. Groups (fragments) are containers inside the diagram. Spans (activations) are nested objects on an actor. All names share one scope inside a sequence diagram.
- **Markdown/text blocks.** `|md ... |` (and `|` + language fences) embed multi-line text or code with no quoting problems. This is D2's answer to label-escaping fragility.
- **Not needed here:** `vars`/substitutions, globs, `classes`, layers/scenarios/steps (multi-board and animation), `sql_table` and `class` shapes, icons and images, tooltips and links, `near`/`grid` layout hints, selectable layout engines (dagre, ELK, TALA).

What VoxDiagram should take from D2: the orthogonal `key: value` attribute model, brace containers, dotted refs, definition-order sequences. What it should leave behind: the breadth.

### 1.2 Mermaid semantics for the kinds in the corpus

- **flowchart/graph:** nodes created on first mention; shape given by bracket syntax at first (or any) mention; edge `A --> B`; label `-- t -->` or `-->|t|`; `subgraph id [title] ... end`; direction `TB|TD|BT|LR|RL`. Quirks the importer must model: an id may be introduced inside an edge line, a node may be redeclared with a different label later (last wins), subgraph ids and node ids share one namespace, fan-out with `&`, chains.
- **sequenceDiagram:** participants appear in first-mention order unless declared; `A->>B: text`, `-->>` is a dashed return; `Note over A,B: text`. Text after the first `:` is the message, so colons are legal but `;` and `#` need escaping.
- **stateDiagram-v2:** `[*]` is both start (as source) and end (as target); `A --> B: label`; state ids are bare words.

### 1.3 Mermaid accessibility (fetched, HIGH)

Mermaid sets `aria-roledescription` on the SVG from the diagram type, supports `accTitle:` (one line) and `accDescr:` (one line or `{ }` block), and emits `<title>` and `<desc>` with `aria-labelledby`/`aria-describedby`. Works for all diagram types. The corpus uses none of this, so today's rendered diagrams have no authored text alternative. VoxDiagram can do better by default (see 4).

---

## 2. Table Stakes

Features without which the 40 corpus diagrams cannot migrate, or the tool fails its stated purpose (LLM-proof authoring). Missing any = milestone incomplete.

### 2.1 Language and IR

| Feature | Why Expected | Complexity | Notes / dependencies |
|---------|--------------|------------|----------------------|
| `graph` diagram kind with `direction` (LR, TB; accept RL, BT) | 90% of corpus; 17 LR, 19 TD/TB | Med | Parse `direction` per diagram. Importer maps `TD` to `TB`. |
| `node id: "Label"` with always-quoted strings and escapes (`\"`, `\\`, `\n`) | Core value prop. 6 corpus blocks have punctuation labels. | Low | Lexer owns string handling, so no label can corrupt the structure. This replaces `<br>`. |
| `container id: "Title" { ... }` with brace nesting | 11 blocks, 25 subgraphs | Med | Grammar supports arbitrary depth; layout must be correct for depth 1 and merely valid at depth 2+. |
| Edge `a -> b` and `a -> b: "label"` | 279 edges, 26 labelled | Low | One arrow style only (corpus is 100% `-->`). |
| Dotted refs to nested nodes (`backend.gateway`) | In the agreed DSL; needed to point into containers | Med | Resolution pass with scope rules. Decide: a bare `gateway` resolves only if unambiguous, to avoid silent wrong links. |
| Node `shape:` attribute: `rect` (default), `rounded`, `diamond`, `cylinder` | Corpus uses all four (324 / 6 / 9 / 1) | Low | Four shapes cover 100% of in-scope usage. Add `start`/`end` pseudo-shapes for state (2.3). |
| Self-loop edges (`a -> a: "label"`) | Required by the one state diagram | Med | Layout special case. Easy to forget, and a classic layout-engine hole. |
| Comments (`//` line) | Authors and LLMs expect them | Low | Preserve in spans, drop from IR. |
| Typed AST + resolved IR with stable node/edge identity | Foundation for every emitter and the importer | Med | Per milestone scope. IR must be serde-serializable so the MCP tool can return it. |
| Span-precise diagnostics with stable codes | Replaces regex "line N" errors; the headline differentiator over Mermaid | Med | See 5. Reuse the workspace's existing diagnostic crate if there is one (`miette = 7.6` is already a workspace dep) instead of inventing a renderer. |
| Error recovery: report many diagnostics per pass | LLM repair loops are cheapest when one call returns all problems | Med | Parser must resync at `}` and newline. Without this the tool is no better than Mermaid's first-error-stops behavior. |
| Duplicate id, undeclared reference, empty label, unclosed `{`, edge to a missing node as hard errors | The "compiler diagnostics" promise | Low | Each one gets a code and a suggested fix (see 5). |
| Canonical formatter (`normalize`) | Deterministic output; makes LLM and human variants converge | Low-Med | Also gives the MCP tool a `normalized` field. Cheap given a typed AST. |

### 2.2 Sequence diagrams

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `sequence` kind: `actor id: "Label"`, messages `a -> b: "text"` in definition order | 2 blocks (D2's definition-order rule matches Mermaid) | Med | No graph layout needed: lanes plus rows, so it is the cheaper of the two kinds. |
| Dashed return message (`a --> b` or `style: dashed`) | `-->>` in one block | Low | Pick one spelling and document it. |
| `note over a, b: "text"` | 1 block | Low-Med | Spans 1 or 2 lanes. |
| Variable-width text handling in headers and messages | Corpus has 80+ character messages (the Term/Bus/Chat block) | Med | Needs wrapping or width-aware lane spacing, otherwise the SVG is illegible. |

Explicitly not required: fragments (`alt`/`loop`/`par`), activations, autonumber. 0 uses.

### 2.3 State diagram (folded into `graph`)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `start` and `end` pseudo-nodes (render as dot and ringed dot), labelled transitions, self-loop | The single state diagram needs exactly this | Low | **Recommendation: do not add a third diagram kind.** One block does not justify a state-machine kind; model it as `graph` plus two shapes. Mermaid emitter writes `stateDiagram-v2` or a flowchart with circle shapes; either is valid. Confirm which renders best on GitHub in the emitter phase. |

### 2.4 Emitters

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Sanitized Mermaid emitter (every label quoted, ids made safe, reserved words like `end` and `graph` escaped) | Whole point for GitHub, README and PRs | Med | Output must pass the existing `validate_and_repair_mermaid` with **no repairs** (`was_repaired == false`): a free regression test against the existing code. Watch the known Mermaid traps: a node id of `end`, ids starting with `o` or `x` after `---`, and `<`/`>` in labels (use entity codes such as `#lt;`). |
| Deterministic SVG emitter | Docs build, CI | **High** | See 2.5. This is the largest single cost in the milestone. |
| Same input produces byte-identical output across runs and machines | Required for CI drift checks and the repo's SSOT-drift culture | Med | No timestamps, no random ids, no HashMap iteration order, fixed float precision (round to 0.1 or integers). |

### 2.5 Layout (hidden table stake)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Layered (Sugiyama-style) auto-layout with LR and TB, container bounding boxes, label-aware node sizes, orthogonal or spline-lite edge routing, edge-label placement | There is no browser to lay it out, so the crate must do it | **High** | Corpus ceiling is ~25 nodes and 21 edges per block, 1-level containers. Hand-rolling a layered layout is the milestone's biggest risk; candidate crates exist (flag for the STACK researcher, do not decide here). Text must be measured without a browser, so use a font-metrics approximation with generous padding. |
| Cycle handling (back edges) | The state diagram and several flowcharts have loops | Med | Standard: reverse back edges during ranking, route around. |

### 2.6 Mermaid to VoxDiagram importer

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Real tokenizer-based parser for the subset in 0.3 and 0.4 (not regexes) | Regexes already failed here (the advisory's point) | Med-High | Handles: header + direction, `subgraph id[title]`/`end`, node shapes (rect, rounded, diamond, cylinder), unquoted and quoted labels, three edge-label spellings, `&` fan-out and chains, `<br>`, `participant ... as`, `->>`/`-->>`, `Note over`, `[*]`. |
| **Strict mode: any unsupported construct is a hard error, never silently dropped** | Migration correctness. A dropped edge is a wrong diagram in the docs. | Low | Importer returns `unsupported` diagnostics with the Mermaid line number. The 40 corpus blocks must import with zero. |
| Deterministic id mapping that preserves Mermaid ids | Reviewable diffs; stable dotted refs | Low | Keep original ids (`P52`, `VFS1`); sanitize only when illegal in VoxDiagram. |
| Migration command that rewrites `docs/src/**` blocks in place, with `--check` | The 41-block migration is mechanical, so a script beats hand edits | Med | Per the repo's glue rule, this is `vox` CLI or a `.vox` script, not a new `.py`/`.sh`. Dry run first; edits land as one reviewable commit. |
| Importer round-trip check: import, emit Mermaid, import again, graphs equal | Cheap, strong correctness property | Low | Test only, not a user feature. |

### 2.7 `vox_validate_diagram` MCP tool: see section 5

### 2.8 Docs-build integration: see section 4

### 2.9 Docs gate

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Gate fails on a new ```` ```mermaid ```` block in `docs/src/` outside `archive/` | In the milestone scope | Low | Extend the existing fence extractor in `vox-doc-pipeline` (`extract_mermaid_blocks`). Message names the replacement (` ```vox-diagram `) and the importer command. |
| Gate validates every ` ```vox-diagram ` block (parse and resolve, 0 errors) | A broken new-format block must fail the same lint | Low-Med | Hook into the same pre-push and CI path as the Mermaid lint. |
| Info string must not collide with the `vox` doctest runner | `vox-doc-pipeline` compiles ```` ```vox ```` blocks as doctests | Low | Verify the runner matches the info string exactly (`vox`), not by prefix, or `vox-diagram` blocks will be fed to the Vox compiler. High-impact, one-line check. |
| Exemption list for non-migrated kinds (the one Gantt) | Gate cannot both ban Mermaid and keep the Gantt | Low | Exemptions are USER-AUTHORIZED-ONLY in this repo's discipline; propose the entry, do not self-approve. |

---

## 3. Differentiators

Not required to migrate, but they justify the project over "keep Mermaid and tighten the lint".

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Machine-applicable fix suggestions on diagnostics (`edits: [{span, replacement}]`) | An agent applies a fix with no LLM round-trip, which removes the 1-3 s repair latency the advisory complains about | Med | Cheap ones: quote an unquoted label, close a brace, "did you mean `backend.gateway`" via edit distance, rename duplicate ids. |
| Auto-generated text alternative from the IR | Real a11y with no author effort: "Flowchart, left to right, 6 nodes, 5 connections. Orchestrator Host contains ..." | Low-Med | Walk the IR into `<desc>`. Authors may override with `title:`/`desc:` attributes. Better than Mermaid, which needs hand-written `accDescr`. |
| `from_mermaid` input on the validate tool: agent pastes Mermaid, receives converted VoxDiagram plus loss report | Gives agents already in the habit of writing Mermaid a one-call path in, reusing the importer | Low (given 2.6) | The tool returns converted source, not a rewrite of the user's file. |
| Theme-by-CSS-variables SVG (see 4) | Dark/light parity with `data-theme`, no regeneration per theme | Med | One SVG serves both themes. |
| Layout stability: small source edits yield small SVG diffs | Reviewable docs PRs. Mermaid re-layouts the world on any change. | Med | Deterministic ranking with stable tie-breaks (source order). Pairs with the byte-identical guarantee. |
| `normalize`/format as part of the tool and CLI | Canonical diagrams in docs, no style drift | Low | Falls out of the typed AST. |
| Stats in tool output (node/edge/container counts, depth, max fan-out, "this will render as a hairball" warning above a threshold) | Gives agents a signal to split a diagram. Targets the advisory's section 2.4 hairball problem. | Low | Warn only. |
| Dependency-light, no JS and no Go at build time | Matches the repo's build-toolchain invariant. Also removes the `cdn.jsdelivr.net` mermaid.js fetch in `docs-astro/src/components/PageSidebar.astro`. | n/a (consequence) | The site currently renders Mermaid client-side from a CDN. Pre-rendered SVG is a genuine robustness and privacy gain. |

---

## 4. Docs-build integration requirements

Current state (verified): `docs-astro/src/components/PageSidebar.astro` dynamically imports `mermaid@11` from jsdelivr at runtime and replaces `pre[data-language="mermaid"]` blocks; `reader-layout.css` has a `.vox-mermaid-diagram` container (line ~436) and themes via `:root[data-theme='light']` plus Starlight `--sl-color-*` variables (dark is the default `:root`).

| Requirement | Detail | Complexity |
|-------------|--------|------------|
| **Inline SVG, not `<img>`** | CSS custom properties only cascade into inline SVG. An `<img src=...svg>` cannot see the page's variables or `data-theme`, so theming and text selection would both break. | Low (decision) |
| **Colors via CSS variables with fallbacks** | Emit classes (`.vd-node`, `.vd-edge`, `.vd-label`, `.vd-container`) and style them with `fill: var(--vd-node-fill, var(--sl-color-gray-6))` and so on, chained to Starlight's `--sl-color-*` tokens that already switch with `data-theme`. No hex literals in the SVG body. Fallbacks make the SVG still legible when copied to GitHub or opened standalone. | Med |
| **Standalone fallback stylesheet** | A `<style>` block in the SVG with default light-theme values, plus `@media (prefers-color-scheme: dark)`, so the same SVG is valid outside the site (PR bodies, exported files). | Low |
| **Text as `<text>`, never paths** | Selectable, searchable (Pagefind indexes it), translatable, and screen-reader readable. `font-family: inherit` with a system fallback. | Low |
| **`<title>` and `<desc>` with `role="img"` and `aria-labelledby`** | Matches the Mermaid a11y model (fetched). Unique `id`s required: the page may hold many inline SVGs, so derive ids from a content hash (also deterministic), never a counter that changes when another diagram is inserted above. | Low |
| **Unique marker/arrowhead/clip-path ids per SVG** | Same duplicate-id hazard for `<marker>`. Prefix every `id`/`url(#...)` with the diagram hash. | Low |
| **`viewBox` + `width:100%; height:auto; max-width` (no fixed pixel size)** | Responsive in the reader layout, which has a collapsing sidebar. | Low |
| **Non-text contrast** | Strokes and arrowheads at least 3:1 against the background (WCAG 1.4.11) in both themes; check the token choices, not just the structure. Add `forced-colors` handling (system colors) so Windows high-contrast works. | Low-Med |
| **Container and node semantics in the DOM** | `<g class="vd-node" data-id="...">` and `<g role="group" aria-label="...">` for containers, so tests and assistive tech can inspect structure. Keep it shallow; do not add per-node focus stops (static image, no keyboard interaction). | Low |
| **Where rendering happens** | Either a build step in `vox-doc-pipeline` that replaces fenced blocks with `<figure class="vox-diagram">` + inline SVG before Astro, or a remark plugin in `docs-astro/src/plugins/` that calls the `vox` binary. This is an architecture decision for the ARCHITECTURE researcher; flag it as a roadmap dependency. Either way the build needs only the Rust binary, not Node-side Mermaid. | Med (decision) |
| **Syntax highlighting for the source** | The fence ```` ```vox-diagram ```` needs a registered language (see `docs-astro/src/plugins/vox-grammar.mjs`) or Expressive Code will warn on an unknown lang. Alias it to a plain or `vox` grammar. | Low |
| **Show source option** | A `<details>` with the DSL under the figure helps readers and gives GitHub viewers (who see only the raw fence) something readable. | Low |
| **Visual verification** | The GUI visual-verification invariant is about `vox-gui`, not docs, so it does not apply directly. Still, snapshot-test the SVG text in Rust (insta-style golden files for the 40 corpus diagrams) and optionally one Playwright screenshot in both themes for the docs site. | Med |

GitHub rendering caveat (design consequence, not a bug): GitHub does not render ```` ```vox-diagram ````. A docs page viewed on github.com shows the DSL as a code block. That is acceptable for `docs/src/` (the site is the reading surface). For README and PR bodies, the workflow is: author in VoxDiagram, **emit Mermaid** with the tool or CLI, paste that. Do not try to keep a generated Mermaid block next to every source block (see anti-features).

---

## 5. Agent-facing `vox_validate_diagram` tool

Design for LLM consumption. Mirror the existing `vox_validate_mermaid` contract (`ToolResult<...>` envelope, `code` or `markdown` input, `valid`, `diagnostics[]`, `count`, repaired text) so agents and orchestrator code reuse one mental model, then extend.

### 5.1 Input

| Field | Purpose |
|-------|---------|
| `code` | A bare VoxDiagram source |
| `markdown` | A whole document; validates every ```` ```vox-diagram ```` block and offsets all spans to the file (mirrors `validate_and_repair_markdown_mermaid`) |
| `from_mermaid: bool` | Treat `code`/`markdown` as Mermaid, import, validate the result, return converted source plus a loss report |
| `emit: ["mermaid", "svg"]` (optional) | Return emitter output so an agent can verify what the reader will see; `svg` is opt-in because it is large |
| `max_diagnostics` (default ~25) | Cap output tokens |

### 5.2 Output

| Field | Detail |
|-------|--------|
| `valid` | True only when there are zero errors (warnings do not fail) |
| `diagram_kind` | `graph` or `sequence` |
| `diagnostics[]` | Each: `code` (stable, e.g. `VD0101`), `severity` (`error`/`warning`/`info`), `message` (one sentence, imperative), `span` (`line`, `col`, `end_line`, `end_col`, plus byte offsets), optional `help` (one line of remediation), optional `fixes[]` (`{description, edits:[{span, replacement}]}`), optional `related[]` (other spans, e.g. the first declaration of a duplicate id) |
| `normalized_source` | Canonical formatted source, so the agent can overwrite instead of hand-patching |
| `stats` | Nodes, edges, containers, max depth |
| `emitted` | Present only if requested |
| `count` | Same convention as the Mermaid tool |

### 5.3 Behaviors that matter for agents

| Behavior | Why | Complexity |
|----------|-----|------------|
| All errors in one response (parser recovery) | Each retry costs a turn; the advisory's core complaint | Med |
| Fixes are deterministic text edits, never LLM calls | The old design burned tokens and 1-3 s on repair prompts | Med |
| Stable machine codes, never renumbered | Agents and a future prompt can key on them | Low |
| Messages quote the offending token and the expected token ("expected a quoted string after `:`, found `Process`") | LLMs fix what they can see | Low |
| Did-you-mean for unknown ids and shape names (edit distance) | Most real errors are typos or dotted-path slips | Low |
| `help` text shows a corrected one-liner | Few-shot at the point of failure | Low |
| Warnings for risky-but-valid input: unused node, orphan container, more than ~25 nodes, empty container, edge-label length over a limit | Quality signal without blocking | Low |
| Deterministic ordering of diagnostics (by span, then code) | Snapshot-testable | Low |
| No file writes. The tool is read-only; it returns text. | Matches existing tool; keeps permissions trivial | Low |
| Tool registration through the existing MCP surface and generated reference (`mcp-tools.generated.md`) | Repo rule: regenerate via `vox ci command-sync`, never hand-edit | Low |

Cross-check: `vox_validate_mermaid` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs` returns `MermaidDiagnostic { line, code, message }` only (line-level, no column, no fix). VoxDiagram's tool is strictly richer; keep the field names `valid`, `diagnostics`, `count`, `code`, `message` compatible so the existing turn-interception code (`observe_and_repair_turn_mermaid`) can be extended to also recognize ```` ```vox-diagram ```` fences with minimal change.

---

## 6. Anti-Features

Explicitly do NOT build in v1.6.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| ER, class, pie, journey, gitGraph, mindmap, timeline, C4, block kinds | 0 corpus uses; each is a separate layout and grammar (this is exactly Mermaid's "15 parsers" disease) | Leave the IR open for new kinds; add one only when a real doc needs it. |
| Gantt kind | 1 use, requires date math and a whole timeline layout | Convert the single block to a Markdown table (it is a dated task list), or keep it as the one exempt Mermaid block. |
| Styling language (`style`, `classDef`, `class`, `:::`, `linkStyle`) | 0 uses in 41 blocks. Invites per-diagram color drift and breaks theming. | Semantic attributes only (`shape`, `kind`); all color comes from CSS tokens. |
| Per-diagram themes or color overrides in the DSL | Same as above, and defeats dark/light parity | Two documented CSS variable sets; the docs theme owns them. |
| Edge styles beyond `->` (dotted, thick, bidirectional, `--o`, `--x`) | 0 uses | Add dashed only for the sequence return message. Add others when a doc needs them. |
| Node shapes beyond rect/rounded/diamond/cylinder/start/end | 0 uses of circle, stadium, subroutine, hexagon, `@{shape}` | Same. |
| Fan-out sugar (`a & b -> c`) and implicit-declaration-by-edge | The corpus uses `&` in 3 blocks, but sugar is where Mermaid ambiguity lives | Importer expands to explicit edges. Require `node` declarations (or an explicit declare-on-first-use rule that the resolver diagnoses). |
| Unquoted labels anywhere | The entire premise | Strings are always quoted. The importer quotes; the parser rejects. |
| HTML in labels (`<br>`, `<b>`, links) | Injection surface in inline SVG and a source of GitHub sanitizer differences | `\n` in strings only. Strip or escape on Mermaid emit. |
| D2-style rich text blocks (`\|md ... \|`) | Always-quoted strings with escapes already solve the delimiter problem; markdown rendering inside SVG is heavy (`foreignObject`) and hurts a11y and portability | Defer. If long text is common later, add multi-line string literals. |
| D2 layers, scenarios, animation, `near`/`grid` position hints, icons, images, tooltips, click links | Not used; layout hints defeat determinism and LLM-friendliness | Auto-layout only. |
| Multiple layout engines or user-selectable layout | Config surface, nondeterminism risk | One built-in layered layout. |
| Interactive or client-side rendering (JS in the page) | Reintroduces fail-stop rendering and the CDN dependency | Build-time SVG, zero page JS. |
| A pre-rendered `<img>` or external `.svg` file per diagram | Breaks theming and a11y (see 4) | Inline SVG in a `<figure>`. |
| Pairing every source block with a generated Mermaid fallback block in the same doc | Doubles the content, guarantees drift, and trips the new "no new mermaid" gate | Emit Mermaid on demand for README/PR use only. |
| Full-fidelity Mermaid to VoxDiagram to Mermaid round-trip for all Mermaid features | Unbounded scope | Import the supported subset strictly, error on the rest. |
| LLM-based repair inside the validate tool | Defeats the deterministic point of the tool | Machine-applicable fixes only; the orchestrator may still call an LLM using the tool's diagnostics. |
| `vox-compiler` `diagram` keyword, LSP, Axis GUI rendering | Out of scope by the user's decision | The `vox-diagram` crate keeps a pure API so a later milestone can wire it in. |
| Migrating `docs/src/archive/` or the 10 `docs/superpowers/` blocks | Archive is tombstoned by AGENTS.md; superpowers is out of stated scope | Gate scope: `docs/src/**` excluding `archive/` only. |
| Removing the existing Mermaid lint and `vox_validate_mermaid` | Still needed for archive, the exempt block, and GitHub-bound output | Keep both. Reuse them as the emitter's regression oracle. |

---

## 7. Feature Dependencies

```
Lexer + Parser (always-quoted strings, braces, ->, dotted refs, recovery)
   -> Typed AST + span table
      -> Resolver (scopes, dotted refs, duplicate/undeclared diagnostics)
         -> IR (graph | sequence)
            -> Formatter (normalize)
            -> Mermaid emitter ------------------> validate_and_repair_mermaid (existing; oracle: must need 0 repairs)
            -> Layout (layered; container boxes; self-loops; label sizing)
               -> SVG emitter (CSS vars, <title>/<desc>, text-as-text, hashed ids)
                  -> Docs-build hook (pipeline or remark plugin) -> figure + inline SVG
IR -> Diagnostics w/ fixes -> vox_validate_diagram (MCP) -> extend mermaid_repair turn interception (optional)
Mermaid tokenizer (strict) -> Importer -> IR
   -> Migration command (rewrite docs/src, --check) -> 40 golden fixtures
   -> `from_mermaid` mode in the MCP tool
Docs gate (new ```mermaid fails; ```vox-diagram validates; doctest runner must not match it)
   depends on: extract_mermaid_blocks (existing) + parser (new) + migration complete
```

Hard ordering constraints:
- The gate must land **after** the migration (or it fails on its own repo). The gate needs an exemption mechanism for the Gantt first.
- Layout must exist before SVG can be called done, but parser + IR + Mermaid emitter + importer + MCP tool do not need layout. They can ship and be verified first.
- Sequence diagrams are independent of the graph layout and can be built in parallel.

Dependencies on existing code (read, not modified by this research):
- `crates/vox-doc-pipeline/src/pipeline/mermaid.rs`: `extract_mermaid_blocks`, `validate_and_repair_mermaid`, `SUPPORTED_DIAGRAM_HEADERS`, `MermaidDiagnostic`/`MermaidValidationResult` shapes. Reuse for the gate, as the emitter oracle, and for field-name compatibility.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs` and `input_schemas.rs`: tool registration pattern and `ToolResult` envelope for `vox_validate_diagram`.
- `docs-astro/src/components/PageSidebar.astro`, `src/styles/reader-layout.css`, `src/plugins/*.mjs`: current client-side Mermaid path and theme tokens the SVG must target.
- Workspace `miette 7.6` (already a dependency): option for human-readable rendering of diagnostics.
- Repo process rules: new crate needs a layer in `docs/src/architecture/layers.toml` and a row in `where-things-live.md`; any new workspace crate edge is CI-gated (`vox ci crate-edges`), so keep `vox-diagram` a low-layer crate with few dependencies (serde at most); new MCP tool requires regenerating `mcp-tools.generated.md` via `vox ci command-sync`; new docs require frontmatter.

---

## 8. MVP Recommendation

Prioritize (in dependency order; each is independently testable against the corpus):
1. **Parser, AST, resolver, diagnostics with recovery** for the `graph` kind (nodes, containers, edges, labels, 4 shapes + start/end, self-loop, direction).
2. **Mermaid emitter** plus **strict importer** for `graph`: first measurable win. All 37 flowchart/graph/state blocks import, re-emit as Mermaid that the existing validator accepts untouched.
3. **`vox_validate_diagram`** with spans, codes, fixes, `normalized_source`, `from_mermaid`.
4. **Layout + SVG emitter** (the high-cost item) with CSS variables and a11y.
5. **Sequence kind** (parser, importer, Mermaid emit, lane layout). 2 blocks, so it can slip to the end of the milestone without stranding the migration; it is cheap because no graph layout is needed.
6. **Docs integration, migration of 41 blocks (40 convertible), gate, Gantt exemption.**

Pick one differentiator to protect even under schedule pressure: **machine-applicable fixes plus all-errors-at-once**. They are what make this better than "Mermaid plus a stricter lint". The auto-generated `<desc>` is second (cheap, and real accessibility).

Defer: chained edge syntax, `|md` blocks, edges directly to containers if the layout makes it hard (0 confirmed corpus uses), per-container direction, nesting deeper than 1 level beyond "valid and renders legibly", Playwright theme screenshots (use Rust golden files first).

---

## 9. Risks the roadmap should watch (feature-level)

- **Layout quality is the milestone risk**, not the parser. Budget it as its own phase with a go/no-go on the 5 largest corpus diagrams (the 31-line `axis-harness-river-design-2026.md` flowchart with 4 subgraphs is the stress case). If the in-house layout cannot meet legibility, the fallback is a smaller layout scope or an off-the-shelf layered-layout crate, not hand-placed coordinates.
- **Text measurement without a browser** produces clipped or oversized boxes. Use conservative metrics and wrap long labels (corpus labels reach 80 characters).
- **Count correction**: roadmap and requirements should say 41 blocks / 26 files (40 convertible), not ~71 / ~55.
- **Doctest runner collision** on the `vox-diagram` info string (see 2.9).
- **Unquoted-label leniency in the importer**: 11 of 37 flowcharts rely on it; a too-strict importer will reject real corpus files, and a too-lenient one will mis-split labels containing `:` or `(`.

---

## Sources

- Repo corpus (measured 2026-10-16 with a fence-aware script over `docs/src/`, cross-checked with `grep`): HIGH. Key files: `docs/src/architecture/*.md`, `docs/src/reference/populi.md`, `docs/src/index.mdx`.
- `/Users/brbrainerd/dev/vox/docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md`: HIGH for repo intent (grammar sketch, 3-tier stack, migration phases). Its LLM error-rate numbers (18-35% vs 4-8%) are unsourced in the advisory; treat as LOW and do not repeat in requirements.
- `/Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/mermaid.rs`, `/Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs`: HIGH (read directly).
- `/Users/brbrainerd/dev/vox/docs-astro/src/components/PageSidebar.astro`, `/Users/brbrainerd/dev/vox/docs-astro/src/styles/reader-layout.css`: HIGH (current rendering and theming).
- D2 sequence diagrams: https://d2lang.com/tour/sequence-diagrams/ (fetched): HIGH. Other D2 features (containers, `_`, chains, `|md`, layers, vars): MEDIUM, training knowledge.
- Mermaid accessibility: https://mermaid.js.org/config/accessibility.html (fetched): HIGH.
- WCAG 1.4.11 non-text contrast (3:1) and CSS custom properties not crossing `<img>` SVG boundaries: MEDIUM, standard web-platform knowledge, not re-fetched.
