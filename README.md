<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>One language. Database, backend, UI, and scientific publication — designed first as a target for large language models to orchestrate systems and autonomously hunt for discovery alongside developers.</strong></p>
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

## Why Vox Exists

The way we build software has changed forever. Today, developers direct Large Language Models to architect, write, and orchestrate logic. Yet, the foundations we rely on—Python, TypeScript, Rust—were designed decades before AI could write code. Their vast, unconstrained surfaces often cause agents to guess, hallucinate, and fail mid-execution.

The core intent behind Vox is to return power to developers by providing an environment where an AI never has to guess. It is a language built from the ground up as a deterministic target for language models. 

By constraining the boundaries of software engineering on behalf of the AI, Vox turns arbitrary text generation into a predictable, self-correcting loop. It is not merely a web framework; it is an orchestration engine designed to help humans and models jointly construct distributed systems, synthesize unstructured data, and power autonomous research pipelines.



### Platform Architecture & Stability

We stratify the platform's development based on a single metric: **Model Predictability**. The systems responsible for data integrity, core language syntax, and agent memory represent the unchanging foundation because they anchor an LLM's understanding of the codebase. Conversely, high-level orchestration features and rendering lifecycles remain cautiously fluid as we iterate on the most effective ways for AI to construct them. 

Rather than maintaining a brittle list of micro-features, we enforce capability stability directly through our continuous integration and compiler test boundaries.

* 🟢 **Tier 1 (Stable):** Production-ready. The AST syntax and runtime behavior are locked to ensure LLM code generation is completely predictable.
* 🟡 **Tier 2 (Preview):** Functionally available, but execution lifecycles or internal AST schemas may evolve to improve model generation rates.
* 🚧 **Tier 3 (Experimental):** Under active architectural planning or limited tightly behind CLI feature flags.

| Domain & Key Capabilities | Stability | How we verify this |
|:--------------------------|:----------|:-------------------|
| **Compiler & Toolchain**<br>Compiling to Rust and TypeScript, auto-formatting, Language Server (LSP) | 🟢 **Stable** | Golden parsing suite, `vox fmt --check`, typed syntax validations |
| **Data & Interfaces**<br>`@table` schema migrations, `@query`/`@server` endpoints | 🟢 **Stable** | In-memory DB roundtrips, strict HTTP payload schemas |
| **Agent Tooling**<br>`@mcp.tool` exposure, orchestrated agents, telemetry | 🟢 **Stable** | Model Context Protocol (MCP) compliance, telemetry gates |
| **Web UI & Routing**<br>`@island` browser wiring, V0 components, file-free `routes {}` | 🟡 **Preview** | Web Intermediate Representation (WebIR) syntax checks |
| **Durable Execution**<br>`workflow` state persistence, `actor` message bounds | 🟡 **Preview** | State persistence audits, unfinished code (TOESTUB) rejection |
| **Models & Local AI**<br>`vox populi` GPU inference, speech recognition, native Rust tuning | 🟡 **Preview** | Burn ML backend evaluations, local hardware probe checks |
| **Autonomous Research**<br>`vox scientia` publication pipeline, Socrates hallucination guard | 🟡 **Preview** | Citation overlap tests, novelty discovery CLI checks |
| **Server-Side Rendering**<br>Native integration with TanStack Start architecture | 🚧 **Experimental** | Active migration flags |
| **Distributed Mesh**<br>Cross-machine task relaying and agent pool distribution | 🚧 **Experimental** | Under active architectural planning |

> *Current footprint as of **v0.4 — April 2026**.*

---

## Architectural Guardrails

Vox enforces its design through machine-verifiable limits. To ensure that autonomous agents and human developers can predictably navigate and build upon the codebase, the framework implements five absolute architectural boundaries that trigger CI failure if breached:

