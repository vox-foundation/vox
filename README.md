<!--
  Maintaining this README:
  - Status tables: when you verify a row, update its State, its "Checked" date, and its note.
    Leave rows you didn't check alone; a stale date is honest, a silently-edited row is not.
  - States are exactly: Working / Prototype / Research / Not started. Nothing goes above Working.
  - Log: append-only, newest first. One line per milestone or finding; link the doc or commit.
  - Bump "Last reviewed" only after walking every status table.
  - Avoid undated counts (crates, LoC, tool totals); they rot silently.
  - The why_vox block is lint-synced with docs/src/index.mdx; edit both together.
  - Bug-level detail and handoffs live in docs/src/architecture/suite-status-audit-2026-09-21.md.
-->
<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>Vox is an experimental, AI-native development suite: a language and compiler, a CLI, an agent harness, an operator GUI, and a native ML lane, built as one research project.</strong></p>

  <p>Led by Bertrand Reyna-Brainerd · stewarded by the Vox Foundation (in formation) · Apache-2.0 · <strong>not production software</strong></p>

  <p><a href="https://voxlang.org">voxlang.org</a></p>
</div>

<p align="center">
  <img src="https://img.shields.io/badge/status-research%20prototype-red?style=flat-square" alt="Status: research prototype"/>
  <a href="https://github.com/vox-foundation/vox/commits/main"><img src="https://img.shields.io/github/last-commit/vox-foundation/vox?style=flat-square&label=updated" alt="Last Updated"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License"/></a>
</p>

> [!WARNING]
> **Last reviewed: 2026-09-21.** Very little here is finished. Of the 70 components
> checked on that date, fewer than a fifth do their basic job reliably, and none are
> hardened. There are no releases. The syntax still breaks. Main-branch CI is red.
> As far as I know, nobody outside the project depends on any of it. Treat the repo
> as a lab notebook, and please don't build anything you rely on with it.

<div align="center">
  <blockquote>
    <p><em>"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence!"</em></p>
    <p>— Nathaniel Hawthorne, <em>The House of the Seven Gables</em> (1851)</p>
  </blockquote>
</div>

## What Vox is

