import React, { useCallback, useEffect, useState } from 'react';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { VoxSelect } from '../../ui/Select';
import {
  listHarnessIssues,
  recordHarnessIssueDecision,
  scanTrainingCorpus,
  proposeHarnessIssueFix,
  listHarnessFixProposals,
  resolveHarnessFixProposal,
  type HarnessIssueRow,
  type HarnessFixProposalRow,
} from './harnessIssuesApi';

/** Review queue for harness issue discovery (Phase 1). */
export function HarnessIssuesPanel({ pushToast }: SurfaceDecoratorProps) {
  const [statusFilter, setStatusFilter] = useState<'pending' | 'confirmed' | 'dismissed'>('pending');
  const [sourceFilter, setSourceFilter] = useState<'all' | 'chat_session' | 'corpus_scan'>('all');
  const [issues, setIssues] = useState<HarnessIssueRow[]>([]);
  const [proposals, setProposals] = useState<HarnessFixProposalRow[]>([]);
  // Every proposal ever made per issue (any status), not just pending_approval
  // — used only to decide retry-eligibility below, so a rejected proposal
  // doesn't get mistaken for "never proposed" and re-offer "Retry propose fix".
  const [proposedIssueIds, setProposedIssueIds] = useState<Set<number>>(new Set());
  const [scanning, setScanning] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [pendingIssues, pendingProposals, allProposals] = await Promise.all([
        listHarnessIssues(statusFilter, sourceFilter === 'all' ? undefined : sourceFilter),
        listHarnessFixProposals('pending_approval'),
        listHarnessFixProposals(),
      ]);
      setIssues(pendingIssues);
      setProposals(pendingProposals);
      setProposedIssueIds(new Set(allProposals.map((p) => p.issue_id)));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Harness issues', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast, statusFilter, sourceFilter]);

  useEffect(() => {
    refresh();
    const id = window.setInterval(refresh, 10_000);
    return () => window.clearInterval(id);
  }, [refresh]);

  const decide = useCallback(
    async (issue: HarnessIssueRow, decision: 'confirmed' | 'dismissed') => {
      try {
        await recordHarnessIssueDecision(issue.id, decision);
        // Dispatch-to-fix is only reachable for issues with a target_path
        // (v1: corpus_scan staleness findings — chat_session issues never
        // set one, since reliably identifying which golden-corpus file a
        // chat error relates to is out of scope for this phase).
        if (decision === 'confirmed' && issue.target_path) {
          await proposeHarnessIssueFix(issue.id, issue.target_path);
        }
        await refresh();
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Harness issues', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
    },
    [pushToast, refresh],
  );

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      const found = await scanTrainingCorpus();
      pushToast({ tone: 'info', title: 'Training corpus scan', body: `${found} new issue(s) found`, cause: 'backend-ok' });
      await refresh();
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Training corpus scan', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setScanning(false);
    }
  }, [pushToast, refresh]);

  const resolveProposal = useCallback(
    async (proposalId: number, approve: boolean) => {
      try {
        await resolveHarnessFixProposal(proposalId, approve);
        await refresh();
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Fix proposal', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
    },
    [pushToast, refresh],
  );

  const retryProposeFix = useCallback(
    async (issue: HarnessIssueRow) => {
      if (!issue.target_path) return;
      try {
        await proposeHarnessIssueFix(issue.id, issue.target_path);
        await refresh();
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Harness issues', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
    },
    [pushToast, refresh],
  );

  return (
    <div className="flex min-h-0 flex-col gap-4 p-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="font-display text-sm uppercase tracking-wide text-text-secondary">Harness Issues</h2>
        <div className="flex items-center gap-2">
          <VoxSelect
            size="xs"
            aria-label="Filter by status"
            value={statusFilter}
            onValueChange={(v) => setStatusFilter(v as typeof statusFilter)}
            options={[
              { value: 'pending', label: 'Pending' },
              { value: 'confirmed', label: 'Confirmed' },
              { value: 'dismissed', label: 'Dismissed' },
            ]}
            className="w-32 border-border-subtle bg-transparent text-xs text-text-secondary"
          />
          <VoxSelect
            size="xs"
            aria-label="Filter by source"
            value={sourceFilter}
            onValueChange={(v) => setSourceFilter(v as typeof sourceFilter)}
            options={[
              { value: 'all', label: 'All sources' },
              { value: 'chat_session', label: 'Chat sessions' },
              { value: 'corpus_scan', label: 'Corpus scan' },
            ]}
            className="w-36 border-border-subtle bg-transparent text-xs text-text-secondary"
          />
          <button
            type="button"
            onClick={scan}
            disabled={scanning}
            className="rounded-md border border-border-subtle px-3 py-1.5 text-xs text-text-secondary hover:bg-overlay-hover disabled:opacity-50"
          >
            {scanning ? 'Scanning…' : 'Scan training corpus'}
          </button>
        </div>
      </div>

      <div role="list" aria-label="Harness issues" className="flex flex-col gap-2">
        {issues.length === 0 ? (
          <div className="text-xs text-text-muted">No issues match this filter.</div>
        ) : (
          issues.map((issue) => (
            <div key={issue.id} role="listitem" className="rounded-md border border-border-subtle p-3">
              <div className="flex items-center justify-between text-xs">
                <span className="font-mono uppercase text-text-muted">{issue.source} · {issue.severity}</span>
              </div>
              <div className="mt-1 text-sm text-text-primary">{issue.summary}</div>
              {issue.status === 'pending' && (
                <div className="mt-2 flex gap-2">
                  <button
                    type="button"
                    onClick={() => decide(issue, 'confirmed')}
                    className="rounded-sm border border-brass/30 bg-brass/10 px-2 py-1 text-xs text-brass"
                  >
                    {issue.target_path ? 'Confirm & propose fix' : 'Confirm'}
                  </button>
                  <button
                    type="button"
                    onClick={() => decide(issue, 'dismissed')}
                    className="rounded-sm border border-border-subtle px-2 py-1 text-xs text-text-muted"
                  >
                    Dismiss
                  </button>
                </div>
              )}
              {issue.status === 'confirmed' &&
                issue.target_path &&
                !proposedIssueIds.has(issue.id) && (
                  <div className="mt-2 flex gap-2">
                    <button
                      type="button"
                      onClick={() => retryProposeFix(issue)}
                      className="rounded-sm border border-brass/30 bg-brass/10 px-2 py-1 text-xs text-brass"
                    >
                      Retry propose fix
                    </button>
                  </div>
                )}
            </div>
          ))
        )}
      </div>

      {proposals.length > 0 && (
        <div role="list" aria-label="Pending fix proposals" className="flex flex-col gap-2">
          {proposals.map((p) => (
            <div key={p.id} role="listitem" className="rounded-md border border-border-subtle p-3">
              <div className="text-xs font-mono text-text-muted">{p.target_path}</div>
              <pre className="mt-1 max-h-40 overflow-y-auto whitespace-pre-wrap text-[10px] text-text-secondary">
                {p.proposed_diff}
              </pre>
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={() => resolveProposal(p.id, true)}
                  className="rounded-sm border border-brass/30 bg-brass/10 px-2 py-1 text-xs text-brass"
                >
                  Approve & apply
                </button>
                <button
                  type="button"
                  onClick={() => resolveProposal(p.id, false)}
                  className="rounded-sm border border-border-subtle px-2 py-1 text-xs text-text-muted"
                >
                  Reject
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
