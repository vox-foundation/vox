import { test, expect, type Page } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';
import fixture from './fixtures/remote-gate-mesh-console.json' with { type: 'json' };

// The fixture is the real Rust gate's output (pinned by `remote_gate_fixture_matches_real_gate`
// in `src/remote/manifest.rs`); never edit it by hand.
const unavailable = Object.fromEntries(
  fixture.surfaces.filter((s) => !s.available).map((s) => [s.surface, s.reason]),
) as Record<string, 'not_in_profile' | 'needs_local_machine' | 'not_remote_capable'>;

async function invokedCommands(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
  );
}

test.describe('Console surface — remote mode', () => {
  test('console stays host-only remotely and never reaches the console backends', async ({ page }) => {
    await installRemoteMode(page, 'console', {}, fixture.axis_surfaces, unavailable);
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    const gate = page.getByTestId('surface-unavailable');
    await expect(gate).toBeVisible();
    await expect(gate).toHaveAttribute('data-reason', 'needs_local_machine');
    await expect(
      page.getByText(
        'This surface works on files, processes, or settings on this machine. Disconnect from the server to use it.',
      ),
    ).toBeVisible();
    const invoked = await invokedCommands(page);
    expect(invoked.filter((c) => c === 'send_to_agent' || c.startsWith('pty_'))).toEqual([]);
    await page.screenshot({ path: `${SHOT_DIR}/console-remote-unavailable.png`, fullPage: true });
  });

  test('mesh from the same gate output is navigable', async ({ page }) => {
    await installRemoteMode(page, 'mesh', {}, fixture.axis_surfaces, unavailable);
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByTestId('surface-unavailable')).toHaveCount(0);
  });
});
