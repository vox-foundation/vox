import { useCallback, useReducer } from 'react';
import { EMPTY_NOTICES, noticeReducer, type Notice } from '../lib/noticeStore';
import type { NoticeInput } from '../lib/notices';

export interface NoticeCenter {
  notices: Notice[];
  record(input: NoticeInput): void;
  markAllRead(): void;
}

/** The app's notice store. `record` is stable, so listeners can capture it once. */
export function useNoticeCenter(): NoticeCenter {
  const [state, dispatch] = useReducer(noticeReducer, EMPTY_NOTICES);
  const record = useCallback((input: NoticeInput) => dispatch({ type: 'record', input }), []);
  const markAllRead = useCallback(() => dispatch({ type: 'markAllRead' }), []);
  return { notices: state.notices, record, markAllRead };
}
