# Stack Research: VoxDiagram (`vox-diagram` crate)

**Domain:** Native text-diagram DSL -> typed IR -> deterministic SVG + sanitized Mermaid, in the Vox Rust workspace
**Researched:** 2026-10-16
**Confidence:** MEDIUM-HIGH (layout engine verified empirically; the one MEDIUM is single-maintainer risk on the chosen layout crate, mitigated by a swap seam)

Scope: only what the NEW features need. Existing workspace deps are reused wherever they cover the need. Versions below were checked against crates.io / npm on 2026-10-16.

---

## Headline decisions

| Question | Decision | One-line reason |
|----------|----------|-----------------|
| Layout engine | **`dagre` 0.1.1 (pure-Rust dagre.js port), behind an internal `layout` seam** | Only maintained pure-Rust option with compound clusters AND edge labels AND variable node sizes; verified here; zero C, one tiny dep tree |
| Layout fallback | `dugong` 0.9.0 (second dagre port) -> `elkrs` 0.1.1 (after license review) -> elkjs in Node | Same adapter shape for the first; ELK only if dagre quality fails the spike |
| Parser | **Hand-written lexer + recursive descent, zero parser deps** | Grammar is ~8 constructs; span-precise diagnostics and recovery are easier by hand than via a combinator generic-type maze |
| SVG writing | **Plain `String` builder with a fixed attribute order** | The `svg` crate stores attributes in a `HashMap` (verified in 0.18.0 source) -> non-deterministic attribute order -> breaks golden SVGs |
| Text measurement | **Static per-character advance table + `unicode-width` for wide chars** | No font files, no font crate, bit-deterministic, matches what mmdr's `--fastText` does; browser font differs anyway |
| Mermaid parser for importer | **Hand-written flowchart/graph importer; do NOT depend on `merman-core`** | Live corpus is 36/40 flowcharts; a one-shot migration does not justify ~20 new transitive crates |
| New npm deps | **None** | Pre-render happens in Rust (`vox-doc-pipeline`, already run before `pnpm build` in `docs-deploy.yml`); Node only inlines the result |

---

## 1. Layout engine (the deferred decision)

### What the layout engine MUST do (derived from the DSL)

1. Nested `container { ... }` (compound/cluster graphs, nesting depth >= 2) with a computed container box.
2. Variable node sizes (box sized from label text, set by us, not by the engine).
3. Edge labels that occupy space and get a position (`a -> b: "label"`).
4. Direction (TB/LR/BT/RL).
5. Bit-identical output across runs and across macOS/Linux/Windows (golden SVG snapshots in CI; the docs gate diffs them).
6. Toolchain invariant: no cmake/nasm/Go/perl/libclang; `cargo-deny` licence allowlist has Apache-2.0/MIT/MPL-2.0 but NOT EPL-2.0 or GPL.

### Candidate matrix (all facts re-checked 2026-10-16)

