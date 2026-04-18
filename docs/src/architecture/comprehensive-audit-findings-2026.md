---
title: "Comprehensive Vox audit findings (April 2026)"
description: "Full-spectrum audit of the Vox codebase, docs, and roadmap; identifies the biggest gaps blocking the 'premier LLM destination for web app code' thesis and lists ~100 prioritized improvements."
category: "architecture"
status: "research"
last_updated: 2026-04-17
training_eligible: true
training_rationale: "Strategic audit context for engineering prioritization."
schema_type: "TechArticle"
---

# Comprehensive Vox audit findings (April 2026)

Bert — this is a cross-cutting audit against the stated vision: *Vox is the premier programming language for LLMs to write code, principally for web apps.* It triangulates the `crates/`, `docs/src/architecture/` SSoTs/roadmaps, ADRs 001–023, the CHANGELOG, the parser error logs, and the golden examples corpus. The goal is to name the real gaps bluntly and give you ~100 concrete, prioritized improvements you can sequence.

## Part I — Executive diagnosis

### What Vox genuinely has (real, shipped, good)

- A real Rust compiler with lexer, parser, HIR, IR, typeck, and dual Rust+TS codegen (`crates/vox-compiler/`).
- A real orchestrator with ~17.5 kLOC of substance: session envelopes, A2A messaging, observability, budgets, and daemon parity (`crates/vox-orchestrator/`).
- A real Populi mesh control plane with NVML probing, scope-tenancy, and HTTP control (`crates/vox-populi/`).
- A real MENS training pipeline: native Candle QLoRA, Qwen 3.5 4B on 16GB, deterministic corpus export, OpenAI-compatible local serve (`crates/vox-tensor/`, `vox-schola`).
- A real RAG stack (Scientia) with BM25+FTS5+vector fusion, RRF, CRAG, Socrates hallucination gating, DuckDuckGo/Tavily/Qdrant backends (`crates/vox-search/`, `crates/vox-scientia-*`).
- A serious architectural governance regime: 23 ADRs, `.voxignore` as single source of truth, Clavis for secrets, TOESTUB detectors, completion-policy gates, `vox ci` guards — the discipline is unusual for a v0.4 language.
- A working `@table` → SQL → typed client → typed server collapse — the core K-complexity value proposition actually works end-to-end for the golden examples.
- A 102-tool MCP surface plus VS Code extension, OpenCode bridge, dashboard — the AI ergonomics are wired.

### The five biggest structural problems

1. **Parser regressions break the "vox run" glue story.** `parser_errors.txt` shows `for`/`if`/`while` cannot appear at top level; `examples/actor.vox` and `examples/mcp_tool.vox` have been archived as *non-parseable on current grammar*. Meanwhile AGENTS.md mandates Vox-as-glue for all project automation. You are telling LLMs to generate `.vox` scripts the parser rejects.
2. **The web-UI pillar embeds React + JSX directly in `.vox` sources.** Authors still write `<div className={…}>`, `use_state`, `onClick`; the training corpus is contaminated with React idioms. The documented "Path C / Vox-native reactivity" is aspiration, not requirement. This directly undermines the LLM-native K-complexity thesis.
3. **LLM-alignment scaffolding is 60–70% unshipped.** GBNF grammar export is ~30 lines (expressions only) with a known CVE in downstream llama.cpp. `correction_hint` is populated in 1 of 31 HIR validation sites. `@llm` decorator has no runtime dispatch. AST-token alignment is all checkboxes, no code. There is **no HumanEval-Vox or SWE-bench-Vox** showing that Vox-generating models actually hallucinate less than Python/TS-generating ones.
4. **Doc/code divergence has become a governance problem, not a doc problem.** 273 architecture docs, multiple "historical"/"superseded" SSoTs, three parallel SSoT generators (operations catalog + MCP registry + capability registry + CLI registry), research → blueprint pipeline unclear, and pages still reference retired surfaces (HTMX, Pico, TanStack virtual routes). The doc tree is growing faster than the code tree is shipping.
5. **Scope is too wide for the evidence.** 67 workspace crates, 280+ CLI subcommands, 23 ADRs, a telemetry subsystem, a cryptography SSoT, a gamification crate, a speech-to-code pipeline, a vision lane, a plugins marketplace, a MENS training pipeline, a mesh, a RAG, a publication pipeline, a workflow runtime, a package manager. There is no plausible world in which all of these are "stable" at the same time. `feature-growth-boundaries.md` and `vox-bell-curve-strategy.md` are trying to restore discipline but the horse is already out of the barn.

