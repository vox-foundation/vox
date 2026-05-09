---
title: "Vox full-stack build artifacts — single source of truth"
description: "Official documentation for Vox full-stack build artifacts — single source of truth for the Vox language."
category: "reference"
last_updated: "2026-04-07"
training_eligible: true

schema_type: "TechArticle"
---

# Vox full-stack build artifacts — single source of truth

This document names **every major output** of `vox build` / `vox run` / `vox bundle` and the **canonical runtime** for the default product path. It complements [vox-web-stack.md](vox-web-stack.md) and [ADR 010 — TanStack web spine](../adr/010-tanstack-web-spine.md).

## Canonical path (default)

| Layer | Artifact | Role |
| ----- | -------- | ---- |
| HTTP API | `target/generated/src/main.rs` (+ `lib.rs`, …) | **Axum** listens on `VOX_PORT` (default 3000). |
| Browser client for `@endpoint(kind: server) fn` | `dist/api.ts` (or `out_dir/api.ts` from `-o`) | **`fetch` POST** to `/api/<name>`; `API_BASE` is `''`; Vite dev proxy forwards `/api` to Axum. |
| Typed web client (`vox-client.ts`) | `out_dir/vox-client.ts` (with `@endpoint(kind: query\|mutation\|server)`) | **`GET`** + JSON query args for `query`; **`POST`** + JSON body for `mutation` / `server` (matches Axum). |
| Route manifest | `out_dir/routes.manifest.ts` | `voxRoutes` tree for SPA/Start adapters (`routes {` present). |
| UI | `out_dir/*.tsx`, `out_dir/*.ts` | React components + router shell; SPA scaffold uses manifest when present. |
| Static HTML shells | `target/generated/public/ssg-shells/**` | From [`vox-ssg`](../../../crates/vox-ssg/src/lib.rs): minimal shells for `routes {` / `@page` (hydration anchor, not a second UI runtime). |
| Embedded static (after frontend build) | `target/generated/public/**` | Vite `dist/` copied here for `rust_embed` in release flows. |

**`vox run`** (app mode): builds TS to `dist/`, runs **`cargo run` in `target/generated`** — the **Rust binary** is the primary server.

## Legacy / opt-in: Express `server.ts`

[`vox-codegen-ts`](../../../crates/vox-compiler/src/codegen_ts/routes.rs) can emit **`server.ts`**, an **Express** app that duplicates `@server` and `http` route registration.

- **Default:** emission is **off** unless **`VOX_EMIT_EXPRESS_SERVER=1`** is set in the environment when running codegen (e.g. `vox build`). The supported client for `@endpoint(kind: server) fn` against Axum is **`api.ts`** from **Rust** codegen ([`emit_api_client`](../../../crates/vox-compiler/src/codegen_rust/emit/mod.rs)).
- **Use case for `VOX_EMIT_EXPRESS_SERVER=1`:** Node-only demos, tests, or containers that intentionally run `npx tsx server.ts` instead of the Rust binary.

## Container images

[`vox-container::generate_default_dockerfile`](../../../crates/vox-container/src/generate.rs) is **Rust-first**: **`FROM debian:bookworm-slim`**, **`COPY vox-app`**, **`CMD ["/app/vox-app"]`** (place the release binary from `vox bundle` / `cargo build --release` in `target/generated` into the build context as **`vox-app`**). **`@environment`** blocks and hand-authored Dockerfiles remain the place for a **Node + `npx tsx server.ts`** lane (requires **`VOX_EMIT_EXPRESS_SERVER=1`** at codegen). See [how-to-deploy.md](../how-to/how-to-deploy.md).

## Axum JSON error envelope (API handlers)

- **`@mutation` with a schema (`@table` present):** the generated handler wraps the body in **`db.transaction(...)`** when applicable; a failed transaction maps to **`Json(serde_json::json!({"error": e.to_string()}))`**.
- **`@query`**, **`@server`**, and mutations **without** that transactional wrapper emit a straight-line handler body; they do **not** automatically wrap every failure in the same `{"error": ...}` object. Use application logic inside the handler (or Axum layers) if you need a uniform error shape for those paths.

## Optional: v0 and external frontends

- **`@v0`** — TSX on disk under `out_dir`; named `export function` required for `routes {` imports ([`v0_tsx_normalize.rs`](../../../crates/vox-cli/src/v0_tsx_normalize.rs)).
- **External React frontend** — see [architecture/external-frontend-interop-plan-2026](../architecture/external-frontend-interop-plan-2026.md). Vox emits plain React components from `component` declarations and a typed `vox-client.ts` from `@endpoint` declarations; an external React/TanStack/mobile app imports them directly. Islands were retired 2026-05-03.

## Related

- [TanStack SSR with Axum](../how-to/tanstack-ssr-with-axum.md) — `VOX_SSR_DEV_URL`, `VOX_ORCHESTRATE_VITE`.
- [ref-cli.md](cli.md) — CLI surface.


