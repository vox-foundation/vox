<div align="center">
  <img src="docs/src/assets/vox_hero_banner.jpeg" alt="Vox - The human voice acting as the great nerve of intelligence" width="100%" />

  <br><br>

  <p><strong>The end of the API layer. Build your database, backend, and UI in one seamless language.</strong></p>
  <p><a href="https://vox-lang.org"><strong>vox-lang.org</strong></a></p>

  <br>

  <p>
    <em>"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence! Or, shall we say, it is itself a thought, nothing but thought, and no longer the substance which we deemed it!"</em>
    <br>
    — Nathaniel Hawthorne, <em>The House of the Seven Gables</em> (1851)
  </p>

</div>

---

Most full-stack projects force you to manually wire a SQL schema, backend types, REST endpoints, and frontend components together. When one drifts, the bug is silent until runtime.

Vox is a language and compiler that collapses these layers into one. You write your data model, server logic, and UI side-by-side. The compiler generates the endpoint wiring, client serialization, and TypeScript types — so you never write that glue by hand. The output is standard **Rust** on the server and **TypeScript/React** in the browser, so you're not locked into a runtime you don't control.

It's aimed at developers building web applications, internal tools, data pipelines, or AI agent integrations who are tired of the same boilerplate every project.

## The CLI

Install the `vox` binary from this repository (see **Quick Start** below). For a curated, task-oriented list of subcommands, run **`vox commands --recommended`**, then `vox --help` for the full tree. Operator reference: [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

## The Language in Five Steps

### 1. Functions and types

```vox
fn hello(name: str) to str {
    ret "Hello " + name + "!"
}
```

Typed parameters, explicit returns, no semicolons. If you know Python or Rust, it reads immediately.

### 2. Your database schema and your application type are the same thing

Normally: write a SQL migration, write a Rust struct, write a TypeScript interface, keep all three in sync. In Vox, you write one declaration and the compiler derives the rest:

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
```

`@table` creates the database table. `@server` generates the HTTP endpoint and a matching TypeScript client function. You write the business logic; the compiler generates the surface.

The query DSL is fluent and compiled — the compiler validates your queries against your schema:

```vox
@query
fn recent_incomplete_tasks() to List[Task] {
    ret db.Task.where({ done: false }).order_by("priority", "desc").limit(10)
}
```

### 3. Frontend components call backend functions directly

No fetch wrappers. No REST endpoint definitions. No client SDK to maintain. `complete_task` is a `@server` function — the compiler generates the network call, serialization, and type safety across the boundary:

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

### 4. A type system that eliminates whole categories of bugs at compile time

Vox has no `null` and no `undefined`. If a value can be absent, you use `Option[T]`. If a function can fail, it returns `Result[T]`. You cannot ignore either — the compiler forces you to handle both branches:

```vox
type AppResult =
    | Success(value: int)
    | Failure(err: str)

fn serialize_result(r: AppResult) to str {
    match r {
        Success(val) -> "num:" + str(val)
        Failure(err) -> "err:" + err
    }
}
```

This same pattern applies everywhere — database queries, network calls, workflow steps. Missing a branch is a compile error, not a runtime crash at 2am.

### 5. Workflows that survive server crashes, actors that remember state

If your server goes down mid-payment, Vox's workflow runtime replays completed steps on restart. The durability is enforced by the compiler and runtime together — not a library you bolt on:

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

Actors persist state across restarts — useful for session state, rate limiters, counters, or any long-lived stateful service:

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

### 6. Expose any function to AI agents with one line

Add `@mcp.tool` and the compiler generates a [Model Context Protocol](https://modelcontextprotocol.io/) tool schema. Claude, Cursor, VS Code Copilot, and any MCP-compatible agent can discover and call your function immediately. No OpenAPI spec. No tool registration:

```vox
type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

@mcp.tool "Search the knowledge base for documents matching the query"
fn search_knowledge(query: str, max_results: int) to SearchResult {
    Found("Result for: " + query, 95)
}
```

The type signature is the contract. The compiler verifies it.

### 7. Tests built into the language

```vox
fn mock_db_read() to str {
    ret "mock_data"
}

@test
fn test_user_count() to Unit {
    let users = ["alice", "bob"]
    assert(len(users) > 0)
    let db_val = mock_db_read()
    assert(db_val is "mock_data")
}
```

`@test` is a language construct. Tests run with `vox test` and integrate with the compiler's type checker.

More examples: [`examples/golden/`](examples/golden/).

---

## Quick Start

### Install

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/vox-foundation/vox/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/vox-foundation/vox/main/scripts/install.ps1 | iex
```

If you already cloned the repo, run `./scripts/install.sh` or `.\scripts\install.ps1` directly.

### Build and run

```bash
vox init my-app
cd my-app
vox run src/main.vox
```

### Explore the CLI

```text
vox commands --recommended   List the most important starter commands
vox doctor                   Verify toolchain and environment
vox check <file>             Fast type validation without full build
vox build <file>             Compile and inspect generated output
vox run <file>               Execute app locally end-to-end
vox test <file>              Run @test functions
vox bundle <file>            Produce deployable binary output
```

Full command reference: [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

---

## What's Real Today

The language, compiler, and CLI are real and actively developed. Some areas to set expectations on:

- **Core language** (functions, types, ADTs, match, actors, workflows) — active and documented.
- **Database layer** (`@table`, `db.*` queries) — active; backed by SQLite/LibSQL.
- **Frontend / island components** — active; emits TypeScript/React.
- **MCP agent tooling** (`@mcp.tool`, `@mcp.resource`) — active.
- **VS Code extension** — active; includes LSP, MCP workspace chat, and Oratio voice-to-code.
- **Distributed/WASM/GPU training** — experimental; documented as such.

The docs are written to reflect what the repo currently does, not a roadmap.

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

- **One language surface.** Schema, server logic, and UI described from one source. The compiler keeps them in sync.
- **Zero-null discipline.** No `null`, no `undefined`. Absence is `Option[T]`; failure is `Result[T]`. Both must be handled to compile.
- **Errors are values.** `Result` and `match` replace exceptions. You cannot silently swallow a failure.
- **Durability without a framework.** `workflow` and `actor` are language keywords. Replay and state persistence come from the compiler and runtime, not a dependency.
- **AI-native surface.** `@mcp.tool` is a first-class decorator, not an integration plugin.

---

## License

Apache-2.0. Source: [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox).
