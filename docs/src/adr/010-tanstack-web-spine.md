---
title: "ADR 010 — TanStack web spine (Router → Start, SSR topology)"
category: adr
constructs: []
last_updated: 2026-03-21
training_eligible: false
difficulty: intermediate
---

# ADR 010 — TanStack as the Vox web spine

**Status**: Accepted  
**Date**: 2026-03-21

---

## Context

Vox compiles `.vox` UI to **React + Vite** (`vox-codegen-ts`), serves static assets via **Axum + `rust_embed`** (`vox-codegen-rust`), and optionally builds a second **islands** bundle. Prior routing used **`react-router-dom`** emitted from `routes:` declarations. The ecosystem direction is **TanStack Router** (typed, composable) and **TanStack Start** (Vite-native full-stack SSR, built on Router).

**Non-goals**: HTML-fragment UIs and classless CSS microframeworks as product paths; the supported graph is React + Tailwind/ShadCN + TanStack (see [vox-web-stack SSOT](../architecture/vox-web-stack-ssot.md)).

---

## Decision

1. **Routing spine**: Adopt **@tanstack/react-router** for codegen from `routes:` (replacing `react-router-dom`).
2. **Long-term framework**: Plan **TanStack Start** for default **SSR** after Router is stable in our scaffold; Start **includes** Router—there is no separate “merge” of incompatible TanStack products, only **composition** (optional TanStack Query / Table later).
3. **SSR production topology (default recommendation)**: **Option B** — **Axum reverse-proxies** HTML/document requests to a **Node-hosted TanStack Start / Vite SSR** server, while Axum remains the **API** and static asset origin for `/api` and embedded `public/`. Alternatives (A: API-only Axum + separate SSR host; C: hybrid static shells from `vox-ssg` + selective SSR) remain documented in the roadmap.
4. **Examples policy**: Maintain a **small golden set** (5–12) of `.vox` examples that CI/parser treat as canonical; move or archive the rest.
5. **v0.dev**: First-class for **both** the main generated app and **islands**; TSX must use **named** `export function Name` aligned with `routes:` / Router (normalization in `vox-cli`).
6. **`vox-codegen-html`**: **Retired** as a workspace crate name—there is no in-tree implementation; static HTML needs are served by **`vox-ssg`** plus the React stack (see reconciliation in roadmap).

---

## Consequences

- **Dependencies**: Generated app `package.json` carries `@tanstack/react-router` instead of `react-router-dom`.
- **Dev UX**: Until Start is wired, **`vox run`** remains **SPA + Axum**; SSR requires an additional process when enabled (documented in how-to).
- **Docs**: Roadmap and backlog live under [`docs/src/architecture/tanstack-web-roadmap.md`](../architecture/tanstack-web-roadmap.md) and [`tanstack-web-backlog.md`](../architecture/tanstack-web-backlog.md).

---

## References

- [TanStack Router — Vite](https://tanstack.com/router/latest/docs/installation/with-vite)
- [TanStack Start — React](https://tanstack.com/start/latest/docs/framework/react/overview)
- [vox-web-stack-ssot.md](../architecture/vox-web-stack-ssot.md)
- [vox-fullstack-artifacts-ssot.md](../architecture/vox-fullstack-artifacts-ssot.md) — canonical vs legacy artifacts (`server.ts`, `VOX_EMIT_EXPRESS_SERVER`, containers)
