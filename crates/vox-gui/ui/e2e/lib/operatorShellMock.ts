/**
 * Tauri invoke mock for operator-shell e2e (dashboard, policies, status bar, chat, palette).
 *
 * Injected via `page.addInitScript(installOperatorShellMock, opts)`.
 */
export interface OperatorShellMockOptions {
  initialView?: string;
  seedGuiPrefs?: Record<string, string>;
}

export function installOperatorShellMock(opts: OperatorShellMockOptions = {}): void {
  const initialView = opts.initialView ?? 'dashboard';
  try {
    localStorage.setItem(
      'vox_workbench_tabs.v1',
      JSON.stringify({
        openTabs: Array.from(new Set(['chat', initialView])),
        activeTab: initialView,
      }),
    );
    localStorage.setItem('vox_sidebar_mode', 'default');
  } catch {
    // sandboxed contexts may deny localStorage
  }

  (window as any).__TAURI_CALLS__ = [];
  (window as any).__GUI_PREFS__ = { ...(opts.seedGuiPrefs ?? {}) };
  (window as any).__TAURI_EVENT_HANDLERS__ = {} as Record<string, Set<(...args: unknown[]) => void>>;

  (window as any).__TAURI_INTERNALS__ = {
    transformCallback: (cb: (...args: unknown[]) => unknown) => {
      const id = `cb_${Math.random().toString(36).slice(2)}`;
      (window as any)[id] = cb;
      return id;
    },
    invoke: async (cmd: string, args?: Record<string, unknown>) => {
      (window as any).__TAURI_CALLS__.push({ cmd, args: args ?? null });
      const prefs = (window as any).__GUI_PREFS__ as Record<string, string>;

      switch (cmd) {
        case 'get_gui_preference':
          return prefs[args?.key as string] ?? null;
        case 'set_gui_preference':
          prefs[args?.key as string] = args?.value as string;
          return null;
        // Non-fresh-install defaults so the first-run OnboardingWizard (Task 15)
        // doesn't cover the shell in every operator-shell-mock-backed spec.
        case 'list_secret_status':
          return [{ id: 'OPENROUTER_API_KEY', isPresent: true }];
        case 'inference_provider_status':
          return [];
        case 'get_initial_view':
          return initialView;
        case 'get_build_info':
          return { version: '0.6.0', display: '0.6.0+build.test (abc123)' };
        case 'get_orchestrator_status_bin':
          return new Uint8Array([0x80]);
        case 'get_orchestrator_status':
          return { agent_count: 0, agents: [], recent_events: [], alerts: [], peers: [] };
        case 'get_identity_summary':
          return { display_name: 'tester@vox', os_user: 'tester' };
        case 'get_command_catalog':
          return { generated_from: 'e2e-mock', entries: [] };
        case 'get_action_manifest':
          return {
            x_vox_version: 2,
            schema_version: 1,
            generated_from: 'e2e-mock',
            actions: [],
          };
        case 'get_routing_summary':
        case 'get_routing_summary_live':
          return {
            active_model: 'mens-8b',
            exploration_spent_usd: 0,
            exploration_budget_usd: 50,
            arm_count: 1,
            model_count: 1,
            decision_preview: null,
          };
        case 'get_active_model':
          return 'mens-8b';
        case 'get_selection_policy':
          return { chain: ['mens-8b'], free_tier: true };
        case 'get_models_snapshot': {
          const inv = (window as any).__TAURI_INTERNALS__.invoke;
          const [cards, routing, active_model, providers] = await Promise.all([
            inv('list_model_cards'), inv('get_routing_summary_live'), inv('get_active_model'), inv('inference_provider_status'),
          ]);
          return { cards, routing, active_model, providers };
        }
        case 'list_model_cards':
          return [{ model_id: 'mens-8b', display_name: 'Mens 8B', id: 'mens-8b' }];
        case 'get_model_scoreboard':
          return [];
        case 'list_gui_runs':
          return [];
        case 'list_orchestrator_tasks':
          return [];
        case 'get_llm_spend':
          return {
            sessionUsd: 0,
            dayUsd: 0,
            totalUsd: 1.25,
            dailyBudgetUsd: 50,
            perSessionBudgetUsd: 10,
          };
        case 'get_gamify_settings':
          return { enabled: false, mode: 'off' };
        case 'get_ludus_profile':
          return {
            user_id: 'local',
            level: 1,
            xp: 0,
            xp_to_next_level: 100,
            xp_progress: 0,
            total_xp_earned: 0,
            crystals: 0,
            lumens: 0,
            energy: 100,
            max_energy: 100,
            current_streak: 0,
            prestige_level: 0,
            title: 'Initiate',
            full_title: 'Initiate',
            trust_tier: 'New',
          };
        case 'list_ludus_notifications':
          return [];
        case 'record_gui_event':
          return { xpGranted: 0, lumensGranted: 0, achievementTitle: null };
        case 'vox_docs_index':
          return [];
        case 'read_doc_markdown':
          return `# ${String(args?.path ?? 'doc')}\n\nMock doc.`;
        case 'vox_search_query':
          return { hits: [], next_cursor: null, total: 0 };
        case 'chat_list_sessions':
          return [
            {
              session_id: 'mock-session-1',
              title: 'Mock chat',
              updated_at: 'now',
              message_count: 0,
              conversation_id: 1,
            },
          ];
        case 'chat_create_session':
          return {
            session_id: 'mock-session-new',
            title: 'New chat',
            updated_at: 'now',
            message_count: 0,
            conversation_id: 2,
          };
        case 'chat_get_messages':
          return [];
        case 'policy_list':
          return [
            {
              id: 'fmt.rust',
              title: 'Rust formatting',
              domain: 'format',
              group: 'Formatting',
            },
          ];
        case 'list_branches':
          return [{ branch: 'main', path: '.', isCurrent: true }];
        case 'policy_status':
          return [];
        case 'policy_show':
          return {
            id: 'fmt.rust',
            title: 'Rust formatting',
            domain: 'format',
            description: 'x',
            blocking: true,
            runsOn: ['push'],
            origin: 'builtin',
            sourceKind: 'lint',
            sourceRef: 'r',
          };
        case 'invoke_mcp_tool': {
          const tool = String(args?.tool ?? '');
          if (tool.includes('pending_approval')) {
            return {
              tool,
              is_error: false,
              result: { success: true, data: { approvals: [] } },
            };
          }
          if (tool.includes('skill')) {
            return { tool, is_error: false, result: { success: true, data: { skills: [] } } };
          }
          return { tool, is_error: false, result: { success: true, data: {} } };
        }
        case 'pty_spawn':
        case 'pty_write':
        case 'pty_close':
        case 'term_session_input':
        case 'term_session_resize':
        case 'term_session_close':
          return null;
        case 'term_session_open': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          const nextIdx = (((window as any).__MOCK_TERM_NEXT_SESSION__ ??= 1) as number) + 1;
          (window as any).__MOCK_TERM_NEXT_SESSION__ = nextIdx;
          const sid = `s-${nextIdx}`;
          store[sid] = {
            id: sid,
            blocks: [],
            replay_utf8: `C:\\workspace\\vox (${sid})> \r\n`,
            seq: 1,
          };
          return sid;
        }
        case 'term_session_list': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          return Object.values(store).map((s: any) => ({
            id: s.id,
            blocks: s.blocks.length,
          }));
        }
        case 'term_session_attach': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          const sid = String((args?.sessionId as string) ?? 's-1');
          const sess = store[sid] ?? store['s-1'];
          return {
            session_id: sid,
            blocks: sess.blocks.map((b: any) => ({ ...b })),
            replay_utf8: sess.replay_utf8,
            seq: sess.seq,
          };
        }
        case 'term_session_submit': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          const sid = String((args?.sessionId as string) ?? 's-1');
          const sess = (store[sid] ??= {
            id: sid,
            blocks: [],
            replay_utf8: '',
            seq: 0,
          });
          const bid = (((window as any).__MOCK_TERM_NEXT_BLOCK__ ??= 1) as number) + 1;
          (window as any).__MOCK_TERM_NEXT_BLOCK__ = bid;
          const rawLine = String(args?.line ?? '').trim();
          const isNu = rawLine.startsWith('!nu') || rawLine.startsWith('/nu');
          const outputText = isNu
            ? '[{"name":"vox-terminal-core","status":"ok"}]\n'
            : `[ok] ${rawLine}\n`;
          const newBlock = {
            id: bid,
            kind: rawLine.startsWith('?') ? 'AgentTurn' : rawLine.startsWith('/') ? 'SlashCommand' : 'Shell',
            status: 'Ok',
            origin: { kind: 'user' },
            input: rawLine,
            output: [{ stream: rawLine.startsWith('?') ? 'Agent' : 'Stdout', text: outputText, ts_ms: Date.now() }],
            exit_code: 0,
            started_ms: Date.now() - 5,
            ended_ms: Date.now(),
          };
          sess.blocks.push(newBlock);
          sess.seq += 1;
          sess.replay_utf8 += `C:\\workspace\\vox> ${rawLine}\r\n${outputText}\r\n`;
          return bid;
        }
        case 'execute_command':
          return { exit_code: 0, stdout: 'ok', stderr: '' };
        case 'submit_orchestrator_task':
          return { ok: true, task_id: '101', message: 'submitted' };
        case 'pause_orchestrator_agent':
        case 'resume_orchestrator_agent':
          return { ok: true };
        case 'discovery_record':
          return null;
        case 'coderabbit_token_present':
          return true;
        case 'coderabbit_report':
          return {
            run_state: { chunks: [{ name: 'crate_vox_db', pr_number: 7, status: 'completed' }] },
            db_status: { findings: 3 },
          };
        case 'coderabbit_plan':
          return {
            baseline_branch: 'cr-baseline',
            total_files: 3,
            chunks: [
              { order: 1, name: 'crate_vox_db', files: ['a.rs', 'b.rs'] },
              { order: 2, name: '06_docs_src', files: ['x.md'] },
            ],
          };
        case 'coderabbit_run_async':
          return { status: 'running' };
        default:
          return null;
      }
    },
  };
}
