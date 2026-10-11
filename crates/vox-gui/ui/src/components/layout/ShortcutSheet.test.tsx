// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, within, act, fireEvent } from '@testing-library/react';

vi.mock('../../transport', () => ({
  voxTransport: { getGuiPreference: vi.fn().mockResolvedValue(null), setGuiPreference: vi.fn().mockResolvedValue(undefined) },
}));

import { ShortcutSheet } from './ShortcutSheet';
import { ACTION_REGISTRY, DEFAULT_BINDINGS, formatChord, isMacPlatform } from '../../lib/keybinds';
import { setBindings, resetKeybindStoreForTests } from '../../lib/keybindStore';

const chordOf = (label: string) => {
  const row = screen.getByText(label).closest('[data-testid^="shortcut-row-"]') as HTMLElement;
  return within(row).getByText((_, el) => el?.tagName === 'KBD').textContent;
};

describe('ShortcutSheet (D-11, D-12)', () => {
  beforeEach(() => resetKeybindStoreForTests());

  it('is a titled dialog listing every registry action (dispatch-intent included) with its chord in <kbd>', () => {
    render(<ShortcutSheet open onClose={() => {}} />);
    expect(screen.getByRole('dialog', { name: 'Keyboard shortcuts' })).toBeTruthy();
    const mac = isMacPlatform();
    for (const a of ACTION_REGISTRY) {
      expect(chordOf(a.label)).toBe(formatChord(DEFAULT_BINDINGS[a.id], mac));
    }
    expect(screen.getByText('Dispatch intent (in composer)')).toBeTruthy();
  });

  it('rows follow ACTION_REGISTRY order', () => {
    render(<ShortcutSheet open onClose={() => {}} />);
    const ids = screen.getAllByTestId(/^shortcut-row-/).map(r => r.getAttribute('data-testid'));
    expect(ids).toEqual(ACTION_REGISTRY.map(a => `shortcut-row-${a.id}`));
  });

  it('a rebind shows in the open sheet without remount (D-17)', () => {
    render(<ShortcutSheet open onClose={() => {}} />);
    act(() => setBindings({ ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+Shift+Y' }));
    expect(chordOf('Toggle sidebar width')).toBe(formatChord('Mod+Shift+Y', isMacPlatform()));
  });

  it('two actions on the same chord render as two rows (DISC-07 adjacency)', () => {
    render(<ShortcutSheet open onClose={() => {}} />);
    act(() => setBindings({ ...DEFAULT_BINDINGS, 'toggle-sidebar': 'Mod+K' }));
    expect(chordOf('Toggle sidebar width')).toBe(chordOf('Open command palette'));
    expect(screen.getAllByTestId(/^shortcut-row-/)).toHaveLength(ACTION_REGISTRY.length);
  });

  it('an empty binding renders a dash, the row stays (DISC-07 empty)', () => {
    render(<ShortcutSheet open onClose={() => {}} />);
    act(() => setBindings({ ...DEFAULT_BINDINGS, 'toggle-sidebar': '' }));
    expect(chordOf('Toggle sidebar width')).toBe('—');
  });

  it('Escape calls onClose', () => {
    const onClose = vi.fn();
    render(<ShortcutSheet open onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  it('renders nothing when closed', () => {
    render(<ShortcutSheet open={false} onClose={() => {}} />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