### The single-sentence verdict

*Vox has the best architectural governance of any v0.4 language I've seen, a real compiler + training + orchestrator stack most projects at this tier lack, and absolutely no empirical evidence for its headline "LLM-native" claim — while simultaneously being too sprawled to ship such evidence at current scope.*

## Part II — ~100 prioritized improvements

Each item carries an explicit priority tag:

- **P0** = ship-stopping / credibility-risking; fix before making any external claims.
- **P1** = materially advances the v1.0 thesis or unblocks several dependents.
- **P2** = high-value incremental; schedule within two quarters.
- **P3** = good hygiene; defer until P0/P1 caught up or fold into ongoing work.

### Category A — Parser & compiler correctness (the oxygen layer)

1. **[P0] Restore top-level script grammar.** Extend `parse_top_level_item` to accept arbitrary statement blocks (or implicitly wrap top-level statements in `fn main()`). Without this, `vox run scripts/*.vox` — which AGENTS.md mandates as the only glue format — is broken. Re-enable `examples/actor.vox` and `examples/mcp_tool.vox` from archive.
2. **[P0] Parallel error collection in the parser.** Replace first-error-bail with `collect_all_errors`; emit every syntactic fault per file. This is a prerequisite for (a) good LSP, (b) LLM self-repair loops, (c) training-time error mining.
3. **[P0] Fix the `vox-doc-pipeline` compile break.** `check.log` shows `lint_file` private-fn import failure; the doc pipeline cannot compile. Either re-export `lint_file` or replace the call site. This is a build blocker.
4. **[P0] Unbreak `cargo check --all-features` across the workspace.** Add a CI lane that runs `cargo check -p vox-cli --all-features` plus `cargo check --workspace --all-features` on every PR; the feature matrix has grown past what the current CI catches.
5. **[P1] Populate `correction_hint` in every HIR validation site.** Currently 1 of 31. Each error must carry (a) minimal patch, (b) expected shape, (c) one concrete example. This is the feedback channel for LLM self-repair.
6. **[P1] Finish the bool exhaustiveness checker.** `match_exhaust.rs` currently lets match-on-`bool` pass with a missing arm; compiled training data teaches LLMs unsound patterns.
7. **[P1] Fix postcondition injection on early returns.** BUG-1 in the llm-target-language plan; contracts silently break for multi-return functions.
8. **[P1] Route IR unification.** Rust and TS emitters currently walk `Decl::Route` independently. Introduce a single `RouteIR` consumed by both emitters so LLMs learn one routing pattern, not two.
9. **[P1] Formal grammar freeze for the v1 surface subset.** Carve out a "Vox-1" subset (golden examples) as the tokenizer/parser contract; mark all other syntax experimental. Publish `contracts/language/vox-language-surface.v1.json`. You cannot ship a "stable" tier without this.
10. **[P2] Deprecation warnings for `ret`, `@component fn`, classic hooks.** The lexer still emits these with no warning; MENS learns deprecated forms. Wire `DeprecatedUsageDetector` into `vox check` output, not just CI.
11. **[P2] Kill the `legacy_ast_nodes` fallback for full-stack declarations.** `@page`, `@layout`, `@action`, `@theme` should either parse typed or be explicitly rejected. Silent untyped storage is a training hazard.
12. **[P2] Parser inventory discipline.** `examples/parser-inventory/` is currently drift-prone. Make `vox check --strict` match the inventory on every CI run; block PRs that change grammar without updating the inventory.
13. **[P2] Make `vox fmt` a parse-print-reparse round-trip test in CI.** `fmt` is wired, but there's no lane that validates idempotence on the golden corpus. Idempotent formatting is a prerequisite for training normalization.
14. **[P3] Cut the 199 kB `spec.rs.bak` file from the tree.** `spec.rs.bak` at 199 673 lines is a training-corpus contaminant and a source of reviewer friction.

### Category B — Web/UI pillar (the principal target)

