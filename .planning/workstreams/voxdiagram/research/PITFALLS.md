# Pitfalls Research: VoxDiagram (native diagram DSL + layout + SVG + docs migration)

**Domain:** Adding a text-DSL -> typed IR -> layout -> deterministic SVG pipeline (plus Mermaid export/import, MCP validator, docs-site pre-render, and corpus migration) to the existing Vox Rust workspace
**Researched:** 2026-10-16
**Confidence:** MEDIUM-HIGH (repo-specific traps and the advisory verdict are HIGH, verified against the repo and primary papers; layout and text-measurement guidance is MEDIUM, from established practice with thin web confirmation; LLM-on-novel-DSL guidance is MEDIUM)

Confidence tags used below: [HIGH] = primary source or repo fact read directly; [MEDIUM] = search-engine-summarised secondary source or well-established practice; [LOW] = unverified or inferred.

---

## 0. Verdict on the advisory's quantitative claims (read this first)

The advisory (`docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md`, section 4 matrix, line 141) asserts LLM syntax error rates of Mermaid 18-35%, D2 4-8%, PlantUML 5-10%, Graphviz DOT 2-5%, Schema IR <1%, and says section 2 is based on "empirical testing across frontier LLMs (GPT-4o, Claude 3.5 Sonnet, Gemini Pro/Flash) and academic benchmarks (MermaidSeqBench, DiagramEval)". The doc is `authorship_model: gemini-3.5-flash`, `status: research`, and the numbers appear exactly once, in that table, with no citation, method, model list, or sample size.

**Verdict: REFUTED as stated. The numbers are unsourced and the two named benchmarks do not contain them. Do not cite them, and do not make any success criterion depend on them.**

| Claim | What the cited source actually says | Verdict |
|---|---|---|
| MermaidSeqBench supports Mermaid error rate 18-35% | arXiv 2511.14967 (IBM Research, NeurIPS 2025 workshop). 132 NL-to-Mermaid **sequence-diagram** samples (10 hand-written seeds + 30 verified LLM-generated + rule-based augmentation). Judged by DeepSeek-V3 and GPT-OSS-120B on six criteria scored by an LLM judge. Reports a "Syntax" **score** (0-100, judge-assigned), not an error rate: for six small models (Qwen2.5 0.5B/7B, Llama 3.1/3.2 1B/8B, Granite 3.3 2B/8B) Syntax scores range 39.6 to 92.0, and the two judges disagree by up to ~30 points on the same model (e.g. Granite 3.3-2B: 75.27 vs 39.60). The string "18-35%" does not appear. No comparison to D2, PlantUML or DOT (PlantUML is mentioned only as future work). Tested **no** frontier model. [HIGH, fetched full text] | Does not support |
| DiagramEval supports it | arXiv 2510.25761 (UIUC, EMNLP 2025). Evaluates LLM-generated **SVG** diagrams (not Mermaid) by converting them to graphs; metrics are node/path alignment F1 (Node F1 0.33-0.35, Path F1 0.20-0.24 for Llama 4 Maverick / Gemini 2.5 Pro / Claude 3.7 Sonnet on a 361-item CVPR dataset). Reports no syntax/parse error rates for any DSL and never mentions Mermaid, D2 or PlantUML. [HIGH, fetched full text] | Does not support; wrong format |
| D2 4-8%, DOT 2-5%, Schema IR <1% | No published head-to-head found for D2 or DOT at all. A search for a four-way comparison found none; the closest PlantUML datum is a 2024 image-to-UML study (arXiv 2404.11376) that counted PlantUML **compile failures that grew with class-diagram complexity**, contradicting "LLMs rarely make syntax errors on PlantUML". [MEDIUM] | No evidence; one contrary datum |
| "Mermaid is fail-stop and fragile" (qualitative) | Directionally supported by practitioner reports: Cline issue #4046 (Gemini 2.5 puts HTML tags in labels), Roo-Code issue #4636, Microsoft GenAIScript "Mermaids Unbroken" (parse error from one misplaced arrow; model repaired it from the error message in one round; one anecdote, no rate). All anecdotal. [MEDIUM] | Plausible, unquantified |
| Documentation/grammar cards fix novel-syntax errors | NeurIPS 2026 workshop poster "Documentation Helps, But Gaps Remain" (DIAGRAM-FAIL-BENCH, 630 Mermaid tasks over 21 diagram types, 3 model families): docs improve render rates but **do not close the gap between "renders" and "meets the task requirements"**. Black-box verifier iteration moved final executable success 51.43% -> 60.00% on a 70-task stress subset; a facet-aware verifier-guided repair method (RA-RAG) reached 95.71% (descriptive, not causal, per the authors). Workshop poster, no arXiv. [MEDIUM-LOW] | Most useful finding: "renders" is not "correct" |
| OmniDiagram: syntax errors in PlantUML/Mermaid are stagnant across model scale | arXiv 2604.05514; only seen via a search-engine summary, PDF unreadable in tooling. [LOW, unverified] | Do not cite until read |

Additional specifics the roadmap should record:

- The advisory lists TALA as part of D2's "superior layout" edge. TALA is now open source (MPL-2.0, same as D2) but ships as a separate binary, not bundled; D2 bundles dagre and ELK. TALA's blog notes randomness unless seeded. [MEDIUM, d2lang.com/blog/tala-is-open-source]
- The advisory says D2 has a "Rust parser exists". Not verified; do not rely on it.
- The 18-35% number is irrelevant to VoxDiagram's actual risk. Even if true for an established language with abundant training data, it says nothing about a **brand-new** DSL with zero training data. The relevant unknown is VoxDiagram's own first-try-valid rate, which nobody has measured (Pitfall 2).

**Recommended handling:** (a) add a roadmap task to strike or annotate the table in the advisory with "UNSOURCED - superseded by measurement in Phase 4" (a roadmap task, since this research may not edit it); (b) restate the milestone's justification on defensible grounds - determinism, no runtime CDN dependency, pre-rendered output, theming, accessibility, control over validation - none of which need the numbers; (c) define the success criterion as a **measured non-inferiority** result on the same harness (Pitfall 2). The only in-house data source is the `MermaidRepair` telemetry category in `crates/vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs`; if events exist, mine them for a real baseline repair rate.

---

## Critical Pitfalls

### Pitfall 1: Building the milestone's justification and success criteria on the unsourced advisory numbers

**What goes wrong:** Requirements, ROADMAP success criteria, or the docs ("VoxDiagram cuts diagram errors 4x") quote 18-35% vs 4-8%. A later honest measurement shows VoxDiagram's first-try validity is *worse* than Mermaid's for the same models (very plausible for an unseen DSL), and the milestone appears to fail or, worse, the claim ships and is later refuted.
**Why it happens:** The advisory is a Vox-authored SSOT-looking doc in `docs/src/architecture/`, authored by an LLM, so it reads as established fact.
**How to avoid:** See section 0. Treat the advisory's matrix as a hypothesis. Success criterion = on a shared task set, VoxDiagram **post-repair** valid-and-semantically-correct rate >= Mermaid's, and first-try rate reported honestly with CIs, whatever it is.
**Warning signs:** Any REQUIREMENTS.md line or doc sentence containing a percentage with no link to a measurement artifact in `.planning/` or `docs/`.
**Phase to address:** Phase 0 (decision record) and Phase 4 (measurement).

---

### Pitfall 2: The "LLMs have never seen VoxDiagram" problem handled by hope instead of design and measurement

