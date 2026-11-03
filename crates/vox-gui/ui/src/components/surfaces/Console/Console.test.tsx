// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { LanguageProvider } from '../../../hooks/useLanguage';

vi.mock('@msgpack/msgpack', () => ({
  decode: vi.fn(() => ({ agents: [] })),
}));

vi.mock('../../../transport', () => ({
  discoverySuggest: vi.fn().mockResolvedValue([]),
  discoveryHelp: vi.fn().mockResolvedValue(null),
  discoveryRecord: vi.fn().mockResolvedValue(undefined),
  termSessionOpen: vi.fn().mockResolvedValue('s-1'),
  termSessionList: vi.fn().mockResolvedValue([{ id: 's-1', blocks: 0 }]),
  termSessionAttach: vi.fn().mockResolvedValue({
    session_id: 's-1',
    blocks: [],
    replay_utf8: '',
    seq: 0,
  }),
  termSessionInput: vi.fn().mockResolvedValue(undefined),
  termSessionSubmit: vi.fn().mockResolvedValue(1),
  termSessionResize: vi.fn().mockResolvedValue(undefined),
  termSessionClose: vi.fn().mockResolvedValue(undefined),
  listenTermOutput: vi.fn().mockResolvedValue(() => {}),
  listenTermEvent: vi.fn().mockResolvedValue(() => {}),
  listenOrchStatus: vi.fn().mockRejectedValue(new Error('not in tauri')),
  sendToAgent: vi.fn().mockResolvedValue('msg-1'),
  voxTransport: {
    getOrchestratorStatusBin: vi.fn().mockResolvedValue(new Uint8Array([0x80])),
    getOrchestratorVersionMismatch: vi.fn().mockResolvedValue(null),
  },
}));
vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    parser = { registerOscHandler: () => ({ dispose() {} }) };
    buffer = { active: { cursorY: 0, baseY: 0 } };
    open() {}
    write() {}
    onData() {}
    dispose() {}
    loadAddon() {}
    registerMarker() {
      return { dispose() {} };
    }
    registerDecoration() {
      return { onRender() {}, dispose() {} };
    }
    get cols() {
      return 80;
    }
    get rows() {
      return 24;
    }
  },
}));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }));

import { Console } from './Console';

function renderConsole(props: React.ComponentProps<typeof Console>) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <LanguageProvider>
      <QueryClientProvider client={client}>
        <Console {...props} />
      </QueryClientProvider>
    </LanguageProvider>,
  );
}

describe('Console', () => {
  beforeEach(() => cleanup());

  it('renders the terminal, block stream, input, and discovery rail', async () => {
    renderConsole({ pushToast: vi.fn() });
    expect(screen.getByLabelText('terminal')).toBeTruthy();
    expect(screen.getByTestId('block-stream-view')).toBeTruthy();
    expect(screen.getByRole('textbox')).toBeTruthy();
    await waitFor(() => expect(screen.getByLabelText('discovery')).toBeTruthy());
  });

  it('submitting a line in the input forwards it to termSessionSubmit', async () => {
    const t = await import('../../../transport');
    renderConsole({ pushToast: vi.fn() });
    await waitFor(() => expect(t.termSessionAttach).toHaveBeenCalledWith('s-1'));
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'echo hi' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(t.termSessionSubmit).toHaveBeenCalledWith('s-1', 'echo hi'));
  });

  it('gives toolbar controls explicit button type', () => {
    renderConsole({ pushToast: vi.fn() });
    expect(screen.getByText('copy last block').getAttribute('type')).toBe('button');
    expect(screen.getByText('send to agent').getAttribute('type')).toBe('button');
  });

  it('renders session tabs from termSessionList and switches activeSessionId on click', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.termSessionList).mockResolvedValueOnce([
      { id: 's-1', blocks: 2 },
      { id: 's-2', blocks: 1 },
    ]);
    renderConsole({ pushToast: vi.fn() });
    await waitFor(() => expect(screen.getByTestId('term-session-tab-s-1')).toBeTruthy());
    const tab2 = screen.getByTestId('term-session-tab-s-2');
    fireEvent.click(tab2);
    await waitFor(() => expect(t.termSessionAttach).toHaveBeenCalledWith('s-2'));
  });

  it('copies vox term --attach <id> to clipboard when clicking continue in vox term', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    const pushToast = vi.fn();
    renderConsole({ pushToast });
    await waitFor(() => expect(screen.getByTestId('term-session-tab-s-1')).toBeTruthy());
    fireEvent.click(screen.getByText('continue in vox term'));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('vox term --attach s-1'));
    expect(pushToast).toHaveBeenCalledWith(
      expect.objectContaining({ tone: 'ok', body: 'vox term --attach s-1' }),
    );
  });

  it('cycles control mode Manual -> Co-Pilot -> Autopilot and drops Autopilot to Manual on user submit (50-02)', async () => {
    renderConsole({ pushToast: vi.fn() });
    const secSelect = screen.getByTestId('term-security-level-select') as HTMLSelectElement;
    expect(secSelect.value).toBe('l2_safe_exec');
    fireEvent.change(secSelect, { target: { value: 'l1_read_only' } });
    expect(secSelect.value).toBe('l1_read_only');

    const pill = screen.getByTestId('term-control-mode-pill');
    expect(pill.textContent).toBe('Manual');
    fireEvent.click(pill);
    expect(pill.textContent).toBe('Co-Pilot');
    fireEvent.click(pill);
    expect(pill.textContent).toBe('Autopilot');

    // Submitting a user line drops Autopilot -> Manual
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'ls' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(pill.textContent).toBe('Manual');
  });

  it('renders IntelliSense popover in console input when typing commands (51-04)', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.discoverySuggest).mockResolvedValueOnce([
      { action_id: 'vox.config.show', completion: 'config show', about: 'show config' },
    ]);
    renderConsole({ pushToast: vi.fn() });
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'vox config' } });

    await waitFor(() => expect(screen.getByTestId('intellisense-popover')).toBeTruthy());
    expect(screen.getByTestId('intellisense-tab-all')).toBeTruthy();
    expect(screen.getByTestId('intellisense-footer')).toBeTruthy();
  });
});

