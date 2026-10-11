/**
 * Shared live keybind store (D-17). App, Settings -> Keybinds and the shortcut
 * sheet all read this one snapshot, so a rebind reaches every consumer without
 * a restart. This module is the only non-test owner of the `gui.keybinds`
 * backend preference. IPC goes through voxTransport only.
 */
import { voxTransport } from '../transport';
import { DEFAULT_BINDINGS, parseBindings, serializeBindings, type Bindings } from './keybinds';

const PREF_KEY = 'gui.keybinds';
let snapshot: Bindings = DEFAULT_BINDINGS;
let loaded = false;
let touched = false; // a setBindings landed; a late load must not clobber it
const listeners = new Set<() => void>();

const notify = () => listeners.forEach(l => l());

function load() {
  if (loaded) return;
  loaded = true;
  voxTransport.getGuiPreference(PREF_KEY)
    .then(json => { if (!touched) { snapshot = parseBindings(json); notify(); } })
    .catch(() => {}); // keep defaults
}

export function getBindings(): Bindings { return snapshot; }

export function subscribeBindings(fn: () => void): () => void {
  listeners.add(fn);
  load();
  return () => { listeners.delete(fn); };
}

/** Replace the live bindings and persist. A failed persist keeps memory; the next save retries. */
export function setBindings(next: Bindings): void {
  touched = true;
  snapshot = next;
  notify();
  voxTransport.setGuiPreference(PREF_KEY, serializeBindings(next)).catch(() => {});
}

export function resetKeybindStoreForTests(): void {
  snapshot = DEFAULT_BINDINGS;
  loaded = false;
  touched = false;
  listeners.clear();
}