**What goes wrong:** Agents asked to draw a diagram emit Mermaid or D2 dialect inside a `vox-diagram` fence (`A[Label] --> B`, `subgraph ... end`, `a -> b: label` without quotes). Parse failures cost repair turns, tokens, and trust; humans stop using the format. Separately, output that parses can still be wrong (nodes missing, edges reversed): the DIAGRAM-FAIL-BENCH finding that docs improve renderability but not task compliance.
**Why it happens:** Zero pretraining data. Research on very-low-resource languages (Mora et al., NeurIPS 2024, "Synthetic Programming Elicitation") confirms models "struggle to compose syntactically valid programs in languages unrepresented in pre-training"; the effective levers are in-context grammar/examples, targeting an intermediate language the model already knows, and compiler feedback in a repair loop. [MEDIUM, search summary]. Grammar Prompting (Wang et al., NeurIPS 2023, arXiv 2305.19234) found that merely prepending the full BNF to the examples gave **no improvement** in preliminary tests; gains came from **per-example minimal grammars**, because BNF as a metalanguage is familiar even when the DSL is not. [HIGH on the paper's claim; transfer to this DSL is untested]
**How to avoid (design):**
1. Make the surface syntax a *conservative intersection* of what models already write fluently: `a -> b: "label"`, braces for containment, `key: value` attributes, **always-quoted labels**, one statement per line. "Close to D2" is a bet, not a fact: D2 is itself far less represented in training data than Mermaid or DOT, so do not assume D2-likeness buys fluency; measure it.
2. Ship a **grammar card**: a short (target <= ~600 tokens) cheat-sheet of 4-6 canonical examples (flow, container, labeled edge, direction, style) plus a 10-line "never do this" list covering the exact dialect-bleed forms (Mermaid `A[Label]`, `-->`, `subgraph/end`). Prefer examples over a full BNF dump (the grammar-prompting result). Expose it via the MCP tool description and the `validate` response so it is always in context at the moment of failure.
3. **Lenient parse, strict emit.** The parser should recognize the common Mermaid/D2 dialect forms and return a *structured diagnostic with the exact canonical rewrite* (`{"code":"mermaid_bracket_label","span":...,"suggested":"a: \"Label\""}`), not just "unexpected token". This turns repair into a mechanical edit that even a small model can do (cf. SLMFix, arXiv 2511.19422: fixing is easier than generating). Also provide a deterministic `--fix` that applies unambiguous rewrites without any LLM call.
4. Bounded repair loop (max 2 rounds), diagnostics machine-readable with line/col spans, routed through the `vox_actor_runtime::llm` facade only (AGENTS.md boundary), no hardcoded vendor hostnames, no pinned model versions.
5. Optionally accept a JSON/YAML structured form (the advisory's Schema IR) as a second input for agents that prefer constrained output; it removes syntax risk but not semantic risk, so do not claim "near zero error" without measuring.
**How to measure (concrete):**
- Build a task set from the existing corpus: the 40 non-archive Mermaid diagrams (once imported to golden IR) each get an NL description; add ~20-40 synthetic tasks. Target >= 60 tasks, ideally 100+.
- Arms: VoxDiagram (grammar card + diagnostics), Mermaid (given an *equally careful* quoting-rules card, otherwise the comparison is rigged), optionally D2-style. Same models via the facade, model ids recorded at run time (no pinning), same temperature and prompt budget.
- Metrics: first-try parse-valid; valid after k repair rounds; **semantic score** = node-set and edge-set F1 vs golden IR (DiagramEval-style node/path alignment); tokens and latency per accepted diagram.
- Syntax oracle = the deterministic parser (and for Mermaid, the real `mermaid.parse`), **never an LLM judge**: MermaidSeqBench's judges disagree by ~30 points.
- Statistics: paired design, report Wilson 95% CIs. Sample size for two independent proportions (alpha 0.05, power 0.8): 25% vs 6% needs ~56 per arm; 15% vs 5% needs ~140 per arm; a ~60-task set can only detect a large effect, so say so.
**Warning signs:** The repair loop's median rounds > 1.5; agents begin emitting ```mermaid again "because it works"; semantic F1 well below parse-valid rate.
**Phase to address:** Phase 1 (diagnostic shape, canonical rewrite suggestions), Phase 4 (grammar card, repair loop, harness), Phase 0 (agree on the metric before building).

---

### Pitfall 3: Non-deterministic output breaks snapshot tests and `ssot-drift` (works on the Mac, fails on hosted Linux CI)

**What goes wrong:** SVG differs run-to-run or machine-to-machine: tests flake, snapshot files churn, and the drift gate fails only in CI (hosted Linux), the one place that cannot be debugged locally.
**Why it happens (all verified or standard):**
- `std::collections::HashMap`/`HashSet` iteration order is randomized per instance (`RandomState`); any layout step that iterates one (node adjacency, layer buckets, barycenter tie-breaks, class/id emission) is order-nondeterministic. Different `HashMap` instances in the same process can iterate differently, so even an in-process "run twice, compare" test catches it.
- **Float transcendental functions are explicitly non-deterministic across platforms.** Rust docs for `f64::sin` (and cos/exp/powf/ln...): "The precision of this function is non-deterministic. This means it varies by platform, Rust version, and can even differ within the same execution from one invocation to the next." [HIGH, doc.rust-lang.org/std/primitive.f64.html]. Any curve routing that uses `sin/cos/atan2/powf/exp` can emit different last-digit coordinates on macOS vs Linux. Plain `+ - * /` and `sqrt` are IEEE-exact and safe.
- Float-to-string: `{}` of `f64` can print `0.30000000000000004`; negative zero prints `-0`; locale is not an issue in Rust but ad-hoc formatting often is elsewhere.
- Sort ties: `sort_unstable_by` with equal keys gives implementation-defined order; barycenter ties are *common*.
- Parallelism: `rayon` (already a doc-pipeline dep) inside layout reorders float reductions.
- Tool-version drift: generated SVG embeds nothing like a version or timestamp (do not add `<!-- generated by ... vX -->` with a build number; `vox-build-meta` injects `+build.N (GITHASH)` on *every commit*, which would make every commit dirty the output).
**How to avoid:**
1. In the `vox-diagram` crate use `BTreeMap`/`BTreeSet`/`Vec`-with-explicit-sort or an insertion-order map keyed by **stable node ids derived from declaration order**; ban `HashMap` in the crate (review rule + a grep test). Break every tie by declaration index. `sort_by` (stable) only.
2. Quantize: all geometry in integer units (or fixed-point 1/64 px), stringified from integers; no transcendental functions in routing (use polylines/orthogonal segments with rounded corners built from `+ - * /`/`sqrt`); no `-0`.
3. No timestamps, versions, or absolute paths in output. Pin the font table (Pitfall 5) in-crate so output cannot depend on the host.
4. Tests: (a) permutation test - shuffle node/edge declaration order *and* HashMap-free ids, assert identical layout where semantics say it should be (or document that declaration order is intentionally significant and test *that*); (b) run layout N=20 times in one process and compare bytes; (c) a golden-bytes snapshot (`insta`, already a workspace dep) for ~10-15 canonical diagrams, run on both macOS dev and Linux CI before merge; (d) property tests (`proptest`, in workspace) for geometric invariants rather than snapshotting every case.
5. Wire the docs-build output into the drift story deliberately (Pitfall 7) rather than letting it become an accidental SSOT file.
**Warning signs:** A snapshot that differs only in the 3rd decimal; a test that passes alone and fails in the full run; `ssot-drift` red on CI, green locally.
**Phase to address:** Phase 2 (layout/SVG), enforced from the first commit of the layout module.

---

### Pitfall 4: Compound/nested graph layout and edge-label placement underestimated (the "write a layout engine" trap)

**What goes wrong:** A flat layered layout looks fine in demos; then real docs diagrams (containers, cross-container edges, labelled edges) produce overlapping labels, edges crossing through unrelated containers, containers that don't enclose their children, or layouts that are valid but unreadable. The phase balloons into reimplementing ELK.
**Why it happens:** Facts about the real corpus: of the 40 non-archive diagrams, 11 use `subgraph`; four have >= 3 subgraphs (largest: `architecture/axis-harness-river-design-2026.md` with 4 subgraphs and 35 source lines); 22 use `&` fan-out; edges routinely cross container boundaries. Compound layout is where layered layout gets hard: Graphviz `dot` can only clip an edge to a cluster boundary (`lhead/ltail`, `compound=true`) when the real endpoint node is *inside* that cluster and cannot point an edge at a cluster itself [MEDIUM]; ELK needs `hierarchyHandling=INCLUDE_CHILDREN` to lay out cross-hierarchy edges in one pass and then must merge hierarchical ports (`mergeHierarchyEdges`), and the setting only matters across >= 2 successive hierarchy levels [MEDIUM]; ELK ports of this design are specialised, and one port (elk-dart) documents edges crossing separated hierarchy boundaries returning with unpositioned labels [MEDIUM]. Crossing minimization is NP-hard, so every implementation is a heuristic (barycenter/median sweeps with a capped iteration count) and quality depends on tie-breaking and iteration caps.
**How to avoid:**
1. **Time-boxed layout spike before committing to a strategy** (Phase 2, first plan): evaluate (a) a minimal in-house Sugiyama with recursive container layout (lay out each container as a unit, treat as a super-node in the parent), (b) an existing Rust crate (the workspace has only `petgraph`; any new crate like `layout-rs` or `rust-sugiyama` needs a license/dep review, a `cargo-deny` pass and likely hakari update, and none are known to do compound graphs well) [LOW on their compound support]. Decision criterion: render all 40 real diagrams acceptably, not toy graphs.
2. Constrain the DSL where the corpus allows: **container depth <= 2** (the advisory's own agent rule 3 already says this) and reject deeper nesting with a diagnostic rather than producing a poor layout.
3. Edge labels are **first-class layout objects**: standard technique is to insert each label as a dummy node in the mid-rank so it reserves space during layering and ordering; placing labels afterwards on a finished routing guarantees collisions.
4. Cross-container edges: route to the container boundary at a port, not through sibling content; define behavior for an edge whose endpoints are in different containers up front.
5. Give authors an escape hatch: optional `order`/`rank` hints so a badly laid-out diagram can be fixed in source instead of by changing the engine.
6. Define acceptance as invariants, tested by proptest and a checker: no node-node overlap, no label-node/label-label overlap, children inside container bounds with padding, all edge endpoints on node boundaries, layer monotonicity along edge direction (except flagged back-edges).
**Warning signs:** Layout code approaching ~2k lines before the first real doc renders; special cases keyed on specific diagrams; a "TODO: labels" in the router.
**Phase to address:** Phase 2 (spike and decision at the start; flag for deeper phase research).

---

### Pitfall 5: Text measurement without a browser: font drift, CJK, emoji, wrapping

**What goes wrong:** Node boxes are sized from guessed text widths. In the reader's browser the text is wider than the box (clipped or overflowing) or the boxes are loose and ragged; CJK and emoji labels are badly mis-sized; long labels never wrap.
**Why it happens:** The site does **not** use a web font: Starlight's default `--sl-font-system` is a system stack (`-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', ...`) [HIGH, read in `docs-astro/node_modules/@astrojs/starlight/style/props.css`]. A server-side measurement against any one font is wrong for most readers; there is no single "site font" to match. Inline SVG `<text>` also inherits whatever font the page CSS gives it. `textLength` "fixes" fit but distorts glyphs. Emoji render from a platform color-emoji font with unknowable metrics; CJK glyphs are ~1em wide vs ~0.5em for Latin.
**How to avoid:**
1. Decide the font policy explicitly: either (a) **ship and pin one font** for diagram text (small subset woff2 via `@font-face`/fontsource, loaded by the site, with the *same file's advance widths* embedded in `vox-diagram`), making measurement exact; or (b) keep the system stack and measure with a deliberately **conservative width table** (widest of common sans fonts) plus 8-12% safety margin and generous padding. Recommend (a) if diagrams are a headline feature, (b) for speed. Either way the font table lives in the crate and is part of deterministic output.
2. Measure per grapheme/char with a table: Latin advance from the font table; East Asian wide/fullwidth = 2 cells (needs `unicode-width`, which is **not** in the workspace today, so it is a new dependency); emoji sequences (`unicode-segmentation` is in the workspace) = fixed 2em box. Do not attempt shaping/kerning/ligatures (rustybuzz/cosmic-text) - overkill for diagram labels.
3. Wrap deterministically at a max label width using Unicode line-break opportunities (UAX #14) or at least word boundaries; never rely on the browser to wrap SVG text.
4. Avoid renderer-dependent text attributes: `dominant-baseline`/`alignment-baseline` differ between Chromium/Firefox/Safari; use explicit `dy` offsets and `text-anchor` only.
5. **Verify in a real browser**: the docs-astro Playwright suite already exists - add a test that, for every rendered diagram, asserts each `<text>`'s `getBBox()` fits inside its node rect with padding, run under at least two font stacks (system and a wide fallback like DejaVu Sans on Linux CI).
6. Also guard the aspect ratio: an LR diagram with 8+ ranks scaled by `max-width:100%` into Starlight's content column shrinks text below legibility. Compute effective scale at the content-column width and either re-flow direction, wrap, or provide a scroll container when scale < ~0.6.
**Warning signs:** Labels touching/overrunning box edges on Windows or Linux but not macOS; screenshot diffs differing only by font.
**Phase to address:** Phase 2 (measurement table, wrapping), Phase 3 (font loading policy, Playwright overflow test).

---

### Pitfall 6: Governance traps - new crate and new dependency edges are USER-AUTHORIZED-ONLY, and tool registration touches ~7 contracts

**What goes wrong:** An executor creates `crates/vox-diagram`, adds `vox-orchestrator-mcp -> vox-diagram` and `vox-doc-pipeline -> vox-diagram`, and `vox ci crate-edges` fails (exact edge-set ratchet). The executor then "fixes" it by regenerating the baseline or writing an `exceptions` entry, which AGENTS.md forbids. Or the milestone stalls mid-phase waiting for approval.
**Why it happens:** `contracts/ci/crate-edges.allow.v1.json` holds 539 allowed edges and 47 exceptions, each exception with `authorized_by: "user (explicit chat approval, date)"`. Precedent in `docs/src/architecture/rust-build-loop-ssot-2026.md`: the `vox-orchestrator-models` extraction was blocked until the owner approved, in chat, an exact list of the layer row and edges ("Task B0"), and an edge outside the list *stopped the work*. New crates also need: a `layers.toml [crates]` row (layer + optional `max_dependents`/`max_loc`), a `where-things-live.md` row, a `workspace-hack` dependency and hakari consistency (`.config/hakari.toml`), and fan-in baseline rows.
**How to avoid:**
1. Make **Phase 0 an authorization checkpoint**: enumerate every edge up front and request one explicit approval. Minimal set to propose: `vox-diagram` (pure, no async, no DB; propose layer 1 or 2) with edges only to leaf crates it truly needs; consumers `vox-orchestrator-mcp -> vox-diagram` (L3 -> down, for the MCP tool) and a CLI/CI consumer. Prefer putting the **docs gate and `vox-diagram` fence validation in `vox-cli-ci` / `vox-cli` (L5)** rather than adding `vox-doc-pipeline -> vox-diagram`; the pipeline's existing Mermaid extractor can be reused for fence discovery without the edge.
2. Pick the crate name once. The advisory mentions both `vox-diagram-core` and `vox-diagram`; the milestone says `vox-diagram`. One crate, not two (two doubles the edges and rows to authorize).
3. Do **not** home the DSL in `vox-compiler` (layer 3, `max_loc = 45_000`, 21 dependents); the advisory's "Phase 3" (compiler keyword + LSP + Axis Canvas) is out of this milestone. AGENTS.md Grammar Unification also forbids new bare keywords for behavior expressible as decorators, so a `.vox`-level `diagram` keyword needs separate design anyway.
4. Reuse before adding dependencies: `quick-xml`, `pulldown-cmark`, `unicode-segmentation`, `petgraph`, `insta`, `proptest`, `serde` are already workspace dependencies. New ones (`unicode-width`, a font-metrics parser) need `cargo-deny` and hakari churn - justify each.
5. **Do not hash with `sha2`/`blake3` directly** (cryptography policy; detector `vox/crypto/banned-crate-import`). If a stable id suffix is needed, take a caller-supplied prefix (Pitfall 11) or go through `vox-crypto` (L1) - which would be one more edge to authorize. Prefer no hashing.
6. MCP tool registration (`vox_validate_diagram`) is not just Rust: precedent `vox_validate_mermaid` appears in `crates/vox-orchestrator-mcp/src/{input_schemas.rs,dispatch.rs,post_verification.rs,chat_tools/chat/mermaid_repair.rs}` and in contracts `contracts/mcp/tool-registry.canonical.yaml`, `contracts/mcp/http-read-role-governance.yaml`, `contracts/operations/catalog.v1.yaml`, `contracts/capability/capability-registry.yaml`, plus generated `capability-index.v1.json`, `model-manifest.generated.json`, `crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts`, `docs/src/reference/mcp-tools.generated.md`. The GUI `gui-coverage-gate` (349/349 MCP tools covered or audited) will fail for a tool with no coverage/audit entry. Plan the tool as a checklist grepped from the `vox_validate_mermaid` precedent, then `vox ci command-sync` for the generated files.
**Warning signs:** `crate-edges` red; `hakari verify` red; `gui-coverage-gate` red; any commit touching `crate-edges.allow.v1.json` `exceptions`.
**Phase to address:** Phase 0 (authorization), Phase 1 (crate creation), Phase 4 (MCP tool registration).

---

### Pitfall 7: Docs-site pre-rendering changes the build graph (Rust binary inside an Astro build) and coexists badly with the existing client-side Mermaid

**What goes wrong:** (a) `docs-astro` build now needs a built `vox`/`vox-diagram` binary or WASM, making the Pages deploy slow or broken; hosted CI caps jobs at 30 min and Rust caches are saved from `main` only, so a cold compile in the docs job is expensive. (b) The pre-render is invoked through an ad-hoc `.mjs` shell-out that nobody type-checks. (c) The old client-side Mermaid loader is left in place, so two pipelines render two ways, or it is removed and 29 archived diagrams silently degrade. (d) Rendered SVG gets committed, creating 40 more generated-drift artifacts and bloating review.
**Why it happens:** Current rendering is entirely client-side: `docs-astro/src/components/PageSidebar.astro` dynamically imports `https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs` (unpinned major, runtime CDN, `securityLevel: 'strict'`), and silently falls back to the code block on failure. The CI Contract prohibits raising job caps; "VoxScript-first" forbids new `.sh`/`.py` glue.
**How to avoid:**
1. Choose the integration point once, before building: **remark/rehype plugin in `docs-astro` that shells out to a prebuilt `vox` (from `voxup`/release artifact) with a content-keyed cache**, running before Expressive Code so the fence is replaced by inline SVG while the *source stays in the page* (llms.txt/Pagefind/source-view keep readable text). Do not compile Rust inside the docs deploy job. Alternative worth costing: WASM build of the crate loaded by the plugin. Mark this decision for deeper research.
2. Plugins live in `docs-astro/src/plugins/*.mjs` today; prefer `.ts` for new code if the toolchain allows (house rule: TypeScript only in TS repos), and keep the shell-out thin.
3. Do not commit rendered SVG. If a committed artifact is wanted for GitHub/README, it is the **Mermaid export** or a generated SVG registered with the SSOT drift machinery (mark `linguist-generated` in `.gitattributes`; add the generator to `run_ssot_drift`; never hand-regenerate - `ssot-autoregen` owns it).
4. Decide the archive explicitly: `docs/src/archive/` holds 29 Mermaid blocks in 28 files and is tombstoned ("do not read, ingest, or modify"). Removing the CDN loader degrades those pages to a visible code block; that is probably acceptable, but it must be a recorded decision, and the gate must exempt `archive/`.
5. Keep the failure mode graceful: a bad diagram must not fail the entire site build silently *or* fatally - fail the build in CI for non-archive pages (it is a source error), show the source block with an error banner at runtime.
**Warning signs:** `docs-astro` deploy time jumps; a `cargo` invocation appears in the docs workflow; both `.vox-mermaid-diagram` and new SVG containers exist on one page.
**Phase to address:** Phase 3.

---

### Pitfall 8: "Sanitized Mermaid export" is not automatically valid Mermaid

**What goes wrong:** The exporter emits Mermaid that GitHub or the site renders as a red syntax-error box - the very failure the milestone exists to eliminate.
**Why it happens:** Mermaid has documented traps even with quoting: the lowercase word `end` as a node id/label breaks flowcharts ("capitalize the entire word or any of the letters"); a node id beginning with `o` or `x` after an edge (`dev---ops`) is parsed as a circle/cross edge terminator; "troublesome characters" need quotes, and `"` itself inside a quoted label needs the `#quot;` entity; `<`/`>`/`&` and `<br/>` are interpreted as HTML in default `htmlLabels` mode; markdown-string backticks have their own meaning [HIGH, mermaid.js.org/syntax/flowchart.html for the first three]. Nested subgraphs and edges to subgraphs have layout restrictions. The existing validator `crates/vox-doc-pipeline/src/pipeline/mermaid.rs` (493 lines) is regex/heuristic - it cannot be the oracle for the exporter.
**How to avoid:**
1. Export **opaque generated ids** (`n0`, `n1`, ... in declaration order) and put *all* text in quoted labels; never reuse user ids as Mermaid ids. Entity-encode `"` (`#quot;`), `<`, `>`, `&`, and newlines (`<br/>` or wrapped lines). Strip control chars.
2. Use the **real Mermaid parser as the test oracle**: in `docs-astro` (Node, pinned `mermaid` devDependency) run `mermaid.parse()` over exporter output for a corpus that includes adversarial labels (`end`, `o`, `x`, quotes, parens, braces, `<script>`, `%%`, `---`, unicode, 500-char lines). This is also the only trustworthy answer to "does the export render on GitHub".
3. Declare feature loss explicitly: VoxDiagram constructs with no Mermaid equivalent (ports, some styling) must degrade with a documented, tested mapping, not silently.
4. Pin the Mermaid version used for the oracle, and note that GitHub's bundled version moves independently.
**Warning signs:** Export tests that only compare strings; no Node-side parse in the test suite.
**Phase to address:** Phase 1 (export) with the Node oracle test in Phase 3/5.

---

### Pitfall 9: Lossy Mermaid importer and a 40-diagram review burden

**What goes wrong:** The importer drops `classDef`, `style`, `linkStyle`, `click`, edge types, shapes, `<br/>`, `&` fan-out, or subgraph direction without telling anyone; migrated diagrams look different and no one notices until readers complain. Or reviewers face dozens of before/after visual diffs and rubber-stamp them.
**Why it happens - the real corpus (measured):**
- `docs/src` has **69** `mermaid` fences in **53** files by a line-start fence scan (the milestone brief says ~71 in ~55; the difference is likely counting method, e.g. indented/other fences - reconcile with the existing `extract_mermaid_blocks`, which handles indentation and `~`/length variants). **29 blocks in 28 files are under `docs/src/archive/` and must not be migrated or even read** (Archival Protocol). **Migratable: 40 blocks in 25 files.** Another ~10 blocks live under `docs/superpowers/` (outside the stated gate scope; decide in/out). `README.md` has none.
- Non-archive types: 37 flowchart/graph (16 `flowchart LR`, 14 `TD`, 5 `graph TD`, 1 `TB`... plus sub-forms), **2 sequenceDiagram, 1 stateDiagram-v2, 1 gantt**. Features present: `&` fan-out 22, `subgraph` 11, `<br` 1, cylinder `[(` 1, `click` 1.
- So a flowchart importer covers ~92%; sequence/state/gantt are 4 blocks that the DSL may not even model (gantt almost certainly not).
**How to avoid:**
1. **Fail closed.** The importer supports an explicit subset and returns a structured `unsupported` list (construct, line) for everything else; it never drops silently. Emit a per-diagram fidelity report.
2. Use Mermaid's own parser as the *oracle for semantic equivalence*: extract vertices/edges from the real parser (Node, pinned version) and compare to the imported IR's node/edge/container sets. Diagrams with identical sets auto-pass the semantic check; humans review only layout and aesthetics.
3. Decide per-type up front: flowcharts -> automated import; the 2 sequence + 1 state + 1 gantt -> hand-convert, defer, or replace with a table/list. Note the conflict this creates with "gate blocks new ```mermaid blocks": either those 4 are converted/replaced or the gate needs a documented allowlist - decide before the gate lands.
4. Review burden: generate one **contact-sheet page** (before = Mermaid render, after = VoxDiagram SVG, side by side, plus the fidelity report) and migrate in small batches by directory, one PR per batch, each batch gated by the checks in Pitfall 14. Do not do 40 diffs in one PR.
5. Know that importing is a one-time tool: do not over-invest in importer completeness beyond the corpus. YAGNI the long tail (state/class/ER/gantt/journey/git graph).
**Warning signs:** Importer has no "unsupported" output channel; the PR description says "all diagrams migrated" with no fidelity artifact; styling (`classDef`) vanishes with no diagnostic.
**Phase to address:** Phase 5.

---

### Pitfall 10: SVG theming and accessibility treated as polish

**What goes wrong:** Diagrams are unreadable in one theme (hard-coded fills), invisible to screen-reader users, fail contrast, or look wrong in forced-colors/print.
**Why it happens:** Starlight switches theme with `data-theme` on `<html>` (the existing `PageSidebar.astro` reads `document.documentElement.dataset.theme`); `prefers-color-scheme` media queries in the SVG will not follow the user's manual toggle. SVG loaded via `<img src>` cannot see page CSS variables at all. Screen readers expose inline SVG inconsistently and cannot "read" a graph.
**How to avoid:**
1. **Inline SVG only**, colors via CSS custom properties with fallbacks (`fill: var(--vd-node-fill, var(--sl-color-gray-6))`), defined in site CSS from Starlight tokens (`--sl-color-*`) for both `[data-theme=light]` and `dark`. No hard-coded hex in generated output other than fallbacks.
2. Contrast: text >= 4.5:1, edges/borders/arrowheads >= 3:1, in *both* themes; check programmatically (axe in Playwright, already present in docs-astro tests). Never encode meaning by color alone (use shape/stroke-dash/label).
3. Each SVG: `role="img"`, `<title>` + `<desc>` with ids wired via `aria-labelledby`, and a generated **text alternative** (an adjacency-list `<details>` or visually-hidden list: "A connects to B: label") because the SVG's internal structure is not reliably announced. Keep it generated from the IR so it cannot drift.
4. Support `forced-colors` (use `currentColor`/system colors for strokes) and print (light background).
5. Explicit `viewBox`, no fixed width/height, so reflow works (and see the legibility-scale guard in Pitfall 5).
**Warning signs:** Any `fill="#..."` in generated output; screenshot review only in dark mode (the repo's GUI policy already demands both themes for GUI surfaces; mirror it).
**Phase to address:** Phase 3.

---

### Pitfall 11: Inline-SVG id and CSS collisions when many diagrams share one page

**What goes wrong:** Arrowheads from diagram A appear on diagram B or vanish; styles from one diagram recolor another; `aria-labelledby` points at the wrong title; Mermaid itself had this bug class (Redmine #44584: Mermaid used `Date.now()` as the diagram id, so blocks rendered in the same millisecond collided and an error graphic was drawn into a valid diagram) [MEDIUM].
**Why it happens:** All inline SVGs share one document id namespace. `<marker id="arrow">`, `<defs>`, `<clipPath>`, `<linearGradient>`, and `url(#...)` references resolve document-wide; the first match wins. A `<style>` element inside inline SVG applies to the **whole page**, not only the SVG.
**How to avoid:**
1. Every generated id carries a caller-supplied unique prefix (`vd-<page-slug>-<fence-index>-`), applied to markers, defs, title/desc ids, and `url()` references in one place in the renderer. Deterministic (position-based), so no hashing and no randomness (Pitfall 3). The renderer API takes `id_prefix` as a required parameter.
2. Emit **no `<style>` element** in the SVG; use presentation attributes plus classes defined once in site CSS (scoped under `.vox-diagram`), or inline `style` with only values from a fixed whitelist.
3. Test: a Playwright page with all 40 migrated diagrams (or a synthetic page of 50) asserts `document.querySelectorAll('[id]')` has no duplicates and that every `url(#x)` target is inside the same SVG.
**Warning signs:** Arrowheads missing on diagram N>1; CSS from a diagram affecting page chrome.
**Phase to address:** Phase 2 (renderer API), Phase 3 (page-level test).

---

### Pitfall 12: Label-driven injection (XSS/CSS/markup) in generated SVG, Mermaid export, and the MCP tool

**What goes wrong:** A label (from an LLM, a PR, a user) contains `</text><script>`, `"onload=`, `<foreignObject>`, `]]>`, control characters, or an id like `a{}*{display:none}` and the SVG becomes active content, malformed XML (blank diagram), or injects CSS into the page. The pre-rendered SVG is then spliced into HTML as raw markup (`set:html` / rehype-raw), bypassing Astro's escaping.
**Why it happens:** Real precedent: Mermaid CVE-2026-50159 - CSS injection via sibling-combinator selectors built from diagram-supplied class/id names (fixed in 10.9.8 and 11.16.1), and CVE-2026-41159 - styles escaping per-diagram scoping via `fontFamily`/`themeCSS` (fixed in 10.9.6 and 11.15.0) [MEDIUM, via OSV/Ubuntu/Mend summaries]. The site currently loads an **unpinned `mermaid@11`** from a CDN at runtime.
**How to avoid:**
1. Treat every label, id, tooltip, and attribute value as untrusted. Build SVG only through a writer that escapes text nodes (`& < >`) and attribute values (`& < > " '`) - use `quick-xml`'s writer (already in the workspace) rather than `format!`. Reject or strip XML-illegal control characters (U+0000-U+0008, U+000B, U+000C, U+000E-U+001F) - they produce *well-formed-looking but invalid* SVG that browsers refuse to render.
2. Never derive CSS class names, element ids, or `href`s from user text; map user ids to generated ones (`n0`...). No `foreignObject`, no `<a href>`/`xlink:href` from input in v1, no user-controlled `style`, no `<script>`, no `<image>`.
3. Length/size caps: label length, node count (e.g. <= 500), edge count, nesting depth, input bytes - at the parser, not the renderer.
4. Property/fuzz tests with `proptest`: arbitrary Unicode strings as labels/ids -> render -> parse with `quick-xml` -> assert well-formed and that the element/attribute set is a subset of an allow-list. This is the guard to **verify by mutation** (repo rule): temporarily delete the escape and confirm the test fails; record before/after presence of the edit so a concurrent formatter cannot hand you a false pass.
5. MCP `vox_validate_diagram` echoes user text in diagnostics: keep it in JSON strings only (never splice into markdown/HTML), and keep within the repo's untrusted-input boundary conventions.
6. Removing the runtime CDN Mermaid also removes the unpinned third-party script from the site; count that as a benefit and a reason to finish the migration.
**Warning signs:** `format!("<text>{}</text>", label)` anywhere; ids copied from user input; rendered SVG injected by string concatenation in the plugin.
**Phase to address:** Phase 2 (writer), Phase 4 (MCP tool), Phase 3 (integration).

---

## Moderate Pitfalls

### Pitfall 13: Validator says "valid" for diagrams that are wrong or low quality
**What goes wrong:** Syntax-only validation gives false confidence (the DIAGRAM-FAIL-BENCH lesson: renders != satisfies the task).
**Prevention:** Beyond parse errors, emit **semantic lints**: undeclared edge endpoints, duplicate ids, orphan nodes, label identical to id, container with one child, edge to self, unreachable component, over-size (> N nodes for readability), cyclic containment. Return them as warnings with stable codes (`vox/diagram/...`, matching the `vox audit` discriminator convention: findings carry `schema_version` and a per-finding discriminator).
**Phase:** 1 and 4.

### Pitfall 14: Fence-name, doctest, lint and Shiki interactions
**What goes wrong:** (a) A diagram authored in a ```` ```vox ```` fence is compiled by the doctest runner and fails the docs gate (`doctest.rs` treats exactly `lang == "vox"` as Vox source; the advisory's own sample uses ```` ```vox ```` with `// vox:skip`). (b) Lint and doctest disagree on fence parsing: `lint.rs` splits the info string on `,` before comparing to `vox`, while `doctest.rs` compares the whole trimmed info string, so ```` ```vox,title=x ```` is Vox to the linter but not to the doctest. (c) Expressive Code/Shiki sees an unknown language and warns or falls back. (d) The new gate's regex hits prose that merely mentions the fence (the advisory itself contains the literal text and 4-backtick forms; docs *about* the gate will too).
**Prevention:** Use exactly one hyphenated token, ```` ```vox-diagram ````, never a comma-attribute form; keep it distinct from `vox`. Register `vox-diagram` as a Shiki alias (plain-text or custom grammar) in `astro.config.mjs` `expressiveCode.shiki.langs` so the unrendered fallback is highlighted. Build the "no new ```mermaid" gate on the **existing fence-aware `extract_mermaid_blocks`** (it tracks fence length and indentation), not a line regex; add fixtures for prose mentions and nested 4-backtick examples; exempt `docs/src/archive/`. Give docs that must show literal mermaid text a ```` ```text ```` fence.
**Phase:** 3 (fence plumbing), 6 (gate).

### Pitfall 15: Test-first policy and snapshot sprawl
**What goes wrong:** `skeleton/untested-pub-api` warnings and the `tdd-guard` pre-commit hook block commits because layout/render `pub fn`s have no adjacent `#[test]`; or the opposite - hundreds of SVG snapshot files whose churn nobody can review.
**Prevention:** Failing test first for every `pub fn` (files >= 30 non-blank lines). Use `insta` golden snapshots for ~10-15 canonical diagrams only; everything else is invariant/property tests (Pitfall 4 list) and IR-level assertions. Mutation-prove the escape and determinism guards. Hosted CI is the gate: run only `cargo check -p vox-diagram` and `cargo nextest run -p vox-diagram` locally, never workspace clippy; background long builds (memory: never block on cargo; lean local verification).
**Phase:** every phase.

### Pitfall 16: SSOT/generated-doc drift from new surfaces
**What goes wrong:** New MCP tool and any new CLI subcommand (`vox diagram ...`) make `cli-command-surface.generated.md`, `mcp-tools.generated.md`, capability index etc. stale; the PR is red on `ssot-drift`; someone adds a "regenerate after merge" commit (the 60+-commit antipattern named in AGENTS.md).
**Prevention:** Fix at the source (catalog/registry YAML), run `vox ci command-sync` in the PR, let `ssot-autoregen` handle mechanical drift. New docs under `docs/src/` need full frontmatter (`title`, `description`, `category`, `topic`, `domain`, `authorship`, `authorship_model`) and a 40-120 word `<!-- kb:summary -->`; never add to the facet backlog or set `human_attested_by`. Any new generated doc counts against `docs-reality-audit`.
**Phase:** 4 and 6.

### Pitfall 17: VoxScript-first and security-invariant violations in migration tooling
**What goes wrong:** An executor writes `migrate.py`/`render-all.sh` for the batch import, the contact sheet, or snapshot refresh.
**Prevention:** All of it is `.vox` run via `vox run` (or a `vox ci` subcommand in Rust). Per the security invariants, scripts that modify the repo must be **committed before an agent executes them**, may not use `shell_exec` to bypass the sandbox, and subprocess calls go through `vox-actor-runtime` primitives. The Node-side Mermaid oracle belongs inside `docs-astro`'s existing vitest/Playwright setup, not a loose script.
**Phase:** 5.

### Pitfall 18: Retiring Mermaid support too early
**What goes wrong:** The migration deletes `vox_validate_mermaid`, `mermaid_repair.rs` and the doc-pipeline Mermaid lint. Agents still emit Mermaid in chat, PR bodies, GitHub issues and `docs/superpowers/`, where there is no VoxDiagram renderer; those outputs lose validation.
**Prevention:** The gate scope is *new fences under non-archive `docs/src/`*. Keep the Phase-55 Mermaid validator and chat self-repair running for non-docs surfaces; only remove them under a separate, explicit retirement plan (`// vox-deprecated-since=... retire-by=...` markers if deprecated). Update the agent guidance (advisory section 7 rules) to say "docs: VoxDiagram; elsewhere: Mermaid via export".
**Phase:** 6.

### Pitfall 19: Scope creep into the compiler, GUI canvas and extra export targets
**What goes wrong:** The advisory's Phase 3 (compiler `diagram` keyword, LSP, Axis Canvas/WebGL, D2/TALA export, interactive features) is absorbed into the milestone and nothing ships.
**Prevention:** Milestone boundary = standalone crate + fence + docs pre-render + MCP validator + migration + gate. Everything else goes to a deferred list. Do not add D2 export (no consumer), animation, or click/links in v1.
**Phase:** 0 (scope), reviewed at each phase boundary.

---

## Minor Pitfalls

### Pitfall 20: Milestone/phase numbering collision
`.planning/PROJECT.md` already defines "Milestone v1.6 Mens Continuous Training" with Phases 77-82, while this work is branch `milestone/v1.6-voxdiagram`. Allocate a distinct milestone label and non-overlapping phase numbers (see `phase-numbering-index`), or planning artifacts and `STATE.md` will conflict.

### Pitfall 21: Overbuilding for scale
All real diagrams are small (largest is ~35 source lines; 40 total). O(n^3) crossing minimization is fine at this size. Do not build incremental layout or WebGL. But the MCP tool accepts arbitrary agent input, so bound it (caps in Pitfall 12) and fuzz the parser for stack depth (deeply nested containers must produce a diagnostic, not a stack overflow).

### Pitfall 22: Stale `training_eligible` / Pagefind / llms.txt expectations
Docs frontmatter can mark pages `training_eligible: true` for MENS; migrating diagram fences changes the corpus text. Keep source fences in the page so llms.txt and Pagefind index the semantic text, not an opaque SVG; never replace the fence with a committed SVG blob.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `HashMap` in layout "just for lookups" | Faster to write | Any iteration of it leaks nondeterminism into SVG | Never inside `vox-diagram`; use `BTreeMap` or sorted `Vec` |
| `format!` to build SVG | No writer abstraction | XSS/malformed XML; escaping scattered | Never |
| Guessing text widths (`chars * 7.0`) | Ships a demo in a day | Clipped/ragged boxes, CJK/emoji wrong | Only the very first spike; replace in Phase 2 |
| Importer that silently ignores unknown syntax | Higher "success" rate | Silent visual/semantic loss across 40 pages | Never; emit `unsupported` |
| Committing rendered SVGs | Avoids build-time render | Drift artifacts, review noise, stale diagrams | Only if registered with ssot-drift and `linguist-generated` |
| Keeping both Mermaid CDN loader and new renderer | Zero archive regression | Two pipelines, unpinned third-party script | Short transition only, with a removal task |
| Editing `crate-edges.allow.v1.json` to get green | Unblocks CI | Violates USER-AUTHORIZED-ONLY rule; silent architecture erosion | Never by an agent |
| LLM-as-judge for syntax validity | No parser needed yet | Judges disagree by ~30 points (MermaidSeqBench) | Never for syntax; fine as a secondary semantic signal |
| Skipping the Playwright browser overflow test | Faster CI | Font drift ships unnoticed | Never |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Astro/Starlight build | Shelling out to `cargo run` per diagram | Prebuilt `vox` binary, content-keyed cache, remark plugin ahead of Expressive Code |
| Starlight theming | `prefers-color-scheme` or hard-coded fills in SVG | CSS custom properties bound to Starlight `data-theme` tokens |
| Expressive Code / Shiki | Unknown `vox-diagram` language triggers fallback warning | Register alias in `expressiveCode.shiki.langs` |
| `vox-doc-pipeline` lint/doctest | Diagram authored in ```` ```vox ```` and compiled | Distinct `vox-diagram` token; fence-aware gate via `extract_mermaid_blocks` |
| `crate-edges` / `layers.toml` | Agent adds an exception or regenerates baseline | Phase 0 owner authorization listing exact edges |
| MCP tool registry | Only adding Rust dispatch | Update registry/catalog/capability/governance contracts; run `vox ci command-sync`; satisfy `gui-coverage-gate` |
| GitHub/README rendering | Expecting VoxDiagram to render | Ship the Mermaid export, validated against real Mermaid in Node |
| LLM calls (repair loop) | Hardcoded vendor host or pinned model | `vox_actor_runtime::llm` facade; model chosen by registry/policy |
| Hashing for ids | Importing `sha2`/`blake3` directly | Caller-supplied deterministic prefix; or `vox-crypto` with an authorized edge |
| Archive docs | Migrating or scanning `docs/src/archive/` | Exempt in importer, gate, and CI; never read it |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Unbounded crossing-minimization iterations | Layout seconds-long on one diagram | Fixed sweep count, node/edge caps, time budget in MCP path | > ~200 nodes (not in corpus; reachable via agent input) |
| Re-rendering every diagram on every docs build | Docs build time climbs | Content-hash cache keyed on source + crate version | Each added diagram; matters in CI cap |
| Deep container nesting recursion | Stack overflow in parser/layout | Depth cap (2 for layout, higher parse cap with diagnostic) + fuzz | Adversarial/agent input |
| Giant SVGs scaled into a narrow column | Illegible text | Effective-scale check; reflow or scroll container | Wide LR flows with 8+ ranks |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Raw label interpolation into SVG | XSS / malformed SVG | `quick-xml` writer + escape tests, mutation-verified |
| User ids/classes as CSS classes or element ids | CSS injection (cf. Mermaid CVE-2026-50159) | Generated ids only; no user-derived selectors |
| `<style>` in inline SVG | Page-wide style leakage | No `<style>` in output; scoped site CSS |
| Runtime unpinned CDN Mermaid | Supply-chain and CVE exposure (CVE-2026-50159 / 41159 fixed only in 11.16.1 / 11.15.0 and later) | Remove at migration end; until then pin and keep `securityLevel: strict` |
| `foreignObject`/`href` support "for convenience" | Script/URL injection | Not in v1 |
| Diagnostics spliced into HTML/markdown | Reflected injection through MCP tool | JSON-only output |
| Unbounded input to `vox_validate_diagram` | DoS | Size/node/depth/time caps |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Diagram looks different from the Mermaid it replaced (direction, grouping, density) | Docs feel regressed; authors lose trust | Side-by-side contact sheet and layout hints |
| Opaque parse errors ("unexpected token") | Agents and humans cannot repair | Spans plus canonical-rewrite suggestions, stable codes |
| No fallback on failure | Blank hole on the page | Show source block plus error banner at runtime; fail CI for non-archive |
| No text alternative | Screen-reader users get nothing | Generated adjacency description from IR |
| Tiny text in wide diagrams | Unreadable on laptops/phones | Effective-scale guard |

## "Looks Done But Isn't" Checklist

- [ ] **Determinism:** Passes on macOS *and* hosted Linux with byte-identical SVG; HashMap-free crate; no transcendental float functions in routing.
- [ ] **Escaping:** Escape deleted by mutation makes a test fail (before/after presence check recorded).
- [ ] **Mermaid export:** Parsed by the real, pinned Mermaid parser in Node over an adversarial label corpus (`end`, `o`/`x` ids, quotes, `<`, `%%`).
- [ ] **Importer:** Emits an `unsupported` list; semantic equivalence checked vs Mermaid's own parser output, not just "no error".
- [ ] **Browser fit:** Playwright asserts every `<text>` bbox fits its node under two font stacks; Chromium at content-column width.
- [ ] **Page-level:** Test page with all diagrams has zero duplicate ids and no cross-SVG `url(#)` references; no `<style>` in SVG.
- [ ] **Themes:** Both light and dark screenshots reviewed; axe contrast clean; forced-colors sane.
- [ ] **Accessibility:** `role="img"`, `<title>`/`<desc>`, text alternative present for every diagram.
- [ ] **Governance:** Owner approval recorded for crate/layer/edges before code; `layers.toml`, `where-things-live.md`, hakari, fan-in baseline updated by the *approved* list only.
- [ ] **MCP tool:** All contract files and generated references updated; `gui-coverage-gate` green; `vox ci command-sync` clean.
- [ ] **Gate:** Fence-aware, exempts `docs/src/archive/`, tested against prose mentions and 4-backtick examples; the 4 non-flowchart blocks resolved or explicitly allowlisted.
- [ ] **Measurement:** A recorded, reproducible NL->diagram harness result with CIs exists, and the advisory's table is annotated.
- [ ] **Frontmatter/KB:** Any new `docs/src/` page has full frontmatter plus `<!-- kb:summary -->`; scoped `vox-doc-pipeline --lint-only` passes.
- [ ] **Archive untouched:** `git diff --stat -- docs/src/archive` is empty.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Nondeterminism discovered late | MEDIUM | Replace maps with BTree/sorted Vec, quantize coords, regenerate snapshots once, add permutation + repeat tests |
| Layout engine rabbit hole | HIGH | Cut scope to depth-2 containers; adopt an existing crate; ship flat+container only; keep Mermaid export as the escape hatch |
| Font drift complaints | LOW-MEDIUM | Switch to pinned font (option a), regenerate metric table, rerun Playwright overflow test |
| Edge/exception rejected by owner | MEDIUM | Re-home consumer (e.g. CLI/CI instead of doc-pipeline), or move fence validation into `vox-cli-ci`; do not fork 100+ lines |
| Mermaid export invalid on GitHub | LOW | Fix id/quoting rules, add the failing case to the Node oracle corpus |
| Silent migration loss found after merge | MEDIUM | Revert that batch (batches are separate PRs), add the missed construct to `unsupported`, re-run with fidelity report |
| LLM first-try rate poor | MEDIUM | Tune grammar card and canonical-rewrite diagnostics; add deterministic `--fix`; widen lenient parsing; keep Mermaid path for agents meanwhile |
| Unauthorized exception committed | HIGH | Revert it; escalate to owner; this is a policy violation |

## Pitfall-to-Phase Mapping

Suggested phase structure (names are suggestions for the roadmapper):

- **Phase 0 - Governance, baseline & decisions:** owner authorization, scope, advisory annotation, eval metric definition.
- **Phase 1 - Core: grammar, IR, parser, diagnostics, Mermaid export.**
- **Phase 2 - Layout, text measurement, SVG renderer.**
- **Phase 3 - Docs-site integration: pre-render, theming, a11y, fence plumbing.**
- **Phase 4 - MCP tool + LLM authoring loop + measurement.**
- **Phase 5 - Importer + corpus migration.**
- **Phase 6 - Enforcement gate + cleanup/retirement decisions.**

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1 Unsourced-number dependence | 0, 4 | Success criterion is a measured non-inferiority result; advisory annotated |
| 2 Zero-training-data DSL | 1 (diagnostics), 4 (card, loop, harness) | Harness report with CIs: first-try, post-repair, semantic F1, vs Mermaid arm |
| 3 Nondeterminism | 2 | Permutation + repeat + cross-OS byte-identical snapshot |
| 4 Compound layout / edge labels | 2 (spike first) | Invariant checker passes on all 40 real diagrams |
| 5 Text measurement | 2, 3 | Playwright bbox-fit test under two font stacks |
| 6 Governance / registration | 0, 1, 4 | `crate-edges`, hakari, `gui-coverage-gate`, `command-sync` green with no new exceptions authored by agents |
| 7 Build integration / old loader | 3 | Docs workflow has no `cargo`; archive decision recorded; single render path |
| 8 Invalid Mermaid export | 1 (+5) | Node real-parser oracle over adversarial corpus |
| 9 Lossy import / review burden | 5 | `unsupported` list, equivalence vs Mermaid parser, contact sheet, batched PRs |
| 10 Theming / a11y | 3 | Both-theme screenshots, axe clean, text alternative present |
| 11 Id/CSS collisions | 2, 3 | Duplicate-id page test; no `<style>` in output |
| 12 Label injection | 2, 4 | proptest well-formedness + allow-list; mutation proof |
| 13 Weak validator | 1, 4 | Semantic lint codes with fixtures |
| 14 Fence/doctest/lint/gate | 3, 6 | Fixtures for prose mentions; archive exempt; scoped doc-lint passes |
| 15 TDD / snapshot sprawl | all | `skeleton/untested-pub-api` clean; snapshot count bounded |
| 16 SSOT drift | 4, 6 | `vox ci ssot-drift` green; no regenerate-after-merge commits |
| 17 VoxScript-first | 5 | No new `.sh`/`.py`; tooling committed before execution |
| 18 Early retirement of Mermaid | 6 | Phase-55 validator still active on non-docs surfaces |
| 19 Scope creep | 0 | Deferred-list reviewed at each boundary |
| 20 Numbering collision | 0 | Distinct milestone label and phase numbers |

## Flags for deeper phase-level research

- **Phase 2:** layout strategy for compound graphs (spike, existing crates vs in-house) and font policy (pinned font vs conservative table) - likely needs its own research.
- **Phase 3:** how the Astro build invokes the Rust renderer (prebuilt binary vs WASM) and its CI/deploy cost.
- **Phase 4:** the harness design and model set; OmniDiagram (arXiv 2604.05514) should be read in full before finalizing the baseline narrative.
- **Phase 1, 5, 6:** standard patterns; low research need.

## Sources

Primary / HIGH:
- MermaidSeqBench (full text fetched): https://arxiv.org/html/2511.14967v1 ; abstract https://arxiv.org/pdf/2511.14967
- DiagramEval (full text fetched): https://arxiv.org/html/2510.25761v2 ; EMNLP 2025 https://aclanthology.org/2025.emnlp-main.640
- Grammar Prompting for DSL generation (Wang et al., NeurIPS 2023): https://arxiv.org/pdf/2305.19234
- Mermaid flowchart syntax (reserved `end`, `o`/`x` adjacency, quoting): https://mermaid.js.org/syntax/flowchart.html
- Rust `f64::sin` documented non-determinism: https://doc.rust-lang.org/std/primitive.f64.html#method.sin
- Starlight default `--sl-font-system` stack: `docs-astro/node_modules/@astrojs/starlight/style/props.css` (local)
- Repo (read directly): `docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md`; `AGENTS.md`; `crates/vox-doc-pipeline/src/pipeline/{mermaid.rs,doctest.rs,lint.rs}`; `docs-astro/src/components/PageSidebar.astro`; `docs-astro/astro.config.mjs`; `docs/src/architecture/layers.toml`; `contracts/ci/crate-edges.allow.v1.json`; `docs/src/architecture/rust-build-loop-ssot-2026.md` (Part B authorization precedent); Cargo.toml workspace dependencies. Corpus counts computed by line-start fence scan of `docs/src` (archive counted but not read).

Secondary / MEDIUM:
- NeurIPS 2026 workshop poster "Documentation Helps, But Gaps Remain" (DIAGRAM-FAIL-BENCH): https://neurips.cc/virtual/2026/173008 (OpenReview https://openreview.net/forum?id=OkCL6M0stQ)
- Synthetic Programming Elicitation (very-low-resource languages): https://neurips.cc/virtual/2024/poster/93893
- SLMFix (small-model DSL error fixing): https://arxiv.org/pdf/2511.19422
- Image-to-UML study with PlantUML compile failures: https://arxiv.org/pdf/2404.11376
- Microsoft GenAIScript "Mermaids Unbroken": https://microsoft.github.io/genaiscript/blog/mermaids/
- Cline #4046 https://github.com/cline/cline/issues/4046 ; Roo-Code #4636 https://github.com/RooCodeInc/Roo-Code/issues/4636
- Mermaid CVE-2026-50159 (CSS injection via class/id names; fixed 10.9.8 / 11.16.1): https://osv.dev/vulnerability/UBUNTU-CVE-2026-50159 ; CVE-2026-41159 (fixed 10.9.6 / 11.15.0): https://releasealert.dev/cve/CVE-2026-41159
- Redmine Mermaid duplicate-id bug: https://www.redmine.org/issues/44584
- ELK `hierarchyHandling`: https://eclipse.dev/elk/reference/options/org-eclipse-elk-hierarchyHandling.html ; `mergeHierarchyEdges`: https://eclipse.dev/elk/reference/options/org-eclipse-elk-layered-mergeHierarchyEdges.html ; Graphviz `lhead`: https://www.graphviz.org/docs/attrs/lhead/
- D2 / TALA open-source note: https://d2lang.com/blog/tala-is-open-source/

LOW / unverified (do not cite as fact): OmniDiagram (arXiv 2604.05514) results; availability of a D2 Rust parser; compound-graph support in third-party Rust layout crates; the claim that D2-style syntax is meaningfully more LLM-fluent than alternatives.

---
*Pitfalls research for: VoxDiagram native diagram DSL, layout, SVG, and docs migration (Vox v1.6 VoxDiagram milestone)*
*Researched: 2026-10-16*
