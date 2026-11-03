/**
 * Chat composer submit -> `chat_turn` -> persist against the tauriMock.
 * Guards three contracts:
 *  - frontend double-dispatch (e.g. duplicate Enter handling): exactly ONE
 *    `chat_turn` per submit (the single composer dispatch since dfb707f38);
 *  - the C2 `already_submitted` contract: the persisted user row must carry
 *    `already_submitted: true` — that flag is what stops the Rust backend's
 *    secretary re-submit (the re-submit itself happens daemon-side and is
 *    invisible to this mock, so the flag IS the observable C2 guard here);
 *  - `chat_turn` already persists the sync reply server-side, so the GUI must
 *    NOT re-persist it via `chat_append_message` (double row on reload).
 *
 * The former "stream tokens into the transcript" flow no longer exists from
 * the composer: Quick chat is request/response, and Background task runs in a
 * throwaway `bg-task-*` session (App.tsx submitFromComposer) so its stream is
 * deliberately not folded into the active chat transcript.
 */
import { test, expect, type Page } from '@playwright/test';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

type Call = { cmd: string; args?: any };
const calls = async (page: Page, pred: (c: Call) => boolean): Promise<Call[]> =>
  ((await page.evaluate(() => (window as any).__TAURI_CALLS__)) as Call[]).filter(pred);
const isTurn = (c: Call) => c.cmd === 'chat_turn';

async function submit(page: Page, text: string) {
  const composer = page.getByLabel('Task composer');
  await composer.fill(text);
  await composer.press('Enter');
}

async function expectUserRowPersisted(page: Page, text: string) {
  await expect
    .poll(async () =>
      (await calls(
        page,
        (c) =>
          c.cmd === 'chat_append_message' &&
          c.args?.input?.role === 'user' &&
          c.args?.input?.content === text &&
          c.args?.input?.already_submitted === true,
      )).length,
    )
    .toBe(1);
}

test.beforeEach(async ({ page }) => {
  await addMockInitScript(page, installTauriMock, 'chat');
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
});

test('quick chat submits one sync chat_turn, renders the reply, and does not re-persist it', async ({ page }) => {
  await submit(page, 'Summarize the repository layout');

  await expect(page.getByText('Summarize the repository layout')).toBeVisible();
  await expect.poll(async () => (await calls(page, isTurn)).length).toBe(1);
  const [turn] = await calls(page, isTurn);
  expect(turn.args.input).toMatchObject({ execution: 'sync', content: 'Summarize the repository layout' });
  await expectUserRowPersisted(page, 'Summarize the repository layout');

  await expect(page.getByText('Mock quick-chat reply.')).toBeVisible();
  // Settle, then confirm the reply was never appended a second time client-side.
  await page.waitForTimeout(500);
  expect(await calls(page, (c) => c.cmd === 'chat_append_message' && c.args?.input?.role === 'assistant')).toHaveLength(0);
});

test('background task submits one background chat_turn carrying the originating chat session', async ({ page }) => {
  await page.getByRole('button', { name: 'Choose send mode' }).click();
  await page.getByRole('button', { name: 'Set send mode: Background task' }).click();
  await submit(page, 'Harden the crypto invariants');

  await expect.poll(async () => (await calls(page, isTurn)).length).toBe(1);
  const [turn] = await calls(page, isTurn);
  expect(turn.args.input.execution).toBe('background');
  // Dispatched under a throwaway session, but lineage points at the real chat.
  expect(turn.args.input.session_id).toMatch(/^bg-task-/);
  expect(turn.args.input.chat_session_id).not.toMatch(/^bg-task-/);
  expect(turn.args.input.chat_session_id).toBeTruthy();
  await expectUserRowPersisted(page, 'Harden the crypto invariants');
});
