import type {
  TermAttachmentDto, TermBlockDto, TermBlockStatus, TermControlModeWire,
  TermOutputChunk, TermPromptKindWire, TermSecurityLevelWire,
} from '../../../transport';
import type { TermControlModeId, TermSecurityLevelId } from '../../../lib/terminalBridge';

export interface SessionPaneState {
  sessionId: string | null;
  blocks: TermBlockDto[];
  pinnedBlockIds: number[];
  seq: number;
  altScreen: boolean;
  cwd: string | null;
  gitBranch: string | null;
  securityLevel: TermSecurityLevelId;
  controlMode: TermControlModeId;
  lastAgentMessage: string | null;
  resyncRequired: boolean;
}

const VALID_STREAMS = new Set<TermOutputChunk['stream']>(['Stdout', 'Stderr', 'Agent', 'Meta']);

function normalizeChunk(raw: Partial<TermOutputChunk> | undefined, fallbackTsMs: number): TermOutputChunk {
  const stream = raw?.stream && VALID_STREAMS.has(raw.stream) ? raw.stream : 'Stdout';
  return {
    stream,
    text: typeof raw?.text === 'string' ? raw.text : '',
    ts_ms: typeof raw?.ts_ms === 'number' ? raw.ts_ms : fallbackTsMs,
  };
}

export function normalizeWireBlock(
  raw: Partial<TermBlockDto> & { id: number; input: string },
  nowMs?: number,
): TermBlockDto {
  const ts = nowMs ?? Date.now();
  const rawOutput = Array.isArray(raw.output) ? raw.output : [];
  return {
    id: Number(raw.id),
    kind: raw.kind ?? 'Shell',
    status: (raw.status ?? 'Running') as TermBlockStatus,
    ...(raw.origin !== undefined ? { origin: raw.origin } : {}),
    ...(raw.pinned !== undefined ? { pinned: Boolean(raw.pinned) } : {}),
    ...(raw.credential_prompt !== undefined ? { credential_prompt: Boolean(raw.credential_prompt) } : {}),
    input: typeof raw.input === 'string' ? raw.input : '',
    output: rawOutput.map((c) => normalizeChunk(c, ts)),
    exit_code: raw.exit_code ?? null,
    started_ms: typeof raw.started_ms === 'number' ? raw.started_ms : ts,
    ended_ms: raw.ended_ms ?? null,
    ...(raw.cwd !== undefined ? { cwd: raw.cwd } : {}),
    ...(raw.git_branch !== undefined ? { git_branch: raw.git_branch } : {}),
    ...(raw.prompt_tail !== undefined ? { prompt_tail: raw.prompt_tail } : {}),
    ...(raw.prompt_kind !== undefined ? { prompt_kind: raw.prompt_kind } : {}),
  };
}

const derivePinnedIds = (blocks: readonly TermBlockDto[]): number[] =>
  blocks.filter((b) => b.pinned).map((b) => b.id);

export function createInitialSessionPaneState(overrides?: Partial<SessionPaneState>): SessionPaneState {
  return {
    sessionId: null, blocks: [], pinnedBlockIds: [], seq: 0, altScreen: false,
    cwd: null, gitBranch: null, securityLevel: 'l3_workspace_write', controlMode: 'co_pilot',
    lastAgentMessage: null, resyncRequired: false, ...overrides,
  };
}

const TO_WIRE_SEC: Record<string, TermSecurityLevelWire> = {
  l1_read_only: 'l1_read_only', L1: 'l1_read_only',
  l2_safe_exec: 'l2_workspace_build', l2_workspace_build: 'l2_workspace_build', L2: 'l2_workspace_build',
  l3_workspace_write: 'l3_structured_default', l3_structured_default: 'l3_structured_default', L3: 'l3_structured_default',
  l4_system_exec: 'l4_host_gated', l4_host_gated: 'l4_host_gated', L4: 'l4_host_gated',
  l5_autopilot_floor: 'l5_autopilot_floor', L5: 'l5_autopilot_floor',
};

const FROM_WIRE_SEC: Record<string, TermSecurityLevelId> = {
  l1_read_only: 'l1_read_only', L1: 'l1_read_only',
  l2_workspace_build: 'l2_safe_exec', l2_safe_exec: 'l2_safe_exec', L2: 'l2_safe_exec',
  l3_structured_default: 'l3_workspace_write', l3_workspace_write: 'l3_workspace_write', L3: 'l3_workspace_write',
  l4_host_gated: 'l4_system_exec', l4_system_exec: 'l4_system_exec', L4: 'l4_system_exec',
  l5_autopilot_floor: 'l5_autopilot_floor', L5: 'l5_autopilot_floor',
};

export const toWireSecurityLevel = (ui: string): TermSecurityLevelWire => TO_WIRE_SEC[ui] ?? 'l3_structured_default';
export const fromWireSecurityLevel = (wire: string): TermSecurityLevelId => FROM_WIRE_SEC[wire] ?? 'l3_workspace_write';

