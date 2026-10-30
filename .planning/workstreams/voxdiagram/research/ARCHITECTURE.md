# Architecture Research: VoxDiagram (v1.6 companion track)

**Domain:** Native diagram format (text DSL -> typed IR -> layout -> SVG / sanitized Mermaid, plus Mermaid importer) integrated into an existing 138+ crate, layered Rust workspace with an Astro/Starlight docs site
**Researched:** 2026-10-16
**Confidence:** HIGH for repo integration facts (all read from source in this session); MEDIUM for Astro/MDX/`starlight-llms-txt` behaviour (not executed, flagged as spikes in the build order)

## Headline Decisions (read this first)

1. **One new crate, `vox-diagram`, Layer 2 (domain library), zero workspace deps.** Pure `std` + `serde`/`thiserror`. No tokio, no DB, no `vox-compiler` (compiler/LSP integration is out of scope and would force L3).
2. **Two new consumer edges are required and BOTH are user-authorized-only exceptions** in `contracts/ci/crate-edges.allow.v1.json`: `vox-doc-pipeline -> vox-diagram` and `vox-orchestrator-mcp -> vox-diagram`. I recommend routing the CLI through a `vox_doc_pipeline::diagram` re-export so `vox-cli` needs **no** third edge (it already lists 66 baseline out-edges against `max_workspace_deps = 60`). Request these before any wiring work starts; this is the Day-0 blocker.
3. **Docs build: Rust pre-render step + thin pure-Node remark plugin.** `vox-doc-pipeline --render-diagrams` (already built by both docs workflows) walks `docs/src`, writes a gitignored manifest; a remark plugin looks up each ` ```vox-diagram ` fence by exact normalized source and inlines the SVG. Reject "plugin spawns `vox`" (the `vox` binary is not built in `docs-deploy.yml`) and reject WASM (needs a wasm32 target + wasm-bindgen, violating the build-toolchain invariant).
4. **Lint/gate lives in `vox-doc-pipeline` (doc lint), not `vox ci`.** Reuse the facet-backlog ratchet pattern for a shrink-only legacy-Mermaid baseline. Keep the existing Mermaid validator and `vox_validate_mermaid` (agents still emit Mermaid into chat/PR bodies); do not extend them.
5. **MCP `vox_validate_diagram` is registered via the operations catalog (SSOT) and fans out to ~9 generated/hand-synced artifacts.** Treat it as a checklist, not a code change.
6. **Reality check on scope numbers:** the live published site has **41 mermaid fences in 26 files** (37 flowchart/graph, 2 sequenceDiagram, 1 stateDiagram-v2, 1 gantt), not ~71. The remainder are in `docs/src/archive/` (tombstoned, never touch) and `docs/superpowers/` (~10, not published). And **Mermaid is not rendered on voxlang.org at all today** - `docs-astro` has no mermaid integration (the 2026-08-22 client-side `astro-mermaid` plan never landed), so every fence, including the one on the home page `docs/src/index.mdx:87`, shows as a grey code block. VoxDiagram is the first diagram renderer the site gets.

## Standard Architecture

### System Overview

```
 AUTHORS / AGENTS                      L5 SURFACES
 ```vox-diagram fences in docs/src/**.md|mdx, MCP callers, `vox diagram ...`
        |                    |                      |
        v                    v                      v
+-----------------+  +--------------------+  +------------------------+
| vox-doc-pipeline|  | vox-orchestrator-  |  | vox-cli                |
| (L3, MODIFIED)  |  | mcp (L3, MODIFIED) |  | (L5, MODIFIED, thin)   |
| - fence lint    |  | - diagram_tools.rs |  | - commands/diagram.rs  |
| - legacy-mermaid|  |   vox_validate_    |  |   calls                |
|   ratchet       |  |   diagram          |  |   vox_doc_pipeline::   |
| - --render-     |  +---------+----------+  |   diagram (re-export)  |
|   diagrams      |            |             +-----------+------------+
| - pub use       |            |                         |
|   vox_diagram   |            |                         |
+--------+--------+            |                         |
         |   (new edge #1)     | (new edge #2)           | (no new edge)
         +----------+----------+-------------------------+
                    v
        +-----------------------------------------------+
        | vox-diagram (L2, NEW, pure, sync, no workspace |
        | deps)                                          |
        |  fence  -> lex/parse -> AST -> validate -> IR  |
        |  IR -> canonical printer (fmt)                 |
        |  IR -> layout (pure fn) -> SVG emitter         |
        |  IR -> Mermaid emitter (valid by construction) |
        |  Mermaid text -> importer -> IR                |
        +-----------------------------------------------+

 DOCS BUILD (CI: docs-deploy.yml / docs-quality.yml; local: pnpm build)
 docs/src/**/*.md(x) --[vox-doc-pipeline --render-diagrams]--> docs-astro/src/generated/diagrams.manifest.json (gitignored)
        |
        v
 docs-astro `prebuild` -> remark plugin (lookup by normalized source) -> inline <figure><svg/></figure> -> Starlight -> dist/
```

### Component Responsibilities

