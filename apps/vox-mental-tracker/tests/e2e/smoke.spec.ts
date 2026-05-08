import { test, expect } from "@playwright/test";

/**
 * Requires a running preview server (`vite` / static dist). Set `BASE_URL` when running locally or in CI.
 */
test("home loads", async ({ page }) => {
  test.skip(!process.env.BASE_URL, "Set BASE_URL to run browser smoke (e.g. http://127.0.0.1:5173)");
  await page.goto(process.env.BASE_URL!);
  await expect(page.getByRole("heading", { name: /Mental Health Tracker/i })).toBeVisible({
    timeout: 30_000,
  });
});