export function toWireControlMode(ui: string): TermControlModeWire {
  if (ui === 'autopilot') return 'autopilot';
  return ui === 'co_pilot' || ui === 'copilot' ? 'co_pilot' : 'manual';
}

export function fromWireControlMode(wire: string): TermControlModeId {
  if (wire === 'autopilot') return 'autopilot';
  return wire === 'co_pilot' || wire === 'copilot' ? 'co_pilot' : 'manual';
}

export function hydrateSessionFromAttachment(
  state: SessionPaneState, snap: TermAttachmentDto, nowMs?: number,
): SessionPaneState {
  const ts = nowMs ?? Date.now();
  const normalizedBlocks = Array.isArray(snap.blocks)
    ? snap.blocks.map((b) => normalizeWireBlock(b, ts))
    : state.blocks;
  let next: SessionPaneState = {
    ...state,
    sessionId: snap.session_id ?? state.sessionId,
    blocks: normalizedBlocks,
    pinnedBlockIds: derivePinnedIds(normalizedBlocks),
    seq: typeof snap.seq === 'number' ? Math.max(state.seq, snap.seq) : state.seq,
    altScreen: snap.alt_screen ?? state.altScreen,
    cwd: snap.cwd !== undefined ? snap.cwd : state.cwd,
    gitBranch: snap.git_branch !== undefined ? snap.git_branch : state.gitBranch,
    securityLevel: snap.security_level ? fromWireSecurityLevel(snap.security_level) : state.securityLevel,
    controlMode: snap.control_mode ? fromWireControlMode(snap.control_mode) : state.controlMode,
    resyncRequired: Boolean(snap.resync_required),
  };
  if (Array.isArray(snap.replayed_events)) {
    for (const ev of snap.replayed_events) next = reduceSessionEvent(next, ev, ts);
  }
  return next;
}

