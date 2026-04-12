import { defineConfig, devices } from "@playwright/test";

const appDir = process.env.VOX_PLAYWRIGHT_APP_DIR;

export default defineConfig({
  testDir: "./playwright",
  timeout: 180_000,
  expect: { timeout: 30_000 },
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "http://127.0.0.1:4173",
    trace: "retain-on-failure",
  },
  webServer: appDir
    ? {
        command: "pnpm run preview -- --host 127.0.0.1 --port 4173 --strictPort",
        cwd: appDir,
        url: "http://127.0.0.1:4173/",
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
      }
    : undefined,
});
