---
title: "Vox Web: Minimal React Interop Implementation Plan"
description: "Complete implementation plan for Vox's minimal-surface, framework-agnostic React interop system. Supersedes the TanStack Start-specific codegen plan. Covers route manifest pattern, vox-client typed fetch SDK, v0/shadcn compatibility, decorator retirement, and full migration with 250+ tasks."
category: "architecture"
last_updated: 2026-04-07
training_eligible: true
---

# Vox Web: Minimal React Interop — Implementation Plan

> **Research foundation:** [`react-interop-research-findings-2026.md`](./react-interop-research-findings-2026.md)  
> **Supersedes:** [`tanstack-start-codegen-spec.md`](./tanstack-start-codegen-spec.md) (archived, not deleted)  
> **Backlog (250+ tasks):** [`react-interop-backlog-2026.md`](./react-interop-backlog-2026.md)

---

## Strategic Principle

**Vox is a component engine and API contract generator, not a framework bundler.**

Vox emits:
1. Pure named-export React functional components (stable forever)
2. A route manifest array (consumed by any router)
3. A typed `fetch` API client (consumed by any data layer)
4. Axum HTTP endpoint handlers (Rust, framework-free)
5. Typed TypeScript interfaces from Vox ADT declarations

Vox does NOT emit:
- Framework-specific file routing conventions (`__root.tsx`, `page.tsx`)
- Framework-specific RSC directives (`"use server"`, `"use client"`)
- Framework-specific server function calls (`createServerFn`)
- Routing configuration files (TanStack `routes.ts`, Next.js `app/` structure)

These belong in **user-owned scaffold files** that Vox generates once and never overwrites.

---

## Architecture Overview

```
Vox Source (.vox)
       │
       ▼ vox build
┌──────────────────────────────────────────────────────────────┐
│ dist/ (regenerated every build)                              │
│                                                              │
│   *.tsx              ← Named-export React components         │
│   routes.manifest.ts ← VoxRoute[] array (path, component,   │
│                         loader?, pendingComponent?)          │
│   vox-client.ts      ← Typed fetch SDK for @query/@mutation  │
│   types.ts           ← TypeScript interfaces from @table     │
│   vox-islands-meta.ts ← Island registry for hydration       │
└──────────────────────────────────────────────────────────────┘

app/ (scaffold — written once, never overwritten)
│   main.tsx            ← ReactDOM.createRoot entry point
│   App.tsx             ← Router adapter (user customizes this)
│   globals.css         ← Tailwind v4 import
│   components.json     ← shadcn/ui registry configuration
│   vite.config.ts      ← Vite config with /api proxy
│   package.json        ← React + react-router + lucide-react
│   tsconfig.json       ← jsx, paths, moduleResolution
└── islands/            ← @island TypeScript implementations
```

**Key design decision:** `App.tsx` is the **adapter**. It imports `voxRoutes` from `dist/routes.manifest.ts` and wires them into whatever router the user prefers. Vox ships a default using `react-router` library mode, which works everywhere.

---

## What Changes vs. The Old Plan

| Area | Old Plan (TanStack-specific) | New Plan (Framework-agnostic) |
|------|------------------------------|-------------------------------|
| Routes output | `__root.tsx` + `*.route.tsx` + `app/routes.ts` | Single `routes.manifest.ts` array |
| Server functions | `createServerFn({ method: "GET" })` | `fetch(`/api/query/${fn}`)` typed SDK |
| Scaffold router | TanStack-specific `app/router.tsx` + `app/client.tsx` + `app/ssr.tsx` | Standard `app/App.tsx` + `main.tsx` |
| Routing dep | `@tanstack/react-router` | `react-router` (library mode) |
| Maintenance risk | High (TanStack API changes frequently) | Very Low (fetch + plain React are stable) |
| v0 compatibility | Requires TanStack cognizance | Perfect: v0 emits named-export React |
| SSR | Requires TanStack Start + Nitro | Optional: user chooses (Next.js, RR7 framework, none) |

---

## Decorator Fate Table (Final)

