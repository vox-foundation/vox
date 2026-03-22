<div align="center">
  <img src="docs/src/assets/logo.png" alt="Vox Logo" width="200" height="auto" />
  <h1>Vox Programming Language</h1>
  <p>Full-stack. AI-native. Single binary.</p>
</div>

<img src="docs/src/assets/hero_bg.png" alt="Vox Hero Banner" width="100%" />

> *"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence! Or, shall we say, it is itself a thought, nothing but thought, and no longer the substance which we deemed it!"*
>
> — Nathaniel Hawthorne, *The House of the Seven Gables* (1851)

---

Vox is a compiled, statically typed programming language where your database schema, server endpoints, frontend components, background workflows, and AI agent tools all live in the same file and compile to the same artifact.

The backend emits Rust (Tokio + Axum). The frontend emits TypeScript (React). The output of `vox bundle` is a single, statically-linked binary that includes your application and an embedded SQLite engine. There is no separate build step for the frontend, no ORM to configure, and no API layer to maintain by hand.

The language is opinionated in one significant way: `null` does not exist. The type system enforces exhaustive pattern matching through Algebraic Data Types (ADTs) and `Option[T]`. This is true both for human authors and for LLMs generating Vox — the compiler gives you the same guarantees either way.

---

## The Language

### Types and Pattern Matching

Basic types are defined with `type`. ADTs support tagged variants with named fields.

```vox
type OrderStatus =
    | Pending
    | Processing(started_at: int)
    | Shipped(tracking_id: str)
    | Failed(reason: str)

fn status_label(s: OrderStatus) to str {
    match s {
        Pending              -> "Awaiting processing"
        Processing(t)        -> "Started at " + str(t)
        Shipped(tracking_id) -> "Tracking: " + tracking_id
        Failed(reason)       -> "Failed: " + reason
    }
}
```

The `match` expression must cover every variant. Missing a case is a compile error, not a runtime exception.

```vox
// Option[T] replaces null checks.
fn unwrap_or(opt: Option, default: str) to str {
    match opt {
        Some(value) -> value
        None        -> default
    }
}

// Result[T, E] replaces thrown exceptions.
fn chain(first: Result, second: Result) to Result {
    match first {
        Ok(v)    -> second
        Err(msg) -> Err(msg)
    }
}
```

### Data Layer

Tables are declared with `@table` and indexed via `@index`. The `db` module exposes typed query, insert, and patch operations that map directly to the declared schema.

```vox
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}

@index Task.by_done  on (done, priority)
@index Task.by_owner on (owner)
```

Server functions that operate on the database compile to Axum HTTP handlers.

```vox
@server fn add_task(title: str, owner: str) to Id[Task] {
    ret db.insert(Task, { title: title, done: false, priority: 0, owner: owner })
}

@server fn complete_task(id: Id[Task]) to Result[Unit] {
    db.patch(id, { done: true })
    ret Ok(())
}
```

### Frontend Components

Components compile to React. State is managed with `use_state`. The compiler knows the types of your backend functions, so data flowing from `@server` to `@component` is type-checked across the boundary.

```vox
import react.use_state

@component fn TaskList(tasks: List[Task]) to Element {
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

style {
    .task-list { padding: "1rem" fontFamily: "sans-serif" }
    .task-row  { display: "flex" gap: "0.5rem" alignItems: "center" }
}

routes {
    "/" to TaskList
}
```

### Actors

An `actor` is an isolated entity with a mailbox and durable state. State is persisted through `state_load` / `state_save` and survives process restarts. The frontend can `spawn` an actor and call its methods directly.

```vox
actor PersistentCounter {
    on increment() to int {
        let current = state_load("counter")
        let next    = current + 1
        state_save("counter", next)
        ret next
    }
}

@component fn CounterApp() to Element {
    let (count, set_count) = use_state(0)
    let bump = fn(_e) set_count(spawn(PersistentCounter).increment())

    <div class="counter">
        <p>"Count: " {count}</p>
        <button on_click={bump}>"+"</button>
    </div>
}
```

### Workflows

Workflows model multi-step operations as durable state machines. Each step is an `activity`. The `with` expression applies retry and timeout policies per-step.

If the process dies between `validate_order` and `charge_payment`, the workflow resumes from the last successfully completed activity — not from the beginning. Side effects are not replayed.

```vox
activity validate_order(data: str) to Result[str] {
    ret Ok("validated-" + data)
}

activity charge_payment(amount: int, token: str) to Result[str] {
    let tx = "tx-" + token
    ret Ok(tx)
}

activity send_confirmation(recipient: str, order_id: str) to Result[str] {
    ret Ok("Confirmed " + order_id + " for " + recipient)
}

workflow process_order(customer: str, data: str, amount: int) to Result[str] {
    let validated = validate_order(data)
        with { timeout: "5s" }

    let payment = charge_payment(amount, "card-tok")
        with { retries: 3, timeout: "30s", initial_backoff: "500ms" }

    let confirmation = send_confirmation(customer, "order-001")
        with { retries: 2, activity_id: "confirm-order-001" }

    ret confirmation
}
```

### Agent Tools (MCP)

The `@mcp.tool` decorator registers a function with the Model Context Protocol. The description string is included in the tool schema delivered to language models. This is the mechanism through which LLMs running in agent mode can call into your application.

