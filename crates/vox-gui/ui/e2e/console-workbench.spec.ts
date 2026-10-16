import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

test.describe('Console workbench tab — Warp parity & intelligent model routing', () => {
  test('console tab renders Warp block stream, executes organic shell/vox/AI commands, and routes models intelligently', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await addMockInitScript(page, installTauriMock, 'console');
    await page.goto('/#view=console');
    await page.waitForSelector('nav', { timeout: 15_000 });

    await expect(page.getByTestId('active-surface')).toHaveAttribute('data-view', 'console');
    await expect(page.getByTestId('console-root')).toBeVisible();

    // 1. Initial session tab and Warp block stream
    await expect(page.getByTestId('term-session-tab-s-1')).toContainText('s-1 (1)');
    const blockStream = page.getByTestId('block-stream-view');
    await expect(blockStream).toBeVisible();
    const cards = page.getByTestId('term-block-card');
    await expect(cards.nth(0)).toContainText('vox run');
    await expect(cards.nth(0)).toContainText('Ready.');

    const input = page.getByRole('textbox', { name: 'console input' });
    await expect(input).toBeVisible();

    // 2. Submit a shell command (!cargo test) and verify block #2 + session counter
    await input.fill('!cargo test -p vox-terminal-core');
    await input.press('Enter');
    await expect(cards.nth(1)).toContainText('cargo test -p vox-terminal-core');
    await expect(cards.nth(1)).toContainText('[ok] cargo test -p vox-terminal-core');
    await expect(page.getByTestId('term-session-tab-s-1')).toContainText('s-1 (2)');

    // 3. Submit a Warp AI ?prompt for debugging and verify intelligent model routing
    await input.fill('?fix the failing test with exit code 101');
    await input.press('Enter');
    await expect(cards.nth(2)).toContainText('task=Debugging');
    await expect(cards.nth(2)).toContainText('model=anthropic/claude-sonnet');
    await expect(cards.nth(2)).toContainText('axes=eff:15/prec:70/lat:15');

    // 4. Submit /model route for architectural planning and verify Elite tier routing
    await input.fill('/model route architect a multi-node terminal session roadmap');
    await input.press('Enter');
    await expect(cards.nth(3)).toContainText('task=Planning');
    await expect(cards.nth(3)).toContainText('model=anthropic/claude-opus');
    await expect(cards.nth(3)).toContainText('tier=Elite');

    // 5. Submit a Vox-native expression (40 + 2) and verify evaluation block
    await input.fill('40 + 2');
    await input.press('Enter');
    await expect(cards.nth(4)).toContainText('Int(42)');
    await expect(page.getByTestId('term-session-tab-s-1')).toContainText('s-1 (5)');

    // 6. Submit a structured Nushell command (!nu ls | to json) and verify structured json badge
    await input.fill('!nu ls | to json');
    await input.press('Enter');
    await expect(cards.nth(5)).toContainText('vox-terminal-core');
    const badge = cards.nth(5).getByTestId('term-block-structured-badge');
    await expect(badge).toBeVisible();
    await expect(badge).toContainText('structured json');

    // 7. Test Security Level selector
    const secSelect = page.getByTestId('term-security-level-select');
    await expect(secSelect).toBeVisible();
    await secSelect.selectOption('l4_system_exec');
    await expect(secSelect).toHaveValue('l4_system_exec');

    // 8. Test Control Mode pill and instant Escape takeover
    const controlPill = page.getByTestId('term-control-mode-pill');
    await expect(controlPill).toBeVisible();
    await controlPill.click(); // manual -> co_pilot
    await expect(controlPill).toContainText('Co-Pilot');
    await controlPill.click(); // co_pilot -> autopilot
    await expect(controlPill).toContainText('Autopilot');
    await page.keyboard.press('Escape'); // instant takeover -> manual
    await expect(controlPill).toContainText('Manual');

    // 9. Test Focus in Chat button
    const focusInChatBtn = cards.nth(5).getByTestId('term-block-focus-chat');
    await expect(focusInChatBtn).toBeVisible();
    await focusInChatBtn.click();

    // 10. Pin block #3 and verify data-pinned state
    await cards.nth(2).getByRole('button', { name: 'Pin' }).click();
    await expect(cards.nth(2)).toHaveAttribute('data-pinned', 'true');

    // Capture visual screenshot for computer-vision inspection
    mkdirSync(OUT_DIR, { recursive: true });
    await page.screenshot({ path: join(OUT_DIR, 'console-chat-symbiosis.png'), fullPage: true });
    await page.screenshot({ path: join(OUT_DIR, 'console-warp-parity-blocks.png'), fullPage: true });

    // 11. Create a second session (+ new session), verify isolation, and switch back to s-1
    await page.getByTestId('term-new-session').click();
    await expect(page.getByTestId('term-session-tab-s-2')).toContainText('s-2 (0)');
    await page.getByTestId('term-session-tab-s-1').click();
    await expect(cards.nth(4)).toContainText('Int(42)');
  });

  test('console input provides multi-source IntelliSense popover, category tabs, and ghost text (51-04)', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await addMockInitScript(page, installTauriMock, 'console');
    await page.goto('/#view=console');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const input = page.getByRole('textbox', { name: 'console input' });
    await expect(input).toBeVisible();

    // 1. Type / to trigger slash commands & skills
    await input.fill('/mo');
    const popover = page.getByTestId('intellisense-popover');
    await expect(popover).toBeVisible();
    await expect(page.getByTestId('intellisense-tab-all')).toBeVisible();
    await expect(page.getByTestId('intellisense-footer')).toBeVisible();

    // 2. Ghost text visible
    const ghost = page.getByTestId('ghost');
    await expect(ghost).toBeVisible();

    // 3. Tab to accept /model
    await input.press('Tab');
    await expect(input).toHaveValue('/model');

    // 4. Capture visual screenshots for CV audit
    const cvDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'test-results', 'intellisense-cv');
    mkdirSync(cvDir, { recursive: true });
    await page.screenshot({ path: join(cvDir, 'console-intellisense.png'), fullPage: true });
    await page.screenshot({ path: join(OUT_DIR, 'console-intellisense-popover.png'), fullPage: true });

    // 5. Escape closes popover
    await input.fill('vox run');
    await expect(popover).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(popover).not.toBeVisible();
  });
});