| Decorator | Status | New Behavior |
|-----------|--------|--------------|
| `component Name() { view: ... }` | **KEEP — canonical** | Emits named-export `.tsx` |
| `@component fn` (classic) | **RETIRE → hard Error** | Migration: `component Name() { }` |
| `@island Name { prop: T }` | **KEEP — core** | Emits island registry entry |
| `@v0 Name` | **KEEP** | Emits island stub with v0 install comment |
| `routes { }` | **KEEP + SIMPLIFY** | Emits `routes.manifest.ts` VoxRoute[] |
| `loading: fn Name()` | **REPURPOSE** | Route manifest: `pendingComponent` field |
| `layout: fn Name()` | **REPURPOSE** | Route manifest: `children` grouping |
| `not_found: fn Name()` | **REPURPOSE** | Route manifest: registered in `App.tsx` scaffold |
| `error_boundary: fn Name()` | **REPURPOSE** | Route manifest: registered in `App.tsx` scaffold |
| `@query fn` | **KEEP + FIX** | `vox-client.ts`: typed `fetch` GET |
| `@mutation fn` | **KEEP + FIX** | `vox-client.ts`: typed `fetch` POST |
| `@server fn` | **KEEP + FIX** | `vox-client.ts`: typed `fetch` POST |
| `context: Name { }` | **RETIRE → hard Error** | No output. Migration: use React Context manually in App.tsx |
| `@hook fn` | **RETIRE → hard Error** | No output. Migration: use hooks in `@island` TypeScript files |
| `@provider fn` | **RETIRE → hard Error** | No output. Migration: add providers in scaffold `App.tsx` |
| `page: "path" { }` | **RETIRE → hard Error** | No output. Migration: use `routes { }` |

---

## New Codegen Output Specification

### 1. Component: `component Name() { }` → `Name.tsx`

**No change.** Path C emission is canonical. Named export, pure React TSX.

```tsx
export function PostList(): React.ReactElement {
  return <div className="posts">...</div>
}
```

---

### 2. Routes: `routes { }` → `routes.manifest.ts`

**Before (broken TanStack virtual files):**
```tsx
// __root.tsx  ← framework-specific, brittle
export const Route = createRootRoute({ ... })

// posts.route.tsx ← framework-specific
export const Route = createFileRoute("/posts")({ ... })
```

**After (stable manifest):**
```ts
// generated/routes.manifest.ts
import type { ComponentType } from "react"
import { Home } from "./Home"
import { PostList } from "./PostList"
import { PostDetail } from "./PostDetail"
import { Spinner } from "./Spinner"
import { NotFoundPage } from "./NotFoundPage"

export type VoxRoute = {
  path: string
  component: ComponentType<any>
  loader?: (ctx: { params: Record<string, string> }) => Promise<unknown>
  pendingComponent?: ComponentType
  errorComponent?: ComponentType<{ error: Error }>
  children?: VoxRoute[]
  index?: boolean
}

export const notFoundComponent = NotFoundPage
export const globalPendingComponent = Spinner

export const voxRoutes: VoxRoute[] = [
  {
    path: "/",
    component: Home,
    index: true,
  },
  {
    path: "/posts",
    component: PostList,
    loader: () => voxFetch("GET", "/api/query/getPosts"),
    pendingComponent: Spinner,
  },
  {
    path: "/posts/:id",
    component: PostDetail,
    loader: ({ params }) => voxFetch("GET", `/api/query/getPost?id=${params.id}`),
  },
]

// Internal fetch primitive — do not use directly; use vox-client.ts
function voxFetch(method: string, path: string, body?: unknown) {
  const base = import.meta.env.VITE_API_URL ?? "http://localhost:4000"
  return fetch(`${base}${path}`, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  }).then(r => { if (!r.ok) throw new Error(`${path} ${r.status}`); return r.json() })
}
```

---

### 3. Data: `@query` / `@mutation` → `vox-client.ts`

**Before (broken TanStack createServerFn):**
```ts
export const getPosts = createServerFn({ method: "POST" })
  .handler(async (data) => fetch("/api/...").then(r => r.json()))
```

**After (stable typed fetch client):**
```ts
// generated/vox-client.ts
// Generated by Vox. Regenerated on every vox build. Do not edit.
const BASE = import.meta.env.VITE_API_URL ?? "http://localhost:4000"

async function $get<T>(path: string): Promise<T> {
  const r = await fetch(`${BASE}${path}`)
  if (!r.ok) throw new Error(`GET ${path} failed: ${r.status}`)
  return r.json()
}

async function $post<T>(path: string, body: unknown): Promise<T> {
  const r = await fetch(`${BASE}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  })
  if (!r.ok) throw new Error(`POST ${path} failed: ${r.status}`)
  return r.json()
}