15. **[P0] Ship a Vox-native reactivity primitive that doesn't require `use_state`.** The recommended "Path C" must become the only path. `component` bodies should read signals/effects/derived declaratively; the compiler owns the lowering to React hooks (or other backends later). Today, authors still have to `import react.use_state` in `.vox` — this is the single biggest contradiction to your thesis.
16. **[P0] Eliminate raw JSX from `.vox` source.** Replace `<div className="…" onClick={…}>` with a Vox-native view language (`view: <div class="…" on:click={…}>`) that lowers to JSX at codegen time. Freeze the old JSX-in-Vox syntax in goldens, warn on new use, remove in v1.
17. **[P0] Treat the `@table` → client/server slice as v1's signature demo.** It works end-to-end today, it's 10× less code than Next.js + Prisma + tRPC, and it is your real moat. Every piece of marketing, every tutorial, every Mens training sample should orbit this demo.
18. **[P1] Auto-type route loaders from `@query` signatures.** Docs imply this works; parser doesn't enforce type matching. A route that declares `with loader: list_posts` should require `loader`'s return type to unify with the component's prop surface at compile time.
19. **[P1] Compile-time typed island props.** `data-prop-*` attributes currently stringify at hydration. Emit a typed accessor (`readProp<T>(el, "name")`) and validate at compile time that every island prop has a Vox→JSON codec.
20. **[P1] Default `vox run` to SSR-orchestrated mode.** Currently users must set `VOX_ORCHESTRATE_VITE=1`. The "single command runs a full-stack app" pitch is invalidated every time a user hits manual env setup.
21. **[P1] Make routing manifest-first everywhere.** `tanstack-start-codegen-spec.md` is already marked historical; finish the migration, remove the dead emitter branches, and delete/retire the cancelled backlog so the next contributor doesn't mine obsolete tasks.
22. **[P2] Adopt modern CSS primitives now on the platform.** Container Queries, View Transitions, `:has()`, `@scope`, nesting. Emit them from `style:` blocks. Match `css-determinism-implementation-plan-2026.md` but ship, don't plan.
23. **[P2] Second codegen backend: compile-away signals (à la Svelte 5/Solid).** React is the starter target; a vanilla-signals backend demonstrates the "framework as compiler target, not dependency" story. Publish bundle-size comparisons; this is where you can credibly out-Next.js Next.js.
24. **[P2] Restore a classless theme system.** Pico is retired; there is no replacement. Ship a `@theme` primitive that emits a named CSS-variables system plus utility shorthands. Without this, LLM-generated apps stay visually ugly-or-bespoke and adoption stalls.
25. **[P2] Deterministic v0.dev normalizer inside the compiler, not the CLI.** Move `v0_tsx_normalize.rs` into `vox-compiler/codegen_ts` so `vox island generate` is a compiler pass, not a side-channel.
26. **[P2] Ship an official Vox → Next.js interop adapter.** Many companies will not rip out Next.js. A `@vox/next` adapter that lets Vox `@island` components mount in an existing Next app is a wedge into real production codebases.
27. **[P3] GUI visual-intelligence loop.** `gui-visual-intelligence-research-2026.md` and `vox-gui-vision-virtuous-cycle-implementation-plan-2026.md` describe a feedback loop; nothing ships. Either start a 4-week spike or explicitly defer to v1.1 in the roadmap.

### Category C — LLM-native language features (prove the thesis)

