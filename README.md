<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>One language. Database, backend, UI, and agent tools — built for developers and the models that generate code for them.</strong></p>
  <p><a href="https://vox-lang.org"><strong>vox-lang.org</strong></a></p>

  <br>

  <p>
    <em>"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence!"</em>
    <br>
    — Nathaniel Hawthorne, <em>The House of the Seven Gables</em> (1851)
  </p>

</div>

---

> **Early project.** Vox is under active development. The language, compiler, database layer, and MCP tooling work today; the distributed agent mesh and cross-node model routing are being built in the open alongside them. Expect fast iteration and some rough edges.

Every full-stack project eventually drowns in sync problems — a SQL schema, a backend struct, a TypeScript interface, and a REST layer that are all supposed to say the same thing but gradually don't. Vox removes the problem by making them one thing. You write your data model, server logic, and UI in the same language; the compiler generates the wiring. The output is standard **Rust** on the server and **TypeScript/React** in the browser — nothing proprietary.

Vox is also designed as a first-class execution target for large language models. When a model needs to run code — retrieve data, call a tool, mutate state — it usually reaches for Python, a language with no type safety and silent failure modes. Vox gives it a compiled, verified surface instead. If the generated code mishandles an absent value or drops an error, the compiler rejects it before anything executes. One coherent language surface means less context for a model to hold, fewer APIs to hallucinate, and a shorter path from generated code to correct behavior.

## The Language in Five Steps

### 1. Schema, server, and query — one declaration

Normally a single entity means a SQL migration, a Rust struct, and a TypeScript interface, kept in sync manually. In Vox, you declare it once:

```vox
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}

@server fn add_task(title: str, owner: str) to Id[Task] {
    ret db.insert(Task, { title: title, done: false, priority: 0, owner: owner })
}

@query
fn recent_incomplete_tasks() to List[Task] {
    ret db.Task.where({ done: false }).order_by("priority", "desc").limit(10)
}
```

`@table` creates the database table. `@server` generates the HTTP endpoint and a matching TypeScript client function. `@query` is validated against your schema at compile time. You write the business logic; the compiler owns the surface.

### 2. Frontend components call backend functions directly

No fetch wrappers. No client SDK to maintain. `complete_task` below is a `@server` function — the compiler generates the network call, serialization, and type safety across the boundary automatically:

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

routes {
    "/" to TaskList
}
```

### 3. A type system that makes silence impossible

Vox has no `null` and no `undefined`. Absence is `Option[T]`; failure is `Result[T]`. The compiler forces you to handle both branches — missing a case is a compile error, not a surprise at 2am:

```vox
@server fn get_task(id: Id[Task]) to Result[Task] {
    let row = db.Task.find(id)
    match row {
        Some(t) -> Ok(t)
        None    -> Error("task not found")
    }
}
```

This pattern is consistent everywhere — database queries, network calls, workflow steps, AI tool calls. For generated code specifically, it's load-bearing: an LLM writing Vox cannot produce a program that silently passes a missing value or swallows an error. The compiler enforces exhaustiveness. The generated code either handles every branch or it doesn't build.

### 4. Workflows that survive crashes, actors that hold state

`workflow` and `activity` are language keywords. If the server goes down mid-payment, the runtime replays completed steps on restart — no library required:

```vox
activity charge_card(amount: int) to Result[str] {
    if amount > 1000 { ret Error("Amount too large") }
    ret Ok("tx_123")
}

workflow checkout(amount: int) to str {
    let result = charge_card(amount)
    match result {
        Ok(tx)  -> "Success: " + tx
        Error(msg) -> "Failed: " + msg
    }
}
```

Actors persist state across restarts — a clean primitive for rate limiters, session state, or background counters:

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

### 5. A language models can write, call, and verify

Vox works in both directions with AI. Your functions become callable by any MCP-compatible agent — Claude, Cursor, VS Code Copilot — with one decorator. No OpenAPI spec, no tool registration, no schema drift:

```vox
type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

