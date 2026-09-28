import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

export interface TrustOverridesPayload {
  responses: Record<string, unknown>;
}

/**
 * Data-driven override helper for Tauri invoke commands, running after the base mock.
 */
export async function installTrustOverrides(
  page: Page,
  payload: TrustOverridesPayload,
): Promise<void> {
  await addMockInitScript(page, installTauriMock, 'chat');
  await page.addInitScript(
    (arg: TrustOverridesPayload) => {
      const internals = (window as any).__TAURI_INTERNALS__;
      const base: ((cmd: string, args?: any) => Promise<unknown>) | undefined =
        internals?.invoke;
      if (typeof base !== 'function') {
        throw new Error('installTrustOverrides must run after installTauriMock');
      }
      internals.invoke = async (cmd: string, args?: any) => {
        if (arg?.responses && Object.prototype.hasOwnProperty.call(arg.responses, cmd)) {
          (window as any).__VOX_IPC_ACTIVE_COUNT__ =
            ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 0) + 1;
          (window as any).__TAURI_CALLS__.push({ cmd, args: args ?? null });
          try {
            return arg.responses[cmd];
          } finally {
            (window as any).__VOX_IPC_ACTIVE_COUNT__ = Math.max(
              0,
              ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 1) - 1,
            );
          }
        }
        return base(cmd, args);
      };
    },
    payload,
  );
}

test('receipt chips render on the assistant turn', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });

  await installTrustOverrides(page, {
    responses: {
      chat_turn: {
        id: 9001,
        role: 'assistant',
        content: 'Checked the repository status.',
        created_at: '2026-09-28T12:00:00.000Z',
        task_id: null,
        model_id: 'opus-4-8',
        events: [
          {
            kind: 'tool_receipt',
            tool: 'vox_git_status',
            receipt_id: '01920000-aaaa-7bbb-8ccc-000000000001',
            fulfilled: true,
            verified: true,
          },
          {
            kind: 'tool_receipt',
            tool: 'vox_skill_list',
            receipt_id: '01920000-aaaa-7bbb-8ccc-000000000002',
            fulfilled: true,
            verified: false,
          },
        ],
      },
    },
  });

  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });

  // ensure Quick chat mode (open the send-mode menu and click Set send mode: Quick chat if the menu is present)
  const chooseModeBtn = page.getByLabel('Choose send mode');
  if (await chooseModeBtn.isVisible()) {
    const isExpanded = await chooseModeBtn.getAttribute('aria-expanded');
    if (isExpanded !== 'true') {
      await chooseModeBtn.click();
    }
    const quickChatBtn = page.getByLabel('Set send mode: Quick chat');
    if (await quickChatBtn.isVisible()) {
      await quickChatBtn.click();
    }
  }

  const composer = page.getByLabel('Task composer');
  await composer.fill('Check the repo status');
  await composer.press('Enter');

  const receiptRows = page.getByTestId('chat-turn-receipt-row');
  await expect(receiptRows).toHaveCount(2);

  const firstRow = receiptRows.nth(0);
  await expect(firstRow).toContainText('vox_git_status');
  await expect(firstRow).toContainText('verified');
  await expect(firstRow).toHaveAttribute('data-verified', 'true');

  const secondRow = receiptRows.nth(1);
  await expect(secondRow).toContainText('vox_skill_list');
  await expect(secondRow).toContainText('unverified');
  await expect(secondRow).toHaveAttribute('data-verified', 'false');

  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as any).__TAURI_CALLS__.some((c: any) => c.cmd === 'chat_turn'),
      ),
    )
    .toBe(true);

  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);

  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({
    path: join(OUT_DIR, 'chat-trust-receipts.png'),
  });
});