28. **[P0] Publish HumanEval-Vox and SWE-bench-Lite-Vox at matched compute.** Without public, reproducible benchmarks showing Vox-generating models outperform TS/Python-generating models at the same parameter count, the LLM-native claim is marketing. This is the single most important external-facing deliverable you do not yet have.
29. **[P0] Complete the GBNF / XGrammar-2 grammar export.** Current GBNF is ~30 LOC (expressions only) with CVE-2026-2069 in downstream serving. Until grammar-constrained decoding is real, LLMs hallucinate out of your coverage.
30. **[P1] Wire `correction_hint` into a compiler→LLM repair loop.** The field exists; the loop does not. Add a `vox repair --on-error <model-endpoint>` command that streams a compile error + hint to an LLM, applies a proposed patch, and re-checks. Ship the demo on the golden corpus.
31. **[P1] Implement `@llm` runtime dispatch.** Parsing + FnDecl fields land in Wave 2, but codegen does nothing. `@llm fn classify(s: str) -> Category` should emit an Axum handler that calls a Clavis-resolved LLM, validates against the return-type schema, and falls back to an `@llm.fallback` function.
32. **[P1] Training-corpus filter to vox-source-only.** MENS currently ingests `.vox` files that have React/JSX embedded. Add `context_filter: "vox_pure"` to the corpus builder; emit a "contamination score" per file; auto-exclude files above threshold.
33. **[P1] Publish the first public Vox adapter on HuggingFace.** `qwen-3.5-4b-vox` (QLoRA adapter, reproducible recipe). Even a modest adapter that makes a public foundation model speak Vox fluently validates the whole MENS thesis.
34. **[P1] AST-token weighted loss.** `ast-token-alignment-2026.md` lists it all unchecked. Implement `align_tokens_to_ast` + training recipe update; measure loss curve delta on a held-out Vox eval set.
35. **[P2] A Vox Rosetta benchmark suite.** Expand `examples/golden/inventory_rosetta_*.vox` into a multi-language (Rust, TS, Python, Vox) matched-task repo; publish "lines of code per task" and "LLM accuracy per task" charts. This is evergreen marketing with hard numbers.
36. **[P2] Compile-time capability/permission tracking exposed to models.** Each function's effects (reads DB, writes DB, hits HTTP, touches filesystem) should be inferable and visible in HIR JSON. LLMs need this to plan safely; it is also a security primitive.
37. **[P2] Structured error payloads in JSON (not just Rust `miette`).** Today `vox check --format json` is partial; promote it to parity with the human renderer, so `vox-compiler` can serve as an MCP tool any agent can call.
38. **[P2] Canonical "AI quickstart" decorator cheat-sheet in machine-readable form.** `docs/agents/` already has JSON manifests; add a single `docs/agents/vox-language-surface.v1.json` that enumerates every decorator, every keyword, every builtin with 3 gold examples apiece. Ground truth for in-context learning.
39. **[P2] Determinism audit of emitted code.** For a fixed Vox input, the emitted Rust, TS, SQL, and manifest bytes must be deterministic. Add `vox ci determinism-audit` running on goldens; any diff fails. Essential for reproducible training.
40. **[P3] Optional "graph IR" mode (research spike).** `research-ts-hallucination-frontier-2026.md` hints at it. Don't build yet; run a 2-week spike measuring whether serialized AST-as-JSON already wins the gain, before investing.

### Category D — Training pipeline (MENS) and corpus

41. **[P0] Reach the 100k organic_vox.jsonl threshold before claiming anything about CPT.** This is explicitly the gating condition in `vox-lang-training-ssot-2026.md`. Publish current count weekly on the dashboard. Without it, CPT is a lie.
42. **[P1] Synthetic corpus generator from golden templates.** Use existing foundation models + `vox check` to generate *and verify* millions of Vox examples from parameterized templates. Compiler-verified synthesis is one of Vox's unique training advantages.
43. **[P1] Close MENS Gap A (full-graph Candle QLoRA).** Proxy-graph backward pass blocks loss-parity with Burn. This is the highest-leverage MENS technical item.
44. **[P1] Close MENS Gap D (nested-LoRA serving in Candle).** `merge-qlora` → `.safetensors` works, but inference server can't consume nested adapters. Without this, iterative fine-tune + eval is crippled.
45. **[P1] GRPO reward-shaping upgrade per Cluster A1.** Replace additive reward with `R = r_syntax × (w1·r_test + w2·r_coverage)`; add negative sampling for parse failures; curriculum seeding. The research is ready; ship the implementation.
46. **[P2] Close MENS Gap B (MoE).** Qwen3-Coder is MoE; you're training on the wrong architecture long-term.
47. **[P2] Close MENS Gaps C (RoPE+LoRA merge in Burn) and E (research-reasoner lane).** Defer if needed but name ETAs.
48. **[P2] Publish the training pipeline as a reproducible runbook.** A single `vox populi train --config qlora-qwen35-vox.toml` that any researcher with a 16GB GPU can reproduce, plus a `docs/src/tutorials/train-your-own-vox-model.md`. External reproducibility is marketing and credibility in one motion.
49. **[P2] Benchmark continuous-learning safety.** The grand-strategy-seed flags "structure snowballing" and base-model collapse. Implement the anti-collapse penalty and publish before/after loss curves.
50. **[P3] Federated training inside Populi.** Long-term wedge: any team with a few spare consumer GPUs can contribute to a pooled Vox adapter training run, with license provenance tracked per contribution. Very on-brand; not urgent.

