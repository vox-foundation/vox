import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { AddressInfo } from 'node:net';

export interface PersistedChatSession {
  session_id: string;
  title: string;
  updated_at: string;
  message_count: number;
  conversation_id: number;
}

export interface PersistedChatMessage {
  id: number;
  role: string;
  content: string;
  created_at: string;
  task_id: string | null;
  model_id?: string | null;
  cost_usd?: number | null;
  latency_ms?: number | null;
  already_submitted?: boolean;
}

export interface PersistedTermBlock {
  id: number;
  kind: string;
  status: 'Ok' | 'Failed' | 'Running';
  origin: { kind: string; agent_id?: string };
  input: string;
  output: Array<{ stream: string; text: string; ts_ms: number }>;
  exit_code: number;
  started_ms: number;
  ended_ms: number;
}

export interface PersistedTermSession {
  id: string;
  blocks: PersistedTermBlock[];
  replay_utf8: string;
  seq: number;
  security_level: string;
  control_mode: string;
}

export interface RealBackendDiskState {
  nextId: number;
  oversightLevel: number;
  kbScopeEnabled: boolean;
  preferences: Record<string, string>;
  sessions: PersistedChatSession[];
  messages: Record<string, PersistedChatMessage[]>;
  tasks: Array<{ id: string; description: string; status: string; session_id: string | null }>;
  termSessions: Record<string, PersistedTermSession>;
}

export interface RealBackendHandle {
  url: string;
  port: number;
  tempDir: string;
  stateFile: string;
  readState: () => RealBackendDiskState;
  close: () => Promise<void>;
}

const SECRET_TOKEN_RE = /\b(sk-[A-Za-z0-9_-]{16,})\b/g;

export function redactTerminalText(text: string): string {
  return text.replace(SECRET_TOKEN_RE, '[REDACTED_KEY]');
}

function initialDiskState(): RealBackendDiskState {
  const initialSid = 'real-session-1';
  return {
    nextId: 100,
    oversightLevel: 3,
    kbScopeEnabled: false,
    preferences: {
      'gui.oversight_level': '3',
      'gui.kb_scope_enabled': 'false',
    },
    sessions: [
      {
        session_id: initialSid,
        title: 'Daily Driver Session',
        updated_at: new Date().toISOString(),
        message_count: 0,
        conversation_id: 1,
      },
    ],
    messages: {
      [initialSid]: [],
    },
    tasks: [],
    termSessions: {
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
        replay_utf8: 'vox> vox run\r\nReady.\r\n',
        seq: 1,
        security_level: 'l3_workspace_write',
        control_mode: 'co_pilot',
      },
    },
  };
}

const DEFAULT_MODELS = [
  {
    id: 'mens-8b',
    model_id: 'mens-8b',
    display_name: 'Mens 8B',
    provider: 'mens',
    tier: 'Local',
    cost_per_1k: 0,
    max_tokens: 200000,
    is_free: true,
    latency_p50_ms: 120,
    success_rate: 0.98,
    quality_score: 0.78,
  },
  {
    id: 'anthropic/claude-opus',
    model_id: 'anthropic/claude-opus',
    display_name: 'Claude Opus',
    provider: 'anthropic',
    tier: 'Elite',
    cost_per_1k: 0.015,
    max_tokens: 200000,
    is_free: false,
    latency_p50_ms: 900,
    success_rate: 0.99,
    quality_score: 0.95,
  },
  {
    id: 'anthropic/claude-sonnet',
    model_id: 'anthropic/claude-sonnet',
    display_name: 'Claude Sonnet',
    provider: 'anthropic',
    tier: 'Pro',
    cost_per_1k: 0.003,
    max_tokens: 200000,
    is_free: false,
    latency_p50_ms: 600,
    success_rate: 0.985,
    quality_score: 0.9,
  },
];