test('claims verdict renders and flags fabricated claims', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });

  await installTrustOverrides(page, {
    responses: {
      chat_turn: {
        id: 9002,
        role: 'assistant',
        content: 'Verified claimed receipts.',
        created_at: '2026-09-28T12:05:00.000Z',
        task_id: null,
        model_id: 'opus-4-8',
        events: [
          {
            kind: 'tool_receipt',
            tool: 'vox_verify_task_claims',
            receipt_id: '01920000-aaaa-7bbb-8ccc-000000000003',
            fulfilled: true,
            verified: true,
          },
          {
            kind: 'receipt_claims',
            valid: 2,
            fabricated: 1,
            unverified: 0,
          },
        ],
      },
    },
  });

  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });

  // ensure Quick chat mode (open the send-mode menu and click Set send mode: Quick chat if the menu is present)
  const chooseModeBtn = page.getByLabel('Choose send mode');
  if (await chooseModeBtn.isVisible()) {
    const isExpanded = await chooseModeBtn.getAttribute('aria-expanded');
    if (isExpanded !== 'true') {
      await chooseModeBtn.click();
    }
    const quickChatBtn = page.getByLabel('Set send mode: Quick chat');
    if (await quickChatBtn.isVisible()) {
      await quickChatBtn.click();
    }
  }

  const composer = page.getByLabel('Task composer');
  await composer.fill('Verify the task claims');
  await composer.press('Enter');

  const claimsRow = page.getByTestId('chat-turn-claims-row');
  await expect(claimsRow).toHaveCount(1);
  await expect(claimsRow).toContainText('claims · 2 valid · 1 fabricated · 0 unverified');
  await expect(claimsRow).toHaveAttribute('data-flagged', 'true');

  const receiptRows = page.getByTestId('chat-turn-receipt-row');
  await expect(receiptRows).toHaveCount(1);
  await expect(receiptRows.nth(0)).toContainText('vox_verify_task_claims');

  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as any).__TAURI_CALLS__.some((c: any) => c.cmd === 'chat_turn'),
      ),
    )
    .toBe(true);

  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);

  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({
    path: join(OUT_DIR, 'chat-trust-claims.png'),
  });
});

test('holding lock renders under its task in the chat rail', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });

  await installTrustOverrides(page, {
    responses: {
      chat_list_sessions: [
        {
          session_id: 'trust-session',
          title: 'Trust chips',
          updated_at: 'now',
          message_count: 0,
          conversation_id: 1,
        },
      ],
      list_orchestrator_tasks: [
        {
          id: 77,
          description: 'Migrate orders table',
          priority: 'normal',
          lifecycle: 'in_progress',
          agent_id: 3,
          session_id: 'trust-session',
          estimated_complexity: 1,
          depends_on: [],
          write_files: [],
          remote_node: null,
          origin: 'orchestrator',
        },
      ],
      activity_query: [
        {
          id: 10,
          ts_ms: 1,
          agent_id: '3',
          session_id: 'trust-session',
          kind: 'LockAcquired',
          summary: 'Lock acquired',
          detail_json: JSON.stringify({
            type: 'lock_acquired',
            agent_id: 3,
            path: 'db://orders/42',
            exclusive: true,
            session_id: 'trust-session',
            task_id: 77,
          }),
        },
      ],
    },
  });

  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });

  // If the app does not auto-select trust-session, click it in the session rail
  const sessionTab = page.getByRole('tab', { name: /Trust chips/i });
  if (await sessionTab.isVisible()) {
    await sessionTab.click();
  }

  const activeTasksRegion = page.getByRole('region', { name: /Active tasks/i });
  await expect(activeTasksRegion).toBeVisible();
  await expect(activeTasksRegion.getByText('Migrate orders table')).toBeVisible();

  const lockChip = activeTasksRegion.getByTestId('execution-rail-lock-chip');
  await expect(lockChip).toBeVisible();
  await expect(lockChip).toHaveAttribute('data-lock-state', 'holding');
  await expect(lockChip).toContainText('holding db://orders/42');

  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as any).__TAURI_CALLS__.some(
          (c: any) =>
            c.cmd === 'activity_query' && c.args?.filter?.session_id === 'trust-session',
        ),
      ),
    )
    .toBe(true);

  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({
    path: join(OUT_DIR, 'chat-trust-lock-holding.png'),
  });
});