### Category E — Agent orchestration & runtime

51. **[P1] Formalize Orchestrator state machine.** OOPAV (Understand → Plan → Act → Verify) exists in docs, not in `runtime.rs`. Encode it as a `TaskState` enum with transition guards, measurable from telemetry.
52. **[P1] Trust-tier RBAC in the orchestrator.** `NodeRecord` carries a `trust_tier`; `Orchestrator::dispatch` ignores it. Wire a capability-matrix check (sensitive ops: filesystem writes, secret reads, network egress) so low-trust nodes can't do high-impact things.
53. **[P1] Workflow durability parity between interpreted and generated Rust paths.** ADR 021 is a *design gate*, not an implemented feature; the two paths can diverge silently. Ship a determinism test per workflow.
54. **[P2] HTN pod manager.** Multi-agent hierarchical task network is promised; deliver a minimum `PodManager` that decomposes `goal -> tasks -> sub-agents` with observable state transitions.
55. **[P2] Populi lease-based remote execution.** ADR 017 is also design-intent only. Ship the minimal lease semantics so "run this task on that GPU node" actually works.
56. **[P2] Consolidate MCP tool count.** 102 tools is too many for most LLMs to context-window gracefully. Segment into `core` / `dev` / `advanced` with a loading manifest. LLMs should get 20–30 tools by default, not 102.
57. **[P2] Orchestrator backend selector hardening.** `VOX_MCP_ORCHESTRATOR_RPC_READS` / `_WRITES` pilots are useful but a per-tool feature flag explosion. Ship the "write pilots general availability" milestone, then retire the env-flag lattice.
58. **[P2] Socrates gating on all agent-emitted code.** Today Socrates is mostly for RAG answers; make it also gate `@island`, `@server`, and workflow generations against tests + coverage before accepting.
59. **[P3] Circuit breakers on sub-agents.** Cluster A2 / A3 research mandates them; hasn't landed.

### Category F — Developer experience (LSP, CLI, errors, docs-in-IDE)

60. **[P1] Incremental LSP backed by the real compiler.** `vox-lsp` is 1.3 kLOC and decoupled from the compiler. Connect it to the HIR validator so edits get live diagnostics, not stale ones. Also surface `correction_hint` as LSP quickfixes.
61. **[P1] Single canonical error renderer.** Today errors are formatted by both `vox check` (miette) and CLI-local code. Converge on one renderer with `--format human|json|sarif`.
62. **[P1] `vox doctor` should be opinionated and actionable.** Right now it's a toolchain/env probe; extend to "your project is missing a Clavis ID for FOO"; "your `.voxignore` is stale"; "your MSRV is behind by N minors."
63. **[P2] Cut CLI surface from ~280 subcommands to a tiered model.** `vox --recommended` exists; enforce it. Move `ludus`, `opencode`, `ars`, `schola`, etc., behind `vox ext` namespace.
64. **[P2] Make `vox check` return an SSE stream of structured diagnostics.** Agents benefit far more from live streaming than from waiting for a monolithic JSON blob.
65. **[P2] Ship a VS Code snippet pack from golden examples.** 90% of what a new user types is a minor variation on a golden; generate snippets deterministically from `examples/golden/*.vox`.
66. **[P2] "Playground" CLI: `vox play`.** A zero-install REPL that ships a single binary (embedded Vite + SQLite) so a brand-new user hits "hello full-stack" in 60 seconds. This is the most underrated adoption lever.
67. **[P2] First-class "copy this prompt into Claude/Cursor/Codex" surface.** `vox llm prompt <task>` that prints the ideal system prompt + context files + MCP tool list for the current repo. Removes friction between Vox and frontier agents.
68. **[P3] Error renderer that includes a fix-me patch (unified-diff) when correction_hint is present.** Tiny UX win; huge when you've got the hints landed.

### Category G — Documentation consolidation (stop the doc bloat)

