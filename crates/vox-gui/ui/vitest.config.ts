import { defineConfig } from 'vitest/config';

// Unit tests live under src/ (vitest). The e2e/ Playwright specs are run
// separately via `pnpm test:e2e` and must NOT be collected by vitest.
export default defineConfig({
  test: {
    // Project-level DOM env (t3code no-dom-test-environment) — do not set per-file.
    environment: 'jsdom',
    include: [
      'src/**/*.{test,spec}.{ts,tsx}',
      'e2e/lib/**/*.test.ts',
      'e2e/review/**/*.test.ts',
      'e2e/real-backend/**/*.test.ts',
    ],
    // Exclude the Playwright specs (e2e/*.spec.ts) but allow the pure unit-tested
    // e2e/lib helpers above to be collected by vitest.
    exclude: ['e2e/*.spec.ts', 'node_modules/**', 'dist/**'],
    setupFiles: ['src/test-setup.ts'],
  },
});
