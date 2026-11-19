import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { backendAvailable, BackendUnavailableError } from './lib/backendGuard';
import {
  parsePendingApprovals,
  type McpInvokeResult,
  type PendingApprovalRow,
} from './lib/mcpToolResult';
import { MODEL_LIST_LIMIT } from './config/constants';
import type { ActionManifest } from './types/actionManifest';
import type {
  ArenaCandidateTurnDto,
  ArenaTurnComparisonDto,
  ArenaVoteOutcomeDto,
  BackendStatus,
  ChatRewindOutcomeDto,
  ClearToolResultsOutcomeDto,
  CommandCatalog,
  OpenLocator,
  OpenOutcome,
  OrchestratorStatus,
  RouteExplanation,
  RoutingHealth,
  RoutingSummary,
  Session,
} from './types/tauri';
import type { TaskRow } from './components/surfaces/Tasks/tasksHelpers';
import type { TownScan } from './components/gamify/urbs/types';
import type { TurnEventDto } from './types/dashboard';

// `OpenLocator` / `OpenOutcome` (the `open_locator` IPC DTOs) live in ./types/tauri
// alongside the other Tauri command types; re-exported here for callers of the hub.
export type { OpenLocator, OpenOutcome } from './types/tauri';

/** A single category's pass/fail tally within a harness eval run (Harness Health surface). */
export interface CategorySummaryDto {
  category: string;
  pass_count: number;
  fail_count: number;
}

/** One persisted `vox harness eval --live` run, as returned by `harness_eval_history`. */
export interface HarnessEvalRunDto {
  run_id: string;
  git_sha: string;
  triggered_by: string;
  pass_count: number;
  fail_count: number;
  skip_count: number;
  total_cost_usd: number;
  started_at_ms: number;
  category_breakdown: CategorySummaryDto[];
}

/** A detected regression between two consecutive harness eval runs. */
export interface RegressionFlagDto {
  kind: string;
  previous_run_id: string;
  current_run_id: string;
  previous_git_sha: string;
  current_git_sha: string;
  changed_files: string[];
  flipped_task_ids: string[];
  detail: string;
}

// __VOX_RAW_IPC_BEGIN__
// The ONLY permitted raw Tauri `invoke`/`listen` uses in this file.
// Guarded by src/guards/transportIpcGuard.test.ts.
function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!backendAvailable()) return Promise.reject(new BackendUnavailableError(cmd));
  return args === undefined ? invoke<T>(cmd) : invoke<T>(cmd, args);
}

function safeListen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  if (!backendAvailable()) return Promise.reject(new BackendUnavailableError(`listen:${event}`));
  return listen<T>(event, handler);
}
// __VOX_RAW_IPC_END__

export type { UnlistenFn };

/**
 * Typed IPC command gateway for Vox Axis surfaces and domain transport helpers.
 * Routes 100% through `safeInvoke` (`backendAvailable()` guarded) and threads
 * the active `PermissionMode` onto `invoke_mcp_tool` calls when non-null.
 */
export function invokeCmd<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (
    cmd === 'invoke_mcp_tool' &&
    args !== undefined &&
    !('permissionMode' in args) &&
    currentPermissionMode !== null
  ) {
    return safeInvoke<T>(cmd, { ...args, permissionMode: currentPermissionMode });
  }
  return safeInvoke<T>(cmd, args);
}

/**
 * Typed IPC event listener gateway for Vox Axis surfaces and hooks.
 * Routes 100% through `safeListen` (`backendAvailable()` guarded).
 */
export function listenEvent<T = unknown>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  return safeListen<T>(event, handler);
}

/** Tauri event name carrying the orchestrator status snapshot (see B1 daemon stream). */
export const ORCH_STATUS_EVENT = 'vox://orch-status';

/**
 * Subscribe to the pushed orchestrator-status event stream. The payload is the
 * same status object shape returned by `get_orchestrator_status` / daemon
 * `orch.status()` (fields like `agent_count`). Returns the `UnlistenFn` to call
 * on cleanup. Rejects if not running inside Tauri (caller should fall back to polling).
 */
export function listenOrchStatus(
  onStatus: (status: OrchestratorStatus) => void,
): Promise<UnlistenFn> {
  return safeListen<OrchestratorStatus>(ORCH_STATUS_EVENT, (event) => onStatus(event.payload));
}

/** Tauri event name carrying a single live AgentEvent (see B4 daemon stream). */
export const AGENT_EVENTS_EVENT = 'vox://agent-events';

/**
 * A serialized `AgentEvent` value as pushed by the daemon's
 * `orch.subscribe_events` stream. The `kind.type` discriminator is a snake_case
 * variant name (e.g. "token_streamed", "task_started"); the remaining `kind`
 * fields vary per variant.
 */
export interface AgentEventFrame {
  id: number;
  timestamp_ms: number;
  kind: { type: string; [k: string]: any };
}

/**
 * Subscribe to the pushed live agent-event stream (B4). Each emission carries
 * one `AgentEventFrame`. Returns the `UnlistenFn` to call on cleanup. Rejects if
 * not running inside Tauri (caller should degrade gracefully).
 */
export function listenAgentEvents(
  onEvent: (e: AgentEventFrame) => void,
): Promise<UnlistenFn> {
  return safeListen<AgentEventFrame>(AGENT_EVENTS_EVENT, (event) => onEvent(event.payload));
}

/** Tauri event name carrying a Scientia-queue change ping (see F2 DB watcher). */
export const SCIENTIA_QUEUE_EVENT = 'vox://scientia-queue';

/**
 * Compact payload pushed when the Scientia queue changes. It is a *signal*, not
 * the queue itself: on receipt the UI refetches via the typed read commands.
 */
export interface ScientiaQueuePing {
  signal: number;
  manifest_count: number;
  research_count: number;
}

/**
 * Subscribe to the pushed Scientia-queue change stream (F2). The Rust side polls
 * the canonical DB and emits only when the queue signal flips, so each callback
 * means "something changed — refetch". Returns the `UnlistenFn` for cleanup.
 * Rejects if not running inside Tauri (caller should keep its interval fallback).
 */
export function listenScientiaQueue(
  onChange: (ping: ScientiaQueuePing) => void,
): Promise<UnlistenFn> {
  return safeListen<ScientiaQueuePing>(SCIENTIA_QUEUE_EVENT, (event) => onChange(event.payload));
}

/** Tauri event for one newly-surfaced discovery inbox row (mirrors `scientia.discovery.surfaced`). */
export const SCIENTIA_DISCOVERY_SURFACED_EVENT = 'vox://scientia-discovery-surfaced';

/** One discovery inbox row pushed when a candidate surfaces. */
export interface DiscoverySurfacedPayload {
  id: number;
  publication_id: string;
  surfaced_at_ms: number;
  intake_tier: string;
  signal_codes: string[];
  /** `research` when signal codes include `research_pipeline.*`, else `commit_watcher`. */
  origin: string;
}

/**
 * Subscribe to newly-surfaced discovery candidates. Each emission is one inbox row;
 * refetch or merge locally on receipt. Rejects outside Tauri (interval fallback).
 */
export function listenDiscoverySurfaced(
  onRow: (row: DiscoverySurfacedPayload) => void,
): Promise<UnlistenFn> {
  return safeListen<DiscoverySurfacedPayload>(SCIENTIA_DISCOVERY_SURFACED_EVENT, (event) =>
    onRow(event.payload),
  );
}

/** Tauri event name carrying browser live-view PNG frames (CDP mirror). */
export const BROWSER_FRAME_EVENT = 'vox://browser-frame';
export const PREVIEW_AVAILABLE_EVENT = 'vox://preview-available';

export interface BrowserFramePayload {
  timestamp_ms: number;
  page_id: string | null;
  image_base64: string | null;
  viewport_width: number | null;
  viewport_height: number | null;
  mime?: string | null;
  action_log: string[];
  error: string | null;
}

export interface BrowserPageSummary {
  page_id: string;
  url: string;
  title: string;
}

export interface BrowserPageInfo {
  page_id: string;
  url: string;
  title: string;
  can_go_back: boolean;
  can_go_forward: boolean;
}

export interface PreviewAvailablePayload {
  url: string;
  app_dir: string | null;
  source: string;
}

/**
 * Subscribe to pushed browser frame snapshots (~3s when a session is active).
 */
export function listenBrowserFrames(
  onFrame: (frame: BrowserFramePayload) => void,
): Promise<UnlistenFn> {
  return safeListen<BrowserFramePayload>(BROWSER_FRAME_EVENT, (event) => onFrame(event.payload));
}

export function listenPreviewAvailable(
  onPreview: (payload: PreviewAvailablePayload) => void,
): Promise<UnlistenFn> {
  return safeListen<PreviewAvailablePayload>(PREVIEW_AVAILABLE_EVENT, (event) => onPreview(event.payload));
}

/**
 * Tauri event name emitted when the secretary detects actionable intent in a
 * chat message and proposes a task. Propose-only (Task 0.2): no task exists
 * yet at this point — the frontend must call `secretary_confirm_task` to
 * actually submit it.
 */
export const SECRETARY_PROPOSED_EVENT = 'vox://secretary-proposed-task';