**Complexity Bounds (The Sprawl Limit)**
To ensure that logic fits securely within an AI's processing window without diluting reasoning, code sprawl—measured internally as Kolmogorov Complexity (K-Complexity)—is hard-limited. A struct or class cannot exceed 500 lines or 12 methods, and a directory cannot contain more than 20 files. Exceeding these limits forces a build failure, requiring the developer (or agent) to refactor logic cleanly into new sub-domains.

**Zero-Skeleton Enforcement (TOESTUB)**
Code cannot be built if it contains empty execution paths or temporary placeholders. The `toestub` CI pipeline automatically blocks identifiers like `stub/todo` or `stub/unimplemented` in production paths, ensuring that AI-generated logic is fully complete before the system permits evaluation.

**Scientific Documentation Hygiene (Scientia)**
An AI is only as capable as the truth it trains on. Vox treats documentation not as casual text, but as formal, machine-verified research. Under the `Scientia` doctrine, code blocks cannot be written loosely; the framework physically requires all `.vox` snippets to be compiler-verified via direct inclusion (`{{#include}}`). Language models and developers can mathematically trust that the publications they read are strictly factual.

**Strict Credential Resolution (Clavis)**
Hidden environment variables cause deployment drift and model hallucinations. Our security pipeline (`secret-env-guard`) blocks any logic that attempts to directly read generic environment variables. All credentials must route through the central Clavis registry, generating complete, auditable clarity on what capabilities an application possesses.

**Context Window Isolation**
Because the language targets large language models, maintaining context hygiene is treated as a structural mandate. A single source of truth (`.voxignore`) automatically derives all agent exclusion boundaries, shielding models from reading generated artifacts or telemetry logs and keeping their attention solely on intentional logic.

---

## How Vox Solves the Training Paradox

It is often assumed that legacy languages hold a permanent advantage in AI generation simply because language models blindly vacuum up massive quantities of their scripts scraped from across the internet. 

Vox bypasses this shotgun approach entirely. The repository includes local training primitives (`vox populi` and the MENS neural training pipeline) that allow developers to natively fine-tune *any* foundation model to master Vox's strict structural boundaries. Because the platform ships with an inference and training mesh that scales seamlessly across a variety of hardware architectures, you are never locked out of AI-assisted engineering just because a model hasn't scraped enough of your syntax. You fine-tune it natively, empowering specialized models to write flawless code tailored exactly to your orchestration needs.

---

## How Vox Works

Code generation fails when an AI is forced to navigate fragmented files, hidden states, and chaotic lifecycles. Vox solves this by functioning as a high-level abstraction that rigorously lowers into safe, deterministic infrastructure.

* **The High-Level Intermediate Representation (HIR):** When an AI writes a `.vox` file, the parser doesn't just strip it into an Abstract Syntax Tree (AST); it lowers it into a strictly unified HIR. This means database bindings, HTTP handshakes, and UI lifecycles are resolved mathematically by the compiler before any code is generated. The AI doesn't write React or SQL; it writes HIR directives, and the compiler handles the mechanical execution logic.
* **Deterministic Rendering (WebIR):** UI is compiled directly to a Web Intermediate Representation (WebIR). Agents never have to juggle React hooks, client state waterfalls, or asynchronous DOM wiring. They emit pure data representations, and the WebIR projection handles the translation down to HTML.
* **Semantic Error Feedback:** There are no implicit runtime exceptions. Complex operations return strict `Result[T]` constraints. Because semantic checking happens exclusively along the unified HIR path, if an agent fails to handle an error state, the compiler catches the omission immediately, providing the LLM with rigorous, syntax-level feedback to self-correct during the generation loop.
* **Native Protocol Projection:** AI capabilities are not a bolted-on SDK. Because the language is AI-native, the AST inherently recognizes decorators like `@mcp.tool`. The compiler automatically projects these into Model Context Protocol manifests, allowing any external agent to execute your logic without you writing a single HTTP route.

---

## The Language

Here's a complete Vox program — a task tracker with a database table, a server endpoint, and a page:

