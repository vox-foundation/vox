import { defineConfig } from "@playwright/test";

const port = 5173;
const url = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 60_000,
  use: {
    baseURL: process.env.BASE_URL ?? url,
  },
  // When BASE_URL is provided externally (e.g. CI driving its own server),
  // skip auto-launch. Otherwise build the bundle and serve via vite preview.
  webServer: process.env.BASE_URL
    ? undefined
    : {
        command:
          "pnpm build:web && pnpm exec vite preview --port 5173 --host 127.0.0.1 --strictPort",
        url,
        reuseExistingServer: true,
        timeout: 180_000,
      },
});
