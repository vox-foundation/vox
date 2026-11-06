// crates/vox-gui/ui/e2e/lib/tauriMockRich.ts
/**
 * Dense, overflow-shaped mock for the review-bundle capture matrix, layered
 * over installTauriMock. Sparse mocks are why occlusion/clipping never
 * showed: 44 tasks with 120+-char unicode/RTL titles, 32 models, 6
 * providers, and a dense msgpack orchestrator snapshot make truncation,
 * overlap, and z-fighting visible.
 *
 * Serialization contract (mirrors tauriMockShared.mockInitScript):
 * addInitScript serialises function SOURCE only, so richMockInitScript()
 * composes one self-contained script string:
 *   1. mockInitScript(installTauriMock, viewKey)   // shared + base mock
 *   2. window.__VOX_RICH_BUILD__      = <buildRichDataset source>
 *   3. window.__VOX_RICH_STATUS_BIN__ = new Uint8Array([...])  // msgpack,
 *      encoded HERE in Node — @msgpack/msgpack can't run inside the page
 *   4. (<installTauriMockRich source>)(viewKey)    // wraps base invoke
 */
import type { Page } from '@playwright/test';
import { encode } from '@msgpack/msgpack';
import { mockInitScript } from './tauriMockShared';
import { installTauriMock } from './tauriMock';

/** Self-contained (no captured module scope) — stringified into the page. */
export function buildRichDataset() {
  const long = (s: string, n: number) => s.repeat(Math.ceil(n / s.length)).slice(0, n);
  const canonicalTaskTitles = [
    'Verify AST symbol-slice extraction and K-prefix prompt cache breakpoints across vox-orchestrator and vox-llm-egress',
    'Upgrade Roman Civilization Codebase Simulator (VRBS ROMA) isometric camera centering and strategic minimap radar',
    'Audit 32 SURFACE_REGISTRY views via @ngi/platform-ssot Computer Vision Flywheel for zero horizontal clipping',
    'Run 5-Spoke MENS QLoRA adapter evaluation against canonical_adversarial_and_real_query_spoke_suite_64',
    'Synchronize bi-directional CAS task ledger and session context pins across ChatExecutionRail and PlanPanel',
  ];
  const hopperTasks = Array.from({ length: 44 }, (_, i) => ({
    item_id: `hop-rich-${i + 1}`,
    intent:
      i === 7
        ? long(`Refactor the international pipeline № ${i} — очень длинное название задачи с юникодом `, 140)
        : i === 5
          ? `משימה ${i} — bidirectional text sample with a fairly long tail describing acceptance criteria in detail`
          : i === 0
            ? long(`Task ${i + 1}: verify cross-crate AST symbol grounding, K-prefix cache breakpoints, and 32-surface CV Flywheel invariants `, 128)
            : canonicalTaskTitles[i % canonicalTaskTitles.length]!,
    priority: i % 3,
    state: i % 6 === 0 ? 'done' : i % 4 === 0 ? 'assigned' : 'inbox',
    task_id: 9100 + i,
    session_id: i % 2 ? `session-${(i % 6) + 1}` : null,
    agent_id: i % 3 ? `agent-${(i % 6) + 1}` : null,
    remote_node: i % 5 === 0 ? 'mesh-eu-west-1.vox.internal' : null,
  }));
  const chatSessions = Array.from({ length: 14 }, (_, i) => ({
    session_id: `session-${i + 1}`,
    title: `Session ${i + 1}: Architecture & SOTA Harness Verification (${i + 1})`,
    message_count: 3 + i * 7,
    updated_at: 'now',
    conversation_id: i + 1,
  }));
  const hostedCatalog = [
    'anthropic/claude-sonnet-4-5',
    'anthropic/claude-opus-4-1',
    'openai/gpt-5-codex',
    'google/gemini-2.5-pro',
    'deepseek/deepseek-r1-0528',
    'mistralai/codestral-2508',
    'meta-llama/llama-4-maverick-17b',
    'qwen/qwen3-coder-480b-a35b',
  ];
  const models = Array.from({ length: 32 }, (_, i) => {
    const id = i % 5 === 0
      ? ['ollama/qwen3-coder-14b-', 'mens/vox-qwen3-8b-r', 'mesh/populi-llama4-node-'][i % 3] + i
      : i < hostedCatalog.length
        ? hostedCatalog[i]!
        : `${hostedCatalog[i % hostedCatalog.length]}-v${i}`;
    return {
      id, model_id: id,
      display_name: id,
      provider: ['anthropic', 'openai', 'google', 'ollama', 'mistralai', 'meta-llama'][i % 6],
      tier: ['Frontier', 'Fast', 'Budget'][i % 3],
      cost_per_1k: i * 0.0007,
      max_tokens: 8192 * ((i % 4) + 1),
      is_free: i % 8 === 0,
      latency_p50_ms: 200 + i * 13,
      success_rate: 0.9 + (i % 10) / 100,
      quality_score: 0.82 + (i % 17) / 100,
    };
  });
  const providers = Array.from({ length: 6 }, (_, i) => ({
    provider: ['OpenRouter', 'Anthropic', 'OpenAI', 'Ollama', 'MENS Local Cluster', 'Populi Mesh'][i],
    key_present: i !== 2,
    is_local: i >= 3,
    local_reachable: i >= 3 ? i !== 5 : null,
    local_models: i >= 3 ? ['local-llama-small', 'qwen-coder-7b', 'mens-8b-instruct'] : [],
  }));
  return { hopperTasks, chatSessions, models, providers };
}