| Candidate | Version / last release | Licence | Compound/nested clusters | Edge labels | Variable node size | Determinism | Verdict |
|-----------|-----------------------|---------|--------------------------|-------------|--------------------|-------------|---------|
| **`dagre`** (kookyleo; now at Actrium/supramark `crates/dagre`) | 0.1.1, 2026-05-06 (2 releases; 7.3k dl) | Apache-2.0 | YES (`set_parent`, border segments, nesting graph) | YES (`EdgeLabel.width/height`, label x/y out) | YES | Verified: 40/40 separate-process runs byte-identical on a 2-level-nested graph with labels. Arithmetic is add/mul/sqrt only (trig only in an ellipse helper we never call) so cross-platform f64 is stable | **PICK** |
| `dugong` (+`dugong-graphlib`), part of `merman` | 0.9.0, 2026-10-12 (24 releases; 132k dl; 599-star parent repo, used by Zed) | MIT OR Apache-2.0 | YES | YES | YES | "Favors deterministic iteration"; uses `rustc-hash`/`hashbrown` (fixed-seed). Has `layout_controlled` + `WorkControl` budget + `LayoutError` (no panics) | **Fallback #1.** Author states the low-level API is "not a separately supported product surface" and it is index/`serde_json::Value`-flavoured; tracks older dagre.js 2.0.2 |
| `rust-sugiyama` | 0.4.0, 2025-09 (47k dl) | MIT | **NO** (README + source: no clusters) | **NO** | Yes (closure) | Uses std `HashMap` throughout (random seed); open issues incl. infinite loop in `from_graph` | Reject |
| `layout-rs` | 0.1.3, 2025-04 (807k dl) | MIT | **NO**: "Support for Subgraphs/Clusters (i.e. Compound Graphs)" is an open feature request; parser flattens subgraphs | Yes | Yes | `HashMap`-based builder | Reject (it is a DOT renderer, not a compound layout lib) |
| `dagre-rs` (TangleGuard) | 0.1.0, 2025-10 | **non-standard `license-file`** (cargo-deny would reject) | No | No | - | - | Reject |
| `ascii-dag` / `graphlay` / `bra0-layout` | young or text-oriented | various | partial | partial | n/a | n/a | Reject: unproven, <=110k dl, no proof of compound-with-labels |
| `elkrs` | 0.1.1, 2026-06-17 (811 dl, 1 star, 1 author) | declares Apache-2.0 | YES (`INCLUDE_CHILDREN`, hierarchical edges, ports) | YES | YES | Claims byte-exact vs ELK 0.11.0 jars (201 goldens) | **Upgrade candidate, not primary.** Provenance risk: it is a translation of EPL-2.0 Java but labelled Apache-2.0; needs legal review. Too new (<6 months, single maintainer) to be the foundation |
| `merman-elk-layered` | 0.9.0 | **EPL-2.0** (honestly labelled) | YES | YES | YES | Active | Blocked by `deny.toml` licence allowlist (no EPL-2.0); would need a user-authorized licence decision |
| **elkjs in Node** (option b) | 0.12.0, 2026-07-17 | EPL-2.0 OR GPL-3.0-or-later | YES, best-in-class (hierarchical edges, ports) | YES | YES | Deterministic (fixed random seed) | **Reject as primary** (see below) |
| **Hand-rolled Sugiyama** (option c) | n/a | n/a | n/a | n/a | n/a | n/a | **Reject** |

### Why `dagre` (and what was verified)

I built a throwaway crate (scratchpad, not in the repo) against `dagre = "=0.1.1"` and laid out a graph with: a top container holding 5 leaves plus a nested container (2 leaves), 11 edges incl. 5 labelled edges, a back-edge (cycle), and variable node widths 80-200.

- Compiles on the pinned toolchain in ~8 s; deps are `log` (+ optional `serde`/`serde_json`, all already in `Cargo.lock`). No `build.rs`, no C.
- Both containers got computed boxes (605x446 and 280x90), children placed inside, labelled edges got `x/y` label anchors, back-edge routed around the outside.
- 40 consecutive process invocations produced a byte-identical output hash.
- Zero runtime-dependency footprint beyond `log` -> negligible `cargo-deny`/crate-edge/hakari impact.

**Honest quality statement for the roadmap:** this is the same algorithm family Mermaid uses today (dagre). The 40 live Mermaid diagrams already render through dagre, so migration is *not a layout regression*. The advisory's "dagre hairball" complaint is about large, hand-LLM-generated graphs; our docs diagrams are small (<= ~20 nodes) and we control spacing, label sizing and edge drawing. If a better layout is wanted later, the upgrade is ELK (fallback chain below), not a rewrite.

### Known dagre-family limitation that shapes the DSL (MUST be handled in the validator)

**Edges whose endpoint is a *container* are unsupported by every dagre port.** I verified this: adding `client -> backend` (backend is a cluster) does not panic, it silently treats the cluster as a leaf, gives it a rank/order and routes the edge to a bogus point inside the box (layout of the whole graph shifts). Mermaid works around this with an anchor-node rewrite.

Required handling (choose in phase planning, default shown):
- **Default:** the semantic checker resolves an edge to a container by lowering to an invisible zero-size anchor leaf inside that container (`__anchor_<container>`), edge drawn to the container border by clipping the final segment to the container rect. Emit an `info`-level diagnostic so the author knows.
- Alternative: reject with a span-precise error `edge-to-container-unsupported` and require dotted refs to a leaf (`backend.gateway`). Simpler, matches the advisory example (`client -> backend.gateway`). **Recommend this for v1.6 and add anchor lowering only if the importer needs it** (check the 40 live blocks: subgraph-targeted edges are the only case).

