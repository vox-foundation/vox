//! Embedded templates for the repo-root **`islands/`** Vite app.

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
