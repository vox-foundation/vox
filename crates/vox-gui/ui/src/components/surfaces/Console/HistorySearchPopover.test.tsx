import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import React from 'react';
import * as transport from '../../../transport';
import {
  HistorySearchPopover,
  clearClientFallbackHistory,
  queryClientFallbackHistory,
  recordClientFallbackHistory,
} from './HistorySearchPopover';

vi.mock('../../../transport', () => ({
  termHistoryQuery: vi.fn().mockResolvedValue([]),
  termHistoryClear: vi.fn().mockResolvedValue(true),
}));

describe('HistorySearchPopover', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    clearClientFallbackHistory();
  });

  it('remains completely unmounted when closed and renders role="region" + role="searchbox" (never dialog or textbox) when open', async () => {
    const { container, rerender } = render(
      <HistorySearchPopover open={false} onSelect={vi.fn()} onCancel={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
    expect(screen.queryByTestId('term-history-search')).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();

    rerender(<HistorySearchPopover open={true} cwd="/repo" onSelect={vi.fn()} onCancel={vi.fn()} />);

    const region = await screen.findByTestId('term-history-search');
    expect(region.getAttribute('role')).toBe('region');
    expect(region.getAttribute('aria-label')).toBe('Command history search');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();

    const searchbox = screen.getByTestId('term-history-input') as HTMLInputElement;
    expect(searchbox.type).toBe('search');
    expect(screen.getByRole('searchbox')).toBe(searchbox);
  });

  it('queries transport.termHistoryQuery, supports Arrow navigation, Enter accept, Tab insert, Esc cancel, and empty states', async () => {
    const onSelect = vi.fn();
    const onCancel = vi.fn();

    // First verify empty state when 0 commands recorded
    vi.mocked(transport.termHistoryQuery).mockResolvedValueOnce([]);
    const { unmount } = render(
      <HistorySearchPopover open={true} cwd="/repo/vox" onSelect={onSelect} onCancel={onCancel} />,
    );
    await waitFor(() => expect(transport.termHistoryQuery).toHaveBeenCalledWith('', '/repo/vox', 50));
    await waitFor(() => expect(screen.getByText('No command history recorded yet')).toBeTruthy());
    expect(
      screen.getByText('Commands executed in this workspace are saved automatically with secrets scrubbed.'),
    ).toBeTruthy();
    unmount();

    // Now render with populated history results
    vi.mocked(transport.termHistoryQuery).mockImplementation(async (q = '') => {
      const rows = [
        { command: 'cargo test -p vox-cli', cwd: '/repo/vox', exit_code: 0, ts_ms: 2000 },
        { command: 'vox ci ssot-drift', cwd: '/repo/vox', exit_code: 1, ts_ms: 1000 },
      ];
      return q ? rows.filter((r) => r.command.includes(q)) : rows;
    });

    render(
      <HistorySearchPopover open={true} cwd="/repo/vox" onSelect={onSelect} onCancel={onCancel} />,
    );

    await waitFor(() => expect(screen.getByTestId('term-history-item-0')).toBeTruthy());
    expect(screen.getByTestId('term-history-item-0').textContent).toContain('cargo test -p vox-cli');
    expect(screen.getByTestId('term-history-item-0').textContent).toContain('exit 0');
    expect(screen.getByTestId('term-history-item-1').textContent).toContain('exit 1');

    const input = screen.getByTestId('term-history-input');
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(screen.getByTestId('term-history-item-1').getAttribute('aria-selected')).toBe('true');

    fireEvent.keyDown(input, { key: 'Tab' });
    expect(onSelect).toHaveBeenCalledWith('vox ci ssot-drift', 'insert');

    fireEvent.keyDown(input, { key: 'ArrowUp' });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSelect).toHaveBeenCalledWith('cargo test -p vox-cli', 'accept');

    fireEvent.keyDown(input, { key: 'Escape' });
    expect(onCancel).toHaveBeenCalledTimes(1);

    // Filter miss renders "No matching commands" with query copy
    fireEvent.change(input, { target: { value: 'zzz-nonexistent' } });
    await waitFor(() => expect(screen.getByText('No matching commands')).toBeTruthy());
    expect(
      screen.getByText('0 matching commands for "zzz-nonexistent" - edit your filter or press Esc to return to the prompt.'),
    ).toBeTruthy();
  });

  it('enforces ignorespace and secret scrubbing in client fallback ring and requires 2-step Confirm Clear', async () => {
    // Leading space / tab commands are skipped before trimming
    recordClientFallbackHistory(' secret-cmd --password foo', '/repo', 0);
    recordClientFallbackHistory('\tsecret-tab-cmd', '/repo', 0);
    expect(queryClientFallbackHistory('', '/repo', 50)).toHaveLength(0);

    // Inline secrets are scrubbed before persistence
    recordClientFallbackHistory(
      'OPENAI_API_KEY=supersecret123 curl -H "Authorization: Bearer my-jwt-token" --key sk-live_1234567890abcdef1234',
      '/repo',
      0,
    );
    const stored = queryClientFallbackHistory('', '/repo', 50);
    expect(stored).toHaveLength(1);
    expect(stored[0].command).toContain('OPENAI_API_KEY=[REDACTED]');
    expect(stored[0].command).toContain('Bearer [REDACTED]');
    expect(stored[0].command).not.toContain('supersecret123');
    expect(stored[0].command).not.toContain('my-jwt-token');
    expect(stored[0].command).not.toContain('sk-live_1234567890abcdef1234');

    // Fall back to client ring when termHistoryQuery returns null
    vi.mocked(transport.termHistoryQuery).mockResolvedValue(null);

    const onCleared = vi.fn();
    render(
      <HistorySearchPopover
        open={true}
        cwd="/repo"
        onSelect={vi.fn()}
        onCancel={vi.fn()}
        onCleared={onCleared}
      />,
    );
    await waitFor(() => expect(screen.getByTestId('term-history-item-0')).toBeTruthy());
    expect(screen.getAllByTestId('term-history-redacted-badge').length).toBeGreaterThanOrEqual(2);

    // Two-step Clear History -> Keep History cancels, Confirm Clear History clears & notifies onCleared
    fireEvent.click(screen.getByRole('button', { name: /Clear History/i }));
    expect(screen.getByRole('button', { name: /Confirm Clear History/i })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Keep History/i }));
    expect(screen.queryByRole('button', { name: /Confirm Clear History/i })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: /Clear History/i }));
    fireEvent.click(screen.getByRole('button', { name: /Confirm Clear History/i }));

    await waitFor(() => expect(transport.termHistoryClear).toHaveBeenCalledTimes(1));
    expect(onCleared).toHaveBeenCalledTimes(1);
    expect(queryClientFallbackHistory('', '/repo', 50)).toHaveLength(0);
    await waitFor(() => expect(screen.getByText('No command history recorded yet')).toBeTruthy());
  });
});
