---
title: "Getting Started with Vox"
description: "Install Vox from source, scaffold a project, and understand every line of a real .vox file."
category: "Tutorials"
sort_order: 1
schema_type: "HowTo"
keywords: ["Vox installation", "getting started Vox", "AI programming language tutorial", "Rust TypeScript compiler"]
---

# Getting Started with Vox

This guide installs Vox from source, scaffolds a notes app, and explains every line of the file that runs it. A first `cargo install` is a full workspace compile — not a five-minute download.

Canonical install detail: [Installing Vox](../reference/installation.md).

## Prerequisites

Before you begin, make sure you have:

- **Rust** (1.98.1) — [Install](https://rustup.rs/). This repo pins that channel in `rust-toolchain.toml`.
- **Node.js** (20+) — [Install](https://nodejs.org/)
- **pnpm** (9+) — `pnpm` is the repo package manager; `npm install -g pnpm` if you do not have it.

> **Tip**: After install, run `vox doctor` to check dependencies and environment variables.

## Step 1: Install Vox

> [!IMPORTANT]
> **Pre-1.0:** Vox has not reached version 1.0 (workspace 0.6.0). There are no published release installers yet. Build from source.

```bash
git clone https://github.com/vox-foundation/vox.git
cd vox
cargo install --locked --path crates/vox-cli
vox doctor
```

## Step 2: Create a new project

Use the Vox CLI to scaffold a new application:

```bash
vox init my-app
cd my-app
```

This scaffolds a complete project structure containing a `src/main.vox` entrypoint.

## Step 3: Write a notes app

Open `src/main.vox` and replace its contents with the following. This one file defines a database table, a browser UI component, a read query, and a write mutation — the whole stack.

```vox
table Note {
    title: str
    content: str
}

component App() {
    view: text() { "Hello Vox" }
}
```

`table Note { ... }` is the single source of truth for this data: the same declaration becomes the SQL schema, the wire format, and the typed client — there's no separate migration file or API type to keep in sync by hand.

`component App() { ... }` is the browser UI. `vox build` compiles it to a plain React/TSX component; `view:` describes what renders.

Add a query and a mutation so the app can actually read and write notes:

```vox
query get_notes() to int {
    // Returns note count; db.Note.all() returns a list
    return len(db.Note.all())
}

mutation create_note(title: str, content: str) to Result[str] {
    db.Note.insert({ title: title, content: content })?
    return Ok("created")
}
```

`query` and `mutation` are the two ways a client talks to your data — reads and writes, kept structurally distinct so an agent (or a reviewer) can tell which functions are safe to retry and which aren't. `create_note` returns `Result[str]`: the `?` after `db.Note.insert(...)` propagates a database error immediately, and any caller of `create_note` is compiler-forced to handle the `Error` arm — there's no way to silently drop it.

## Step 4: Type check

Run a fast static analysis and type check:

```bash
vox check src/main.vox
```

## Step 5: Build

Compile the application to its backend Rust crate and frontend TypeScript components:

```bash
vox build src/main.vox -o dist
```

You'll see step-by-step progress indicating lexical analysis and code generation.

## Step 6: Run

Run the generated binary directly:

```bash
vox run src/main.vox
```

Open `http://localhost:3000` in your browser. You'll see the `App` component's "Hello Vox" text — the same server that's now running also exposes `get_notes`/`create_note` as typed endpoints, callable from the generated `vox-client.ts` bridge.

## What you just built

Four declarations, one file, no boilerplate glue:

| Declaration | What it does | What `vox build` emits |
|---|---|---|
| `table Note { ... }` | Defines a database table | SQL schema + migration diff + typed client |
| `query get_notes()` | Read-only database operation | Optimized read endpoint |
| `mutation create_note(...)` | Write-enabled database operation | Insert/update endpoint with `Result` error handling |
| `component App()` | Browser UI | React/TSX component |

Two more declaration kinds you'll reach for as the app grows:

| Declaration | What it does |
|---|---|
| `server name(...) to T` | A server-side function that isn't a direct db read/write (e.g. calling an external API) |
| `tool "description" fn(...)` | Exposes a function to any Model Context Protocol client — the same function an HTTP caller uses, now callable by an agent |

Full grammar reference: [decorators and bare keywords](../reference/ref-decorators.md).

## What's next?

- **[First full-stack app](tut-first-app.md)** — a longer walkthrough that adds auth, deployment, and a second table
- **[Golden Examples](../examples/golden.md)** — strictly verified, compiler-checked code snippets covering every language feature
- **[Language Reference](../reference/ref-syntax.md)** — full syntax reference
- **[Building Agents](../how-to/how-to-ai-agents.md)** — build MCP tools and agents with `tool`/`resource`
- **[Deployment Guide](../reference/deployment-compose.md)** — production rollout
