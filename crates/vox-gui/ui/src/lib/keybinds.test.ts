import { describe, it, expect } from 'vitest';
import { ACTION_REGISTRY, chordFromEvent, matchAction, parseBindings, DEFAULT_BINDINGS } from './keybinds';
describe('keybinds', () => {
  it('registry lists only real, dispatchable actions', () => {
    expect(ACTION_REGISTRY.map(a => a.id)).toContain('open-palette');
    expect(ACTION_REGISTRY.map(a => a.id)).toContain('pause-resume-agent');
    for (const a of ACTION_REGISTRY) expect(DEFAULT_BINDINGS[a.id]).toBeTruthy();
  });
  it('chordFromEvent normalizes modifiers', () => {
    expect(chordFromEvent({ key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false } as any)).toBe('Mod+K');
    expect(chordFromEvent({ key: 'B', metaKey: false, ctrlKey: true, shiftKey: false, altKey: false } as any)).toBe('Mod+B');
  });
  it('matchAction resolves a chord to an action id via bindings', () => {
    expect(matchAction('Mod+K', DEFAULT_BINDINGS)).toBe('open-palette');
    expect(matchAction('Mod+J', DEFAULT_BINDINGS)).toBeNull();
  });
});

import { formatChord, isEditableTarget, isShortcutHelpKey, paletteActionRows, isMacPlatform } from './keybinds';
// @vitest-environment jsdom is not set here: build elements via a tiny stub where DOM is not needed.
const key = (over: Record<string, unknown>) => ({ key: '?', metaKey: false, ctrlKey: false, altKey: false, defaultPrevented: false, target: { tagName: 'DIV', isContentEditable: false, closest: () => null }, ...over }) as any;

