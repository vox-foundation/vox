<div align="center">
  <img src="docs/src/assets/logo.png" alt="Vox Logo" width="200" height="auto" />
  <h1>Vox Programming Language</h1>
  <p>Your backend, frontend, and database. One file. One binary. Zero nulls.</p>
  <p><strong><a href="https://vox-lang.org">vox-lang.org</a></strong></p>
</div>

<img src="docs/src/assets/hero_bg.png" alt="Vox Hero Banner" width="100%" />

> *"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence! Or, shall we say, it is itself a thought, nothing but thought, and no longer the substance which we deemed it!"*
>
> — Nathaniel Hawthorne, *The House of the Seven Gables* (1851)

---

# Why Vox?

Most full-stack projects are really three projects stitched together: a database, a backend, and a frontend, each with its own tooling, type definitions, and deployment steps. Most bugs live at the seams between them.

Vox is a single, AI-native language built on **7 Core Tenets**:
1. **Uniformity:** One language for the entire stack (Frontend + Backend + Infrastructure).
2. **Durability:** Execution survives failures. Workflows and Actors are first-class primitives.
3. **Distribution:** Location transparency is the default via the Mens.
4. **AI-Native:** Designed to be easily generated and reasoned about by LLMs.
5. **Performance:** Compiles to highly optimized Rust and TypeScript (React).
6. **ZERO Null States:** `null` is permanently banned. All absent states use `Option[T]`.
7. **Time Awareness:** Agents and workflows explicitly track elapsed time.

The output is always one binary:

```bash
vox bundle app.vox --release --target x86_64-unknown-linux-musl
```

That binary contains your backend (Rust/Axum), your frontend (React/TypeScript), and an embedded database (SQLite/Turso). Deploy it anywhere that runs a Linux binary. No runtime, no Docker required, no `npm install` on the server.

---

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

---

## The CLI

The `vox` binary is the entrypoint for compile, run, package, and diagnostics.

Start with this first-time flow:

```text
vox commands --recommended   List the most important starter commands
vox doctor                   Why: verify toolchain and env before coding
vox build <file>             Why: compile and inspect generated output
vox check <file>             Why: fast type validation without full build
vox run <file>               Why: execute app locally end-to-end
vox bundle <file>            Why: produce deployable binary output
```

For full command coverage (including feature-gated surfaces), use:

```text
vox commands --format json --include-nested
vox --help
```

Canonical command reference: [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

---

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


---

## Documentation & Resources

Want to dig deeper? We maintain a strictly standardized set of docs:

1. **[Frequently Asked Questions (FAQ)](docs/src/explanation/faq.md)** — Start here for deep answers on architecture, scaling, null safety, and AI integration.
2. **[Architecture Single Source of Truth](AGENTS.md)** — The definitive guide to the Vox compiler pipeline, repository rules, and core tenets.
3. **[First Full-Stack App](docs/src/how-to/first-full-stack-app.md)** — Step-by-step tutorial.
4. **[CLI Reference](docs/src/reference/cli.md)** — All terminal commands and flags.
5. **[Syntax Reference](examples/STYLE.md)** — The 0.8.0 syntax standard.

---

## License

Apache-2.0. Source: [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox).

Sponsorship links (GitHub Sponsors, Open Collective) will appear here when live.
