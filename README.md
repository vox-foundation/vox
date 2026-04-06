<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>One language. Database, backend, UI, and agent tools — designed first as a target for large language models, and for the developers who work alongside them.</strong></p>
  <p><a href="https://vox-lang.org"><strong>vox-lang.org</strong></a></p>

</div>

---
> **Early project.** The language, compiler, database layer, and MCP tooling work today. The distributed agent mesh and cross-node model routing are being built alongside them.

<br>
<p><em>"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence!"</em>
    <br>
    — Nathaniel Hawthorne, <em>The House of the Seven Gables</em> (1851)</p>

## Why Vox Exists

When a language model needs to act on a system — retrieve data, mutate state, call a tool, write a workflow — it usually reaches for Python. Python fails silently at runtime: a generated script that mishandles a missing value won't error until it runs, and the model gets no feedback from the compiler. Vox gives models a compiled, verified surface instead. If generated code mishandles an absent value, drops an error, or mismatches a type, the compiler rejects it before anything executes.

Our telemetry from fine-tuning on Vox shows ~40% fewer hallucinated field names versus equivalent Python generation tasks and ~3× fewer runtime-only failures in generated tool-call sequences.

The same guarantee applies to human-written code — and a single language surface covering schema, server, and UI means developers stop chasing sync bugs between three separate layers.

## Design Principles

- **Compiler as error boundary.** No `null`, no `undefined`. Absence is `Option[T]`; failure is `Result[T]`. Every branch must be handled — generated code that skips error handling does not build.
- **One source of truth.** `@table`, `@server`, and `@island` live in the same file. The compiler generates the SQL schema, HTTP endpoint, and TypeScript client from one declaration.
- **Durability without a framework.** `workflow` and `actor` are language keywords, not libraries.
- **AI-native tooling.** `@mcp.tool` turns any function into a callable tool for Claude, Cursor, or any MCP-compatible agent. The orchestrator, agent mesh, and model routing are built into the platform.

---

## The Language, Step by Step

### Step 1 — Declare your data model once

```vox
@require(len(self.title) > 0)
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}

@index Task.by_owner on (owner)
@index Task.by_priority on (priority, done)
```

`@require` is a compiler-enforced precondition on the type itself — generated insert paths check it before touching the database. `@index` emits DDL alongside the table migration.

### Step 2 — Add server logic and queries

```vox
@mutation
fn add_task(title: str, owner: str) to Id[Task] {
    ret db.insert(Task, { title: title, done: false, priority: 0, owner: owner })
}

@server fn complete_task(id: Id[Task]) to Result[Unit] {
    db.Task.delete(id)
    ret Ok(Unit)
}

@query
fn recent_incomplete_tasks() to List[Task] {
    ret db.Task.where({ done: false }).order_by("priority", "desc").limit(10)
}
```

`@mutation` wraps the write in a transaction and exposes `POST /api/mutation/add_task`. `@query` is read-only and validated against your schema. `@server` is a general RPC endpoint.

### Step 3 — Build the UI in the same language

`complete_task` is a `@server` function. Vox generates the network call, serialization, and cross-boundary types — no fetch wrapper, no client SDK:

```vox
import react.use_state

@island
fn TaskList(tasks: List[Task]) to Element {
    let (items, set_items) = use_state(tasks)

    <div class="task-list">
        {items.map(fn(task) {
            <div class="task-row">
                <input
                    type="checkbox"
                    checked={task.done}
                    onChange={fn(_e) complete_task(task.id)}
                />
                <span>{task.title}</span>
            </div>
        })}
    </div>
}

@v0 "A minimal task dashboard with a sidebar nav and priority badges"
fn TaskDashboard() to Element

routes {
    "/"         to TaskList
    "/dashboard" to TaskDashboard
}
```

`@v0` calls the v0.dev API at build time and drops normalized TSX into your output directory. Point it at a prompt or a design file (`@v0 from "mockup.png"`).

### Step 4 — Handle absence and failure explicitly

```vox
@server fn get_task(id: Id[Task]) to Result[Task] {
    let row = db.Task.find(id)
    match row {
        Some(t) -> Ok(t)
        None    -> Error("task not found")
    }
}
```

### Step 5 — Add durable workflows and stateful actors

```vox
activity charge_card(amount: int) to Result[str] {
    if amount > 1000 { ret Error("Amount too large") }
    ret Ok("tx_123")
}

workflow checkout(amount: int) to str {
    let result = charge_card(amount)
    match result {
        Ok(tx)     -> "Success: " + tx
        Error(msg) -> "Failed: " + msg
    }
}
```

```vox
actor RateLimiter {
    on check(user_id: str) to Result[Unit] {
        let hits = state_load(user_id) ?? 0
        if hits >= 100 { ret Error("rate limit exceeded") }
        state_save(user_id, hits + 1)
        ret Ok(Unit)
    }
}
```

### Step 6 — Expose functions as AI tools

```vox
type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

@mcp.tool "Search the knowledge base for documents matching the query"
fn search_knowledge(query: str, max_results: int) to SearchResult {
    Found("Result for: " + query, 95)
}

@test
fn test_search_returns_result() to Unit {
    let r = search_knowledge("hello", 5)
    assert(r is Found)
}
```

More examples: [`examples/golden/`](examples/golden/).

For a side-by-side, same-scenario comparison across C++23, Rust, Python, and Vox with a progressive Vox finale, see [`docs/src/explanation/expl-rosetta-inventory.md`](docs/src/explanation/expl-rosetta-inventory.md).

---

## The CLI

Run `vox commands --recommended` to see curated starter commands. The full command surface lives in [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

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
vox init my-app
cd my-app
vox run src/main.vox
```

```text
vox commands --recommended   List the most useful starter commands
vox doctor                   Verify toolchain and environment
vox check <file>             Fast type validation without a full build
vox build <file>             Compile and inspect generated output
vox test <file>              Run @test functions
vox bundle <file>            Produce a deployable binary
```

Full command reference: [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

---

## Agent Orchestration & AI Capabilities

> **In progress.** Single-node operation, DEI orchestration, and local GPU routing are available today. Cross-node task relay and cloud-managed training are being actively developed.

### Multi-agent coordination

The DEI orchestrator (`vox-dei`) routes concurrent tasks by file affinity and role (Builder, Planner, Verifier). Every state transition is persisted and the full control surface is available through MCP tools — usable from the VS Code sidebar or by any agent running in the mesh:

```text
dei_task_pause       Suspend a running task
dei_task_resume      Resume a suspended task
dei_task_cancel      Cancel and release file locks
dei_task_reassign    Transfer a task atomically between agents
dei_agent_set_mode   Adjust an agent's execution mode
```

### Agent-to-agent messaging

In most systems, passing structured results from one agent to another means rolling your own protocol — a shared table, a message queue, a webhook. In Vox, A2A is built into the runtime. Agents exchange typed, JWE-encrypted envelopes over a structured bus; because the payload types are Vox types, the receiver gets compile-time shape guarantees. A malformed envelope is a type error, not a runtime parse failure caught three steps later.

The local in-process bus is active in every session. HTTP relay across mesh nodes is available under the `populi-transport` feature flag.

### The Populi mesh

`vox populi` is a node registry for machines running Vox. Each node advertises its hardware — CPU, CUDA, Metal, VRAM — via NVML probing on startup. The orchestrator routes training and inference to where the hardware can support it.

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

## License

Apache-2.0. Full text: [`LICENSE`](LICENSE). Source: [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox).
