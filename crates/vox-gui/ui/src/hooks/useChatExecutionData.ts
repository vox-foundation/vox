import { useEffect, useState } from 'react';
import { decode } from '@msgpack/msgpack';
import { filterBySession, type TaskRow } from '../components/surfaces/Tasks/tasksHelpers';
import type { ChatExecutionTask } from '../components/surfaces/Chat/ChatExecutionRail';
import type { OrchestratorStatus, RoutingSummary } from '../types/tauri';
import { voxTransport, activityQuery, type ActivityRowDto } from '../transport';

export const CHAT_EXECUTION_POLL_MS = 5_000;
export const CHAT_LOCK_ACTIVITY_LIMIT = 200;

function boundResourceId(raw: string, maxLen = 64): string {
  if (raw.length <= maxLen) return raw;
  return raw.slice(0, maxLen - 3) + '...';
}

export function lockStatesFromActivity(
  rows: ActivityRowDto[],
): Map<string, NonNullable<ChatExecutionTask['lock']>> {
  const result = new Map<string, NonNullable<ChatExecutionTask['lock']>>();
  if (!Array.isArray(rows)) return result;

  const seen = new Set<string>();

  for (const row of rows) {
    if (row.kind !== 'LockAcquired' && row.kind !== 'LockReleased') {
      continue;
    }
    if (!row.detail_json) continue;
    let detail: Record<string, unknown>;
    try {
      const parsed = JSON.parse(row.detail_json);
      if (!parsed || typeof parsed !== 'object') continue;
      detail = parsed as Record<string, unknown>;
    } catch {
      continue;
    }

    if (typeof detail.task_id !== 'number' || typeof detail.path !== 'string') {
      continue;
    }

    const taskIdKey = String(detail.task_id);
    if (seen.has(taskIdKey)) {
      continue;
    }
    seen.add(taskIdKey);

    if (row.kind === 'LockAcquired') {
      result.set(taskIdKey, {
        resourceId: boundResourceId(detail.path),
        state: 'holding',
      });
    }
  }

  return result;
}

export function mapOrchestratorTasksForSession(
  rows: TaskRow[],
  sessionId: string | undefined,
): ChatExecutionTask[] {
  if (!sessionId) return [];
  return filterBySession(rows, sessionId)
    .filter(t => t.lifecycle !== 'completed')
    .map(t => ({
      id: String(t.id),
      title: t.description,
      status: t.lifecycle,
    }));
}

export function intentsFromRoutingSummary(summary: RoutingSummary | null): string[] {
  const preview = summary?.decision_preview;
  if (!preview) return [];

  const intents: string[] = [];
  if (preview.selected_model) {
    const state = preview.discovery_state ? ` · ${preview.discovery_state}` : '';
    intents.push(`${preview.selected_model}${state}`);
  }

  for (const alt of preview.alternatives ?? []) {
    if (intents.length >= 3) break;
    intents.push(`Alt: ${alt}`);
  }

  return intents.slice(0, 3);
}

function meshPeersFromStatusBin(statusBin: Uint8Array | null): number {
  if (!statusBin) return 0;
  try {
    const status = decode(statusBin) as OrchestratorStatus;
    return (status.peers ?? []).length;
  } catch {
    return 0;
  }
}

export interface ChatExecutionData {
  tasks: ChatExecutionTask[];
  intents: string[];
  meshPeers: number;
}

export function useChatExecutionData(sessionId: string | undefined): ChatExecutionData {
  const [tasks, setTasks] = useState<ChatExecutionTask[]>([]);
  const [intents, setIntents] = useState<string[]>([]);
  const [meshPeers, setMeshPeers] = useState(0);

  useEffect(() => {
    if (!sessionId) {
      setTasks([]);
      setIntents([]);
      setMeshPeers(0);
      return;
    }

    let cancelled = false;

    const refresh = async () => {
      try {
        // ponytail: client-side kind filter over the newest 200 session rows; add a multi-kind server filter if a busy session pushes an old still-held lock out of that window.
        const [rows, summary, statusBin, activityRows] = await Promise.all([
          voxTransport.listOrchestratorTasks(),
          voxTransport.getRoutingSummaryLive(),
          voxTransport.getOrchestratorStatusBin().catch(() => null),
          activityQuery({
            agent_id: null,
            kind: null,
            session_id: sessionId,
            limit: CHAT_LOCK_ACTIVITY_LIMIT,
            before_id: null,
          }).catch(() => []),
        ]);
        if (cancelled) return;
        const mappedTasks = mapOrchestratorTasksForSession(rows, sessionId);
        const lockMap = lockStatesFromActivity(activityRows);
        const tasksWithLocks = mappedTasks.map(t => {
          const lock = lockMap.get(t.id);
          return lock ? { ...t, lock } : t;
        });
        setTasks(tasksWithLocks);
        setIntents(intentsFromRoutingSummary(summary));
        setMeshPeers(meshPeersFromStatusBin(statusBin));
      } catch {
        if (!cancelled) {
          setTasks([]);
          setIntents([]);
          setMeshPeers(0);
        }
      }
    };

    refresh();
    const id = window.setInterval(refresh, CHAT_EXECUTION_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [sessionId]);

  return { tasks, intents, meshPeers };
}