export function reduceSessionEvent(state: SessionPaneState, event: unknown, nowMs?: number): SessionPaneState {
  if (!event || typeof event !== 'object') return state;
  const ts = nowMs ?? Date.now();
  const ev = event as Record<string, unknown>;

  if ('BlockOpened' in ev && ev.BlockOpened && typeof ev.BlockOpened === 'object') {
    const p = ev.BlockOpened as Record<string, unknown>;
    const id = Number(p.id);
    const input = typeof p.input === 'string' ? p.input : '';
    const cwd = (p.cwd as string | null | undefined) ?? state.cwd;
    const gitBranch = (p.git_branch as string | null | undefined) ?? state.gitBranch;
    const existingIdx = state.blocks.findIndex((b) => b.id === id);
    const nextBlocks = [...state.blocks];
    if (existingIdx >= 0) {
      const prev = nextBlocks[existingIdx]!;
      nextBlocks[existingIdx] = {
        ...prev, kind: (p.kind as string) ?? prev.kind, input: input || prev.input,
        ...(p.origin != null ? { origin: p.origin as TermBlockDto['origin'] } : {}),
        cwd, git_branch: gitBranch,
      };
    } else {
      nextBlocks.push(normalizeWireBlock({
        id, kind: (p.kind as string) ?? 'Shell', status: 'Running',
        origin: (p.origin as TermBlockDto['origin']) ?? { kind: 'user' },
        input, output: [], exit_code: null, started_ms: ts, ended_ms: null, cwd, git_branch: gitBranch,
      }, ts));
    }
    return { ...state, blocks: nextBlocks, cwd, gitBranch, resyncRequired: false };
  }

  if ('OutputAppended' in ev && ev.OutputAppended && typeof ev.OutputAppended === 'object') {
    const p = ev.OutputAppended as { id: unknown; chunk?: Partial<TermOutputChunk> };
    const id = Number(p.id);
    const chunk = normalizeChunk(p.chunk, ts);
    let matched = false;
    const nextBlocks = state.blocks.map((b) => {
      if (b.id !== id) return b;
      matched = true;
      return { ...b, output: [...b.output, chunk] };
    });
    return matched ? { ...state, blocks: nextBlocks, resyncRequired: false } : state;
  }

  if ('BlockClosed' in ev && ev.BlockClosed && typeof ev.BlockClosed === 'object') {
    const p = ev.BlockClosed as { id: unknown; exit_code?: number | null; status?: TermBlockStatus | null };
    const id = Number(p.id);
    const exitCode = typeof p.exit_code === 'number' ? p.exit_code : null;
    const status: TermBlockStatus = p.status ?? (exitCode === 0 || exitCode === null ? 'Ok' : 'Failed');
    const nextBlocks = state.blocks.map((b) =>
      b.id === id ? { ...b, exit_code: exitCode ?? b.exit_code, status, ended_ms: b.ended_ms ?? ts } : b,
    );
    return { ...state, blocks: nextBlocks, resyncRequired: false };
  }

  if ('BlockAwaitingInput' in ev && ev.BlockAwaitingInput && typeof ev.BlockAwaitingInput === 'object') {
    const p = ev.BlockAwaitingInput as {
      id: unknown; prompt_tail?: string; prompt_kind?: TermPromptKindWire; credential_prompt?: boolean;
    };
    const id = Number(p.id);
    const nextBlocks = state.blocks.map((b) =>
      b.id === id ? {
        ...b, status: 'AwaitingInput' as const, prompt_tail: p.prompt_tail ?? b.prompt_tail,
        prompt_kind: p.prompt_kind ?? b.prompt_kind ?? null, credential_prompt: Boolean(p.credential_prompt),
      } : b,
    );
    return { ...state, blocks: nextBlocks, resyncRequired: false };
  }

  if ('BlockPinned' in ev && ev.BlockPinned && typeof ev.BlockPinned === 'object') {
    const p = ev.BlockPinned as { id: unknown; pinned?: boolean };
    const id = Number(p.id);
    const pinned = Boolean(p.pinned);
    const nextBlocks = state.blocks.map((b) => (b.id === id ? { ...b, pinned } : b));
    const pinnedSet = new Set(state.pinnedBlockIds);
    if (pinned) pinnedSet.add(id); else pinnedSet.delete(id);
    return { ...state, blocks: nextBlocks, pinnedBlockIds: [...pinnedSet], resyncRequired: false };
  }

  if ('AgentMessage' in ev && ev.AgentMessage && typeof ev.AgentMessage === 'object') {
    const p = ev.AgentMessage as { text?: string };
    return { ...state, lastAgentMessage: typeof p.text === 'string' ? p.text : state.lastAgentMessage, resyncRequired: false };
  }

  if ('CwdChanged' in ev && ev.CwdChanged && typeof ev.CwdChanged === 'object') {
    const p = ev.CwdChanged as { cwd?: string; git_branch?: string | null };
    return {
      ...state, cwd: typeof p.cwd === 'string' ? p.cwd : state.cwd,
      gitBranch: p.git_branch !== undefined ? p.git_branch : state.gitBranch, resyncRequired: false,
    };
  }

  if ('AltScreenChanged' in ev && ev.AltScreenChanged && typeof ev.AltScreenChanged === 'object') {
    const p = ev.AltScreenChanged as { active?: boolean };
    return { ...state, altScreen: Boolean(p.active), resyncRequired: false };
  }

  if ('SnapshotResync' in ev && ev.SnapshotResync && typeof ev.SnapshotResync === 'object') {
    const p = ev.SnapshotResync as {
      blocks?: Array<Partial<TermBlockDto> & { id: number; input: string }>;
      resync_required?: boolean; alt_screen?: boolean; cwd?: string | null; git_branch?: string | null;
    };
    const nextBlocks = Array.isArray(p.blocks) ? p.blocks.map((b) => normalizeWireBlock(b, ts)) : state.blocks;
    return {
      ...state, blocks: nextBlocks, pinnedBlockIds: derivePinnedIds(nextBlocks),
      altScreen: Boolean(p.alt_screen), cwd: p.cwd !== undefined ? p.cwd : state.cwd,
      gitBranch: p.git_branch !== undefined ? p.git_branch : state.gitBranch,
      resyncRequired: Boolean(p.resync_required),
    };
  }

  if ('SecurityLevelChanged' in ev && ev.SecurityLevelChanged && typeof ev.SecurityLevelChanged === 'object') {
    const p = ev.SecurityLevelChanged as { security_level?: string };
    return { ...state, securityLevel: p.security_level ? fromWireSecurityLevel(p.security_level) : state.securityLevel, resyncRequired: false };
  }

  if ('ControlModeChanged' in ev && ev.ControlModeChanged && typeof ev.ControlModeChanged === 'object') {
    const p = ev.ControlModeChanged as { control_mode?: string };
    return { ...state, controlMode: p.control_mode ? fromWireControlMode(p.control_mode) : state.controlMode, resyncRequired: false };
  }

  return state;
}

const ALT_ENTER_RE = /\x1b\[\?(?:1049|1047|47)h/g;
const ALT_EXIT_RE = /\x1b\[\?(?:1049|1047|47)l/g;

export function scanAltScreenTransition(chunk: string): boolean | null {
  if (!chunk || !chunk.includes('\x1b[?')) return null;
  let lastIdx = -1;
  let nextState: boolean | null = null;
  for (const m of chunk.matchAll(ALT_ENTER_RE)) {
    if ((m.index ?? -1) > lastIdx) { lastIdx = m.index ?? -1; nextState = true; }
  }
  for (const m of chunk.matchAll(ALT_EXIT_RE)) {
    if ((m.index ?? -1) > lastIdx) { lastIdx = m.index ?? -1; nextState = false; }
  }
  return nextState;
}
