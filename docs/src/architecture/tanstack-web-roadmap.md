---
title: "TanStack web roadmap (Router → Start)"
category: architecture
last_updated: 2026-03-21
---

# TanStack web roadmap

This document implements the execution narrative for [ADR 010: TanStack web spine](../adr/010-tanstack-web-spine.md). **Authoritative decisions** remain in the ADR; this file tracks **phases**, **dependencies**, and **open choices**.

## Phase ladder

| Phase | Goal | Status |
| ----- | ---- | ------ |
| 0 | SSOT + hygiene, `vox-codegen-html` retirement | Done |
| 1 | Minimal golden `examples/` + parser parity | Done |
| 2 | **TanStack Router** in `vox-codegen-ts` + templates | Done |
| 3 | **pnpm workspace** linking main Vite app + `islands/` | Mostly done (see backlog) |
| 4 | **TanStack Start** + full SSR default (Axum proxy topology) | In progress (dev proxy + orchestration) |
| 5 | Optional **TanStack Query / Table** aligned with `@loading` / data decls | Future |
| 6 | **v0.dev** unified docs + lint parity (main + islands) | Done (shared normalization) |

## SSR topology (summary)

**Default (ADR 010)**: Axum **reverse-proxies** document requests to a **Node** TanStack Start / SSR dev server; Axum keeps **API** routes and can still **`rust_embed`** `public/` for static chunks.

**Development**: two processes (`vox run` / compilerd for Rust + `pnpm` SSR dev) until a single orchestrator exists—see [how-to: TanStack SSR with Axum](../how-to/tanstack-ssr-with-axum.md).

## `vox-codegen-html` reconciliation

The name appears in historical docs and Ludus quests; **no crate** ships under `crates/vox-codegen-html` in this repository. **Canonical** HTML-ish output:

- **`vox-ssg`** — static shells under `target/generated/public/ssg-shells/`
- **React + Vite** — primary UI surface per [vox-web-stack-ssot.md](vox-web-stack-ssot.md)

## v0.dev (main + islands)

- **Same normalization**: `crates/vox-cli/src/v0_tsx_normalize.rs` for **named** exports used by Router imports.
- **Islands**: `islands/src/<Name>/<Name>.component.tsx`; **main app**: generated `*.tsx` next to `App.tsx`.
- **Env**: `V0_API_KEY` unchanged.

## Related links

- [TanStack web backlog](tanstack-web-backlog.md) (checkbox task decomposition)
- [vox-web-stack-ssot.md](vox-web-stack-ssot.md)
