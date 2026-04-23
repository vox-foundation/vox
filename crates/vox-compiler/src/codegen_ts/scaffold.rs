//! One-time user-owned scaffold files (never overwrite if present).

use std::path::Path;

/// Files relative to project root (`app/`, `vite.config.ts`, etc.).
pub type ScaffoldFile = (String, String);

#[must_use]
pub fn react_interop_scaffold_files(_project_name: &str) -> Vec<ScaffoldFile> {
    vec![
        (
            "app/main.tsx".to_string(),
            r#"import React from "react"
import ReactDOM from "react-dom/client"
import { App } from "./App"
import "./globals.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
"#
            .to_string(),
        ),
        (
            "app/App.tsx".to_string(),
            r#"// User-owned router adapter — customize for React Router, TanStack Router, or Next.js.
import { BrowserRouter, Route, Routes } from "react-router"
import { Suspense, type ReactNode } from "react"
import { voxRoutes, type VoxRoute } from "../dist/routes.manifest"

// When `routes.manifest.ts` exports `notFoundComponent`, `errorComponent`, or `globalPendingComponent`,
// import them from the same module and wire e.g. `<Route path="*" element={<notFoundComponent />} />`.

function renderRoutes(routes: VoxRoute[]): ReactNode {
  return routes.map((r) => (
    <Route
      key={r.path}
      path={r.path}
      index={r.index}
      element={
        <Suspense fallback={r.pendingComponent ? <r.pendingComponent /> : <span />}>
          <r.component />
        </Suspense>
      }
    >
      {r.children ? renderRoutes(r.children) : null}
    </Route>
  ))
}

export function App(): React.ReactElement {
  return (
    <BrowserRouter>
      <Routes>{renderRoutes(voxRoutes)}</Routes>
    </BrowserRouter>
  )
}
"#
            .to_string(),
        ),
        (
            "app/globals.css".to_string(),
            "@import \"tailwindcss\";\n".to_string(),
        ),
        (
            "app/components.json".to_string(),
            r#"{
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
"#
            .to_string(),
        ),
        (
            "vite.config.ts".to_string(),
            r#"import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"
import path from "path"

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "app") },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: process.env.VITE_API_URL ?? "http://127.0.0.1:4000",
        changeOrigin: true,
      },
    },
  },
})
"#
            .to_string(),
        ),
        (
            "tsconfig.json".to_string(),
            r#"{
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
"#
            .to_string(),
        ),
        (
            "package.json".to_string(),
            r#"{
  "name": "vox-app",
  "type": "module",
  "private": true,
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
    "@tailwindcss/vite": "^4.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "tailwindcss": "^4.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0"
  }
}
"#
            .to_string(),
        ),
    ]
}

/// Write scaffold files under `project_root` if missing.
pub fn write_scaffold_if_missing(project_root: &Path, project_name: &str) -> std::io::Result<()> {
    for (rel, content) in react_interop_scaffold_files(project_name) {
        let path = project_root.join(&rel);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
    }
    Ok(())
}
