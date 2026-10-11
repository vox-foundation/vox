/**
 * Shared live keybind store (D-17). App, Settings -> Keybinds and the shortcut
 * sheet all read this one snapshot, so a rebind reaches every consumer without
 * a restart. This module is the only non-test owner of the `gui.keybinds`
 * backend preference. IPC goes through voxTransport only.
 *
 * WR-10: nothing is persisted until the initial load has succeeded. A rebind made
 * earlier is held as a delta and merged over the saved bindings once they arrive, so a
 * failed or late load can never overwrite the user's stored bindings with defaults.
 */
import { voxTransport } from '../transport';
import { DEFAULT_BINDINGS, parseBindings, serializeBindings, type Bindings } from './keybinds';

const PREF_KEY = 'gui.keybinds';
let snapshot: Bindings = DEFAULT_BINDINGS;
let ready = false; // the initial load succeeded: persisting is safe
let loading = false;
let pending: Bindings = {}; // rebinds made before `ready`
const listeners = new Set<() => void>();

const notify = () => listeners.forEach(l => l());

function persist(b: Bindings) {
  voxTransport.setGuiPreference(PREF_KEY, serializeBindings(b)).catch(() => {}); // memory keeps the value
}

/** Starts (or retries after a failure) the initial load; a no-op while loading or once loaded. */
function load() {
  if (ready || loading) return;
  loading = true;
  voxTransport.getGuiPreference(PREF_KEY)
    .then(json => {
      ready = true;
      const hadPending = Object.keys(pending).length > 0;
      snapshot = { ...parseBindings(json), ...pending };
      pending = {};
      if (hadPending) persist(snapshot);
      notify();
    })
    .catch(() => {}) // stay on defaults, unpersisted; the next subscribe / rebind retries
    .finally(() => { loading = false; });
}

export function getBindings(): Bindings { return snapshot; }

export function subscribeBindings(fn: () => void): () => void {
  listeners.add(fn);
  load();
  return () => { listeners.delete(fn); };
}

/** Replace the live bindings and persist (deferred until the initial load succeeds). */
export function setBindings(next: Bindings): void {
  if (ready) {
    snapshot = next;
    persist(next);
  } else {
    for (const k of Object.keys(next)) if (next[k] !== snapshot[k]) pending[k] = next[k];
    snapshot = next;
    load();
  }
  notify();
}

export function resetKeybindStoreForTests(): void {
  snapshot = DEFAULT_BINDINGS;
  ready = false;
  loading = false;
  pending = {};
  listeners.clear();
}
