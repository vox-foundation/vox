<div align="center">
  <img src="docs/src/assets/logo.png" alt="Vox Logo" width="200" height="auto" />
  <h1>Vox Programming Language</h1>
  <p>Your backend, frontend, and database. One file. One binary.</p>
</div>

<img src="docs/src/assets/hero_bg.png" alt="Vox Hero Banner" width="100%" />

> *"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence! Or, shall we say, it is itself a thought, nothing but thought, and no longer the substance which we deemed it!"*
>
> — Nathaniel Hawthorne, *The House of the Seven Gables* (1851)

---

Most full-stack projects are really three projects stitched together: a database, a backend, and a frontend, each with its own tooling, its own type definitions, and its own deployment step. Most bugs live at the seams between them.

Vox is a single language for all three layers. You define your data shapes once. Your server functions and your UI components share those types directly. The compiler enforces that every piece fits together. The output is one binary.

```bash
vox bundle app.vox --release --target x86_64-unknown-linux-musl
```

That binary contains your backend (Rust/Axum), your frontend (React/TypeScript), and an embedded SQLite database. Deploy it anywhere that runs a Linux binary. No runtime, no Docker required, no npm install on the server.

The other big decision Vox makes for you: there is no `null`. Every value that might be absent is explicit. Every error path that might fail is explicit. If you forget to handle a case, it is a compile error. This removes a whole class of bugs that typically show up in production, not in tests.

---

## What Vox saves you

### No more duplicate type definitions

In a typical project, a database schema lives in SQL, a matching struct lives in your server code, and a matching TypeScript type lives in your frontend. Three places to update when the shape of your data changes.

In Vox, the schema and the type are the same declaration:

```vox
@table type Task {
    title:    str
    done:     bool
    priority: int
    owner:    str
}

// This index is created automatically in SQLite.
@index Task.by_owner on (owner)
```

Server functions read and write that type directly, and frontend components receive it as-is. No mapping layer in between.

### No separate API layer

When a frontend component calls a backend function, there is no endpoint definition to write, no fetch call to wrap, no contract to keep in sync.

```vox
@server fn add_task(title: str, owner: str) to Id[Task] {
    ret db.insert(Task, { title: title, done: false, priority: 0, owner: owner })
}

@server fn complete_task(id: Id[Task]) to Result[Unit] {
    db.patch(id, { done: true })
    ret Ok(())
}
```

The compiler generates the HTTP endpoint, the TypeScript client call, and the serialization. Your component just calls the function.

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

### No null checks, no runtime surprises

In most languages, `null` or `undefined` can appear anywhere. You find out about it at runtime. Vox does not have `null`. A value that might be absent is `Option`, and you must handle both cases to compile.

```vox
fn find_owner(tasks: List[Task], name: str) to Option {
    tasks.find(fn(t) t.owner == name)
}

// Both branches must be covered. Missing either is a compile error.
match find_owner(tasks, "alice") {
    Some(task) -> show_task(task)
    None       -> show_empty_state()
}
```

The same applies to errors. Functions that can fail return `Result`. The `match` expression makes you handle both the success and failure path before the code compiles.

```vox
match complete_task(task_id) {
    Ok(())    -> "Done"
    Err(msg)  -> "Failed: " + msg
}
```

### Long-running tasks that survive crashes

If your server processes a multi-step operation — charge a card, send an email, update a ledger — and crashes halfway through, the typical outcome is inconsistent state. You build retry logic, idempotency keys, and recovery handlers to compensate.

Vox has a first-class construct for this: `workflow`. Each step is an `activity`. If the process dies after step 2 of 4, on restart it picks up at step 3. Steps are not re-executed.

```vox
activity validate_order(data: str) to Result[str] {
    ret Ok("validated-" + data)
}

activity charge_payment(amount: int, token: str) to Result[str] {
    ret Ok("tx-" + token)
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

The `with` expression sets per-step retry and timeout policy inline. No separate configuration file. No wrapper library.

### Persistent actors

Actors are the right model for things with long-lived state: a counter, a cart, a session, a rate limiter. A Vox `actor` persists its state automatically. You do not write serialization, storage, or cache invalidation code.

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

`spawn(PersistentCounter)` gives you a handle to the actor. Its internal state survives server restarts.

### AI agent tools, without a framework

If you want a language model to be able to call one of your functions, you add one decorator. The compiler generates the tool schema and registers it with the Model Context Protocol server automatically.

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

The description string becomes part of the tool schema that the model sees. The types flow through unchanged.

### Built-in testing

Tests live in the same file as the code they test. No test runner to configure.

```vox
type Shape =
    | Circle(radius: int)
    | Rectangle(width: int, height: int)

