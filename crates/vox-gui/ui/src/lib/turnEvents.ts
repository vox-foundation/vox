import type { TurnEventDto } from '../types/dashboard';

/**
 * Turn-event kinds Rust puts in a chat reply's `events` (`contracts/gui/turn-event-kinds.v1.json`;
 * producers in `crates/vox-orchestrator-mcp/src/chat_tools/chat/{agent_loop,turn_events}.rs`).
 */
export const TURN_EVENT_KINDS: readonly string[] = [
  'skill_activated',
  'delegation_spawned',
  'research_milestone',
  'tool_receipt',
  'receipt_claims',
  'routing_decision',
];

const isText = (v: unknown): v is string => typeof v === 'string' && v.length > 0;
const isCount = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const isFlag = (v: unknown): v is boolean => typeof v === 'boolean';
const optionalText = (v: unknown) => v === undefined || isText(v);

const RESOLVED_FROM = new Set(['catalog', 'bootstrap', 'local']);

/** Required-field checks, one per kind above (a Map, so `constructor` is not a kind). */
const VALIDATORS = new Map<string, (e: TurnEventDto) => boolean>([
  ['skill_activated', (e) => isText(e.skill_id)],
  ['delegation_spawned', (e) => isText(e.tool) && isCount(e.agent_id)],
  [
    'research_milestone',
    (e) =>
      isText(e.tool) &&
      isCount(e.waves_executed) &&
      isCount(e.claims_verified) &&
      isCount(e.contradictions_resolved),
  ],
  [
    'tool_receipt',
    (e) => isText(e.tool) && isText(e.receipt_id) && isFlag(e.fulfilled) && isFlag(e.verified),
  ],
  ['receipt_claims', (e) => isCount(e.valid) && isCount(e.fabricated) && isCount(e.unverified)],
  [
    'routing_decision',
    (e) =>
      isText(e.family) &&
      isText(e.resolved_id) &&
      typeof e.resolved_from === 'string' &&
      RESOLVED_FROM.has(e.resolved_from) &&
      isText(e.reason) &&
      optionalText(e.mode) &&
      optionalText(e.objective),
  ],
]);

/** True for a known kind whose required fields are present and well-typed. */
export function isKnownTurnEvent(e: TurnEventDto | null | undefined): e is TurnEventDto {
  if (!e || typeof e.kind !== 'string') return false;
  const check = VALIDATORS.get(e.kind);
  return check !== undefined && check(e);
}

/**
 * Canonical mode names keyed by wire value (`ClutchId`). The one owner of these labels: the
 * composer, rail and status bar import them from here.
 */
export const MODE_NAMES = Object.freeze({
  free: 'Free',
  efficiency: 'Efficient',
  balanced: 'Balanced',
  genius: 'Genius',
} as const);

/** Canonical mode name for a wire value (`efficiency` → `Efficient`); unknown values pass through. */
export function modeLabel(wire: string): string {
  return Object.prototype.hasOwnProperty.call(MODE_NAMES, wire)
    ? MODE_NAMES[wire as keyof typeof MODE_NAMES]
    : wire;
}

/**
 * What a routing decision may show as the model (the one owner of this rule): the catalog id only
 * when it came from a live source, the local id marked local, the family marked offline for the
 * bootstrap fallback. Never a "(latest)" claim.
 */
export function routingModelLabel(e: TurnEventDto): string {
  if (e.resolved_from === 'catalog') return String(e.resolved_id);
  if (e.resolved_from === 'local') return `${String(e.resolved_id)} (local)`;
  return `${String(e.family)} (offline)`;
}