```vox
@table type Task {      // defines database schema
    title: str
    done:  bool
}

@server fn complete_task(id: Id[Task]) to Result[Unit] {
    db.Task.delete(id)
    ret Ok(Unit)        // signals success; the caller must handle failure too
}

@island TaskList {      // a live, interactive component in the browser
    tasks: list[Task]
}

component TaskPage() { // the static page that hosts it
    view: <div><TaskList tasks=[...] /></div>
}

routes { "/" to TaskPage }
```

One file. The compiler generates the SQL schema, the server endpoint, and the browser-side code that connects them. There's no separate ORM configuration, no hand-written API route, no TypeScript interface to keep in sync. The steps below build this up from first principles.

### Step 1 — Declare your data

In most projects, a data type lives in three places at once: a database schema (SQL), a server model (TypeScript or Python class), and a client type. They drift apart silently. Vox collapses all three into one declaration:

```vox
@require(len(self.title) > 0)    // the compiler rejects empty titles on insert
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}

@index Task.by_owner on (owner)  // the database index, declared next to the type
```

`@table` tells the compiler this type lives in the database. It generates the SQL table and handles schema migrations — the process of updating a live database when you change the shape of your data — automatically. `@require` is a rule the compiler bakes into every write path: it's not just a runtime check, it can't be bypassed. `@index` creates a database index for fast lookups by owner, declared right next to the type it belongs to.

### Step 2 — Write server functions

A web application needs ways to read data, write data, and do custom logic. In Vox, you declare the intent with a decorator rather than wiring up a router by hand:

```vox
@query
fn recent_tasks() to list[Task] {
    // read-only; becomes a GET /api/query/recent_tasks endpoint automatically
    ret db.Task.where({ done: false }).order_by("priority", "desc").limit(10)
}

@server fn get_task(id: Id[Task]) to Result[Task] {
    let row = db.Task.find(id)
    match row {
        Some(t) -> Ok(t)           // task found: return it
        None    -> Error("not found")  // task missing: return an error
    }
}

@mutation
fn add_task(title: str, owner: str) to Id[Task] {
    // writes are wrapped in a transaction automatically
    ret db.insert(Task, { title: title, done: false, priority: 0, owner: owner })
}
```

`@query` exposes a read-only endpoint — the kind that should never change data, so Vox enforces that. `@mutation` wraps the write in a database transaction, meaning if something goes wrong partway through, the whole operation is rolled back cleanly. `@server` is for everything else — general-purpose logic with a `POST` endpoint.

The return type `Result[Task]` is where Vox earns its keep: it forces every piece of code that calls `get_task` to handle *both* outcomes — the found case and the not-found case. There's no way to call this function and silently ignore the error. The compiler won't build code that does.

### Step 3 — Build the UI

Modern web apps split into two concerns: the **server**, which renders initial HTML and handles data, and the **browser**, which handles interactivity. The key design challenge is keeping those boundaries clean without leaking React hook syntax, JSX idioms, or CSS class names into your core application logic.

Vox solves this with two distinct primitives:

```vox
// An island is a piece of the page that's interactive in the browser.
// React lives inside the generated artifact — not in your .vox source.
@island TaskList {
    tasks: list[Task]              // same Task type from Step 1 — no duplication
    on_complete: fn(str) -> Unit   // a callback the browser can call
}

// A component is server-rendered — fast initial load, no JavaScript needed.
// It composes islands using the same Vox syntax, with no JSX boilerplate.
component TaskPage() {
    view: <div className="task-list">
        <TaskList tasks=[...] on_complete={complete_task} />
    </div>
}

routes { "/" to TaskPage }
```

`@island` marks the boundary where the browser takes over. The compiler generates the React component, the browser lifecycle wiring, and the typed client stub — none of that appears in your `.vox` source. `component` stays on the server: rendered to HTML, fast to load, and written entirely in Vox syntax. The `routes { }` block maps URLs to pages without touching a router configuration file.

