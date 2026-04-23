import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

test("golden route screenshot and accessibility snapshot", async ({ page }) => {
  const outDir = process.env.VOX_PLAYWRIGHT_OUT_DIR;
  if (!outDir) {
    throw new Error("VOX_PLAYWRIGHT_OUT_DIR must be set by the Rust harness");
  }
  fs.mkdirSync(outDir, { recursive: true });

  await page.goto("/");
  await expect(page.locator("body")).toBeVisible();
  await page.screenshot({ path: path.join(outDir, "route.png"), fullPage: true });

  const snap = await page.accessibility.snapshot();
  fs.writeFileSync(path.join(outDir, "a11y.json"), JSON.stringify(snap, null, 2), "utf8");
});
