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
        // `build:web` runs `cargo run --release -p vox-cli`; a cold build (no
        // warm cargo/target cache yet for the current Cargo.lock, e.g. the
        // first run after a lockfile change) can take well over 10 minutes on
        // a GitHub-hosted 2-core runner (600_000ms wasn't enough, observed
        // 2026-09-27). The CI job now carries a real Swatinem/rust-cache
        // (see the workflow), and its own timeout-minutes is hard-capped at
        // 30 by policy, so this leaves headroom for the rest of the job
        // (checkout, deps, the actual Playwright run, artifact upload)
        // rather than eating the whole budget itself.
        timeout: 1_200_000,
      },
});