export const RICH_DATASET = buildRichDataset();

/** Dense OrchestratorStatus for dashboard/flow/console. Encoded to msgpack
 * at compose time — NOT inside the page. */
export function buildRichOrchestratorStatus() {
  const agentTasks = [
    'Verify Roman Civilization Simulator (VRBS ROMA) isometric camera bounds & minimap frustum',
    'Synthesize K-prefix prompt cache breakpoints in vox-llm-egress::wire',
    'Evaluate 5-Spoke MENS QLoRA adapter pass@1 across 64 adversarial & real queries',
    'Audit 32 SURFACE_REGISTRY views via @ngi/platform-ssot Computer Vision Flywheel',
    'Synchronize bi-directional CAS task ledger and session context pins',
    'Enforce TOESTUB hollow-body & paper-code reachability detectors across workspace',
    'Validate AST symbol-slice extraction and module-graph integrity in vox-orchestrator',
    'Stream single-pass SSE delta.tool_calls across OpenRouter and local MENS backends',
    'Verify three-role fenced PostgreSQL telemetry &BLAKE3 leaderboard snapshots',
  ];
  const agents = Array.from({ length: 9 }, (_, i) => ({
    id: i + 1,
    codename: ['Aquila', 'Bellona', 'Cato', 'Drusus', 'Egeria', 'Faunus', 'Gallus', 'Hersilia', 'Iovis'][i],
    name: `agent-${i + 1}`,
    in_progress: i % 3 !== 0,
    paused: i === 4,
    progress: i % 3 === 0 ? null : ((i * 11) % 100) / 100,
    current_phase: ['plan', 'implement', 'verify', 'review'][i % 4],
    task_description: agentTasks[i % agentTasks.length]!,
    cost: i * 0.42,
    budget: i % 2 ? 5 : null,
    eta: `${5 + i}m`,
    active_skill: i % 2 ? 'superpowers:test-driven-development' : undefined,
  }));
  const recent_events = Array.from({ length: 24 }, (_, i) => ({
    id: i + 1,
    kind: (['task_started', 'phase_change', 'task_completed', 'doubt_raised'] as const)[i % 4],
    tag: `agent-${(i % 9) + 1}`,
    title: `Event ${i + 1}: ${['started', 'phase → verify', 'completed', 'doubt raised'][i % 4]}`,
    body: 'A stream event body long enough to exercise two-line clamping in the console event feed rendering path.',
    timestamp: 'now',
  }));
  return {
    agent_count: agents.length,
    total_queued: 44, total_in_progress: 6, total_completed: 128, total_doubted: 3,
    total_weighted_load: 7.5, predicted_load: 8.2,
    agents, recent_events,
    alerts: [
      { id: 'al-1', level: 'warn', title: 'Budget 80% consumed', body: 'Exploration spend approaching the configured cap.' },
      { id: 'al-2', level: 'ok', title: 'Mesh healthy', body: 'All peers reachable.' },
    ],
    peers: [
      { id: 'node-a', status: 'online' },
      { id: 'node-b', status: 'online' },
      { id: 'node-remote-very-long-hostname.example.internal', status: 'degraded' },
    ],
    total_cost: 12.34, budget_cap: 50, mesh_throughput: 3.2,
  };
}

/** Self-contained installer: runs AFTER installTauriMock in the same init
 * script; wraps the base invoke and overrides only the dense commands. */