export interface SecretaryProposedPayload {
  /** Client-side proposal id — NOT a hopper/task id, no task has been submitted yet. */
  item_id: string;
  intent: string;
  confidence_pct: number;
  /** Chat session id, passed back to `secretary_confirm_task` on confirm. */
  session_id: string;
}

/**
 * Subscribe to the secretary proposed task event.
 */
export function listenSecretaryProposed(
  onProposed: (payload: SecretaryProposedPayload) => void,
): Promise<UnlistenFn> {
  return safeListen<SecretaryProposedPayload>(SECRETARY_PROPOSED_EVENT, (event) => onProposed(event.payload));
}


export interface ExecuteOutput {
  exit_code: number;
  stdout: string;
  stderr: string;
}

/** Mirrors Rust `IdentitySummaryDto` from `commands/identity.rs`. */
export interface IdentitySummary {
  display_name: string;
  os_user?: string | null;
}

/** Mirrors Rust `LlmSpendDto` from `commands/user_config.rs` (camelCase on wire). */
export interface LlmSpendDto {
  sessionUsd: number;
  dayUsd: number;
  totalUsd: number;
  dailyBudgetUsd: number;
  perSessionBudgetUsd: number;
}

export interface GamifySettingsDto {
  enabled: boolean;
  mode: string;
}

/** Wire DTO returned by `record_gui_event` (camelCase on the Tauri bridge). */
export interface GuiEventResultDto {
  xpGranted: number;
  lumensGranted: number;
  achievementTitle?: string | null;
}

export interface CommandMetadata {
    product_lane: string | null;
    feature_gate: string | null;
    catalog_group: string | null;
    status: string;
}

export interface RegistryOperation {
  path: string[];
  status: string;
  product_lane: string | null;
  feature_gate: string | null;
  catalog_group: string | null;
  surface: string;
}

export interface RegistryFile {
  schema_version: number;
  operations: RegistryOperation[];
}

/** Resolved set of operations keyed by underscore-joined path for O(1) lookup. */
type RegistryIndex = Map<string, RegistryOperation>;

/**
 * T0.3: GUI-selected `PermissionMode` wire string
 * (`"ask" | "accept_edits" | "accept_all" | "plan"`), threaded onto every
 * `invoke_mcp_tool` call this transport makes. Module-level (not per-call)
 * so any surface driving a tool call — not just the approvals view that
 * owns the toggle — picks up the currently selected mode without every
 * call site needing to know about it. `null` (the default) means "no mode
 * selected"; the Rust side treats an absent mode as the fail-safe `ask`
 * default (today's always-park behavior for dangerous tools).
 *
 * Mirrors `vox_orchestrator_mcp::permission_modes::PermissionMode`
 * (`contracts/orchestration/permission-modes.v1.yaml`).
 */
let currentPermissionMode: string | null = null;

/** Read the currently selected `PermissionMode` wire string, or `null`. */
export function getPermissionMode(): string | null {
  return currentPermissionMode;
}

/**
 * Set the `PermissionMode` wire string threaded onto subsequent
 * `invoke_mcp_tool` calls. Pass `null` (or `'ask'`) to return to the
 * fail-safe default.
 */
export function setPermissionMode(mode: string | null): void {
  currentPermissionMode = mode;
}

class VoxTransport {
  private registryCache: RegistryFile | null = null;
  private registryIndex: RegistryIndex | null = null;
  /** Singleton promise so concurrent callers don't double-fetch. */
  private registryFetch: Promise<RegistryFile> | null = null;
  private actionManifestCache: ActionManifest | null = null;
  private actionManifestFetch: Promise<ActionManifest> | null = null;

  async getRegistry(): Promise<RegistryFile> {
    if (this.registryCache) return this.registryCache;
    if (!this.registryFetch) {
      this.registryFetch = safeInvoke<RegistryFile>('get_full_registry')
        .then((r) => {
          this.registryCache = r;
          this.registryIndex = new Map(r.operations.map((op) => [op.path.join('_'), op]));
          return r;
        })
        .catch((err) => {
          this.registryFetch = null;
          throw err;
        });
    }
    return this.registryFetch;
  }

  /** Invalidate caches — call when the registry may have changed on disk. */
  invalidateRegistry() {
    this.registryCache = null;
    this.registryIndex = null;
    this.registryFetch = null;
    this.actionManifestCache = null;
    this.actionManifestFetch = null;
  }

  async getActionManifest(): Promise<ActionManifest> {
    if (this.actionManifestCache) return this.actionManifestCache;
    if (!this.actionManifestFetch) {
      this.actionManifestFetch = safeInvoke<ActionManifest>('get_action_manifest')
        .then((m) => {
          this.actionManifestCache = m;
          return m;
        })
        .catch((err) => {
          this.actionManifestFetch = null;
          throw err;
        });
    }
    return this.actionManifestFetch;
  }

  /** Return all operations for a given product_lane (e.g. "platform", "app"). */
  async getOperationsByLane(lane: string): Promise<RegistryOperation[]> {
    const reg = await this.getRegistry();
    return reg.operations.filter(op => op.product_lane === lane);
  }

  /** Return all operations for a given feature_gate. */
  async getGatedOperations(gate: string): Promise<RegistryOperation[]> {
    const reg = await this.getRegistry();
    return reg.operations.filter(op => op.feature_gate?.includes(gate));
  }

  async resolvePath(actionId: string): Promise<string[]> {
    await this.getRegistry(); // ensures index is built
    const cleanId = actionId.startsWith('vox_') ? actionId.substring(4) : actionId;

    // 1. Exact match on underscore-joined path.
    if (this.registryIndex?.has(cleanId)) {
      return this.registryIndex.get(cleanId)!.path;
    }

    // 2. Try with dashes (CLI convention).
    const dashId = cleanId.replace(/_/g, '-');
    for (const [key, op] of this.registryIndex ?? []) {
      if (op.path.join('-') === dashId) return op.path;
    }

    // 3. Prefix-aware fallback for orchestrator/dei/gamify namespaces.
    const parts = cleanId.split('_');
    if (parts[0] === 'dei' || parts[0] === 'orchestrator') {
      return ['dei', ...parts.slice(1).map(p => p.replace(/_/g, '-'))];
    }
    if (parts[0] === 'gamify') {
      return ['ludus', ...parts.slice(1).map(p => p.replace(/_/g, '-'))];
    }

    return [cleanId.replace(/_/g, '-')];
  }

  async getCatalog(): Promise<CommandCatalog> {
    return safeInvoke<CommandCatalog>('get_command_catalog');
  }

  async listModels(limit = MODEL_LIST_LIMIT) {
    return safeInvoke('list_model_cards', { limit });
  }

  /** Live keyed-provider search (OpenRouter TTL cache + Anthropic/Google/Mens). */
  async searchModels(query: string, limit = 80) {
    return safeInvoke('search_model_cards', { query, limit });
  }

  async getActiveModel() {
    return safeInvoke<string | null>('get_active_model');
  }

  async setActiveModel(modelId: string) {
    return safeInvoke('set_active_model', { modelId });
  }

  async getRoutingSummaryLive(): Promise<RoutingSummary> {
    return safeInvoke<RoutingSummary>('get_routing_summary_live');
  }

  /** How routing would choose now for a mode, task and complexity, from the selector's own ranking. */
  async explainRouting(mode: string, task: string, complexity: number): Promise<RouteExplanation | null> {
    return safeInvoke<RouteExplanation | null>('explain_routing', { mode, task, complexity });
  }

  async getRoutingHealth(): Promise<RoutingHealth | null> {
    return safeInvoke<RoutingHealth | null>('get_routing_health');
  }

  async getAutoModelRecommendation(): Promise<AutoModelRecommendation> {
    return safeInvoke<AutoModelRecommendation>('get_auto_model_recommendation');
  }

  async listOrchestratorTasks(): Promise<TaskRow[]> {
    return safeInvoke<TaskRow[]>('list_orchestrator_tasks');
  }

  async setRoutingPriority(priority: {
    efficiency: number;
    precision: number;
    latency: number;
    availability: number;
    balance: number;
    mobile: number;
  }) {
    return safeInvoke('set_routing_priority', priority);
  }

  /** Read the persisted selection-policy JSON (`{"steps":[...]}`). */
  async getSelectionPolicy(): Promise<string> {
    return safeInvoke<string>('get_selection_policy');
  }

  /** Persist a selection-policy JSON; backend validates it parses as SelectionPolicy. */
  async setSelectionPolicy(json: string): Promise<void> {
    return safeInvoke('set_selection_policy', { json });
  }

  async getModelScoreboard(windowDays = 7) {
    return safeInvoke('get_model_scoreboard', { windowDays });
  }

  async explainModelSelection(task: string, complexity?: number) {
    return safeInvoke('explain_model_selection', { task, complexity });
  }

  async suggestModelForTask(task: string) {
    return safeInvoke('suggest_model_for_task', { task });
  }

