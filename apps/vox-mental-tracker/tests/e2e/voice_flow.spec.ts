import { test, expect } from "@playwright/test";

/**
 * Voice-flow E2E (Phase 2 D1).
 *
 * Browser lane test: stubs the Speech.transcribe_microphone bridge before
 * the page boots so the in-page parser sees a deterministic transcript.
 * Native STT is exercised by the Capacitor build pipeline, not here.
 */
test("voice → parse → save round-trip", async ({ page }) => {
  test.skip(!process.env.BASE_URL, "Set BASE_URL to run browser smoke (e.g. http://127.0.0.1:5173)");

  await page.addInitScript(() => {
    // The codegen-emitted React component calls the Capacitor Speech plugin
    // through a generated wrapper; the test stub intercepts the wrapper so we
    // don't depend on a microphone or the platform plugin at this layer.
    (globalThis as unknown as Record<string, unknown>).__VOX_TEST_TRANSCRIPT__ =
      "I feel like a 4 today";
  });

  await page.goto(process.env.BASE_URL! + "/voice");

  await page.getByRole("button", { name: /Transcribe/i }).click();
  await expect(page.getByText(/RAW:/)).toContainText("I feel like a 4 today", {
    timeout: 5_000,
  });

  await page.getByRole("button", { name: /^Parse$/ }).click();
  await expect(page.getByText(/KIND:/)).toContainText("mood_recorded");
  await expect(page.getByText(/PAYLOAD:/)).toContainText("mood_score");

  await page.getByRole("button", { name: /^Save$/ }).click();
  await expect(page.getByText(/Last saved:/)).not.toContainText(/Last saved:\s*$/);
});
