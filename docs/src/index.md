---
title: "Vox: The AI-Native Programming Language"
description: "Vox is a unified full-stack language that compiles to Rust and TypeScript. Zero null states, durable workflows, native MCP support, and a built-in ML training pipeline."
category: "getting-started"
sort_order: 0
keywords: ["Vox programming language", "AI-native language", "Rust compiler", "full-stack language"]
last_updated: 2026-03-24
training_eligible: true
difficulty: "beginner"
---

<div class="vox-hero">
  <img src="assets/logo.png" alt="Vox Logo" class="logo" />
  <h1>Vox Programming Language</h1>
  <p class="subtitle">Your backend, frontend, and database. One file. One binary. Zero nulls.</p>
  <div class="vox-cta-container">
    <a href="how-to/first-full-stack-app.html" class="vox-cta primary">Get Started</a>
    <a href="explanation/faq.html" class="vox-cta secondary">Read the FAQ</a>
    <a href="https://github.com/vox-foundation/vox" class="vox-cta secondary">GitHub</a>
  </div>
</div>

**What is Vox?** 
Vox is a unified, full-stack programming language designed to bridge the gap between high-level AI intent and low-level system performance. By compiling directly to **Rust** for backend durability and **TypeScript** for frontend reactivity, Vox enables developers to write their entire application stack in a single, LLM-friendly syntax.

## Why Vox?

The software industry has fragmented into hundreds of specialized frameworks. Vox solves this by unifying the stack natively:
- **AI-Native Grammar**: The grammar is free of syntactical ambiguities, making it easier for Large Language Models (LLMs) to generate pristine Abstract Syntax Trees (AST).
- **Uniformity**: Frontend components, backend services, and database schemas live together in one `.vox` file. Define a `@table` and you get the schema, the CRUD API, and the React types for free.
- **Durable Execution**: Workflows survive machine failures. If a server goes down during a multi-step `workflow`, Vox automatically resumes exactly where it left off upon restart.
- **Zero Null States**: Null references are completely banned from the language. All absence of value must be represented by `Option[T]` or `Result`, eliminating the most common source of runtime crashes.
- **Native ML Pipeline**: Integrated training with **Mens** allows you to perform Quantized Low-Rank Adaptation (QLoRA) directly within the Vox ecosystem.
- **First-Class AI Agents**: Adding `@mcp.tool` to any function instantly exposes it as a Model Context Protocol generic tool for external AI agents.

## Quick Start
Get your first Vox app running and deployed locally in under 5 minutes:

### 1. Install the CLI
Ensure you have Rust installed, then install the Vox compiler CLI directly:
```bash
cargo install --locked --path crates/vox-cli
```

### 2. Initialize a Project
Use the CLI to scaffold a new project with the default TanStack template:
```bash
vox init my-app && cd my-app
```

### 3. Run Your Application
Start the development server, which hot-reloads both your Rust backend and TypeScript frontend:
```bash
vox run src/main.vox
```

## What Vox Saves You

### No Duplicate Type Definitions

In a typical project, a database schema lives in SQL, a matching struct lives in your server code, and a matching TypeScript type lives in your frontend. Three places to update.

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

### No Separate API Layer

When a frontend component calls a backend function, there is no endpoint definition to write, no fetch call to wrap, no contract to keep in sync.

```vox
@server fn add_task(title: str, owner: str) to Id[Task] {
    // VoxDb acts as the unified data API (Codex/Turso)
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

style {
    .task-list { padding: "1rem" fontFamily: "sans-serif" }
    .task-row  { display: "flex" gap: "0.5rem" alignItems: "center" }
}

routes {
    "/" to TaskList
}
```

### No Null Checks, No Runtime Surprises

Vox does not have `null`. A value that might be absent is `Option[T]`, and you must handle both cases to compile.

```vox
fn find_owner(tasks: List[Task], name: str) to Option[Task] {
    tasks.find(fn(t) t.owner == name)
}

// Both branches must be covered. Missing either is a compile error.
match find_owner(tasks, "alice") {
    Some(task) -> show_task(task)
    None       -> show_empty_state()
}
```

The same applies to errors. Functions that can fail return `Result`. The `match` expression makes you handle both success and failure before the code compiles.

```vox
match complete_task(task_id) {
    Ok(())    -> "Done"
    Err(msg)  -> "Failed: " + msg
}
```

### Durable Workflows That Survive Crashes

If your server crashes halfway through charging a card, Vox picks up exactly where it left off on restart. Workflows coordinate retries natively.