  async callTool(name: string, args: Record<string, any> = {}): Promise<ExecuteOutput> {
    if (name === 'vox_list_models') {
      const models = await this.listModels(args.limit ?? MODEL_LIST_LIMIT);
      return { exit_code: 0, stdout: JSON.stringify(models), stderr: '' };
    }
    if (name === 'vox_set_active_model' && args.model_id) {
      await this.setActiveModel(`${args.model_id}`);
      return { exit_code: 0, stdout: 'ok', stderr: '' };
    }
    if (name === 'vox_explain_model' && args.task) {
      const out = await this.explainModelSelection(
        String(args.task),
        Number(args.intelligence ?? 50),
      );
      return { exit_code: 0, stdout: JSON.stringify(out), stderr: '' };
    }
    if (name === 'vox_suggest_model' && args.task) {
      const out = await this.suggestModelForTask(String(args.task));
      return { exit_code: 0, stdout: JSON.stringify(out), stderr: '' };
    }
    const manifest = await this.getActionManifest();
    const canonical = name.startsWith('vox_') ? name : `vox_${name}`;
    const action = manifest.actions.find((a) =>
      a.mcp_name === canonical ||
      a.id === name ||
      a.id === canonical.replace(/^vox_/, '').replace(/_/g, '.') ||
      a.command === `vox ${name.replace(/^vox_/, '').replace(/_/g, ' ')}`
    );
    if (action?.handler_kind === 'mcp') {
      const tool = action.mcp_name ?? canonical;
      const result = await safeInvoke<any>('invoke_mcp_tool', {
        tool,
        args,
        permissionMode: currentPermissionMode,
      });
      const isError =
        result != null &&
        typeof result === 'object' &&
        (result as { is_error?: boolean }).is_error === true;
      return {
        exit_code: isError ? 1 : 0,
        stdout: JSON.stringify(result),
        stderr: '',
      };
    }
    const path = action?.cli_path ?? (await this.resolvePath(name));
    const res = await safeInvoke<ExecuteOutput>('execute_command', {
      path,
      args: { ...args, __argv: args.__argv ?? [] },
    });
    return res;
  }

  /**
   * Run a CLI verb straight through `execute_command` (never the MCP detour of `callTool`).
   * D-03 gate: the backend refuses a destructive verb with a string starting with
   * `CONFIRMATION_REQUIRED_PREFIX` unless `confirmed` is true; pass it only after user consent.
   * An undefined `confirmed` is dropped by JSON, so Rust sees None.
   */
  async executeCommand(
    path: string[],
    args: Record<string, unknown>,
    opts?: { confirmed?: boolean },
  ): Promise<ExecuteOutput> {
    return safeInvoke<ExecuteOutput>('execute_command', {
      path,
      args: { ...args, __argv: args.__argv ?? [] },
      confirmed: opts?.confirmed,
    });
  }

  async getMetadata(path: string[]): Promise<CommandMetadata | null> {
    return safeInvoke('get_command_metadata', { path });
  }

  logFrontend(level: 'error' | 'warn' | 'info', message: string): Promise<void> {
    return safeInvoke('log_frontend', { level, message });
  }

  getGuiPreference(key: string): Promise<string | null> {
    return safeInvoke<string | null>('get_gui_preference', { key });
  }

  setGuiPreference(key: string, value: string): Promise<void> {
    return safeInvoke('set_gui_preference', { key, value });
  }

  invokeMcpTool(
    tool: string,
    args: Record<string, unknown> = {},
  ): Promise<{ is_error?: boolean; result?: unknown }> {
    return safeInvoke('invoke_mcp_tool', {
      tool,
      args,
      permissionMode: currentPermissionMode,
    });
  }

  openLocator(locator: OpenLocator): Promise<OpenOutcome> {
    return safeInvoke<OpenOutcome>('open_locator', { locator });
  }

  voxDocsIndex(): Promise<{ title: string; description: string; path: string }[]> {
    return safeInvoke('vox_docs_index');
  }

  readDocMarkdown(path: string): Promise<string> {
    return safeInvoke('read_doc_markdown', { path });
  }

  writeDocMarkdown(path: string, content: string): Promise<void> {
    return safeInvoke('write_doc_markdown', { path, content });
  }

  /** Recent chat harness eval runs (Harness Health surface). */
  harnessEvalHistory(limit = 50): Promise<HarnessEvalRunDto[]> {
    return safeInvoke('harness_eval_history', { limit });
  }

  /** Detected regressions between consecutive chat harness eval runs. */
  harnessEvalRegressions(): Promise<RegressionFlagDto[]> {
    return safeInvoke('harness_eval_regressions');
  }

  /** VG-1 build-time GUI content manifest (gui-content-manifest.json). */
  voxContentManifest(): Promise<import('./hooks/useContentManifest').ContentManifestEntry[]> {
    return safeInvoke('vox_content_manifest');
  }

  /** Policy catalog rows for federated OmniSearch (see policy_list IPC). */
  listPolicies(): Promise<{ name: string; status?: string }[]> {
    return safeInvoke<{ id: string }[]>('policy_list', { domain: null, group: null }).then(rows => {
      if (!Array.isArray(rows)) return [];
      return rows.map(r => ({ name: r.id }));
    });
  }

  voxSearchQuery(query: string, limit: number, scope: string[]): Promise<{
    hits: unknown[];
    facets_by_source: { value: string; count: number }[];
    facets_by_kind: { value: string; count: number }[];
    total: number;
    next_cursor: number | null;
    corpora: string[];
    repo_truncated: boolean;
  }> {
    return safeInvoke('vox_search_query', { query, limit, scope });
  }

  /** Raw MessagePack orchestrator snapshot (same payload as `get_orchestrator_status`). */
  getOrchestratorStatusBin(): Promise<Uint8Array> {
    return safeInvoke<Uint8Array>('get_orchestrator_status_bin');
  }

  /**
   * Daemon/GUI version mismatch cached by `PersistentDaemon`, or `null` if
   * none. The Rust side serializes `VersionMismatch` as a named
   * `{ daemonVersion, guiVersion }` object (not a positional tuple), so no
   * index-based mapping is needed here — the wire shape already matches.
   */
  getOrchestratorVersionMismatch(): Promise<{ daemonVersion: string; guiVersion: string } | null> {
    return safeInvoke<{ daemonVersion: string; guiVersion: string } | null>(
      'orchestrator_version_mismatch'
    );
  }

  getIdentitySummary(): Promise<IdentitySummary> {
    return safeInvoke<IdentitySummary>('get_identity_summary');
  }

  getLlmSpend(sessionId?: string | null): Promise<LlmSpendDto> {
    return safeInvoke<LlmSpendDto>('get_llm_spend', sessionId != null ? { sessionId } : {});
  }

  getGamifySettings(): Promise<GamifySettingsDto> {
    return safeInvoke<GamifySettingsDto>('get_gamify_settings');
  }

  recordGuiEvent(
    eventType: string,
    metadata?: Record<string, unknown>,
  ): Promise<GuiEventResultDto> {
    return safeInvoke<GuiEventResultDto>('record_gui_event', {
      eventType,
      metadata: metadata ?? null,
    });
  }

  getMemoryStatus(): Promise<{ corpus_counts: Record<string, number> }> {
    return safeInvoke<{ corpus_counts: Record<string, number> }>('get_memory_status');
  }

  doubtTask(taskId: number, reason?: string): Promise<unknown> {
    return safeInvoke('doubt_orchestrator_task', { taskId, reason: reason ?? null });
  }

  overruleTask(taskId: number, reason: string): Promise<unknown> {
    return safeInvoke('overrule_orchestrator_task', { taskId, reason });
  }

  mercatusLoadConfig(): Promise<unknown> {
    return safeInvoke('mercatus_load_config');
  }

  mercatusSaveConfig(config: unknown): Promise<void> {
    return safeInvoke('mercatus_save_config', { config });
  }

  async oratioTranscribe(seconds: number): Promise<{ text: string; raw_text?: string; refined_text?: string | null }> {
    return safeInvoke('oratio_transcribe', { seconds });
  }

  /** Vox Urbs (gamify town): workspace crate/file scan for the town layout. */
  workspaceTownScan(): Promise<TownScan> {
    return safeInvoke<TownScan>('workspace_town_scan');
  }

  /** Vox Urbs: CI fleet status tap (CASTRVM landmark). */
  harnessCiFleetStatus(): Promise<HarnessCiFleetDto> {
    return safeInvoke<HarnessCiFleetDto>('harness_ci_fleet_status');
  }

  /** Vox Urbs: VCS branch/PR status tap (PORTVS landmark). */
  vcsTownStatus(): Promise<HarnessVcsTownDto> {
    return safeInvoke<HarnessVcsTownDto>('vcs_town_status');
  }

  /** Vox Urbs: hopper queue tap (PORTVS ship count). */
  hopperList(): Promise<HarnessHopperItemDto[]> {
    return safeInvoke<HarnessHopperItemDto[]>('hopper_list');
  }

  /** Vox Urbs: MCP aqueduct server health tap (AQVAE landmark). */
  mcpServersStatus(): Promise<Array<{ name: string; ok: boolean }>> {
    return safeInvoke<Array<{ name: string; ok: boolean }>>('mcp_servers_status');
  }

  /** Current per-category/per-source task-policy overrides (Settings panel). */
  getTaskPolicyOverrides(): Promise<TaskPolicyOverrides> {
    return safeInvoke<TaskPolicyOverrides>('get_task_policy_overrides');
  }

