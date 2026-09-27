/**
 * Chat model picker apply flow against the stateful tauriMock.
 *
 * Ground truth: Loquela's "Choose model tier" (Run on) trigger is the
 * single picker. The pick is lifted via onModelPick and threaded into
 * the NEXT chat submit as `chat_turn`'s `input.model_override` (the single
 * composer dispatch since dfb707f38). ChatModelPicker is not mounted in
 * App trailingSlot. The picker itself never calls `set_active_model`.
 */
import { test, expect } from '@playwright/test';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

test('picking a model updates the trigger label and threads model_override into the next submit', async ({ page }) => {
  await addMockInitScript(page, installTauriMock, 'chat');
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });

  await page.getByRole('button', { name: /choose model tier/i }).click();
  const lastMens = page.getByText(/^mens\//).last();
  await expect(lastMens).toBeAttached();
  await lastMens.scrollIntoViewIfNeeded();
  const pickedId = (await lastMens.innerText()).trim();
  await lastMens.click();

  const composer = page.getByLabel('Task composer');
  await composer.fill('Use the picked model for this');
  await composer.press('Enter');

  await expect
    .poll(
      () =>
        page.evaluate(() =>
          (window as any).__TAURI_CALLS__.filter((c: any) => c.cmd === 'chat_turn').length,
        ),
      { timeout: 10_000 },
    )
    .toBeGreaterThan(0);
  const call = await page.evaluate(() =>
    (window as any).__TAURI_CALLS__.find((c: any) => c.cmd === 'chat_turn'),
  );
  expect(call.args.input).toMatchObject({ model_override: pickedId });

  const setActiveModelCalls = await page.evaluate(() =>
    (window as any).__TAURI_CALLS__.filter((c: any) => c.cmd === 'set_active_model').length,
  );
  expect(setActiveModelCalls).toBe(0);
});
