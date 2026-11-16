import { describe, it, expect, beforeEach } from 'vitest';
import {
  getTerminalBridgeState,
  requestConsoleBlockFocus,
  consumePendingConsoleBlockFocus,
  onConsoleBlockFocus,
  resetTerminalBridgeForTests,
  type ConsoleFocusTarget,
} from '../../../lib/terminalBridge';
import {
  createInitialSessionPaneState,
  hydrateSessionFromAttachment,
  normalizeWireBlock,
  reduceSessionEvent,
  scanAltScreenTransition,
  toWireSecurityLevel,
  fromWireSecurityLevel,
  toWireControlMode,
  fromWireControlMode,
} from './sessionReducer';

describe('terminalBridge Console focus pub/sub', () => {
  beforeEach(() => {
    resetTerminalBridgeForTests();
  });

  it('publishes focus targets to subscribers and stores pending target until consumed', () => {
    const received: ConsoleFocusTarget[] = [];
    const unsub = onConsoleBlockFocus((t) => received.push(t));

    requestConsoleBlockFocus({ sessionId: 's-9', blockId: 4 });
    expect(received).toEqual([{ sessionId: 's-9', blockId: 4 }]);
    expect(consumePendingConsoleBlockFocus()).toEqual({ sessionId: 's-9', blockId: 4 });
    expect(consumePendingConsoleBlockFocus()).toBeNull();

    unsub();
    requestConsoleBlockFocus({ sessionId: 's-10', blockId: null });
    expect(received).toHaveLength(1);
    resetTerminalBridgeForTests();
    expect(consumePendingConsoleBlockFocus()).toBeNull();
    expect(getTerminalBridgeState().securityLevel).toBe('l3_workspace_write');
    expect(getTerminalBridgeState().controlMode).toBe('co_pilot');
  });
});

describe('sessionReducer — 11 SessionEvent variants & attachment hydration', () => {
  it('normalizes wire blocks and hydrates initial attachment + replayed_events', () => {
    const raw = normalizeWireBlock(
      {
        id: 7,
        kind: 'Shell',
        input: 'cargo check',
        output: [{ stream: 'Stdout', text: 'ok\n' } as never],
      },
      1000,
    );
    expect(raw.started_ms).toBe(1000);
    expect(raw.ended_ms).toBeNull();
    expect(raw.status).toBe('Running');
    expect(raw.exit_code).toBeNull();
    expect(raw.output[0]).toEqual({ stream: 'Stdout', text: 'ok\n', ts_ms: 1000 });

    const initial = createInitialSessionPaneState();
    const hydrated = hydrateSessionFromAttachment(
      initial,
      {
        session_id: 's-1',
        seq: 12,
        replay_utf8: '$ ',
        resync_required: true,
        alt_screen: false,
        cwd: '/workspace/vox',
        git_branch: 'main',
        security_level: 'l3_structured_default',
        control_mode: 'co_pilot',
        blocks: [
          {
            id: 1,
            kind: 'Shell',
            status: 'Ok',
            pinned: true,
            input: 'pwd',
            output: [{ stream: 'Stdout', text: '/workspace/vox\n', ts_ms: 500 }],
            exit_code: 0,
            started_ms: 500,
            ended_ms: 510,
          },
        ],
        replayed_events: [
          {
            BlockOpened: {
              id: 2,
              kind: 'Shell',
              input: 'ls',
              origin: { kind: 'user' },
              cwd: '/workspace/vox',
              git_branch: 'main',
            },
          },
        ],
      },
      2000,
    );

    expect(hydrated.sessionId).toBe('s-1');
    expect(hydrated.seq).toBe(12);
    expect(hydrated.cwd).toBe('/workspace/vox');
    expect(hydrated.gitBranch).toBe('main');
    expect(hydrated.securityLevel).toBe('l3_workspace_write');
    expect(hydrated.controlMode).toBe('co_pilot');
    expect(hydrated.pinnedBlockIds).toEqual([1]);
    expect(hydrated.blocks).toHaveLength(2);
    expect(hydrated.blocks[1]?.input).toBe('ls');
  });

  it('reduces all 11 externally-tagged Rust SessionEvent variants deterministically', () => {
    let state = createInitialSessionPaneState({ sessionId: 's-1' });

    // 1. BlockOpened
    state = reduceSessionEvent(
      state,
      {
        BlockOpened: {
          id: 10,
          kind: 'Shell',
          input: 'cargo test',
          origin: { kind: 'agent', agent_id: 'a-1' },
          cwd: '/repo',
          git_branch: 'feat/pane',
        },
      },
      1000,
    );
    expect(state.blocks).toHaveLength(1);
    expect(state.blocks[0]).toMatchObject({
      id: 10,
      status: 'Running',
      input: 'cargo test',
      cwd: '/repo',
      git_branch: 'feat/pane',
      started_ms: 1000,
      ended_ms: null,
    });

    // 2. OutputAppended (must append to matching block id and stamp fallback ts_ms)
    state = reduceSessionEvent(
      state,
      {
        OutputAppended: {
          id: 10,
          chunk: { stream: 'Stdout', text: 'running 3 tests\n' },
        },
      },
      1050,
    );
    expect(state.blocks[0]?.output).toEqual([
      { stream: 'Stdout', text: 'running 3 tests\n', ts_ms: 1050 },
    ]);

    // 3. BlockAwaitingInput
    state = reduceSessionEvent(
      state,
      {
        BlockAwaitingInput: {
          id: 10,
          prompt_tail: 'Password:',
          prompt_kind: 'credential',
          credential_prompt: true,
        },
      },
      1100,
    );
    expect(state.blocks[0]).toMatchObject({
      status: 'AwaitingInput',
      prompt_tail: 'Password:',
      prompt_kind: 'credential',
      credential_prompt: true,
    });

    // 4. BlockPinned (pin + unpin)
    state = reduceSessionEvent(state, { BlockPinned: { id: 10, pinned: true } }, 1120);
    expect(state.blocks[0]?.pinned).toBe(true);
    expect(state.pinnedBlockIds).toEqual([10]);

    state = reduceSessionEvent(state, { BlockPinned: { id: 10, pinned: false } }, 1130);
    expect(state.blocks[0]?.pinned).toBe(false);
    expect(state.pinnedBlockIds).toEqual([]);

    // 5. BlockClosed
    state = reduceSessionEvent(
      state,
      { BlockClosed: { id: 10, exit_code: 0, status: 'Ok' } },
      1200,
    );
    expect(state.blocks[0]).toMatchObject({
      exit_code: 0,
      status: 'Ok',
      ended_ms: 1200,
    });

    // 6. AgentMessage
    state = reduceSessionEvent(state, { AgentMessage: { text: 'Agent summary ready' } }, 1250);
    expect(state.lastAgentMessage).toBe('Agent summary ready');

    // 7. CwdChanged
    state = reduceSessionEvent(
      state,
      { CwdChanged: { cwd: '/repo/crates', git_branch: 'feat/webgl' } },
      1300,
    );
    expect(state.cwd).toBe('/repo/crates');
    expect(state.gitBranch).toBe('feat/webgl');

    // 8. AltScreenChanged
    state = reduceSessionEvent(state, { AltScreenChanged: { active: true } }, 1350);
    expect(state.altScreen).toBe(true);

    // 9. SecurityLevelChanged
    state = reduceSessionEvent(
      state,
      { SecurityLevelChanged: { security_level: 'l4_host_gated' } },
      1400,
    );
    expect(state.securityLevel).toBe('l4_system_exec');

    // 10. ControlModeChanged
    state = reduceSessionEvent(
      state,
      { ControlModeChanged: { control_mode: 'autopilot' } },
      1450,
    );
    expect(state.controlMode).toBe('autopilot');

    // 11. SnapshotResync
    state = reduceSessionEvent(
      state,
      {
        SnapshotResync: {
          blocks: [
            {
              id: 99,
              kind: 'System',
              status: 'Ok',
              origin: { kind: 'system' },
              pinned: true,
              input: 'resynced',
              output: [{ stream: 'Agent', text: 'done', ts_ms: 1500 }],
              exit_code: 0,
              started_ms: 1490,
              ended_ms: 1500,
            },
          ],
          resync_required: true,
          alt_screen: false,
          cwd: '/fresh',
          git_branch: 'release',
        },
      },
      1500,
    );
    expect(state.resyncRequired).toBe(true);
    expect(state.altScreen).toBe(false);
    expect(state.cwd).toBe('/fresh');
    expect(state.gitBranch).toBe('release');
    expect(state.blocks).toHaveLength(1);
    expect(state.blocks[0]?.id).toBe(99);
    expect(state.pinnedBlockIds).toEqual([99]);
  });
});

