import type { Page } from '@playwright/test';

/**
 * Installs a real HTTP/IPC Tauri invoke bridge in the Playwright page (`REAL-01`).
 *
 * Unlike `installTauriMock`, this bridge holds ZERO in-browser mock fixtures:
 * every command invocation (`chat_turn`, `chat_list_sessions`, `chat_get_messages`,
 * `term_session_submit`, `get_command_catalog`, etc.) performs a live network
 * `fetch()` round-trip to the ephemeral backend server (`serverUrl/rpc`) backed
 * by on-disk state. Reloading the page (`await page.reload()`) wipes browser
 * memory and re-hydrates strictly from the backend server.
 */
export async function installRealBackendBridge(
  page: Page,
  serverUrl: string,
  defaultView: string = 'chat',
): Promise<void> {
  await page.addInitScript(
    ({ serverUrl, defaultView }) => {
      try {
        window.localStorage.setItem('vox_sidebar_mode', JSON.stringify('wide'));
        window.localStorage.setItem('vox_onboarding_dismissed', 'true');
      } catch {
        // ignore sandboxed storage errors
      }

      const w = window as any;
      w.__TAURI_CALLS__ = [];
      w.__VOX_IPC_ACTIVE_COUNT__ = 0;
      w.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener: (_event: string, _eventId: number) => {},
      };
      w.__TAURI_EVENT_LISTENERS__ = {} as Record<string, string[]>;
      w.__TAURI_EMIT__ = (event: string, payload: unknown) => {
        const ids: string[] = (w.__TAURI_EVENT_LISTENERS__ ?? {})[event] ?? [];
        for (const id of ids) {
          const cb = w[id];
          if (typeof cb === 'function') cb({ event, id: 0, payload });
        }
      };

      w.__TAURI_INTERNALS__ = {
        transformCallback: (cb: (...args: unknown[]) => unknown) => {
          const id = `cb_${Math.random().toString(36).slice(2)}`;
          w[id] = cb;
          return id;
        },
        invoke: async (cmd: string, args?: any) => {
          w.__VOX_IPC_ACTIVE_COUNT__ = (w.__VOX_IPC_ACTIVE_COUNT__ || 0) + 1;
          w.__TAURI_CALLS__.push({ cmd, args: args ?? null });
          try {
            if (cmd === 'plugin:event|listen') {
              const eventId: number = (w.__TAURI_EVENT_NEXT_ID__ =
                (w.__TAURI_EVENT_NEXT_ID__ ?? 0) + 1);
              if (typeof args?.event === 'string' && typeof args?.handler === 'string') {
                (w.__TAURI_EVENT_LISTENERS__[args.event] ??= []).push(args.handler);
                (w.__TAURI_EVENT_BY_ID__ ??= {})[eventId] = {
                  event: args.event,
                  handler: args.handler,
                };
              }
              return eventId;
            }
            if (cmd === 'plugin:event|unlisten') {
              const hit = (w.__TAURI_EVENT_BY_ID__ ?? {})[args?.eventId];
              if (hit && w.__TAURI_EVENT_LISTENERS__?.[hit.event]) {
                w.__TAURI_EVENT_LISTENERS__[hit.event] = w.__TAURI_EVENT_LISTENERS__[
                  hit.event
                ].filter((h: string) => h !== hit.handler);
                delete w.__TAURI_EVENT_BY_ID__[args.eventId];
              }
              return null;
            }

            const resp = await fetch(`${serverUrl}/rpc`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ cmd, args: args ?? {}, defaultView }),
            });
            if (!resp.ok) {
              throw new Error(`Real-backend RPC ${cmd} failed: HTTP ${resp.status}`);
            }
            const envelope = await resp.json();
            if (cmd === 'get_orchestrator_status_bin' && Array.isArray(envelope.result)) {
              return new Uint8Array(envelope.result);
            }
            return envelope.result;
          } finally {
            w.__VOX_IPC_ACTIVE_COUNT__ = Math.max(0, (w.__VOX_IPC_ACTIVE_COUNT__ || 1) - 1);
          }
        },
      };
    },
    { serverUrl, defaultView },
  );
}
