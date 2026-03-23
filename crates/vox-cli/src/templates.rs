//! Embedded templates for scaffolding a complete web application.
//! These are baked into the compiler binary so no external files are needed.
//!
//! ## TanStack npm versions
//! [`TANSTACK_REACT_ROUTER_RANGE`] is shared by the SPA and TanStack Start scaffolds. For the
//! **file-route** Start path (no `routes:`), run **`pnpm run routes:gen`** after changing
//! `src/routes/**` to refresh `routeTree.gen.ts` via `tsr` from **`@tanstack/router-cli`**.

/// Shared `@tanstack/react-router` semver range (SPA + Start `package_json`).
pub const TANSTACK_REACT_ROUTER_RANGE: &str = "^1.120.0";
/// `@tanstack/react-start` semver range (TanStack Start scaffold only).
pub const TANSTACK_REACT_START_RANGE: &str = "^1.120.0";
/// `@tanstack/router-cli` semver range (`tsr generate` / `pnpm run routes:gen`).
pub const TANSTACK_ROUTER_CLI_RANGE: &str = "^1.120.0";

/// Default `index.html` for a Vite + React scaffold.
pub fn index_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Vox App</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
</head>
<body>
  <div id="root"></div>
  <script type="module" src="/src/main.tsx"></script>
</body>
</html>
"#
}

/// `main.tsx` entry that mounts the generated React component.
pub fn main_tsx(component_name: &str) -> String {
    format!(
        r#"import React from "react";
import ReactDOM from "react-dom/client";
import {{ {component_name} }} from "./generated/{component_name}";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <{component_name} />
  </React.StrictMode>
);
"#
    )
}

/// Base stylesheet for the generated dark-theme app shell.
pub fn index_css() -> &'static str {
    r#"/* Vox Generated App — Dark Theme Design System */
:root {
  --bg-primary: #0f1117;
  --bg-secondary: #1a1d27;
  --bg-tertiary: #252836;
  --bg-accent: #2d3142;
  --text-primary: #e8eaf0;
  --text-secondary: #9ca3b4;
  --text-muted: #6b7280;
  --accent: #6366f1;
  --accent-hover: #818cf8;
  --accent-glow: rgba(99, 102, 241, 0.25);
  --success: #34d399;
  --error: #f87171;
  --border: #2e3244;
  --border-focus: #6366f1;
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 16px;
  --shadow-sm: 0 1px 3px rgba(0,0,0,0.3);
  --shadow-md: 0 4px 12px rgba(0,0,0,0.4);
  --shadow-lg: 0 8px 32px rgba(0,0,0,0.5);
  --font: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --transition: 200ms cubic-bezier(0.4, 0, 0.2, 1);
}

*, *::before, *::after {
  margin: 0; padding: 0; box-sizing: border-box;
}

html, body, #root {
  height: 100%; width: 100%;
  font-family: var(--font);
  background: var(--bg-primary);
  color: var(--text-primary);
  -webkit-font-smoothing: antialiased;
}
"#
}

/// `package.json` with React, Vite, and TypeScript dev dependencies.
///
/// When `tanstack_start` is true, adds **`@tanstack/react-start`** and uses `vite dev` / `vite build`
/// ([TanStack Start](https://tanstack.com/start/latest/docs/framework/react/build-from-scratch)).
pub fn package_json(tanstack_start: bool) -> String {
    if tanstack_start {
        return format!(
            r#"{{
  "name": "vox-generated-app",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "dev:ssr-upstream": "vite --port 3001 --strictPort",
    "routes:gen": "tsr generate"
  }},
  "dependencies": {{
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "@tanstack/react-router": "{tr}",
    "@tanstack/react-start": "{ts}"
  }},
  "devDependencies": {{
    "@types/node": "^22.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "@tanstack/router-cli": "{tcli}",
    "typescript": "^5.7.0",
    "vite": "^6.0.0"
  }}
}}
"#,
            tr = TANSTACK_REACT_ROUTER_RANGE,
            ts = TANSTACK_REACT_START_RANGE,
            tcli = TANSTACK_ROUTER_CLI_RANGE
        );
    }
    format!(
        r#"{{
  "name": "vox-generated-app",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "dev:ssr-upstream": "vite --port 3001 --strictPort"
  }},
  "dependencies": {{
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "@tanstack/react-router": "{tr}"
  }},
  "devDependencies": {{
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0"
  }}
}}
"#,
        tr = TANSTACK_REACT_ROUTER_RANGE
    )
}