// @query fn getPosts() -> list[Post]
export async function getPosts(): Promise<Post[]> {
  return $get<Post[]>("/api/query/getPosts")
}

// @mutation fn createPost(title: str, body: str) -> Post
export async function createPost(data: { title: string; body: string }): Promise<Post> {
  return $post<Post>("/api/mutation/createPost", data)
}
```

---

### 4. Scaffold: New Files (written once, never overwritten)

#### `app/main.tsx`
```tsx
import React from "react"
import ReactDOM from "react-dom/client"
import { App } from "./App"
import "./globals.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><App /></React.StrictMode>
)
```

#### `app/App.tsx` — The Adapter
```tsx
// This file is yours to modify. Vox generated it once and will never overwrite it.
// To use a different router (TanStack Router, Next.js, etc.), replace the body of this file.
import { BrowserRouter, Routes, Route, Navigate } from "react-router"
import { Suspense } from "react"
import {
  voxRoutes,
  notFoundComponent: NotFound,
  globalPendingComponent: GlobalSpinner,
  type VoxRoute,
} from "../dist/routes.manifest"

function renderRoutes(routes: VoxRoute[]) {
  return routes.map(r => (
    <Route
      key={r.path}
      path={r.path}
      index={r.index}
      element={
        <Suspense fallback={r.pendingComponent ? <r.pendingComponent /> : <GlobalSpinner />}>
          <r.component />
        </Suspense>
      }
    >
      {r.children && renderRoutes(r.children)}
    </Route>
  ))
}

export function App() {
  return (
    <BrowserRouter>
      <Routes>
        {renderRoutes(voxRoutes)}
        <Route path="*" element={<NotFound />} />
      </Routes>
    </BrowserRouter>
  )
}
```

#### `app/globals.css`
```css
/* Tailwind v4 */
@import "tailwindcss";
```

#### `app/components.json`
```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "default",
  "rsc": false,
  "tailwind": {
    "config": "",
    "css": "app/globals.css",
    "baseColor": "slate",
    "cssVariables": true
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui"
  }
}
```

Note: `rsc: false` ensures v0.dev generates client-compatible components (no `"use server"`/`"use client"` directives). This is the critical v0 compatibility flag.

#### `vite.config.ts`
```ts
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import path from "path"

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./app") },
  },
  server: {
    port: 3000,
    proxy: {
      "/api": {
        target: process.env.VITE_API_URL ?? "http://localhost:4000",
        changeOrigin: true,
      },
    },
  },
})
```

#### `package.json`
```json
{
  "name": "vox-app",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "react-router": "^7.0.0",
    "lucide-react": "^0.400.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "tailwindcss": "^4.0.0",
    "@tailwindcss/vite": "^4.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0"
  }
}
```

#### `tsconfig.json`
```json
{
  "compilerOptions": {
    "jsx": "react-jsx",
    "moduleResolution": "Bundler",
    "module": "ESNext",
    "target": "ES2022",
    "skipLibCheck": true,
    "strictNullChecks": true,
    "paths": { "@/*": ["./app/*"] }
  },
  "include": ["app", "dist"]
}
```

---

## Vox Source Syntax: New Route Entry Forms

### Current (must still parse):
```vox
routes {
  "/" to Home
  "/posts" to PostList
}
```

### Extended (new, must parse after Wave A):
```vox
loading: fn GlobalSpinner() {
  view: <div class="spinner">Loading…</div>
}

not_found: fn NotFoundPage() {
  view: <div class="not-found">404</div>
}