describe('scanAltScreenTransition & security/control wire mappers', () => {
  it('detects enter/exit ALT_SCREEN sequences and resolves the last transition in a chunk', () => {
    expect(scanAltScreenTransition('normal output\r\n')).toBeNull();
    expect(scanAltScreenTransition('enter vim \x1b[?1049h frame')).toBe(true);
    expect(scanAltScreenTransition('\x1b[?1047h')).toBe(true);
    expect(scanAltScreenTransition('\x1b[?47h')).toBe(true);
    expect(scanAltScreenTransition('exit vim \x1b[?1049l prompt')).toBe(false);
    expect(scanAltScreenTransition('\x1b[?1047l')).toBe(false);
    expect(scanAltScreenTransition('\x1b[?47l')).toBe(false);
    expect(scanAltScreenTransition('\x1b[?1049h redraw \x1b[?1049l')).toBe(false);
    expect(scanAltScreenTransition('\x1b[?1049l then reopen \x1b[?1049h')).toBe(true);
  });

  it('round-trips all 5 security levels and 3 control modes between UI and wire formats', () => {
    const secPairs = [
      ['l1_read_only', 'l1_read_only'],
      ['l2_safe_exec', 'l2_workspace_build'],
      ['l3_workspace_write', 'l3_structured_default'],
      ['l4_system_exec', 'l4_host_gated'],
      ['l5_autopilot_floor', 'l5_autopilot_floor'],
    ] as const;

    for (const [ui, wire] of secPairs) {
      expect(toWireSecurityLevel(ui)).toBe(wire);
      expect(fromWireSecurityLevel(wire)).toBe(ui);
    }

    expect(toWireControlMode('manual')).toBe('manual');
    expect(toWireControlMode('co_pilot')).toBe('co_pilot');
    expect(toWireControlMode('copilot')).toBe('co_pilot');
    expect(toWireControlMode('autopilot')).toBe('autopilot');

    expect(fromWireControlMode('manual')).toBe('manual');
    expect(fromWireControlMode('co_pilot')).toBe('co_pilot');
    expect(fromWireControlMode('copilot')).toBe('co_pilot');
    expect(fromWireControlMode('autopilot')).toBe('autopilot');
  });
});
