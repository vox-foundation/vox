import type { NoticeInput, NoticeSeverity } from './notices';

export const MAX_NOTICES = 200;
/** A repeat of the same notice within this window merges into it (×N). */
export const COALESCE_WINDOW_MS = 60_000;
/** Longest body kept; a task-failed error can be a whole stack trace. */
export const MAX_BODY_CHARS = 1024;

const RANK: Record<NoticeSeverity, number> = { success: 0, info: 0, warning: 1, error: 2 };

export interface Notice {
  id: string;
  groupKey: string;
  severity: NoticeSeverity;
  scope: NoticeInput['scope'];
  source: string;
  title: string;
  body?: string;
  cmd?: string;
  count: number;
  lastAtMs: number;
  read: boolean;
}

export interface NoticeState {
  notices: Notice[];
  seq: number;
}

export const EMPTY_NOTICES: NoticeState = { notices: [], seq: 0 };

export type NoticeAction = { type: 'record'; input: NoticeInput } | { type: 'markAllRead' };

export function noticeReducer(state: NoticeState, action: NoticeAction): NoticeState {
  switch (action.type) {
    case 'markAllRead':
      return { ...state, notices: state.notices.map(n => (n.read ? n : { ...n, read: true })) };
    case 'record': {
      const input = action.input;
      const atMs = input.atMs ?? Date.now();
      const groupKey = input.groupKey ?? `${input.source}:${input.title}`;
      const body = input.body?.slice(0, MAX_BODY_CHARS);
      const hit = state.notices.findIndex(n => n.groupKey === groupKey && atMs - n.lastAtMs < COALESCE_WINDOW_MS);
      if (hit !== -1) {
        const prev = state.notices[hit];
        const merged: Notice = {
          ...prev,
          // Title and body describe the latest occurrence; the severity stays at the worst one seen.
          title: input.title,
          severity: RANK[input.severity] > RANK[prev.severity] ? input.severity : prev.severity,
          body,
          cmd: input.cmd,
          count: prev.count + 1,
          lastAtMs: atMs,
          read: false,
        };
        return { ...state, notices: [merged, ...state.notices.filter((_, i) => i !== hit)] };
      }
      const notice: Notice = {
        id: `notice-${state.seq + 1}`,
        groupKey,
        severity: input.severity,
        scope: input.scope,
        source: input.source,
        title: input.title,
        body,
        cmd: input.cmd,
        count: 1,
        lastAtMs: atMs,
        read: false,
      };
      return { seq: state.seq + 1, notices: [notice, ...state.notices].slice(0, MAX_NOTICES) };
    }
  }
}

/** Unread warnings and errors: the number the bell shows. */
export function unreadProblems(notices: Notice[]): number {
  return notices.filter(n => !n.read && (n.severity === 'warning' || n.severity === 'error')).length;
}