routes {
  "/" to Home
  "/posts" to PostList with loader: getPosts
  "/posts/:id" to PostDetail with (loader: getPost, pending: Spinner)
  "/app" as layout AppShell {
    "/app/dashboard" to Dashboard
    "/app/settings" to Settings
  }
  not_found: NotFoundPage
  error: ErrorFallback
}
```

All of these map into the `VoxRoute[]` manifest — zero framework coupling.

---

## Execution Waves

### Wave 0 — AST/Parser Extensions
**Goal:** Support the new `routes { }` sub-syntax.

Tasks:
- `RouteEntry.loader: Option<String>` — name of a @query fn
- `RouteEntry.pending_component: Option<String>` — name of a loading: fn
- `RouteEntry.layout_name: Option<String>` — name of a layout group
- `RoutesDecl.not_found_component: Option<String>`
- `RoutesDecl.error_component: Option<String>`
- Parser: `with loader: fnName` clause after `to ComponentName`
- Parser: `with (loader: fnName, pending: SpinnerName)` variant
- Parser: `"/path" as layout Name { ... }` sub-block
- Parser: `not_found: ComponentName` terminal in routes body
- Parser: `error: ComponentName` terminal in routes body
- Parser: hard error on `@hook fn` — message + docs link
- Parser: hard error on `@provider fn` — message + docs link
- Parser: hard error on `page: "path" { }` — message + docs link
- Parser: deprecation warning on `context: Name { }` — message + docs link
- `cargo check` gate

### Wave 1 — HIR De-deprecation
**Goal:** Remove `#[deprecated]` from HIR fields that are canonical AppContract items.

Tasks:
- Remove `#[deprecated]` from `HirModule::client_routes`
- Remove `#[deprecated]` from `HirModule::islands`
- Remove `#[deprecated]` from `HirModule::loadings`
- Remove `#[deprecated]` from `HirModule::layouts`
- Remove `#[deprecated]` from `HirModule::not_founds`
- Remove `#[deprecated]` from `HirModule::error_boundaries`
- Change all 6 fields from `MigrationOnly` → `AppContract` in `field_ownership_map()`
- Add `layouts`, `loadings`, `not_founds`, `error_boundaries` to `SemanticHirModule`
- Remove `#[allow(deprecated)]` from `generate_with_options` for these 6 fields
- `cargo check` gate

### Wave 2 — Retire True Legacy Codegen
**Goal:** Remove the code paths that generate stale, broken output.

Tasks:
- Upgrade `@component fn` lint from Warning → Error in `typeck/ast_decl_lints.rs`
- Add hard Error lint for `Decl::Context`
- Add Error lint for `Decl::Hook` (belt+suspenders behind parser error)
- Add Error lint for `Decl::Page`
- Remove `hir.components` loop from `codegen_ts/emitter.rs`
- Remove `hir.v0_components` standalone loop (keep @v0 as island)
- Remove `hir.components` CSS loop from `emitter.rs`
- Remove `VoxTanStackRouter.tsx` emission path from `tanstack_programmatic_routes.rs`
- Remove `App.tsx` (SPA RouterProvider) emission path
- Keep `routeTree.gen.ts` re-export emission as a no-op / delete
- Remove `#[allow(deprecated)]` for `components`, `v0_components`, `pages` in `generate_with_options`
- Update `web_projection_cache` condition: use `reactive_components.is_empty() && loadings.is_empty()`
- `cargo check` gate + `cargo test` (many snapshot failures expected — update snapshots)

### Wave 3 — Route Manifest Emitter (New)
**Goal:** Replace the broken virtual file route emitter with the stable manifest emitter.

Tasks:
- Create `crates/vox-compiler/src/codegen_ts/route_manifest.rs` [NEW FILE]
- Add `pub fn emit_route_manifest(hir: &HirModule) -> String` 
- Emit `VoxRoute` TypeScript type definition at top of manifest
- Emit `notFoundComponent` export if `RoutesDecl.not_found_component` is set
- Emit `globalPendingComponent` export from module-level `loading:` fn if set
- Emit `voxRoutes: VoxRoute[]` array
- For each `RouteEntry`:
  - Emit `{ path, component }` minimum
  - If `loader`: emit `loader: (ctx) => voxFetch(...)` or `loader: () => voxFetch(...)` depending on whether path has `:params`
  - If `pending_component`: emit `pendingComponent: SpinnerName`
  - If `layout_name`: group children under parent `{ path: layoutPath, component: LayoutComp, children: [...] }`
- Emit `voxFetch` internal helper at bottom
- Import all referenced component names at top of manifest
- Emit `index: true` for root `/` route when path is `""` or `"/"`
- Register module in `codegen_ts/mod.rs`
- Wire into `emitter.rs::generate_with_options`: replace `push_route_tree_files` call with `push_route_manifest_file`
- `cargo check` gate