@mcp.tool "Search the knowledge base for documents matching the query"
fn search_knowledge(query: str, max_results: int) to SearchResult {
    Found("Result for: " + query, 95)
}
```

And when a model needs to *generate* code to act on your system — not just call a predefined tool but actually write logic — Vox is the right target. The type system constrains the output space. A model that writes Vox gets compiler feedback instead of runtime exceptions. `@test` closes the loop:

```vox
@test
fn test_search_returns_result() to Unit {
    let r = search_knowledge("hello", 5)
    assert(r is Found)
}
```

More examples: [`examples/golden/`](examples/golden/).

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

If you already cloned the repo, run `./scripts/install.sh` or `.\scripts\install.ps1` directly.

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

Vox ships a distributed execution layer that connects multiple Vox instances — local or remote — into a coordinated task graph.

### Multi-agent coordination

The DEI orchestrator (`vox-dei`) runs multiple agents concurrently against a shared codebase, routing tasks by file affinity and role (Builder, Planner, Verifier). The full control surface is exposed through MCP tools, so the VS Code sidebar and other agents share the same interface:

```text
dei_task_pause       Suspend a running task
dei_task_resume      Resume a suspended task
dei_task_cancel      Cancel and release file locks
dei_task_reassign    Transfer a task atomically between agents
dei_agent_set_mode   Adjust an agent's execution mode
```

### The Populi mesh

`vox populi` is a node registry for machines running Vox. Each node advertises its hardware — CPU, CUDA, Metal, VRAM — via NVML probing on startup. The orchestrator uses those hints to route training and inference where the hardware can support it.

```bash
VOX_MESH_ENABLED=1 VOX_MESH_NODE_ID=my-node vox populi serve
```

> **In progress.** Single-node operation and local GPU routing work today. Cross-node task relay is implemented behind the `populi-transport` feature flag; the end-to-end scheduling path is being hardened.

### Model selection & provider routing

A policy engine manages multi-provider inference with automatic retry, rate-limit awareness, and optional BYOK tracking:

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

`vox populi probe` detects your GPU and recommends a QLoRA configuration. Training runs natively in Rust via [Burn](https://github.com/tracel-ai/burn) — no Python:

```bash
vox populi train --config qlora.toml
vox populi serve --model mens/runs/latest/model_final.bin --port 8080
```

~4,000 tokens/second on an RTX 4080 SUPER. Served on an OpenAI-compatible `/v1/completions` endpoint.

> **In progress.** Native training and local inference work today. Cloud-managed training and remote GPU provisioning are roadmap items.

### Agent-to-agent (A2A) messaging

Agents exchange typed, JWE-encrypted envelopes over a structured A2A bus. The local in-process bus is active in every session; HTTP relay across mesh nodes is available under the `populi-transport` feature flag.

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
| **ADRs** | [Architecture decisions](docs/src/adr/README.md) |
| **Syntax guide** | [Examples & style](examples/STYLE.md) |

---

## Design Principles

- **One source of truth.** Schema, server logic, and UI live together. The compiler keeps them in sync.
- **Zero-null discipline.** No `null`, no `undefined`. Absence is `Option[T]`; failure is `Result[T]`. Both must be handled to compile.
- **Errors are values.** `Result` and `match` replace exceptions. You cannot silently swallow a failure.
- **Durability without a framework.** `workflow` and `actor` are language keywords, not libraries.
- **A better LLM target.** A constrained, compiled language with a single coherent surface gives models less to hallucinate, and gives the compiler authority to reject bad generated code before it runs.
- **AI-native.** `@mcp.tool` is a first-class decorator. The orchestrator, agent mesh, and model routing are built into the platform.
- **Honest about maturity.** Capabilities are documented as they land — including the ones still being hardened.

---

## License

Apache-2.0. Source: [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox).
