/**
 * Deterministic, pure-ASCII, zero-secret terminal fixtures & Tauri IPC mock overrides
 * for the 8-state Roman Scriptorium & Dual-Surface CV Flywheel (`TERM-ROMA-CV-03`).
 *
 * CRITICAL: `buildTerminalRichFixtures` and `installTerminalRichMock` are serialized via
 * `.toString()` into `richMockInitScript`. They MUST NOT reference any outer-module
 * imports, helpers, or constants outside their own function bodies!
 */

export function buildTerminalRichFixtures() {
  const sgr16Out = [
    '\x1b[30m ANSI0-BLK \x1b[0m \x1b[31mRED\x1b[0m \x1b[32mGRN\x1b[0m \x1b[33mYLW\x1b[0m \x1b[34mBLU\x1b[0m \x1b[35mMAG\x1b[0m \x1b[36mCYN\x1b[0m \x1b[37mWHT\x1b[0m',
    '\x1b[90mBR0\x1b[0m \x1b[91mBR1\x1b[0m \x1b[92mBR2\x1b[0m \x1b[93mBR3\x1b[0m \x1b[94mBR4\x1b[0m \x1b[95mBR5\x1b[0m \x1b[96mBR6\x1b[0m \x1b[97mBR7\x1b[0m',
    '\x1b[1mBOLD\x1b[0m \x1b[7m PASS \x1b[0m crates/vox-gui/src/main.rs:42:5 - 0 errors across workspace',
    '',
  ].join('\n');

  const failedOut = [
    '\x1b[31merror[E0308]\x1b[0m: mismatched types in terminal ledger assertion',
    '  --> \x1b[36mcrates/vox-terminal-core/src/lib.rs:88:9\x1b[0m',
    '   | \x1b[33mexpected u32, found usize\x1b[0m',
    '',
  ].join('\n');

  const jsonLedgerOut =
    '[{"surface":"console","theme":"roman-scriptorium","ansi_colors":16,"wcag_min":4.74},{"surface":"chat-dock","theme":"dual-surface","ansi_colors":16,"wcag_min":5.57}]\n';

  const agentTurnOut =
    '[agent-model: anthropic/claude-sonnet]\nVerified Dual-Surface tokens.\n\nVOX_PROPOSED_COMMAND: vox audit code\n';

  const slashBudgetOut =
    'Session spend: $0.18 / $10.00 - Daily cap: $1.42 / $50.00 - Active mode: Co-Pilot\n';

  const replayUtf8 = [
    '+--------------------------------------------------------------+',
    '| VOX TERMINAL MONITOR - SESSION s-1 - DUAL-SURFACE CALIBRATED |',
    '+--------------------------------------------------------------+',
    '\x1b[32mREADY\x1b[0m vox-terminal-core attached (80x24)',
    '\x1b[33mWARN\x1b[0m  co-pilot policy gate active on workspace writes',
    '\x1b[36mcargo check\x1b[0m -p vox-gui -- 0 warnings',
    '',
  ].join('\r\n');

  const sessions: Record<string, any> = {
    's-1': {
      id: 's-1',
      cwd: '~/workspace/vox',
      git_branch: 'feat/roman-scriptorium',
      security_level: 'l3_structured_default',
      control_mode: 'co_pilot',
      alt_screen: false,
      seq: 5,
      replay_utf8: replayUtf8,
      blocks: [
        {
          id: 1,
          kind: 'Shell',
          status: 'Ok',
          origin: { kind: 'user' },
          cwd: '~/workspace/vox',
          git_branch: 'feat/roman-scriptorium',
          input: 'vox check --color=always',
          output: [{ stream: 'Stdout', text: sgr16Out, ts_ms: 1700000001000 }],
          exit_code: 0,
          started_ms: 1700000000800,
          ended_ms: 1700000001000,
        },
        {
          id: 2,
          kind: 'Shell',
          status: 'Failed',
          origin: { kind: 'user' },
          credential_prompt: false,
          cwd: '~/workspace/vox',
          git_branch: 'feat/roman-scriptorium',
          input: 'cargo test -p vox-terminal-core',
          output: [{ stream: 'Stderr', text: failedOut, ts_ms: 1700000002000 }],
          exit_code: 101,
          started_ms: 1700000001600,
          ended_ms: 1700000002000,
        },
        {
          id: 3,
          kind: 'Shell',
          status: 'Ok',
          origin: { kind: 'user' },
          cwd: '~/workspace/vox',
          git_branch: 'feat/roman-scriptorium',
          input: 'vox status --json',
          output: [{ stream: 'Stdout', text: jsonLedgerOut, ts_ms: 1700000003000 }],
          exit_code: 0,
          started_ms: 1700000002900,
          ended_ms: 1700000003000,
        },
        {
          id: 4,
          kind: 'AgentTurn',
          status: 'Ok',
          origin: { kind: 'agent', agent_id: 'terminal-agent' },
          cwd: '~/workspace/vox',
          git_branch: 'feat/roman-scriptorium',
          input: '? check terminal contrast gates',
          output: [{ stream: 'Agent', text: agentTurnOut, ts_ms: 1700000004000 }],
          exit_code: 0,
          started_ms: 1700000003500,
          ended_ms: 1700000004000,
        },
        {
          id: 5,
          kind: 'SlashCommand',
          status: 'Ok',
          origin: { kind: 'user' },
          cwd: '~/workspace/vox',
          git_branch: 'feat/roman-scriptorium',
          input: '/budget',
          output: [{ stream: 'Meta', text: slashBudgetOut, ts_ms: 1700000005000 }],
          exit_code: 0,
          started_ms: 1700000004950,
          ended_ms: 1700000005000,
        },
      ],
    },
    's-2': {
      id: 's-2',
      cwd: '~/workspace/vox/crates/vox-terminal-core',
      git_branch: 'feat/roman-scriptorium',
      security_level: 'l3_structured_default',
      control_mode: 'co_pilot',
      alt_screen: false,
      seq: 1,
      replay_utf8: '\x1b[32mREADY\x1b[0m vox-terminal-core split pane session s-2\r\n',
      blocks: [
        {
          id: 1,
          kind: 'Shell',
          status: 'Ok',
          origin: { kind: 'user' },
          cwd: '~/workspace/vox/crates/vox-terminal-core',
          git_branch: 'feat/roman-scriptorium',
          input: 'vox audit arch',
          output: [{ stream: 'Stdout', text: '\x1b[32mPASS\x1b[0m layer invariants clean\n', ts_ms: 1700000006000 }],
          exit_code: 0,
          started_ms: 1700000005800,
          ended_ms: 1700000006000,
        },
      ],
    },
  };

  const history = [
    { command: 'vox audit code', cwd: '~/workspace/vox', exit_code: 0, ts_ms: 1700000010000 },
    { command: 'cargo test -p vox-terminal-core', cwd: '~/workspace/vox', exit_code: 0, ts_ms: 1700000009000 },
    { command: 'pnpm --dir crates/vox-gui/ui run tokens:build', cwd: '~/workspace/vox', exit_code: 0, ts_ms: 1700000008000 },
    { command: 'vox check scripts/setup.vox', cwd: '~/workspace/vox', exit_code: 0, ts_ms: 1700000007000 },
    { command: 'git status --short', cwd: '~/workspace/vox', exit_code: 0, ts_ms: 1700000006000 },
  ];

  const completions = [
    { label: 'vox check', insert_text: 'vox check ', kind: 'subcommand', detail: 'Type-check Vox workspace', score: 98, replace_start: 0, replace_end: 3 },
    { label: 'vox audit code', insert_text: 'vox audit code', kind: 'subcommand', detail: 'Run source policy detectors', score: 94, replace_start: 0, replace_end: 3 },
    { label: 'crates/vox-terminal-core', insert_text: 'crates/vox-terminal-core', kind: 'directory', detail: 'Terminal session core crate', score: 88, replace_start: 0, replace_end: 3 },
  ];

  const runbooks = [
    {
      id: 'rb-verify-terminal',
      title: 'Verify Terminal Gates',
      description: 'Run Dual-Surface terminal checks',
      path: 'scripts/verify-terminal.vox',
      kind: 'vox',
      command_template: 'vox run scripts/verify-terminal.vox --theme {{theme}}',
      params: [
        {
          name: 'theme',
          default_value: 'arcane',
          description: 'Target theme',
          options: ['arcane', 'slate', 'travertine', 'high-contrast'],
        },
      ],
      tags: ['terminal', 'cv'],
    },
    {
      id: 'rb-check-tokens',
      title: 'Build Theme Tokens',
      description: 'Regenerate Style Dictionary CSS bundles',
      path: 'scripts/check-tokens.vox',
      kind: 'vox',
      command_template: 'vox run scripts/check-tokens.vox',
      params: [],
      tags: ['tokens'],
    },
  ];

  return { sessions, history, completions, runbooks };
}

