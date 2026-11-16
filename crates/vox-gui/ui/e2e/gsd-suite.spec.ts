import { test, expect } from '@playwright/test';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

test.describe('GSD Suite GUI + Slash E2E', () => {
  test('renders GSD Studio, executes phases, stores/reads CAS documents, and creates milestone', async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'gsd');
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const studio = page.getByTestId('gsd-studio-view');
    await expect(studio).toBeVisible();
    await expect(studio.getByText(/v7\.0 · GSD Deep Suite Integration/)).toBeVisible();
    await expect(page.getByTestId('gsd-phase-card-407')).toBeVisible();
    await expect(page.getByTestId('gsd-phase-card-412')).toBeVisible();

    // Preview existing CAS document
    await page.getByTestId('gsd-doc-item-.planning/STATE.md').click();
    await expect(page.getByTestId('gsd-doc-preview')).toContainText('Current Phase: 411');

    // Store a new CAS document
    await page.getByLabel('Document relative path').fill('.planning/UAT-NOTES.md');
    await page.getByLabel('Document markdown content').fill('# UAT Notes\n\nVerified Axis GUI.');
    await page.getByRole('button', { name: 'Store CAS Doc' }).click();
    await expect(page.getByTestId('gsd-doc-item-.planning/UAT-NOTES.md')).toBeVisible();

    // Execute phase 411 waves
    const phase411 = page.getByTestId('gsd-phase-card-411');
    await phase411.getByRole('button', { name: 'Execute Phase' }).click();
    await expect(phase411.getByText('executing')).toBeVisible();

    // Create new milestone v7.1 via form
    await page.getByLabel('Milestone version').fill('v7.1');
    await page.getByLabel('Milestone title').fill('Autonomic GSD Waves');
    await page.getByRole('button', { name: 'New Milestone' }).click();
    await expect(studio.getByText(/v7\.1 · Autonomic GSD Waves/)).toBeVisible();
  });

  test('dispatches /gsd-new-milestone from Loquela composer to GSD Studio', async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'chat');
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const composer = page.getByLabel('Task composer');
    await composer.fill('/gsd-new-milestone v7.2 Composer Driven Milestone');
    await composer.press('Enter');

    await expect(page.getByTestId('gsd-studio-view')).toBeVisible();
    await expect(page.getByTestId('gsd-studio-view')).toContainText('v7.2 · Composer Driven Milestone');
  });
});
