import { useEffect, useState } from 'react';
import { filterBySession, type TaskRow } from '../components/surfaces/Tasks/tasksHelpers';
import type { ChatExecutionTask } from '../components/surfaces/Chat/ChatExecutionRail';
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
    if (
      row.kind !== 'LockAcquired' &&
      row.kind !== 'LockReleased' &&
      row.kind !== 'LockWaiting'
    ) {
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

    const resourcePath =
      typeof detail.resource_id === 'string'
        ? detail.resource_id
        : typeof detail.path === 'string'
          ? detail.path
          : null;

    if (typeof detail.task_id !== 'number' || !resourcePath) {
      continue;
    }

    const taskIdKey = String(detail.task_id);
    if (seen.has(taskIdKey)) {
      continue;
    }
    seen.add(taskIdKey);

    if (row.kind === 'LockAcquired') {
      result.set(taskIdKey, {
        resourceId: boundResourceId(resourcePath),
        state: 'holding',
      });
    } else if (row.kind === 'LockWaiting') {
      result.set(taskIdKey, {
        resourceId: boundResourceId(resourcePath),
        state: 'waiting',
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

export interface ChatExecutionData {
  tasks: ChatExecutionTask[];
}

export function useChatExecutionData(sessionId: string | undefined): ChatExecutionData {
  const [tasks, setTasks] = useState<ChatExecutionTask[]>([]);

  useEffect(() => {
    if (!sessionId) {
      setTasks([]);
      return;
    }

    let cancelled = false;

    const refresh = async () => {
      try {
        // ponytail: client-side kind filter over the newest 200 session rows; add a multi-kind server filter if a busy session pushes an old still-held lock out of that window.
        const [rows, activityRows] = await Promise.all([
          voxTransport.listOrchestratorTasks(),
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
        const mappedTaskIds = new Set(mappedTasks.map(t => t.id));
        const syntheticTasks: ChatExecutionTask[] = [];
        for (const [taskId, lock] of lockMap.entries()) {
          if (lock.state === 'waiting' && !mappedTaskIds.has(taskId)) {
            syntheticTasks.push({
              id: taskId,
              title: 'Waiting for resource lock',
              status: 'waiting',
              lock,
            });
          }
        }
        setTasks([...tasksWithLocks, ...syntheticTasks]);
      } catch {
        if (!cancelled) {
          setTasks([]);
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

  return { tasks };
}
