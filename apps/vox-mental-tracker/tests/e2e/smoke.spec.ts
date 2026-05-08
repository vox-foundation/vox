import { test, expect } from "@playwright/test";

/**
 * Home page boots. The Playwright webServer block in playwright.config.ts
 * auto-launches `vite preview` against the bundled web-dist/ when BASE_URL
 * isn't externally provided, so this runs out of the box on `pnpm e2e`.
 */
test("home loads", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /Mental Health Tracker/i })).toBeVisible({
    timeout: 30_000,
  });
});