### Why NOT ELK-in-Node (option b)

- Forces a Rust -> Node -> Rust round-trip (Rust has to know node sizes first, Node returns coordinates, Rust draws SVG). Every golden/snapshot test and the `linux`/`tests` CI shards (Rust-only, no `pnpm install`) would need Node or frozen fixtures.
- The SVG emitter must stay in Rust (requirement), so splitting layout into JS splits the pipeline across two languages for the one step that most needs to be reproducible.
- 8 MB of GWT-compiled JS, licence EPL-2.0 OR GPL-3.0 (fine for a build tool, but a second licence surface to document).
- Gains (hierarchical edges, ports, orthogonal routing) are not needed by the MVP DSL.
- Keep the door open: define the seam's input/output as plain serializable structs shaped like ELK's JSON (`children`, `edges`, `labels`, `width/height`) so `elkrs` (same JSON in/out) or elkjs can be plugged in without touching parser/IR/emitters.

### Why NOT hand-rolled Sugiyama (option c)

A correct compound Sugiyama = acyclic + network-simplex ranking + nesting graph + border segments + barycenter ordering with subgraph constraints + Brandes-Koepf. `dagre` is ~8.6k LOC of source and ~8.8k LOC of tests (528 tests). Re-implementing it is a milestone by itself, violates the "reuse / minimum code" bar, and `AGENTS.md` forbids forking 100+ line chunks of upstream (defactor policy). Revisit only if all of dagre/dugong/elkrs fail the spike.

### Required architecture around the pick (so the pick is cheap to reverse)

```
crates/vox-diagram/src/layout/
  mod.rs        // pub fn solve(&LayoutInput, &LayoutConfig) -> Result<LayoutOutput, LayoutError>
  dagre_impl.rs // the ONLY file that imports `dagre`  (~150 lines: IR -> dagre Graph, results back)
```

- `LayoutInput`: node id, parent id, width, height; edge (from, to, label size); direction. `LayoutOutput`: node rects (top-left x,y,w,h), container rects, edge polylines, edge label anchors. All `f64` rounded to 2 decimals at the seam boundary.
- Wrap `dagre::layout` in `std::panic::catch_unwind` (it returns `()` and may panic on malformed graphs); map to `LayoutError::EngineFailure` so the MCP/CLI path can never crash on user input.
- Guard before layout: node/edge count caps (e.g. 300 nodes / 600 edges) with a diagnostic, because dagre has no work budget (dugong does).
- Snapshot-test the seam with `insta` so a crate bump that changes coordinates shows up as a reviewable diff.

### Spike gate (do first in the layout phase, <= 1 day)

Render the 10 largest live Mermaid diagrams (by node count) through the `dagre` seam. Accept if <= 2 need DSL tweaks. If not: swap in `dugong` (same dagre algorithm, 1-hour adapter), then evaluate `elkrs` (needs a licence/provenance check by a human before it enters `Cargo.toml`; `deny.toml` will pass an `Apache-2.0` label but the underlying code is ELK/EPL-2.0 derived).

---

## 2. Recommended Stack

### Core (new dependencies of `vox-diagram`)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `dagre` | `0.1.1` (caret = 0.1.x only; add to `[workspace.dependencies]`, `default-features = false`) | Compound layered layout | See section 1 |
| `unicode-width` | `0.2` (resolves to 0.2.2, **already in `Cargo.lock`** as a transitive dep; promote to `[workspace.dependencies]`) | East-Asian-wide / emoji char widths for label measurement | Zero new crate in the graph, MIT/Apache, no deps |
| `serde` (workspace) | existing | `Serialize` for IR + `Diagnostic` JSON (MCP result, `--json` CLI) | Same pattern as `MermaidDiagnostic` in `vox-doc-pipeline` |
| `serde_json` (workspace, `1.0.150` locked) | existing | IR JSON dump / MCP payloads | Already everywhere |
| `workspace-hack` | existing | hakari unification | Every crate has it; `cargo hakari generate` after adding `dagre`/`unicode-width` |