  /** `clutch`/`risk` merge onto the scope's existing entry — `undefined` leaves that axis unchanged. */
  setTaskPolicyOverride(
    scopeKind: 'category' | 'source',
    scopeKey: string,
    clutch?: string,
    risk?: string,
  ): Promise<void> {
    return safeInvoke<void>('set_task_policy_override', { scopeKind, scopeKey, clutch, risk });
  }

  clearTaskPolicyOverride(scopeKind: 'category' | 'source', scopeKey: string): Promise<void> {
    return safeInvoke<void>('clear_task_policy_override', { scopeKind, scopeKey });
  }

  termHistoryRecord(command: string, cwd?: string, exitCode?: number): Promise<boolean | null> {
    return termHistoryRecord(command, cwd, exitCode);
  }

  termHistoryQuery(query = '', cwd?: string, limit = 50): Promise<TermHistoryEntryDto[] | null> {
    return termHistoryQuery(query, cwd, limit);
  }

  termHistoryClear(): Promise<boolean> {
    return termHistoryClear();
  }

  termComplete(req: TermCompletionRequestDto): Promise<TermCompletionItemDto[] | null> {
    return termComplete(req);
  }

  termRunbooksList(cwd?: string): Promise<TermRunbookDto[] | null> {
    return termRunbooksList(cwd);
  }

  termSessionVoxify(sessionId: string, shellHint?: string): Promise<boolean> {
    return termSessionVoxify(sessionId, shellHint);
  }

  termAgentPrompt(sessionId: string, prompt: string): Promise<TermAgentTurnDto> {
    return termAgentPrompt(sessionId, prompt);
  }

  termAgentExplainFix(
    sessionId: string,
    blockId: number,
    mode: 'explain' | 'fix',
  ): Promise<TermAgentTurnDto> {
    return termAgentExplainFix(sessionId, blockId, mode);
  }

  termAgentCancel(sessionId: string, blockId: number): Promise<boolean> {
    return termAgentCancel(sessionId, blockId);
  }

  termSlashExec(sessionId: string, rawLine: string): Promise<TermSlashResultDto> {
    return termSlashExec(sessionId, rawLine);
  }

  termBlockFeedback(
    sessionId: string,
    blockId: number,
    verdict: 'accepted' | 'rejected' | 'corrected',
    correctedCommand?: string,
    originalCommand?: string,
  ): Promise<TermFeedbackResultDto> {
    return termBlockFeedback(sessionId, blockId, verdict, correctedCommand, originalCommand);
  }

  listTerminalPendingApprovals(sessionId?: string | null): Promise<PendingApprovalRow[]> {
    return listTerminalPendingApprovals(sessionId);
  }

  resolveTerminalApproval(
    input: ResolveTerminalApprovalInput,
  ): Promise<{ is_error?: boolean; result?: unknown }> {
    return resolveTerminalApproval(input);
  }
}

/** One category/source entry in {@link TaskPolicyOverrides}; either axis may be unset (inherit). */
export interface TaskPolicyEntry {
  clutch?: string;
  risk?: string;
}

/** Mirrors Rust `vox_orchestrator::config::TaskPolicyOverrides`. */
export interface TaskPolicyOverrides {
  category: Record<string, TaskPolicyEntry>;
  source: Record<string, TaskPolicyEntry>;
}

/** Mirrors Rust CI fleet status DTO consumed by the Vox Urbs harness taps. */
export interface HarnessCiFleetDto {
  runners: { name: string; busy: boolean; online: boolean }[];
  queued: number;
}

/** Mirrors Rust VCS town status DTO consumed by the Vox Urbs harness taps. */
export interface HarnessVcsTownDto {
  branches: { name: string; is_head: boolean; track: string }[];
  prs: { number: number; title: string; head_ref: string }[];
  prs_available: boolean;
}

/** Mirrors Rust hopper item DTO consumed by the Vox Urbs harness taps. */
export interface HarnessHopperItemDto {
  state: string;
}

export const voxTransport = new VoxTransport();

// ---------------------------------------------------------------------------
// Vox Console: discovery engine + PTY terminal transport wrappers.
// ---------------------------------------------------------------------------

export interface Suggestion {
  action_id: string;
  completion: string;
  about: string;
  kind?: string;
  category?: string;
  signature?: string;
  score?: number;
}

export interface ActionHelp {
  action_id: string;
  about: string;
  args: { name: string; help: string; required: boolean }[];
  example: string;
}

export function discoverySuggest(typed: string, limit = 8): Promise<Suggestion[]> {
  return safeInvoke<Suggestion[]>('discovery_suggest', { typed, limit });
}

export function discoveryHelp(actionId: string): Promise<ActionHelp | null> {
  return safeInvoke<ActionHelp | null>('discovery_help', { actionId });
}

export function discoveryRecord(
  actionId: string,
  used: boolean,
  nowMs: number,
  dwellMs: number,
): Promise<void> {
  return safeInvoke('discovery_record', { actionId, used, nowMs, dwellMs });
}

export function ptySpawn(tabId: string, cols: number, rows: number): Promise<void> {
  return safeInvoke('pty_spawn', { tabId, cols, rows });
}

export function ptyWrite(tabId: string, data: string): Promise<void> {
  return safeInvoke('pty_write', { tabId, data });
}

export function ptyClose(tabId: string): Promise<void> {
  return safeInvoke('pty_close', { tabId });
}

export type TermBlockOrigin =
  | { kind: 'user' }
  | { kind: 'agent'; agent_id: string }
  | { kind: 'system' }
  | 'user'
  | 'agent'
  | 'system';

export type TermBlockStatus = 'Running' | 'AwaitingInput' | 'Ok' | 'Failed' | 'Cancelled';

export type TermSecurityLevelWire =
  | 'l1_read_only'
  | 'l2_workspace_build'
  | 'l3_structured_default'
  | 'l4_host_gated'
  | 'l5_autopilot_floor';

export type TermControlModeWire = 'manual' | 'co_pilot' | 'autopilot';

export type TermPromptKindWire =
  | 'yes_no'
  | 'credential'
  | 'selection'
  | 'press_enter'
  | 'interactive_tui';

export interface TermOutputChunk {
  stream: 'Stdout' | 'Stderr' | 'Agent' | 'Meta';
  text: string;
  ts_ms: number;
}

export interface TermBlockDto {
  id: number;
  kind: unknown;
  status: TermBlockStatus;
  origin?: TermBlockOrigin;
  pinned?: boolean;
  credential_prompt?: boolean;
  input: string;
  output: TermOutputChunk[];
  exit_code: number | null;
  started_ms: number;
  ended_ms: number | null;
  cwd?: string | null;
  git_branch?: string | null;
  prompt_tail?: string;
  prompt_kind?: TermPromptKindWire | null;
}

export type TermSessionEventWire =
  | {
      BlockOpened: {
        id: number;
        kind?: unknown;
        input?: string;
        origin?: TermBlockOrigin | null;
        cwd?: string | null;
        git_branch?: string | null;
      };
    }
  | {
      OutputAppended: {
        id: number;
        chunk: {
          stream: TermOutputChunk['stream'];
          text: string;
          ts_ms?: number;
        };
      };
    }
  | {
      BlockClosed: {
        id: number;
        exit_code?: number | null;
        status?: TermBlockStatus | null;
      };
    }
  | {
      AgentMessage: {
        text: string;
      };
    }
  | {
      CwdChanged: {
        cwd: string;
        git_branch?: string | null;
      };
    }
  | {
      AltScreenChanged: {
        active: boolean;
      };
    }
  | {
      SnapshotResync: {
        blocks: Array<Partial<TermBlockDto> & { id: number; input: string }>;
        resync_required?: boolean;
        alt_screen?: boolean;
        cwd?: string | null;
        git_branch?: string | null;
      };
    }
  | {
      BlockAwaitingInput: {
        id: number;
        prompt_tail: string;
        prompt_kind: TermPromptKindWire;
        credential_prompt?: boolean;
      };
    }
  | {
      BlockPinned: {
        id: number;
        pinned: boolean;
      };
    }
  | {
      SecurityLevelChanged: {
        security_level: TermSecurityLevelWire | string;
      };
    }
  | {
      ControlModeChanged: {
        control_mode: TermControlModeWire | string;
      };
    };

export interface TermSessionInfoDto {
  id: string;
  blocks: number;
  security_level?: TermSecurityLevelWire;
  control_mode?: TermControlModeWire;
  alt_screen?: boolean;
  cwd?: string | null;
  git_branch?: string | null;
}

export interface TermAttachmentDto {
  session_id: string;
  blocks: TermBlockDto[];
  replay_utf8: string;
  seq: number;
  replayed_events?: TermSessionEventWire[];
  resync_required?: boolean;
  alt_screen?: boolean;
  cwd?: string | null;
  git_branch?: string | null;
  security_level?: TermSecurityLevelWire;
  control_mode?: TermControlModeWire;
}

export interface TermOutputPayload {
  session_id: string;
  seq: number;
  data: string;
}

export interface TermEventPayload {
  session_id: string;
  event: TermSessionEventWire | Record<string, unknown>;
}

export function termSessionOpen(cols: number, rows: number): Promise<string> {
  return safeInvoke<string>('term_session_open', { cols, rows });
}