69. **[P0] Archive research docs older than 60 days without status ≥ current.** 273 architecture docs is unmanageable. Publish a two-week freeze; anything without a `status: current` stamp moves to `docs/src/archive/research-2026-q1/`. Re-surface only what is load-bearing today.
70. **[P0] One "what is shipped today" page, auto-generated from CI.** Stop asking humans to reason about `tier: stable | preview | experimental` maps. `vox ci capability-snapshot` should emit `docs/src/reference/shipped-v0.4.md` weekly. The README table of tiers should be a rendered fragment, not hand-edited.
71. **[P1] Collapse the SSoT proliferation.** You have `canonical-map.v1.yaml` plus operations catalog plus MCP registry plus CLI registry plus capability registry plus language-surface plus telemetry-trust plus completion-policy. Promote `canonical-map.v1.yaml` to actual root-of-truth; every other SSoT becomes a *view* of it.
72. **[P1] Every roadmap doc must carry an owner and an ETA.** Half of the `*-roadmap-2026.md` files lack both. No owner, no ETA = close or archive.
73. **[P1] Retire the HTMX, Pico, TanStack-virtual-routes ghosts.** They linger in paragraphs across a dozen docs. Global find-replace + CI guard for banned terms.
74. **[P2] Diátaxis audit by agent, not by human.** Run a `vox scientia` job over the doc tree that classifies every page as tutorial / how-to / reference / explanation; flag the misclassified. You already have the infra.
75. **[P2] Publish a 500-word "Vox in one page" external explainer.** The README is excellent but 445 lines. There is no two-paragraph version for a skeptical engineer. Write one, pin it on the landing page.
76. **[P3] One-per-quarter ADR retrospective.** Which ADRs actually shaped code? Which became dead letters? Retire the dead ones.

### Category H — SSOT convergence & CI

77. **[P1] Finish Stream H (language surface SSOT generator).** Every other SSoT is downstream of it; adding a decorator should be a single-file edit that cascades. This is named as a blocker in `orphan-surface-inventory.md` and yet is still in progress.
78. **[P1] One doc-to-code drift CI job.** Replace the half-dozen guard commands with a single `vox ci drift` that fans out. Guard commands should not be a per-incident response surface.
79. **[P1] CI build-time budgets enforced, not advisory.** Today `VOX_BUILD_TIMINGS_BUDGET_FAIL=1` is opt-in. Make it the default for main; block merges that blow past budgets.
80. **[P2] Workspace `cargo-hakari` (workspace-hack) upkeep verified in CI.** Dependency cone keeps expanding; hakari lapses silently undo its value.
81. **[P2] `vox ci completion-audit` must block merges, not just report.** 768 task IDs is impressive; enforcement is what makes it credible.
82. **[P2] Dependency sprawl guard.** `dependency-sprawl-research-2026.md` exists; implement `vox ci dep-sprawl` with a real per-crate cap.
83. **[P3] Single GitHub/GitLab unified runner contract.** Today there's drift between the two. Consolidate.

### Category I — Ecosystem, adoption, external credibility

84. **[P0] Pick a marquee app and ship it in public Vox, on a real domain, with telemetry enabled.** A public blog, a small SaaS landing page, or an open-source Slack clone written 100% in Vox, deployed, and readable. No claim will land without a production reference.
85. **[P1] Language-benchmark game: Vox vs. Next.js vs. Phoenix LiveView.** Pick 5 canonical full-stack tasks (auth, CRUD, background jobs, realtime, uploads). Build each in all three, publish LOC, time-to-complete, bundle size, and LLM one-shot correctness. This is linkable for years.
86. **[P1] A single pinned landing-page demo: "Write a full app by talking to it."** Oratio + MENS + scaffold: "I want a habit tracker with SQLite, email reset, and a charts page." Ship the recording. *This is the only thing that makes "premier LLM destination" intuitively real to outsiders.*
87. **[P1] Publish a Vox-on-Claude/Cursor/Gemini/Codex/Cody compatibility matrix.** `docs/agents/ai-ide-feature-matrix-2026.json` exists; render it as an HTML page on the site, updated weekly.
88. **[P2] Submit a `tree-sitter-vox` to GitHub's linguist registry.** Without linguist inclusion, Vox files don't get syntax highlighting on GitHub; every Vox repo looks amateur.
89. **[P2] Stand up a public MCP endpoint hosting Vox tooling for free.** Let any agent anywhere call `vox_check`, `vox_lint`, `vox_explain` without install. Adoption goes up by an order of magnitude when there's zero install.
90. **[P2] Weekly `vox status` post.** A single blog post format: shipped / in flight / numbers this week / research highlights. Consistency beats scope for building a following.
91. **[P2] Onboarding funnel instrumentation.** You have telemetry; most projects don't measure "time-from-clone-to-working-app". Target sub-10-minute; publish the number.
92. **[P3] Formal governance for the Foundation.** Right now the repo says "backed by Open Collective" but governance structure isn't documented. Write up the bylaws before anyone asks.