Everything else (lexer, parser, checker, Mermaid emitter, SVG writer, flowchart importer, text metrics table) is first-party code in the crate with **no** dependency.

### Dev-only

| Library | Version | Purpose |
|---------|---------|---------|
| `insta` (workspace, `features = ["json"]`; 1.48.0 locked) | existing | Golden SVG / Mermaid / diagnostics snapshots |
| `proptest` (workspace, 1.12.0; `default-features = false, features = ["std"]`) | existing | Properties: `parse(print(ast)) == ast`; emitted Mermaid always satisfies the sanitizer invariants; layout is deterministic |

### Not new, reused as-is

| Existing | Where used |
|----------|-----------|
| `pulldown-cmark` 0.13 (in `vox-doc-pipeline`) | Extract ` ```vox-diagram ` fences + byte offsets in the docs gate; `vox-diagram` itself never touches Markdown |
| `regex` | Only in `vox-doc-pipeline` for the new "no new ```mermaid block" gate, if needed; not in `vox-diagram` |
| `vox_validate_mermaid` / `mermaid.rs` | Used as a **differential test oracle**: every Mermaid string emitted by `vox-diagram` must produce zero diagnostics and `was_repaired == false` in `vox-doc-pipeline` tests (free, in-repo, no extra dep) |

---

## 3. Parser approach: hand-written, zero deps

What the workspace already has: `logos 0.16` (used only by `vox-compiler`'s lexer), `winnow` 0.7/1.0 (transitive only, no first-party user), `pest` 2.8 and `nom` 7 (transitive only). **No `chumsky` anywhere** (not in `Cargo.lock`).

Recommendation: hand-written `lexer.rs` (byte-offset `Span { start, end }`, tokens: ident, dotted path, string, `{ } : ->`, keywords `node container direction ...`) + `parser.rs` recursive descent with *line/brace synchronization* error recovery (collect all diagnostics, never stop at the first).

Why not the libraries:
- `chumsky` 0.13 (2026-05): strongest error recovery, but not in the tree, big generic-heavy compile-time cost, and its error type must be re-mapped to our `Diagnostic` anyway. Overkill for a grammar this small.
- `winnow` 1.0.x: fine, but hand-rolled errors are still manual; no payoff over 300 lines of descent code.
- `logos` 0.16: already a workspace pin, so it is the only defensible library for the *lexer alone*. Skip in v1.6: tokens here are trivial and we need quote-aware recovery (unterminated string -> diagnostic at its start offset) that is easier in a hand-written scanner. Reconsider only if the DSL grows (sequence/state diagrams).

Diagnostics: own `struct Diagnostic { code: &'static str, severity, span: Range<usize>, line, col, message, hint }` (serde). Do **not** pull `miette` into the library (L2 stays lean); the CLI can print `path:line:col: code: message` plus a caret line itself. Stable codes (e.g. `diagram/unterminated-string`, `diagram/unknown-ref`, `diagram/duplicate-id`, `diagram/edge-to-container`) match the existing `mermaid/...` code style.

Hard rules from `AGENTS.md` that apply: new `pub fn` needs a test in the same file first (TDD guard); files under the 500 non-blank-line `god_object` cap -> split `lexer.rs / parser.rs / ast.rs / check.rs / layout/ / svg.rs / mermaid_out.rs / import.rs / text.rs`.

---

## 4. SVG writing: plain string builder

- `svg` crate 0.18.0 (last release 2024-09): `pub type Attributes = HashMap<String, Value>` (confirmed in source) -> attribute order depends on the per-process random hash seed -> golden SVGs flap. **Reject.**
- Write a ~100-line `SvgWriter` over `String`: `fn open(tag, &[(&str, &str)])` emitting attributes in the order passed; one `xml_escape()` (`& < > " '`) applied to every text node and attribute value; numbers through one `fmt_num(f64)` (2 decimals, trim trailing zeros, normalize `-0`).
- Determinism rules to bake in: no timestamps, no random ids (element ids = `vd-<blake3-of-source-first-8>` or sequence index; **do not** add `blake3` directly -- application hashing must go through `vox-crypto` per `AGENTS.md`, so prefer sequence-index ids and let `vox-doc-pipeline` content-address if it needs a manifest key), LF newlines, `\n`-terminated.
- Theming: emit `fill="currentColor"`/CSS custom properties (`var(--sl-color-text)`, etc.) instead of baked colours so one SVG works in Starlight light/dark; no `<style>` with external fonts, no `<script>`, no `<foreignObject>`.
- Edge geometry: dagre returns polylines; draw with straight segments + rounded corners via quadratic Bezier (`Q`) computed with plain arithmetic (no trig -> cross-platform bit-stable). Arrowheads as a shared `<marker>`.

## 5. Text measurement (no browser, no font crate)

Rejected: `fontdue` 0.9.4, `ab_glyph` 0.2.32, `ttf-parser` 0.25.1, `skrifa` 0.48, `fontdb`. None is in `Cargo.lock`; each needs a bundled font file (binary asset + OFL licence record in `assets/`), and the measured result is still only an estimate because the reader's browser renders the SVG text with *its* font (Starlight's system stack).

