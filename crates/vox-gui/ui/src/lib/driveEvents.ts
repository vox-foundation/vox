export const DRIVE_EVENTS_CAP = 500;
export const DRIVE_EVENT_TEXT_CAP = 4096;
export const DRIVE_EVENT_RAW_CAP = 8192;

export interface DriveTurnEvent {
  seq: number;
  ts_ms: number;
  kind: string;
  turn_id: string | null;
  text?: string;
  raw?: string;
}

export interface DriveEventState {
  events: DriveTurnEvent[];
  events_dropped: number;
  last_turn_id: string | null;
  next_seq: number;
}

export interface DriveEventInput {
  turn_id?: string | null;
  kind: string;
  text?: string;
  raw?: unknown;
}

const BEARER_PATTERN = /Bearer\s+[^\s,;"']+/gi;

function redact(value: string): string {
  return value.replace(BEARER_PATTERN, '[redacted]');
}

function truncate(value: string, cap: number): string {
  return value.length > cap ? value.slice(0, cap) : value;
}

function sanitizeText(text: string | undefined): string | undefined {
  return text === undefined ? undefined : truncate(redact(text), DRIVE_EVENT_TEXT_CAP);
}

function sanitizeRaw(raw: unknown): string | undefined {
  if (raw === undefined) return undefined;
  let serialized: string;
  try {
    serialized = JSON.stringify(raw);
  } catch {
    serialized = JSON.stringify({ error: 'unserializable_raw' });
  }
  return truncate(redact(serialized), DRIVE_EVENT_RAW_CAP - 8);
}

export function appendDriveEvent(state: DriveEventState, input: DriveEventInput): DriveEventState {
  const event: DriveTurnEvent = {
    seq: state.next_seq,
    ts_ms: Date.now(),
    kind: input.kind,
    turn_id: input.turn_id ?? null,
  };
  const text = sanitizeText(input.text);
  if (text !== undefined) event.text = text;
  const raw = sanitizeRaw(input.raw);
  if (raw !== undefined) event.raw = raw;

  const events = [...state.events, event];
  const dropped = Math.max(0, events.length - DRIVE_EVENTS_CAP);
  return {
    events: dropped > 0 ? events.slice(dropped) : events,
    events_dropped: state.events_dropped + dropped,
    last_turn_id: input.turn_id ?? state.last_turn_id,
    next_seq: state.next_seq + 1,
  };
}

export function clearDriveEvents(state: DriveEventState): DriveEventState {
  return {
    events: [],
    events_dropped: 0,
    last_turn_id: null,
    next_seq: state.next_seq,
  };
}

export function recordAgentFrame(
  state: DriveEventState,
  frame: unknown,
  turnId: string,
): DriveEventState {
  const kind = frame as { kind?: { type?: unknown; text?: unknown } };
  const type = typeof kind.kind?.type === 'string' ? kind.kind.type : 'agent_event';
  const text = typeof kind.kind?.text === 'string' ? kind.kind.text : undefined;
  return appendDriveEvent(state, { turn_id: turnId, kind: type, text, raw: frame });
}
