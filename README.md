<div align="center">
  <img src="docs/src/assets/logo.png" alt="Vox Logo" width="200" height="auto" />
  <h1>Vox Programming Language</h1>
  <p><strong>The Native Language of the AI Era.</strong></p>
</div>

<img src="docs/src/assets/hero_bg.png" alt="Vox Hero" width="100%" />

> *"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence! Or, shall we say, it is itself a thought, nothing but thought, and no longer the substance which we deemed it!"*  
> — **Nathaniel Hawthorne**, *The House of the Seven Gables* (1851)

---

## The Vox Ecosystem

Vox is an aggressively type-safe, AI-native systems language that compiles directly into high-performance **Rust (Tokio/Axum)** on the backend and **TypeScript (React)** on the frontend. 

But it’s far more than just a programming language. It is a unified, end-to-end framework designed from the ground up for small engineering teams and autonomous AI agents building modern, distributed, intelligence-driven software.

### Why Adopt Vox?

Vox solves the core friction points of modern development by unifying your entire stack. It is engineered both for exceptional developer ergonomics—reducing cognitive load for humans—and strict predictability—eliminating ambiguity for Large Language Models.

Here are the dimensions that make the Vox ecosystem unmatched:

- **Unmatched Developer Ergonomics**: Define your data schema, your frontend React components, and your backend server logic in a single, cohesive file. Avoid the boilerplate of GraphQL schemas, ORMs, and context-switching between different repositories.
- **Vox Dei (Distributed Execution Intelligence)**: The orchestrator is built straight into the system. Vox Dei provides transparent agent-to-agent messaging protocols and real-time execution recording. Building complex, durable workflow orchestration is no longer an infrastructure challenge; it is a native language primitive. 
- **Vox Populi (Native ML)**: Vox features a completely native Rust/Burn machine learning ecosystem. You can execute high-throughput inference, perform QLoRA fine-tuning, and coordinate model states directly from the language workspace, bridging the gap between application logic and raw tensor performance.
- **Zero-Null Type Safety (No Bugs)**: `null` and `undefined` states are unconditionally banned. Vox enforces rigorous, exhaustive pattern matching via Algebraic Data Types (ADTs) and `Option[T]`. This absolutely guarantees no sudden runtime NPE crashes, creating an ironclad predictability contract that ensures AI coding agents output flawless syntax.
- **A Fully-Loaded CLI Hub**: The `vox` CLI acts as your command center. Access instantaneous local live-reloading (`vox build`), compile highly-optimized statically linked applications (`vox bundle`), trace execution routes, and manage LLM data pipelines all from one terminal window.

---

## Code That Looks Like Magic (v0.3 Brace Syntax)

With the modernized v0.2/v0.3 brace syntax, writing code feels intuitive, lightweight, and structured. 

### 1. Unified Types and Server Logic
No external ORMs needed. Your types safely map to the embedded SQLite backend through the `db.` API.

```vox
type UserStatus = 
    | Active(last_login: int)
    | Suspended(reason: str)
    | Pending

@table type Task {
    title: str
    done: bool
}

@server fn toggle_task(id: int) to Result[Unit] {
    let task = db.query_one("SELECT * FROM Task WHERE id = ?", id)
    db.execute("UPDATE Task SET done = ? WHERE id = ?", !task.done, id)
    ret Ok(())
}
```

### 2. Frontend Components and Clean Routing
Components fluidly return JSX blocks without verbose `render` functions. Define your frontend layout alongside your logic, and attach top-level routes seamlessly.

```vox
import react.use_state

@component fn TaskItem(task: Task) to Element {
    let (checked, set_checked) = use_state(task.done)
    
    <div class="task-ui">
        <input 
            type="checkbox" 
            checked={checked} 
            onChange={fn(_e) {
                set_checked(!checked)
                toggle_task(task.id)
            }} 
        />
        <p>{task.title}</p>
    </div>
}

routes {
    "/" to TaskItem
}
```

### 3. Agent Tooling and Durable Workflows
An `activity` inside Vox is a durable step that reliably survives sudden process crashes. Concurrently, empowering an autonomous AI agent to access your backend logic is as trivial as attaching an `@mcp.tool` decorator for automated Model Context Protocol mapping.

```vox
// Automatically survives server drops via the Orchestrator
@workflow fn process_payment(amount: float) to Result[Receipt] {
    let intent = await stripe.create_intent(amount)
    let receipt = await finalize_transaction(intent)
    ret Ok(receipt)
}

// Instantly exported as a tool to the internal Vox Dei agent system
@mcp.tool fn evaluate_system_health() to float {
    ret system.cpu_usage() * 100.0
}
```

---

## Build and Deployment

Go from local iteration to a globally distributed native binary with incredible velocity.

```bash
# Iterative local development with instantaneous live-reload
vox build app.vox -o dist --watch

# Compile an entirely independent, statically-linked binary containing Frontend + Backend + SQLite
vox bundle app.vox --release --target x86_64-unknown-linux-musl

# Engage the Vox Populi native tensor capabilities directly
vox populi train --backend qlora
```

### Installation

Vox is designed in Rust for ultimate performance.

```bash
# Securely install directly via Cargo
cargo install --path crates/vox-cli
```

*Are you bootstrapping natively on Windows against Turso pipelines? Use `scripts/install.ps1` for an enforced toolchain configuration.* See the complete [Setup Instructions](docs/src/how-to-setup.md).

---

## Deep Dive Documentation

Delve into the underlying architecture. The compiler stack is built purely in Rust (Lexer `logos` ➔ Parser ➔ HIR Lowering ➔ Typeck ➔ Codegen backends).

- **Architecture Roadmap & Guidelines:** [AGENTS.md](./AGENTS.md)
- **Walking Through Your First Application:** [First App Documentation](docs/src/how-to/first-full-stack-app.md)
- **Syntax and Coding Standard Corpus:** [STYLE.md](examples/STYLE.md)

---

## Support and Open Source

Vox is proudly open-source under the **Apache-2.0** License. Help us build the unified technology stack of the AI era.

- 🌱 [Sponsor us on GitHub](https://github.com/sponsors)
- ☕ [Support the Vox Engineering Team on Open Collective](https://opencollective.com)