| Component | Responsibility | Status |
|-----------|----------------|--------|
| `crates/vox-diagram/` | Grammar, spans, diagnostics with stable `diagram/*` codes and fix hints, IR, canonical printer, layout, SVG + Mermaid emitters, Mermaid importer, markdown fence extraction | NEW |
| `crates/vox-doc-pipeline/src/pipeline/diagrams.rs` | Walk `docs/src` (skip `archive/`), call `vox-diagram`, write manifest; shared by lint, `--render-diagrams`, and the `vox diagram` CLI | NEW file |
| `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs` | Shrink-only legacy-Mermaid baseline (seed/tighten/gate), modelled on `facet_backlog.rs` | NEW file |
| `crates/vox-doc-pipeline/src/pipeline/{lint.rs,types.rs,mod.rs}` | New `LintKind::InvalidDiagram` and `LintKind::LegacyMermaid`; flag parsing for `--render-diagrams`, `--seed/--tighten-mermaid-baseline`; keep edits small (mod.rs is 884 lines, god-object limit is 500) | MODIFIED |
| `crates/vox-doc-pipeline/src/claims/self_heal_bot.rs` | Add diagram fence validation next to existing Mermaid check (BOT-03 pre-PR gate) | MODIFIED (small) |
| `crates/vox-doc-pipeline/src/pipeline/openwiki.rs` | Later: generators emit ` ```vox-diagram ` from IR builders instead of Mermaid strings (`generate_subsystem_hubs` is currently only reachable from tests) | MODIFIED (late) |
| `crates/vox-orchestrator-mcp/src/diagram_tools.rs` | `vox_validate_diagram` implementation | NEW |
| `crates/vox-orchestrator-mcp/src/{dispatch.rs,input_schemas.rs,lib.rs}` | Dispatch arm, input schema, `pub mod` | MODIFIED |
| `crates/vox-cli/src/commands/diagram.rs` + `lib.rs` + `cli_dispatch/mod.rs` | `vox diagram check|fmt|render|export|import` clap wrapper | NEW + MODIFIED |
| `docs-astro/scripts/render-diagrams.mjs` | Locate the `vox-doc-pipeline` binary, run it once, fail loudly with the exact command if missing | NEW |
| `docs-astro/src/plugins/remark-vox-diagram.mjs` | mdast `code[lang=vox-diagram]` -> inline SVG via `data.hName`/`hChildren`; hard error on manifest miss | NEW |
| `docs-astro/{astro.config.mjs,package.json}` + `src/styles/*.css` | Register plugin after `remarkVoxInclude`; `prebuild`/`predev` run render step; add `hast-util-from-html`; CSS variable mapping to `--sl-*` | MODIFIED |

## Layer and Dependency Analysis (verified against layers.toml, crate_edges.rs, allow file)

### Layer: L2

`layers.toml` defines L2 as "Domain libraries (pure-data domain logic)", dependencies must point same-layer or down. `vox-diagram` has no async runtime, DB, or compiler need, so it is L2 with an expected dependency set of `serde`, `serde_json`, `thiserror` (+ `petgraph` only if STACK research prefers it for layout; it is already a workspace dep) and dev-deps `insta` (json) and `proptest`, all already in the root `Cargo.toml`. It must **not** depend on `vox-compiler` (L3) - that would be an upward-impossible/heavy edge and drags the compiler into every consumer.

Do not split into `vox-diagram-core` + `vox-diagram` (the advisory sketches both names). Each crate costs a `layers.toml` row, a WTL row, a docs-api page, and consumer edges that need exceptions. One crate, split into modules; file limit is 500 non-blank lines per file (`arch/god_object` is an Error), so plan modules up front: `lexer`, `parser`, `ast`, `validate`, `ir`, `print`, `layout`, `svg`, `theme`, `mermaid_emit`, `mermaid_import`, `fence`, `diagnostics`.

### New-crate registration checklist (each one is gated)

| Artifact | Why | Gate |
|----------|-----|------|
| Root `Cargo.toml` `[workspace.dependencies] vox-diagram = { path = "crates/vox-diagram" }` (members is `crates/*`, so membership is automatic) | Consumers use `workspace = true` | cargo |
| `crates/vox-diagram/Cargo.toml` with `description` (guard `description = "error"`), `workspace-hack` dep (`{ version = "0.6", path = "../workspace-hack" }`, copy `vox-kb-facets`), `[lints] workspace = true` | hakari + arch-check Rule | `vox-arch-check` |
| `docs/src/architecture/layers.toml` `[crates]`: `vox-diagram = { layer = 2, max_loc = 12_000 }` (max_loc is only Rule 13's growth baseline) | `MissingLayer` is an unconditional crate-edges violation | `vox ci crate-edges`, `vox-arch-check` |
| `docs/src/architecture/where-things-live.md`: crate row in the L2 table + concept rows (diagram DSL/SVG/Mermaid emit; fence lint; render manifest) | `where_things_live = "error"` and Rule 12 three-way parity | `vox-arch-check` |
| Crate-level `//!` docs | `docstring = "warn"` | arch-check |
| `orphan = "error"`: crate needs a consumer; either land with the doc-pipeline consumer in the same PR, or carry `orphan_exempt = true` temporarily (precedent: `vox-oauth-pkce`) and remove it when wired | Orphan rule | arch-check |
| Regenerated: `docs/src/api/vox-diagram.generated.md` + `docs/src/api/index.generated.md`, `contracts/ci/crate-graph.v1.json`, `crate-build-map.v1.json`, `contracts/capabilities/capability-index.v1.json` | Generated; the `ssot-autoregen` PR bot commits them. Do not hand-regenerate after merge (AGENTS.md) | `vox ci ssot-drift` |

### Crate-edge impact (the one thing that needs a human)

`vox ci crate-edges` reads the **live** `cargo metadata` graph and fails any edge not in the frozen baseline `edges` list or the human-gated `exceptions` list. The baseline is machine-tightened only; exceptions are USER-AUTHORIZED-ONLY (AGENTS.md "Dependency Discipline"). There is no "new crates are free" path: the closest precedent, `vox-kb-facets` (KB Phase 4), needed three exceptions (`vox-doc-pipeline`, `vox-search`, `vox-corpus` -> `vox-kb-facets`), each with `authorized_by: user (explicit chat approval, 2026-10-10)`.

| Edge | Layer direction | Needed? | Status |
|------|-----------------|---------|--------|
| `vox-doc-pipeline -> vox-diagram` | L3 -> L2, downward | Yes: lint, render manifest, CLI logic host | **EXCEPTION REQUIRED (user-authorized)** |
| `vox-orchestrator-mcp -> vox-diagram` | L3 -> L2, downward | Yes: `vox_validate_diagram` must not copy a parser | **EXCEPTION REQUIRED (user-authorized)** |
| `vox-cli -> vox-diagram` | L5 -> L2, downward | **Avoid**: use `pub use vox_diagram as diagram;` in `vox-doc-pipeline` (vox-cli already depends on it) | Not requested. If the user prefers a direct edge, request it in the same batch |
| `vox-diagram -> workspace-hack` | exempt (`EXEMPT = "workspace-hack"`) | Yes | No action |

Proposed exception reasons (for the PR description, not to be written by an agent): doc-pipeline - "fence lint, render manifest and `vox diagram` CLI logic call `vox_diagram::{check,render_svg,import_mermaid}`; L3 -> L2 downward"; mcp - "`vox_validate_diagram` calls `vox_diagram::check`; replaces nothing, avoids a third copy of a fence parser".

Why not depend on `vox-doc-pipeline` from MCP instead? Same edge count, but pulls `rayon`, `pulldown-cmark`, `vox-compiler` surface the MCP crate does not want, and the narrower crate is the policy-preferred move. Note the existing duplication: `vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs` re-implements the doc-pipeline Mermaid validator rather than depending on it. Do not repeat that for diagrams; the new `fence` module in `vox-diagram` replaces three hand-rolled fence scanners (doc-pipeline `mermaid.rs::extract_mermaid_blocks`, MCP `mermaid_repair.rs::extract_mermaid_blocks`, doc-pipeline `lint.rs` inline scan) for the diagram path.

Also check `vox-cli`'s `max_workspace_deps = 60`: the baseline already lists 66 `vox-cli` out-edges, so that budget is either already warn-only-exceeded or counted differently. A re-export avoids testing it.

## How the Docs Build Consumes VoxDiagram

### What exists today (verified)

- `docs-astro` is pure Node at build time: `prebuild`/`predev` run `node scripts/setup-content.mjs` (symlink mirror of `docs/src` into `src/content/docs`, plus mounts under `repo/`). It reads Rust-produced data only as **committed contract files** (`contracts/documentation/docs-sidebar-section-order.v1.json`, freshness ledger) or **committed generated markdown** (`docs/src/api/*.generated.md`, `docs/src/reference/*.generated.md`). There is no precedent for Astro invoking a Rust binary.
- Custom remark plugins already exist and are unit-tested with `node --test tests/unit/*.test.mjs` (`remark-vox-include.mjs`, `remark-doc-links.mjs`); build-output Playwright specs live in `docs-astro/tests/build-output/`.
- `docs-deploy.yml` runs `cargo build -p vox-doc-pipeline --profile ci` then `cargo run -p vox-doc-pipeline` (lint mode) **before** `pnpm build`; `docs-quality.yml` runs `cargo build -p vox-cli -p vox-doc-pipeline` then `pnpm build`. The `vox` binary is **not** guaranteed on the deploy path; `vox-doc-pipeline` is.

### Options evaluated

| Option | Verdict | Reason |
|--------|---------|--------|
| **A. Rust pre-render writes artifacts, Node plugin reads them** | **Recommended** | Binary already built in both workflows; one process for all fences (ms); plugin stays pure Node and unit-testable with a fixture manifest; no wasm toolchain |
| B. Remark plugin spawns `vox` per fence | Reject | `vox` not built in docs-deploy; per-fence spawn; makes `pnpm build` silently depend on a binary path. (If a spawn is wanted, spawn `vox-doc-pipeline` once in `prebuild`, which is option A) |
| C. WASM `vox-diagram` loaded in Node | Reject for v1.6 | Needs `wasm32` target + `wasm-bindgen` tooling; AGENTS.md build-toolchain invariant (pinned Rust + C compiler + Node/pnpm only); repo already retired wasm targets for scripts. Revisit only for a live-preview editor |
| D. Commit rendered SVGs (`*.generated.svg`) and add an `ssot-drift` stage | Reject | Reintroduces the regenerate-after-merge churn AGENTS.md calls out; SVG is a pure function of committed source, so nothing is gained |

### Recommended pipeline

```
docs/src/**/*.md(x)  (excl. archive/)
   |  vox-doc-pipeline --render-diagrams --out docs-astro/src/generated/diagrams.manifest.json
   |    walk -> vox_diagram::fence::extract(lang="vox-diagram") -> check -> render_svg
   |    any diagnostic => exit 1 (same diagnostics as lint; build never ships a broken diagram)
   v
