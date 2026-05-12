import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// vox build emits to ./dist (codegen output: routes manifest + per-page TSX).
// Vite serves from this dir as the project root, with src/main.tsx as the entry.
// Final web bundle goes to ./web-dist so the codegen output and the bundled
// site never collide; Vite `build.outDir` stays aligned with `vox compile` / Tauri `frontendDist`.
export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "web-dist",
    emptyOutDir: true,
  },
  server: {
    port: 5173,
    host: "127.0.0.1",
    strictPort: true,
  },
  preview: {
    port: 5173,
    host: "127.0.0.1",
    strictPort: true,
  },
});
