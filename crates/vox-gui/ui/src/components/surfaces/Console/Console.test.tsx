import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor, act } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { LanguageProvider } from '../../../hooks/useLanguage';
import { requestOpenRunbook, resetTerminalBridgeForTests } from '../../../lib/terminalBridge';
import { detectSubshellShellHint } from './useConsoleWarpActions';

vi.mock('@msgpack/msgpack', () => ({
  decode: vi.fn(() => ({ agents: [] })),
}));

vi.mock('../../../transport', () => ({
  discoverySuggest: vi.fn().mockResolvedValue([]), discoveryHelp: vi.fn().mockResolvedValue(null), discoveryRecord: vi.fn().mockResolvedValue(undefined),
  termComplete: vi.fn().mockResolvedValue([]), termHistoryQuery: vi.fn().mockResolvedValue([]), termHistoryRecord: vi.fn().mockResolvedValue(undefined), termHistoryClear: vi.fn().mockResolvedValue(undefined),
  termRunbooksList: vi.fn().mockResolvedValue([]), termSessionVoxify: vi.fn().mockResolvedValue(true),
  termSessionOpen: vi.fn().mockResolvedValue('s-1'), termSessionList: vi.fn().mockResolvedValue([{ id: 's-1', blocks: 0 }]),
  termSessionAttach: vi.fn().mockResolvedValue({ session_id: 's-1', blocks: [], replay_utf8: '', seq: 0 }),
  termSessionInput: vi.fn().mockResolvedValue(undefined), termSessionSubmit: vi.fn().mockResolvedValue(1), termSessionResize: vi.fn().mockResolvedValue(undefined), termSessionClose: vi.fn().mockResolvedValue(undefined),
  termSessionSetSecurity: vi.fn().mockResolvedValue(undefined), termSessionSetControl: vi.fn().mockResolvedValue(undefined), termSessionPin: vi.fn().mockResolvedValue(undefined), termSessionPromoteBg: vi.fn().mockResolvedValue(false),
  termAgentPrompt: vi.fn().mockResolvedValue({ block_id: 99, status: 'ok', proposed_command: null, provider: 'ollama', model: 'mens-local' }),
  termAgentExplainFix: vi.fn().mockResolvedValue({ block_id: 100, status: 'ok', proposed_command: 'cargo check', provider: 'ollama', model: 'mens-local' }),
  termAgentCancel: vi.fn().mockResolvedValue(true),
  termSlashExec: vi.fn().mockResolvedValue({ command: '/budget', block_id: 101, delegated_to: null, output: '[budget]' }),
  termBlockFeedback: vi.fn().mockResolvedValue({ recorded: true, session_id: 's-1', block_id: 1, verdict: 'accepted' }),
  listTerminalPendingApprovals: vi.fn().mockResolvedValue([]),
  resolveTerminalApproval: vi.fn().mockResolvedValue({ is_error: false, result: { success: true, data: { resolved: true } } }),
  listenTermOutput: vi.fn().mockResolvedValue(() => {}), listenTermEvent: vi.fn().mockResolvedValue(() => {}), listenOrchStatus: vi.fn().mockRejectedValue(new Error('not in tauri')),
  sendToAgent: vi.fn().mockResolvedValue('msg-1'),
  voxTransport: { getOrchestratorStatusBin: vi.fn().mockResolvedValue(new Uint8Array([0x80])), getOrchestratorVersionMismatch: vi.fn().mockResolvedValue(null), openLocator: vi.fn().mockResolvedValue({ ok: true }) },
}));
vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    parser = { registerOscHandler: () => ({ dispose() {} }) };
    buffer = { active: { cursorY: 0, baseY: 0, length: 0, getLine: () => undefined } };
    open() {} write() {} reset() {} focus() {} onData() {} dispose() {} loadAddon() {}
    registerMarker() { return { dispose() {} }; }
    registerDecoration() { return { onRender() {}, dispose() {} }; }
    get cols() { return 80; }
    get rows() { return 24; }
  },
}));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }));

