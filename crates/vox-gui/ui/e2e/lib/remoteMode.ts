import type { Page } from '@playwright/test';

/**
 * Install a Tauri IPC mock in which Axis reports remote mode (plan 06 `axis_backend_status`).
 * Plan 06 computes per-surface availability in Rust from the embedded manifest; this mock
 * bypasses Rust, so each spec states it: every `axisSurfaces` entry is available unless
 * `unavailable` names a reason for it (`surfaceAvailability` treats unlisted surfaces as
 * `not_in_profile`).
 */
export async function installRemoteMode(
  page: Page,
  view: string,
  responses: Record<string, unknown>,
  axisSurfaces: string[],
  unavailable: Record<string, 'not_in_profile' | 'needs_local_machine' | 'not_remote_capable'> = {},
): Promise<void> {
  await page.addInitScript(
    ({ view, responses, axisSurfaces, unavailable }) => {
      localStorage.setItem('vox_sidebar_mode', 'default');
      localStorage.setItem('vox_onboarding_dismissed', 'true');
      const w = window as unknown as Record<string, unknown>;
      const invokes: Array<[string, unknown]> = [];
      w.__VOX_INVOKES__ = invokes;
      w.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: unknown) => {
          invokes.push([cmd, args]);
          if (cmd === 'get_initial_view') return view;
          if (cmd === 'axis_backend_status') {
            return {
              mode: 'remote',
              url: 'https://vox.lan:3921/',
              profile: {
                profiles: ['dev'],
                max_exposure: 'operator',
                subsystems: { booted: [], failed: [] },
                tools: [],
                axis_surfaces: axisSurfaces,
                server_version: '0.6.0',
              },
              surfaces: axisSurfaces.map((surface) =>
                unavailable[surface]
                  ? { surface, available: false, reason: unavailable[surface] }
                  : { surface, available: true, reason: null },
              ),
            };
          }
          return cmd in responses ? responses[cmd] : null;
        },
      };
    },
    { view, responses, axisSurfaces, unavailable },
  );
}

export const SHOT_DIR = 'review-bundle/latest';