export function termSessionList(): Promise<TermSessionInfoDto[]> {
  return safeInvoke<TermSessionInfoDto[]>('term_session_list');
}

export function termSessionAttach(
  sessionId: string,
  afterSeq?: number | null,
): Promise<TermAttachmentDto> {
  return safeInvoke<TermAttachmentDto>(
    'term_session_attach',
    afterSeq != null ? { sessionId, afterSeq } : { sessionId },
  );
}

export function termSessionInput(sessionId: string, data: string): Promise<void> {
  return safeInvoke('term_session_input', { sessionId, data });
}

export function termSessionSubmit(sessionId: string, line: string): Promise<number> {
  return safeInvoke<number>('term_session_submit', { sessionId, line });
}

export function termSessionResize(sessionId: string, cols: number, rows: number): Promise<void> {
  return safeInvoke('term_session_resize', { sessionId, cols, rows });
}

export function termSessionClose(sessionId: string): Promise<void> {
  return safeInvoke('term_session_close', { sessionId });
}

export function termSessionSetSecurity(
  sessionId: string,
  securityLevel: TermSecurityLevelWire,
): Promise<void> {
  return safeInvoke('term_session_set_security', { sessionId, securityLevel });
}

export function termSessionSetControl(
  sessionId: string,
  controlMode: TermControlModeWire,
): Promise<void> {
  return safeInvoke('term_session_set_control', { sessionId, controlMode });
}

export function termSessionPin(
  sessionId: string,
  blockId: number,
  pinned: boolean,
): Promise<void> {
  return safeInvoke('term_session_pin', { sessionId, blockId, pinned });
}

export function termSessionPromoteBg(
  sessionId: string,
  blockId?: number | null,
): Promise<boolean> {
  return safeInvoke<boolean>('term_session_promote_bg', {
    sessionId,
    ...(blockId != null ? { blockId } : {}),
  });
}

export interface TermHistoryEntryDto {
  command: string;
  cwd?: string | null;
  exit_code?: number | null;
  ts_ms: number;
}

export type TermCompletionKindDto =
  | 'executable'
  | 'builtin'
  | 'path'
  | 'directory'
  | 'git_ref'
  | 'subcommand'
  | 'flag'
  | 'vox_macro'
  | 'slash'
  | 'history';

export interface TermCompletionItemDto {
  label: string;
  insert_text: string;
  kind: TermCompletionKindDto;
  detail?: string | null;
  score: number;
  replace_start: number;
  replace_end: number;
}

export interface TermCompletionRequestDto {
  line: string;
  cursor: number;
  cwd?: string;
  shellMode?: string;
  limit?: number;
}

export interface TermRunbookParamDto {
  name: string;
  default_value?: string | null;
  description?: string | null;
  options: string[];
}

export interface TermRunbookDto {
  id: string;
  title: string;
  description: string;
  path: string;
  kind: 'vox' | 'markdown';
  command_template: string;
  params: TermRunbookParamDto[];
  tags: string[];
}

export function termHistoryRecord(
  command: string,
  cwd?: string,
  exitCode?: number,
): Promise<boolean | null> {
  return safeInvoke<boolean>('term_history_record', {
    command,
    ...(cwd !== undefined ? { cwd } : {}),
    ...(exitCode !== undefined ? { exitCode } : {}),
  }).catch(() => null);
}

export function termHistoryQuery(
  query = '',
  cwd?: string,
  limit = 50,
): Promise<TermHistoryEntryDto[] | null> {
  return safeInvoke<TermHistoryEntryDto[]>('term_history_query', {
    query,
    ...(cwd !== undefined ? { cwd } : {}),
    limit,
  }).catch(() => null);
}

export function termHistoryClear(): Promise<boolean> {
  return safeInvoke<void>('term_history_clear').then(
    () => true,
    () => false,
  );
}

export function termComplete(
  req: TermCompletionRequestDto,
): Promise<TermCompletionItemDto[] | null> {
  return safeInvoke<TermCompletionItemDto[]>('term_complete', {
    line: req.line,
    cursor: req.cursor,
    ...(req.cwd !== undefined ? { cwd: req.cwd } : {}),
    ...(req.shellMode !== undefined ? { shellMode: req.shellMode } : {}),
    ...(req.limit !== undefined ? { limit: req.limit } : {}),
  }).catch(() => null);
}

export function termRunbooksList(cwd?: string): Promise<TermRunbookDto[] | null> {
  return safeInvoke<TermRunbookDto[]>(
    'term_runbooks_list',
    cwd !== undefined ? { cwd } : {},
  ).catch(() => null);
}

export function termSessionVoxify(
  sessionId: string,
  shellHint?: string,
): Promise<boolean> {
  return safeInvoke<void>('term_session_voxify', {
    sessionId,
    ...(shellHint !== undefined ? { shellHint } : {}),
  }).then(
    () => true,
    () => false,
  );
}

export interface TermAgentTurnDto {
  block_id: number;
  status: string;
  proposed_command?: string | null;
  provider?: string | null;
  model?: string | null;
}

export interface TermSlashResultDto {
  command: string;
  block_id?: number | null;
  delegated_to?: string | null;
  output: string;
}

export interface TermFeedbackResultDto {
  recorded: boolean;
  session_id: string;
  block_id: number;
  verdict: string;
}

export interface ResolveTerminalApprovalInput {
  approvalId: string;
  outcome: 'approved' | 'rejected' | 'modified';
  modifiedCommand?: string;
  steeringNote?: string;
  allowPattern?: string;
}

function offlineOrRethrow<T>(err: unknown, fallback: T): Promise<T> {
  return err instanceof BackendUnavailableError ? Promise.resolve(fallback) : Promise.reject(err);
}

export function termAgentPrompt(sessionId: string, prompt: string): Promise<TermAgentTurnDto> {
  return safeInvoke<TermAgentTurnDto>('term_agent_prompt', { sessionId, prompt }).catch((err) =>
    offlineOrRethrow(err, {
      block_id: 0,
      status: 'offline',
      proposed_command: null,
      provider: null,
      model: null,
    }),
  );
}

export function termAgentExplainFix(
  sessionId: string,
  blockId: number,
  mode: 'explain' | 'fix',
): Promise<TermAgentTurnDto> {
  return safeInvoke<TermAgentTurnDto>('term_agent_explain_fix', {
    sessionId,
    blockId,
    mode,
  }).catch((err) =>
    offlineOrRethrow(err, {
      block_id: 0,
      status: 'offline',
      proposed_command: null,
      provider: null,
      model: null,
    }),
  );
}

export function termAgentCancel(sessionId: string, blockId: number): Promise<boolean> {
  return safeInvoke<boolean>('term_agent_cancel', { sessionId, blockId }).catch(() => false);
}

export function termSlashExec(sessionId: string, rawLine: string): Promise<TermSlashResultDto> {
  const cmd = rawLine.trim().split(/\s+/)[0] ?? '/';
  return safeInvoke<TermSlashResultDto>('term_slash_exec', { sessionId, rawLine }).catch((err) =>
    offlineOrRethrow(err, {
      command: cmd.startsWith('/') ? cmd : `/${cmd}`,
      block_id: null,
      delegated_to: null,
      output: '',
    }),
  );
}

export function termBlockFeedback(
  sessionId: string,
  blockId: number,
  verdict: 'accepted' | 'rejected' | 'corrected',
  correctedCommand?: string,
  originalCommand?: string,
): Promise<TermFeedbackResultDto> {
  return safeInvoke<TermFeedbackResultDto>('term_block_feedback', {
    sessionId,
    blockId,
    verdict,
    ...(correctedCommand !== undefined ? { correctedCommand } : {}),
    originalCommand: originalCommand ?? null,
  }).catch((err) =>
    offlineOrRethrow(err, {
      recorded: false,
      session_id: sessionId,
      block_id: blockId,
      verdict,
    }),
  );
}

export function listTerminalPendingApprovals(
  sessionId?: string | null,
): Promise<PendingApprovalRow[]> {
  return voxTransport
    .invokeMcpTool('vox_pending_approvals', {})
    .then((res) => {
      const rows = parsePendingApprovals(res as McpInvokeResult).filter(
        (r) => r.tool === 'vox_terminal_exec' || r.tool === 'vox_terminal_send_input',
      );
      return sessionId ? rows.filter((r) => !r.session_id || r.session_id === sessionId) : rows;
    })
    .catch(() => []);
}

export function resolveTerminalApproval(
  input: ResolveTerminalApprovalInput,
): Promise<{ is_error?: boolean; result?: unknown }> {
  return voxTransport.invokeMcpTool('vox_resolve_approval', {
    approval_id: input.approvalId,
    outcome: input.outcome,
    ...(input.modifiedCommand ? { modified_command: input.modifiedCommand } : {}),
    ...(input.steeringNote ? { steering_note: input.steeringNote } : {}),
    ...(input.allowPattern ? { allow_pattern: input.allowPattern } : {}),
  });
}

export interface AutoModelRecommendation {
  selected_model_id: string;
  detected_vram_gb: number;
  tier_reason: string;
}

