import { test, expect } from '@playwright/test';
import {
  startRealBackendServer,
  type RealBackendHandle,
} from './real-backend/realBackendServer';
import { installRealBackendBridge } from './real-backend/realBackendBridge';

test.describe('Phase 80 Real-Backend E2E Verification Harness (REAL-01, REAL-02, REAL-03)', () => {
  let backend: RealBackendHandle;

  test.beforeEach(async () => {
    backend = await startRealBackendServer();
  });

  test.afterEach(async () => {
    await backend.close();
  });

  test('Chat + Loquela round-trips over live HTTP backend and persists across page.reload() (REAL-01, REAL-02)', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await installRealBackendBridge(page, backend.url, 'chat');
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const composer = page.getByLabel('Task composer');
    await expect(composer).toBeVisible();
    await composer.fill('Summarize the repository layout');
    await composer.press('Enter');

    await expect(page.getByText('Summarize the repository layout')).toBeVisible();
    await expect(
      page.getByText('Verified real-backend reply (cost: $0.0014, model: anthropic/claude-sonnet).'),
    ).toBeVisible();

    // Verify on-disk backend state recorded both user and assistant messages + cost receipt
    const disk = backend.readState();
    const msgs = disk.messages['real-session-1'] ?? [];
    expect(msgs.length).toBe(2);
    expect(msgs[0].role).toBe('user');
    expect(msgs[0].already_submitted).toBe(true);
    expect(msgs[1].role).toBe('assistant');
    expect(msgs[1].cost_usd).toBe(0.0014);

    // Reload browser page: wipes in-page memory and proves re-hydration from real backend disk state
    await page.reload();
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.getByText('Summarize the repository layout')).toBeVisible();
    await expect(
      page.getByText('Verified real-backend reply (cost: $0.0014, model: anthropic/claude-sonnet).'),
    ).toBeVisible();
  });

  test('Terminal (Console) executes blocks over live HTTP backend and persists across page.reload() (REAL-01, REAL-03)', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await installRealBackendBridge(page, backend.url, 'console');
    await page.goto('/#view=console');
    await page.waitForSelector('nav', { timeout: 15_000 });

    await expect(page.getByTestId('console-root')).toBeVisible();
    await expect(page.getByTestId('term-session-tab-s-1')).toContainText('s-1 (1)');

    const input = page.getByRole('textbox', { name: 'console input' });
    await input.fill('!cargo test -p vox-terminal-core');
    await input.press('Enter');

    const cards = page.getByTestId('term-block-card');
    await expect(cards.nth(1)).toContainText('[ok] cargo test -p vox-terminal-core');

    await input.fill('40 + 2');
    await input.press('Enter');
    await expect(cards.nth(2)).toContainText('Int(42)');
    await expect(page.getByTestId('term-session-tab-s-1')).toContainText('s-1 (3)');

    // Reload browser page and verify blocks re-hydrate from backend disk state
    await page.reload();
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.getByTestId('term-session-tab-s-1')).toContainText('s-1 (3)');
    await expect(page.getByTestId('term-block-card').nth(2)).toContainText('Int(42)');
  });

  test('Universal Palette (⌘K) and Capability Explorer query live backend command catalog (REAL-03)', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await installRealBackendBridge(page, backend.url, 'explorer');
    await page.goto('/#view=explorer');
    await page.waitForSelector('nav', { timeout: 15_000 });

    await expect(page.getByRole('heading', { name: 'Capability Explorer' })).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByRole('treegrid', { name: 'Capabilities' })).toBeVisible();

    await page.keyboard.press('Control+k');
    const searchInput = page.getByPlaceholder(/search surfaces, commands/i);
    await expect(searchInput).toBeVisible();
    await searchInput.fill('vox build');

    const buildRow = page.getByRole('button', { name: /^vox build\b/ });
    await expect(buildRow).toBeVisible({ timeout: 10_000 });
    await expect(buildRow).toContainText('CLI');
  });
});