### Wave 4 — vox-client.ts Emitter (Fix)
**Goal:** Replace broken `createServerFn` emission with stable typed fetch emission.

Tasks:
- Add `fn emit_server_fn_client(hir: &HirModule) -> String` to `emitter.rs` or new file
- Emit `$get<T>` and `$post<T>` private helpers using `import.meta.env.VITE_API_URL`
- For each `@query` fn: emit `async function fnName(params): Promise<ReturnType>` that calls `$get`
- For each `@mutation` fn: emit `async function fnName(params): Promise<ReturnType>` that calls `$post`
- For each `@server` fn: emit same as mutation
- For `@query` fns with 0 params: URL is `/api/query/fnName` with no query string
- For `@query` fns with params: URL is `/api/query/fnName` + serialize params as query string
- For `@mutation` / `@server` with params: URL is `/api/mutation/fnName` or `/api/server/fnName`, body is JSON
- Remove old `serverFns.ts` emission (was using `createServerFn`)
- Output file is now `vox-client.ts` (rename from `serverFns.ts`)
- Update all tests that reference `serverFns.ts` → `vox-client.ts`
- Update `vox-tanstack-query.tsx` import from `serverFns` → `vox-client`
- `cargo check` + tests

### Wave 5 — Scaffold Emitter (New)
**Goal:** Generate one-time scaffold files that the user owns permanently.

Tasks:
- Create `crates/vox-compiler/src/codegen_ts/scaffold.rs` [NEW FILE]
- `fn emit_main_tsx() -> &'static str` — returns `app/main.tsx` content
- `fn emit_app_tsx(not_found: Option<&str>, error: Option<&str>, pending: Option<&str>) -> String` — returns `app/App.tsx` adapting `voxRoutes`
- `fn emit_globals_css() -> &'static str` — returns `app/globals.css` with Tailwind v4 `@import`
- `fn emit_components_json(project_name: &str) -> String` — returns `app/components.json` with `rsc: false`
- `fn emit_vite_config() -> &'static str` — returns `vite.config.ts` with proxy + `@` alias
- `fn emit_package_json(project_name: &str) -> String` — returns `package.json` (React 19, RR7, Tailwind v4)
- `fn emit_tsconfig() -> &'static str` — returns `tsconfig.json`
- `fn generate_scaffold_files(hir: &HirModule, project_name: &str) -> Vec<(String, String)>` — assembles all
- Register in `codegen_ts/mod.rs`
- Wire into `vox build --scaffold` CLI flag: loop over files, if file exists → skip, else write
- Wire into `vox init --web`: call scaffold + print instructions
- `cargo check` gate

### Wave 6 — CLI + Templates Update
**Goal:** Align templates and CLI entry points with new outputs.

Tasks:
- Remove `tanstack.rs` template references to `@tanstack/react-start`, `vinxi`, `createServerFn`
- Update `templates/package_json()` to emit React 19 + react-router + lucide-react deps
- Update `templates/vite_config()` to emit proxy-based config (not tanstackStart plugin)
- Update `templates/tsconfig()` to Tailwind v4 compatible
- Update `frontend.rs::find_component_name` or equivalent — entry point is now `app/main.tsx`, not `App.tsx`
- Update `npm_install_and_build` to not run `tsr generate` (no TanStack Router CLI needed)
- Update `build_islands_if_present` — island package.json does not need `react-router` dep
- Update `vox init --web` template vox file to use canonical Path C syntax
- Update `vox run` orchestration: in dev, start Vite on port 3000 + Axum on port 4000 (simplified from 4-process TanStack Start)
- `cargo check -p vox-cli` gate

### Wave 7 — Documentation Updates
**Goal:** Bring all docs into sync with the new model.

