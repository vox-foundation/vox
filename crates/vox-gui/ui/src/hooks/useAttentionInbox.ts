import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { voxTransport, feedbackList, feedbackResolve, listenFeedbackChanged, hopperList, type FeedbackRow, type HopperTaskDto } from '../transport';
import { parsePendingApprovals, unwrapMcpEnvelope, type McpInvokeResult, type PendingApprovalRow } from '../lib/mcpToolResult';
import { ATTENTION_POLL_MS } from '../config/constants';

export interface AttentionInbox {
  approvals: PendingApprovalRow[];
  needsYou: FeedbackRow[];
  withheld: FeedbackRow[];
  blockedTasksCount: number;
  /** Raw hopper task rows, for consumers (e.g. TasksView) that need the full
   *  per-task list rather than just the derived blocked count. */
  hopperTasks: HopperTaskDto[];
  /** Sources whose last fetch failed (`approvals`, `feedback`, `tasks`); their lists are empty because they are unknown, not because they are clear. */
  degraded: string[];
  /** Items awaiting a human decision: pending approvals + needs-you feedback. */
  totalCount: number;
  refresh(): Promise<void>;
  resolveApproval(approvalId: string, outcome: 'approved' | 'rejected'): Promise<void>;
  resolveFeedback(feedbackId: string, action: Record<string, unknown>): Promise<void>;
}

export function useAttentionInbox(): AttentionInbox {
  const [approvals, setApprovals] = useState<PendingApprovalRow[]>([]);
  const [needsYou, setNeedsYou] = useState<FeedbackRow[]>([]);
  const [withheld, setWithheld] = useState<FeedbackRow[]>([]);
  const [blockedTasksCount, setBlockedTasksCount] = useState(0);
  const [hopperTasks, setHopperTasks] = useState<HopperTaskDto[]>([]);
  const [degraded, setDegraded] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    const emptyFeedback = { needsYou: [] as FeedbackRow[], withheld: [] as FeedbackRow[] };
    const [approvalRes, feedback, tasks] = await Promise.allSettled([
      Promise.resolve(voxTransport.invokeMcpTool('vox_pending_approvals', {})),
      Promise.resolve(feedbackList()),
      Promise.resolve(hopperList()),
    ]);
    // An MCP error reply resolves instead of rejecting; it is a failed source all the same.
    const approvalReply = approvalRes.status === 'fulfilled' && !approvalRes.value?.is_error ? approvalRes.value : undefined;
    setDegraded([
      approvalRes.status === 'rejected' || approvalRes.value?.is_error ? 'approvals' : null,
      feedback.status === 'rejected' ? 'feedback' : null,
      tasks.status === 'rejected' ? 'tasks' : null,
    ].filter((s): s is string => s !== null));
    const safeFeedback = (feedback.status === 'fulfilled' && feedback.value) || emptyFeedback;
    const safeTasks = (tasks.status === 'fulfilled' && tasks.value) || [];
    setApprovals(approvalReply ? parsePendingApprovals(approvalReply as McpInvokeResult) : []);
    setNeedsYou(safeFeedback.needsYou ?? []);
    setWithheld(safeFeedback.withheld ?? []);
    const gates = new Set<number>((safeFeedback.needsYou ?? []).flatMap((f) => f.gates ?? []));
    setBlockedTasksCount(safeTasks.filter((t) => gates.has(t.task_id)).length);
    setHopperTasks(safeTasks);
  }, []);

  useEffect(() => {
    refresh();
    let unFeedback: (() => void) | null = null;
    let unTasks: (() => void) | null = null;
    Promise.resolve(listenFeedbackChanged(() => { refresh(); })).then((u) => { unFeedback = u ?? null; }).catch(() => {});
    Promise.resolve(listen<void>('vox://tasks-changed', () => { refresh(); })).then((u) => { unTasks = u ?? null; }).catch(() => {});
    const id = setInterval(refresh, ATTENTION_POLL_MS);
    return () => { unFeedback?.(); unTasks?.(); clearInterval(id); };
  }, [refresh]);

  const resolveApproval = useCallback(async (approvalId: string, outcome: 'approved' | 'rejected') => {
    const res = await voxTransport.invokeMcpTool('vox_resolve_approval', { approval_id: approvalId, outcome });
    const data = res ? (unwrapMcpEnvelope(res.result) as { resolved?: boolean } | null) : null;
    if (!res || res.is_error || data?.resolved === false) {
      throw new Error(`resolve failed for ${approvalId}`);
    }
    setApprovals((prev) => prev.filter((a) => a.approval_id !== approvalId));
    await refresh();
  }, [refresh]);

  const resolveFeedback = useCallback(async (feedbackId: string, action: Record<string, unknown>) => {
    await feedbackResolve(feedbackId, action);
    await refresh();
  }, [refresh]);

  return { approvals, needsYou, withheld, blockedTasksCount, hopperTasks, totalCount: approvals.length + needsYou.length, refresh, resolveApproval, resolveFeedback, degraded };
}