import { Console } from './Console';
import { resetSessionPaneAttachRegistryForTests } from './TerminalSessionPane';

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
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    resetSessionPaneAttachRegistryForTests();
    resetTerminalBridgeForTests();
  });

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

  it('cycles control mode Co-Pilot -> Autopilot -> Manual and drops Autopilot to Manual on user input or submit (50-02)', async () => {
    renderConsole({ pushToast: vi.fn() });
    const secSelect = screen.getByTestId('term-security-level-select');
    expect(secSelect).toHaveTextContent('L3 Workspace Write');
    fireEvent.click(secSelect);
    fireEvent.click(screen.getByRole('option', { name: 'L1 Read-Only' }));
    expect(secSelect).toHaveTextContent('L1 Read-Only');

    const pill = screen.getByTestId('term-control-mode-pill');
    expect(pill.textContent).toBe('Co-Pilot');
    fireEvent.click(pill);
    expect(pill.textContent).toBe('Autopilot');

    // Typing into InputEditor preempts Autopilot -> Manual immediately (CR-05)
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'ls' } });
    expect(pill.textContent).toBe('Manual');

    fireEvent.click(pill);
    expect(pill.textContent).toBe('Co-Pilot');
    fireEvent.click(pill);
    expect(pill.textContent).toBe('Autopilot');
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

  it('splits panes via Split Pane Right, renders live cwd and git_branch session tabs, and collapses on Close Terminal Pane', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.termSessionList).mockResolvedValueOnce([
      { id: 's-1', blocks: 1, cwd: '/Users/dev/vox', git_branch: 'feat/split-panes' },
    ]);
    vi.mocked(t.termSessionOpen).mockResolvedValueOnce('s-2');
    vi.mocked(t.termSessionAttach).mockImplementation(async (sid: string) => ({
      session_id: sid,
      blocks: [],
      replay_utf8: '',
      seq: 0,
      cwd: sid === 's-1' ? '/Users/dev/vox' : '/Users/dev/vox/crates',
      git_branch: sid === 's-1' ? 'feat/split-panes' : 'main',
    }));

    const { container } = renderConsole({ pushToast: vi.fn() });
    const tab1 = await screen.findByTestId('term-session-tab-s-1');
    expect(tab1.textContent).toContain('vox (');
    expect(tab1.textContent).toContain('feat/split-panes');

    fireEvent.click(screen.getByTestId('term-split-right'));
    await waitFor(() => expect(t.termSessionOpen).toHaveBeenCalledWith(80, 24));
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid^="console-split-leaf-"]').length).toBe(2),
    );
    expect(screen.getByRole('separator')).toBeTruthy();

    fireEvent.click(screen.getByTestId('term-close-pane'));
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid^="console-split-leaf-"]').length).toBe(1),
    );
  });

  it('syncs security level and control mode wire values to backend and resets Autopilot to manual on submit', async () => {
    const t = await import('../../../transport');
    renderConsole({ pushToast: vi.fn() });
    await waitFor(() => expect(screen.getByTestId('term-session-tab-s-1')).toBeTruthy());

    const secSelect = screen.getByTestId('term-security-level-select');
    fireEvent.click(secSelect);
    fireEvent.click(screen.getByRole('option', { name: 'L1 Read-Only' }));
    expect(t.termSessionSetSecurity).toHaveBeenCalledWith('s-1', 'l1_read_only');

    fireEvent.click(secSelect);
    fireEvent.click(screen.getByRole('option', { name: 'L3 Workspace Write' }));
    expect(t.termSessionSetSecurity).toHaveBeenCalledWith('s-1', 'l3_structured_default');

    const pill = screen.getByTestId('term-control-mode-pill');
    fireEvent.click(pill);
    expect(t.termSessionSetControl).toHaveBeenCalledWith('s-1', 'autopilot');

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'pwd' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(t.termSessionSetControl).toHaveBeenCalledWith('s-1', 'manual');
  });

  it('guards single-key shortcuts so Ctrl+A / Ctrl+R and button focus never trigger SendToAgent or command re-run (UI BLOCKER #1)', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.termSessionAttach).mockResolvedValueOnce({
      session_id: 's-1', replay_utf8: '', seq: 1,
      blocks: [{
        id: 1, kind: 'Shell', status: 'Ok', origin: { kind: 'user' }, input: 'rm -rf build',
        output: [{ stream: 'Stdout', text: 'done\n', ts_ms: 1 }], exit_code: 0, started_ms: 1, ended_ms: 2,
      }],
    });

    renderConsole({ pushToast: vi.fn() });
    const card = await screen.findByTestId('term-block-card');

    // Ctrl+A / Ctrl+R / Meta+A / Meta+R inside Console must NOT open SendToAgent or re-run
    fireEvent.keyDown(card, { key: 'a', ctrlKey: true });
    fireEvent.keyDown(card, { key: 'a', metaKey: true });
    fireEvent.keyDown(card, { key: 'r', ctrlKey: true });
    fireEvent.keyDown(card, { key: 'r', metaKey: true });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(t.termSessionSubmit).not.toHaveBeenCalled();

    // Unmodified 'a' / 'r' while focused on a button or combobox must NOT fire block shortcuts
    const copyBtn = screen.getByText('copy last block');
    copyBtn.focus();
    fireEvent.keyDown(copyBtn, { key: 'a' });
    fireEvent.keyDown(copyBtn, { key: 'r' });
    expect(t.termSessionSubmit).not.toHaveBeenCalled();
  });

  it('isolates per-session state across split panes and focuses an existing leaf when selecting an already-open session tab', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.termSessionList).mockResolvedValueOnce([
      { id: 's-1', blocks: 1, security_level: 'l1_read_only', control_mode: 'manual' },
    ]);
    vi.mocked(t.termSessionOpen).mockResolvedValueOnce('s-2');
    vi.mocked(t.termSessionAttach).mockImplementation(async (sid: string) => ({
      session_id: sid, blocks: [], replay_utf8: '', seq: 0,
    }));

    const { container } = renderConsole({ pushToast: vi.fn() });
    await screen.findByTestId('term-session-tab-s-1');
    expect(screen.getByTestId('term-security-level-select')).toHaveTextContent('L1 Read-Only');
    expect(screen.getByTestId('term-control-mode-pill').textContent).toBe('Manual');

    // Split right creates pane-2 with s-2 using backend defaults (L3 Workspace Write / Co-Pilot)
    fireEvent.click(screen.getByTestId('term-split-right'));
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid^="console-split-leaf-"]').length).toBe(2),
    );
    expect(screen.getByTestId('term-security-level-select')).toHaveTextContent('L3 Workspace Write');
    expect(screen.getByTestId('term-control-mode-pill').textContent).toBe('Co-Pilot');

    // Clicking tab s-1 while pane-2 is active focuses existing pane-1 instead of binding both leaves to s-1
    fireEvent.click(screen.getByTestId('term-session-tab-s-1'));
    expect(screen.getByTestId('console-split-leaf-pane-1').getAttribute('data-active-pane')).toBe('true');
    expect(screen.getByTestId('term-security-level-select')).toHaveTextContent('L1 Read-Only');
    expect(screen.getByTestId('term-control-mode-pill').textContent).toBe('Manual');
  });

  it('opens HistorySearchPopover (role=region, not dialog) on Ctrl+R and focuses textarea on Cmd+J while keeping textbox unique (444-04)', async () => {
    renderConsole({ pushToast: vi.fn() });
    const textarea = screen.getByRole('textbox');
    expect(textarea.tagName).toBe('TEXTAREA');
    expect(textarea.getAttribute('aria-label')).toBe('console input');

    fireEvent.keyDown(window, { key: 'j', metaKey: true });
    expect(document.activeElement).toBe(textarea);

    fireEvent.keyDown(window, { key: 'r', ctrlKey: true });
    const historyRegion = await screen.findByTestId('term-history-search');
    expect(historyRegion.getAttribute('role')).toBe('region');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByRole('textbox')).toBe(textarea);
  });

  it('supports multi-line Shift+Enter submission, Runbook Insert/Run from DiscoveryRail, and requestOpenRunbook bridge (444-04)', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.termRunbooksList).mockResolvedValue([
      {
        id: 'rb-1',
        title: 'Setup Workspace',
        description: 'Install hooks',
        path: 'scripts/setup.vox',
        kind: 'vox',
        command_template: 'vox run scripts/setup.vox --env {{env}}',
        params: [{ name: 'env', default_value: 'dev', options: [] }],
        tags: ['setup'],
      },
    ]);
    renderConsole({ pushToast: vi.fn() });
    await waitFor(() => expect(t.termSessionAttach).toHaveBeenCalledWith('s-1'));

    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'echo line1', selectionStart: 10, selectionEnd: 10 } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true });
    expect(textarea.value).toBe('echo line1\n');
    expect(t.termSessionSubmit).not.toHaveBeenCalled();
    fireEvent.change(textarea, { target: { value: 'echo line1\necho line2' } });
    fireEvent.keyDown(textarea, { key: 'Enter' });
    await waitFor(() => expect(t.termSessionSubmit).toHaveBeenCalledWith('s-1', 'echo line1\necho line2'));

    fireEvent.click(screen.getByTestId('discovery-tab-runbooks'));
    await screen.findByTestId('term-runbook-card-rb-1');
    fireEvent.click(screen.getByTestId('term-runbook-insert'));
    expect(textarea.value).toBe('vox run scripts/setup.vox --env dev');
    expect(t.termSessionSubmit).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId('term-runbook-run'));
    await waitFor(() =>
      expect(t.termSessionSubmit).toHaveBeenCalledWith('s-1', 'vox run scripts/setup.vox --env dev'),
    );

    act(() => {
      requestOpenRunbook({ runbookId: 'rb-1', command: 'vox run scripts/setup.vox', mode: 'insert' });
    });
    await waitFor(() => expect(textarea.value).toBe('vox run scripts/setup.vox'));
  });

  it('invokes termSessionVoxify on Voxify Subshell click, disables until next subshell, and sanitizes errors in pushToast (444-04)', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.termSessionAttach).mockResolvedValueOnce({
      session_id: 's-1',
      blocks: [
        {
          id: 7,
          kind: 'Shell',
          status: 'Running',
          origin: { kind: 'user' },
          input: 'zsh',
          output: [],
          started_ms: 10,
        },
      ],
      replay_utf8: '',
      seq: 1,
    });
    const pushToast = vi.fn();
    renderConsole({ pushToast });

    const voxifyBtn = await screen.findByTestId('term-voxify-subshell');
    expect(voxifyBtn.textContent).toContain('Voxify Subshell');
    await waitFor(() => expect(screen.getByTestId('term-subshell-voxify-banner')).toBeTruthy());

    fireEvent.click(voxifyBtn);
    await waitFor(() => expect(t.termSessionVoxify).toHaveBeenCalledWith('s-1', 'zsh'));
    await waitFor(() => expect(voxifyBtn.textContent).toContain('Subshell Voxified'));
    expect((voxifyBtn as HTMLButtonElement).disabled).toBe(false);
    expect(screen.queryByTestId('term-subshell-voxify-banner')).toBeNull();

    const textarea = screen.getByLabelText('console input');
    fireEvent.change(textarea, { target: { value: 'bash' } });
    fireEvent.keyDown(textarea, { key: 'Enter' });
    await waitFor(() => expect(voxifyBtn.textContent).toContain('Voxify Subshell'));

    vi.mocked(t.termSessionVoxify).mockRejectedValueOnce(new Error('window.__TAURI_INTERNALS__ broken'));
    fireEvent.click(voxifyBtn);
    await waitFor(() =>
      expect(pushToast).toHaveBeenCalledWith(
        expect.objectContaining({
          tone: 'warn',
          title: 'Voxify Subshell failed',
          cause: 'backend-error',
        }),
      ),
    );
    const toastArg = pushToast.mock.calls.at(-1)?.[0] as { body: string };
    expect(toastArg.body).not.toContain('__TAURI_INTERNALS__');
  });

  it('detectSubshellShellHint aligns with voxify.rs for env, sudo, docker, and kubectl exec', () => {
    expect(detectSubshellShellHint('env FOO=1 BAR=2 zsh -l')).toBe('zsh');
    expect(detectSubshellShellHint('sudo -u deploy -i bash')).toBe('bash');
    expect(detectSubshellShellHint('docker exec -it -u root app-1 pwsh')).toBe('pwsh');
    expect(detectSubshellShellHint('kubectl exec --stdin --tty pod-1 -c api fish')).toBe('fish');
    expect(detectSubshellShellHint('cargo test -p vox-terminal-core')).toBeNull();
  });

  it('keeps textbox unique on mount and routes ? / /slash / agent-mode / shell lines accurately (445-04)', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.termSessionAttach).mockResolvedValueOnce({
      session_id: 's-1', replay_utf8: '', seq: 1,
      blocks: [{ id: 5, kind: 'Shell', status: 'Ok', origin: { kind: 'user' }, input: 'pwd', output: [{ stream: 'Stdout', text: '/repo\n', ts_ms: 10 }], exit_code: 0, started_ms: 10, ended_ms: 15 }],
    });
    renderConsole({ pushToast: vi.fn() });
    await waitFor(() => expect(t.termSessionAttach).toHaveBeenCalledWith('s-1'));
    const input = screen.getByRole('textbox') as HTMLTextAreaElement;
    expect(input.getAttribute('aria-label')).toBe('console input');

    fireEvent.change(input, { target: { value: '? show listening ports' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(t.termAgentPrompt).toHaveBeenCalledWith('s-1', 'show listening ports'));
    expect(t.termSessionSubmit).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: '/budget' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(t.termSlashExec).toHaveBeenCalledWith('s-1', '/budget'));

    fireEvent.change(input, { target: { value: '/compact' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(t.termSlashExec).toHaveBeenCalledWith('s-1', '/compact'));

    fireEvent.click(screen.getByTestId('term-shell-mode-select'));
    fireEvent.click(screen.getByRole('option', { name: 'agent' }));
    fireEvent.change(input, { target: { value: 'check workspace status' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(t.termAgentPrompt).toHaveBeenCalledWith('s-1', 'check workspace status'));

    fireEvent.click(screen.getByTestId('term-shell-mode-select'));
    fireEvent.click(screen.getByRole('option', { name: 'default' }));
    fireEvent.change(input, { target: { value: 'cargo check' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(t.termSessionSubmit).toHaveBeenCalledWith('s-1', 'cargo check'));
  });

  it('wires Explain/Fix, Co-Pilot proposal gating, Edit in Prompt, inline approvals, MENS feedback, and Autopilot preemption banner (445-04)', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.listTerminalPendingApprovals).mockResolvedValue([
      { approval_id: 'AP-term-1', tool: 'vox_terminal_exec', summary: 'Co-Pilot review - L3 Workspace Write', requested_at_ms: 1000, session_id: 's-1', command: 'cargo test -p vox-gui', control_mode: 'co_pilot' },
    ]);
    vi.mocked(t.termSessionAttach).mockResolvedValueOnce({
      session_id: 's-1', replay_utf8: '', seq: 2,
      blocks: [
        { id: 7, kind: 'Shell', status: 'Failed', origin: { kind: 'user' }, input: 'cargo test -p broken', output: [{ stream: 'Stderr', text: 'error: broken\n', ts_ms: 10 }], exit_code: 101, started_ms: 10, ended_ms: 20 },
        { id: 8, kind: 'AgentTurn', status: 'Ok', origin: { kind: 'agent', agent_id: 'terminal-agent' }, input: '? fix block #7', output: [{ stream: 'Agent', text: '[agent-model: ollama - mens-local]\nRun:\n```sh\ncargo check -p vox-gui\n```\n', ts_ms: 30 }], exit_code: 0, started_ms: 25, ended_ms: 35 },
      ],
    });
    renderConsole({ pushToast: vi.fn() });
    await waitFor(() => expect(screen.getByTestId('term-approval-card')).toBeTruthy());
    const input = screen.getByRole('textbox') as HTMLTextAreaElement;
    expect(input.getAttribute('aria-label')).toBe('console input');

    fireEvent.click(screen.getByTestId('term-approval-approve'));
    await waitFor(() => expect(t.resolveTerminalApproval).toHaveBeenCalledWith({ approvalId: 'AP-term-1', outcome: 'approved' }));
    vi.mocked(t.listTerminalPendingApprovals).mockResolvedValue([]);

    fireEvent.click(screen.getByTestId('term-explain-fix-block'));
    await waitFor(() => expect(t.termAgentExplainFix).toHaveBeenCalledWith('s-1', 7, 'fix'));

    fireEvent.click(screen.getByTestId('term-proposed-fix-edit'));
    expect(input.value).toBe('cargo check -p vox-gui');
    expect(t.termSessionSubmit).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId('term-proposed-fix-run'));
    const localCard = await screen.findByTestId('term-approval-card');
    expect(localCard.textContent).toContain('cargo check -p vox-gui');
    expect(t.termSessionSubmit).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId('term-approval-approve'));
    await waitFor(() => expect(t.termSessionSubmit).toHaveBeenCalledWith('s-1', 'cargo check -p vox-gui'));

    fireEvent.click(screen.getAllByTestId('term-feedback-accept')[0]!);
    await waitFor(() => expect(t.termBlockFeedback).toHaveBeenCalledWith('s-1', 7, 'accepted', undefined));

    const pill = screen.getByTestId('term-control-mode-pill');
    fireEvent.click(pill);
    expect(pill.textContent).toBe('Autopilot');
    fireEvent.change(input, { target: { value: 'ls' } });
    expect(pill.textContent).toBe('Manual');
    const preemptBanner = await screen.findByTestId('term-autopilot-preempted-banner');
    expect(preemptBanner.textContent).toContain('Autopilot preempted to Manual on operator input - queued agent commands paused.');
    fireEvent.click(screen.getByTestId('term-preempt-resume-copilot'));
    expect(pill.textContent).toBe('Co-Pilot');
    expect(screen.queryByTestId('term-autopilot-preempted-banner')).toBeNull();
  });

  it('preserves Session viewport cleared across SnapshotResync { blocks: [] }, routes /ai badge+submit, and forwards CR-02/CR-03 errors/originals', async () => {
    const t = await import('../../../transport');
    let emitTermEvent: ((payload: unknown) => void) | undefined;
    vi.mocked(t.listenTermEvent).mockImplementation(async (cb: (payload: unknown) => void) => {
      emitTermEvent = cb;
      return () => {};
    });
    vi.mocked(t.termSessionAttach).mockResolvedValueOnce({
      session_id: 's-1',
      replay_utf8: '',
      seq: 1,
      blocks: [
        {
          id: 5,
          kind: 'Shell',
          status: 'Ok',
          origin: { kind: 'user' },
          input: 'pwd',
          output: [{ stream: 'Stdout', text: '/repo\n', ts_ms: 10 }],
          exit_code: 0,
          started_ms: 10,
          ended_ms: 15,
        },
      ],
    });
    vi.mocked(t.termSlashExec).mockImplementationOnce(async (sid: string, rawLine: string) => {
      emitTermEvent?.({
        session_id: sid,
        seq: 2,
        event: {
          SnapshotResync: {
            blocks: [],
            resync_required: false,
            alt_screen: false,
          },
        },
      });
      return { command: rawLine, block_id: null, delegated_to: null, output: 'Cleared' };
    });

    renderConsole({ pushToast: vi.fn() });
    await waitFor(() => expect(screen.getByTestId('term-block-card')).toBeTruthy());
    expect(t.termSessionAttach).toHaveBeenCalledTimes(1);

    const input = screen.getByRole('textbox') as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: '/clear' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(screen.getByText('Session viewport cleared')).toBeTruthy());
    expect(t.termSessionAttach).toHaveBeenCalledTimes(1);

    fireEvent.change(input, { target: { value: '/ai list listening ports' } });
    expect(screen.getByText('Ask Agent')).toBeTruthy();
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(t.termAgentPrompt).toHaveBeenCalledWith('s-1', 'list listening ports'));

    act(() => {
      emitTermEvent?.({
        session_id: 's-1',
        seq: 3,
        event: {
          BlockOpened: { id: 9, kind: 'Shell', origin: { kind: 'user' }, input: 'echo ok' },
        },
      });
    });
    expect(screen.queryByText('Session viewport cleared')).toBeNull();
  });

  it('keeps pending approval intact on CR-02 Edit & Run error and passes CR-03 originalCommand on valid modification', async () => {
    const t = await import('../../../transport');
    vi.mocked(t.listTerminalPendingApprovals).mockResolvedValue([
      {
        approval_id: 'AP-err-1',
        tool: 'vox_terminal_exec',
        summary: 'ExecPolicy Ask',
        requested_at_ms: 1000,
        session_id: 's-1',
        command: 'cargo test',
      },
    ]);
    vi.mocked(t.termSessionAttach).mockResolvedValueOnce({
      session_id: 's-1',
      replay_utf8: '',
      seq: 1,
      blocks: [
        {
          id: 8,
          kind: 'AgentTurn',
          status: 'Ok',
          origin: { kind: 'agent', agent_id: 'terminal-agent' },
          input: '? check crates',
          output: [{ stream: 'Agent', text: '```sh\ncargo check -p vox-gui\n```\n', ts_ms: 20 }],
          exit_code: 0,
          started_ms: 10,
          ended_ms: 25,
        },
      ],
    });
    vi.mocked(t.resolveTerminalApproval).mockResolvedValueOnce({
      is_error: true,
      result: { success: false, error: 'Edited command blocked by ExecPolicy: subshell forbidden.' },
    });

    renderConsole({ pushToast: vi.fn() });
    await screen.findByTestId('term-approval-card');

    fireEvent.click(screen.getByTestId('term-approval-edit-toggle'));
    fireEvent.change(screen.getByTestId('term-approval-edit-input'), {
      target: { value: 'cargo test && rm -rf /' },
    });
    fireEvent.click(screen.getByTestId('term-approval-edit-submit'));

    expect(await screen.findByTestId('term-approval-edit-error')).toHaveTextContent('ExecPolicy');
    expect(screen.getByTestId('term-approval-card')).toBeTruthy();
    expect(t.termBlockFeedback).not.toHaveBeenCalled();

    vi.mocked(t.resolveTerminalApproval).mockResolvedValueOnce({
      is_error: false,
      result: { success: true, data: { resolved: true } },
    });
    vi.mocked(t.listTerminalPendingApprovals).mockResolvedValue([]);

    fireEvent.change(screen.getByTestId('term-approval-edit-input'), {
      target: { value: 'cargo test -p vox-gui' },
    });
    fireEvent.click(screen.getByTestId('term-approval-edit-submit'));

    await waitFor(() =>
      expect(t.termBlockFeedback).toHaveBeenCalledWith(
        's-1',
        0,
        'corrected',
        'cargo test -p vox-gui',
        'cargo test',
      ),
    );
    await waitFor(() => expect(screen.queryByTestId('term-approval-card')).toBeNull());

    fireEvent.click(screen.getByTestId('term-proposed-cmd-run'));
    await screen.findByTestId('term-approval-card');
    fireEvent.click(screen.getByTestId('term-approval-edit-toggle'));
    fireEvent.change(screen.getByTestId('term-approval-edit-input'), {
      target: { value: 'cargo check -p vox-cli' },
    });
    fireEvent.click(screen.getByTestId('term-approval-edit-submit'));

    await waitFor(() =>
      expect(t.termBlockFeedback).toHaveBeenCalledWith(
        's-1',
        8,
        'corrected',
        'cargo check -p vox-cli',
        'cargo check -p vox-gui',
      ),
    );
  });
});
