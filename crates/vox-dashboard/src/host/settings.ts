/**
 * Dashboard settings — replaces vscode.workspace.getConfiguration("vox").
 *
 * Persisted at GET/PUT /api/dashboard/settings (see src/api/settings.rs).
 *
 * Usage:
 *   import { settings } from '../host';
 *   const model = await settings.get('activeModel', 'gemini-2.0-flash-lite');
 *   await settings.set('activeModel', 'anthropic/claude-opus-4-7');
 */

const ENDPOINT = '/api/dashboard/settings';

type SettingsMap = Record<string, unknown>;

let _cache: SettingsMap | null = null;
type Listener = (settings: SettingsMap) => void;
const _listeners: Set<Listener> = new Set();

async function loadRemote(): Promise<SettingsMap> {
  try {
    const r = await fetch(ENDPOINT);
    if (r.ok) return (await r.json()) as SettingsMap;
  } catch { /* offline / dev — fall through to cache or empty */ }
  return {};
}

function notify() {
  const snap = { ...(_cache ?? {}) };
  for (const fn of _listeners) fn(snap);
}

export const settings = {
  async load(): Promise<SettingsMap> {
    _cache = await loadRemote();
    return { ..._cache };
  },

  async get<T = unknown>(key: string, defaultValue?: T): Promise<T> {
    if (_cache === null) await this.load();
    const v = (_cache as SettingsMap)[key];
    return v !== undefined ? (v as T) : (defaultValue as T);
  },

  async set(key: string, value: unknown): Promise<void> {
    if (_cache === null) await this.load();
    (_cache as SettingsMap)[key] = value;
    notify();
    try {
      await fetch(ENDPOINT, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ [key]: value }),
      });
    } catch { /* fire and forget in offline mode */ }
  },

  subscribe(fn: Listener): () => void {
    _listeners.add(fn);
    if (_cache !== null) fn({ ..._cache });
    return () => _listeners.delete(fn);
  },
};
