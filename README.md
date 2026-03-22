<div align="center">
  <img src="docs/src/assets/logo.png" alt="Vox Logo" width="200" height="auto" />
  <h1>Vox Programming Language</h1>
  <p><strong>A Full-Stack, AI-Native Systems Language.</strong></p>
</div>

<img src="docs/src/assets/hero_bg.png" alt="Vox Hero Banner" width="100%" />

> *"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence! Or, shall we say, it is itself a thought, nothing but thought, and no longer the substance which we deemed it!"*  
> — **Nathaniel Hawthorne**, *The House of the Seven Gables* (1851)

---

## What is Vox?

Vox is a statically typed programming language that compiles to Rust (Tokio/Axum) for the backend and TypeScript (React) for the frontend. It is designed to combine system-level performance with the integrated developer experience of modern web frameworks. 

By unifying the database schema, server logic, and user interface into a single syntax, Vox removes the boundary between the frontend and backend. It is explicitly designed to be easily generated and reasoned about by large language models, featuring minimal syntax and a strict type system.

### Ecosystem Components

Vox consists of three primary subsystems:

1. **The Core Language**: A unified syntax with Algebraic Data Types (ADTs) and exhaustiveness checking. Null values (`null`, `undefined`) are banned in favor of `Option[T]` and `Result[T, E]`. This prevents null pointer exceptions and provides a strict contract for code generation.
2. **Vox Dei (Orchestrator)**: The runtime includes a distributed execution engine. It provides primitives for persistent state, retry mechanisms, and transparent agent-to-agent messaging. Workflows are durable by default, meaning execution state survives process restarts.
3. **Vox Populi (Native ML)**: A native machine learning layer built on Rust and Burn. It allows you to run high-throughput inference and execute QLoRA fine-tuning directly from the CLI without depending on external Python toolchains.

---

## Language Features by Example

The v0.2/v0.3 brace syntax uses explicit blocks and a clear separation of concerns within a single file. 

### 1. Data Layer and Backend Functions
Vox replaces external ORMs and migration scripts with the `@table` decorator. Types are mapped directly to an embedded SQLite database, making the data store accessible natively via the built-in `db` module.

```vox
type UserStatus = 
    | Active(last_login: int)
    | Suspended(reason: str)
    | Pending

@table type Task {
    id: int
    title: str
    done: bool
    status: UserStatus
}

// Server functions compile to Axum HTTP endpoints.
@server fn toggle_task(task_id: int) to Result[Unit] {
    let task = db.query_one("SELECT * FROM Task WHERE id = ?", task_id)
    db.execute("UPDATE Task SET done = ? WHERE id = ?", !task.done, task_id)
    ret Ok(())
}
```

### 2. Frontend Reactivity
Frontend components compile to React and emit zero-runtime CSS. Because the frontend and backend share the same type system, data fetching is type-safe by default.

```vox
import react.use_state

// Components seamlessly call backend functions.
@component fn TaskList(initial_tasks: List[Task]) to Element {
    let (tasks, set_tasks) = use_state(initial_tasks)
    
    <div class="container">
        {tasks.map(fn(task) {
            <div class="task-row">
                <input 
                    type="checkbox" 
                    checked={task.done} 
                    onChange={fn(_e) toggle_task(task.id)} 
                />
                <span>{task.title}</span>
            </div>
        })}
    </div>
}

// Route declarations map URLs to components.
routes {
    "/tasks" to TaskList
}
```

### 3. Durable Workflows and Agent Tooling
Vox provides native decorators for long-running processes and AI integrations.

```vox
// A @workflow creates a durable state machine. If the server crashes 
// during `stripe.create_intent`, it will resume from that exact step 
// upon restart, preventing duplicate charges.
@workflow fn process_payment(amount: float) to Result[Receipt] {
    let intent = await stripe.create_intent(amount)
    let receipt = await finalize_transaction(intent)
    ret Ok(receipt)
}

// The @mcp.tool decorator automatically exposes the function 
// to language models via the Model Context Protocol.
@mcp.tool fn evaluate_system_health() to SystemMetrics {
    ret system.get_metrics()
}
```

---

## Tooling and CLI

The `vox` CLI manages the entire lifecycle of your application, from local development to model training.

```bash
# Start the development server with live-reloading.
vox build app.vox -o dist --watch

# Compile the application into a single, statically-linked binary.
# This binary includes your frontend, backend, and the SQLite engine.
vox bundle app.vox --release --target x86_64-unknown-linux-musl

# Train a model using the local Populi backend and QLoRA.
vox populi train --backend qlora
```

### Installation

Compile and install the CLI from source using Cargo:

```bash
# Install the core compiler
cargo install --path crates/vox-cli
```

*For Windows environments bootstrapping against Turso*: Use the included scripts `scripts/install.ps1` or `scripts/install.sh` to configure the required C-toolchain. See the [Setup Instructions](docs/src/how-to-setup.md).

---

## Architecture and Internals

The compiler pipeline is written in Rust and operates through the following stages: Lexer (`logos`) ➔ Parser ➔ HIR Lowering ➔ Typeck ➔ Codegen backends.

For deeper technical specifics:
- **System Architecture:** [AGENTS.md](./AGENTS.md)
- **First Application Walkthrough:** [First App Documentation](docs/src/how-to/first-full-stack-app.md)
- **Syntax Corpus Details:** [STYLE.md](examples/STYLE.md)

---

## License and Sponsorship

Vox is open-source software, released under the **Apache-2.0** License. Contributions and sponsorships help sustain the development of the toolchain:

- [Sponsor on GitHub](https://github.com/sponsors/brbrainerd) <!-- Update to specific GitHub Sponsor link if needed -->
- [Support on Open Collective](https://opencollective.com/vox-lang) <!-- Update to specific Open Collective link if needed -->
