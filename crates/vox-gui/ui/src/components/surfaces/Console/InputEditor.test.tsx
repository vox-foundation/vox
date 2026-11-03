// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import React from 'react';

vi.mock('../../../transport', () => ({
  discoverySuggest: vi.fn().mockResolvedValue([
    { action_id: 'vox.config.show', completion: 'config show', about: 'show config' },
  ]),
}));

import { InputEditor } from './InputEditor';

describe('InputEditor', () => {
  beforeEach(() => cleanup());

  it('shows ghost text for the top suggestion as you type', async () => {
    render(<InputEditor onSubmit={vi.fn()} onActiveSuggestion={vi.fn()} />);
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'vox config' } });
    // Ghost shows only the completion remainder after what's typed ("config").
    await waitFor(() => expect(screen.getByTestId('ghost').textContent).toContain('show'));
  });

  it('accepts ghost text on Tab and submits on Enter', async () => {
    const onSubmit = vi.fn();
    render(<InputEditor onSubmit={onSubmit} onActiveSuggestion={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'vox config' } });
    await waitFor(() => expect(screen.getByTestId('ghost').textContent).toBeTruthy());
    fireEvent.keyDown(input, { key: 'Tab' });
    expect(input.value).toBe('vox config show');
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSubmit).toHaveBeenCalledWith('vox config show');
  });

  it('marks the input as inline-autocomplete and announces the active completion', async () => {
    render(<InputEditor onSubmit={vi.fn()} onActiveSuggestion={vi.fn()} />);
    const input = screen.getByRole('textbox');
    expect(input.getAttribute('aria-autocomplete')).toBe('inline');
    fireEvent.change(input, { target: { value: 'vox config' } });
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toMatch(/config show/i),
    );
  });

  it('renders term-shell-mode-select and auto-prefixes !nu in nu mode unless starting with / or ?', async () => {
    const onSubmit = vi.fn();
    render(<InputEditor onSubmit={onSubmit} onActiveSuggestion={vi.fn()} />);

    const select = screen.getByTestId('term-shell-mode-select') as HTMLSelectElement;
    expect(select.value).toBe('default');

    fireEvent.change(select, { target: { value: 'nu' } });
    expect(select.value).toBe('nu');

    const input = screen.getByRole('textbox') as HTMLInputElement;

    // Normal command: auto-prefixes !nu
    fireEvent.change(input, { target: { value: 'ls | where size > 10kb' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSubmit).toHaveBeenCalledWith('!nu ls | where size > 10kb');

    // / route command: no !nu prefix
    onSubmit.mockClear();
    fireEvent.change(input, { target: { value: '/model flash' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSubmit).toHaveBeenCalledWith('/model flash');

    // ? agent command: no !nu prefix
    onSubmit.mockClear();
    fireEvent.change(input, { target: { value: '?how to parse json' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSubmit).toHaveBeenCalledWith('?how to parse json');
  });

  it('renders IntelliSensePopover with category tabs and keyboard navigation (51-03)', async () => {
    render(<InputEditor onSubmit={vi.fn()} onActiveSuggestion={vi.fn()} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'vox config' } });

    await waitFor(() => expect(screen.getByTestId('intellisense-popover')).toBeTruthy());
    expect(screen.getByTestId('intellisense-tab-all')).toBeTruthy();

    const options = screen.getAllByTestId('intellisense-option');
    expect(options.length).toBeGreaterThanOrEqual(1);
    expect(options[0].getAttribute('data-active')).toBe('true');

    fireEvent.keyDown(input, { key: 'Escape' });
    expect(screen.queryByTestId('intellisense-popover')).toBeNull();
  });
});