export function installTerminalRichMock(): void {
  const w = window as any;
  const internals = w.__TAURI_INTERNALS__;
  const prevInvoke: ((cmd: string, args?: any) => Promise<any>) | undefined = internals?.invoke;
  const buildFn: (() => ReturnType<typeof buildTerminalRichFixtures>) | undefined = w.__VOX_TERM_RICH_BUILD__;
  if (typeof prevInvoke !== 'function' || typeof buildFn !== 'function') {
    throw new Error('installTerminalRichMock requires installTauriMock and __VOX_TERM_RICH_BUILD__');
  }

  const fx = buildFn();
  w.__MOCK_TERM_SESSIONS__ = fx.sessions;
  w.__MOCK_TERM_NEXT_SESSION__ = 2;
  w.__MOCK_TERM_NEXT_BLOCK__ = 5;
  w.__MOCK_TERM_HISTORY__ = fx.history.slice();

  internals.invoke = async (cmd: string, args?: any) => {
    const store = (w.__MOCK_TERM_SESSIONS__ ??= fx.sessions);
    switch (cmd) {
      case 'term_session_list':
        return Object.values(store).map((s: any) => ({
          id: s.id,
          blocks: Array.isArray(s.blocks) ? s.blocks.length : 0,
          cwd: s.cwd ?? '~/workspace/vox',
          git_branch: s.git_branch ?? 'feat/roman-scriptorium',
          alt_screen: Boolean(w.__MOCK_TERM_ALT_SCREEN__ || s.alt_screen),
          security_level: s.security_level ?? 'l3_structured_default',
          control_mode: s.control_mode ?? 'co_pilot',
        }));
      case 'term_session_attach': {
        const sid = String(args?.sessionId ?? 's-1');
        const sess = store[sid] ?? store['s-1'];
        return {
          session_id: sess.id,
          blocks: sess.blocks.map((b: any) => ({ ...b })),
          replay_utf8: sess.replay_utf8,
          seq: sess.seq,
          cwd: sess.cwd ?? '~/workspace/vox',
          git_branch: sess.git_branch ?? 'feat/roman-scriptorium',
          alt_screen: Boolean(w.__MOCK_TERM_ALT_SCREEN__ || sess.alt_screen),
          security_level: sess.security_level ?? 'l3_structured_default',
          control_mode: sess.control_mode ?? 'co_pilot',
        };
      }
      case 'term_history_query': {
        const q = String(args?.query ?? '').trim().toLowerCase();
        const lim = typeof args?.limit === 'number' ? args.limit : 50;
        const rows: any[] = Array.isArray(w.__MOCK_TERM_HISTORY__) ? w.__MOCK_TERM_HISTORY__ : fx.history;
        const filtered = q ? rows.filter((r) => String(r.command).toLowerCase().includes(q)) : rows;
        return filtered.slice(0, lim).map((r) => ({ ...r }));
      }
      case 'term_history_record': {
        const rows: any[] = (w.__MOCK_TERM_HISTORY__ ??= fx.history.slice());
        rows.unshift({
          command: String(args?.command ?? ''),
          cwd: args?.cwd ?? '~/workspace/vox',
          exit_code: typeof args?.exitCode === 'number' ? args.exitCode : 0,
          ts_ms: Date.now(),
        });
        return true;
      }
      case 'term_history_clear':
        w.__MOCK_TERM_HISTORY__ = [];
        return null;
      case 'term_complete':
        return fx.completions.map((c: any) => ({ ...c }));
      case 'term_runbooks_list':
        return fx.runbooks.map((r: any) => ({ ...r, params: r.params.map((p: any) => ({ ...p })) }));
      case 'term_session_voxify':
        return null;
      case 'term_session_set_control': {
        const sess = store[String(args?.sessionId ?? 's-1')] ?? store['s-1'];
        if (sess) sess.control_mode = args?.controlMode ?? 'co_pilot';
        return null;
      }
      case 'term_session_set_security': {
        const sess = store[String(args?.sessionId ?? 's-1')] ?? store['s-1'];
        if (sess) sess.security_level = args?.securityLevel ?? 'l3_structured_default';
        return null;
      }
      case 'term_agent_prompt':
      case 'term_agent_explain_fix': {
        const sid = String(args?.sessionId ?? 's-1');
        const sess = store[sid] ?? store['s-1'];
        const bid = Number(w.__MOCK_TERM_NEXT_BLOCK__ || 5) + 1;
        w.__MOCK_TERM_NEXT_BLOCK__ = bid;
        const text = '[agent-model: anthropic/claude-sonnet]\nVerified Dual-Surface tokens.\n\nVOX_PROPOSED_COMMAND: vox audit code\n';
        sess.blocks.push({
          id: bid,
          kind: 'AgentTurn',
          status: 'Ok',
          origin: { kind: 'agent', agent_id: 'terminal-agent' },
          input: String(args?.prompt ?? '? explain failed block'),
          output: [{ stream: 'Agent', text, ts_ms: Date.now() }],
          exit_code: 0,
          started_ms: Date.now() - 20,
          ended_ms: Date.now(),
        });
        sess.seq += 1;
        return { block_id: bid, model_id: 'anthropic/claude-sonnet', proposed_command: 'vox audit code', text };
      }
      case 'term_agent_cancel':
        return { cancelled: true };
      case 'term_slash_exec': {
        const sid = String(args?.sessionId ?? 's-1');
        const sess = store[sid] ?? store['s-1'];
        const bid = Number(w.__MOCK_TERM_NEXT_BLOCK__ || 5) + 1;
        w.__MOCK_TERM_NEXT_BLOCK__ = bid;
        const out = 'Session spend: $0.18 / $10.00 - Active mode: Co-Pilot\n';
        sess.blocks.push({
          id: bid,
          kind: 'SlashCommand',
          status: 'Ok',
          origin: { kind: 'user' },
          input: String(args?.command ?? '/budget'),
          output: [{ stream: 'Meta', text: out, ts_ms: Date.now() }],
          exit_code: 0,
          started_ms: Date.now() - 10,
          ended_ms: Date.now(),
        });
        sess.seq += 1;
        return { block_id: bid, output: out };
      }
      case 'term_block_feedback':
        (w.__MOCK_TERM_FEEDBACK__ ??= []).push({ ...args });
        return { recorded: true };
      case 'invoke_mcp_tool':
        if (args?.tool === 'vox_pending_approvals' && w.__MOCK_TERM_APPROVAL_ACTIVE__ === true) {
          return {
            tool: 'vox_pending_approvals',
            is_error: false,
            result: {
              data: {
                approvals: [
                  {
                    approval_id: 'apr-term-446',
                    session_id: 's-1',
                    tool: 'vox_terminal_exec',
                    command: 'cargo test -p vox-terminal-core -- --nocapture',
                    summary: 'cargo test -p vox-terminal-core -- --nocapture',
                    policy_tier: 'Ask',
                    control_mode: 'co_pilot',
                    credential_prompt: false,
                    requested_at_ms: Date.now() - 5000,
                  },
                ],
              },
            },
          };
        }
        if (args?.tool === 'vox_resolve_approval') {
          w.__MOCK_TERM_APPROVAL_ACTIVE__ = false;
          return { tool: 'vox_resolve_approval', is_error: false, result: { ok: true, data: { resolved: true } } };
        }
        return prevInvoke(cmd, args);
      case 'chat_get_messages': {
        const msgs = await prevInvoke(cmd, args);
        if (!w.__MOCK_CHAT_TERM_DOCK__ || !Array.isArray(msgs)) return msgs;
        return msgs.map((m: any, idx: number) =>
          idx === msgs.length - 1 && m.role === 'assistant'
            ? {
                ...m,
                events: [
                  ...(Array.isArray(m.events) ? m.events : []),
                  {
                    id: 'ev-term-446',
                    ts: 8,
                    kind: 'tool_receipt',
                    tool: 'vox_terminal_exec',
                    session_id: 's-1',
                    block_id: 1,
                    command: 'vox check --color=always',
                    output: '\x1b[32mPASS\x1b[0m vox check: 0 errors across workspace\n',
                    exit_code: 0,
                    fulfilled: true,
                  },
                ],
              }
            : m,
        );
      }
      default:
        return prevInvoke(cmd, args);
    }
  };
}