Chosen: a `static ADVANCE_1000: [u16; 95]` table for printable ASCII (Helvetica/Arial-class advance widths per 1000 em, i.e. the Adobe core-14 AFM metrics), `width = sum(advance) * font_px / 1000`; non-ASCII via `unicode_width::UnicodeWidthChar` (wide = 1.0 em, narrow = 0.55 em fallback, zero-width = 0). Word-wrap at a configurable max label width on whitespace, `\n` explicit line breaks inside quoted labels (the live corpus has 16 lines using `<br>`, which the importer maps to `\n`). Node width = `max(min_w, text_w + 2*pad_x)`; add ~8% slack so a wider real font does not overflow the box; `text-anchor="middle"` so errors stay centred. This is the same approach `mermaid-rs-renderer --fastText` uses ("calibrated fallback widths for ASCII"). Keep the table as the single calibration knob (`TextMetrics` struct) so it can be retuned without touching layout.

## 6. Mermaid parser for the importer

Rust Mermaid parsers that exist (checked):

| Crate | Version | Covers | Deps | Use? |
|-------|---------|--------|------|------|
| `merman-core` (+`merman`) | 0.9.0, 2026-10-12, MIT/Apache, MSRV 1.95, used by Zed | 25+ diagram families, parity-tested vs mermaid.js | ~20 direct (`lol_html`, `cssparser`, `lalrpop-util`, `granit-parser`, `url`, `json5`, `euclid`, ...) | **No** for the importer. Maybe a *dev-only oracle* (see below) |
| `mermaid-rs-renderer` | 0.3.1, MIT | renderer (23 types), internal parser | `fontdb`, `ttf-parser`, `json5`, `regex`, ... | No (renderer, not an importable AST) |
| `selkie-rs` | 0.3.0, MIT | parser+renderer | `pest`, `fontdue`, `roxmltree`, `chrono`, `uuid` | No |
| `mermaid-parser` | 0.1.0 | class diagrams only (pest) | - | No |

Measured corpus (live `docs/src`, excluding the tombstoned `archive/`): **25 files, 40 blocks** (the "~71 in ~55" figure includes 28 archive files / 29 blocks that `AGENTS.md` says not to touch). Block types: 16 `flowchart LR`, 14 `flowchart TD`, 5 `graph TD`, 1 `flowchart TB`, 2 `sequenceDiagram`, 1 `stateDiagram-v2`, 1 `gantt`. Features seen in the 909 live lines: 40 `subgraph`, 12 `-->|label|`, 14 `{decision}` shapes, 2 `-.->`, 16 `<br>` labels, `A & B --> C` fan-out, 1 `click`, 0 `classDef`/`style`/`:::`/`linkStyle`.

