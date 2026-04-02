---
title: "TanStack web backlog"
description: "Official documentation for TanStack web backlog for the Vox language. Detailed technical reference, architecture guides, and implementati"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# TanStack web backlog

Decompose epics into actionable tasks. Check off as you complete; prefer **issues/PRs** for assignment, this file as **SSOT mirror**.

## Phase 0 — Hygiene

- [x] Narrative: non-product UI paths described in SSOT/ADR without legacy stack names
- [x] Remove or rewrite **`vox-codegen-html`** references (Cargo exclude comment, forward-migration charter, Ludus quests, CodeRabbit planner allowlist)
- [x] Link ADR 010 + this roadmap from **AGENTS.md** (optional one-liner)

## Phase 1 — Examples

- [x] Create `examples/archive/` and move non-golden `.vox` files
- [x] Update `crates/vox-parser/tests/parity_test.rs` `MUST_PARSE` (recursive walk)
- [x] Document golden list in `examples/README.md`
- [x] `examples/STYLE.md` + `FEATURE_INDEX.md` + `PARSE_STATUS.md`; optional **`VOX_EXAMPLES_STRICT_PARSE=1`** in `parity_test`

## Phase 2 — TanStack Router

- [x] Emit `createRootRoute` / `createRoute` / `createRouter` / `RouterProvider` from `routes:` ([`vox-codegen-ts/src/emitter.rs`](../../../crates/vox-compiler/src/codegen_ts/emitter.rs))
- [x] Add `@tanstack/react-router` to [`templates.rs`](../../../crates/vox-cli/src/templates/tanstack.rs) `package_json`; drop unused router dep from **`islands`** `package.json` template
- [x] Prefer **`App`** entry in [`fs_utils::find_component_name`](../../../crates/vox-cli/src/fs_utils.rs) when `App.tsx` exists
- [x] Integration tests: `routes:` codegen assertions ([`pipeline.rs`](../../../crates/vox-integration-tests/tests/pipeline.rs))

## Phase 3 — pnpm workspace

- [x] Emit root **`pnpm-workspace.yaml`** when `islands/` + main app paths are known ([`frontend.rs`](../../../crates/vox-cli/src/frontend.rs))
- [x] Document **root** `pnpm install` / `pnpm -r build` in [ref-cli.md](../reference/cli.md)
- [x] Align **islands** workspace paths: resolve **`islands/`** or **`packages/islands/`** (`island_package_root`, `pnpm-workspace.yaml`, `build_islands_if_present`)

## Phase 4 — TanStack Start + SSR

- [x] Scaffold Start-compatible **`vite.config`** / entry ([`templates.rs`](../../../crates/vox-cli/src/templates/tanstack.rs) `vite_config(..., tanstack_start: true)` + [`frontend.rs`](../../../crates/vox-cli/src/frontend.rs))
- [x] **`routes:` + Start**: single router ownership — codegen **`VoxTanStackRouter.tsx`** + `voxRouteTree`, **`routeTree.gen.ts`** re-export ([`emitter.rs`](../../../crates/vox-compiler/src/codegen_ts/emitter.rs) + `CodegenOptions.tanstack_start`)
- [x] Regenerate **file-route** `routeTree.gen.ts` via **TanStack Router CLI** (`pnpm run routes:gen` / `tsr generate`) for the no-`routes:` path — **`npm_install_and_build`** + **`dev`/`build`** scripts run it when not using programmatic `voxRouteTree`
- [x] **`vox run`**: optional Vite upstream via **`VOX_ORCHESTRATE_VITE=1`** + **`VOX_SSR_DEV_URL`** (see how-to)
- [x] Generated Axum **`serve_dispatch`**: GET non-`/api` proxy to **`VOX_SSR_DEV_URL`** when set
- [x] Production **Docker** sketch — see [TanStack SSR with Axum](../how-to/tanstack-ssr-with-axum.md#production-docker-sketch) (multi-stage Node build + Rust binary; adjust paths to your crate/binary name)
- [x] **CI**: `pnpm install` + `vite build` on **`web-vite-build-smoke`** (`ubuntu-latest` exception) with **`examples/full_stack_minimal.vox`** (opt-in local: `VOX_WEB_VITE_SMOKE=1`)

## Phase 5 — Query / Table (optional)

- [x] **`@loading`**: lexer/parser → `Decl::Loading` → `Spinner.tsx` + TanStack Router **`pendingComponent`** on programmatic `routes:` ([`tanstack_programmatic_routes.rs`](../../../crates/vox-compiler/src/codegen_ts/tanstack_programmatic_routes.rs))
- [ ] Map **`@query` / server-fn data** to **`useVoxServerQuery`** call sites in generated components (authors can wrap manually today; see [`tanstack_query_emit.rs`](../../../crates/vox-compiler/src/codegen_ts/tanstack_query_emit.rs))
- [x] Table-heavy UIs: **TanStack Table** — prefer for sort/filter/column-heavy grids when staying in React; hand-rolled `<table>` or lightweight lists remain fine for simple cases (see [vox-web-stack.md](../reference/vox-web-stack.md#data-grids-tanstack-table))

## Phase 6 — v0

- [x] **`vox build`** validates each present `{Name}.tsx` for `@v0` against the **named export** contract; **`cargo test -p vox-cli v0_tsx_normalize`** covers matchers; optional **`vox doctor`** check when **`VOX_WEB_TS_OUT`** points at the TS output dir
- [x] Docs: [@v0](../api/decorators/v0.md) links **v0.dev**, **named exports**, **islands** / `vox island`, and **doctor** env
