import { termSessionSetControl, type TermBlockDto } from '../transport';

export type TermSecurityLevelId =
  | 'L1'
  | 'L2'
  | 'L3'
  | 'L4'
  | 'L5'
  | 'l1_read_only'
  | 'l2_safe_exec'
  | 'l3_workspace_write'
  | 'l4_system_exec'
  | 'l5_autopilot_floor';

export type TermControlModeId = 'manual' | 'copilot' | 'autopilot' | 'co_pilot';
export type TermShellModeId = 'default' | 'nu' | 'vox';

export interface TerminalBridgeState {
  activeSessionId: string | null;
  cwd?: string | null;
  blocks: TermBlockDto[];
  pinnedBlockIds: number[];
  securityLevel: TermSecurityLevelId;
  controlMode: TermControlModeId;
  shellMode: TermShellModeId;
}

const DEFAULT_STATE: TerminalBridgeState = {
  activeSessionId: null,
  cwd: null,
  blocks: [],
  pinnedBlockIds: [],
  securityLevel: 'l3_workspace_write',
  controlMode: 'co_pilot',
  shellMode: 'default',
};

export interface ConsoleFocusTarget {
  sessionId: string | null;
  blockId?: number | null;
}

export interface RunbookLaunchRequest {
  runbookId?: string;
  command?: string;
  mode: 'insert' | 'run' | 'open_rail';
}

let currentState: TerminalBridgeState = { ...DEFAULT_STATE };
let pendingConsoleFocus: ConsoleFocusTarget | null = null;
let pendingOpenRunbook: RunbookLaunchRequest | null = null;
const stateListeners = new Set<(state: TerminalBridgeState) => void>();
const commandListeners = new Set<(cmd: string, origin?: string) => void>();
const attentionListeners = new Set<(chipOrPrompt: string) => void>();
const consoleFocusListeners = new Set<(target: ConsoleFocusTarget) => void>();
const openRunbookListeners = new Set<(req: RunbookLaunchRequest) => void>();

export function getTerminalBridgeState(): TerminalBridgeState {
  return currentState;
}

export function publishTerminalSnapshot(patch: Partial<TerminalBridgeState>): void {
  currentState = { ...currentState, ...patch };
  for (const listener of stateListeners) {
    listener(currentState);
  }
}

export const publishTerminalBridgeState = publishTerminalSnapshot;

export function subscribeTerminalBridge(
  listener: (state: TerminalBridgeState) => void,
): () => void {
  stateListeners.add(listener);
  return () => {
    stateListeners.delete(listener);
  };
}

export function requestTerminalCommand(cmd: string, origin?: string): void {
  for (const listener of commandListeners) {
    listener(cmd, origin);
  }
}

export function onTerminalCommand(
  listener: (cmd: string, origin?: string) => void,
): () => void {
  commandListeners.add(listener);
  return () => {
    commandListeners.delete(listener);
  };
}

export function requestChatAttention(chipOrPrompt: string): void {
  for (const listener of attentionListeners) {
    listener(chipOrPrompt);
  }
}

export function onChatAttention(
  listener: (chipOrPrompt: string) => void,
): () => void {
  attentionListeners.add(listener);
  return () => {
    attentionListeners.delete(listener);
  };
}

export function requestConsoleBlockFocus(target: ConsoleFocusTarget): void {
  pendingConsoleFocus = { ...target };
  for (const listener of consoleFocusListeners) {
    listener(pendingConsoleFocus);
  }
}

export function consumePendingConsoleBlockFocus(): ConsoleFocusTarget | null {
  const next = pendingConsoleFocus;
  pendingConsoleFocus = null;
  return next;
}

export function onConsoleBlockFocus(
  listener: (target: ConsoleFocusTarget) => void,
): () => void {
  consoleFocusListeners.add(listener);
  return () => {
    consoleFocusListeners.delete(listener);
  };
}

export function requestOpenRunbook(req: RunbookLaunchRequest): void {
  pendingOpenRunbook = { ...req };
  for (const listener of openRunbookListeners) {
    listener(pendingOpenRunbook);
  }
}

export function consumePendingOpenRunbook(): RunbookLaunchRequest | null {
  const next = pendingOpenRunbook;
  pendingOpenRunbook = null;
  return next;
}

export function onOpenRunbook(
  handler: (req: RunbookLaunchRequest) => void,
): () => void {
  openRunbookListeners.add(handler);
  return () => {
    openRunbookListeners.delete(handler);
  };
}

export function triggerManualTakeover(sessionId?: string | null): void {
  const sid = sessionId ?? currentState.activeSessionId;
  publishTerminalSnapshot({ controlMode: 'manual' });
  if (sid && typeof termSessionSetControl === 'function') {
    void Promise.resolve(termSessionSetControl(sid, 'manual')).catch(() => {});
  }
}

export function resetTerminalBridgeForTests(): void {
  currentState = { ...DEFAULT_STATE };
  pendingConsoleFocus = null;
  pendingOpenRunbook = null;
  stateListeners.clear();
  commandListeners.clear();
  attentionListeners.clear();
  consoleFocusListeners.clear();
  openRunbookListeners.clear();
}

export interface TermMentionOption {
  token: '@term' | '@term:last' | '@term:failed' | '@term:pinned';
  label: string;
  desc: string;
}

export const TERM_MENTION_OPTIONS: TermMentionOption[] = [
  { token: '@term', label: '@term', desc: 'Attach recent terminal session blocks' },
  { token: '@term:last', label: '@term:last', desc: 'Attach last executed terminal block' },
  { token: '@term:failed', label: '@term:failed', desc: 'Attach most recent failed terminal block' },
  { token: '@term:pinned', label: '@term:pinned', desc: 'Attach all pinned terminal blocks' },
];