```vox
activity charge_payment(amount: int, token: str) to Result[str] {
    ret Ok("tx-" + token)
}

@workflow
fn process_order(customer: str, amount: int) to Result[str] {
    let payment = charge_payment(amount, "card-tok")
        with { retries: 3, timeout: "30s", initial_backoff: "500ms" }

    ret payment
}
```

### Persistent Actors

A Vox `actor` persists its state automatically across server restarts. You do not write serialization, storage, or cache invalidation code.

```vox
actor PersistentCounter {
    on increment() to int {
        let current = state_load("counter")
        let next    = current + 1
        state_save("counter", next)
        ret next
    }
}
```

### AI Agents out-of-the-box

Add one decorator, and Vox generates the Model Context Protocol (MCP) tool schema natively.

```vox
type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

@mcp.tool "Search the knowledge base for documents matching the query"
fn search_knowledge(query: str, max_results: int) to SearchResult {
    Found("Result for: " + query, 95)
}
```

## Installation

Vox installation is managed via a unified bootstrap path that keeps logic in Rust (`vox-bootstrap`) while allowing cargo-free user installs.

- **End users (cargo-free):** `scripts/install.*` downloads a standalone `vox-bootstrap` release binary, verifies SHA-256 via release `checksums.txt`, and runs it.
- **Contributors (repo checkout):** the same scripts prefer `cargo run --locked -p vox-bootstrap` when run from the repo with Cargo available.
- With **`--install`**, bootstrap tries prebuilt **`vox`** release binaries first, then falls back to building from source when a repo checkout + Cargo are present.

Supported artifact names and targets are documented in [`docs/src/ci/binary-release-contract.md`](docs/src/ci/binary-release-contract.md).

### 1. Unified Install (Mac/Linux)
```bash
curl -fsSL https://raw.githubusercontent.com/vox-foundation/vox/main/scripts/install.sh | bash -s -- --install
```

### 2. Unified Install (Windows)
From PowerShell (repo path):
```powershell
git clone https://github.com/vox-foundation/vox.git
cd vox
.\scripts\install.ps1 -InstallClang -Apply -Install
```

From PowerShell (cargo-free path):
```powershell
$tmp = Join-Path $env:TEMP "vox-install.ps1"
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/vox-foundation/vox/main/scripts/install.ps1" -OutFile $tmp
powershell -NoProfile -ExecutionPolicy Bypass -File $tmp -Install
```

### 3. NVIDIA GPU Install (Mens QLoRA Training)
To enable native hardware-accelerated LLM QLoRA training on NVIDIA hardware, do not use the bootstrap script. Instead, build the `vox-cuda-release` alias from a Visual Studio Developer shell:
```bash
cargo vox-cuda-release
```

## Navigating the Documentation

Vox utilizes the **Diátaxis** documentation framework. Choose your path based on what you need to achieve:

### 🚀 Tutorials (Learning)
Guided, step-by-step lessons to learn the platform.
- [Tutorial: Your First Full-Stack App](tutorials/tut-first-app.md)
- [Tutorial: Actor Basics](tutorials/tut-actor-basics.md)
- [Tutorial: Workflow Durability](tutorials/tut-workflow-durability.md)

### 🛠️ How-To Guides (Problem Solving)
Goal-oriented recipes for common tasks.
- [How To: Deploy to Production](how-to/how-to-deploy.md)
- [How To: Build AI Agents & MCP Tools](how-to/how-to-ai-agents.md)
- [How To: Train Mens Models](how-to/how-to-train-mens-4080.md)

### 📚 Reference (Information)
Technical descriptions of language machinery and syntax.
- [Language Syntax Guide](reference/ref-language.md)
- [Type System Reference](reference/ref-type-system.md)
- [CLI Commands Reference](reference/cli.md)
- [Decorator Registry](reference/ref-decorators.md)

### 🧠 Explanations (Understanding)
Deep dives into the architecture and theory.
- [Compiler Lowering Phases](explanation/expl-compiler-lowering.md)
- [The Durable Execution Model](explanation/expl-durable-execution.md)
- [The Vox Runtime Architecture](explanation/expl-runtime.md)

## Join the Foundation

Vox is developed by the **Vox Foundation** under the Apache-2.0 license. We are building a future where software is declarative, distributed, and naturally understood by both humans and context-aware AIs.

- Read the source code on [GitHub](https://github.com/vox-foundation/vox)
- Review our [Architecture Decision Records (ADR)](adr/README.md)
- Check out [Golden Examples](examples/golden.md) for compiled, working snippet code.