The distinction isn't just organizational. It means React's mental model — hooks, lifecycle, client state — is confined to the generated layer. You write Vox; the compiler decides what runs where.

> **v0.dev integration:** `vox island generate TaskDashboard "A minimal sidebar dashboard"` calls the v0.dev API (requires `V0_API_KEY`) and writes the generated component into `islands/src/TaskDashboard/`. The `@v0` build hook triggers this automatically during `vox build`.

### Step 4 — Durable logic and AI tools

Real applications need things that survive a server restart, can be retried on failure, and can be called by people *and* AI agents. Vox builds all three in at the language level:

```vox
// An activity is a step that can be retried independently if it fails
activity charge_card(amount: int) to Result[str] {
    if amount > 1000 { ret Error("Amount too large") }
    ret Ok("tx_123")
}

// A workflow orchestrates activities and survives crashes — its state is durable
workflow checkout(amount: int) to str {
    let result = charge_card(amount)
    match result {
        Ok(tx)     -> "Success: " + tx
        Error(msg) -> "Failed: " + msg
    }
}

// One decorator makes this function callable by Claude, Cursor, or any AI agent
@mcp.tool "Search the knowledge base"
fn search_knowledge(query: str) to str {
    "Result for: " + query
}

// Tests live in the same file, run with `vox test`
@test
fn test_search() to Unit {
    assert(search_knowledge("hello") is str)
}
```

`workflow` and `actor` are keywords in the language, not third-party frameworks you install. A workflow tracks its own progress — if the server restarts halfway through `checkout`, it picks up where it left off. An `actor` is a named entity that receives typed messages and can hold its own state across many calls.