### Category J — Scope discipline & organizational shape

93. **[P0] Pick 10 crates that are v1.0 and deprecate or feature-gate the rest.** Ops reality: compiler, cli, db, clavis, runtime, orchestrator, populi, tensor (MENS), search (Scientia), toestub. Everything else is experimental until the top 10 are green. Publish the list and the freeze.
94. **[P0] Name a v1.0 release criterion publicly.** You cannot have a "stable" tier and also be at v0.4 with breaking parser regressions. Pick 3 measurable criteria (e.g. "100 % of golden corpus stays green for 60 days", "HumanEval-Vox published", "one marquee app in production"), publish, and let them drive prioritization.
95. **[P1] "Research → blueprint → backlog" pipeline gate.** Research docs are proliferating without obvious downstream outcomes. Rule: a `*-research-2026.md` that hasn't produced an `*-implementation-plan-*.md` in 90 days is auto-archived.
96. **[P1] Per-quarter scope budget for new crates.** No new top-level crate without retiring one. Post the crate ledger in README.
97. **[P1] Make `vox-dei` either merge-back or close.** It's a stub `lib.rs` carried in the workspace with zero content and an active CI guard forbidding imports of it. Either delete or resurrect with real code.
98. **[P2] Deprecate `vox-py` exclusion and all Python glue entirely.** AGENTS.md already forbids new Python glue; finish the migration of the remaining `scripts/*.py` and `scripts_update*.py` at repo root.
99. **[P2] Prune the 30+ `scratch_*`, `tmp_*`, `scratch_update*.py` root-level files.** They leak into Mens corpus and clutter contributor searches. Move to `scratch/` or delete.
100. **[P2] Cut `target-*` sibling directories (`target-stubcheck`, `target-ws-check`, `target-agent-verify`).** These are CI artifacts that shouldn't be committed; add them to `.gitignore` and purge from working copies.
101. **[P3] Monorepo ergonomics.** Consider `cargo-nextest` as the canonical test runner + parallel shards; measure CI wall time improvements; document.
102. **[P3] Consider ReleaseTrain-style cadence.** A public release every 6 weeks with a predictable feature surface helps outside contributors plan; today's release cadence is invisible.

## Part III — Sequencing: what to do first

If resources are scarce, the ordering that preserves the most credibility fastest is:

1. **Week 1 — Bleeding stops.** Items 1, 2, 3, 4, 69, 70, 93, 94. (Parser/build stops failing; scope gets honest in public.)
2. **Weeks 2–4 — Flagship demo and bench.** Items 17, 28, 33, 84, 85, 86. (You need a public benchmark, a public adapter, and a public app. Without these, the pitch floats.)
3. **Months 2–3 — Pillar repair.** Items 15, 16, 18, 19, 30, 31, 41, 51, 60, 77. (Vox-native reactivity lands; LLM repair loop real; trained adapter real; orchestrator formalizes.)
4. **Months 4–6 — Ecosystem.** Items 23, 26, 35, 87, 88, 89, 90. (Second backend; Next adapter; GitHub linguist; weekly cadence; public MCP endpoint.)
5. **Ongoing — Hygiene.** Everything else, with SSoT convergence (77, 78) and doc archival (69) as the two steady-state rituals.

## Part IV — What this audit cannot decide for you

Four questions meaningfully reshape the above priority list. They are asked separately via `AskUserQuestion`; answer them and priorities will shift.

- The audience you optimize for (labs adopting Vox vs. devs shipping apps vs. enterprises vs. contributors).
- Your web-framework fork: React-embedded-forever vs. Vox-native reactivity vs. multi-backend.
- Your team scale (determines whether 102-item list is a backlog or a fantasy).
- Your v1.0 gating criterion (determines what "done" looks like).

— End of audit.
