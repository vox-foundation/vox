import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface ChatSession {
  session_id: string;
  title: string;
  updated_at: string;
  message_count: number;
  conversation_id: number;
  repository_id: string | null;
}

interface ArchiveOptions {
  wasActive?: boolean;
  onReassign?: (nextActiveSessionId: string) => void;
}

export function useChatSessions(onLoadError?: (err: unknown) => void) {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [includeArchived, setIncludeArchived] = useState(false);

  const load = useCallback(async (opts?: { includeArchived?: boolean }) => {
    const list = await invoke<ChatSession[]>('chat_list_sessions', {
      limit: 200,
      includeArchived: opts?.includeArchived ?? includeArchived,
    });
    const valid = Array.isArray(list) ? list.filter((s): s is ChatSession => Boolean(s?.session_id)) : [];
    setSessions(valid);
    return valid;
  }, [includeArchived]);

  useEffect(() => {
    load().catch(err => onLoadError?.(err));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const upsertSession = useCallback((s: ChatSession) => {
    if (!s?.session_id) return;
    setSessions(prev => [s, ...prev.filter(item => item.session_id !== s.session_id)]);
  }, []);

  // No optimistic update: on failure, invoke() rejects and this function rethrows without
  // touching `sessions` state, matching ChatSurface.tsx's existing await-then-update-on-success
  // pattern (there is no rollback anywhere in the current codebase to replicate).
  const createSession = useCallback(async (title?: string) => {
    const created = await invoke<ChatSession>('chat_create_session', { title });
    if (created?.session_id) {
      setSessions(prev => [created, ...prev.filter(s => s.session_id !== created.session_id)]);
    }
    return created;
  }, []);

  const renameSession = useCallback(async (sessionId: string, title: string) => {
    await invoke('chat_rename_session', { sessionId, title });
    setSessions(prev => prev.map(s => (s.session_id === sessionId ? { ...s, title } : s)));
  }, []);

  const archiveSession = useCallback(async (sessionId: string, opts?: ArchiveOptions) => {
    await invoke('chat_archive_session', { sessionId });
    setSessions(prev => {
      const remaining = prev.filter(s => s.session_id !== sessionId);
      if (opts?.wasActive && remaining.length > 0) {
        opts.onReassign?.(remaining[0].session_id);
      }
      return remaining;
    });
  }, []);

  const unarchiveSession = useCallback(async (sessionId: string) => {
    await invoke('chat_unarchive_session', { sessionId });
    await load();
  }, [load]);

  const toggleArchivedView = useCallback(async () => {
    const next = !includeArchived;
    setIncludeArchived(next);
    await load({ includeArchived: next });
  }, [includeArchived, load]);

  return {
    sessions,
    includeArchived,
    createSession,
    upsertSession,
    renameSession,
    archiveSession,
    unarchiveSession,
    toggleArchivedView,
    reload: load,
  };
}