describe('show-shortcuts action (D-11)', () => {
  it('is registered and bound to Mod+/', () => {
    expect(ACTION_REGISTRY.find(a => a.id === 'show-shortcuts')?.label).toBe('Show keyboard shortcuts');
    expect(DEFAULT_BINDINGS['show-shortcuts']).toBe('Mod+/');
  });
});
describe('formatChord', () => {
  it('formats per platform', () => {
    expect(formatChord('Mod+Shift+D', true)).toBe('⌘⇧D');
    expect(formatChord('Mod+Shift+D', false)).toBe('Ctrl+Shift+D');
    expect(formatChord('Mod+K', false)).toBe('Ctrl+K');
    expect(formatChord('Mod+Alt+K', true)).toBe('⌘⌥K');
  });
  it('empty chord renders a dash (DISC-07 empty bindings)', () => {
    expect(formatChord('', true)).toBe('—');
    expect(formatChord('', false)).toBe('—');
  });
  it('isMacPlatform does not throw', () => { expect(typeof isMacPlatform()).toBe('boolean'); });
});
describe('isEditableTarget', () => {
  const el = (tagName: string, extra: Record<string, unknown> = {}) => ({ tagName, isContentEditable: false, closest: () => null, ...extra }) as any;
  it('true for input, textarea, select, contenteditable and its children', () => {
    expect(isEditableTarget(el('INPUT'))).toBe(true);
    expect(isEditableTarget(el('TEXTAREA'))).toBe(true);
    expect(isEditableTarget(el('SELECT'))).toBe(true);
    expect(isEditableTarget(el('DIV', { isContentEditable: true }))).toBe(true);
    expect(isEditableTarget(el('SPAN', { closest: (s: string) => (s === '[contenteditable]' ? {} : null) }))).toBe(true);
  });
  it('false for button, div, null', () => {
    expect(isEditableTarget(el('BUTTON'))).toBe(false);
    expect(isEditableTarget(el('DIV'))).toBe(false);
    expect(isEditableTarget(null)).toBe(false);
  });
});
describe('isShortcutHelpKey (prohibition: ? never hijacks typing)', () => {
  it('? on a non-editable target opens', () => { expect(isShortcutHelpKey(key({}))).toBe(true); });
  it('rejects non-? keys', () => { expect(isShortcutHelpKey(key({ key: '/' }))).toBe(false); });
  it('rejects meta, ctrl without alt, defaultPrevented', () => {
    expect(isShortcutHelpKey(key({ metaKey: true }))).toBe(false);
    expect(isShortcutHelpKey(key({ ctrlKey: true }))).toBe(false);
    expect(isShortcutHelpKey(key({ defaultPrevented: true }))).toBe(false);
  });
  it('AltGr (ctrl+alt) still opens', () => { expect(isShortcutHelpKey(key({ ctrlKey: true, altKey: true }))).toBe(true); });
  it('rejects when the target is an input / contenteditable child', () => {
    expect(isShortcutHelpKey(key({ target: { tagName: 'INPUT', isContentEditable: false, closest: () => null } }))).toBe(false);
    expect(isShortcutHelpKey(key({ target: { tagName: 'SPAN', isContentEditable: false, closest: (s: string) => (s === '[contenteditable]' ? {} : null) } }))).toBe(false);
  });
});
describe('paletteActionRows (D-13 helper, DISC-07 edges)', () => {
  const ids = new Set(ACTION_REGISTRY.map(a => a.id));
  it('follows registry order and formats chords', () => {
    const rows = paletteActionRows(DEFAULT_BINDINGS, ids, false);
    expect(rows.map(r => r.id)).toEqual(ACTION_REGISTRY.map(a => a.id));
    expect(rows.find(r => r.id === 'open-palette')?.chord).toBe('Ctrl+K');
  });
  it('omits actions without a handler (dispatch-intent is composer-only)', () => {
    const noDispatch = new Set([...ids].filter(i => i !== 'dispatch-intent'));
    expect(paletteActionRows(DEFAULT_BINDINGS, noDispatch, false).map(r => r.id)).not.toContain('dispatch-intent');
  });
  it('adjacent/equal chords: both actions appear as separate rows', () => {
    const rows = paletteActionRows({ ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+K' }, ids, true);
    expect(rows.filter(r => r.chord === '⌘K').map(r => r.id)).toEqual(['open-palette', 'toggle-sidebar']);
  });
  it('stable ordering across rebinds', () => {
    const a = paletteActionRows(DEFAULT_BINDINGS, ids, true).map(r => r.id);
    const b = paletteActionRows({ ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' }, ids, true).map(r => r.id);
    expect(b).toEqual(a);
  });
  it('empty bindings fall back to defaults', () => {
    const rows = paletteActionRows({}, ids, false);
    expect(rows.find(r => r.id === 'toggle-sidebar')?.chord).toBe('Ctrl+B');
  });
});

describe('parseBindings (WR-12: a corrupt stored value must never reach formatChord)', () => {
  it('drops non-string chords and non-action keys, keeps valid overrides', () => {
    const b = parseBindings(JSON.stringify({ 'open-palette': 5, 'toggle-sidebar': 'Mod+Y', 'show-shortcuts': null, bogus: 'Mod+Z' }));
    expect(b['open-palette']).toBe(DEFAULT_BINDINGS['open-palette']);
    expect(b['show-shortcuts']).toBe(DEFAULT_BINDINGS['show-shortcuts']);
    expect(b['toggle-sidebar']).toBe('Mod+Y');
    expect(b).not.toHaveProperty('bogus');
  });

  it('non-object JSON (null, array, number) gives defaults', () => {
    for (const j of ['null', '[1,2]', '7', '"x"']) expect(parseBindings(j)).toEqual(DEFAULT_BINDINGS);
  });

  it('paletteActionRows never throws on a parsed corrupt value', () => {
    const b = parseBindings(JSON.stringify({ 'open-palette': { a: 1 } }));
    const rows = paletteActionRows(b, new Set(['open-palette']), true);
    expect(rows[0].chord).toBe('⌘K');
  });
});
