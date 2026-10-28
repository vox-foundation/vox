// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

import {
  discoverySuggest,
  ptyWrite,
  termSessionOpen,
  termSessionList,
  termSessionAttach,
  termSessionInput,
  termSessionSubmit,
  termSessionResize,
  termSessionClose,
} from './transport';

describe('console transport', () => {
  beforeEach(() => invokeMock.mockReset());

  it('discoverySuggest forwards typed + limit to invoke', async () => {
    invokeMock.mockResolvedValue([
      { action_id: 'vox.config.show', completion: 'config show', about: '' },
    ]);
    const out = await discoverySuggest('config', 5);
    expect(invokeMock).toHaveBeenCalledWith('discovery_suggest', { typed: 'config', limit: 5 });
    expect(out[0].action_id).toBe('vox.config.show');
  });

  it('ptyWrite forwards tab id + data', async () => {
    invokeMock.mockResolvedValue(undefined);
    await ptyWrite('tab-1', 'ls\n');
    expect(invokeMock).toHaveBeenCalledWith('pty_write', { tabId: 'tab-1', data: 'ls\n' });
  });

  it('termSession* wrappers forward SessionHost IPC commands', async () => {
    invokeMock.mockResolvedValueOnce('s-1');
    const sid = await termSessionOpen(100, 30);
    expect(sid).toBe('s-1');
    expect(invokeMock).toHaveBeenCalledWith('term_session_open', { cols: 100, rows: 30 });

    invokeMock.mockResolvedValueOnce([{ id: 's-1', blocks: 0 }]);
    const list = await termSessionList();
    expect(list).toEqual([{ id: 's-1', blocks: 0 }]);
    expect(invokeMock).toHaveBeenCalledWith('term_session_list');

    invokeMock.mockResolvedValueOnce({
      session_id: 's-1',
      blocks: [],
      replay_utf8: '$ ',
      seq: 2,
    });
    const att = await termSessionAttach('s-1');
    expect(att.replay_utf8).toBe('$ ');
    expect(invokeMock).toHaveBeenCalledWith('term_session_attach', { sessionId: 's-1' });

    invokeMock.mockResolvedValueOnce(undefined);
    await termSessionInput('s-1', 'pwd\n');
    expect(invokeMock).toHaveBeenCalledWith('term_session_input', {
      sessionId: 's-1',
      data: 'pwd\n',
    });

    invokeMock.mockResolvedValueOnce(1);
    const bid = await termSessionSubmit('s-1', 'echo hi');
    expect(bid).toBe(1);
    expect(invokeMock).toHaveBeenCalledWith('term_session_submit', {
      sessionId: 's-1',
      line: 'echo hi',
    });

    invokeMock.mockResolvedValueOnce(undefined);
    await termSessionResize('s-1', 120, 40);
    expect(invokeMock).toHaveBeenCalledWith('term_session_resize', {
      sessionId: 's-1',
      cols: 120,
      rows: 40,
    });

    invokeMock.mockResolvedValueOnce(undefined);
    await termSessionClose('s-1');
    expect(invokeMock).toHaveBeenCalledWith('term_session_close', { sessionId: 's-1' });
  });
});
