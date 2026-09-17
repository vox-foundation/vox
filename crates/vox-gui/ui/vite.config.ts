import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const TAURI_DEV_PORT = 1420; // required by Tauri devUrl; do not change

export default defineConfig(({ command }) => ({
  // Production bundle is loaded by the Tauri webview from frontendDist.
  // Absolute `/assets/...` URLs 404 there and leave Axis (drive) blank.
  base: command === 'build' ? './' : '/',
  plugins: [
    react(),
    {
      name: 'tauri-strip-crossorigin',
      transformIndexHtml(html: string) {
        // WKWebView treats `crossorigin` as CORS; the asset protocol has no
        // ACAO headers, so the module never runs and Axis stays blank.
        return html.replace(/ crossorigin(="[^"]*")?/g, '');
      },
    },
  ],
  clearScreen: false,
  // esbuild 0.28+ cannot downlevel destructuring for legacy browser targets during dep prebundle.
  build: {
    target: 'es2022',
  },
  optimizeDeps: {
    esbuildOptions: {
      target: 'es2022',
    },
  },
  server: {
    port: TAURI_DEV_PORT,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
}))