diagrams.manifest.json  { schema_version, diagrams: [ { source, svg, title, description } ] }   (gitignored)
   |
   v
remark-vox-diagram.mjs: for each code node lang=="vox-diagram":
   key = value.replace(/\r\n?/g,"\n").trimEnd()  (same normalization in Rust: vox_diagram::fence::normalize)
   entry = manifest.get(key)  -> if missing: throw "stale or missing; run: <exact command>"
   node.data = { hName:"figure", hProperties:{className:["vox-diagram"]}, hChildren:[ fromHtml(entry.svg) ] }
```

Design notes:

- **Key by exact normalized source, not by file path or line.** Astro's `file.path` goes through symlink mirrors and `mounted_from` rewriting (see `remark-doc-links.mjs`), and mdast line numbers may or may not include frontmatter. Source-keyed lookup is immune to both and dedupes identical diagrams.
- **No hashing in `vox-diagram`.** Keys are the source text itself; this sidesteps the cryptography policy (only `vox-crypto` for hashing app data) entirely.
- **Inline SVG, theme via CSS variables.** SVG uses `var(--vd-*, <hex fallback>)`; a small stylesheet maps `--vd-*` to Starlight's `--sl-color-*` so the light/dark toggle works with no re-render. SVG must also be valid standalone (fallback hex), with `role="img"`, `<title>`, `<desc>` generated from the IR (the textual description is a free accessibility win and what Pagefind indexes).
- **MDX matters.** The home page `docs/src/index.mdx` carries a mermaid fence, and raw `html` mdast nodes are not reliable in MDX. `data.hName`/`hChildren` (mdast-util-to-hast contract) yields real hast elements for both `.md` and `.mdx`. **Spike in the integration phase** (MEDIUM confidence): confirm on Astro 6 / Starlight 0.38 that this survives `index.mdx`. Fallback: emit light and dark SVG files to `docs-astro/public/diagrams/` and replace the node with two mdast `image` nodes carrying `data.hProperties.className` (loses inline text and CSS-variable theming, but works everywhere).
- **Expressive Code interplay.** Because the code node becomes a `figure` before rehype, Expressive Code never sees the `vox-diagram` language (no Shiki "unknown language" warning, no need to extend `voxGrammar`). If the spike shows hChildren is ignored for `code` nodes, use the image fallback rather than fighting rehype ordering.
- **`llms-full.txt` must keep the fence source.** The superseded 2026-08-22 plan explicitly chose client-side rendering to avoid stripping diagram source from `llms-full.txt`. `starlight-llms-txt` builds from the raw entry body (to be confirmed, MEDIUM) so it should keep the DSL text; add a build-output assertion in `docs-astro/tests/build-output/llms.spec.ts` style. The DSL is more useful to agents than Mermaid.
- **Dependency:** the plugin needs `hast-util-from-html` as an explicit `docs-astro` dependency (pnpm strict isolation). CI uses `--frozen-lockfile` in four install steps, so commit the regenerated `docs-astro/pnpm-lock.yaml` in the same change.
- **Workflow path filters:** `docs-deploy.yml` and `docs-quality.yml` trigger on `crates/vox-doc-pipeline/**` but not `crates/vox-diagram/**`; add it to both or an emitter change will not rebuild/verify the site.
- **Mounted repo docs** (`/repo/<route>/` from `contracts/documentation/site-mounted-repo-docs.v1.json`) are not walked by the Rust step. Declare `vox-diagram` fences unsupported outside `docs/src` for v1.6; the plugin's hard error makes this visible rather than silent.
- **Dev loop:** `predev` runs the same render step; editing a fence during `astro dev` requires re-running it. Acceptable; do not build watch mode.

### What the Mermaid emitter is for

The docs site does not need Mermaid. The sanitized Mermaid emitter serves GitHub-rendered surfaces (PR bodies, READMEs, issue comments) and agents that must hand Mermaid to a third-party renderer. Today `README.md` has zero mermaid fences, so there is no docs consumer in v1.6 beyond `vox diagram export --format mermaid` and the cross-check test below. Accepted trade-off: on github.com, ` ```vox-diagram ` shows as readable source (the LLM-friendly property), not a picture.

## MCP Tool `vox_validate_diagram`

### Where it is registered (verified from `vox_validate_mermaid` and `vox_validate_vuv`)

Source of truth is `contracts/operations/catalog.v1.yaml`; the others are projections or fixtures.

| # | Artifact | Action | How |
|---|----------|--------|-----|
| 1 | `contracts/operations/catalog.v1.yaml` | Add row `id: validate.diagram`, `mcp: { name: vox_validate_diagram, http_read_role_eligible: true, exposure: public, requires: [] }`, `side_effect_class: none`, `product_lane: ai`, `cli: null` | hand edit (SSOT) |
| 2 | `contracts/mcp/tool-registry.canonical.yaml` | Projection | `vox ci operations-sync --target mcp --write` (or `--target all`) |
| 3 | `contracts/capability/capability-registry.yaml` (`mcp.vox_validate_diagram`) | Projection | `operations-sync --target capability` |
| 4 | `contracts/mcp/http-read-role-governance.yaml` `read_role_tools` (sorted list) | Add name if `http_read_role_eligible: true` | hand edit; verify with `operations_catalog_exposure` |
| 5 | `contracts/profiles/lang.v1.yaml` `tools.include` | Add `vox_validate_diagram` next to `vox_validate_vuv`. Note `vox_validate_mermaid` is in no profile; confirm what `vox ci profile-parity` requires and follow the same outcome | hand edit + `profile-parity` |
| 6 | `crates/vox-orchestrator-mcp/src/diagram_tools.rs` + `lib.rs` `pub mod` | Implementation; follow the WTL "Add an MCP tool" row (`<group>_tools.rs`), not `chat_tools/chat/` (that is chat-turn specific) | code |
| 7 | `crates/vox-orchestrator-mcp/src/input_schemas.rs` | Add schema next to `"vox_validate_mermaid"` (`code` / `markdown` / optional `emit: ["canonical","mermaid"]`, `additionalProperties:false`) | code |
| 8 | `crates/vox-orchestrator-mcp/src/dispatch.rs` | Add arm next to line ~898 `"vox_validate_mermaid"` | code |
| 9 | `crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt` | Insert name (sorted) | hand edit |
| 10 | `docs/src/reference/mcp-tools.generated.md`, `contracts/capabilities/capability-index.v1.json`, `contracts/capability/model-manifest.generated.json`, `crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts`, `docs/src/api/vox-orchestrator-mcp.generated.md` | Generated | `vox ci command-sync --write`, `vox ci capability-index --write`; autoregen bot otherwise |

Semantics: input `code` (raw DSL) or `markdown` (extract ` ```vox-diagram ` fences); output `{ valid, diagnostics: [{line, column, code: "diagram/...", message, fix}], canonical?, mermaid? }` with line numbers mapped into the markdown file for the `markdown` input, mirroring `vox_validate_mermaid`'s contract so the repair loop is identical. Diagnostics are deterministic with a machine-applicable `fix` hint where one exists; this is the "no LLM repair round-trip" benefit over the Mermaid regex path.

Out of scope for v1.6 (keep `agent_loop.rs` from growing; WTL says do not grow it): chat-turn auto-interception for `vox-diagram` fences. Optional later: parallel branch in `post_verification.rs` (line ~145 checks only ` ```mermaid `).

## Existing Mermaid Lint: Reuse or Replace?

**Decision: keep, do not extend, and add a ratchet in front of it.**

| Existing piece | v1.6 treatment | Reason |
|----------------|----------------|--------|
| `vox-doc-pipeline/src/pipeline/mermaid.rs` (493 lines, regex repair, `SUPPORTED_DIAGRAM_HEADERS`) | Keep unchanged. Used by `lint.rs` (InvalidMermaid), `--fix` autofix in `mod.rs` (~line 384), `openwiki.rs`, `self_heal_bot.rs` | Still validates the 41 legacy fences until migrated; also serves as the **independent oracle** for the new Mermaid emitter (`vox-diagram` output must produce zero `validate_and_repair_mermaid` diagnostics - an integration test in doc-pipeline, no new edge direction) |
| `vox_validate_mermaid` + `observe_and_repair_turn_mermaid` in MCP | Keep | Agents still emit Mermaid into chat, PR bodies, non-doc files; advisory phase 1 says retain |
| `LintKind::InvalidMermaid` | Keep for baselined/legacy blocks | |
| New `LintKind::LegacyMermaid` | **Add** | Any non-archive ` ```mermaid ` fence not covered by the baseline is an error with a heal message pointing to `vox diagram import` |
| New `LintKind::InvalidDiagram { code, msg }` | **Add** | Parses every ` ```vox-diagram ` fence; line = fence line + diagnostic line |

### Where the "no new mermaid in docs/src" gate goes

In `vox-doc-pipeline` lint, not `vox ci`. Reasons: (1) it already owns fence scanning, the `--paths` scoped mode used by the fast pre-push tier, and the `docs-quality.yml` hard gate (`cargo run -p vox-doc-pipeline -- --lint-only`); (2) the exact precedent for a shrink-only ratchet is `facet_backlog.rs` with `--seed-facet-backlog` / `--tighten-facet-backlog` and a base-ref comparison that `vox ci pre-push` runs - copy that shape (`--seed-mermaid-baseline`, `--tighten-mermaid-baseline`, `contracts/documentation/mermaid-legacy-baseline.v1.yaml`, rows = `path -> count`); (3) a `vox ci` subcommand would have to re-implement fence scanning and add a `cmd_enums.rs` variant plus catalog row for no gain.

Sequencing the gate: land it **early** (seeded at 41, 26 file rows) so no new Mermaid creeps in while migration runs (agents are actively writing docs in this repo), then tighten to empty as files migrate. The final state is zero baseline rows; if any of the 2 sequenceDiagram / 1 stateDiagram-v2 / 1 gantt fences are not importable in v1.6, hand-translate them or leave exactly those rows in the baseline with a reason. Treat the seed like the facet backlog: a one-time, milestone-scoped seed; never re-seed to admit a later block. `docs/src/archive/` stays excluded exactly as `lint.rs` excludes it today.

Note the doctest runner is unaffected: `doctest.rs` and `lint.rs` compare the fence language to exactly `"vox"`, so ` ```vox-diagram ` is never compiled as Vox. The lint's fence regex already accepts `-` in info strings.

## CLI: `vox diagram`

Yes, add it. It is the human/agent and CI entry point, makes `vox_validate_diagram` testable without MCP, and is the migration tool (`import --write`). Keep it thin: all logic is in `vox_doc_pipeline::diagram` (re-export of `vox-diagram` plus the file-walking helpers), so the CLI adds no crate edge.

| Subcommand | Behaviour |
|------------|-----------|
| `vox diagram check <paths...|->` | `.md/.mdx` -> extract fences; otherwise whole file / stdin is one diagram. Non-zero on diagnostics; `--json` for the same shape as the MCP tool |
| `vox diagram fmt [--check]` | Canonical printer; in markdown rewrites fences in place |
| `vox diagram render <file> [-o out.svg] [--theme light|dark]` | Standalone SVG |
| `vox diagram export --format mermaid` | Sanitized Mermaid from VoxDiagram |
| `vox diagram import <file> [--write]` | Mermaid -> VoxDiagram text; `--write` replaces ` ```mermaid ` fences in markdown in place (the migration driver) |

### Registration checklist (verified from `vox grammar` as template)

1. `crates/vox-cli/src/lib.rs`: `Cli::Diagram { cmd: commands::diagram::DiagramCmd }` variant (see `Cli::Grammar` at ~line 502) and `commands/mod.rs`.
2. `crates/vox-cli/src/cli_dispatch/mod.rs`: match arm (see `Cli::Grammar` at ~line 625).
3. `contracts/operations/catalog.v1.yaml`: one `cli:` row per leaf path (`path: [diagram, check]` ...) with `status: active`, `handler_rust: commands::diagram::run`, `ref_cli_required: true`, `feature_gate: null` (see the `grammar` row at ~line 21300).
4. `crates/vox-cli/src/commands/command_registry_handler_needles.rs`: add `// commands::diagram` (the `command-compliance` gate greps handler paths in-tree).
5. `docs/src/reference/cli.md`: mention (required by `ref_cli_required`).
6. `contracts/gui/surface-registry.v1.yaml`: add `cli_group: diagram` row, `representation_tier: none` (effective tier derives to GenericForm in `gui_coverage_gate.rs`). Do **not** add a `coverage-exemptions.v1.yaml` entry: the ratchet file `coverage-exemption-ratchet.v1.json` caps unresolved exemptions at 14 and is shrink-only.
7. Regenerate: `vox ci operations-sync --target all --write`, then `vox ci command-sync --write` (updates `contracts/cli/command-registry.yaml`, `docs/src/reference/cli-command-surface.generated.md`); `gui-surface-coverage` / `gui-surface-registry` are `ssot-drift` stages.
8. `vox-cli` has `max_loc = 90_000`; keep `commands/diagram.rs` under ~150 lines.

## Data Flows

### Authoring and validation (agent loop)

```
Agent writes ```vox-diagram fence
   -> vox_validate_diagram (MCP)  ----------------> vox_diagram::check(source)
   <- diagnostics [{line,col,code,message,fix}]  <----/
   -> agent applies fix, re-validates (deterministic, no LLM repair round-trip)
Commit -> pre-push fast tier: vox-doc-pipeline scoped lint (InvalidDiagram, LegacyMermaid)
PR     -> docs-quality.yml: lint-only (hard gate) -> Build Starlight (render step inside prebuild)
```

### Site build

```
docs/src --(vox-doc-pipeline --render-diagrams)--> manifest (gitignored) --(remark plugin)--> inline SVG in HTML
                                                                     \--> miss/stale => build FAILS with exact command
```

### Migration (one-time)

```
vox diagram import --write docs/src/**  -> reviewed diff -> mermaid-legacy-baseline tighten
   verify per file: (a) vox diagram check ok, (b) node/edge set equal to original (import->Mermaid emit->compare), (c) site build renders
```

## Patterns to Follow

### Pattern 1: Pure core, thin shells
`vox-diagram` exposes pure functions over `&str` (`check`, `format`, `render_svg`, `to_mermaid`, `import_mermaid`, `fence::extract`). All IO (walking, manifests, MCP JSON, clap) lives in consumers. Layout is a pure `Ir -> Laid` function distinct from emission so STACK research can swap the layout algorithm/library without touching emitters.

### Pattern 2: Determinism as a tested contract
Same input must produce byte-identical SVG/Mermaid/canonical text (ordered maps only, fixed float formatting, no timestamps, no random ids). Enforce with `insta` golden snapshots plus a `proptest` property `parse(print(ast)) == ast`. Determinism is what makes the "not committed, regenerated per build" decision safe.

### Pattern 3: Ratchet, not flag day
Reuse the facet-backlog ratchet (seed once, only shrink, base-ref comparison in pre-push) so the Mermaid ban lands before migration completes.

### Pattern 4: Stable diagnostic codes with fix hints
`diagram/<kebab-code>` namespace (parallels `mermaid/*` in MCP); every diagnostic carries span + message + optional machine-applicable fix, shared verbatim by lint, CLI `--json`, and MCP.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Parsing with regexes
**Why wrong:** this is the failure the advisory documents for Mermaid. **Instead:** hand-written lexer + recursive-descent parser with spans; the importer also tokenizes (do not reuse the regex repair passes as an importer).

### Anti-Pattern 2: A third fence scanner / a copied validator in the MCP crate
**Instead:** one `fence` module in `vox-diagram`, used by lint, render, CLI, and MCP.

### Anti-Pattern 3: Committing rendered SVGs or wiring a drift stage
**Instead:** gitignored manifest, hard build error on miss. AGENTS.md explicitly warns against regenerate-after-merge commit patterns.

### Anti-Pattern 4: Growing god files
`pipeline/mod.rs` (884 lines), `agent_loop.rs`, `lib.rs` of vox-cli are at or over limits. Put new logic in new files; add only match arms/flags to the big ones.

### Anti-Pattern 5: Coupling to the compiler or Axis GUI in v1.6
No `vox-compiler` grammar/LSP edge, no GUI renderer. A pure L2 crate keeps the later Tauri IPC / compiler integration open without a re-layering.

### Anti-Pattern 6: Adding the crate edge by regenerating the baseline
Never run `crate-edges` baseline regeneration to admit the new edges; propose the exceptions and stop.

## Suggested Build Order

Dependencies drive the order: the exceptions must be authorized before any consumer edge compiles under CI; the parser/IR gates everything; emitters gate docs integration; importer gates migration; the ratchet should land before migration.

| # | Phase | Contents | Depends on | Notes / gates |
|---|-------|----------|------------|---------------|
| 0 | **Day-0: authorization** | Ask the user to approve the 2 exceptions (doc-pipeline, mcp) in one batch; decide re-export vs direct CLI edge | none | Blocking human step. Agents must not write `exceptions` |
| 1 | **Core crate** | Scaffold + all registrations (checklist above, `orphan_exempt = true` if no consumer yet), lexer/parser/AST, validate, diagnostics, IR, canonical printer, `fence` module. Tests first (Test-First Policy: `skeleton/untested-pub-api`) | 0 | Gate: `cargo check -p vox-diagram`, `cargo nextest run -p vox-diagram`, `vox-arch-check`, `crate-edges`. DSL grammar content is a FEATURES decision |
| 2 | **Emitters** | Layout, SVG (CSS-variable theme, title/desc), Mermaid emitter. Cross-check: Mermaid output passes `vox-doc-pipeline::mermaid` validator with zero diagnostics | 1 | Text-measurement/label-clipping risk goes to PITFALLS. Snapshot goldens |
| 3 | **Doc-pipeline consumer** (+ remove `orphan_exempt`) | `diagrams.rs`, `LintKind::InvalidDiagram`, `--render-diagrams`, `pub use vox_diagram as diagram`; `self_heal_bot` check | 1, 2, exception #1 | Touch `mod.rs`/`lint.rs`/`types.rs` minimally |
| 4a | **Legacy-Mermaid ratchet** | `mermaid_gate.rs`, `LintKind::LegacyMermaid`, seed at 41 / 26 files, `--tighten-mermaid-baseline`, pre-push base-ref comparison | 3 | Land before migration so nothing grows meanwhile |
| 4b | **MCP tool** (parallel with 4a/4c) | `diagram_tools.rs`, dispatch/schema, operations catalog row, projections, fixtures, profile, generated docs | 1, exception #2 | Use the 10-row checklist |
| 4c | **CLI** (parallel) | `vox diagram check|fmt|render|export` first (`import` after 5), catalog rows, needles, cli.md, GUI surface-registry row | 3 | Regenerate with `operations-sync` then `command-sync` |
| 5 | **Mermaid importer** | Flowchart/graph first (37 of 41 fences); decide sequence/state/gantt (4 fences) import vs hand-translate | 1, 2 | Fidelity test over the real 41 fences (node/edge-set equality via the Mermaid emitter round trip) |
| 6 | **Docs-astro integration** | `render-diagrams.mjs`, `remark-vox-diagram.mjs`, config/package/lockfile, CSS, unit test (`tests/unit`), build-output spec, workflow path filters | 2, 3 | **Spikes:** hChildren on `index.mdx`; `llms-full.txt` retains fence source; Expressive Code non-interference |
| 7 | **Migration** | `vox diagram import --write` over 26 files, human review, tighten baseline to empty (or documented residue); flip the home page fence first as the smoke test | 4a, 5, 6 | Docs frontmatter/KB-facet rules apply to any new doc; never touch `docs/src/archive/` |
| 8 | **Hardening / follow-through** | `openwiki.rs` generators emit `vox-diagram` from IR; DSL reference page under `docs/src/reference/` (its examples dogfood the renderer; use a "show source" option or `<details>`); update AGENTS.md "Markdown Hygiene" and advisory section 7 to prefer ` ```vox-diagram `; mark advisory `status`; optional `post_verification.rs` branch | 6, 7 | Docs changes need frontmatter + `kb:summary` per AGENTS.md |

Parallelism: after phase 3, 4a/4b/4c are independent and can be separate worktrees (one agent per worktree). Phase 5 can start as soon as phase 2 lands and run alongside 3-4.

## Research Flags for Later Phases

| Phase | Flag | Reason |
|-------|------|--------|
| 1 | Deeper research (FEATURES/STACK) | DSL surface: which diagram kinds (flow/architecture only vs sequence/state/gantt), containment, edge labels, themes. Importer scope in phase 5 follows from this |
| 2 | Deeper research | Layout algorithm choice (hand-rolled layered layout vs a crate) and deterministic text measurement without font metrics |
| 6 | Needs spikes | Astro 6 + MDX hChildren, `starlight-llms-txt` source retention (MEDIUM confidence items) |
| 4b/4c | Standard patterns | Fully mapped from `vox_validate_vuv`/`grammar` precedent; run the generators, do not hand-edit generated files |
| 4a | Standard patterns | Direct copy of `facet_backlog.rs` shape |

## Scaling Considerations

| Concern | Now (41 fences) | Growth |
|---------|-----------------|--------|
| Render time | Milliseconds in one process | Linear; memoize by source inside the manifest builder if fences exceed thousands |
| Manifest size | Roughly 41 x 10-30 KB SVG | Compress or switch to per-diagram files only if it exceeds a few MB |
| Crate budget | `max_loc` baseline only (guard is `off` for hard LoC) | File-size limit (500 lines) is the real constraint; plan modules |
| Fan-in | 2 direct consumers | Stays below any fan-in budget; Axis GUI later adds one L4/L5 edge (needs its own exception) |

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `vox-diagram` <-> `vox-doc-pipeline` | Direct fn calls; re-export `diagram` | Exception #1; doc-pipeline owns walking/manifest/lint |
| `vox-diagram` <-> `vox-orchestrator-mcp` | Direct fn calls | Exception #2; JSON shaping stays in MCP |
| `vox-cli` <-> `vox-doc-pipeline` | Existing edge, new `diagram` module | No new edge |
| `vox-doc-pipeline` binary <-> `docs-astro` | CLI invocation from `prebuild`, JSON manifest | New pattern; binary located via `VOX_DOC_PIPELINE_BIN`, then `target/{ci,debug,release}/`; no implicit `cargo` |
| manifest <-> remark plugin | Exact normalized-source key | Normalization defined in `vox_diagram::fence::normalize`, mirrored in JS and unit-tested on both sides |
| operations catalog <-> MCP/CLI/capability registries | `vox ci operations-sync` projections | Catalog is SSOT; do not edit projections by hand |

## Open Questions

- Does the user accept the re-export (CLI via `vox_doc_pipeline::diagram`) or want a direct `vox-cli -> vox-diagram` edge (3 exceptions)?
- Does `vox ci profile-parity` require new public MCP tools to appear in a profile? (`vox_validate_mermaid` is in none; `vox_validate_vuv` is in `lang`.)
- Is a published JSON Schema (`contracts/diagrams/diagram.v1.schema.json`, advisory phase 2) wanted in v1.6? Deriving `Serialize`/`Deserialize` on the IR is free and makes `check --json` and snapshots easy, but a schema file adds a `contracts/index.yaml` entry and a parity gate. Recommendation: defer.
- Dev-loop fidelity: acceptable that `astro dev` needs a manual re-render after editing a fence?

## Sources

All HIGH confidence unless noted; every path was read in this session.

- `/Users/brbrainerd/dev/vox/docs/src/architecture/layers.toml` (layer definitions, guards, `[crates]`, orphan/WTL rules)
- `/Users/brbrainerd/dev/vox/docs/src/architecture/where-things-live.md` (L2 table, "Add an MCP tool" row)
- `/Users/brbrainerd/dev/vox/docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md`
- `/Users/brbrainerd/dev/vox/crates/vox-cli-ci/src/crate_edges.rs`, `/Users/brbrainerd/dev/vox/contracts/ci/crate-edges.allow.v1.json` (rules; `vox-kb-facets` exception precedent)
- `/Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/{Cargo.toml,src/main.rs,src/pipeline/{mod,lint,mermaid,doctest,facet_backlog,openwiki}.rs,src/claims/self_heal_bot.rs}`
- `/Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/{dispatch,input_schemas,gui_registry_tools}.rs`, `chat_tools/chat/mermaid_repair.rs`, `tests/fixtures/dev-advertised-tools.default-features.txt`
- `/Users/brbrainerd/dev/vox/contracts/{operations/catalog.v1.yaml,mcp/tool-registry.canonical.yaml,mcp/http-read-role-governance.yaml,capability/capability-registry.yaml,profiles/lang.v1.yaml,gui/surface-registry.v1.yaml,gui/coverage-exemptions.v1.yaml,gui/coverage-exemption-ratchet.v1.json}`
- `/Users/brbrainerd/dev/vox/crates/vox-cli/src/{lib.rs,cli_dispatch/mod.rs,commands/grammar.rs,commands/command_registry_handler_needles.rs,commands/ci/gui_coverage_gate.rs}`, `crates/vox-cli-ci/src/cmd_enums.rs`
- `/Users/brbrainerd/dev/vox/docs-astro/{package.json,astro.config.mjs,scripts/setup-content.mjs,src/plugins/*.mjs,tests/}`
- `/Users/brbrainerd/dev/vox/.github/workflows/{docs-deploy.yml,docs-quality.yml}`
- `/Users/brbrainerd/dev/vox/docs/superpowers/plans/2026-08-22-mermaid-rendering-and-parse-gate.md` (superseded client-side plan; documents the `llms-full.txt` and no-Chromium constraints)
- `/Users/brbrainerd/dev/vox/docs/src/index.mdx` line 87 (home page mermaid fence); fence census run over `docs/src` excluding `archive/`
- MEDIUM/unverified: Astro 6 / MDX handling of `data.hChildren`, `starlight-llms-txt` raw-body behaviour, Expressive Code vs user rehype ordering (flagged as phase 6 spikes)

---
*Architecture research for: VoxDiagram integration (v1.6)*
*Researched: 2026-10-16*
