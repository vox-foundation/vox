import type { Toast } from '../types/tauri';
import type { AgentEventFrame } from './chatCorrelation';
import { mapAgentEvent } from './mapAgentEvent';
import { toastGroupKey } from './toastQueue';

/** docs/src/architecture/gui-observability-ssot-2026.md — every message the GUI shows is a notice. */
export type NoticeSeverity = 'success' | 'info' | 'warning' | 'error';
/** Scopes this store holds. `turn` and `session` notices live in the chat trace and rail, never here. */
export type NoticeScope = 'action' | 'engine' | 'app';

export interface NoticeInput {
  severity: NoticeSeverity;
  scope: NoticeScope;
  /** Producer id: a toast's `cause`, or `engine`. */
  source: string;
  title: string;
  body?: string;
  cmd?: string;
  /** Coalescing identity; defaults to `source` + `title`. */
  groupKey?: string;
  atMs?: number;
}

/** An agent-event frame as the GUI bridge forwards it, with the engine's severity. */
export type SeverityFrame = AgentEventFrame & { severity?: string };

const TONE_SEVERITY: Record<Toast['tone'], NoticeSeverity> = {
  ok: 'success',
  info: 'info',
  warn: 'warning',
  error: 'error',
};

/** Event fields that carry a human-readable reason, in order of preference. */
const DETAIL_FIELDS = ['error', 'detail', 'reason', 'tool_key'] as const;

export function noticeFromToast(t: Toast, atMs: number = Date.now()): NoticeInput {
  return {
    severity: TONE_SEVERITY[t.tone],
    scope: 'action',
    source: t.cause,
    title: t.title,
    body: t.body,
    cmd: t.cmd,
    // Same identity the toast stack coalesces by.
    groupKey: toastGroupKey(t),
    atMs,
  };
}

/** Engine warnings and errors only; everything else stays in the trace and the activity log. */
export function noticeFromAgentEvent(frame: SeverityFrame): NoticeInput | null {
  if (frame.severity !== 'warning' && frame.severity !== 'error') return null;
  const item = mapAgentEvent(frame);
  const kind = frame.kind;
  const detail = DETAIL_FIELDS.map(f => kind[f]).find((v): v is string => typeof v === 'string' && v !== '');
  // One notice per event type *and* subject, so two different tasks' failures never merge into one.
  const subject = kind.task_id ?? kind.agent_id ?? kind.workflow_id;
  return {
    severity: frame.severity,
    scope: 'engine',
    source: 'engine',
    title: item.title,
    body: item.body || detail,
    groupKey: `engine:${kind.type}${subject != null ? `:${String(subject)}` : ''}`,
    atMs: frame.timestamp_ms || Date.now(),
  };
}
