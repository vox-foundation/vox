---
title: "Vox full-stack web UI (SSOT)"
category: architecture
last_updated: 2026-03-22
---

# Vox full-stack web UI — single source of truth

## Language boundary

- **`.vox` source** uses **only Vox syntax** (including Vox JSX-like UI). Do not embed TypeScript or JavaScript in `.vox` files.
- **TypeScript and React** appear only in **generated artifacts** (`dist/`, `app/src/generated/`), **pnpm scaffolds** under `crates/vox-cli` templates, and the optional repo-root **`islands/`** Vite app (ShadCN, v0 output).

## Shipped stack

| Layer | Role |
| ----- | ---- |
| `vox-codegen-ts` | `@component`, `@island` (meta), `routes:`, tables, activities → `.tsx` / `.ts` |
| `vox-codegen-rust` | `http`, server fns, actors → Axum + `rust_embed` of `public/` |
| Vite + React 19 | Main app under `dist/app` (scaffolded by `vox run` / `vox bundle`) |
| `@tanstack/react-router` | Client routing for `routes:` (see [ADR 010](../adr/010-tanstack-web-spine.md)) |
| Optional **`islands/`** | Second Vite bundle; copied to `target/generated/public/islands/` when present |
| **v0.dev** | `V0_API_KEY`; TSX normalized to **named** `export function Name` for `routes:` imports |

## Canonical Frontend

The **VS Code extension** (`vox-vscode/`) is the **Single Source of Truth** for the Vox user-facing frontend experience. It integrates chat, planning (MCP), language support (LSP), and real-time visualization.

- **Orchestrator Dashboard**: Standalone HTML/CSS visualization for agents and tasks has been relocated to **`tools/dashboard/`**.
- **Unified Grammar**: Vocabulary is synchronized via **`tree-sitter-vox/GRAMMAR_SSOT.md`**.
- **Retired**: Legacy `frontend/` (Next.js) and `packages/vox-ui/` have been removed.

## Not part of Vox

Vox does **not** ship HTML-fragment UIs or classless CSS microframeworks as first-class product paths. Use **React + Vite + Tailwind/ShadCN + TanStack Router** (→ TanStack Start per [ADR 010](../adr/010-tanstack-web-spine.md)) for all interactive web UI.

## External references (ecosystem)

- [TanStack Router + Vite](https://tanstack.com/router/latest/docs/installation/with-vite)
- [TanStack Start (React)](https://tanstack.com/start/latest/docs/framework/react/overview)

## Implementation touchpoints

- Templates: `crates/vox-cli/src/templates.rs` (`package.json`, Vite config, islands bootstrap).
- Frontend build: `crates/vox-cli/src/frontend.rs` (`build_islands_if_present`).
- v0: `crates/vox-cli/src/v0.rs`, `crates/vox-cli/src/v0_tsx_normalize.rs`.
- Hooks import scan: `crates/vox-codegen-ts/src/component.rs`.
- **`vox run` auto mode**: `crates/vox-cli/src/commands/run.rs` + `commands/runtime/run/run.rs` — default is an `@page` scan in the first 8 KiB; override with **`[web] run_mode`** in `Vox.toml` (`auto` \| `app` \| `script`) or env **`VOX_WEB_RUN_MODE`** (same values; parsed in `vox-config`).
- **TanStack Start scaffold (opt-in)**: `Vox.toml` **`[web] tanstack_start = true`** or **`VOX_WEB_TANSTACK_START=1`** — `crates/vox-cli/src/templates.rs` + `frontend.rs` emit Start file layout + `@tanstack/react-start` (see [vox-fullstack-artifacts-ssot.md](vox-fullstack-artifacts-ssot.md)).
- **`@island`**: lexer/parser → `Decl::Island` (`vox-ast`); `vox build` writes **`target/generated/public/ssg-shells/`** HTML shells via **`vox-ssg`** (from `routes:` / `@page`).

## Roadmap

- [TanStack web roadmap](tanstack-web-roadmap.md) — phases Router → Start, SSR, workspace merge.
- [TanStack web backlog](tanstack-web-backlog.md) — checkbox task decomposition.
- [ADR 010 — TanStack web spine](../adr/010-tanstack-web-spine.md) — decisions (topology, examples, v0, `vox-codegen-html` retirement).

## Examples (canonical `.vox` shape)

- [`examples/STYLE.md`](../../../examples/STYLE.md) — target formatting for golden examples (LLM + human).
- [`examples/PARSE_STATUS.md`](../../../examples/PARSE_STATUS.md) — golden vs optional strict parse (`VOX_EXAMPLES_STRICT_PARSE`).

## Related docs

- [vox-fullstack-artifacts-ssot.md](vox-fullstack-artifacts-ssot.md) — build outputs, Express `server.ts` opt-in, containers.
- [`docs/src/ref-cli.md`](../ref-cli.md) — CLI including `vox island` (feature `island`).
- [TanStack SSR with Axum](../how-to/tanstack-ssr-with-axum.md) — dev topology during SSR adoption.
- [Mesh SSOT](mesh-ssot.md) — worker/runtime mesh registry and HTTP control plane; not emitted by `vox-codegen-*` (operator env only).
- [`AGENTS.md`](../../../AGENTS.md) — architecture index.
