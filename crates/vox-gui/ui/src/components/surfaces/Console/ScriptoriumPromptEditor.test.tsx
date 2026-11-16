import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import React from 'react';
import * as transport from '../../../transport';
import {
  clearClientFallbackHistory,
  queryClientFallbackHistory,
  recordClientFallbackHistory,
} from './HistorySearchPopover';
import { ScriptoriumPromptEditor } from './ScriptoriumPromptEditor';

vi.mock('../../../transport', () => ({
  discoverySuggest: vi.fn().mockResolvedValue([
    { action_id: 'vox.config.show', completion: 'config show', about: 'show config' },
  ]),
  termComplete: vi.fn().mockResolvedValue(null),
  termHistoryQuery: vi.fn().mockResolvedValue(null),
  termHistoryRecord: vi.fn().mockResolvedValue(true),
  termHistoryClear: vi.fn().mockResolvedValue(true),
}));

describe('ScriptoriumPromptEditor', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    clearClientFallbackHistory();
  });

  it('supports Shift+Enter multi-line editing, incomplete-line continuation on Enter, and full script submission with history recording', async () => {
    const onSubmit = vi.fn();
    render(<ScriptoriumPromptEditor onSubmit={onSubmit} cwd="/repo/vox" />);

    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    expect(textarea.tagName).toBe('TEXTAREA');
    expect(textarea.getAttribute('aria-label')).toBe('console input');
    expect(textarea.rows).toBe(1);
    expect(screen.getByTestId('scriptorium-highlight-layer').tagName).toBe('DIV');
    expect(textarea.style.getPropertyPriority('background-color')).toBe('important');

    // Shift+Enter inserts newline without submitting
    fireEvent.change(textarea, {
      target: { value: 'echo line1', selectionStart: 10, selectionEnd: 10 },
    });
    textarea.setSelectionRange(10, 10);
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true });
    expect(onSubmit).not.toHaveBeenCalled();
    expect(textarea.value).toBe('echo line1\n');

    // Incomplete quote / pipe / backslash inserts newline on plain Enter instead of submitting
    fireEvent.change(textarea, { target: { value: 'echo "hello' } });
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    fireEvent.keyDown(textarea, { key: 'Enter' });
    expect(onSubmit).not.toHaveBeenCalled();
    expect(textarea.value).toBe('echo "hello\n');

    fireEvent.change(textarea, { target: { value: 'ls |' } });
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    fireEvent.keyDown(textarea, { key: 'Enter' });
    expect(onSubmit).not.toHaveBeenCalled();
    expect(textarea.value).toBe('ls |\n');

    fireEvent.change(textarea, { target: { value: 'cargo build \\' } });
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    fireEvent.keyDown(textarea, { key: 'Enter' });
    expect(onSubmit).not.toHaveBeenCalled();
    expect(textarea.value).toBe('cargo build \\\n');

    // Complete multi-line script submits full payload on Enter and records history
    const script = 'cargo check -p vox-cli\ncargo test -p vox-cli';
    fireEvent.change(textarea, { target: { value: script } });
    fireEvent.keyDown(textarea, { key: 'Enter' });

    expect(onSubmit).toHaveBeenCalledWith(script);
    expect(transport.termHistoryRecord).toHaveBeenCalledWith(script, '/repo/vox');
    expect(queryClientFallbackHistory('', '/repo/vox', 10)[0]?.command).toBe(script);
    expect(textarea.value).toBe('');
  });

  it('intercepts multi-line paste without auto-submitting, displays paste safety banner, and clears on Escape', () => {
    const onSubmit = vi.fn();
    render(<ScriptoriumPromptEditor onSubmit={onSubmit} />);

    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    fireEvent.paste(textarea, {
      clipboardData: {
        getData: (type: string) => (type === 'text/plain' ? 'echo one\necho two' : ''),
      },
    });

    expect(onSubmit).not.toHaveBeenCalled();
    expect(textarea.value).toBe('echo one\necho two');

    const banner = screen.getByTestId('term-paste-safety-banner');
    expect(banner.textContent).toContain('Pasted 2 lines into editor - review script and press Enter to execute.');

    fireEvent.keyDown(textarea, { key: 'Escape' });
    expect(textarea.value).toBe('');
    expect(screen.queryByTestId('term-paste-safety-banner')).toBeNull();
  });

  it('recalls history on ArrowUp/ArrowDown with prefix filtering at outer line boundaries and preserves interior multi-line caret movement', async () => {
    recordClientFallbackHistory('git status', '/repo', 0);
    recordClientFallbackHistory('cargo check', '/repo', 0);

    render(<ScriptoriumPromptEditor onSubmit={vi.fn()} cwd="/repo" />);
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;

    // Empty draft cycles all history entries newest -> oldest and restores empty draft on ArrowDown
    fireEvent.keyDown(textarea, { key: 'ArrowUp' });
    expect(textarea.value).toBe('cargo check');

    fireEvent.keyDown(textarea, { key: 'ArrowUp' });
    expect(textarea.value).toBe('git status');

    fireEvent.keyDown(textarea, { key: 'ArrowDown' });
    expect(textarea.value).toBe('cargo check');

    fireEvent.keyDown(textarea, { key: 'ArrowDown' });
    expect(textarea.value).toBe('');

    // Non-empty draft prefix-filters ArrowUp recall and restores exact draft on ArrowDown (WR-04)
    fireEvent.change(textarea, { target: { value: 'git ' } });
    textarea.setSelectionRange(4, 4);
    fireEvent.keyDown(textarea, { key: 'ArrowUp' });
    expect(textarea.value).toBe('git status');

    fireEvent.keyDown(textarea, { key: 'ArrowDown' });
    expect(textarea.value).toBe('git ');

    // On line 2 of a 3-line buffer, ArrowUp does NOT overwrite buffer with history
    const threeLines = 'line1\nline2\nline3';
    fireEvent.change(textarea, { target: { value: threeLines } });
    textarea.setSelectionRange(8, 8); // inside line2
    fireEvent.keyDown(textarea, { key: 'ArrowUp' });
    expect(textarea.value).toBe(threeLines);
  });

  it('opens HistorySearchPopover on Ctrl+R with initialQuery, inserts on Tab, executes on Enter (CR-02), and handles AI Assist pill', async () => {
    recordClientFallbackHistory('vox doctor', '/repo', 0);
    const onSubmit = vi.fn();

    const { rerender } = render(
      <ScriptoriumPromptEditor onSubmit={onSubmit} cwd="/repo" historyOpenRequestSeq={0} />,
    );
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;

    // Ctrl+R opens HistorySearchPopover seeded with current single-line draft; Escape restores prior draft
    fireEvent.change(textarea, { target: { value: 'vox' } });
    fireEvent.keyDown(textarea, { key: 'r', ctrlKey: true });
    expect(await screen.findByTestId('term-history-search')).toBeTruthy();
    expect((screen.getByTestId('term-history-input') as HTMLInputElement).value).toBe('vox');
    // Prompt textarea remains the ONLY role="textbox" even while HistorySearchPopover is open!
    expect(screen.getAllByRole('textbox')).toHaveLength(1);

    fireEvent.keyDown(screen.getByTestId('term-history-input'), { key: 'Escape' });
    expect(screen.queryByTestId('term-history-search')).toBeNull();
    expect(textarea.value).toBe('vox');

    // Tab ('insert') populates textarea without submitting; Enter ('accept') executes via onSubmit (CR-02)
    fireEvent.change(textarea, { target: { value: '' } });
    rerender(<ScriptoriumPromptEditor onSubmit={onSubmit} cwd="/repo" historyOpenRequestSeq={1} />);
    await waitFor(() => expect(screen.getByTestId('term-history-item-0')).toBeTruthy());
    fireEvent.keyDown(screen.getByTestId('term-history-input'), { key: 'Tab' });
    expect(screen.queryByTestId('term-history-search')).toBeNull();
    expect(textarea.value).toBe('vox doctor');
    expect(onSubmit).not.toHaveBeenCalled();

    fireEvent.change(textarea, { target: { value: '' } });
    rerender(<ScriptoriumPromptEditor onSubmit={onSubmit} cwd="/repo" historyOpenRequestSeq={2} />);
    await waitFor(() => expect(screen.getByTestId('term-history-item-0')).toBeTruthy());
    fireEvent.keyDown(screen.getByTestId('term-history-input'), { key: 'Enter' });
    expect(screen.queryByTestId('term-history-search')).toBeNull();
    expect(onSubmit).toHaveBeenCalledWith('vox doctor');
    expect(textarea.value).toBe('');
    onSubmit.mockClear();

    // Natural-language question shows non-hijacking AI Assist pill; clicking prefixes '?' without submitting
    fireEvent.change(textarea, { target: { value: 'how do i list open ports' } });
    const pill = screen.getByTestId('term-ai-assist-pill');
    expect(pill.textContent).toBe('AI Assist');
    expect(screen.getByTestId('console-input-mode-badge').textContent).toBe('shell');

    fireEvent.click(pill);
    expect(onSubmit).not.toHaveBeenCalled();
    expect(textarea.value).toBe('?how do i list open ports');
    expect(screen.getByTestId('console-input-mode-badge').textContent).toBe('agent');
  });

  it('completes @ macros and /cl -> /clear  via Tab', async () => {
    render(<ScriptoriumPromptEditor onSubmit={vi.fn()} />);
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;

    fireEvent.change(textarea, { target: { value: '@' } });
    await waitFor(() => expect(screen.getByTestId('intellisense-popover')).toBeTruthy());
    expect(screen.getByTestId('intellisense-popover').textContent).toContain('@file:');
    expect(screen.getByTestId('intellisense-popover').textContent).toContain('@term:last');

    fireEvent.change(textarea, { target: { value: '/cl' } });
    fireEvent.keyDown(textarea, { key: 'Tab' });
    expect(textarea.value).toBe('/clear ');
  });
});