Decision: **hand-written flowchart/graph importer** in `vox-diagram::import` handling: header+direction, node shapes -> `kind`, quoted/unquoted labels, `-->` / `---` / `-.->` / `==>` with `|label|` and `-- text -->` forms, `subgraph id [Title] ... end` -> `container`, `&` fan-out, `<br>` -> `\n`; anything else -> a diagnostic and the block is left as Mermaid (allow-listed in the gate). The 4 non-flowchart blocks stay Mermaid with an explicit allowlist entry; no importer for sequence/state/gantt in v1.6 (out of the "flowchart-class" DSL).

Optional dev-only oracle for the "guaranteed-valid Mermaid" claim: `merman-core = "0.9"` as a `dev-dependency` of **`vox-doc-pipeline`** (not of `vox-diagram`). Cost: ~20 new crates in `Cargo.lock` and a `cargo-deny` licence pass. Recommended default for v1.6: **skip it** and rely on (1) the sanitizer invariants + proptest, (2) the existing `mermaid.rs` validator as differential oracle. Add it only if a reviewer wants an upstream-grammar oracle. Confidence MEDIUM (not verified that every transitive licence is on the `deny.toml` allowlist).

---

## 7. Integration points (layer, consumers, edges)

**Layer:** `vox-diagram = { layer = 2 }` in `docs/src/architecture/layers.toml` (pure-data domain logic, comparable to `vox-similarity`; no async, no I/O, no DB, no compiler dependency). Suggested `max_loc = 8_000`. Add the row to `docs/src/architecture/where-things-live.md` in the same PR (Rule 12 WTL parity is `error`).

**Consumers (all downward edges, L3 -> L2):**

| Consumer | Layer | Use |
|----------|-------|-----|
| `vox-doc-pipeline` | L3 | Docs gate (reject new ` ```mermaid ` blocks outside allowlist, validate ` ```vox-diagram ` blocks), **pre-render** step (see below), `import` CLI mode |
| `vox-orchestrator-mcp` | L3 | `vox_validate_diagram` tool (parse+check only; no layout needed). Note: it does **not** currently depend on `vox-doc-pipeline`, so this is a separate new edge |
| `vox-cli` (optional) | L5 | `vox diagram check|render|import` -- already depends on `vox-doc-pipeline`; add a direct edge only if the subcommand needs types directly |

**Process gates the roadmap must schedule (these are blockers, not afterthoughts):**
- **Crate-edge ratchet:** new edges `vox-doc-pipeline -> vox-diagram` and `vox-orchestrator-mcp -> vox-diagram` are not in `contracts/ci/crate-edges.allow.v1.json`. Per `AGENTS.md` Dependency Discipline, an agent must **propose** the baseline/`exceptions` entries in the PR description and stop; adding them is USER-AUTHORIZED-ONLY. Plan a user-approval step.
- **hakari:** adding `dagre` / promoting `unicode-width` changes `crates/workspace-hack/Cargo.toml`; run `cargo hakari generate` (nightly verifies drift).
- **Workspace pin placement:** `dagre` and `unicode-width` go in root `[workspace.dependencies]` (house style: centralized external versions).
- **MCP surface sync:** a new tool changes `docs/src/reference/mcp-tools.generated.md` (`cargo run -p vox-cli -- ci command-sync`) and the GUI coverage gate (349/349 MCP tools covered-or-audited) will fail until `vox_validate_diagram` is covered or audited; register in `input_schemas.rs`/`dispatch.rs` of `vox-orchestrator-mcp`.
- **Docs frontmatter + doctest:** any new doc about the format needs the required frontmatter; ` ```vox-diagram ` fences are not ` ```vox ` so they are not compiled by the doctest runner, but make sure `doctest-md` does not treat the info string `vox-diagram` as `vox` (prefix match) -- verify in the gate phase.
- **Test-first policy:** every new `pub fn` ships with a test in the same file (TOESTUB `skeleton/untested-pub-api`).
- **Cargo via the build broker:** plain `cargo` on PATH; no manual concurrency.

### Docs-build integration (how SVG gets onto voxlang.org without new Node deps)

`.github/workflows/docs-deploy.yml` already runs `cargo build -p vox-doc-pipeline --profile ci` and `cargo run -p vox-doc-pipeline` **before** `pnpm build`. So:

