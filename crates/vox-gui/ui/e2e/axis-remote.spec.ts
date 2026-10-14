/**
 * Remote Axis (plan 06): surface gating names its reason, the Settings connect flow walks the TOFU
 * fingerprint confirm, and local mode is unchanged. installTauriMock in remote mode via an
 * axis_backend_status override. Screenshots land in review-bundle/latest/.
 *
 * Run: pnpm --dir crates/vox-gui/ui exec playwright test e2e/axis-remote.spec.ts --project=chromium
 */
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';
import { addInvokeOverrides } from './lib/invokeOverrides';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, '..', 'review-bundle', 'latest');
mkdirSync(OUT_DIR, { recursive: true });

const FP = '3f9a'.repeat(16);
const PROFILE = {
  profiles: ['mesh'],
  max_exposure: 'operator',
  subsystems: { booted: ['db', 'mesh'], failed: [] },
  tools: ['vox_mesh_status'],
  axis_surfaces: ['mesh', 'repository', 'console', 'models'],
  server_version: '0.6.0',
};
const REMOTE_STATUS = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: PROFILE,
  surfaces: [
    { surface: 'mesh', available: true, reason: null },
    { surface: 'repository', available: true, reason: null },
    { surface: 'settings', available: true, reason: null },
    { surface: 'console', available: false, reason: 'needs_local_machine' },
    { surface: 'models', available: false, reason: 'not_remote_capable' },
    { surface: 'tasks', available: false, reason: 'not_in_profile' },
  ],
};

async function openRemote(page: Page, view: string) {
  await page.setViewportSize({ width: 1400, height: 900 });
  await addMockInitScript(page, installTauriMock, view);
  await addInvokeOverrides(page, { responses: { axis_backend_status: REMOTE_STATUS } });
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
}

/** Self-contained (serialized into the page): stateful connect flow with a first-use pin prompt. */
function installConnectFlow(arg: { fp: string; remote: unknown }): void {
  const internals = (window as any).__TAURI_INTERNALS__;
  const base = internals.invoke;
  const local = { mode: 'local', url: null, profile: null, surfaces: [] };
  let status: unknown = local;
  (window as any).__AXIS_CONNECT_CALLS__ = [];
  internals.invoke = async (cmd: string, args?: any) => {
    if (cmd === 'axis_backend_status') return status;
    if (cmd === 'axis_connect_remote') {
      (window as any).__AXIS_CONNECT_CALLS__.push(args);
      if (!args?.acceptFingerprint) {
        throw JSON.stringify({ error: { code: 'tls_pin_required', message: 'first connection', fingerprint: arg.fp } });
      }
      status = arg.remote;
      return status;
    }
    if (cmd === 'axis_disconnect_remote') {
      status = local;
      return status;
    }
    return base(cmd, args);
  };
}

test.describe('Remote Axis', () => {
  for (const [view, reason, file] of [
    ['console', 'needs_local_machine', 'axis-remote-console-needs-local.png'],
    ['models', 'not_remote_capable', 'axis-remote-models-not-remote-capable.png'],
    ['tasks', 'not_in_profile', 'axis-remote-tasks-not-in-profile.png'],
  ] as const) {
    test(`${view} shows "not available on this server" (${reason})`, async ({ page }) => {
      await openRemote(page, view);
      const gate = page.getByTestId('surface-unavailable');
      await expect(gate).toBeVisible();
      await expect(gate).toHaveAttribute('data-reason', reason);
      await expect(page.getByText(/not available on this server/i).first()).toBeVisible();
      await expect(page.getByTestId('backend-status-pill')).toBeVisible();
      await page.screenshot({ path: join(OUT_DIR, file), fullPage: true });
    });
  }

  test('mesh renders remotely and the status pill names the server', async ({ page }) => {
    await openRemote(page, 'mesh');
    await expect(page.getByTestId('surface-unavailable')).toHaveCount(0);
    await expect(page.getByTestId('backend-status-pill')).toHaveAttribute('aria-label', 'Connected to vox.lan:3921, profile mesh');
    await page.screenshot({ path: join(OUT_DIR, 'axis-remote-mesh-available.png'), fullPage: true });
  });

  test('connect flow confirms the certificate fingerprint, then connects', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await addMockInitScript(page, installTauriMock, 'settings');
    await page.addInitScript(installConnectFlow, { fp: FP, remote: REMOTE_STATUS });
    await page.goto('/');
    await page.getByRole('button', { name: /^server$/i }).click();
    await page.getByLabel(/server url/i).fill('https://vox.lan:3921');
    await page.getByLabel(/bearer token/i).fill('t0ken');
    await page.getByRole('button', { name: /^connect$/i }).click();
    const dialog = page.getByRole('dialog', { name: /confirm server certificate/i });
    await expect(dialog).toBeVisible();
    await expect(page.getByTestId('server-fingerprint')).toContainText('3f9a 3f9a');
    await page.screenshot({ path: join(OUT_DIR, 'axis-remote-connect-tofu.png'), fullPage: true });
    await page.getByRole('button', { name: /trust and connect/i }).click();
    await expect(page.getByTestId('server-connected')).toBeVisible();
    await expect(page.getByTestId('backend-status-pill')).toBeVisible();
    const calls = await page.evaluate(() => (window as any).__AXIS_CONNECT_CALLS__);
    expect(calls).toEqual([
      { url: 'https://vox.lan:3921', token: 't0ken', acceptFingerprint: null },
      { url: 'https://vox.lan:3921', token: 't0ken', acceptFingerprint: FP },
    ]);
    await page.screenshot({ path: join(OUT_DIR, 'axis-remote-connected.png'), fullPage: true });
  });

  test('local mode is unchanged: no pill, no gate', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await addMockInitScript(page, installTauriMock, 'console');
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.getByTestId('surface-unavailable')).toHaveCount(0);
    await expect(page.getByTestId('backend-status-pill')).toHaveCount(0);
    await page.screenshot({ path: join(OUT_DIR, 'axis-local-unchanged.png'), fullPage: true });
  });
});
