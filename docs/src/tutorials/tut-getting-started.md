---
title: "Getting Started with Vox"
description: "Install Vox from source, scaffold a project with vox init, and understand every line of the starter src/main.vox."
category: "Tutorials"
status: "current"
sort_order: 1
schema_type: "HowTo"
keywords: ["Vox installation", "getting started Vox", "AI programming language tutorial", "Rust TypeScript compiler"]
---

# Getting Started with Vox

This guide installs Vox from source, scaffolds a project with `vox init`, and explains every line of the `src/main.vox` it generates. A first `cargo install` is a full workspace compile — not a five-minute download.

Canonical install detail: [Installing Vox](../reference/installation.md).

## Prerequisites

Before you begin, make sure you have:

- **Rust** (1.98.1) — [Install](https://rustup.rs/). This repo pins that channel in `rust-toolchain.toml`.
- **Node.js** and **pnpm** — needed for the generated frontend. The minimum versions are listed once, in [Installing Vox → Quick install](../reference/installation.md#quick-install-from-source).

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

This writes `Vox.toml` and a `src/main.vox` entrypoint.

## Step 3: Read the starter app

Open `src/main.vox`. `vox init` generates this file:

```vox
# My Vox App — a full-stack starter
#
# Run with: vox build src/main.vox -o dist && vox run src/main.vox

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

`table Note { ... }` is the single source of truth for this data: the same declaration becomes the SQL table and the Rust row type on the server, and the `Note` TypeScript interface on the client — there is no separate migration file or API type to keep in sync by hand.

`server add_note(...)` and `server list_notes()` run on the server. Each becomes a `POST /api/<name>` route plus a typed function of the same name in the generated `vox-client.ts`. In the starter they are stubs — they return fixed values and do not touch the `Note` table yet; that is the first thing you will change.

Both return `Result[str]`, so any caller is compiler-forced to handle the `Error` arm — there is no way to silently drop a failure.

`component App() { ... }` is the browser UI. `vox build` compiles it to a React/TSX component; `view:` describes what renders.

`routes { "/" to App }` mounts `App` at the site root.

## Step 4: Type check

Run a fast static analysis and type check:

```bash
vox check src/main.vox
```

## Step 5: Build

Compile the application to its backend Rust crate and frontend TypeScript:

```bash
vox build src/main.vox -o dist
```

The TypeScript (`App.tsx`, `vox-client.ts`, `schema.ts`, …) lands in `dist/`; the generated Rust server crate lands in `target/generated/`.

## Step 6: Run

```bash
vox run src/main.vox
```

Open `http://localhost:3000` in your browser. You'll see the `App` component's "My Vox App" heading; the same server also answers `/api/add_note` and `/api/list_notes`.

## What you just built

Four declarations, one file, no boilerplate glue:

| Declaration | What it does | What `vox build` emits |
|---|---|---|
| `table Note { ... }` | Defines a database table | SQL table + Rust row type + TypeScript `Note` interface |
| `server add_note(...)` | Server-side function | `POST /api/add_note` handler + typed `add_note` client call |
| `server list_notes()` | Server-side function | `POST /api/list_notes` handler + typed `list_notes` client call |
| `component App()` + `routes` | Browser UI mounted at `/` | React/TSX component + route table |

Two more declaration kinds you'll reach for as the app grows:

| Declaration | What it does |
|---|---|
| `query name(...) to T` / `mutation name(...) to T` | Read-only and write database operations, kept structurally distinct so callers can tell which are safe to retry |
| `tool "description" fn(...)` | Exposes a function to any Model Context Protocol client — the same function an HTTP caller uses, now callable by an agent |

Full grammar reference: [decorators and bare keywords](../reference/ref-decorators.md).

## What's next?

- **[First full-stack app](tut-first-app.md)** — a longer walkthrough of the same scaffold
- **[Golden Examples](../examples/golden.md)** — strictly verified, compiler-checked code snippets covering every language feature
- **[Language Reference](../reference/ref-syntax.md)** — full syntax reference
- **[Building Agents](../how-to/how-to-ai-agents.md)** — build MCP tools and agents with `tool`/`resource`
- **[Deployment Guide](../reference/deployment-compose.md)** — production rollout
