/**
 * Keyboard shortcut registry — replaces VS Code keybinding contributions.
 *
 * Usage:
 *   import { keymap } from '../host';
 *   const unbind = keymap.bind('ctrl+shift+p', () => commandPalette.open());
 *   // later: unbind();
 */

export interface KeyBinding {
  key: string;       // e.g. "ctrl+shift+p", "cmd+k"
  handler: () => void;
  description?: string;
}

const _bindings: Map<string, KeyBinding[]> = new Map();

const MOD_ORDER = ['ctrl', 'alt', 'shift', 'cmd'] as const;

function normalise(key: string): string {
  const parts = key.toLowerCase().replace(/\s+/g, '').replace(/\bmeta\b/g, 'cmd').split('+');
  const mods = MOD_ORDER.filter(m => parts.includes(m));
  const k = parts.filter(p => !(MOD_ORDER as readonly string[]).includes(p));
  return [...mods, ...k].join('+');
}

function fromEvent(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey)  parts.push('ctrl');
  if (e.altKey)   parts.push('alt');
  if (e.shiftKey) parts.push('shift');
  if (e.metaKey)  parts.push('cmd');
  parts.push(e.key.toLowerCase());
  return parts.join('+');
}

if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e: KeyboardEvent) => {
    const chord = fromEvent(e);
    const handlers = _bindings.get(chord);
    if (handlers?.length) {
      e.preventDefault();
      handlers[handlers.length - 1].handler();
    }
  });
}

export const keymap = {
  bind(key: string, handler: () => void, description?: string): () => void {
    const chord = normalise(key);
    const binding: KeyBinding = { key: chord, handler, description };
    const existing = _bindings.get(chord) ?? [];
    _bindings.set(chord, [...existing, binding]);
    return () => {
      const arr = _bindings.get(chord) ?? [];
      _bindings.set(chord, arr.filter((b) => b !== binding));
    };
  },

  list(): KeyBinding[] {
    return Array.from(_bindings.values()).flat();
  },
};
