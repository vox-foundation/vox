<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>One language. Database, backend, UI, and agent tools — designed first as a target for large language models, and for the developers who work alongside them.</strong></p>
  <p><a href="https://vox-lang.org"><strong>vox-lang.org</strong></a></p>

</div>

<p align="center">
  <a href="https://vox-lang.org"><img src="https://img.shields.io/badge/docs-vox--lang.org-blue?style=flat-square" alt="Documentation"/></a>
  <a href="https://github.com/vox-foundation/vox/releases"><img src="https://img.shields.io/github/v/release/vox-foundation/vox?style=flat-square&label=latest" alt="Latest Release"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License"/></a>
  <a href="https://vox-lang.org/feed.xml"><img src="https://img.shields.io/badge/RSS-updates-orange?style=flat-square" alt="RSS Feed"/></a>
</p>

---
<!-- Code examples in this file should mirror examples/golden/*.vox -->
<!-- Run: vox check examples/golden/*.vox to verify -->
> **v0.4 — April 2026.** Below is the current status of each major system area.
>
> | Area | Status |
> |------|--------|
> | Compiler — Vox source to TypeScript + Rust output | ✅ Production |
> | `@table` / `@server` / `@query` / `@mutation` decorators | ✅ Production |
> | `@island` UI (React + TanStack, client-side hydration) | ✅ Available |
> | `workflow` / `actor` / `activity` keywords | ✅ Available |
> | `@mcp.tool` — expose Vox functions to AI agents | ✅ Production |
> | LSP server, VS Code extension, Oratio speech-to-code | ✅ Available |
> | `vox populi` — local GPU inference and multi-provider routing | ✅ Available |
> | `@v0` — v0.dev component generation (requires `V0_API_KEY`) | ✅ Available |
> | Agent orchestration with MCP control surface | ✅ Available |
> | Agent-to-agent messaging (in-process) | ✅ Available |
> | Agent-to-agent messaging (HTTP relay, `populi-transport`) | ✅ Available (opt-in) |
> | `vox bundle` — single-binary production build | ✅ Available |
> | Distributed agent mesh (cross-machine task relay) | 🔄 In progress |
> | Fine-tuning pipeline (GRPO / MENS) | 🔄 In progress |
> | Grammar-constrained generation | 🔄 In progress |
> | Full server-side rendering (TanStack Start, Wave 5+) | 🔄 In progress |
> | Actor-internal state API | 🔄 In progress (syntax stabilizing) |

<br>
<p><em>"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence!"</em>
    <br>
    — Nathaniel Hawthorne, <em>The House of the Seven Gables</em> (1851)</p>

## Why Vox Exists

Software systems are becoming collaborative — not just between developers, but between developers and AI agents working side by side on the same codebase. Vox is designed for that world: a single compiled language covering database schema, server logic, UI, and agent tools, where both humans and models get immediate, structural feedback from the compiler rather than discovering problems at runtime.

## Design Principles

- **Compiler as error boundary.** No `null`, no `undefined`. Absence is `Option[T]`; failure is `Result[T]`. Every branch must be handled — code that skips error handling does not build.
- **One source of truth.** `@table`, `@server`, and `@island` declared together; the compiler generates the SQL schema, HTTP endpoint, and TypeScript client from one declaration.
- **Durability without a framework.** `workflow` and `actor` are language keywords, not libraries.
- **AI-native by design.** `@mcp.tool` exposes any function as a callable tool for Claude, Cursor, or any MCP-compatible agent. Orchestration, agent mesh, and model routing are built into the platform.

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

Modern web apps split into two concerns: the **server**, which renders initial HTML and handles data, and the **browser**, which handles interactivity. In Vox, both live in the same file and share the same types:

```vox
// An island is a piece of the page that's interactive in the browser
@island TaskList {
    tasks: list[Task]              // same Task type from Step 1 — no duplication
    on_complete: fn(str) -> Unit   // a callback the browser can call
}

// A component is server-rendered — fast initial load, no JavaScript needed
component TaskPage() {
    view: <div className="task-list">
        <TaskList tasks=[...] on_complete={complete_task} />
    </div>
}

routes { "/" to TaskPage }
```

`@island` marks the parts of your UI that need to be interactive in the browser — the list that responds to clicks, not the static header. Everything else is `component`: rendered on the server, fast to load, and simple. The compiler generates the browser-side JavaScript for the island, the router configuration for `routes`, and the typed function that lets the island call the server's `complete_task` — you write none of that glue yourself.

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

Run `vox commands --recommended` for a curated first-time map of subcommands. For repository hygiene, `vox ci gui-smoke` runs deterministic WebIR lowering tests and can opt into Vite (`VOX_WEB_VITE_SMOKE=1`) or Playwright (`VOX_GUI_PLAYWRIGHT=1`) lanes documented in the same CLI reference.

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

| Intent | Start here |
|--------|-----------|
| **Guided walkthrough** | [Getting Started](docs/src/tutorials/tut-getting-started.md) |
| **Language overview** | [What is Vox?](docs/src/index.md) |
| **FAQ** | [Language, runtime, MCP](docs/src/explanation/faq.md) |
| **CLI reference** | [Command surface](docs/src/reference/cli.md) |
| **Type system** | [Type system reference](docs/src/reference/ref-type-system.md) |
| **Decorators** | [Full decorator registry](docs/src/reference/ref-decorators.md) |
| **VS Code & voice** | [LSP, MCP sidebar, Oratio speech](vox-vscode/README.md) |
| **Contributing** | [Contributor hub](docs/src/contributors/contributor-hub.md) |
| **ADRs** | [Architecture decisions](docs/src/adr/index.md) |
| **Syntax guide** | [Examples & style](examples/STYLE.md) |

---

## Community

- **RSS Feed**: [`vox-lang.org/feed.xml`](https://vox-lang.org/feed.xml) — changelog and doc updates
- **GitHub Discussions**: Architecture questions, language design feedback, and roadmap input

## License

Apache-2.0. Full text: [`LICENSE`](LICENSE). Source: [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox).