export function installTauriMockRich(viewKey: string): void {
  const internals = (window as any).__TAURI_INTERNALS__;
  const base: ((cmd: string, args?: any) => Promise<unknown>) | undefined = internals?.invoke;
  const build = (window as any).__VOX_RICH_BUILD__;
  const statusBin = (window as any).__VOX_RICH_STATUS_BIN__;
  if (typeof base !== 'function' || typeof build !== 'function' || !statusBin) {
    throw new Error('installTauriMockRich must be injected via addRichMockInitScript after installTauriMock');
  }
  void viewKey; // navigation is seeded by installTauriMock
  const data = build();
  internals.invoke = async (cmd: string, args?: any) => {
    switch (cmd) {
      case 'hopper_list':
        return data.hopperTasks.map((t: any) => ({ ...t }));
      case 'list_orchestrator_tasks':
        return [
          {
            id: 200,
            description: 'Consolidate Loquela toolbar & floating Execution Posture popover',
            priority: 'high',
            lifecycle: 'done',
            agent_id: 1,
            session_id: 'session-1',
            estimated_complexity: 2,
            depends_on: [],
            write_files: ['crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx'],
            remote_node: null,
            origin: 'orchestrator',
          },
          {
            id: 201,
            description: 'Audit & make Chat task list and session context editable',
            priority: 'high',
            lifecycle: 'in_progress',
            agent_id: 1,
            session_id: 'session-1',
            estimated_complexity: 2,
            depends_on: [200],
            write_files: ['crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx'],
            remote_node: null,
            origin: 'orchestrator',
          },
          {
            id: 202,
            description: 'Verify CV Flywheel pixel & DOM invariants across 32 GUI surfaces',
            priority: 'normal',
            lifecycle: 'pending',
            agent_id: 2,
            session_id: 'session-1',
            estimated_complexity: 1,
            depends_on: [201],
            write_files: [],
            remote_node: null,
            origin: 'orchestrator',
          },
        ];
      case 'chat_list_sessions': {
        const limit = typeof args?.limit === 'number' ? args.limit : data.chatSessions.length;
        return data.chatSessions.slice(0, limit).map((s: any) => ({ ...s }));
      }
      case 'list_model_cards':
        return data.models;
      case 'inference_provider_status':
        return data.providers;
      case 'explain_routing':
        return {
          mode: args?.mode ?? 'efficiency',
          task: args?.task ?? 'codegen',
          complexity: args?.complexity ?? 7,
          chosen: 'anthropic/claude-sonnet-4-5',
          only_candidate: false,
          total_models: 32,
          candidates: [
            {
              id: 'anthropic/claude-sonnet-4-5',
              provider: 'anthropic',
              tier: 'Frontier',
              is_free: false,
              price_out_per_m: 3.0,
              score: 0.94,
              quality: { value: 0.94, source: 'benchmark', index: 89.4, inherited_from: null },
              parts: { quality: 0.45, efficiency: 0.25, latency: 0.16, other: 0.08, bonuses: 0 },
            },
            {
              id: 'google/gemini-2.5-pro',
              provider: 'google',
              tier: 'Frontier',
              is_free: false,
              price_out_per_m: 2.5,
              score: 0.89,
              quality: { value: 0.91, source: 'benchmark', index: 86.8, inherited_from: null },
              parts: { quality: 0.42, efficiency: 0.24, latency: 0.15, other: 0.08, bonuses: 0 },
            },
            {
              id: 'mens/vox-qwen3-8b-r3',
              provider: 'mens',
              tier: 'Fast',
              is_free: true,
              price_out_per_m: 0.0,
              score: 0.86,
              quality: { value: 0.84, source: 'benchmark', index: 81.2, inherited_from: null },
              parts: { quality: 0.36, efficiency: 0.32, latency: 0.14, other: 0.04, bonuses: 0 },
            },
          ],
          excluded: [
            { reason: 'flagship_excluded_by_mode', count: 3, examples: ['anthropic/claude-opus-4-1', 'openai/gpt-5-pro'] },
            { reason: 'superseded', count: 2, examples: ['anthropic/claude-3-5-sonnet'] },
          ],
        };
      case 'get_routing_health':
        return {
          schema_version: 1,
          checked_at_unix: 1729363200,
          models: 32,
          cloud_models: 24,
          benchmarked: 28,
          inherited: 4,
          unknown_tier_cloud: 0,
          quality_scale: 'derived',
          price_bands: 'derived',
          efficient_pick: 'anthropic/claude-sonnet-4-5',
          violations: [],
        };
      case 'get_gamify_settings':
        return { enabled: true, mode: 'balanced' };
      case 'get_orchestrator_status_bin':
        return statusBin;
      case 'invoke_mcp_tool':
        if (args?.tool === 'vox_pending_approvals') {
          return {
            tool: 'vox_pending_approvals',
            is_error: false,
            result: {
              data: {
                approvals: [
                  {
                    approval_id: 'apr-201',
                    tool: 'vox_run_shell',
                    summary: 'vox run scripts/migrate-schema.vox --apply',
                    requested_at_ms: Date.now() - 12_000,
                  },
                ],
              },
            },
          };
        }
        return base(cmd, args);
      default:
        return base(cmd, args);
    }
  };
}

/** Compose the full self-contained init script (exported for unit tests). */
export function richMockInitScript(viewKey: string): string {
  const statusBytes = Array.from(encode(buildRichOrchestratorStatus())).join(',');
  return [
    mockInitScript(installTauriMock, viewKey),
    `window.__VOX_RICH_BUILD__ = ${buildRichDataset.toString()};`,
    `window.__VOX_RICH_STATUS_BIN__ = new Uint8Array([${statusBytes}]);`,
    `(${installTauriMockRich.toString()})(${JSON.stringify(viewKey)});`,
  ].join('\n');
}

/** The ONLY supported way to inject the rich mock into a Playwright page. */
export async function addRichMockInitScript(page: Page, viewKey: string): Promise<void> {
  await page.addInitScript({ content: richMockInitScript(viewKey) });
}
