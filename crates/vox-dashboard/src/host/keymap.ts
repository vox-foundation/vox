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

function normalise(key: string): string {
  return key.toLowerCase().replace(/\s+/g, '').replace('meta', 'cmd');
}

function fromEvent(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('ctrl');
  if (e.metaKey) parts.push('cmd');
  if (e.altKey) parts.push('alt');
  if (e.shiftKey) parts.push('shift');
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