1. `vox-doc-pipeline` (Rust) walks `docs/src/**` (excluding `archive/`), renders every ` ```vox-diagram ` fence to SVG via `vox-diagram`, and writes `docs-astro/src/generated/diagrams/<source-hash>.svg` plus a `manifest.json` (hash -> svg path). Hash = a content hash computed through the project's approved hashing path (`vox-crypto`), not a new direct hashing dep.
2. A tiny `docs-astro/src/plugins/remark-vox-diagram.mjs` (same shape as the existing `remark-vox-include.mjs`; `.mjs` is the convention in that directory) replaces the fenced node with the inline SVG looked up by hash and **fails the build on a missing entry**, mirroring the include plugin's fail-fast rule.
3. This also removes the runtime `import('https://cdn.jsdelivr.net/npm/mermaid@11/...')` CDN fetch in `PageSidebar.astro` for migrated pages (the existing Mermaid client path stays for the allow-listed leftovers).

No `pnpm add` is required. Node never calls Rust and Rust never calls Node.

---

## Installation

```toml
# Cargo.toml  [workspace.dependencies]  (add)
dagre         = { version = "0.1.1", default-features = false }   # pure-Rust dagre.js port; compound + edge labels; layout seam only (vox-diagram/src/layout/dagre_impl.rs)
unicode-width = "0.2"                                              # already in Cargo.lock at 0.2.2 via transitive deps

