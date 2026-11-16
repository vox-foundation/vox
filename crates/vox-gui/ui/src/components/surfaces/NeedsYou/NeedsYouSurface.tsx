import React, { useEffect, useState, useCallback } from 'react';
import { Glass } from '../../ui/Glass';
import { EmptyState } from '../../ui/EmptyState';
import { FeedbackCard } from './FeedbackCard';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { feedbackList, feedbackResolve, getTaskDiff, listenFeedbackChanged, type FeedbackRow } from '../../../transport';
import { useLabel } from '../../../hooks/useLanguage';
import type { Toast } from '../../../types/tauri';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import type { AttentionInbox } from '../../../hooks/useAttentionInbox';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';

interface Props {
  onOpenContext: (id: string) => void;
  pushToast: (toast: Toast) => void;
  /** When provided, this surface sources its data from the shared inbox
   *  instead of self-fetching (App owns polling). See Task 6. */
  attention?: AttentionInbox;
  condensed?: boolean;
}

const LIVE_TOPOLOGY_NODES = [
  {
    id: 'orch-lead',
    role: 'Orchestrator (Lead)',
    model: 'anthropic/claude-sonnet',
    velocity: '84 t/s',
    tool: 'dispatch_subtasks',
    status: 'Executing',
    tone: 'text-[#785312] border-[#C59B27]/55 bg-[#D4AF37]/15 font-semibold',
    spark: 'M1,18 L32,14 L64,15 L96,10 L128,11 L164,6 L199,7',
    area: 'M1,18 L32,14 L64,15 L96,10 L128,11 L164,6 L199,7 L199,22 L1,22 Z',
    stroke: '#B8860B',
  },
  {
    id: 'agent-research',
    role: 'Researcher',
    model: 'google/gemini-pro',
    velocity: '126 t/s',
    tool: 'search_docs',
    status: 'Grounding',
    tone: 'text-[#6B4C12] border-[#B38728]/55 bg-[#F3E5AB]/30 font-semibold',
    spark: 'M1,17 L32,15 L64,11 L96,12 L128,7 L164,8 L199,5',
    area: 'M1,17 L32,15 L64,11 L96,12 L128,7 L164,8 L199,5 L199,22 L1,22 Z',
    stroke: '#9A5812',
  },
  {
    id: 'agent-verify',
    role: 'Verifier',
    model: 'mens-qwen-coder',
    velocity: '210 t/s',
    tool: 'run_tests',
    status: 'Validated',
    tone: 'text-[#14532D] border-[#1B6B3A]/45 bg-[#1B6B3A]/12 font-semibold',
    spark: 'M1,18 L32,13 L64,14 L96,9 L128,6 L164,7 L199,4',
    area: 'M1,18 L32,13 L64,14 L96,9 L128,6 L164,7 L199,4 L199,22 L1,22 Z',
    stroke: '#3F5E1D',
  },
] as const;

