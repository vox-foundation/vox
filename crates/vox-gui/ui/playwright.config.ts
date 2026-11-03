import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  globalSetup: './e2e/review/globalSetup.ts',
  // Only Playwright specs end in .spec.ts; e2e/lib/*.test.ts are vitest unit tests
  // (they import 'vitest') and must NOT be collected by the Playwright runner.
  testMatch: '**/*.spec.ts',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  // `open: 'never'` keeps the HTML artifact without blocking headless/CI runs on a report server.
  reporter: [['html', { open: 'never' }], ['line']],
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      // Review-capture only: the user evaluates in Firefox; Gecko layout
      // differs from Blink. The asserting sweep stays chromium-only.
      name: 'firefox-review',
      grep: /@review-capture/,
      use: { ...devices['Desktop Firefox'] },
    },
  ],
  webServer: {
    // Run vite directly, NOT via `pnpm run dev`: pnpm starts vite in its own process
    // group, so Playwright's group-kill leaves it orphaned holding the stdio pipes and
    // Playwright then waits on them forever (hung the CI ui leg at "Terminating the
    // WebServer"). Same command as the `dev` script (`vite`), minus the wrapper.
    command: 'node ./node_modules/vite/bin/vite.js',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
