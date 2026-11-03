// @vitest-environment jsdom
import { renderHook } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useKeybinds } from './useKeybinds';
import { DEFAULT_BINDINGS } from '../lib/keybinds';
describe('useKeybinds', () => {
  it('fires the bound action on matching keydown', () => {
    const onPalette = vi.fn();
    renderHook(() => useKeybinds({ 'open-palette': onPalette }, DEFAULT_BINDINGS));
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, cancelable: true }));
    expect(onPalette).toHaveBeenCalledTimes(1);
  });
});

import { act, renderHook as rh } from '@testing-library/react';
import { useKeybindings, useShortcutHelpKey } from './useKeybinds';
import { setBindings, resetKeybindStoreForTests } from '../lib/keybindStore';

describe('useKeybindings (D-17)', () => {
  it('re-renders with the new value after setBindings', () => {
    resetKeybindStoreForTests();
    const { result } = rh(() => useKeybindings());
    expect(result.current['toggle-sidebar']).toBe('Mod+B');
    act(() => setBindings({ ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' }));
    expect(result.current['toggle-sidebar']).toBe('Mod+Shift+Y');
  });
});

describe('useShortcutHelpKey (prohibition: ? never hijacks typing)', () => {
  const press = (target: EventTarget) => {
    const e = new KeyboardEvent('keydown', { key: '?', bubbles: true, cancelable: true });
    target.dispatchEvent(e);
    return e;
  };
  it('calls fn for ? on the body and prevents default', () => {
    const fn = vi.fn();
    rh(() => useShortcutHelpKey(fn));
    const e = press(document.body);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(e.defaultPrevented).toBe(true);
  });
  it('does not call fn and does not swallow ? when typing in an input', () => {
    const fn = vi.fn();
    rh(() => useShortcutHelpKey(fn));
    const input = document.createElement('input');
    document.body.appendChild(input);
    const e = press(input);
    expect(fn).not.toHaveBeenCalled();
    expect(e.defaultPrevented).toBe(false);
    input.remove();
  });
  it('does not call fn for a contenteditable child', () => {
    const fn = vi.fn();
    rh(() => useShortcutHelpKey(fn));
    const host = document.createElement('div');
    host.setAttribute('contenteditable', 'true');
    const child = document.createElement('span');
    host.appendChild(child);
    document.body.appendChild(host);
    press(child);
    expect(fn).not.toHaveBeenCalled();
    host.remove();
  });
});
