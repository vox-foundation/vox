import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { mockInitScript } from './tauriMockShared';
import { installTauriMock } from './tauriMock';
import { familyKey } from '../../src/lib/modelFamily';

// e2e/lib -> e2e -> ui -> vox-gui -> crates -> repo root
const here = dirname(fileURLToPath(import.meta.url));
const BOOTSTRAP = resolve(here, '../../../../../contracts/orchestration/model-catalog.bootstrap.v1.json');
const CONTRACT_FAMILIES = new Set(
  (JSON.parse(readFileSync(BOOTSTRAP, 'utf8')) as Array<{ id: string }>).map((e) => familyKey(e.id)),
);
const LOCAL_PROVIDERS = new Set(['mens', 'local', 'ollama', 'populi_local']);

function makeFakeWindow(): any {
  const storage: Record<string, string> = {};
  return {
    localStorage: {
      setItem: (k: string, v: string) => {
        storage[k] = v;
      },
      getItem: (k: string) => storage[k] ?? null,
    },
  };
}

async function withMock<T>(fn: (invoke: (cmd: string) => Promise<any>) => Promise<T>): Promise<T> {
  const prev = (global as any).window;
  const win = makeFakeWindow();
  (global as any).window = win;
  try {
    // eslint-disable-next-line no-new-func -- exercising the exact addInitScript path
    new Function(mockInitScript(installTauriMock, 'chat'))();
    return await fn((cmd) => win.__TAURI_INTERNALS__.invoke(cmd));
  } finally {
    (global as any).window = prev;
  }
}

describe('tauriMock model data comes from contract families', () => {
  it('every cloud model card id is a family key of the bootstrap catalog', async () => {
    await withMock(async (invoke) => {
      const cards = (await invoke('list_model_cards')) as Array<{ id: string; provider: string }>;
      const cloud = cards.filter((c) => !LOCAL_PROVIDERS.has(c.provider));
      expect(cloud.length).toBeGreaterThan(0);
      for (const c of cloud) expect(CONTRACT_FAMILIES.has(c.id), c.id).toBe(true);
    });
  });

  it('the routing summary names a contract family, says where it came from, and carries no version', async () => {
    await withMock(async (invoke) => {
      const s = await invoke('get_routing_summary_live');
      expect(['catalog', 'bootstrap', 'local']).toContain(s.resolved_from);
      expect(CONTRACT_FAMILIES.has(s.family), s.family).toBe(true);
      expect(typeof s.reason).toBe('string');
      for (const id of [s.active_model, s.decision_preview.selected_model, ...s.decision_preview.alternatives]) {
        expect(CONTRACT_FAMILIES.has(id), id).toBe(true);
      }
    });
  });

  it('selection policy, explanation, suggestion and active model use contract families', async () => {
    await withMock(async (invoke) => {
      for (const id of (await invoke('get_selection_policy')).chain) expect(CONTRACT_FAMILIES.has(id), id).toBe(true);
      expect(CONTRACT_FAMILIES.has((await invoke('explain_model_selection')).chosen)).toBe(true);
      expect(CONTRACT_FAMILIES.has(await invoke('suggest_model_for_task'))).toBe(true);
      expect(CONTRACT_FAMILIES.has(await invoke('get_active_model'))).toBe(true);
    });
  });

  it('the auto model recommendation mock has the live selector shape', async () => {
    await withMock(async (invoke) => {
      const r = await invoke('get_auto_model_recommendation');
      expect(Object.keys(r).sort()).toEqual(['floor_mb', 'free_mb', 'reason', 'selected_model_id', 'size']);
      expect(r.selected_model_id).toMatch(/@[0-9a-f]{40}$/);
      expect(r.size).toBe('8B');
      expect(r.floor_mb).toBeLessThanOrEqual(r.free_mb);
      expect(typeof r.reason).toBe('string');
      expect(r.reason.length).toBeGreaterThan(0);
    });
  });
});