export function getAutoModelRecommendation(): Promise<AutoModelRecommendation> {
  return safeInvoke<AutoModelRecommendation>('get_auto_model_recommendation');
}

/** One cell of the mode × category routing table (`vox_orchestrator::models::leaderboard`). */
export interface ModeRoute {
  mode: string;
  category: string;
  model_id: string | null;
  only_candidate: boolean;
  candidates: number;
  quality: number | null;
  /** `measured` (graded bench) · `benchmark` (public index) · `inherited` · `estimate` · `none`. */
  quality_source: string;
  bench_cases: number;
}

export interface BenchRow {
  model_id: string;
  served_as: string | null;
  cases: number;
  passed: number;
  mean_score: number;
  error_rate: number;
  p50_ttft_ms: number | null;
  p95_latency_ms: number | null;
  tokens_per_sec: number | null;
  cost_per_solve_usd: number | null;
  tokens_per_solve: number | null;
  confidence: string;
  last_run_ms: number;
}

export interface ModelLeaderboard {
  routes: ModeRoute[];
  bench: BenchRow[];
  window_days: number;
}

export function getModelLeaderboard(): Promise<ModelLeaderboard> {
  return safeInvoke<ModelLeaderboard>('get_model_leaderboard');
}

/** What auto-route would pick right now: the backend `decide()` preview (same selector,
 *  scoreboard and benchmark evidence the daemon routes a chat turn with). */
export function getAutoRoutePreview(): Promise<RoutingSummary> {
  return safeInvoke<RoutingSummary>('get_routing_summary_live');
}

/** Remote Axis: current backend (local daemon or remote gateway) and per-surface availability. */
export function axisBackendStatus(): Promise<BackendStatus> {
  return safeInvoke<BackendStatus>('axis_backend_status');
}

/** Connect to a `vox serve` gateway. Rejects with an Axis error envelope (`tls_pin_required`, …). */
export function axisConnectRemote(
  url: string,
  token: string | null,
  acceptFingerprint: string | null,
): Promise<BackendStatus> {
  return safeInvoke<BackendStatus>('axis_connect_remote', { url, token, acceptFingerprint });
}

export function axisDisconnectRemote(): Promise<BackendStatus> {
  return safeInvoke<BackendStatus>('axis_disconnect_remote');
}

export const PTY_OUTPUT_EVENT = 'vox://pty-output';
export const PTY_EXIT_EVENT = 'vox://pty-exit';

export function listenPtyOutput(
  onChunk: (tabId: string, data: string) => void,
): Promise<UnlistenFn> {
  return safeListen<{ tab_id: string; data: string }>(PTY_OUTPUT_EVENT, (e) =>
    onChunk(e.payload.tab_id, e.payload.data),
  );
}

export function listenPtyExit(onExit: (tabId: string) => void): Promise<UnlistenFn> {
  return safeListen<{ tab_id: string }>(PTY_EXIT_EVENT, (e) => onExit(e.payload.tab_id));
}

export const TERM_OUTPUT_EVENT = 'vox://term-output';
export const TERM_EVENT_EVENT = 'vox://term-event';

export function listenTermOutput(
  onChunk: (payload: TermOutputPayload) => void,
): Promise<UnlistenFn> {
  return safeListen<TermOutputPayload>(TERM_OUTPUT_EVENT, (e) => onChunk(e.payload));
}

export function listenTermEvent(
  onEvent: (payload: TermEventPayload) => void,
): Promise<UnlistenFn> {
  return safeListen<TermEventPayload>(TERM_EVENT_EVENT, (e) => onEvent(e.payload));
}

// ---------------------------------------------------------------------------
// Policy enable/disable + edit transport wrappers.
// ---------------------------------------------------------------------------

export function policySetEnabled(id: string, enabled: boolean): Promise<void> {
  return safeInvoke('policy_set_enabled', { id, enabled });
}

export function policyEdit(id: string, title?: string, description?: string): Promise<void> {
  return safeInvoke('policy_edit', { id, title: title ?? null, description: description ?? null });
}

/** Send a free-form note to an agent's A2A inbox. Resolves to the message id. */
export function sendToAgent(agentId: string, body: string): Promise<string> {
  return safeInvoke<string>('send_to_agent', { agentId, body });
}

export interface SendTerminalBlockToAgentArgs {
  agentId: string;
  sessionId: string;
  blockId: number;
  lineStart?: number | null;
  lineEnd?: number | null;
  note?: string | null;
}

/** Resolve and scrub a terminal block on the backend and send it to an agent's durable A2A inbox. */
export function sendTerminalBlockToAgent(args: SendTerminalBlockToAgentArgs): Promise<string> {
  return safeInvoke<string>('send_terminal_block_to_agent', {
    agentId: args.agentId,
    sessionId: args.sessionId,
    blockId: args.blockId,
    lineStart: args.lineStart ?? null,
    lineEnd: args.lineEnd ?? null,
    note: args.note ?? null,
  });
}

export interface ContextBudgetPayload {
  max_context_tokens: number;
  reserved_tokens: number;
  threshold_tokens: number;
  usable_tokens: number;
  strategy: string;
  /** Cumulative input+output tokens used in the session from llm_interactions. Zero when no session. */
  used_tokens: number;
}

export function getContextBudget(sessionId?: string | null): Promise<ContextBudgetPayload> {
  return safeInvoke<ContextBudgetPayload>('get_context_budget', sessionId != null ? { sessionId } : {});
}

export function setContextBudgetStrategy(
  sessionId: string | null | undefined,
  strategy: string,
): Promise<ContextBudgetPayload> {
  return safeInvoke<ContextBudgetPayload>('set_context_budget_strategy', {
    sessionId: sessionId ?? null,
    strategy,
  });
}

export interface SessionContextPinDto {
  id: string;
  label: string;
  scope?: string;
}

export function getSessionContextPins(
  sessionId?: string | null,
): Promise<SessionContextPinDto[]> {
  return safeInvoke<SessionContextPinDto[]>('get_session_context_pins', {
    sessionId: sessionId ?? null,
  });
}

export function setSessionContextPins(
  sessionId: string | null | undefined,
  pins: SessionContextPinDto[],
): Promise<SessionContextPinDto[]> {
  return safeInvoke<SessionContextPinDto[]>('set_session_context_pins', {
    sessionId: sessionId ?? null,
    pins,
  });
}

export function getSessionChatPosture<T = unknown>(
  sessionId?: string | null,
): Promise<T> {
  return safeInvoke<T>('get_session_chat_posture', {
    sessionId: sessionId ?? null,
  });
}

export function setSessionChatPosture<T = unknown>(
  sessionId: string | null | undefined,
  posture: T,
): Promise<T> {
  return safeInvoke<T>('set_session_chat_posture', {
    sessionId: sessionId ?? null,
    posture,
  });
}

export function clearSessionToolResults(
  sessionId: string,
  keepRecentToolOutputs = 2,
): Promise<ClearToolResultsOutcomeDto> {
  return safeInvoke<ClearToolResultsOutcomeDto>('clear_session_tool_results', {
    sessionId,
    keepRecentToolOutputs,
  });
}

export interface ArenaCompareInputDto {
  session_id: string;
  prompt: string;
  primary_model?: string | null;
  challenger_model?: string | null;
  active_skill?: string | null;
}

export interface ArenaVoteInputDto {
  session_id: string;
  prompt: string;
  winner: 'a' | 'b' | 'tie' | string;
  model_a: string;
  content_a: string;
  latency_ms_a?: number;
  model_b: string;
  content_b: string;
  latency_ms_b?: number;
}

export function chatArenaCompare(
  input: ArenaCompareInputDto,
): Promise<ArenaTurnComparisonDto> {
  return safeInvoke<ArenaTurnComparisonDto>('chat_arena_compare', { input });
}

export function chatArenaVote(input: ArenaVoteInputDto): Promise<ArenaVoteOutcomeDto> {
  return safeInvoke<ArenaVoteOutcomeDto>('chat_arena_vote', { input });
}

export function chatForkSession(
  sessionId: string,
  forkFromMessageId: number,
  branchTitle?: string | null,
): Promise<Session> {
  return safeInvoke<Session>('chat_fork_session', {
    sessionId,
    forkFromMessageId,
    branchTitle: branchTitle ?? null,
  });
}

export function chatRewindToTurn(
  sessionId: string,
  targetMessageId: number,
  restoreVcsCheckpoint = true,
): Promise<ChatRewindOutcomeDto> {
  return safeInvoke<ChatRewindOutcomeDto>('chat_rewind_to_turn', {
    sessionId,
    targetMessageId,
    restoreVcsCheckpoint,
  });
}

export type {
  ArenaCandidateTurnDto,
  ArenaTurnComparisonDto,
  ArenaVoteOutcomeDto,
  ChatRewindOutcomeDto,
  ClearToolResultsOutcomeDto,
};

export type PlanNodeStatus =
  | 'pending'
  | 'queued'
  | 'in_progress'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'superseded'
  | 'blocked_on_approval';

export interface PlanNodeDto {
  node_id: string;
  description: string;
  status: PlanNodeStatus;
}

export function listPlanNodes(planSessionId: string, planVersion: number): Promise<PlanNodeDto[]> {
  return safeInvoke<PlanNodeDto[]>('list_plan_nodes', { planSessionId, planVersion });
}