`@mcp.tool` is one line that connects your function to the [Model Context Protocol](https://modelcontextprotocol.io) — the standard that lets AI assistants call tools in editors and agent pipelines. With one decorator, `search_knowledge` becomes something Claude or Cursor can invoke directly.

More examples: [`examples/golden/`](examples/golden/).

For a side-by-side comparison with C++, Rust, and Python solving the same problem, see [`docs/src/explanation/expl-rosetta-inventory.md`](docs/src/explanation/expl-rosetta-inventory.md).

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
vox bundle <file>        Full production build: codegen → pnpm build → single binary
vox doctor               Verify toolchain, environment, and secret health
```

Full command reference: [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

## The CLI

Run `vox commands --recommended` for a curated first-time map of subcommands. For repository hygiene, `vox ci gui-smoke` runs deterministic Web Intermediate Representation (WebIR) routing tests and can opt into Vite (`VOX_WEB_VITE_SMOKE=1`) or Playwright (`VOX_GUI_PLAYWRIGHT=1`) lanes documented in the same CLI reference.

---

## Agent Orchestration & AI Capabilities

### Multi-agent coordination

The **orchestrator** (`vox-orchestrator`) assigns tasks to agents by file affinity and role. **`vox-dei`** handles human-in-the-loop review — pausing, reassigning, or confirming work before it proceeds. The control surface is available as MCP tools, usable from the VS Code sidebar or any MCP-compatible agent:

<!-- tool names sourced from crates/vox-mcp/src/tools/dispatch.rs -->
```text
vox_pause_agent      Suspend a running agent and queue its tasks
vox_resume_agent     Resume a paused agent
vox_retire_agent     Retire an agent and release all locks
vox_reorder_task     Change dispatch priority of a queued task
vox_queue_status     Show orchestrator queue and agent states
```

### Agent-to-agent messaging

In most systems, passing results between agents means building your own protocol — a shared table, a queue, a webhook. In Vox, agent-to-agent messaging is built into the runtime. Agents exchange typed, encrypted messages; because both sides use the same declared Vox type, the compiler catches mismatches in each codebase before anything runs.

The in-process message bus is active in every session. Cross-machine relay is available with the `populi-transport` feature.

### The Populi mesh

`vox populi` is a node registry for machines running Vox. Each node detects and advertises its hardware — CPU, CUDA, Metal, VRAM — on startup. The orchestrator routes training and inference jobs to the machines that can handle them.

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

### Local GPU & native training

`vox populi probe` detects your GPU and recommends a QLoRA configuration. Training runs natively in Rust via [Burn](https://github.com/tracel-ai/burn) — no Python, no `pip install`, no virtual environment:

```bash
vox populi train --config qlora.toml
vox populi serve --model mens/runs/latest/model_final.bin --port 8080
```

~4,000 tokens/second on an RTX 4080 SUPER. Served on an OpenAI-compatible `/v1/completions` endpoint.

---

## Documentation

Vox documentation is structured around the **Diátaxis** framework, explicitly separating tutorials, how-to guides, explanations, and pure reference material. 

| Section | Description | Key Links |
|---------|-------------|-----------|
| **Getting Started** | High-level overviews and introductory setup. | [What is Vox?](docs/src/index.md) <br> [Getting Started](docs/src/tutorials/tut-getting-started.md) |
| **Journeys & Tutorials** | Step-by-step guides for full-stack patterns. | [First Full-Stack App](docs/src/how-to/first-full-stack-app.md) <br> [AI Agents & MCP](docs/src/how-to/how-to-ai-agents.md) |
| **How-To Guides** | Goal-oriented recipes for specific problems. | [Model Domain Logic](docs/src/how-to/how-to-custom-types.md) <br> [Native Training](docs/src/journeys/native-training.md) |
| **Explanations** | Theoretical deep-dives and architectural 'Why's. | [Compiler Architecture](docs/src/explanation/expl-architecture.md) <br> [AI Orchestration](docs/src/explanation/expl-ai-orchestration.md) |
| **Reference** | Authoritative lists, CLI maps, and type systems. | [CLI Surface](docs/src/reference/cli.md) <br> [Decorator Registry](docs/src/reference/ref-decorators.md) |
| **Architecture** | Single-Source-of-Truth (SSOT) planning and ADRs. | [Master Arch Index](docs/src/architecture/architecture-index.md) <br> [Contributor Hub](docs/src/contributors/contributor-hub.md) |

---

## Community, Backing & License

Vox is built in the open and designed for immense commercial and academic scale.

### Backing Vox (Open Collective)
The Vox Foundation operates as a transparent, community-backed entity through **Open Collective**. We believe that foundational language development shouldn't be captured by a single corporate interest.
- **Sponsorship & Grants:** By contributing via our Open Collective, you directly fund developer grants, heavy CI/CD hardware (crucial for our MENS neural training networks), and academic bounties.
- **Complete Transparency:** Every dollar raised and spent is public. You can see exactly how the community's resources are allocated to further the language.

### The Apache 2.0 License
Vox is licensed under the **Apache License, Version 2.0**. For software developers, this means:
- **Commercial Use:** You can use Vox to build commercial, closed-source, or proprietary applications without being forced to open-source your own code (unlike the GPL).
- **Patent Protection:** The license grants you explicit patent rights from contributors. If a contributor holds a patent on the code they submitted, you are legally protected to use it.
- **Modification & Distribution:** You are free to modify the compiler, the runtime, or the standard library, as long as you include the original copyright notices.

Read the full text here: [`LICENSE`](LICENSE). Source: [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox).

### Get Involved
- **GitHub Discussions**: The primary hub for architecture questions, language design feedback, and roadmap input.
- **X (Twitter)**: Follow **[@vox_foundation](https://x.com/vox_foundation)** for milestone updates, release notes, and research highlights.
- **Discord**: Join the **[Vox Developer Discord](https://discord.gg/vox-lang)** to collaborate with engineers, share LLM fine-tunes, and get real-time compiler help.
- **RSS Feed**: [`vox-lang.org/feed.xml`](https://vox-lang.org/feed.xml) — changelogs and architectural decision records.
