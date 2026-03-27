//! Embedded templates for the repo-root **`islands/`** Vite app.
//!
//! # Island mount V1 hydration contract
//!
//! The compiler emits DOM that matches this runtime:
//! - **`data-vox-island="<Name>"`** — key into the eager import registry (see
//!   `vox_compiler::codegen_ts::island_emit`).
//! - **`data-prop-<kebab>`** — serialized as HTML attributes; the mount script maps each to a
//!   **camelCase** React prop using the same kebab rule as `island_data_prop_attr` in the compiler.
//!
//! Unknown island names call **`console.warn`** and skip that node.
//!
//! **Decode helper (OP-S041):** [`islands_props_from_element_ts`] is the standalone SSOT for turning
//! `data-prop-foo-bar` attributes into React `fooBar` props (same bytes embedded in [`islands_island_mount_tsx`]).
//! The slice length must match the `"data-prop-"` prefix emitted by the compiler; the regex camelCase hop
//! must mirror `island_data_prop_attr`’s kebab rule on emit (underscores → hyphens).
//!
//! **V1 lock:** The `data-prop-*` → camelCase rule must stay aligned with
//! `vox_compiler::codegen_ts::island_emit::island_data_prop_attr` and `island_mount_format_version`.
//! Bump the compiler’s **`ISLAND_MOUNT_FORMAT_VERSION`** (and this template’s contract comment) together when the wire format changes.
//!
//! Embedded markers: **`vox:island-mount contract=V1`**, **`vox:island-metrics contract=V1`** (optional `globalThis` export for parity tooling).
//!
//! **Telemetry + hydration policy B/C (OP-S069 / S095 / S121 / S143 / S173 / S201):** changing metric globals
//! or `propsFromElement` affects `reactive_smoke`, `full_stack_minimal_build`, and `web_ir_lower_emit` gates.

use std::sync::OnceLock;

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

const ISLAND_MOUNT_PRE: &str = r#"import { StrictMode, type ComponentType } from "react";
import { createRoot } from "react-dom/client";

// vox:island-mount contract=V1 (kebab data-prop-* → camelCase props; keep in sync with vox-compiler island_emit)

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
}"#;

const ISLAND_MOUNT_PROPS_FROM_ELEMENT: &str = r#"function propsFromElement(el: Element): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const attr of el.attributes) {
    if (attr.name === "data-vox-island") {
      continue;
    }
    if (attr.name.startsWith("data-prop-")) {
      const key = attr.name
        .slice("data-prop-".length)
        .replace(/-([a-z])/g, (_: string, c: string) => c.toUpperCase());
      if (!key) {
        continue;
      }
      out[key] = attr.value;
    }
  }
  return out;
}"#;

const ISLAND_MOUNT_POST: &str = r#"const voxIslandsV1Metrics = {
  formatVersion: 1,
  unknownIslandWarnCount: 0,
};

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
      voxIslandsV1Metrics.unknownIslandWarnCount += 1;
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

// vox:island-metrics contract=V1 — optional global for test / dashboard parity (safe to elide in SSR-only bundles).
if (typeof globalThis !== "undefined") {
  Object.defineProperty(globalThis as object, "__VOX_ISLANDS_V1_METRICS", {
    value: voxIslandsV1Metrics,
    configurable: true,
    enumerable: true,
    writable: false,
  });
}
"#;

/// V1 **`propsFromElement`** implementation only (no imports). Same bytes as embedded in [`islands_island_mount_tsx`];
/// exposed for contract tests without scanning the full bundle.
pub fn islands_props_from_element_ts() -> &'static str {
    ISLAND_MOUNT_PROPS_FROM_ELEMENT
}

/// Hydrates `[data-vox-island="Name"]` nodes from built island modules (also built as **`island-mount.js`**).
pub fn islands_island_mount_tsx() -> &'static str {
    static FULL: OnceLock<&'static str> = OnceLock::new();
    FULL.get_or_init(|| {
        Box::leak(
            [
                ISLAND_MOUNT_PRE,
                "\n\n",
                islands_props_from_element_ts(),
                "\n\n",
                ISLAND_MOUNT_POST,
            ]
            .concat()
            .into_boxed_str(),
        )
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_mount_includes_props_helper_and_contract_marker() {
        let full = islands_island_mount_tsx();
        assert!(
            full.contains("vox:island-mount contract=V1"),
            "missing contract marker:\n{full}"
        );
        assert!(
            full.contains(islands_props_from_element_ts()),
            "full template should embed islands_props_from_element_ts verbatim"
        );
    }

    #[test]
    fn island_mount_warns_unknown_island() {
        let full = islands_island_mount_tsx();
        assert!(
            full.contains("[vox-islands] unknown island:"),
            "expected warn path for missing registry entry:\n{full}"
        );
        assert!(
            full.contains("console.warn"),
            "unknown island should use console.warn:\n{full}"
        );
    }

    #[test]
    fn island_mount_props_skip_empty_prop_key() {
        let p = islands_props_from_element_ts();
        assert!(
            p.contains("if (!key) {\n        continue;\n      }"),
            "malformed data-prop- (empty local name) should be ignored:\n{p}"
        );
    }

    #[test]
    fn island_mount_exports_v1_metrics_contract() {
        let full = islands_island_mount_tsx();
        assert!(
            full.contains("vox:island-metrics contract=V1"),
            "missing metrics marker:\n{full}"
        );
        assert!(
            full.contains("__VOX_ISLANDS_V1_METRICS") && full.contains("formatVersion: 1"),
            "expected V1 metrics surface:\n{full}"
        );
    }
}
