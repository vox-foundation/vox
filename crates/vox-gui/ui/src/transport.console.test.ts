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
  termHistoryRecord,
  termHistoryQuery,
  termHistoryClear,
  termComplete,
  termRunbooksList,
  termSessionVoxify,
  termAgentPrompt,
  termAgentExplainFix,
  termAgentCancel,
  termSlashExec,
  termBlockFeedback,
  listTerminalPendingApprovals,
  resolveTerminalApproval,
  voxTransport,
} from './transport';
import { parsePendingApprovals } from './lib/mcpToolResult';
import {
  consumePendingOpenRunbook,
  onOpenRunbook,
  requestOpenRunbook,
  resetTerminalBridgeForTests,
} from './lib/terminalBridge';

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

  it('termSessionAttach preserves 1-arg payload and sends afterSeq only when non-null', async () => {
    const mockSnap = { session_id: 's-1', blocks: [], replay_utf8: '', seq: 10 };
    invokeMock.mockResolvedValue(mockSnap);

    await termSessionAttach('s-1');
    expect(invokeMock).toHaveBeenLastCalledWith('term_session_attach', { sessionId: 's-1' });

    await termSessionAttach('s-1', 42);
    expect(invokeMock).toHaveBeenLastCalledWith('term_session_attach', {
      sessionId: 's-1',
      afterSeq: 42,
    });

    await termSessionAttach('s-1', null);
    expect(invokeMock).toHaveBeenLastCalledWith('term_session_attach', { sessionId: 's-1' });

    await termSessionAttach('s-1', undefined);
    expect(invokeMock).toHaveBeenLastCalledWith('term_session_attach', { sessionId: 's-1' });
  });

  it('termHistory*, termComplete, termRunbooksList, and termSessionVoxify invoke expected IPC endpoints and keys', async () => {
    invokeMock.mockResolvedValueOnce(true);
    const recorded = await termHistoryRecord('cargo check', '/repo', 0);
    expect(recorded).toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith('term_history_record', {
      command: 'cargo check',
      cwd: '/repo',
      exitCode: 0,
    });

    invokeMock.mockResolvedValueOnce([
      { command: 'cargo check', cwd: '/repo', exit_code: 0, ts_ms: 1700000000000 },
    ]);
    const entries = await termHistoryQuery('cargo', '/repo', 25);
    expect(entries).toHaveLength(1);
    expect(invokeMock).toHaveBeenLastCalledWith('term_history_query', {
      query: 'cargo',
      cwd: '/repo',
      limit: 25,
    });

    invokeMock.mockResolvedValueOnce(undefined);
    const cleared = await termHistoryClear();
    expect(cleared).toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith('term_history_clear');

    invokeMock.mockResolvedValueOnce([
      {
        label: 'check',
        insert_text: 'check',
        kind: 'subcommand',
        detail: 'Check local package',
        score: 92,
        replace_start: 6,
        replace_end: 8,
      },
    ]);
    const completions = await termComplete({
      line: 'cargo ch',
      cursor: 8,
      cwd: '/repo',
      shellMode: 'default',
      limit: 16,
    });
    expect(completions?.[0]?.label).toBe('check');
    expect(invokeMock).toHaveBeenLastCalledWith('term_complete', {
      line: 'cargo ch',
      cursor: 8,
      cwd: '/repo',
      shellMode: 'default',
      limit: 16,
    });

    invokeMock.mockResolvedValueOnce([
      {
        id: 'scripts/setup.vox',
        title: 'Setup',
        description: 'Bootstrap repo',
        path: 'scripts/setup.vox',
        kind: 'vox',
        command_template: 'vox run scripts/setup.vox',
        params: [],
        tags: [],
      },
    ]);
    const runbooks = await voxTransport.termRunbooksList('/repo');
    expect(runbooks?.[0]?.id).toBe('scripts/setup.vox');
    expect(invokeMock).toHaveBeenLastCalledWith('term_runbooks_list', { cwd: '/repo' });

    invokeMock.mockResolvedValueOnce(undefined);
    const voxified = await termSessionVoxify('s-1', 'zsh');
    expect(voxified).toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith('term_session_voxify', {
      sessionId: 's-1',
      shellHint: 'zsh',
    });
  });

  it('terminalBridge publishes and consumes RunbookLaunchRequest payloads across Omnibar and Console', () => {
    resetTerminalBridgeForTests();
    const seen: unknown[] = [];
    const unsub = onOpenRunbook((req) => seen.push(req));

    requestOpenRunbook({ runbookId: 'scripts/setup.vox', mode: 'open_rail' });
    expect(seen).toEqual([{ runbookId: 'scripts/setup.vox', mode: 'open_rail' }]);
    expect(consumePendingOpenRunbook()).toEqual({
      runbookId: 'scripts/setup.vox',
      mode: 'open_rail',
    });
    expect(consumePendingOpenRunbook()).toBeNull();

    unsub();
    requestOpenRunbook({ command: 'vox doctor', mode: 'run' });
    expect(seen).toHaveLength(1);
    expect(consumePendingOpenRunbook()).toEqual({ command: 'vox doctor', mode: 'run' });
  });

  it('parsePendingApprovals preserves optional terminal fields and stays backward-compatible', () => {
    const parsed = parsePendingApprovals({
      tool: 'vox_pending_approvals',
      is_error: false,
      result: {
        success: true,
        data: {
          approvals: [
            {
              approval_id: 'AP-000001',
              tool: 'vox_run_shell',
              summary: 'rm -rf build',
              requested_at_ms: 1000,
            },
            {
              approval_id: 'AP-000002',
              tool: 'vox_terminal_exec',
              summary: 'git push --force',
              requested_at_ms: 2000,
              floor_category: 'git_destructive',
              session_id: 's-1',
              command: 'git push --force',
              control_mode: 'co_pilot',
              credential_prompt: true,
            },
          ],
        },
      },
    });
    expect(parsed).toEqual([
      {
        approval_id: 'AP-000001',
        tool: 'vox_run_shell',
        summary: 'rm -rf build',
        requested_at_ms: 1000,
      },
      {
        approval_id: 'AP-000002',
        tool: 'vox_terminal_exec',
        summary: 'git push --force',
        requested_at_ms: 2000,
        floor_category: 'git_destructive',
        session_id: 's-1',
        command: 'git push --force',
        control_mode: 'co_pilot',
        credential_prompt: true,
      },
    ]);
  });

  it('termAgent*, termSlashExec, termBlockFeedback, and terminal approval helpers invoke exact contracts', async () => {
    invokeMock.mockResolvedValueOnce({
      block_id: 11,
      status: 'ok',
      proposed_command: 'cargo check',
      provider: 'ollama',
      model: 'mens-local',
    });
    const promptTurn = await termAgentPrompt('s-1', 'check workspace');
    expect(promptTurn.block_id).toBe(11);
    expect(promptTurn.proposed_command).toBe('cargo check');
    expect(invokeMock).toHaveBeenLastCalledWith('term_agent_prompt', {
      sessionId: 's-1',
      prompt: 'check workspace',
    });

    invokeMock.mockResolvedValueOnce({
      block_id: 12,
      status: 'ok',
      proposed_command: 'cargo test -p vox-gui',
      provider: 'ollama',
      model: 'mens-local',
    });
    const fixTurn = await termAgentExplainFix('s-1', 7, 'fix');
    expect(fixTurn.block_id).toBe(12);
    expect(invokeMock).toHaveBeenLastCalledWith('term_agent_explain_fix', {
      sessionId: 's-1',
      blockId: 7,
      mode: 'fix',
    });

    invokeMock.mockResolvedValueOnce(true);
    const cancelled = await termAgentCancel('s-1', 12);
    expect(cancelled).toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith('term_agent_cancel', {
      sessionId: 's-1',
      blockId: 12,
    });

    invokeMock.mockResolvedValueOnce({
      command: '/budget',
      block_id: 13,
      delegated_to: null,
      output: '[budget]\ncontext_tokens: 100 / 16000',
    });
    const slash = await termSlashExec('s-1', '/budget');
    expect(slash.command).toBe('/budget');
    expect(invokeMock).toHaveBeenLastCalledWith('term_slash_exec', {
      sessionId: 's-1',
      rawLine: '/budget',
    });

    invokeMock.mockResolvedValueOnce({
      recorded: true,
      session_id: 's-1',
      block_id: 12,
      verdict: 'corrected',
    });
    const fb = await termBlockFeedback('s-1', 12, 'corrected', 'cargo test -p vox-gui --lib');
    expect(fb.recorded).toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith('term_block_feedback', {
      sessionId: 's-1',
      blockId: 12,
      verdict: 'corrected',
      correctedCommand: 'cargo test -p vox-gui --lib',
      originalCommand: null,
    });

    invokeMock.mockResolvedValueOnce({
      recorded: true,
      session_id: 's-1',
      block_id: 0,
      verdict: 'corrected',
    });
    const fbOrig = await termBlockFeedback(
      's-1',
      0,
      'corrected',
      'cargo test -p vox-gui',
      'cargo test --all',
    );
    expect(fbOrig.recorded).toBe(true);
    expect(invokeMock).toHaveBeenLastCalledWith('term_block_feedback', {
      sessionId: 's-1',
      blockId: 0,
      verdict: 'corrected',
      correctedCommand: 'cargo test -p vox-gui',
      originalCommand: 'cargo test --all',
    });

    invokeMock.mockResolvedValueOnce({
      is_error: false,
      result: {
        success: true,
        data: {
          approvals: [
            {
              approval_id: 'AP-1',
              tool: 'vox_run_shell',
              summary: 'non-terminal',
              requested_at_ms: 10,
            },
            {
              approval_id: 'AP-2',
              tool: 'vox_terminal_exec',
              summary: 'cargo test',
              requested_at_ms: 20,
              session_id: 's-1',
              command: 'cargo test',
            },
            {
              approval_id: 'AP-3',
              tool: 'vox_terminal_send_input',
              summary: 'other session',
              requested_at_ms: 30,
              session_id: 's-2',
            },
          ],
        },
      },
    });
    const pending = await listTerminalPendingApprovals('s-1');
    expect(pending).toHaveLength(1);
    expect(pending[0]?.approval_id).toBe('AP-2');
    expect(invokeMock).toHaveBeenLastCalledWith(
      'invoke_mcp_tool',
      expect.objectContaining({ tool: 'vox_pending_approvals', args: {} }),
    );

    invokeMock.mockResolvedValueOnce({ is_error: false, result: { success: true } });
    await resolveTerminalApproval({
      approvalId: 'AP-2',
      outcome: 'modified',
      modifiedCommand: 'cargo test -p vox-terminal-core',
      steeringNote: 'scope to single crate',
      allowPattern: 'cargo test*',
    });
    expect(invokeMock).toHaveBeenLastCalledWith(
      'invoke_mcp_tool',
      expect.objectContaining({
        tool: 'vox_resolve_approval',
        args: {
          approval_id: 'AP-2',
          outcome: 'modified',
          modified_command: 'cargo test -p vox-terminal-core',
          steering_note: 'scope to single crate',
          allow_pattern: 'cargo test*',
        },
      }),
    );
  });
});
