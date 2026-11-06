---
title: "Getting Started: From 60-Second Sandbox to Full-Stack & Axis IDE"
description: "Run your first .vox program in 60 seconds, build a single-file full-stack app in 5 minutes, and launch the Axis Agent Workbench in 15 minutes."
category: "Tutorials"
status: "current"
sort_order: 1
topic: "language-grammar"
domain: "language"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
schema_type: "HowTo"
keywords: ["Vox installation", "getting started Vox", "AI programming language tutorial", "Rust TypeScript compiler"]
code_anchors:
  - "examples/golden/getting_started.vox"
  - "crates/vox-cli/src/main.rs"
---

# Getting Started: From 60-Second Sandbox to Full-Stack & Axis IDE

<!-- kb:summary -->
This quickstart guides developers through Vox in three hands-on stages: first, running `.vox` immediately in the browser compiler playground or sandboxed `vox run` interpreter in 60 seconds with zero build wait; second, authoring a 5-minute single-file full-stack application combining `table`, `query`, `mutation`, `server`, `component`, and `tool` declarations; and third, building the `vox` CLI from source in 15 minutes to run durable `workflow`/`activity` journals and launch the Vox Axis desktop workbench.
<!-- /kb:summary -->

Pick the starting depth that matches what you want to build right now:

```vox-diagram
graph {
    direction: right
    node T1: "60 Seconds: Zero-Install Playground & vox run"
    node T2: "5 Minutes: Single-File Full-Stack App"
    node T3: "15 Minutes: Source Build, Workflows & Axis GUI"
    T1 -> T2
    T2 -> T3
}
```

---

## 1. 60-Second Zero-Install Sandbox