/// Vite config with `/api` proxy to the given backend port.
pub fn vite_config(backend_port: u16, tanstack_start: bool) -> String {
    if tanstack_start {
        return format!(
            r#"import {{ defineConfig }} from "vite";
import {{ tanstackStart }} from "@tanstack/react-start/plugin/vite";
import react from "@vitejs/plugin-react";

export default defineConfig({{
  plugins: [
    tanstackStart(),
    react(),
  ],
  server: {{
    proxy: {{
      "/api": {{
        target: "http://127.0.0.1:{backend_port}",
        changeOrigin: true,
      }},
    }},
  }},
  build: {{
    outDir: "dist",
  }},
}});
"#
        );
    }
    format!(
        r#"import {{ defineConfig }} from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({{
  plugins: [react()],
  server: {{
    proxy: {{
      "/api": {{
        target: "http://127.0.0.1:{backend_port}",
        changeOrigin: true,
      }},
    }},
  }},
  build: {{
    outDir: "dist",
  }},
}});
"#
    )
}

/// TanStack Start root route (`src/routes/__root.tsx`).
pub fn tanstack_start_root_tsx() -> &'static str {
    r#"/// <reference types="vite/client" />
import "../index.css";
import type { ReactNode } from "react";
import {
  HeadContent,
  Outlet,
  Scripts,
  createRootRoute,
} from "@tanstack/react-router";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "Vox App" },
    ],
  }),
  component: RootComponent,
});

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  );
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  );
}
"#
}

/// Index file route when **`App.tsx`** exists but there is **no** [`VoxTanStackRouter.tsx`].
///
/// Do not use this when `App.tsx` is the SPA [`RouterProvider`] bundle from `routes:` codegen — that
/// layout uses [`tanstack_start_route_tree_gen_reexport`] + programmatic `voxRouteTree` instead.
pub fn tanstack_start_index_for_app() -> &'static str {
    r#"import { createFileRoute } from "@tanstack/react-router";
import App from "../generated/App";

export const Route = createFileRoute("/")({
  component: App,
});
"#
}

/// Index file route when there is no `App.tsx` — mount the primary `@component` export.
pub fn tanstack_start_index_for_component(component_name: &str) -> String {
    format!(
        r#"import {{ createFileRoute }} from "@tanstack/react-router";
import {{ {component_name} }} from "../generated/{component_name}";

export const Route = createFileRoute("/")({{
  component: {component_name},
}});
"#
    )
}

/// `src/router.tsx` for TanStack Start (imports generated `routeTree.gen.ts`).
pub fn tanstack_start_router_tsx() -> &'static str {
    r#"import { createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";

export function getRouter() {
  return createRouter({
    routeTree,
    scrollRestoration: true,
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
"#
}

/// Minimal `routeTree.gen.ts` for a single `/` file route (mirrors TanStack Start counter example layout).
pub fn tanstack_start_route_tree_gen() -> &'static str {
    r#"/* eslint-disable */
// @ts-nocheck
// This file was automatically generated by TanStack Router.
// You should NOT make any changes in this file as it will be overwritten.
// After editing `src/routes/**`, run `pnpm run routes:gen` (`tsr` from `@tanstack/router-cli`) to regenerate.

import { Route as rootRouteImport } from "./routes/__root";
import { Route as IndexRouteImport } from "./routes/index";

const IndexRoute = IndexRouteImport.update({
  id: "/",
  path: "/",
  getParentRoute: () => rootRouteImport,
} as any);

export interface FileRoutesByFullPath {
  "/": typeof IndexRoute;
}
export interface FileRoutesByTo {
  "/": typeof IndexRoute;
}
export interface FileRoutesById {
  __root__: typeof rootRouteImport;
  "/": typeof IndexRoute;
}
export interface FileRouteTypes {
  fileRoutesByFullPath: FileRoutesByFullPath;
  fullPaths: "/";
  fileRoutesByTo: FileRoutesByTo;
  to: "/";
  id: "__root__" | "/";
  fileRoutesById: FileRoutesById;
}
export interface RootRouteChildren {
  IndexRoute: typeof IndexRoute;
}

declare module "@tanstack/react-router" {
  interface FileRoutesByPath {
    "/": {
      id: "/";
      path: "/";
      fullPath: "/";
      preLoaderRoute: typeof IndexRouteImport;
      parentRoute: typeof rootRouteImport;
    };
  }
}

const rootRouteChildren: RootRouteChildren = {
  IndexRoute: IndexRoute,
};
export const routeTree = rootRouteImport
  ._addFileChildren(rootRouteChildren)
  ._addFileTypes();

import type { getRouter } from "./router.tsx";

declare module "@tanstack/react-start" {
  interface Register {
    ssr: true;
    router: Awaited<ReturnType<typeof getRouter>>;
  }
}
"#
}

/// `src/routeTree.gen.ts` when `routes:` codegen produced [`VoxTanStackRouter.tsx`] (programmatic tree).
pub fn tanstack_start_route_tree_gen_reexport() -> &'static str {
    r#"/* eslint-disable */
// @ts-nocheck
// Re-exports the programmatic route tree from `vox-codegen-ts` for TanStack Start (`getRouter` in router.tsx).

export { voxRouteTree as routeTree } from "./generated/VoxTanStackRouter";

import type { getRouter } from "./router";

declare module "@tanstack/react-start" {
  interface Register {
    ssr: true;
    router: ReturnType<typeof getRouter>;
  }
}
"#
}