export function updatePlanNode(
  planSessionId: string,
  planVersion: number,
  nodeId: string,
  description: string,
): Promise<void> {
  return safeInvoke<void>('update_plan_node', {
    input: { plan_session_id: planSessionId, plan_version: planVersion, node_id: nodeId, description },
  });
}

export function insertPlanNode(
  planSessionId: string,
  planVersion: number,
  nodeId: string,
  description: string,
  dependsOn: string[] = [],
): Promise<void> {
  return safeInvoke<void>('insert_plan_node', {
    input: {
      plan_session_id: planSessionId,
      plan_version: planVersion,
      node_id: nodeId,
      description,
      depends_on: dependsOn,
    },
  });
}

/** Flips every `blocked_on_approval` node in `planSessionId` back to
 *  `pending` — the `PlanPanel` footer's "Approve" button. Returns the
 *  number of nodes affected. */
export function approvePlanNodes(planSessionId: string): Promise<number> {
  return safeInvoke<number>('approve_plan_nodes', { planSessionId });
}

export interface ActivityRowDto {
  id: number;
  ts_ms: number;
  agent_id?: string;
  session_id?: string;
  kind: string;
  summary: string;
  detail_json: string;
}

export interface ActivityFilterDto {
  agent_id: string | null;
  kind: string | null;
  session_id?: string | null;
  limit: number;
  before_id: number | null;
}

export function activityQuery(filter: ActivityFilterDto): Promise<ActivityRowDto[]> {
  return safeInvoke<ActivityRowDto[]>('activity_query', { filter });
}

export const ACTIVITY_APPENDED_EVENT = 'vox://activity-appended';

export function listenActivityAppended(onAppend: () => void): Promise<UnlistenFn> {
  return safeListen<void>(ACTIVITY_APPENDED_EVENT, () => onAppend());
}

// `getGraphifyStatus` (direct `vox_graphify_status` Tauri command) retired in
// T8 — the GUI now reads status through `useGraphifyStatus` →
// `voxTransport.invokeMcpTool('vox_search_status')` (the shared MCP dispatch).

export interface FeedbackRow {
  feedbackId: string;
  kind: 'clarification' | 'doubt' | 'skill_proposal';
  prompt: string;
  options: string[];
  gates: number[];
  doubtedTaskId: number | null;
  surface: 'needs_you' | 'withheld';
  infoGainBits: number;
}

const toRow = (r: any): FeedbackRow => ({
  feedbackId: r.id,
  kind: r.kind,
  prompt: r.prompt,
  options: r.options ?? [],
  gates: r.gates ?? [],
  doubtedTaskId: r.doubted_task_id ?? null,
  surface: r.surface,
  infoGainBits: r.info_gain_bits ?? 0,
});

export function normalizeFeedback(raw: any): { needsYou: FeedbackRow[]; withheld: FeedbackRow[] } {
  const ny = (raw?.needs_you ?? []).map(toRow).sort((a: FeedbackRow, b: FeedbackRow) => {
    if (a.kind !== b.kind) return a.kind === 'doubt' ? -1 : 1; // doubts pinned top
    return b.infoGainBits - a.infoGainBits;
  });
  return { needsYou: ny, withheld: (raw?.withheld ?? []).map(toRow) };
}

/** Pending working-tree diff (`get_task_diff`); `path` narrows it to one file. */
export function getTaskDiff(path: string | null = null): Promise<string> {
  return safeInvoke<string>('get_task_diff', { path });
}

function extractDaemonError(err: unknown, fallback: string): string {
  if (typeof err === 'string' && err.trim()) return err;
  if (err && typeof err === 'object') {
    const rec = err as { message?: unknown; code?: unknown };
    if (typeof rec.message === 'string' && rec.message.trim()) return rec.message;
    if (typeof rec.code === 'string' && rec.code.trim()) return rec.code;
  }
  return fallback;
}

export async function feedbackList(): Promise<{ needsYou: FeedbackRow[]; withheld: FeedbackRow[] }> {
  // Tauri delivers `invoke_mcp_tool` as a parsed `{ is_error, result }` object (never JSON.parse).
  const res = await safeInvoke<{
    is_error?: boolean;
    result?: { success?: boolean; data?: unknown; error?: unknown } | string;
  }>('invoke_mcp_tool', {
    tool: 'vox_feedback_list',
    args: {},
    ...(currentPermissionMode !== null ? { permissionMode: currentPermissionMode } : {}),
  });
  const parsed = res?.result;
  if (typeof parsed === 'string') {
    if (res?.is_error) throw new Error(parsed || 'Failed to list feedback');
  } else if (res?.is_error || !parsed || parsed.success === false || ('error' in parsed && parsed.error)) {
    throw new Error(extractDaemonError(parsed?.error, 'Failed to list feedback'));
  }
  return normalizeFeedback(typeof parsed === 'object' && parsed ? parsed.data : undefined);
}

export async function feedbackResolve(feedbackId: string, action: Record<string, unknown>): Promise<void> {
  const res = await safeInvoke<{
    is_error?: boolean;
    result?: { success?: boolean; error?: unknown } | string;
  }>('invoke_mcp_tool', {
    tool: 'vox_resolve_feedback',
    args: { feedback_id: feedbackId, action },
    ...(currentPermissionMode !== null ? { permissionMode: currentPermissionMode } : {}),
  });
  const parsed = res?.result;
  if (typeof parsed === 'string') {
    if (res?.is_error) throw new Error(parsed || 'Failed to resolve feedback');
  } else if (res?.is_error || !parsed || parsed.success === false || ('error' in parsed && parsed.error)) {
    throw new Error(extractDaemonError(parsed?.error, 'Failed to resolve feedback'));
  }
}

export function listenFeedbackChanged(onChange: () => void): Promise<UnlistenFn> {
  return safeListen<any>(AGENT_EVENTS_EVENT, (e) => {
    const t = e?.payload?.kind?.type;
    if (t === 'feedback_requested' || t === 'feedback_resolved') onChange();
  });
}

export interface HopperTaskDto {
  item_id: string;
  intent: string;
  priority: number;
  state: string;
  task_id: number;
  session_id?: string | null;
  agent_id?: string | null;
  remote_node?: string | null;
}

/** List hopper task items (see `TasksView` / attention-inbox consumers). */
export function hopperList(): Promise<HopperTaskDto[]> {
  return safeInvoke<HopperTaskDto[]>('hopper_list');
}

/** Mark a hopper to-do done (terminal Done state; distinct from cancel). */
export function hopperMarkDone(itemId: string): Promise<HopperTaskDto> {
  return safeInvoke<HopperTaskDto>('hopper_mark_done', { itemId });
}

/** Submit a new hopper task with optional file/scope affinity. */
export function hopperSubmit(intent: string, affinity: string[] = []): Promise<HopperTaskDto> {
  return safeInvoke<HopperTaskDto>('hopper_submit', { intent, affinity });
}

/** Rename an existing hopper task item. */
export function hopperRename(itemId: string, intent: string): Promise<HopperTaskDto> {
  return safeInvoke<HopperTaskDto>('hopper_rename', { itemId, intent });
}

/** Cancel a hopper task item. */
export function hopperCancel(itemId: string): Promise<HopperTaskDto> {
  return safeInvoke<HopperTaskDto>('hopper_cancel', { itemId });
}

/** Reprioritize a hopper task item (`1` = Urgent, `2` = Normal, `3` = Background). */
export function hopperReprioritize(itemId: string, priority: number): Promise<HopperTaskDto> {
  return safeInvoke<HopperTaskDto>('hopper_reprioritize', { itemId, priority });
}

/** Edit an orchestrator task's description. */
export function editOrchestratorTask(taskId: number | string, description: string): Promise<unknown> {
  return safeInvoke('edit_orchestrator_task', { taskId, description });
}

/** Reorder an orchestrator task's priority (`"urgent"` | `"normal"` | `"background"`). */
export function reorderOrchestratorTask(taskId: number | string, priority: string): Promise<unknown> {
  return safeInvoke('reorder_orchestrator_task', { taskId, priority });
}

/** Cancel an orchestrator task by numeric or string id. */
export function cancelOrchestratorTask(taskId: number | string): Promise<unknown> {
  return safeInvoke('cancel_orchestrator_task', { taskId });
}

/** Flag an orchestrator task for Socratic doubt verification. */
export function doubtOrchestratorTask(taskId: number | string, reason?: string | null): Promise<unknown> {
  return safeInvoke('doubt_orchestrator_task', { taskId, reason: reason ?? null });
}

/** Force immediate verification on an orchestrator task. */
export function forceOrchestratorVerify(taskId: number | string): Promise<unknown> {
  return safeInvoke('force_orchestrator_verify', { taskId });
}

/** Skip verification on an orchestrator task. */
export function skipOrchestratorVerify(taskId: number | string): Promise<unknown> {
  return safeInvoke('skip_orchestrator_verify', { taskId });
}

/** Approve an orchestrator task's plan gate. */
export function approveOrchestratorTaskPlan(taskId: number | string): Promise<unknown> {
  return safeInvoke('approve_orchestrator_task_plan', { taskId });
}