# crates/vox-diagram/Cargo.toml
[dependencies]
dagre         = { workspace = true }
unicode-width = { workspace = true }
serde         = { workspace = true }
serde_json    = { workspace = true }
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[dev-dependencies]
insta    = { workspace = true }
proptest = { workspace = true }
```

```bash
# After editing manifests (via the build broker, plain cargo):
cargo hakari generate        # refresh crates/workspace-hack
cargo check -p vox-diagram
cargo nextest run -p vox-diagram
# No npm / pnpm changes.
```

## Alternatives Considered

| Recommended | Alternative | When to use the alternative |
|-------------|-------------|-----------------------------|
| `dagre` 0.1.1 | `dugong` 0.9.0 | If `dagre` goes unmaintained or a bug needs an upstream fix quickly (dugong has far higher release cadence and a `WorkControl` budget). Adapter cost ~1 hour because the data model is identical |
| `dagre` 0.1.1 | `elkrs` 0.1.1 / elkjs 0.12.0 | Spike shows unacceptable layouts, or the DSL later needs ports / true hierarchical edges / orthogonal routing. `elkrs` after a provenance+licence review; elkjs only as an offline pre-render tool |
| Hand-written parser | `chumsky` 0.13 | DSL grows into multiple diagram grammars needing rich auto-recovery |
| Hand-written parser | `logos` 0.16 (lexer only) | Lexer becomes non-trivial or is shared with `vox-compiler` tokens |
| String SVG builder | `svg` 0.18.0 | Never (HashMap attributes) |
| Static metrics table | `ttf-parser`/`fontdue` + bundled font | Only if the site ships a fixed webfont and pixel-exact fit matters; add then as a calibration step, not before |
| Hand-written importer | `merman-core` 0.9.0 | If the importer must later cover sequence/state/class/ER at scale |

## What NOT to Use

| Avoid | Why | Use instead |
|-------|-----|-------------|
| `svg` crate | Non-deterministic attribute order (std `HashMap`) | `String` writer with fixed order |
| `rust-sugiyama`, `layout-rs` | No compound clusters (verified); std `HashMap` nondeterminism / open hang bug | `dagre` |
| `dagre-rs` (TangleGuard) | Non-standard licence file; no compound | `dagre` |
| `merman-elk-layered` | EPL-2.0 not on `deny.toml` allowlist | `dugong` / `elkrs` (reviewed) |
| `elkrs` as primary | 1-month-old, 1-star, licence provenance unclear | Keep as spike/upgrade path |
| elkjs / `mermaid` / `@mermaid-js/*` as build deps | Node round-trip breaks Rust-only CI and golden tests; client Mermaid is the thing we are replacing | Rust pre-render |
| Any font crate + bundled font file | Binary asset, still only an estimate of the reader's font | Static metrics table |
| `miette` in `vox-diagram` | Keeps L2 lean; MCP wants JSON | Own `Diagnostic` struct (serde) |
| Direct `blake3`/`sha2` in `vox-diagram` | Crypto policy: application hashing goes through `vox-crypto`; banned-crate-import detector | Sequence-index ids; hash in the consumer via `vox-crypto` |
| `headless_chrome`/`chromiumoxide`/Puppeteer for rendering | Pulls `aws-lc-rs` TLS provider workspace-wide (see AGENTS.md crypto section) | n/a |
| Rewriting layout by hand | Months of work, forking policy | Reuse `dagre` |

## Version Compatibility

| Item | Compatible with | Notes |
|------|-----------------|-------|
| `dagre` 0.1.1 (edition 2024) | Rust 1.98.1 (`rust-toolchain.toml`) | Built clean in a scratch crate |
| `dugong` 0.9.0 / `merman` 0.9.0 | MSRV 1.95 -> OK on 1.98.1 | Fallback only |
| `unicode-width` 0.2.2 | already locked (alongside 0.1.14, duplicates already tolerated: `multiple-versions = "warn"`) | |
| `cargo-deny` licences | `dagre` Apache-2.0, `dugong` MIT OR Apache-2.0, `unicode-width` MIT OR Apache-2.0 -> all on allowlist | `merman-elk-layered` EPL-2.0 would not be |
| Build-toolchain invariant | No dep adds cmake/nasm/Go/perl/libclang | `dagre` -> `log` only |

## Confidence by claim

| Claim | Confidence | Basis |
|-------|-----------|-------|
| `dagre` 0.1.1 supports compound + labels + variable sizes and is deterministic | HIGH | Built and ran it; 40/40 identical runs; source/README read |
| dagre-family cannot route edges to cluster nodes | HIGH | Reproduced (silent mis-layout) |
| `rust-sugiyama` / `layout-rs` lack compound support | HIGH | Source tree + open issue #"Support for Subgraphs/Clusters" |
| `svg` 0.18.0 uses HashMap attributes | HIGH | Read source |
| `dagre` long-term maintenance | MEDIUM | Single maintainer; repo moved into a monorepo (Actrium/supramark); no code commits since May 2026; mitigated by the seam + `dugong` fallback |
| `elkrs` suitability | LOW-MEDIUM | New, unaudited provenance; not recommended until reviewed |
| `merman-core` licence closure vs `deny.toml` | LOW | Not exhaustively checked; only matters if the optional oracle is adopted |
| Live corpus numbers (40 blocks / 25 files) | HIGH | Counted from `docs/src` excluding `archive/` |

## Sources

- crates.io API (2026-10-16): `dagre` 0.1.1, `dugong` 0.9.0, `dugong-graphlib` 0.9.0, `rust-sugiyama` 0.4.0, `layout-rs` 0.1.3, `dagre-rs` 0.1.0, `elkrs` 0.1.1, `merman*` 0.9.0, `mermaid-rs-renderer` 0.3.1, `selkie-rs` 0.3.0, `svg` 0.18.0, `fontdue` 0.9.4, `ab_glyph` 0.2.32, `ttf-parser` 0.25.1, `skrifa` 0.48.0, `chumsky` 0.13.0, `winnow` 1.0.4, `logos` 0.16.1, `unicode-width` 0.2.2, `proptest` 1.12.0
- npm registry: `elkjs` 0.12.0 (EPL-2.0 OR GPL-3.0-or-later), `@dagrejs/dagre` 3.1.1
- GitHub: kookyleo/dagre-rs -> Actrium/supramark `crates/dagre`; Latias94/merman (`dugong`, `merman-elk-layered`); depetrol/elkrs; paddison/rust-sugiyama; nadavrot/layout (issues list); 1jehuang/mermaid-rs-renderer
- Local verification: scratch crate building `dagre =0.1.1`; downloaded `svg-0.18.0` and `dagre-0.1.1` sources; workspace `Cargo.toml`, `Cargo.lock`, `deny.toml`, `docs/src/architecture/layers.toml`, `contracts/ci/crate-edges.allow.v1.json`, `.github/workflows/docs-deploy.yml`, `docs-astro/` plugins, `docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md`