/// Strict `tsconfig.json` suitable for Vite + React.
pub fn tsconfig_json() -> &'static str {
    r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": false,
    "noUnusedParameters": false,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
"#
}

/// Minimal `package.json` for repo-root **`islands/`** (`vox island`); uses **pnpm**.
pub fn islands_package_json() -> &'static str {
    r#"{
  "name": "vox-islands",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0"
  }
}
"#
}

/// Vite config for **`islands/`** (no API proxy; bundles discovered island modules + `island-mount` entry).
pub fn islands_vite_config() -> &'static str {
    r#"import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: "./index.html",
        "island-mount": "./src/island-mount.tsx",
      },
      output: {
        entryFileNames: (chunk) =>
          chunk.name === "island-mount" ? "island-mount.js" : "assets/[name]-[hash].js",
        chunkFileNames: "assets/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
  },
});
"#
}

/// Hydrates `[data-vox-island="Name"]` nodes from built island modules (also built as **`island-mount.js`**).
pub fn islands_island_mount_tsx() -> &'static str {
    r#"import { StrictMode, type ComponentType } from "react";
import { createRoot } from "react-dom/client";

type IslandModule = Record<string, ComponentType<Record<string, unknown>> | undefined> & {
  default?: ComponentType<Record<string, unknown>>;
};

const eagerModules = import.meta.glob<IslandModule>("./**/*.{tsx,ts}", { eager: true });

function islandComponentName(path: string): string | null {
  const nested = path.match(/\/([^/]+)\/\1\.component\.tsx$/);
  if (nested) {
    return nested[1] ?? null;
  }
  const flat = path.match(/\/([^/]+)\.tsx$/);
  return flat ? (flat[1] ?? null) : null;
}

const registry = new Map<string, ComponentType<Record<string, unknown>>>();
for (const [path, mod] of Object.entries(eagerModules)) {
  const name = islandComponentName(path);
  if (!name) {
    continue;
  }
  const Cmp = mod.default ?? mod[name];
  if (typeof Cmp === "function") {
    registry.set(name, Cmp);
  }
}

function propsFromElement(el: Element): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const attr of el.attributes) {
    if (attr.name === "data-vox-island") {
      continue;
    }
    if (attr.name.startsWith("data-prop-")) {
      const key = attr.name
        .slice("data-prop-".length)
        .replace(/-([a-z])/g, (_: string, c: string) => c.toUpperCase());
      out[key] = attr.value;
    }
  }
  return out;
}

function mountAll(): void {
  document.querySelectorAll("[data-vox-island]").forEach((node) => {
    if (!(node instanceof HTMLElement)) {
      return;
    }
    const name = node.getAttribute("data-vox-island");
    if (!name) {
      return;
    }
    const Cmp = registry.get(name);
    if (!Cmp) {
      console.warn(`[vox-islands] unknown island: ${name}`);
      return;
    }
    const props = propsFromElement(node);
    const root = createRoot(node);
    root.render(
      <StrictMode>
        <Cmp {...props} />
      </StrictMode>
    );
  });
}

mountAll();
"#
}

/// Root `index.html` for **`islands/`** Vite app.
pub fn islands_index_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Vox Islands</title>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="/src/main.tsx"></script>
</body>
</html>
"#
}

/// Entry that eagerly pulls all island TSX so `vite build` includes generated components.
pub fn islands_main_tsx() -> &'static str {
    r#"import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import.meta.glob("./**/*.{tsx,ts}", { eager: true });

function Host() {
  return <div data-vox-islands-host style={{ display: "none" }} aria-hidden="true" />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Host />
  </StrictMode>
);
"#
}
