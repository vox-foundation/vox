<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>A programming language for human intent and machine execution. One <code>.vox</code> file compiles to a database schema, a type-safe server, and live browser code.</strong></p>
  <p><a href="https://vox-lang.org"><strong>vox-lang.org</strong></a></p>

</div>

<p align="center">
  <a href="https://vox-lang.org"><img src="https://img.shields.io/badge/docs-vox--lang.org-blue?style=flat-square" alt="Documentation"/></a>
  <a href="https://github.com/vox-foundation/vox/commits/main"><img src="https://img.shields.io/github/last-commit/vox-foundation/vox?style=flat-square&label=updated" alt="Last Updated"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License"/></a>
  <a href="https://vox-lang.org/feed.xml"><img src="https://img.shields.io/badge/RSS-updates-orange?style=flat-square" alt="RSS Feed"/></a>
</p>

---
<!-- Code examples in this file should mirror examples/golden/*.vox -->
<!-- Run: vox check examples/golden/*.vox to verify -->

<br>
<div align="center">
  <blockquote>
    <p><em>"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence!"</em></p>
    <p>— Nathaniel Hawthorne, <em>The House of the Seven Gables</em> (1851)</p>
  </blockquote>
</div>

---

<!-- ANCHOR: how_vox -->
<!-- ANCHOR: why_vox -->
## The Architecture: Designed for AI and Humans

Programming languages predate LLMs by decades. JavaScript's dynamic typing fails silently at runtime, C++'s pointer mutation hides state, and Python's configuration layers run deep. While human developers manage these trade-offs, for an AI agent navigating them simultaneously, they compound into hallucination.

A million-token context window sounds generous until the signal is buried in boilerplate<sup>[1](#ref1)</sup>. Decades of patching the object-relational impedance mismatch<sup>[2](#ref2)</sup> have ballooned the accidental complexity<sup>[3](#ref3)</sup> and technical debt of modern systems<sup>[4](#ref4)</sup>, leaving codebases too brittle for agents to safely refactor.
<!-- ANCHOR_END: why_vox -->

### Platform Architecture & Stability

Stability is stratified by **model predictability**. Core surfaces (data, logic, memory) lock first; rendering surfaces remain fluid.

#### Stability Tiers
* 🟢 **Stable** — rules locked; LLM output is deterministic.
* 🟡 **Preview** — functionally complete; execution pipelines still optimizing.
* 🚧 **Experimental** — under active design; not deployable.

#### Domain Matrix

| Domain & Purpose | What It Manages | Tier Status & Impact | Verification Pipeline |
|:-----------------|:----------------|:---------------------|:----------------------|
| **Core Syntax & Engine**<br>Language foundation. | AST, type safety, compiler directives, LSP. | 🟢 **Stable**<br>Syntax rules are locked; generation is highly predictable. | Golden parsing suite, typed AST validations. |
| **Data & Connectivity**<br>How data is saved and shared. | `@table` auto-migrations, `@query`/`@server` endpoints, HTTP payloads. | 🟢 **Stable**<br>API contracts are functionally complete. | In-memory DB roundtrips, strict schema testing. |
| **Agent Tooling System**<br>AI access to external actions. | Orchestration logic, `@mcp.tool` exposure, telemetry. | 🟢 **Stable**<br>Complete Model Context Protocol compliance is established. | MCP protocol assertions, telemetry gate checks. |
| **RAG & Knowledge Curation**<br>Memory for autonomous research. | `vox scientia` pipeline, Hallucination Guards (Socrates). | 🟡 **Preview**<br>Retrieval heuristics and Socrates guard policies are actively evolving. | Citation alignment checks, novelty discovery scans. |
| **Durable Execution**<br>Multi-step tasks and continuity. | State survival via `workflow` and `actor` models. | 🟡 **Preview**<br>State preservation lifecycles may undergo optimization. | Durability integrity sweeps, zero-placeholder enforcement. |
| **Hardware & Tuning (MENS)**<br>Local AI training and inference. | `vox populi` GPU mesh, adapter training, audio inference. | 🟡 **Preview**<br>Hardware-dependent support mappings are expanding. | Local hardware discovery tests, ML pipeline sweeps. |
| **Web UI & Rendering**<br>What the user sees. | `@island` browser wiring, React generation, UI routing. | 🟡 **Preview**<br>Client-side projections and web component translation may shift. | WebIR constraints, deterministic generation audits. |
| **Distributed Node Mesh**<br>Cross-machine coordination. | Cross-machine inference routing, agent task distribution. | 🚧 **Experimental**<br>Still under active design; not ready for deployment. | Pending standardizations. |

*(v0.4, April 2026)*

<br>

<div align="center">
  <img src="docs/src/assets/vox_unification_diagram.png" alt="Vox Architecture Unification vs Legacy Fragmentation">
</div>

---

### Pillar 1: The Single Source of Truth
Agents require a single source of truth. A core concept like a `Task` no longer needs to be defined three times across SQL, the backend API, and the client. The `@table` primitive collapses schema and interface into one AST node.

```rust
// [ @table ]
// Auto-generates SQL and gracefully handles schema migrations.
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}

// [ @index ]
// The database index, declared inline next to the type.
@index Task.by_owner on (owner)
```

### Pillar 2: Compile-Time Determinism
Agents ignore edge cases. By eliminating hidden exceptions in favor of a strict `Result[T]` type, Vox makes unhandled errors a compile-time failure, granting immediate syntax-level feedback before broken code executes.

```rust
// [ @query ]
// Read-only endpoint; Vox strictly enforces that it never mutates data.
// Becomes a GET /api/query/recent_tasks endpoint automatically.
@query
fn recent_tasks() to list[Task] {
    ret db.Task
        .where({ done: false })
        .order_by("priority", "desc")
        .limit(10)
}

// [ Result[Task] ]
// Forces every caller to handle both success and error branches.
// The compiler will not build code that ignores an error.
@server fn get_task(id: Id[Task]) to Result[Task] {
    let row = db.Task.find(id)
    match row {
        Some(t) -> Ok(t)               // Task found: return it
        None    -> Error("not found")  // Task missing: return an error
    }
}

// [ @mutation ]
// Auto-transacted write; automatically rolls back on network or logic failure.
@mutation
fn add_task(title: str, owner: str) to Id[Task] {
    ret db.insert(Task, {
        title: title,
        done: false,
        priority: 0,
        owner: owner
    })
}
```

### Pillar 3: Strict Network Boundaries (Web UI)
WebIR restricts interactive state to explicit boundaries (`@island`), protecting the agent's context window. The compiler natively implements the "Islands Architecture"<sup>[6](#ref6)</sup> without exposing React hooks or lifecycle waterfalls inside the `.vox` source file.

```rust
// [ @island ]
// Marks the browser boundary. The compiler generates the React component,
// lifecycle wiring, and typed client stub. None of it appears in the .vox source.
@island TaskList {
    tasks: list[Task]              // Same Task type from Pillar 1
    on_complete: fn(str) -> Unit   // A callback the browser can easily trigger
}

// [ component ]
// Server-rendered execution: fast initial load, written entirely in Vox syntax.
// React's hooks and lifecycles are strictly confined to the generated layer.
component TaskPage() {
    view: (
        <div className="task-list">
            <TaskList
                tasks=[...]
                on_complete={complete_task}
            />
        </div>
    )
}

// [ routes ]
// Safely maps the URL directly to the statically verifiable component.
routes { "/" to TaskPage }
```

> **v0.dev integration:** `vox island generate TaskDashboard "A minimal sidebar dashboard"` calls the v0.dev API (requires `V0_API_KEY`) and writes the generated component into `islands/src/TaskDashboard/`. The `@v0` build hook triggers this automatically during `vox build`.

### Pillar 4: Durable State & Agent Interoperability
Multi-agent pipelines crash, and external tools fail. By integrating durable execution<sup>[7](#ref7)</sup> and the "let it crash" actor model<sup>[8](#ref8)</sup>, a `workflow` guarantees state survival automatically.

The `@mcp.tool` decorator projects these hardened native functions directly to Anthropic's Model Context Protocol (MCP)<sup>[5](#ref5)</sup> for external tool use.<sup>[9](#ref9)</sup>

<table width="100%">
<tr>
<td width="66%" align="center" valign="middle">
  <img src="docs/src/assets/durable_essentialist_loop.webp" width="100%">
</td>
<td width="33%" valign="top">

```rust
// [ activity: Compute Node Execution ]
// Flaky steps on transient workers (Node A/B).
activity charge_card(req: int) to Result[str] {
    // Retries automatically on node death or OOM
    ret Ok("tx_123")
}

// [ workflow: Durable Orchestration ]
// Commits to Arca Vault; survives node crashes.
workflow checkout(req: int) to str {
    let result = charge_card(req)
    match result {
        Ok(tx)   -> "Result: Ok(" + tx + ")"
        Error(e) -> "Fault: " + e
    }
}

// [ @mcp.tool: MCP Interface ]
// Expose the workflow to Anthropic's MCP boundary.
@mcp.tool "Process durable checkout"
fn complete_purchase(req: int) to str {
    checkout(req)
}
```

</td>
</tr>
</table>

### Pillar 5: Solving the Training Paradox
Legacy languages saturate the internet's training data. To catch up, `vox populi` and the MENS pipeline allow you to locally fine-tune foundation models natively on Vox's structural boundaries, bridging the data gap using Rust-accelerated pipelines.

---

More: [`examples/golden/`](examples/golden/) · [Rosetta comparison (C++, Rust, Python)](docs/src/explanation/expl-rosetta-inventory.md)
<!-- ANCHOR_END: how_vox -->

---

## Quick Start

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/vox-foundation/vox/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/vox-foundation/vox/main/scripts/install.ps1 | iex
```

```bash
# Create your first project
vox init my-app
cd my-app
vox build src/main.vox -o dist
vox run src/main.vox
```

```text
vox init [name]          Scaffold a new project (templates: chatbot, dashboard, api)
vox build <file>         Compile → TypeScript + Rust output
vox check <file>         Fast type validation
vox run <file>           Development server (Axum + TanStack dev proxy)
vox dev <file>           Hot-reload dev mode
vox test <file>          Run @test functions
vox fmt <file>           Format source
vox bundle <file>        Full production build → pnpm → single binary
vox doctor               Verify toolchain, environment, and secret health
```

Full command reference: [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

> `vox commands --recommended` gives a curated first-time subcommand map. `vox ci gui-smoke` runs deterministic WebIR routing tests; opt into Vite (`VOX_WEB_VITE_SMOKE=1`) or Playwright (`VOX_GUI_PLAYWRIGHT=1`) lanes.

---

## Agent Orchestration & AI Capabilities

### Multi-agent coordination

`vox-orchestrator` assigns tasks by file affinity and role. `vox-dei` handles human-in-the-loop review — pausing, reassigning, or confirming work before it proceeds. The control surface is exposed as MCP tools, available from the VS Code sidebar or any MCP-compatible agent:

<!-- tool names sourced from crates/vox-orchestrator/src/mcp_tools/tools/dispatch.rs -->
```text
vox_pause_agent      Suspend a running agent and queue its tasks
vox_resume_agent     Resume a paused agent
vox_retire_agent     Retire an agent and release all locks
vox_reorder_task     Change dispatch priority of a queued task
vox_queue_status     Show orchestrator queue and agent states
```

### Agent-to-agent messaging

Agent-to-agent messaging is built into the runtime — no external queue, shared table, or webhook required. Both sides use the same declared Vox type; the compiler catches mismatches before anything runs. Cross-machine relay is available with the `populi-transport` feature.

### The Populi mesh

`vox populi` is a hardware-aware node registry. Each node detects and advertises its hardware — CPU, CUDA, Metal, VRAM — on startup; the orchestrator routes training and inference jobs to the machines that can handle them.

```bash
VOX_MESH_ENABLED=1 VOX_MESH_NODE_ID=my-node vox populi serve
```

### Model selection & provider routing

| Provider | Support | Notes |
|---|---|---|
| Ollama (local) | First-class | No cost, no disclosure |
| Google Gemini | First-class | Privacy acknowledgment required |
| Groq | First-class | Authoritative rate-limit headers |
| OpenRouter | First-class | Local estimate |
| OpenAI / Anthropic | Gated | Pro / Enterprise |
| Together AI | Gated | ML-focused |

```bash
vox populi status --quotas   # view per-provider usage and remaining budget
```

### Local GPU & Native Training (MENS)

MENS is a native Rust training pipeline (Burn + Candle) — no Python, no virtualenv. `vox populi probe` detects CUDA, Metal, and WebGPU and orchestrates multiple parallel pipelines:

1. **QLoRA Fine-Tuning:** Train specialized adapter weights from your team's internal `src/` repositories.
2. **Speech-to-Code (ASR):** Map vocal commands to AST modifications using local Whisper/Qwen models.
3. **Local Mesh Serving:** Deploy models via an OpenAI-compatible `/v1/completions` endpoint for offline agentic orchestration.

```bash
# Profile hardware and begin a QLoRA fine-tune
vox populi train --config qlora.toml

# Expose the fine-tuned adapter over the local mesh network
vox populi serve --model mens/runs/latest/model_final.bin --port 8080
```

---

## Documentation

Docs follow the **Diátaxis** framework (tutorials, how-to guides, explanations, reference):

| Section | Description | Key Links |
|---------|-------------|-----------|
| **Getting Started** | High-level overviews and setup. | [What is Vox?](docs/src/index.md) <br> [Getting Started](docs/src/tutorials/tut-getting-started.md) |
| **Journeys & Tutorials** | Step-by-step guides for full-stack patterns. | [First Full-Stack App](docs/src/how-to/first-full-stack-app.md) <br> [AI Agents & MCP](docs/src/how-to/how-to-ai-agents.md) |
| **How-To Guides** | Goal-oriented recipes for specific problems. | [Model Domain Logic](docs/src/how-to/how-to-custom-types.md) <br> [Native Training](docs/src/journeys/native-training.md) |
| **Explanations** | Theoretical deep-dives and architectural 'Why's. | [Compiler Architecture](docs/src/explanation/expl-architecture.md) <br> [AI Orchestration](docs/src/explanation/expl-ai-orchestration.md) |
| **Reference** | Authoritative lists, CLI maps, and type systems. | [CLI Surface](docs/src/reference/cli.md) <br> [Decorator Registry](docs/src/reference/ref-decorators.md) |
| **Architecture** | SSOT planning and ADRs. | [Master Arch Index](docs/src/architecture/architecture-index.md) <br> [Contributor Hub](docs/src/contributors/contributor-hub.md) |
| **Operations & Quality** | Deployment runbooks, CI constraints, Docker topology. | [Docker Deployment](docs/src/reference/deployment-compose.md) <br> [CI Runner Contract](docs/src/ci/runner-contract.md) |

> **Looking to contribute?** We actively track undocumented surfaces. Check our [Known Documentation Gaps & Backlog](docs/src/api/DOC_GAPS.md) to see where the community needs help.

---

## Architectural Guardrails

These aren't style suggestions — they fail CI. Each one exists because we've seen what happens without it.

### No skeleton code (`vox-toestub`)

`todo!()`, `unimplemented!()`, empty function bodies, and hollow arrow functions in production paths are a build blocker. The `vox-toestub` crate runs a suite of detectors — `StubDetector`, `EmptyBodyDetector`, `HollowFnDetector`, `ReachabilityDetector`, and others — on every CI pass under `vox ci toestub-scoped`.

**Why it matters for AI codebases:** AI agents produce plausible-looking scaffolding. A `todo!()` doesn't signal incompletion to the compiler — it silently defers the failure to runtime. TOESTUB makes that deferral a build error. The `VictoryClaimDetector` goes further, flagging "implementation complete" adjacent to `unimplemented!()`.

```bash
vox stub-check --path crates/my-crate   # run locally before pushing
vox ci toestub-scoped                   # full workspace scan in CI
```

### Complexity bounds (`GodObjectDetector`, `SprawlDetector`)

No struct or impl block may exceed 500 lines or 12 methods. No directory may contain more than 20 files. Both limits are enforced by dedicated detectors in `vox-toestub`.

**Why it matters:** LLM reasoning over a module degrades sharply beyond its coherent processing window. The 500-line limit is calibrated for a 32K-token context; the 20-file directory cap forces domain decomposition before any module becomes ungovernable. The `vox-orchestrator` crate documents this directly in its module comment: *"decomposed from the original god-object."*

### All credentials routed through Clavis (`secret-env-guard`, `operator-env-guard`)

Direct `std::env::var` calls for secrets are a CI failure. All credentials are declared as `SecretId` variants in `crates/vox-clavis/src/lib.rs` and resolved via `vox_clavis::resolve_secret(...)`. `vox ci secret-env-guard` scans changed files for raw environment reads and fails the build on any found outside a strict allowlist.

**Why it matters:** Raw environment variable reads make it impossible to audit what an application is capable of. Every credential that goes through Clavis appears in `vox clavis doctor`, is picked up by `vox ci clavis-parity`, and is visible to every operator. There is no path for an API key to enter the system through a casual `env::var("SOME_KEY")`. The `SecretDetector` in `vox-toestub` catches hardcoded values as a separate failure class.

### Documentation is compiler-verified (`vox-doc-pipeline`, `SchemaComplianceDetector`)

All `.vox` code blocks in `docs/src/` must use `{{#include}}` against a verified file in `examples/golden/`, or be marked `// vox:skip`. Uncompilable loose snippets are a CI failure via `SchemaComplianceDetector`.

**Why it matters:** Documentation that silently diverges from working code is worse than no documentation — it misleads human readers and contaminates the RAG context that agents retrieve from. Every snippet in `examples/golden/` is compiled against the current compiler, including every example in this README.

### Context isolation is centrally managed (`.voxignore` → `vox ci sync-ignore-files`)

`.voxignore` is the single source of truth for files excluded from AI context. Derived files (`.cursorignore`, `.aiignore`, `.aiexclude`) are regenerated automatically. Editing them directly causes a CI drift failure.

**Why it matters:** Generated artifacts, telemetry logs, and build outputs degrade model attention. Without a centrally managed exclusion surface, each tool maintains its own ignore file independently — and agents begin ingesting their own previous outputs as source of truth. `.voxignore` enforces the boundary once, not four times.

### No DRY violations, deprecated symbols, or unwired modules

`vox-toestub` ships additional detectors: `DryViolationDetector` flags copy-pasted logic blocks; `DeprecatedUsageDetector` blocks retired crate names and environment variables (see the retired-symbols table in `AGENTS.md`); `UnwiredModuleDetector` catches modules declared but never imported.

```bash
vox ci toestub-scoped --report    # full findings report with severity breakdown
```



---

<!-- ANCHOR: community_license -->
## Community, Backing & License

### Backing Vox (Open Collective)

Community-backed via **Open Collective** — every dollar raised and spent is public. Sponsorships fund developer grants, CI hardware for MENS neural training, and academic bounties.

[Open Collective →](https://opencollective.com/vox-foundation)

### License

**Apache 2.0** — commercial use permitted, patent rights granted, modifications allowed with attribution.

[`LICENSE`](LICENSE) · [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox)

### Get Involved

Vox Scientia aggregates community research wherever developers are talking. Roadmap decisions and architectural questions are tracked in GitHub Discussions — the format our tooling can index, parse, and feed back into the system.

- **[GitHub Discussions](https://github.com/vox-foundation/vox/issues)**: Architecture questions, language design feedback, and roadmap input.
- **RSS Feed**: [`vox-lang.org/feed.xml`](https://vox-lang.org/feed.xml) — changelogs and architectural decision records.
<!-- ANCHOR_END: community_license -->

---

## References

<a id="ref1"></a>**[1]** Liu, N. F., Lin, K., Hewitt, J., Paranjape, A., Bevilacqua, M., Petroni, F., & Liang, P. (2023). "Lost in the Middle: How Language Models Use Long Contexts." *Transactions of the Association for Computational Linguistics*. arXiv: <https://arxiv.org/abs/2307.03172>

<a id="ref2"></a>**[2]** Copeland, G., & Maier, D. (1984). "Making Smalltalk a Database System." *SIGMOD '84*, 316–325. DOI: <https://doi.org/10.1145/602259.602287>

<a id="ref3"></a>**[3]** Brooks, F. P. (1987). "No Silver Bullet—Essence and Accidents of Software Engineering." *IEEE Computer*, 20(4), 10-19. DOI: <https://doi.org/10.1109/MC.1987.1663532>

<a id="ref4"></a>**[4]** Cunningham, W. (1992). "The WyCash Portfolio Management System." *Addendum to the proceedings of OOPSLA '92*, 29-30. DOI: <https://doi.org/10.1145/157709.157715>

<a id="ref5"></a>**[5]** Anthropic. (2024). *Model Context Protocol*. <https://modelcontextprotocol.io>

<a id="ref6"></a>**[6]** Miller, J. (2020). *Islands Architecture*. JasonFormat. <https://jasonformat.com/islands-architecture/>

<a id="ref7"></a>**[7]** Fateev, M., & Abbas, S. (2019). *Temporal*. Temporal Technologies. <https://temporal.io>

<a id="ref8"></a>**[8]** Armstrong, J. (2003). *Making reliable distributed systems in the presence of software errors* [Ph.D. thesis, Royal Institute of Technology, Stockholm]. <https://erlang.org/download/armstrong_thesis_2003.pdf>

<a id="ref9"></a>**[9]** Unison Computing. *Unison Language: A new approach to distributed programming*. <https://unison-lang.org>