Tasks:
- Update `docs/src/architecture/tanstack-web-roadmap.md`: archive Phase 7 tanstack-start items, add Phase 8 "Minimal Interop Shell"
- Update `docs/src/architecture/tanstack-web-backlog.md`: reflect actual current state
- Create or update `docs/src/reference/ref-web-model.md`: rewrite route syntax, loader syntax, not_found, error, layout grouping
- Update `docs/src/reference/vox-web-stack.md`: reflect new outputs (routes.manifest.ts, vox-client.ts, scaffold App.tsx)
- Create `docs/src/how-to/tanstack-ssr-with-axum.md` addendum: note that TanStack Start is a user adapter choice, not Vox-generated
- Create `docs/src/how-to/v0-shadcn-vox.md`: end-to-end guide for using v0.dev, shadcn, and Vox together
- Update `docs/src/api/decorators/loading.md`: now maps to `pendingComponent` in route manifest
- Update `docs/src/api/decorators/layout.md`: now maps to layout group in route manifest
- Create/update `docs/src/api/decorators/not_found.md`: maps to `notFoundComponent` export
- Create/update `docs/src/api/decorators/error_boundary.md`: maps to `App.tsx` error boundary
- Update `docs/src/api/decorators/query.md`: now generates `vox-client.ts` GET
- Update `docs/src/api/decorators/mutation.md`: now generates `vox-client.ts` POST
- Update `docs/src/api/decorators/component.md` (classic `@component fn`): mark retired
- Create `docs/src/api/decorators/context.md`: mark retired with migration guide
- Create `docs/src/api/decorators/hook.md`: mark retired with migration guide
- Create `docs/src/api/decorators/provider.md`: mark retired with migration guide
- Update `docs/src/api/decorators/page.md` if exists: mark retired
- Update `docs/src/architecture/architecture-index.md`: link to react-interop-research-findings-2026.md
- Run mdbook build and fix any broken links

### Wave 8 — Golden Examples
**Goal:** Update examples to use canonical, new syntax.

Tasks:
- Create `examples/golden/blog_fullstack.vox`:
  - `@table Post { id: int, title: str, body: str }`
  - `@query fn getPosts() -> list[Post]`
  - `@mutation fn createPost(title: str, body: str) -> Post`
  - `loading: fn Spinner() { ... }`
  - `not_found: fn NotFoundPage() { ... }`
  - `component PostList() { ... }` using `voxClient.getPosts()` in a `useEffect`
  - `component PostForm() { ... }` calling `voxClient.createPost(...)`
  - `routes { "/posts" to PostList with loader: getPosts, "/posts/new" to PostForm, not_found: NotFoundPage }`
- Create `examples/golden/v0_shadcn_island.vox`:
  - Demonstrates `@island` + `@v0` declarations
  - Shows how v0-generated component is consumed
- Create `examples/golden/layout_groups.vox`:
  - Demonstrates `"/app" as layout AppShell { children }` route grouping
- Update `examples/golden/rest_api.vox` if it exists:
  - Replace any old `@component fn` or `serverFns` references
  - Use canonical `@query`/`@mutation` → vox-client.ts
- Create `examples/README.md` entries for all new golden files
- Run `cargo test -p vox-parser` parity tests (all golden must parse)
- Run `VOX_EXAMPLES_STRICT_PARSE=1 cargo test` (strict parse gate)

### Wave 9 — Tests
**Goal:** Add thorough test coverage for all new codegen.

Tasks:
- Add snapshot test: `routes_manifest_basic` — `routes { "/" to Home }` → `voxRoutes contains VoxRoute for /`
- Add snapshot test: `routes_manifest_with_loader` — `with loader: getPosts` → `loader:` in manifest
- Add snapshot test: `routes_manifest_with_pending` — `with pending: Spinner` → `pendingComponent: Spinner`
- Add snapshot test: `routes_manifest_not_found` — `not_found: NotFoundPage` → `notFoundComponent` export
- Add snapshot test: `routes_manifest_layout_group` — `"/app" as layout Shell { ... }` → nested children VoxRoute
- Add unit test: `vox_client_query_emits_get_fetch` — `@query fn` → `$get(...)` in vox-client.ts
- Add unit test: `vox_client_mutation_emits_post_fetch` — `@mutation fn` → `$post(...)` in vox-client.ts
- Add unit test: `vox_client_has_vite_api_url_constant` — manifest contains `import.meta.env.VITE_API_URL`
- Add unit test: `vox_client_no_createServerFn_reference` — assert `createServerFn` does not appear in output
- Add unit test: `scaffold_main_tsx_uses_createRoot` — scaffold `app/main.tsx` contains `ReactDOM.createRoot`
- Add unit test: `scaffold_components_json_rsc_false` — `components.json` has `"rsc": false`
- Add unit test: `scaffold_app_tsx_imports_voxRoutes` — `App.tsx` imports from `routes.manifest`
- Add unit test: `scaffold_vite_config_has_api_proxy` — `vite.config.ts` has `/api` proxy entry
- Add unit test: `classic_component_fn_emits_error_diagnostic` — `@component fn` → TypeckSeverity::Error
- Add integration test: `pipeline_blog_fullstack_produces_manifest_and_client` — compiles blog_fullstack.vox, asserts `routes.manifest.ts` + `vox-client.ts` both present
- Update `pipeline.rs` TanStack integration tests — remove assertions for `VoxTanStackRouter.tsx`/`__root.tsx`
- `cargo test -p vox-compiler -p vox-cli -p vox-integration-tests` full pass

