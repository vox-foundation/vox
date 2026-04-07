---
title: "Compiler Module: vox-codegen-ts"
description: "Rust crate vox-codegen-ts: emits TypeScript clients, islands metadata, and web glue."
category: "api-crate"
status: current
last_updated: 2026-04-08
---

# Compiler Module: vox-codegen-ts

## Related architecture

- [ADR 012 — Internal web IR strategy](../adr/012-internal-web-ir-strategy.md)
- [Internal Web IR implementation blueprint](../architecture/internal-web-ir-implementation-blueprint.md)
- [Internal Web IR side-by-side schema](../architecture/internal-web-ir-side-by-side-schema.md)

Generated TypeScript for the web stack (`routes.manifest.ts`, `vox-client.ts`, islands, components) aligns with these documents and the React interop migration charter.

## `vox-client.ts` transport

- **Verbs:** `@query` uses **`GET`** with deterministic JSON-in-query encoding (sorted keys; each value is `JSON.stringify` + `encodeURIComponent`). `@mutation` and `@server` use **`POST`** + JSON body — same shapes as the generated Axum handlers.
- **Base URL:** `import.meta.env.VITE_API_URL` when set; otherwise relative paths.
- **Not generated:** TanStack `createServerFn`, Next.js `"use server"`, or other framework RPC shims — those strings are CI-forbidden in compiler output.

## TanStack Query (`vox-tanstack-query.tsx`)

- **Emitted helper:** `useVoxServerQuery(queryKey, queryFn)` wraps `@tanstack/react-query` `useQuery` with stable keys (see `crates/vox-compiler/src/codegen_ts/tanstack_query_emit.rs`).
- **`routes.manifest.ts` loaders:** `loader` properties are **plain async** functions that call imports from `./vox-client` — they **cannot** call React hooks. For cache sharing / deduplication, use `useVoxServerQuery` **inside route components** (or child components), e.g. `useVoxServerQuery(['list_posts'], () => list_posts({}))`.
- **Comment contract:** When any route has a loader, the manifest includes short comments pointing authors at `./vox-tanstack-query` (see `route_manifest.rs`).
