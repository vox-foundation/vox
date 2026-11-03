import { Page, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from '../lib/tauriMock';
import { addMockInitScript } from '../lib/tauriMockShared';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'review-bundle', 'hitl');

export interface StagePayloadSnapshot {
  stageId: string;
  status: string;
  inputPayloadText: string;
  outputPayloadText: string;
}

export async function launchResearchSession(page: Page): Promise<void> {
  await addMockInitScript(page, installTauriMock, 'research');
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 20_000 });
  await page.evaluate(() => (document as any).fonts?.ready);
}

export async function toggleDiagnosticsProber(page: Page): Promise<void> {
  const toggleDiagnosticsBtn = page.getByTestId('toggle-diagnostics-btn');
  await expect(toggleDiagnosticsBtn).toBeVisible({ timeout: 10_000 });
  await toggleDiagnosticsBtn.click();
  const prober = page.getByTestId('live-source-prober');
  await expect(prober).toBeVisible();
}

export async function submitProbeQuery(page: Page, query: string): Promise<void> {
  const probeInput = page.getByTestId('probe-query-input');
  await expect(probeInput).toBeVisible();
  await probeInput.fill(query);

  const probeSubmitBtn = page.getByTestId('probe-submit-btn');
  await expect(probeSubmitBtn).toBeVisible();
  await probeSubmitBtn.click();

  const statusBadge = page.getByTestId('status-badge-200').first();
  await expect(statusBadge).toBeVisible({ timeout: 10_000 });
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
}

export async function openInspectorDrawer(page: Page): Promise<void> {
  const drawer = page.getByTestId('inspector-drawer');
  if (!(await drawer.isVisible())) {
    await page.keyboard.press('Control+Shift+D');
    await expect(drawer).toBeVisible({ timeout: 5_000 });
  }
}

export async function stepNextStage(page: Page): Promise<void> {
  const stepNextBtn = page.getByRole('button', { name: 'Step Next' });
  await expect(stepNextBtn).toBeVisible();
  await stepNextBtn.click();
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
}

export async function selectStageInDrawer(page: Page, stageId: string): Promise<void> {
  const drawer = page.getByTestId('inspector-drawer');
  await expect(drawer).toBeVisible();
  const stageBtn = drawer.getByRole('button', { name: new RegExp(`^${stageId}`) }).first();
  await expect(stageBtn).toBeVisible();
  await stageBtn.click();
}

export async function captureStageArtifact(
  page: Page,
  stageIndex: number,
  stageName: string
): Promise<string> {
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
  mkdirSync(OUT_DIR, { recursive: true });
  const filename = `stage-${stageIndex}-${stageName}.png`;
  const fullPath = join(OUT_DIR, filename);
  await page.screenshot({ path: fullPath });
  return fullPath;
}

export async function extractCurrentPayloads(page: Page): Promise<StagePayloadSnapshot> {
  const drawer = page.getByTestId('inspector-drawer');
  await expect(drawer).toBeVisible();

  // Make sure Payloads tab is active
  const payloadsTab = drawer.getByRole('button', { name: /payloads/i });
  await payloadsTab.click();

  const stageHeader = drawer.locator('span:has-text("Stage:")');
  const stageIdText = (await stageHeader.textContent()) ?? '';
  const stageId = stageIdText.replace('Stage:', '').trim();

  const preBlocks = drawer.locator('pre');
  const count = await preBlocks.count();
  const inputPayloadText = count > 0 ? (await preBlocks.nth(0).textContent()) ?? '' : '';
  const outputPayloadText = count > 1 ? (await preBlocks.nth(1).textContent()) ?? '' : '';

  return {
    stageId,
    status: 'inspected',
    inputPayloadText,
    outputPayloadText,
  };
}