const COMMAND_CATALOG_ENTRIES = [
  ['check', 'vox check', 'Check a Vox script for type/syntax errors', 'core'],
  ['build', 'vox build', 'Compile a Vox package to binary', 'core'],
  ['test', 'vox test', 'Execute unit and doc tests', 'core'],
  ['run', 'vox run', 'Run a Vox script or tool', 'core'],
  ['fmt', 'vox fmt', 'Format Vox source files', 'core'],
  ['doctor', 'vox doctor', 'Inspect and self-heal local workspace readiness', 'core'],
  ['fabrica fmt', 'vox fabrica fmt', 'Format Fabrica workflow specifications', 'fabrica'],
  ['populi status', 'vox populi status', 'Inspect Populi mesh node status', 'populi'],
].map(([rawPath, command, about, group]) => ({
  path: rawPath.split(' '),
  command,
  about,
  aliases: [],
  has_subcommands: false,
  compiled_in: true,
  source_group: group,
  feature_gate: null,
  tier: 'recommended',
  arguments: [
    {
      name: 'path',
      short: null,
      long: 'path',
      help: 'Target path',
      required: false,
      takes_value: true,
      value_kind: 'value',
      possible_values: [],
      default_values: [],
    },
  ],
}));

export async function startRealBackendServer(): Promise<RealBackendHandle> {
  const tempDir = mkdtempSync(join(tmpdir(), 'vox-real-backend-'));
  const stateFile = join(tempDir, 'voxdb-state.json');
  writeFileSync(stateFile, JSON.stringify(initialDiskState(), null, 2), 'utf8');

  const readState = (): RealBackendDiskState => {
    if (!existsSync(stateFile)) return initialDiskState();
    return JSON.parse(readFileSync(stateFile, 'utf8')) as RealBackendDiskState;
  };

  const writeState = (s: RealBackendDiskState): void => {
    writeFileSync(stateFile, JSON.stringify(s, null, 2), 'utf8');
  };

  const handleRpc = (cmd: string, args: any, defaultView: string): unknown => {
    const state = readState();
    switch (cmd) {
      case 'get_initial_view':
        return defaultView;
      case 'get_build_info':
        return { version: '0.6.0', display: '0.6.0+real-backend' };
      case 'get_orchestrator_status_bin':
        return [0x80];
      case 'get_orchestrator_status':
        return { agent_count: 0, agents: [], recent_events: [], alerts: [], peers: [] };
      case 'get_action_manifest':
        return { x_vox_version: 2, schema_version: 1, generated_from: 'real-backend', actions: [] };
      case 'get_gui_preference': {
        const key = String(args?.key ?? '');
        return state.preferences[key] ?? null;
      }
      case 'set_gui_preference': {
        const key = String(args?.key ?? '');
        const value = String(args?.value ?? '');
        state.preferences[key] = value;
        if (key === 'gui.oversight_level') {
          state.oversightLevel = Number(value) || 3;
        }
        if (key === 'gui.kb_scope_enabled') {
          state.kbScopeEnabled = value === 'true';
        }
        writeState(state);
        return key;
      }
      case 'get_oversight_level':
        return state.oversightLevel;
      case 'set_oversight_level': {
        const lvl = Math.min(5, Math.max(1, Number(args?.level ?? args?.oversightLevel ?? 3)));
        state.oversightLevel = lvl;
        state.preferences['gui.oversight_level'] = String(lvl);
        writeState(state);
        return lvl;
      }
      case 'get_gamify_settings':
        return { enabled: false, mode: 'off' };
      case 'get_identity_summary':
        return { display_name: 'operator@vox', os_user: 'operator' };
      case 'get_active_model':
        return 'anthropic/claude-sonnet';
      case 'list_model_cards':
        return DEFAULT_MODELS;
      case 'get_routing_summary_live':
        return {
          active_model: 'anthropic/claude-sonnet',
          exploration_spent_usd: 1.2,
          exploration_budget_usd: 50,
          arm_count: 3,
          model_count: 3,
          family: 'anthropic/claude-sonnet',
          resolved_from: 'live',
          reason: 'highest precision within latency target',
          decision_preview: {
            selected_model: 'anthropic/claude-sonnet',
            discovery_state: 'confirmed',
            alternatives: ['mens-8b', 'anthropic/claude-opus'],
            rejection_reasons: [],
            intelligence_score: 0.9,
            efficiency_score: 0.8,
            latency_score: 0.75,
          },
        };
      case 'get_models_snapshot':
        return {
          cards: DEFAULT_MODELS,
          routing: {
            active_model: 'anthropic/claude-sonnet',
            exploration_spent_usd: 1.2,
            exploration_budget_usd: 50,
            arm_count: 3,
            model_count: 3,
            family: 'anthropic/claude-sonnet',
            resolved_from: 'live',
            reason: 'highest precision within latency target',
          },
          active_model: 'anthropic/claude-sonnet',
          providers: [
            { provider: 'OpenRouter', key_present: true, is_local: false, local_reachable: null, local_models: [] },
            { provider: 'Mens', key_present: true, is_local: true, local_reachable: true, local_models: ['mens-8b'] },
          ],
        };
      case 'get_selection_policy':
        return { chain: ['anthropic/claude-sonnet', 'mens-8b'], free_tier: true };
      case 'list_secret_status':
        return [{ id: 'OPENROUTER_API_KEY', isPresent: true, present: true, preview: 'sk-...real' }];
      case 'inference_provider_status':
        return [
          { provider: 'OpenRouter', key_present: true, is_local: false, local_reachable: null, local_models: [] },
          { provider: 'Mens', key_present: true, is_local: true, local_reachable: true, local_models: ['mens-8b'] },
        ];
      case 'chat_list_sessions':
        return state.sessions;
      case 'chat_create_session': {
        const idx = state.sessions.length + 1;
        const sid = `real-session-${idx}`;
        const created: PersistedChatSession = {
          session_id: sid,
          title: String(args?.title ?? `Session ${idx}`),
          updated_at: new Date().toISOString(),
          message_count: 0,
          conversation_id: idx,
        };
        state.sessions.unshift(created);
        state.messages[sid] = [];
        writeState(state);
        return created;
      }
      case 'chat_get_messages': {
        const sid = String(args?.sessionId ?? state.sessions[0]?.session_id ?? 'real-session-1');
        return state.messages[sid] ?? [];
      }
      case 'chat_append_message': {
        const input = args?.input ?? args ?? {};
        const sid = String(input.session_id ?? state.sessions[0]?.session_id ?? 'real-session-1');
        const id = ++state.nextId;
        const msg: PersistedChatMessage = {
          id,
          role: String(input.role ?? 'user'),
          content: String(input.content ?? ''),
          created_at: new Date().toISOString(),
          task_id: input.task_id ?? null,
          model_id: input.model_id ?? null,
          cost_usd: input.cost_usd ?? null,
          latency_ms: input.latency_ms ?? null,
          already_submitted: Boolean(input.already_submitted),
        };
        (state.messages[sid] ??= []).push(msg);
        const sess = state.sessions.find((s) => s.session_id === sid);
        if (sess) {
          sess.message_count = state.messages[sid].length;
          sess.updated_at = msg.created_at;
        }
        writeState(state);
        return id;
      }
      case 'chat_turn': {
        const input = args?.input ?? {};
        const sid = String(input.session_id ?? state.sessions[0]?.session_id ?? 'real-session-1');
        const isSync = input.execution === 'sync';
        const id = ++state.nextId;
        const modelId = String(input.model_override ?? 'anthropic/claude-sonnet');
        if (!isSync) {
          const taskId = String(++state.nextId);
          state.tasks.push({
            id: taskId,
            description: String(input.content ?? ''),
            status: 'Running',
            session_id: String(input.chat_session_id ?? sid),
          });
          writeState(state);
          return {
            id,
            role: 'assistant',
            content: '',
            created_at: new Date().toISOString(),
            task_id: taskId,
            model_id: modelId,
            events: [],
          };
        }
        const replyContent = `Verified real-backend reply (cost: $0.0014, model: ${modelId}).`;
        const replyMsg: PersistedChatMessage = {
          id,
          role: 'assistant',
          content: replyContent,
          created_at: new Date().toISOString(),
          task_id: null,
          model_id: modelId,
          cost_usd: 0.0014,
          latency_ms: 142,
        };
        (state.messages[sid] ??= []).push(replyMsg);
        const sess = state.sessions.find((s) => s.session_id === sid);
        if (sess) {
          sess.message_count = state.messages[sid].length;
          sess.updated_at = replyMsg.created_at;
        }
        writeState(state);
        return {
          ...replyMsg,
          events: [],
        };
      }
      case 'term_session_list':
        return Object.values(state.termSessions).map((s) => ({
          id: s.id,
          blocks: s.blocks.length,
          security_level: s.security_level,
          control_mode: s.control_mode,
        }));
      case 'term_session_open': {
        const idx = Object.keys(state.termSessions).length + 1;
        const sid = `s-${idx}`;
        state.termSessions[sid] = {
          id: sid,
          blocks: [],
          replay_utf8: `vox (${sid})> \r\n`,
          seq: 1,
          security_level: 'l3_workspace_write',
          control_mode: 'co_pilot',
        };
        writeState(state);
        return sid;
      }
      case 'term_session_attach': {
        const sid = String(args?.sessionId ?? 's-1');
        const sess = state.termSessions[sid] ?? state.termSessions['s-1'];
        return {
          session_id: sess.id,
          blocks: sess.blocks,
          replay_utf8: sess.replay_utf8,
          seq: sess.seq,
        };
      }
      case 'term_session_submit': {
        const sid = String(args?.sessionId ?? 's-1');
        const sess = (state.termSessions[sid] ??= {
          id: sid,
          blocks: [],
          replay_utf8: '',
          seq: 0,
          security_level: 'l3_workspace_write',
          control_mode: 'co_pilot',
        });
        const bid = ++state.nextId;
        const rawLine = String(args?.line ?? '').trim();
        let kind = 'Shell';
        let inputText = rawLine;
        let stream = 'Stdout';
        let outText = `[ok] ${rawLine}\n`;
        if (rawLine === '40 + 2' || rawLine.startsWith('fn ') || rawLine.startsWith('let ')) {
          kind = 'VoxNative';
          outText = 'Int(42)\n';
        } else if (rawLine.startsWith('?')) {
          kind = 'AgentTurn';
          inputText = rawLine.slice(1).trim();
          stream = 'Agent';
          outText = `[routed model=anthropic/claude-sonnet task=Debugging tier=Pro axes=eff:15/prec:70/lat:15] ${inputText}\n`;
        } else if (rawLine.startsWith('!')) {
          inputText = rawLine.slice(1).trim();
          outText = `[ok] ${inputText}\n`;
        }
        const block: PersistedTermBlock = {
          id: bid,
          kind,
          status: 'Ok',
          origin: { kind: 'user' },
          input: inputText,
          output: [{ stream, text: outText, ts_ms: Date.now() }],
          exit_code: 0,
          started_ms: Date.now() - 4,
          ended_ms: Date.now(),
        };
        sess.blocks.push(block);
        sess.seq += 1;
        sess.replay_utf8 += `vox> ${rawLine}\r\n${outText.replace(/\n/g, '\r\n')}`;
        writeState(state);
        return bid;
      }
      case 'term_session_context': {
        const sid = String(args?.sessionId ?? 's-1');
        const maxBlocks = Number(args?.maxBlocks ?? 10);
        const sess = state.termSessions[sid] ?? state.termSessions['s-1'];
        return sess.blocks.slice(-maxBlocks).map((b) => ({
          id: b.id,
          input: redactTerminalText(b.input),
          output: redactTerminalText(b.output.map((o) => o.text).join('')),
          exit_code: b.exit_code,
          origin: b.origin,
        }));
      }
      case 'get_command_catalog':
        return {
          generated_from: 'real-backend',
          entries: COMMAND_CATALOG_ENTRIES,
        };
      case 'get_command_metadata':
        return { safety_class: 'read_only', confirmation_policy: 'none' };
      case 'execute_command':
        return { exit_code: 0, stdout: 'real-backend command ok\n', stderr: '' };
      case 'list_orchestrator_tasks':
        return state.tasks;
      case 'hopper_list':
        return [];
      default:
        return null;
    }
  };

  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
    if (req.method === 'OPTIONS') {
      res.writeHead(204);
      res.end();
      return;
    }

    const chunks: Buffer[] = [];
    req.on('data', (chunk: Buffer) => chunks.push(chunk));
    req.on('end', () => {
      try {
        const rawBody = Buffer.concat(chunks).toString('utf8');
        if (req.method === 'POST' && req.url === '/rpc') {
          const parsed = rawBody ? JSON.parse(rawBody) : {};
          const result = handleRpc(
            String(parsed.cmd ?? ''),
            parsed.args ?? {},
            String(parsed.defaultView ?? 'chat'),
          );
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true, result }));
          return;
        }
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: false, error: 'not_found' }));
      } catch (err) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: false, error: String(err) }));
      }
    });
  });

  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address() as AddressInfo;
  const port = address.port;
  const url = `http://127.0.0.1:${port}`;

  return {
    url,
    port,
    tempDir,
    stateFile,
    readState,
    close: async () => {
      await new Promise<void>((resolve) => server.close(() => resolve()));
      rmSync(tempDir, { recursive: true, force: true });
    },
  };
}