You do **not** need to compile the Rust workspace to try Vox. You can run `.vox` programs immediately in the [Interactive `.vox` Compiler Playground](../index.mdx#try-it) or, if `vox` is already on your `PATH`, execute any `.vox` script directly in the capability-sandboxed interpreter (`vox run --interp`).

Create a single file `hello_tasks.vox`:

```vox
table Task {
    title: str
    done:  bool
}

query count_tasks() to int {
    return len(db.Task.all())
}

tool "Mark a task complete in the database" complete_task(id: Id[Task]) to Result[str] {
    db.Task.delete(id)?
    return Ok("done")
}

mutation seed_and_complete() to Result[str] {
    let id = db.Task.insert({ title: "Run Vox in 60 seconds", done: false })?
    print("Tasks queued: " + str(count_tasks()))
    let status = complete_task(id)?
    print("Tasks remaining: " + str(count_tasks()))
    return Ok(status)
}
```

Run it immediately in the interpreter with zero Rust compile step:

```bash
vox run --interp hello_tasks.vox
```

What happens in those 22 lines:
- **`table Task { ... }`** defines an in-memory or Turso/SQLite table with a typed primary key (`Id[Task]`).
- **`query count_tasks() to int`** declares a read-only database query.
- **`tool "..." complete_task(...) to Result[str]`** exposes an AI-callable Model Context Protocol (MCP) tool that returns `Result[str]` (`Ok` or `Error`) instead of throwing hidden exceptions.

---

## 2. 5-Minute Single-File Full-Stack App

This section is only the sketch: one file, a few declarations, then `vox check` and `vox run`. The task-list build is [First Full-Stack App](tut-first-app.md). The side-by-side SQL, Axum, and React output is the [full-stack walkthrough](tut-fullstack-walkthrough.mdx).

When you are ready to turn a script into a full-stack web application (database schema + Rust Axum HTTP endpoints + React/TypeScript UI), scaffold a project:

```bash
vox init my-app
cd my-app
```

Open `src/main.vox`. One `.vox` file declares your data model, backend server endpoints, reactive UI component, and HTTP routes:

```vox
table Note {
    title: str
    content: str
    created_at: str
}

server add_note(title: str, content: str) to Result[str] {
    return Ok("Added: " + title)
}

server list_notes() to Result[str] {
    return Ok("[]")
}

component App() {
    view: column(raw_class="app") {
        heading(level=1) { "My Vox App" }
        text() { "Edit src/main.vox to get started" }
    }
}

routes {
    "/" to App
}
```

### How One `.vox` File Compiles to 4 Targets

![One .vox file fanning out into a database, a server, and a browser UI](../assets/one-file-three-layers.png)

| Declaration | Purpose | What `vox build` Emits |
|---|---|---|
| `table Note { ... }` | Single source of truth for data | SQLite/Turso DDL + migration diff + typed Rust/TS row structs |
| `server add_note(...)` | Backend write endpoint | Typed Rust Axum route handler with `Result[T]` error propagation |
| `server list_notes()` | Backend read endpoint | Typed Rust Axum route handler + generated TypeScript fetch client |
| `component App()` | Reactive browser UI | TypeScript / React (`.tsx`) component |
| `routes { "/" to App }` | URL routing table | Client and server router bindings |

Type-check and run your application:

```bash
vox check src/main.vox
vox run src/main.vox
```

---

## 3. 15-Minute Agent Harness, Durable Workflows & Axis GUI

> [!NOTE]
> **Building `vox` from Source:** Pre-1.0 workspace builds compile the Rust compiler, orchestrator, and native runtime crates from source (`rust-toolchain.toml` pins the workspace Rust channel). See [Installing Vox](../reference/installation.md) for platform prerequisites.

### Step 1: Install, then verify

Install from source with [Installing Vox](../reference/installation.md). The same clone and `cargo install` commands are on the [repository README](https://github.com/vox-foundation/vox#trying-it). This tutorial does not repeat them. Then confirm the toolchain:

```bash
vox doctor
```

### Step 2: Add a Crash-Safe Durable Workflow

Unlike ordinary functions that lose state if a server restarts mid-request, `activity` and `workflow` blocks journal every step ordinal to SQLite/Turso so interrupted runs replay deterministically:

```vox
activity charge_card(amount_cents: int) to Result[str] {
    if amount_cents > 50000 {
        return Error("Amount exceeds single-charge limit")
    }
    return Ok("tx_verified_9001")
}

workflow process_subscription(amount_cents: int) to str {
    let res = charge_card(amount_cents)
    match res {
        Ok(tx) => "Charged receipt: " + tx
        Error(reason) => "Declined: " + reason
    }
}
```

### Step 3: Launch the Vox Axis Desktop Workbench

Start the orchestrator daemon and open the [Vox Axis Desktop Workbench](../how-to/how-to-axis-workbench.md) (`vox-gui`) to inspect live MCP tool calls, Loquela risk/clutch posture controls (`Alt+1..4`), and the 132-crate Architecture Atlas:

```bash
cargo run -p vox-gui
```

## Next Steps

Each link owns one job. Stay on that page for that job.

- **[First Full-Stack App](tut-first-app.md)** — fill in the task list: mutations, component, and route.
- **[Interactive Full-Stack Walkthrough](tut-fullstack-walkthrough.mdx)** — compare one `.vox` file with the SQL, Axum, and React it emits.
- **[UI with VUV view-calls](tut-ui-integration.md)** — compose `component` trees, iteration, and typed style kwargs.
- **[Use a React component from Vox](use-a-react-component-from-vox.md)** — import an existing React component.
- **[Actors](tut-actor-basics.md)** — persistent actors and identity.
- **[Durable Workflows](tut-workflow-durability.md)** — replayable activities, timers, and crash recovery. The short workflow in section 3 is not that lesson.
- **[Vox Axis Desktop Workbench](../how-to/how-to-axis-workbench.md)** — Loquela posture controls, `@kb` pins, and panel layout. That guide is a how-to, not a tutorial.
- **Published adapters** — [Qwen3 0.6B](https://huggingface.co/brbrainerd/vox-mens-qwen3-0.6b), [1.7B](https://huggingface.co/brbrainerd/vox-mens-qwen3-1.7b), [4B](https://huggingface.co/brbrainerd/vox-mens-qwen3-4b), and the [index](https://huggingface.co/brbrainerd/vox-mens-index). `vox mens hub download --repo brbrainerd/vox-mens-qwen3-4b --output-dir <dir>` fetches a card when `vox-ml-cli` is built with `mens-hf-hub`. The 8B adapter is not public.
- **[Language Syntax Reference](../reference/ref-syntax.md)** — types, pattern matching, and decorators.