```vox
type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

@mcp.tool "Search the knowledge base for documents matching the query"
fn search_knowledge(query: str, max_results: int) to SearchResult {
    Found("Result for: " + query, 95)
}

@mcp.tool "Run a read-only SQL query and return results as JSON"
fn run_query(sql: str) to str {
    ret "[]"
}
```

### Built-in Testing

`@test` functions are first-class. Run them with `vox test`.

```vox
type Shape =
    | Circle(radius: int)
    | Rectangle(width: int, height: int)

fn area(s: Shape) to int {
    match s {
        Circle(r)           -> r * r
        Rectangle(w, h)     -> w * h
    }
}

@test fn test_circle() to Unit {
    assert(area(Circle(5)) is 25)
}

@test fn test_rectangle() to Unit {
    assert(area(Rectangle(4, 6)) is 24)
}
```

---

## The CLI

The `vox` binary is the single entry point for the full lifecycle.

```
vox build <file>           Compile .vox → Rust + TypeScript
vox run <file>             Build and run locally
vox bundle <file>          Produce a statically linked binary
vox test <file>            Execute @test functions
vox check <file>           Type-check without output
vox fmt <file>             Format source (in development)
vox compact <file>         Compact source for LLM context windows
vox lsp                    Launch the Language Server (tower-lsp)
vox doc                    Generate documentation
vox init [name]            Scaffold a new project
vox install <pkg>          Install a package
vox audit                  Check dependencies for advisories
vox clean                  Remove build artifacts
vox stub-check             Run TOESTUB anti-pattern detection
vox orchestrator <action>  Manage multi-agent task queues
vox dashboard              Open the orchestrator HUD web UI
vox agent <action>         Register and manage AI agents
vox populi train           Train a Populi model locally
vox populi review          Run an AI-assisted code review
vox scientia               Research and query the local knowledge store
vox share <action>         Publish or browse shared artifacts
vox snippet <action>       Save and search code snippets
vox review coderabbit …    Submit semantic batched PRs to CodeRabbit
```

Build and deploy:

```bash
# Development with live-reload
vox build app.vox -o dist --watch

# Single statically-linked binary: backend + frontend + SQLite
vox bundle app.vox --release --target x86_64-unknown-linux-musl

# Scaffold a new project
vox init my-project

# Run all @test functions in a file
vox test app.vox
```

---

## Vox Populi — Native ML

Populi is the native ML subsystem. It runs entirely in Rust — no Python runtime, no subprocess — using Burn for training and Candle for inference.

```bash
# Fine-tune a local model with QLoRA (CPU baseline, also supports CUDA)
vox populi train --backend qlora --tokenizer hf --model ./model

# Run inference against the local Populi model
vox populi run --prompt "Write a Vox server function that returns all tasks"

# Inspect model and quota status
vox populi status --quotas
```

Populi accepts training data in JSONL format. Categories are filterable via `context_filter`. Checkpoints emit with a configurable `adapter_tag`. Training state is reported to the orchestrator in real time.

---

## Vox Dei — Distributed Execution Intelligence

The Dei orchestrator manages multi-agent task coordination. Agents are assigned to file affinities (matched by path scope), dispatched to task queues, and their messaging is logged transparently for replay and debugging.

```bash
vox orchestrator status          # View active agents and current task state
vox orchestrator dispatch <task> # Push a task into the queue
vox dashboard                    # Open the real-time HUD
vox agent list                   # Show registered agents and their capabilities
```

The orchestrator integrates with TOESTUB — a rule engine that detects known anti-patterns in Vox code before they propagate. Run it standalone or as part of CI:

```bash
vox stub-check --all
```

---

## Installation

Install from source via Cargo:

```bash
cargo install --path crates/vox-cli
```

**Windows (Turso integration):** The Turso embedded database requires LLVM/clang-cl. Use `scripts/install.ps1` to configure the C-toolchain automatically. Full notes in the [Setup Guide](docs/src/how-to-setup.md).

**CUDA (Populi QLoRA on NVIDIA):** Build with the `gpu` feature:

```bash
cargo build -p vox-cli --release --features gpu
```

Run this from a Visual Studio Developer shell on Windows so `nvcc` resolves MSVC correctly.

---

## Architecture

```
Lexer (logos) → Parser (Rowan CST) → AST → HIR → Typeck → Codegen
                                                              ├── Rust (Axum/Tokio)
                                                              └── TypeScript (React)
```

The parser produces a lossless Rowan-based green tree for full LSP support with error recovery. Type checking is bidirectional with Hindley-Milner inference and unification. Codegen emits Rust via `quote!` and structured TypeScript.

For a complete reference: [AGENTS.md](./AGENTS.md).

---

## Resources

- **First full-stack application walkthrough:** [docs/src/how-to/first-full-stack-app.md](docs/src/how-to/first-full-stack-app.md)
- **Language syntax and style:** [examples/STYLE.md](examples/STYLE.md)
- **CLI reference:** [docs/src/ref-cli.md](docs/src/ref-cli.md)
- **Architecture and roadmap:** [AGENTS.md](./AGENTS.md)

---

## License and Support

Apache-2.0. Source is at [github.com/brbrainerd/vox](https://github.com/brbrainerd/vox).

Sponsorship links will appear here when the GitHub Sponsors and Open Collective pages are live.
