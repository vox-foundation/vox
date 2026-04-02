---
title: "Vox full-stack build artifacts — single source of truth"
description: "Official documentation for Vox full-stack build artifacts — single source of truth for the Vox language."
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Vox full-stack build artifacts — single source of truth

This document names **every major output** of `vox build` / `vox run` / `vox bundle` and the **canonical runtime** for the default product path. It complements [vox-web-stack.md](vox-web-stack.md) and [ADR 010 — TanStack web spine](../adr/010-tanstack-web-spine.md).

## Canonical path (default)

| Layer | Artifact | Role |
| ----- | -------- | ---- |
| HTTP API | `target/generated/src/main.rs` (+ `lib.rs`, …) | **Axum** listens on `VOX_PORT` (default 3000). |
| Browser client for `@server fn` | `dist/api.ts` (or `out_dir/api.ts` from `-o`) | **`fetch` POST** to `/api/<name>`; `API_BASE` is `''`; Vite dev proxy forwards `/api` to Axum. |
| UI | `out_dir/*.tsx`, `out_dir/*.ts` | React components + TanStack Router `App.tsx` when `routes:` exists. |
| Static HTML shells | `target/generated/public/ssg-shells/**` | From [`vox-ssg`](../../../crates/vox-ssg/src/lib.rs): minimal shells for `routes:` / `@page` (hydration anchor, not a second UI runtime). |
| Embedded static (after frontend build) | `target/generated/public/**` | Vite `dist/` copied here for `rust_embed` in release flows. |

**`vox run`** (app mode): builds TS to `dist/`, runs **`cargo run` in `target/generated`** — the **Rust binary** is the primary server.

## Legacy / opt-in: Express `server.ts`

[`vox-codegen-ts`](../../../crates/vox-compiler/src/codegen_ts/routes.rs) can emit **`server.ts`**, an **Express** app that duplicates `@server` and `http` route registration.

- **Default:** emission is **off** unless **`VOX_EMIT_EXPRESS_SERVER=1`** is set in the environment when running codegen (e.g. `vox build`). The supported client for `@server fn` against Axum is **`api.ts`** from **Rust** codegen ([`emit_api_client`](../../../crates/vox-compiler/src/codegen_rust/emit/mod.rs)).
- **Use case for `VOX_EMIT_EXPRESS_SERVER=1`:** Node-only demos, tests, or containers that intentionally run `npx tsx server.ts` instead of the Rust binary.

## Container images

[`vox-container::generate_default_dockerfile`](../../../crates/vox-container/src/generate.rs) is **Rust-first**: **`FROM debian:bookworm-slim`**, **`COPY vox-app`**, **`CMD ["/app/vox-app"]`** (place the release binary from `vox bundle` / `cargo build --release` in `target/generated` into the build context as **`vox-app`**). **`@environment`** blocks and hand-authored Dockerfiles remain the place for a **Node + `npx tsx server.ts`** lane (requires **`VOX_EMIT_EXPRESS_SERVER=1`** at codegen). See [how-to-deploy.md](../how-to/how-to-deploy.md).

## Optional: islands and v0

- **`islands/`** — separate Vite app; built by `vox run` / bundle when `islands/package.json` exists ([`frontend.rs`](../../../crates/vox-cli/src/frontend.rs)).
- **`@v0`** — TSX on disk under `out_dir`; named `export function` required for `routes:` imports ([`v0_tsx_normalize.rs`](../../../crates/vox-cli/src/v0_tsx_normalize.rs)).

## Related

- [TanStack SSR with Axum](../how-to/tanstack-ssr-with-axum.md) — `VOX_SSR_DEV_URL`, `VOX_ORCHESTRATE_VITE`.
- [ref-cli.md](cli.md) — CLI surface.
