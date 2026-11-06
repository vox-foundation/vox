import { describe, it, expect, vi, beforeEach } from 'vitest';

const getPref = vi.fn();
const setPref = vi.fn();
vi.mock('../transport', () => ({
  voxTransport: { getGuiPreference: (k: string) => getPref(k), setGuiPreference: (k: string, v: string) => setPref(k, v) },
}));

import { getBindings, subscribeBindings, setBindings, resetKeybindStoreForTests } from './keybindStore';
import { DEFAULT_BINDINGS } from './keybinds';

const flush = () => new Promise(r => setTimeout(r, 0));

describe('keybindStore (D-17)', () => {
  beforeEach(() => {
    resetKeybindStoreForTests();
    getPref.mockReset().mockResolvedValue(null);
    setPref.mockReset().mockResolvedValue(undefined);
  });

  it('loads gui.keybinds exactly once on first subscribe', async () => {
    getPref.mockResolvedValue(JSON.stringify({ 'open-palette': 'Mod+P' }));
    const off1 = subscribeBindings(() => {});
    const off2 = subscribeBindings(() => {});
    await flush();
    expect(getPref).toHaveBeenCalledTimes(1);
    expect(getPref).toHaveBeenCalledWith('gui.keybinds');
    expect(getBindings()['open-palette']).toBe('Mod+P');
    expect(getBindings()['toggle-sidebar']).toBe(DEFAULT_BINDINGS['toggle-sidebar']);
    off1(); off2();
  });

  it('null, invalid JSON and a rejected load all give defaults', async () => {
    getPref.mockResolvedValue('{not json');
    subscribeBindings(() => {});
    await flush();
    expect(getBindings()).toEqual(DEFAULT_BINDINGS);
    resetKeybindStoreForTests();
    getPref.mockRejectedValue(new Error('no backend'));
    subscribeBindings(() => {});
    await flush();
    expect(getBindings()).toEqual(DEFAULT_BINDINGS);
  });

  it('setBindings notifies synchronously and persists once the store has loaded', async () => {
    const fn = vi.fn();
    subscribeBindings(fn);
    await flush(); // initial load (null -> defaults)
    fn.mockClear();
    const next = { ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' };
    setBindings(next);
    expect(fn).toHaveBeenCalled();
    expect(getBindings()).toBe(next);
    expect(setPref).toHaveBeenCalledWith('gui.keybinds', JSON.stringify(next));
  });

  it('a rejected persist leaves the in-memory value', async () => {
    setPref.mockRejectedValue(new Error('disk'));
    const next = { ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' };
    setBindings(next);
    await flush();
    expect(getBindings()).toEqual(next);
  });

  it('a late load does not clobber a rebind made before it resolved', async () => {
    let resolve!: (v: string | null) => void;
    getPref.mockReturnValue(new Promise<string | null>(r => { resolve = r; }));
    subscribeBindings(() => {});
    const next = { ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' };
    setBindings(next);
    resolve(null);
    await flush();
    expect(getBindings()).toEqual(next);
  });

  it('WR-10: a rebind before the load resolves is merged over the saved bindings, not over defaults', async () => {
    getPref.mockResolvedValue(JSON.stringify({ 'open-palette': 'Mod+P' }));
    subscribeBindings(() => {});
    setBindings({ ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' }); // load not resolved yet
    expect(setPref).not.toHaveBeenCalled(); // nothing persisted over an unloaded store
    await flush();
    expect(getBindings()['open-palette']).toBe('Mod+P'); // the user's saved binding survives
    expect(getBindings()['toggle-sidebar']).toBe('Mod+Shift+Y');
    expect(JSON.parse(setPref.mock.calls[0][1])).toMatchObject({ 'open-palette': 'Mod+P', 'toggle-sidebar': 'Mod+Shift+Y' });
  });

  it('WR-10: a failed initial load never persists defaults, and the next subscriber retries', async () => {
    getPref.mockRejectedValueOnce(new Error('backend not ready')).mockResolvedValue(JSON.stringify({ 'open-palette': 'Mod+P' }));
    subscribeBindings(() => {});
    await flush();
    setBindings({ ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' }); // retries the load itself
    expect(setPref).not.toHaveBeenCalled();
    await flush();
    expect(getPref).toHaveBeenCalledTimes(2);
    expect(getBindings()['open-palette']).toBe('Mod+P');
    expect(JSON.parse(setPref.mock.calls[0][1])['open-palette']).toBe('Mod+P');
    subscribeBindings(() => {});
    await flush();
    expect(getPref).toHaveBeenCalledTimes(2); // loaded: no further reads
  });
});