---

## v0.dev / shadcn Compatibility Checklist

These are the hard requirements for Vox → v0 interop. All must be true:

- [ ] Scaffold `components.json` has `"rsc": false`
- [ ] Scaffold `vite.config.ts` has `@` alias pointing to `./app`
- [ ] Scaffold `tsconfig.json` has `"paths": { "@/*": ["./app/*"] }`
- [ ] Generated components use `className=` (not `class=`) — verify JSX emitter
- [ ] Generated components use named exports (`export function Name`)
- [ ] Generated route manifest does NOT use `"use server"` or `"use client"`
- [ ] Generated `vox-client.ts` does NOT use `createServerFn`
- [ ] `@island` TypeScript files land in `islands/src/` (unchanged from current)
- [ ] `@v0` stubs emit a comment: `// Install this island: npx shadcn@latest add [URL]`
- [ ] Tailwind v4 import is `@import "tailwindcss"` in `globals.css` (not old v3 directives)
- [ ] `lucide-react` is in scaffold `package.json` deps (v0 components import from it)

---

## Migration Guide for Existing .vox Files

### `@component fn` → `component Name() { }`

```vox
// BEFORE (error after migration)
@component fn MyButton(label: str) {
  view: <button>{{ label }}</button>
}

// AFTER (canonical Path C)
component MyButton(label: str) {
  view: <button>{{ label }}</button>
}
```

Run `vox migrate component src/` to auto-migrate.

### `context: AuthContext { user: User }` → Delete

Not emitted. Replace with React Context in `@island` TypeScript or pass via props.

### `@hook fn useCounter()` → Move to island TypeScript

```ts
// islands/src/Counter/Counter.tsx
import { useState } from "react"

function useCounter(initial: number) {
  const [count, setCount] = useState(initial)
  return { count, increment: () => setCount(c => c + 1) }
}

export function Counter({ initial }: { initial: number }) {
  const { count, increment } = useCounter(initial)
  return <button onClick={increment}>{count}</button>
}
```

### `@provider fn ThemeProvider()` → Move to scaffold App.tsx

```tsx
// app/App.tsx — add your providers here
import { ThemeProvider } from "./providers/theme"
...
export function App() {
  return (
    <ThemeProvider>
      <BrowserRouter>...</BrowserRouter>
    </ThemeProvider>
  )
}
```

---

## Done Criteria

- [ ] `cargo check -p vox-compiler -p vox-cli -p vox-integration-tests` — 0 errors
- [ ] `cargo test -p vox-compiler` — all pass (snapshots updated)
- [ ] `cargo test -p vox-integration-tests` — all pass
- [ ] `vox build --scaffold` on `examples/golden/blog_fullstack.vox`:
  - Produces `dist/routes.manifest.ts` with VoxRoute[] array
  - Produces `dist/vox-client.ts` with typed fetch functions
  - Produces `dist/types.ts` with Post interface
  - Produces `app/main.tsx`, `app/App.tsx`, `app/components.json`, `vite.config.ts`, `package.json`
  - Does NOT produce `VoxTanStackRouter.tsx`, `__root.tsx`, or `createServerFn` references
- [ ] `npx shadcn@latest add button` works in the generated project (validates `components.json`)
- [ ] `pnpm install && vite dev` starts without errors on generated output
- [ ] A v0-generated component (`LoginForm.tsx`) can be placed in `islands/` and imported without modification
