import { useEffect, useSyncExternalStore } from 'react';
import { chordFromEvent, isShortcutHelpKey, matchAction, type ActionId, type Bindings } from '../lib/keybinds';
import { getBindings, subscribeBindings } from '../lib/keybindStore';
export function useKeybinds(handlers: Partial<Record<ActionId, () => void>>, bindings: Bindings) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const id = matchAction(chordFromEvent(e), bindings);
      const fn = id ? handlers[id] : undefined;
      if (fn) { e.preventDefault(); fn(); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [handlers, bindings]);
}

/** Live bindings from the shared store (D-17). */
export function useKeybindings(): Bindings {
  return useSyncExternalStore(subscribeBindings, getBindings, getBindings);
}

/** Bare `?` outside a text field (D-11). Editable targets are left alone so the character types. */
export function useShortcutHelpKey(onOpen: () => void) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isShortcutHelpKey(e)) { e.preventDefault(); onOpen(); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onOpen]);
}