// ---------------------------------------------------------------------------
// Chat turn transport wrapper.
// ---------------------------------------------------------------------------

/** Mirrors Rust `ChatTurnInput` (`commands/chat_turn.rs`). Built exclusively by
 *  `lib/buildChatTurn.ts` so every composer control reaches the backend on both
 *  execution paths. */
export interface ChatTurnInput {
  session_id: string;
  content: string;
  /** Sync = terminal request/response. Background = orchestrator task with a
   *  correlated event stream. Plan = `vox_plan` with `require_approval: true`,
   *  the GUI's `/plan`. Set from the composer's send-mode toggle. */
  execution: 'sync' | 'background' | 'plan';
  model_override?: string | null;
  /** Composer "Run on" tier: local|mesh|cloud|auto. NOT `cognitive_profile`. */
  tier?: string | null;
  clutch?: string | null;
  risk?: string | null;
  context_files: string[];
  active_skill?: string | null;
  skill_exclusions: string[];
  /** Opt-in, non-blocking post-reply grounding/hallucination check (see
   *  `GroundingCheckToggle.tsx` and Rust `ChatTurnInput::grounding_check_enabled`). */
  grounding_check_enabled?: boolean | null;
  priority?: string | null;
  dry_run?: boolean | null;
  allow_duplicate?: boolean | null;
  /** Interaction mode from the composer (plan|act|verify); forwarded as the
   *  `mode` enqueue hint on the background path and sync tool args. */
  mode?: string | null;
  /** Oversight level from the composer (`1`..`5`). */
  oversight_level?: string | null;
  /** Knowledge retrieval scope from the composer (`auto`|`docs`|`code`|`memory`|`off`). */
  kb_scope?: string | null;
  /** The real originating chat session, distinct from `session_id` above
   *  (which can be a synthetic background-session id). See Rust
   *  `ChatTurnInput::chat_session_id`. */
  chat_session_id?: string | null;
  force_research?: boolean | null;
  research_mode?: 'off' | 'auto' | 'fast' | 'deep' | null;
  research_scope?: string | null;
  domain_mode?: string | null;
  site_scope?: string | null;
  waves?: number | null;
  /** End-to-end ChatHop correlation (UUID minted once per send). */
  trace_id?: string | null;
  /** Per-submit turn id (UUID); pairs with Drive `last_turn_id`. */
  turn_id?: string | null;
}

/** Mirrors Rust `ChatTurnDto` returned by `chat_turn`. On the background branch
 *  only `task_id` / `duplicate_of` are meaningful — that path persists no
 *  assistant row (`id` is 0, `content` empty). */
export interface ChatTurnDto {
  id: number;
  role: string;
  content: string;
  created_at: string;
  task_id: string | null;
  model_id?: string;
  latency_ms?: number;
  selection_reason?: string;
  /** True when the opt-in grounding check flagged this reply as low-confidence. */
  grounding_flagged?: boolean;
  /** Set (with a null `task_id`) when the daemon refused a near-duplicate. */
  duplicate_of?: string | null;
  /** Turn events derived from tool results this turn (e.g. a skill-activation
   *  chip) — empty on the background branch. See Rust `turn_event_for_result`. */
  events?: TurnEventDto[];
  /** Set on the `execution: 'plan'` branch only — the DAG `PlanPanel` should
   *  point at. */
  plan_session_id?: string | null;
  plan_version?: number | null;
}

/**
 * The single chat dispatch. `execution: 'sync'` runs the agent loop and returns
 * the persisted reply; `execution: 'background'` enqueues an orchestrator task
 * and returns its `task_id`. Throws on failure — see `lib/chatSend.ts` for the
 * higher-level `sendChatTurn` wrapper consumed by `App.tsx`.
 */
export function chatTurn(input: ChatTurnInput): Promise<ChatTurnDto> {
  return safeInvoke<ChatTurnDto>('chat_turn', { input });
}

// ---------------------------------------------------------------------------
// CodeRabbit sweep transport wrappers.
// ---------------------------------------------------------------------------

export function codeRabbitTokenPresent(): Promise<boolean> {
  return safeInvoke<boolean>('coderabbit_token_present');
}

/** Generic over the view's own `Report` shape — this hub has no opinion on it. */
export function codeRabbitReport<T = unknown>(): Promise<T> {
  return safeInvoke<T>('coderabbit_report');
}

export interface CodeRabbitSweepArgs {
  since: string;
  cap: number;
  rankWeights: string;
  top: number | null;
  fullRepo: boolean;
  [key: string]: unknown;
}

/** Generic over the view's own `Manifest` shape — this hub has no opinion on it. */
export function codeRabbitPlan<T = unknown>(args: CodeRabbitSweepArgs): Promise<T> {
  return safeInvoke<T>('coderabbit_plan', args);
}

export function codeRabbitRunAsync(args: CodeRabbitSweepArgs): Promise<void> {
  return safeInvoke('coderabbit_run_async', args);
}

// ---------------------------------------------------------------------------
// Scientia publication pipeline transport wrappers.
// ---------------------------------------------------------------------------

export function promoteScientiaCandidate<T = unknown>(
  candidateId: string,
  venue?: string | null,
): Promise<T> {
  return safeInvoke<T>('promote_scientia_candidate', {
    candidateId,
    venue: venue ?? null,
  });
}

export function scientiaPublicationManuscript<T = unknown>(publicationId: string): Promise<T> {
  return safeInvoke<T>('scientia_publication_manuscript', { publicationId });
}

export function scientiaPublicationLatex<T = unknown>(publicationId: string): Promise<T> {
  return safeInvoke<T>('scientia_publication_latex', { publicationId });
}

export function scientiaPublicationFindingPage<T = unknown>(publicationId: string): Promise<T> {
  return safeInvoke<T>('scientia_publication_finding_page', { publicationId });
}

export function scientiaPublicationExtractClaims<T = unknown>(publicationId: string): Promise<T> {
  return safeInvoke<T>('scientia_publication_extract_claims', { publicationId });
}

export function scientiaPublicationReplay<T = unknown>(
  publicationId: string,
  measuredScore: number,
): Promise<T> {
  return safeInvoke<T>('scientia_publication_replay', { publicationId, measuredScore });
}

export function scientiaPublicationClaims<T = unknown>(publicationId: string): Promise<T> {
  return safeInvoke<T>('scientia_publication_claims', { publicationId });
}

export function scientiaPublicationVenueRecommend<T = unknown>(
  candidateClass: string,
): Promise<T> {
  return safeInvoke<T>('scientia_publication_venue_recommend', { candidateClass });
}

export function scientiaPublicationFlywheelAdvance<T = unknown>(
  publicationId: string,
  targetStage?: string | null,
): Promise<T> {
  return safeInvoke<T>('scientia_publication_flywheel_advance', {
    publicationId,
    targetStage: targetStage ?? null,
  });
}

export function scientiaPublicationArxivBundle<T = unknown>(publicationId: string): Promise<T> {
  return safeInvoke<T>('scientia_publication_arxiv_bundle', { publicationId });
}

export function scientiaPublicationArxivReadiness<T = unknown>(publicationId: string): Promise<T> {
  return safeInvoke<T>('scientia_publication_arxiv_readiness', { publicationId });
}

export function scientiaPublicationClassify<T = unknown>(args: {
  publicationId: string;
  candidateClass?: string | null;
  publicationTier?: string | null;
  arxivPrimaryCategory?: string | null;
  arxivSecondaryCategories?: string | null;
  licenseSpdx?: string | null;
  orcid?: string | null;
  rorAffiliation?: string | null;
  prereservedDoi?: string | null;
  arxivId?: string | null;
  autofillImrad?: boolean | null;
}): Promise<T> {
  return safeInvoke<T>('scientia_publication_classify', {
    publicationId: args.publicationId,
    candidateClass: args.candidateClass ?? null,
    publicationTier: args.publicationTier ?? null,
    arxivPrimaryCategory: args.arxivPrimaryCategory ?? null,
    arxivSecondaryCategories: args.arxivSecondaryCategories ?? null,
    licenseSpdx: args.licenseSpdx ?? null,
    orcid: args.orcid ?? null,
    rorAffiliation: args.rorAffiliation ?? null,
    prereservedDoi: args.prereservedDoi ?? null,
    arxivId: args.arxivId ?? null,
    autofillImrad: args.autofillImrad ?? null,
  });
}

export function scientiaPublicationStageAdvance<T = unknown>(args: {
  publicationId: string;
  targetStage?: string | null;
  orcid?: string | null;
  prereservedDoi?: string | null;
  arxivId?: string | null;
  autofillImrad?: boolean | null;
}): Promise<T> {
  return safeInvoke<T>('scientia_publication_stage_advance', {
    publicationId: args.publicationId,
    targetStage: args.targetStage ?? null,
    orcid: args.orcid ?? null,
    prereservedDoi: args.prereservedDoi ?? null,
    arxivId: args.arxivId ?? null,
    autofillImrad: args.autofillImrad ?? null,
  });
}

export function scientiaV76AuditDiff<T = unknown>(): Promise<T> {
  return safeInvoke<T>('scientia_v76_audit_diff');
}

export * from './transport/t3Wave3Rpc';
export * from './transport/conciliumRpc';