Vox began as a programming language designed for LLM code generation (see [The language](#the-language)). To build and test that language with AI agents, I kept building the tools around it, and the tools became most of the project. Today it is one repository and one `vox` binary covering:

- **Language & compiler.** `.vox` source compiles to a TypeScript/React frontend, a Rust (Axum) server, and a SQL schema, or runs directly in an interpreter.
- **CLI (`vox`).** Build, run, check, scaffold, deploy, and a large surface of project and repo tooling.
- **Agent harness.** An orchestrator daemon, an MCP tool server, model registry and routing, skills (agentskills.io format), and plugins.
- **Interfaces.** A Tauri operator GUI for driving agents, a VS Code extension, and a terminal UI.
- **ML lane (MENS).** A Rust-native (Candle) QLoRA trainer and inference server aimed at a Vox-specialised model.
- **Research tools.** Deep research, SCIENTIA publication, hybrid search, a code-intelligence graph.
- **Dev infrastructure.** CI gates, architecture and source-policy audits, docs pipeline, secrets (Clavis), telemetry.

Many of these are feature-gated or optional plugins, so the default binary does not include all of them. Each row in the tables below notes when that applies.

## How it's built, and who it's for

I'm the only maintainer, and I write most of the code with AI coding agents. That is part of the experiment: as of 2026-09-21, roughly nine in ten commits were authored or co-authored by an AI agent. It also explains the repo's shape. Breadth grew very fast, while depth, hardening, and outside validation lag far behind. A large share of commits fix recently landed work. I keep audits that are critical of the project in-tree under [`docs/src/architecture/`](docs/src/architecture/) rather than smoothing them over.

Vox is stewarded by the **Vox Foundation**, which is still being set up as of 2026-09. Donors will help steer what the project works on. The work itself is free and open source under Apache-2.0: whatever comes out of it belongs to everyone. Support goes through [Open Collective](https://opencollective.com/vox-foundation), which has a public ledger.

## Status

The four states, in order of how much you can lean on them:

- **Working.** Does its basic job when tried. Not hardened, no stability promise.
- **Prototype.** Code exists and partly works. Expect gaps and wrong behaviour.
- **Research.** Exploratory. Not working as intended yet.
- **Not started.** Planned only, or a stub.

Rows checked on 2026-09-21 were verified two ways: by running an installed `vox 0.6.0+build.5357`, which is about 450 commits behind `main`, and by reading current `main` source. Where the two differ, the note describes `main`. Per-bug detail: [suite status audit](docs/src/architecture/suite-status-audit-2026-09-21.md).

### Language & compiler

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| Parser & type checker (`vox check`) | Working | 2026-09-21 | 85 of 87 golden examples check; the 2 failures come from a commit newer than the tested build. Grammar still changes; last break 2026-08-09. |
| Interpreter (`vox run` on scripts) | Working | 2026-09-21 | Runs `fn main()` scripts, including this repo's own automation (46 of 51 `scripts/*.vox` type-check). Older builds default to the cargo lane; use `--interp` there. |
| Full-stack codegen (`vox build`) | Prototype | 2026-09-21 | Emits TS/React, an Axum server, and SQL DDL. The generated server's `Cargo.toml` points into this repo by relative path, so it only builds inside a Vox checkout. No test builds a generated app outside the repo or serves requests from one. |
| OpenAPI & client emit (`vox emit`) | Prototype | 2026-09-21 | OpenAPI works. Client emit fails with a write error on any file containing a `component`. |
| Formatter (`vox fmt`) | Prototype | 2026-09-21 | **Deletes comments and silently drops `workflow`/`activity` declarations.** Don't run it on files you care about. |
| Tests (`vox test`, `@test`) | Prototype | 2026-09-21 | Goes through generated Rust and cargo, so it shares the inside-a-checkout limitation. |
| REPL (`vox repl`) | Prototype | 2026-09-21 | Single expressions only; `let`/`fn` don't persist between lines. |
| Dev loop (`vox dev`) | Working | 2026-09-21 | Watches and rebuilds. |
| Durable workflows (`workflow`/`activity`) | Prototype | 2026-09-21 | Interpreter support for a restricted subset. Crash-replay durability is still [design](docs/src/architecture/true-workflow-durability-design-2026.md). |
| Mobile (`--target mobile`, React Native) | Research | 2026-09-21 | Emits an Expo project. The device runtime is thin, and the iOS E2E workflow has never passed. |
| Desktop packaging (`vox compile`) | Prototype | 2026-09-21 | Emits a Tauri project. Unit tests only; no end-to-end packaging test. |
| LSP (`vox lsp`) | Prototype | 2026-09-21 | Diagnostics, hover, completion, symbols, semantic tokens, code actions. No go-to-definition. Formatting uses the broken formatter. No installer ships the `vox-lsp` binary. |
| Scaffolding (`vox init`, `vox new`) | Prototype | 2026-09-21 | `vox run` fails on the `vox init` starter (fix pending), and the starter trips its own `id`-column lint. `vox new web` writes 2 files. |
| Packages (`vox add`/`lock`/`sync`) | Prototype | 2026-09-21 | Manifest editing works locally. |
| Package registry (`vox pm`) | Not started | 2026-09-21 | The default registry URL doesn't exist. |
| Grammar export (`vox grammar`) | Working | 2026-09-21 | EBNF, Lark, JSON-Schema. |
| Deploy & share (`vox deploy`, `vox share`) | Prototype | 2026-09-21 | Target scaffolding and dry-runs. Real deployment of a generated app is untested. |

### Agent harness

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| Orchestrator daemon | Prototype | 2026-09-21 | Answers RPC (spawn agent, status, tasks). A real task run was not tried. Churn is high. |
| MCP server (`vox mcp`) | Prototype | 2026-09-21 | 338 tools registered. A test checks that each compiled-in tool has a handler, and one is an explicit stub. Most have no behavioural tests. Not in the default build (`--features mcp-server`). |
| Chat (`vox chat`) | Prototype | 2026-09-21 | Defaults to OpenRouter. A fixed 30s request timeout is too short for a cold local model. |
| Harness eval (`vox harness eval`) | Working | 2026-09-21 | Offline golden set passes; live tasks are opt-in. |
| Model registry & routing (`vox model`) | Prototype | 2026-09-21 | Catalog listing works. Scoreboard and cost views are empty on a fresh DB. |
| Planning / dispatch preview | Prototype | 2026-09-21 | `vox plan` is thin. `vox dispatch preview` prints "not yet wired", and `vox workflow ls` is a stub that never contacts the daemon. |
| Rollback (`vox rollback`) | Prototype | 2026-09-21 | Starts a fresh daemon per call rather than talking to the running one, and overwrites the running daemon's token file. |
| Memory search (`vox memory search`) | Prototype | 2026-09-21 | Retrieval runs, but the CLI never prints the hits. Web search is part of the default plan. |
| Skills (`vox skill`) | Prototype | 2026-09-21 | Registry is tested. The CLI is not in the default build (`--features ars`). |
| Plugins & bundles (`vox plugin`, `vox bundle`) | Prototype | 2026-09-21 | List, info, and doctor work. 11 of 18 catalog plugins point at repos that don't exist. |
| LLM repair & generate (`vox repair`, `vox generate`) | Prototype | 2026-09-21 | `repair` calls OpenRouter directly. `generate` probes endpoints a stock Ollama server doesn't have. |
| Policy & exec-policy (`vox policy`, `vox shell check`) | Working | 2026-09-21 | Lists the policy catalog; allows and blocks shell commands as specified. |
| Identity & auth (`vox auth`, `vox login`) | Prototype | 2026-09-21 | Local trust store works; login flow not exercised. |
| OpenClaw runtime | Research | 2026-09-21 | CLI and MCP tools behind `--features ars`; little recent work. |

### Interfaces

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| Operator GUI (`vox-gui`, Tauri) | Prototype | 2026-09-21 | 36 routed surfaces, all calling registered backend commands. E2E tests run against mocks, and the release build workflow is failing. The highest-churn crate since July. It does not compile `.vox` apps into desktop apps. |
| GUI chat & agent flow | Prototype | 2026-09-21 | Needs the orchestrator daemon. "Run on mesh" is always empty, because mesh transport isn't compiled in. |
| GUI research surface | Prototype | 2026-09-21 | The daemon ignores the lane toggle, and doc drafts are hardcoded placeholder text. |
| VS Code extension | Prototype | 2026-09-21 | Compiles. Not packaged or published, and has no tests. Its extension ID collides with an unrelated published extension. See [`apps/editor/vox-vscode`](apps/editor/vox-vscode/). |
| Terminal UI (`vox term`) | Research | 2026-09-21 | Draws a prompt, but submitted input is discarded. The promised headless mode isn't implemented. |
| Speech-to-code (`vox speech`) | Research | 2026-09-21 | Whisper code exists behind a default-off feature. The stock command errors. |
| Visual review (Visus) | Research | 2026-09-21 | Screenshot plus VLM audit. Not in the default build. |
| Installer & updater (`voxup`, `vox upgrade`) | Prototype | 2026-09-21 | Code exists, but there are no releases to install. `vox upgrade` wrongly reports "up to date". |
| Docs site ([voxlang.org](https://voxlang.org)) | Prototype | 2026-09-21 | Builds. Every deploy since 2026-05-12 has failed at the Cloudflare step, so the live site is stale; its feed stops at 2026-05-15. |

### ML lane (MENS)

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| Fine-tuning (`vox mens train`) | Research | 2026-09-21 | Trainer runs on Metal. Gradient accumulation discards most gradients, and the training data is thin (about 7k rows, about 1.2k unique responses). |
| Serving (`vox mens serve`) | Prototype | 2026-09-21 | Non-streaming only. Needs an `execution-api` + GPU build or the [`vox-ml` bundle](docs/src/how-to/how-to-migrate-from-cargo-features.md). |
| Corpus & eval gates | Prototype | 2026-09-21 | Extraction runs. The eval prompt format differs from training, so its scores don't measure the trained setup. |
| Quantization / GGUF export | Prototype | 2026-09-21 | Library is tested. There's no standalone GGUF export; it goes through `merge-qlora --gguf-out` with llama.cpp. |
| Grammar-constrained decoding | Not started | 2026-09-21 | The crate exists and is declared as a dependency, but nothing calls it. |
| Released model weights | Not started | 2026-09-21 | Nothing is published, and no trained model is worth using yet. |
| Mesh control plane (`vox populi`) | Prototype | 2026-09-21 | Status and a bearer-authenticated control server exist. Multi-node flows untested. |
| Distributed training & routing | Research | 2026-09-21 | Off-process routing is off by default. The cloud training path has never completed a job. |

### Research tools

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| Deep research (`vox research`) | Prototype | 2026-09-21 | A 2026-09-20 review found off-topic results scored as good, because judge-parsing failures fell back to a constant score. Fixes are not yet merged. |
| SCIENTIA & publication | Prototype | 2026-09-21 | Large and heavily tested. Not exercised end to end in this review. |
| Socrates evidence gate | Prototype | 2026-09-21 | Wired into task submit. Enforcement and fusion are both off by default. |
| Hybrid search (`vox-search`) | Prototype | 2026-09-21 | See the memory search row: results are computed but never shown. |
| Code graph (`vox graph`) | Working | 2026-09-21 | Query finds definitions. Ranking is nearly flat. Needs `vox graph refresh --auto` once per clone. |
| Multi-repo catalog (`vox repo`, `vox catalog`) | Working | 2026-09-21 | Add, list, and text query work. |

### Dev infrastructure

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| CI gate runner (`vox ci`) | Working | 2026-09-21 | Cheap guards pass. Refuses to run from a stale binary, by design. |
| Architecture checker (`vox-arch-check`) | Working | 2026-09-21 | Finds real layer violations in the current tree. |
| Drift checker & docs pipeline | Working | 2026-09-21 | Both produce real findings. |
| Source-policy audits (`vox audit code`) | Prototype | 2026-09-21 | Large detector set. A full run didn't finish in 2 minutes, and it can't be scoped to a path. |
| Commit effort audit (`vox audit effort`) | Prototype | 2026-09-21 | End-to-end with a mock judge; the real LLM judge is untried. |
| Secrets (Clavis, `vox secrets`) | Working | 2026-09-21 | Doctor reports sources and gaps correctly. |
| Config & capability registry | Working | 2026-09-21 | |
| Local database (VoxDB) | Prototype | 2026-09-21 | Additive migrations only. A DB newer than the binary is reported as "legacy", with advice to export and re-create it. |
| Telemetry | Prototype | 2026-09-21 | Records locally. Nothing consumes it, and the local spool grows without bound even with upload off. |
| Build broker (cargo shim) | Prototype | 2026-09-21 | Exists but not installed on the checked machine. |
| VCS layer (jj-lib) | Prototype | 2026-09-21 | Used inside the orchestrator; no CLI. |
| Script sandbox (`--caps`) | Prototype | 2026-09-21 | Present with tests. Without `--caps`, local scripts get every capability ([ADR-048](docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md)). |
| WASM / container skill sandboxes | Research | 2026-09-21 | Optional plugins; nothing depends on them. |
| Snapshot cleanup (`vox snapshot orphans`) | Prototype | 2026-09-21 | Resolves every path wrongly and reports success. |
| Self-hosted CI fleet | Prototype | 2026-09-21 | Fleet down (0 of 2 runners); six main-branch workflows failing. |

### The central claim

| Component | State | Checked | Notes |
|:---|:---|:---|:---|
| Evidence that Vox improves LLM code generation | Not started | 2026-09-21 | No valid benchmark yet. An eval harness is designed but unrun. The figures in [`language-benchmark-2026.md`](docs/src/architecture/language-benchmark-2026.md) have no data behind them. |

Longer-horizon targets, mostly unbuilt: [v1.0 release criteria](docs/src/architecture/v1-release-criteria.md). Per-surface detail: [stability matrix](docs/src/reference/stability.md), which still uses the older, overstated grades and is being re-graded to these states.

## Log

Newest first.

- **2026-09-21**: Audited the whole suite against the code ([findings and bug handoff](docs/src/architecture/suite-status-audit-2026-09-21.md)) and rewrote this README around it. The previous README called several areas "Stable" or "Mature" that aren't, and its quick start failed.
- **2026-09-20**: MENS pipeline review. The trainer was discarding most gradients, and published eval numbers had no artifacts behind them.
- **2026-09-20**: Deep-research review. The planner was skipped on the default lane, judge scores were constant fallbacks, and fallback model slugs were dead.
- **2026-09-10**: [GUI chat/model-knobs audit](docs/src/architecture/chat-ui-model-knobs-audit-2026-09-10.md).
- **2026-09-08**: [ADR-048](docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md) makes the interpreter the execution and sandbox tier, retiring the WASM, container, and microVM isolation modes.
- **2026-08-09**: Core-syntax convergence (#469).
- **2026-06-30**: Retired `@`-decorator spellings became hard parse errors; data-layer declarations are now bare keywords.
- **2026-06-05**: Hardened the [v1.0 criteria](docs/src/architecture/v1-release-criteria.md) after an audit found the compiler's own correctness was ungated.
- **2026-05-26**: v0.6.0 ([CHANGELOG](CHANGELOG.md)). `@endpoint` retired, telemetry unified, single-machine multi-agent orchestration.
- **2026-05-12**: The Tauri operator GUI replaced the old Axum dashboard.
- **2026-05-08**: Plugin system redesign: slim core, runtime-loadable plugins, skill marketplace.
- **2026-02-17**: Git history begins, with an import of the existing compiler: lexer, parser, type checker, LSP, and Rust/TypeScript/WASM/LLVM codegen.

## The language

The suite exists to serve this question, so the language is still the core research bet.

<!-- ANCHOR: why_vox -->
## Why Vox

Mainstream languages predate LLMs by decades. They tolerate implicit state — nulls, exceptions, schemas restated three times across the stack. That's tractable for a person; it's a minefield for a statistical code generator. A million-token context window doesn't help when most of it is integration boilerplate.

<div align="center">
  <img src="docs/src/assets/old_internet_knot_abstract.png" alt="A diagram illustrating the complexity of traditional web development fragmentation." width="80%" />
  <p>
    <strong>Fragmentation in Traditional Web Development</strong><br />
    Traditional development requires restating data models and logic across frontend, API, backend, and database layers. This duplication creates significant maintenance overhead and increases the risk of integration drift.
  </p>
</div>

Vox is what falls out when you design the language *after* the model: collapse the duplications, push errors into the type system, draw the browser/server boundary in one place, and build durability and tool exposure into the grammar instead of layering them on top.
<!-- ANCHOR_END: why_vox -->

That is a hypothesis, not a result. It hasn't been measured yet (see [The central claim](#the-central-claim)).

In practice, one `.vox` file declares tables, queries, mutations, server functions, UI components, and routes. The compiler is designed to split it along a domain boundary: logic goes to Rust, and browser code goes to TypeScript. The check that would enforce that split isn't built yet. The grammar rule is that bare keywords (`table`, `query`, `component`, `workflow`, …) open scopes, and decorators (`@auth`, `@uses`, `@pure`, `@scheduled`, …) modify declarations. The syntax is still settling. [`AGENTS.md`](AGENTS.md) lists what has been retired, and the [where things live](docs/src/architecture/where-things-live.md) table maps concepts to crates.

## Trying it

There are no releases, so build from source. You need Rust **1.98.1** (pinned in `rust-toolchain.toml`) and a C compiler. Node and pnpm are needed for generated frontends or the GUI.

```bash
git clone https://github.com/vox-foundation/vox.git
cd vox
cargo install --locked --path crates/vox-cli
vox doctor
```

Scaffold a project and inspect what the compiler emits:

```bash
vox init my-app
cd my-app
vox check src/main.vox
vox build src/main.vox -o dist
```

The TypeScript lands in `dist/`. The generated Rust server lands in `src/target/generated/`.

> **Known issues (2026-09-21):**
> - `vox run src/main.vox` fails on the scaffolded project. A routing fix is pending.
> - The generated server only builds when the output sits inside a clone of this repo, because its `Cargo.toml` points at Vox's own crates by relative path. A fix is in progress.
> - `vox fmt` deletes comments and drops some declarations.
> - `vox doctor` exits 0 even when checks fail, and some of its hints are wrong on macOS.

More: [Installing Vox](docs/src/reference/installation.md) · [CLI reference](docs/src/reference/cli.md).

## The CLI

The full CLI surface, including every `vox ci`, `vox populi`, and `vox mens` subcommand, lives at [`docs/src/reference/cli.md`](docs/src/reference/cli.md). Run `vox commands --recommended` for first-time discovery.

## Contributing

Curiosity, criticism, and experiments are welcome. Expect things to move under you. Start at the [Contributor Hub](docs/src/contributors/contributor-hub.md). The [TOESTUB Guide](docs/src/contributors/toestub-contributor-guide.md) covers common CI gate failures. Repo rules for humans and agents are in [`AGENTS.md`](AGENTS.md).

## License & contact

[Apache 2.0](LICENSE). Discussion: [GitHub Issues](https://github.com/vox-foundation/vox/issues). Support the Vox Foundation: [Open Collective](https://opencollective.com/vox-foundation).