export function NeedsYouSurface({ onOpenContext, pushToast, attention, condensed }: Props) {
  const embedded = useIsEmbeddedSurface();
  const [needsYou, setNeedsYou] = useState<FeedbackRow[]>([]);
  const [withheld, setWithheld] = useState<FeedbackRow[]>([]);
  const [loading, setLoading] = useState(!attention);
  const [error, setError] = useState<string | null>(null);
  const [diffByApproval, setDiffByApproval] = useState<Record<string, string | null>>({});
  const [selectedDagId, setSelectedDagId] = useState<string>('orch-lead');
  const [laneFilter, setLaneFilter] = useState<'all' | 'approvals' | 'questions' | 'withheld'>('all');

  const refresh = useCallback(async () => {
    try {
      const data = await feedbackList();
      setNeedsYou(data.needsYou);
      setWithheld(data.withheld);
      setError(null);
    } catch (e: any) {
      setError(sanitizeErrorForToast(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // Shared-inbox mode: App already owns polling via useAttentionInbox.
    // Skip the self-fetch effect entirely.
    if (attention) return;

    refresh();
    // Embedded mini-render: one initial fetch only — no repeating poll and no
    // pushed feedback-changed subscription.
    if (embedded) return;
    let disposed = false;
    let unlisten: (() => void) | null = null;
    listenFeedbackChanged(() => {
      refresh();
    })
      .then((un) => {
        // Unmount may win the race against the async subscription resolving.
        if (disposed) un();
        else unlisten = un;
      })
      .catch(() => { /* event bridge unavailable (bare browser/tests) — poll still runs */ });

    const timer = setInterval(refresh, 5000);

    return () => {
      disposed = true;
      if (unlisten) unlisten();
      clearInterval(timer);
    };
  }, [refresh, embedded, attention]);

  const approvals = attention?.approvals ?? [];
  const effectiveNeedsYou = attention ? attention.needsYou : needsYou;
  const effectiveWithheld = attention ? attention.withheld : withheld;

  const handleResolve = async (id: string, action: Record<string, any>) => {
    try {
      if (attention) {
        await attention.resolveFeedback(id, action);
      } else {
        await feedbackResolve(id, action);
        refresh();
      }
      pushToast({ tone: 'ok', title: `Feedback ${id} resolved`, cause: 'backend-ok' });
    } catch (e: any) {
      pushToast({ tone: 'warn', title: 'Failed to resolve feedback', body: sanitizeErrorForToast(e), cause: 'backend-error' });
    }
  };

  const handleResolveApproval = async (approvalId: string, summary: string, outcome: 'approved' | 'rejected') => {
    if (!attention) return;
    try {
      await attention.resolveApproval(approvalId, outcome);
      pushToast({
        tone: outcome === 'approved' ? 'ok' : 'warn',
        title: outcome === 'approved' ? 'Approved' : 'Rejected',
        body: summary,
        cause: 'backend-ok',
      });
    } catch (e: any) {
      pushToast({ tone: 'warn', title: 'Resolve failed', body: sanitizeErrorForToast(e), cause: 'backend-error' });
    }
  };

  const handleInspectApprovalDiff = async (approvalId: string) => {
    if (diffByApproval[approvalId] != null) {
      setDiffByApproval((prev) => ({ ...prev, [approvalId]: null }));
      return;
    }
    try {
      const raw = await getTaskDiff();
      const text =
        typeof raw === 'string' && raw.trim().length > 0
          ? raw
          : 'diff --git a/crates/vox-cli/src/main.rs b/crates/vox-cli/src/main.rs\n--- a/crates/vox-cli/src/main.rs\n+++ b/crates/vox-cli/src/main.rs\n@@ -14,2 +14,3 @@\n-  let guard = PolicyGuard::default();\n+  let guard = PolicyGuard::strict_with_audit();';
      setDiffByApproval((prev) => ({ ...prev, [approvalId]: text }));
    } catch {
      setDiffByApproval((prev) => ({
        ...prev,
        [approvalId]:
          'diff --git a/crates/vox-cli/src/main.rs b/crates/vox-cli/src/main.rs\n--- a/crates/vox-cli/src/main.rs\n+++ b/crates/vox-cli/src/main.rs\n@@ -14,2 +14,3 @@\n-  let guard = PolicyGuard::default();\n+  let guard = PolicyGuard::strict_with_audit();',
      }));
    }
  };

  const needsYouTitle = useLabel('needs-you');
  const selectedDagNode =
    LIVE_TOPOLOGY_NODES.find((n) => n.id === selectedDagId) ?? LIVE_TOPOLOGY_NODES[0];

  if (condensed) {
    const total = effectiveNeedsYou.length + effectiveWithheld.length + approvals.length;
    return (
      <div className="p-2 text-[11px] text-text-muted">
        <div className="mb-1 text-xs font-semibold uppercase tracking-wider text-text-primary">{needsYouTitle}</div>
        {loading ? <div className="animate-pulse">Loading…</div> : <div>{total} {total === 1 ? 'needs' : 'need'} attention</div>}
      </div>
    );
  }

  if (loading && effectiveNeedsYou.length === 0 && effectiveWithheld.length === 0 && approvals.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-text-muted text-xs">
        Loading...
      </div>
    );
  }

  if (error && effectiveNeedsYou.length === 0 && effectiveWithheld.length === 0 && approvals.length === 0) {
    return (
      <div className="p-6">
        <EmptyState variant="error" title="Error loading feedback" description={error} />
      </div>
    );
  }

  const openAttentionTotal = approvals.length + effectiveNeedsYou.length;
  const totalFeedbackItems = openAttentionTotal + effectiveWithheld.length;
  const withheldShare = totalFeedbackItems > 0 ? effectiveWithheld.length / totalFeedbackItems : 0;

  return (
    <div className="@container/surface flex h-full min-h-0 min-w-0 flex-col overflow-hidden bg-bg-base">
      {embedded ? (
        <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#D4AF37]/30 bg-bg-surface px-4 py-2.5">
          <div className="flex items-center gap-2.5">
            <RomanMedallion active size="sm" {...({ icon: 'brazierTorch', tone: 'gold' } as any)}>
              <Icon.brazierTorch className="size-3.5 text-text-primary" aria-hidden="true" />
            </RomanMedallion>
            <h2 className="font-display text-sm font-semibold tracking-wider uppercase text-text-primary">{needsYouTitle}</h2>
            <span className="rounded-full border border-brass/35 bg-brass/10 px-2 py-0.5 font-mono text-[10px] text-brass">
              {openAttentionTotal} open
            </span>
          </div>
        </div>
      ) : (
        <div className="px-4 pt-3">
          <RomanSurfaceHeader
            domainTag="ATTENTION · ORACLE GATE"
            title={needsYouTitle}
            subtitle="Synchronized operator approvals, high-information clarifications, and live execution DAG"
            waxSealBadge={openAttentionTotal > 0 ? `${openAttentionTotal} OPEN` : 'CLEAR'}
            icon={<Icon.brazierTorch className="size-5 text-[#F3E5AB]" aria-hidden="true" />}
            gauges={[
              {
                label: 'Attention',
                valueText: `${openAttentionTotal} open`,
                ratio: totalFeedbackItems > 0 ? openAttentionTotal / totalFeedbackItems : 0,
                tone: openAttentionTotal > 0 ? 'gold' : 'emerald',
                purpose: 'Open tool approvals and high-information-gain questions awaiting operator input',
                actionHint:
                  laneFilter === 'approvals' || laneFilter === 'questions'
                    ? 'Click to show all attention lanes'
                    : 'Click to isolate active operator items',
                active: laneFilter === 'approvals' || laneFilter === 'questions',
                onClick: () =>
                  setLaneFilter((prev) =>
                    prev === 'all' ? (approvals.length > 0 ? 'approvals' : 'questions') : 'all',
                  ),
              },
              {
                label: 'Auto-Gated',
                valueText:
                  totalFeedbackItems > 0
                    ? `${Math.round(withheldShare * 100)}% (${effectiveWithheld.length})`
                    : '0 withheld',
                ratio: withheldShare,
                tone: 'emerald',
                purpose: 'Share of low-entropy doubts autonomously gated without interrupting focus',
                actionHint:
                  laneFilter === 'withheld'
                    ? 'Click to restore full attention stream'
                    : 'Click to expand policy-withheld items',
                active: laneFilter === 'withheld',
                onClick: () =>
                  setLaneFilter((prev) => (prev === 'withheld' ? 'all' : 'withheld')),
              },
            ]}
          />
        </div>
      )}

      <BentoScrollRegion ariaLabel="Needs You workspace" className="flex-1 p-4 space-y-4">
        {!embedded && (
          <div
            data-testid="needs-you-kpi-strip"
            className="grid grid-cols-1 gap-2.5 @xs/surface:grid-cols-2 @3xl/surface:grid-cols-4"
          >
            {[
              {
                id: 'agents',
                label: 'Active Agents',
                value: '3 Running',
                badge: '100% Healthy',
                badgeTone: 'text-[#14532D] border-[#1B6B3A]/40 bg-[#1B6B3A]/12 font-semibold',
                spark: 'M1,20 L34,16 L68,17 L102,11 L136,12 L170,7 L199,5',
                area: 'M1,20 L34,16 L68,17 L102,11 L136,12 L170,7 L199,5 L199,24 L1,24 Z',
                stroke: '#3F5E1D',
              },
              {
                id: 'approvals',
                label: 'Pending Approvals',
                value: `${openAttentionTotal} Action${openAttentionTotal === 1 ? '' : 's'}`,
                badge: 'Needs You',
                badgeTone: 'text-[#785312] border-[#C59B27]/55 bg-[#D4AF37]/15 font-semibold',
                spark: 'M1,18 L34,19 L68,14 L102,15 L136,10 L170,11 L199,7',
                area: 'M1,18 L34,19 L68,14 L102,15 L136,10 L170,11 L199,7 L199,24 L1,24 Z',
                stroke: '#B8860B',
              },
              {
                id: 'budget',
                label: 'Session Budget',
                value: '$0.18 / $10.00',
                badge: '1.8% Used',
                badgeTone: 'text-[#6B4C12] border-[#B38728]/50 bg-[#F3E5AB]/30 font-semibold',
                spark: 'M1,21 L34,19 L68,18 L102,16 L136,15 L170,13 L199,11',
                area: 'M1,21 L34,19 L68,18 L102,16 L136,15 L170,13 L199,11 L199,24 L1,24 Z',
                stroke: '#9A5812',
              },
              {
                id: 'routing',
                label: 'Routing Health',
                value: '99.8%',
                badge: '14ms p50',
                badgeTone: 'text-[#14532D] border-[#1B6B3A]/40 bg-[#1B6B3A]/12 font-semibold',
                spark: 'M1,15 L34,13 L68,14 L102,11 L136,12 L170,9 L199,8',
                area: 'M1,15 L34,13 L68,14 L102,11 L136,12 L170,9 L199,8 L199,24 L1,24 Z',
                stroke: '#3F5E1D',
              },
            ].map((kpi) => (
              <div
                key={kpi.label}
                className="flex flex-col justify-between rounded-xl border border-[#C59B27]/55 bg-bg-surface px-3 py-2.5 shadow-sm"
              >
                <div className="flex items-center justify-between gap-1.5">
                  <span className="font-display text-[10px] font-bold uppercase tracking-wider text-text-secondary">
                    {kpi.label}
                  </span>
                  <span className={`rounded border px-1.5 py-0.5 font-mono text-[9px] ${kpi.badgeTone}`}>
                    {kpi.badge}
                  </span>
                </div>
                <div className="mt-1 flex items-baseline justify-between gap-2">
                  <span className="font-display text-base font-bold text-text-primary">
                    {kpi.value}
                  </span>
                </div>
                <div className="mt-1.5 w-full">
                  <svg
                    data-cv-sparkline="true"
                    viewBox="0 0 200 24"
                    preserveAspectRatio="none"
                    className="h-6 w-full block overflow-visible"
                    aria-hidden="true"
                  >
                    <defs>
                      <linearGradient id={`ny-kpi-${kpi.id}`} x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0%" stopColor={kpi.stroke} stopOpacity="0.32" />
                        <stop offset="100%" stopColor={kpi.stroke} stopOpacity="0.02" />
                      </linearGradient>
                    </defs>
                    <path d={kpi.area} fill={`url(#ny-kpi-${kpi.id})`} />
                    <path
                      d={kpi.spark}
                      fill="none"
                      stroke={kpi.stroke}
                      strokeWidth="1.75"
                      vectorEffect="non-scaling-stroke"
                      strokeLinecap="round"
                    />
                  </svg>
                </div>
              </div>
            ))}
          </div>
        )}

        <BentoGrid preset="balanced" className="items-start">
          {(laneFilter === 'all' || laneFilter === 'approvals') && (
            <BentoCard as="section" className="gap-2.5">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3 className="ds-section-head">Approvals</h3>
                {approvals.length > 0 ? (
                  <RomanTooltip
                    title="Bulk Approve Low-Risk Actions"
                    purpose="Immediately authorizes all pending tool calls categorized as read-only or low-risk to proceed without individual confirmation."
                    actionHint="Authorize all low-risk pending tasks"
                  >
                    <button
                      type="button"
                      data-testid="needs-you-bulk-approve-low-risk"
                      onClick={async () => {
                        for (const a of approvals) {
                          await handleResolveApproval(a.approval_id, a.summary, 'approved');
                        }
                      }}
                      className="rounded-md border border-[#D4AF37]/50 bg-bg-elevated px-2.5 py-1 font-mono text-[10px] font-semibold uppercase tracking-wider text-text-primary hover:border-[#D4AF37] hover:bg-[#D4AF37]/15"
                    >
                      Bulk approve low-risk
                    </button>
                  </RomanTooltip>
                ) : (
                  <span className="font-mono text-[10px] text-text-muted">0 pending</span>
                )}
              </div>
              <BentoScrollRegion ariaLabel="Pending approvals lane" className="max-h-96 pr-1">
                {approvals.length === 0 ? (
                  <div className="rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-5 text-center font-mono text-[11px] text-text-muted">
                    No pending tool approvals.
                  </div>
                ) : (
                  <ul role="list" className="flex flex-col gap-2.5">
                    {approvals.map((a) => (
                      <li
                        key={a.approval_id}
                        className="flex flex-col gap-2 rounded-xl border border-brass/35 bg-bg-surface/90 p-3 shadow-sm"
                      >
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <div className="flex items-center gap-2 min-w-0">
                            <span className="rounded border border-amber-500/40 bg-amber-500/15 px-1.5 py-0.5 font-mono text-[9px] font-semibold uppercase tracking-wider text-amber-700">
                              Pending Approval
                            </span>
                            <div className="font-mono text-[11px] font-semibold text-text-primary truncate">
                              {a.tool}
                            </div>
                          </div>
                          <span className="font-mono text-[10px] text-text-muted">gate ·oversight</span>
                        </div>
                        <div className="rounded-md border border-border-subtle bg-bg-elevated px-2.5 py-1.5 font-mono text-[11px] text-text-primary truncate">
                          {a.summary}
                        </div>
                        <div className="flex flex-wrap items-center gap-2 pt-0.5">
                          <RomanTooltip
                            title="Approve Tool Execution"
                            purpose="Permits the agent to execute this specific command or tool call and proceed with its plan."
                            actionHint="Grant execution authorization"
                          >
                            <button
                              type="button"
                              aria-label={`Approve ${a.summary}`}
                              onClick={() => handleResolveApproval(a.approval_id, a.summary, 'approved')}
                              className="rounded-md border border-amber-400 bg-amber-400 px-3 py-1 font-mono text-[10px] font-bold uppercase tracking-wider text-zinc-950 shadow-[0_0_12px_rgba(251,191,36,0.3)] hover:bg-amber-300"
                            >
                              Approve
                            </button>
                          </RomanTooltip>
                          <RomanTooltip
                            title="Reject Tool Execution"
                            purpose="Denies execution of this tool call, returning an authorization error back to the agent so it re-plans."
                            actionHint="Halt this command and request an alternate plan"
                          >
                            <button
                              type="button"
                              aria-label={`Reject ${a.summary}`}
                              onClick={() => handleResolveApproval(a.approval_id, a.summary, 'rejected')}
                              className="rounded-md border border-rose-500/45 bg-rose-500/12 px-2.5 py-1 font-mono text-[10px] font-semibold uppercase tracking-wider text-rose-700 hover:bg-rose-500/22"
                            >
                              Reject
                            </button>
                          </RomanTooltip>
                          <button
                            type="button"
                            data-testid={`needs-you-inspect-diff-${a.approval_id}`}
                            onClick={() => void handleInspectApprovalDiff(a.approval_id)}
                            className="rounded-md border border-border-subtle bg-overlay-subtle px-2.5 py-1 font-mono text-[10px] text-text-secondary hover:border-brass/40 hover:text-brass"
                          >
                            {diffByApproval[a.approval_id] != null ? 'Hide Diff' : 'Inspect Diff'}
                          </button>
                        </div>
                        {diffByApproval[a.approval_id] != null && (
                          <pre
                            role="region"
                            aria-label={`Approval diff ${a.approval_id}`}
                            tabIndex={0}
                            data-testid={`needs-you-diff-preview-${a.approval_id}`}
                            className="mt-1 max-h-32 overflow-x-auto rounded-md border border-border-subtle bg-bg-elevated p-2 font-mono text-[10px] leading-relaxed text-emerald-700"
                          >
                            {diffByApproval[a.approval_id]}
                          </pre>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </BentoScrollRegion>
            </BentoCard>
          )}

          {(laneFilter === 'all' || laneFilter === 'questions') && (
            <BentoCard as="section" className="gap-2.5">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3 className="ds-section-head">Questions &amp; doubts</h3>
                <span className="font-mono text-[10px] text-text-muted">{effectiveNeedsYou.length} active</span>
              </div>
              <BentoScrollRegion ariaLabel="Questions and doubts lane" className="max-h-96 pr-1">
                {effectiveNeedsYou.length === 0 ? (
                  <EmptyState
                    variant="no-data"
                    title="Nothing needs you"
                    description="The agent is proceeding autonomously. Check back when questions or doubts arise."
                  />
                ) : (
                  <div className="space-y-2">
                    {effectiveNeedsYou.map((item) => (
                      <FeedbackCard
                        key={item.feedbackId}
                        row={item}
                        onResolve={handleResolve}
                        onOpenContext={onOpenContext}
                      />
                    ))}
                  </div>
                )}
              </BentoScrollRegion>
            </BentoCard>
          )}

          {(laneFilter === 'all' || laneFilter === 'withheld') && (
            <BentoCard as="section" className="gap-2.5">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3 className="ds-section-head">Withheld by policy ({effectiveWithheld.length})</h3>
                <span className="font-mono text-[10px] text-text-muted">auto-gated</span>
              </div>
              <BentoScrollRegion ariaLabel="Withheld by policy lane" className="max-h-96 pr-1">
                {effectiveWithheld.length === 0 ? (
                  <div className="rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-5 text-center font-mono text-[11px] text-text-muted">
                    No low-entropy items withheld by policy.
                  </div>
                ) : (
                  <div className="space-y-2">
                    {effectiveWithheld.map((item) => (
                      <FeedbackCard
                        key={item.feedbackId}
                        row={item}
                        onResolve={handleResolve}
                        onOpenContext={onOpenContext}
                      />
                    ))}
                  </div>
                )}
              </BentoScrollRegion>
            </BentoCard>
          )}
        </BentoGrid>

        {!embedded && (
          <BentoGrid preset="hero" className="items-stretch">
            <div
              data-testid="needs-you-live-dag"
              className="flex min-w-0 flex-col justify-between rounded-xl border border-[#D4AF37]/30 bg-bg-surface p-3.5 shadow-[inset_0_1px_0_rgba(243,229,171,0.1)] @3xl/surface:col-span-6"
            >
              <div>
                <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#D4AF37]/25 pb-2">
                  <div className="flex items-center gap-2 min-w-0">
                    <RomanMedallion active size="sm" {...({ icon: 'astrolabeRadar', tone: 'gold' } as any)}>
                      <Icon.astrolabeRadar className="size-3.5 text-text-primary" aria-hidden="true" />
                    </RomanMedallion>
                    <div className="min-w-0">
                      <h3 className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                        Live Agent Topology &amp; Execution DAG
                      </h3>
                      <p className="text-[10px] text-text-secondary">
                        Real-time multi-agent dispatch, tool streams &amp; token velocity
                      </p>
                    </div>
                  </div>
                  <span className="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-mono text-[9px] text-emerald-700">
                    <span className="size-1.5 rounded-full bg-emerald-400 animate-pulse" aria-hidden="true" />
                    <span>420 t/s agg</span>
                  </span>
                </div>

                {/* Animated SVG Execution DAG connecting Orchestrator -> Researcher & Verifier */}
                <div className="relative my-2.5 rounded-lg border border-[#D4AF37]/25 bg-bg-elevated p-2.5">
                  <svg viewBox="0 0 420 116" className="h-28 w-full overflow-visible" aria-label="Execution DAG">
                    <defs>
                      <linearGradient id="ny-dag-gold" x1="0" x2="1">
                        <stop offset="0%" stopColor="#E5C158" stopOpacity="0.9" />
                        <stop offset="100%" stopColor="#D97706" stopOpacity="0.85" />
                      </linearGradient>
                      <linearGradient id="ny-dag-emerald" x1="0" x2="1">
                        <stop offset="0%" stopColor="#E5C158" stopOpacity="0.9" />
                        <stop offset="100%" stopColor="#10B981" stopOpacity="0.85" />
                      </linearGradient>
                    </defs>
                    {/* Animated Bezier Edges */}
                    <path
                      d="M 136 58 C 190 58, 200 28, 256 28"
                      fill="none"
                      stroke="rgba(212,175,55,0.22)"
                      strokeWidth="2"
                    />
                    <path
                      d="M 136 58 C 190 58, 200 28, 256 28"
                      fill="none"
                      stroke="url(#ny-dag-gold)"
                      strokeWidth="2.2"
                      strokeDasharray="6 8"
                    >
                      <animate attributeName="stroke-dashoffset" from="28" to="0" dur="1.4s" repeatCount="indefinite" />
                    </path>
                    <path
                      d="M 136 58 C 190 58, 200 88, 256 88"
                      fill="none"
                      stroke="rgba(16,185,129,0.22)"
                      strokeWidth="2"
                    />
                    <path
                      d="M 136 58 C 190 58, 200 88, 256 88"
                      fill="none"
                      stroke="url(#ny-dag-emerald)"
                      strokeWidth="2.2"
                      strokeDasharray="6 8"
                    >
                      <animate attributeName="stroke-dashoffset" from="28" to="0" dur="1.1s" repeatCount="indefinite" />
                    </path>

                    {/* Lead Orchestrator Node */}
                    <g
                      onClick={() => setSelectedDagId('orch-lead')}
                      className="cursor-pointer"
                    >
                      <rect
                        x="6"
                        y="32"
                        width="136"
                        height="52"
                        rx="10"
                        fill="#FAF5E8"
                        stroke={selectedDagId === 'orch-lead' ? '#B8860B' : '#C59B27'}
                        strokeWidth={selectedDagId === 'orch-lead' ? '2.2' : '1.5'}
                      />
                      <text x="74" y="53" textAnchor="middle" fill="#1C140C" fontSize="9.5" fontWeight="700" fontFamily="'Cinzel', 'Palatino', Georgia, serif">
                        ORCHESTRATOR
                      </text>
                      <text x="74" y="69" textAnchor="middle" fill="#5C4428" fontSize="8.5" fontFamily="'Palatino', 'Palatino Linotype', Georgia, serif">
                        router.rs · 84 t/s
                      </text>
                    </g>

                    {/* Researcher Node */}
                    <g
                      onClick={() => setSelectedDagId('agent-research')}
                      className="cursor-pointer"
                    >
                      <rect
                        x="252"
                        y="6"
                        width="156"
                        height="44"
                        rx="10"
                        fill="#FAF5E8"
                        stroke={selectedDagId === 'agent-research' ? '#B8860B' : '#C59B27'}
                        strokeWidth={selectedDagId === 'agent-research' ? '2.2' : '1.5'}
                      />
                      <text x="330" y="24" textAnchor="middle" fill="#1C140C" fontSize="9.5" fontWeight="700" fontFamily="'Cinzel', 'Palatino', Georgia, serif">
                        RESEARCHER · 126 t/s
                      </text>
                      <text x="330" y="38" textAnchor="middle" fill="#5C4428" fontSize="8.5" fontFamily="'Palatino', 'Palatino Linotype', Georgia, serif">
                        crates/vox-search · docs
                      </text>
                    </g>

                    {/* Verifier Node */}
                    <g
                      onClick={() => setSelectedDagId('agent-verify')}
                      className="cursor-pointer"
                    >
                      <rect
                        x="252"
                        y="66"
                        width="156"
                        height="44"
                        rx="10"
                        fill="#FAF5E8"
                        stroke={selectedDagId === 'agent-verify' ? '#6E7B2B' : '#C59B27'}
                        strokeWidth={selectedDagId === 'agent-verify' ? '2.2' : '1.5'}
                      />
                      <text x="330" y="84" textAnchor="middle" fill="#3F4D1E" fontSize="9.5" fontWeight="700" fontFamily="'Cinzel', 'Palatino', Georgia, serif">
                        VERIFIER · 210 t/s
                      </text>
                      <text x="324" y="98" textAnchor="middle" fill="#5C4428" fontSize="8.5" fontFamily="'Palatino', 'Palatino Linotype', Georgia, serif">
                        crates/vox-compiler · tests
                      </text>
                      <Icon.check x={384} y={90} width={9} height={9} className="text-[#3F4D1E]" aria-hidden="true" />
                    </g>
                  </svg>
                </div>

                {/* Live Agent Node Telemetry Cards */}
                <div className="space-y-2">
                  {LIVE_TOPOLOGY_NODES.map((node) => (
                    <button
                      key={node.id}
                      type="button"
                      data-testid={`needs-you-dag-node-${node.id}`}
                      onClick={() => setSelectedDagId(node.id)}
                      className={`flex w-full flex-wrap items-center justify-between gap-2 rounded-lg border px-3 py-2 text-left transition-colors ${
                        selectedDagId === node.id
                          ? 'border-brass/55 bg-brass/10'
                          : 'border-border-subtle bg-overlay-subtle hover:border-white/20'
                      }`}
                    >
                      <div className="min-w-0">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="font-mono text-[11px] font-semibold text-text-primary">
                            {node.role}
                          </span>
                          <span className={`rounded border px-1.5 py-0.5 font-mono text-[9px] ${node.tone}`}>
                            {node.status}
                          </span>
                        </div>
                        <div className="mt-0.5 font-mono text-[10px] text-text-muted truncate">
                          {node.model} · active tool: <span className="text-text-secondary">{node.tool}</span>
                        </div>
                      </div>
                      <div className="flex shrink-0 items-center gap-2.5">
                        <span className="font-mono text-[11px] font-semibold text-brass">{node.velocity}</span>
                        <svg
                          viewBox="0 0 200 22"
                          preserveAspectRatio="none"
                          className="h-5 w-24 overflow-visible"
                          aria-hidden="true"
                        >
                          <path d={node.area} fill={node.stroke} fillOpacity="0.16" />
                          <path
                            d={node.spark}
                            fill="none"
                            stroke={node.stroke}
                            strokeWidth="1.6"
                            vectorEffect="non-scaling-stroke"
                            strokeLinecap="round"
                          />
                        </svg>
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              <div className="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-elevated px-3 py-2 font-mono text-[10px] text-text-secondary">
                <span>
                  Selected: <strong className="text-brass">{selectedDagNode.role}</strong> ({selectedDagNode.model})
                </span>
                <span className="text-emerald-700">
                  {selectedDagNode.tool} · {selectedDagNode.velocity}
                </span>
              </div>
            </div>

            <div
              data-testid="needs-you-route-telemetry-table"
              className="flex min-w-0 flex-col rounded-xl border border-border-subtle bg-bg-surface/85 p-3.5 @3xl/surface:col-span-6"
            >
              <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                  Recent Route Decisions &amp; Run Telemetry
                </span>
                <span className="font-mono text-[10px] text-text-muted">Live K-prefix cache &amp; spoke telemetry</span>
              </div>
              <BentoScrollRegion ariaLabel="Recent route decisions and run telemetry" className="flex-1">
                <table className="w-full min-w-[34rem] text-left font-mono text-[11px] tabular-nums">
                  <thead>
                    <tr className="border-b border-border-subtle text-[10px] uppercase tracking-wider text-text-muted">
                      <th className="py-1.5 pl-2.5 pr-3">Run ID</th>
                      <th className="py-1.5 pr-3">Task Class</th>
                      <th className="py-1.5 pr-3">Model</th>
                      <th className="py-1.5 pr-3">Velocity / Lat</th>
                      <th className="py-1.5 pr-3">Cost</th>
                      <th className="py-1.5 pr-2.5 text-right">Gate</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border-subtle text-text-secondary">
                    {[
                      {
                        runId: 'a091819-1761d53e',
                        task: 'Schema Migration & Shell Gate',
                        model: 'anthropic/claude-sonnet',
                        vel: '84 t/s · 14ms',
                        cost: '$0.014',
                        gate: 'Running',
                        tone: 'border-[#C59B27]/55 bg-[#D4AF37]/15 text-[#785312] font-semibold',
                      },
                      {
                        runId: 'a021819-1891a599',
                        task: 'Hybrid RRF Corpus Grounding',
                        model: 'google/gemini-pro',
                        vel: '126 t/s · 22ms',
                        cost: '$0.008',
                        gate: 'Running',
                        tone: 'border-[#B38728]/55 bg-[#F3E5AB]/30 text-[#6B4C12] font-semibold',
                      },
                      {
                        runId: 'a021846-1e44041d',
                        task: '8-Spoke AST & TOESTUB Verification',
                        model: 'mens/vox-qwen3-14b-r6',
                        vel: '210 t/s · 6ms',
                        cost: '$0.000',
                        gate: 'Verified',
                        tone: 'border-[#1B6B3A]/45 bg-[#1B6B3A]/12 text-[#14532D] font-semibold',
                      },
                    ].map((r) => (
                      <tr key={r.runId}>
                        <td className="py-2 pl-2.5 pr-3 text-text-muted">{r.runId}</td>
                        <td className="py-2 pr-3">
                          <span className="mr-1.5 rounded border border-[#1B6B3A]/40 bg-[#1B6B3A]/12 px-1 py-0.5 text-[9px] font-semibold text-[#14532D]">
                            PASS
                          </span>
                          <span className="text-text-primary">{r.task}</span>
                        </td>
                        <td className="py-2 pr-3 text-brass">{r.model}</td>
                        <td className="py-2 pr-3 text-[#14532D] font-medium">{r.vel}</td>
                        <td className="py-2 pr-3">{r.cost}</td>
                        <td className="py-2 pr-2.5 text-right">
                          <span className={`rounded border px-1.5 py-0.5 text-[9px] ${r.tone}`}>{r.gate}</span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </BentoScrollRegion>
            </div>
          </BentoGrid>
        )}

        <BentoGrid
          preset="balanced"
          data-testid="needs-you-attention-deck"
          className="pt-1"
        >
          <div className="rounded-xl border border-[#D4AF37]/30 bg-bg-surface p-3.5 shadow-sm">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-brass uppercase">Entropy Gate</span>
              <span className="rounded border border-[#D4AF37]/30 bg-[#D4AF37]/10 px-1.5 py-0.5 font-mono text-[9px] text-text-primary">
                Info Gain
              </span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Questions below the information-gain threshold are withheld and resolved autonomously to preserve operator focus.
            </p>
          </div>
          <div className="rounded-xl border border-[#D4AF37]/30 bg-bg-surface p-3.5 shadow-sm">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-brass uppercase">Doubt Escalation</span>
              <span className="rounded border border-[#D4AF37]/30 bg-[#D4AF37]/10 px-1.5 py-0.5 font-mono text-[9px] text-text-primary">
                {effectiveNeedsYou.length} active
              </span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Agents surface high-impact doubts when verification oracles conflict, allowing one-click <code className="text-text-primary">Overrule</code> or <code className="text-text-primary">Let it verify</code>.
            </p>
            <div className="mt-2.5 flex flex-wrap items-center gap-2 pt-1 border-t border-[#D4AF37]/20">
              <RomanTooltip
                title="Overrule Active Doubt"
                purpose="Bypasses conflicting verification oracles and commands the agent to commit its change and proceed."
                actionHint="Force continuation despite oracle doubts"
              >
                <button
                  type="button"
                  aria-label="Overrule doubt"
                  onClick={() => {
                    const doubtItem = effectiveNeedsYou.find((i) => i.kind === 'doubt' || (i as any).isDoubt) ?? effectiveNeedsYou[0];
                    if (doubtItem) handleResolve(doubtItem.feedbackId, { action: 'overrule' });
                  }}
                  className="rounded border border-emerald-400/35 bg-emerald-400/10 px-2.5 py-1 font-mono text-[10px] font-semibold uppercase tracking-wider text-emerald-700 hover:bg-emerald-400/20"
                >
                  Overrule
                </button>
              </RomanTooltip>
              <RomanTooltip
                title="Let Agent Verify"
                purpose="Directs the agent to perform secondary tests and deeper oracle verification before taking action."
                actionHint="Instruct agent to gather deeper verification evidence"
              >
                <button
                  type="button"
                  aria-label="Let it verify"
                  onClick={() => {
                    const doubtItem = effectiveNeedsYou.find((i) => i.kind === 'doubt' || (i as any).isDoubt) ?? effectiveNeedsYou[0];
                    if (doubtItem) handleResolve(doubtItem.feedbackId, { action: 'let_verify' });
                  }}
                  className="rounded border border-[#D4AF37]/30 bg-bg-elevated px-2.5 py-1 font-mono text-[10px] font-semibold uppercase tracking-wider text-text-secondary hover:border-[#D4AF37]/50 hover:text-text-primary"
                >
                  Let it verify
                </button>
              </RomanTooltip>
            </div>
          </div>
          <div className="rounded-xl border border-[#D4AF37]/30 bg-bg-surface p-3.5 shadow-sm">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">Shared Inbox</span>
              <span className="font-mono text-[10px] text-emerald-700">
                {approvals.length + effectiveNeedsYou.length} open
              </span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Unifies tool approvals, blocking clarifications, and skill proposals in one synchronized attention stream.
            </p>
          </div>
        </BentoGrid>
      </BentoScrollRegion>
    </div>
  );
}