fn area(s: Shape) to int {
    match s {
        Circle(r)        -> r * r
        Rectangle(w, h)  -> w * h
    }
}

@test fn test_circle() to Unit {
    assert(area(Circle(5)) is 25)
}

@test fn test_rectangle() to Unit {
    assert(area(Rectangle(4, 6)) is 24)
}
```

```bash
vox test app.vox
```

---

## The CLI

```
vox build <file>           Compile .vox to Rust + TypeScript
vox run <file>             Build and run locally
vox bundle <file>          Produce a statically-linked binary
vox test <file>            Run @test functions
vox check <file>           Type-check without producing output
vox fmt <file>             Format (in development)
vox compact <file>         Compact source for LLM context windows
vox lsp                    Launch the Language Server
vox doc                    Generate documentation
vox init [name]            Scaffold a new project
vox install <pkg>          Install a package
vox audit                  Check dependencies for security advisories
vox clean                  Remove build artifacts
vox stub-check             Run TOESTUB anti-pattern detection
vox orchestrator <action>  Manage multi-agent task queues
vox dashboard              Open the orchestrator web UI
vox agent <action>         Register and manage AI agents
vox populi train           Train a local Populi model
vox populi review          AI-assisted code review
vox scientia               Query the local knowledge store
vox share <action>         Publish or browse shared artifacts
vox snippet <action>       Save and search code snippets
vox review coderabbit …    Submit semantic batched PRs to CodeRabbit
```

---

## Vox Populi — Local ML

Populi is a Rust-native machine learning layer. It runs entirely in-process — no Python subprocess, no external model server.

```bash
# Fine-tune a local model with QLoRA
vox populi train --backend qlora --tokenizer hf --model ./model

# Run inference against the trained model
vox populi run --prompt "Write a Vox server function that returns all tasks"

# Check model status and token quotas
vox populi status --quotas
```

QLoRA fine-tuning accepts training data in JSONL format. GPU training on NVIDIA hardware is supported with the `gpu` feature flag (see Installation).

---

## Vox Dei — Agent Orchestration

Dei coordinates multi-agent execution. Agents are assigned to task queues by file scope, their messaging is logged for replay, and the orchestrator maintains execution history for debugging.

```bash
vox orchestrator status          # View active agents and queued tasks
vox orchestrator dispatch <task> # Add a task to the queue
vox dashboard                    # Open the real-time web UI
vox agent list                   # Show registered agents
```

TOESTUB is a static rule engine that checks Vox source for known anti-patterns before they land in a PR:

```bash
vox stub-check --all
```

---

## Installation

```bash
cargo install --path crates/vox-cli
```

**Windows (Turso):** Requires LLVM/clang-cl. Use `scripts/install.ps1` to configure the C-toolchain. See the [Setup Guide](docs/src/how-to-setup.md).

**NVIDIA GPU (QLoRA training):** Build from a Visual Studio Developer shell:

```bash
cargo build -p vox-cli --release --features gpu
```

---

## Architecture

```
Lexer → Parser (lossless CST) → AST → HIR → Typeck → Codegen
                                                        ├── Rust (Axum/Tokio)
                                                        └── TypeScript (React)
```

The parser is error-recovering, meaning the Language Server can provide diagnostics on incomplete files. Type inference is bidirectional. For a full breakdown: [AGENTS.md](./AGENTS.md).

---

## Resources

- **First application walkthrough:** [docs/src/how-to/first-full-stack-app.md](docs/src/how-to/first-full-stack-app.md)
- **Language syntax reference:** [examples/STYLE.md](examples/STYLE.md)
- **CLI reference:** [docs/src/ref-cli.md](docs/src/ref-cli.md)
- **Architecture:** [AGENTS.md](./AGENTS.md)

---

## License

Apache-2.0. Source: [github.com/brbrainerd/vox](https://github.com/brbrainerd/vox).

Sponsorship links (GitHub Sponsors, Open Collective) will appear here when live.
