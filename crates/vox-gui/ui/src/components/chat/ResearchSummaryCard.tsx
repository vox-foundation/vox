import React from 'react';
import type { ResearchSummary, ResearchClaimsBreakdown } from '../../lib/types';
import { cn } from '../../lib/cn';
import { ExternalLink, CheckCircle2, Clock, AlertTriangle } from 'lucide-react';

export interface ResearchSummaryCardProps {
  summary?: ResearchSummary;
  sessionId?: string;
  query?: string;
  status?: string;
  claims?: Partial<ResearchClaimsBreakdown>;
  onOpenResearch?: (sessionId: string) => void;
  className?: string;
}

export function formatResearchStatus(status?: string): { label: string; tone: 'complete' | 'progress' | 'refuted' | 'neutral' } {
  if (!status) {
    return { label: 'Research Complete', tone: 'complete' };
  }
  const s = status.toLowerCase().trim();
  if (s === 'in_progress' || s === 'running' || s === 'pending' || s.includes('in progress')) {
    return { label: 'Research In Progress', tone: 'progress' };
  }
  if (s === 'refuted' || s.includes('refuted')) {
    return { label: 'Refuted by Evidence', tone: 'refuted' };
  }
  if (s === 'complete' || s === 'done' || s.includes('complete')) {
    return { label: 'Research Complete', tone: 'complete' };
  }
  return { label: status, tone: 'neutral' };
}

export function ResearchSummaryCard({
  summary,
  sessionId,
  query,
  status,
  claims,
  onOpenResearch,
  className,
}: ResearchSummaryCardProps) {
  const activeSessionId = summary?.sessionId ?? sessionId ?? '';
  const activeQuery = summary?.query ?? summary?.topic ?? query;
  const rawStatus = summary?.status ?? status;
  const { label: statusLabel, tone } = formatResearchStatus(rawStatus);

  const claimsBreakdown: ResearchClaimsBreakdown = {
    supported: summary?.claims?.supported ?? claims?.supported ?? 0,
    contested: summary?.claims?.contested ?? claims?.contested ?? 0,
    refuted: summary?.claims?.refuted ?? claims?.refuted ?? 0,
  };

  const badgeStyles = {
    complete: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300',
    progress: 'border-cyan-500/30 bg-cyan-500/10 text-cyan-300',
    refuted: 'border-rose-500/30 bg-rose-500/10 text-rose-300',
    neutral: 'border-border-subtle bg-overlay-subtle text-text-secondary',
  };

  const statusIcons = {
    complete: <CheckCircle2 className="size-3 text-emerald-400" aria-hidden="true" />,
    progress: <Clock className="size-3 text-cyan-400 animate-pulse" aria-hidden="true" />,
    refuted: <AlertTriangle className="size-3 text-rose-400" aria-hidden="true" />,
    neutral: null,
  };

  return (
    <div
      data-testid="research-summary-card"
      className={cn(
        'my-2 rounded-xl border border-border-subtle bg-surface-subtle/80 p-3 shadow-md backdrop-blur-sm transition-all',
        className
      )}
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle/60 pb-2">
        <div className="flex items-center gap-2">
          <span
            data-testid="research-status-badge"
            className={cn(
              'inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-medium',
              badgeStyles[tone]
            )}
          >
            {statusIcons[tone]}
            <span>{statusLabel}</span>
          </span>
          {activeSessionId && (
            <span
              data-testid="research-session-id"
              className="font-mono text-[10px] text-text-muted"
            >
              #{activeSessionId}
            </span>
          )}
        </div>

        <button
          type="button"
          data-testid="open-research-studio-btn"
          onClick={() => onOpenResearch?.(activeSessionId)}
          className="inline-flex items-center gap-1.5 rounded-lg border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-[11px] font-medium text-text-primary hover:border-accent hover:bg-overlay-muted transition-colors cursor-pointer"
        >
          <span>Open in Research Studio</span>
          <ExternalLink className="size-3 text-text-muted" aria-hidden="true" />
        </button>
      </div>

      {activeQuery && (
        <div className="mt-2.5">
          <h4
            data-testid="research-summary-topic"
            className="text-[12px] font-semibold text-text-primary leading-tight line-clamp-2"
          >
            {activeQuery}
          </h4>
        </div>
      )}

      {summary?.summary && (
        <p
          data-testid="research-summary-text"
          className="mt-1.5 text-[11px] leading-relaxed text-text-secondary line-clamp-3"
        >
          {summary.summary}
        </p>
      )}

      <div className="mt-2.5 flex flex-wrap items-center gap-2 pt-1 font-mono text-[10px]">
        <span
          data-testid="pill-supported"
          className="inline-flex items-center rounded-md border border-emerald-500/25 bg-emerald-500/10 px-2 py-0.5 text-emerald-300"
        >
          {claimsBreakdown.supported} Supported
        </span>
        <span
          data-testid="pill-contested"
          className="inline-flex items-center rounded-md border border-amber-500/25 bg-amber-500/10 px-2 py-0.5 text-amber-300"
        >
          {claimsBreakdown.contested} Contested
        </span>
        <span
          data-testid="pill-refuted"
          className="inline-flex items-center rounded-md border border-rose-500/25 bg-rose-500/10 px-2 py-0.5 text-rose-300"
        >
          {claimsBreakdown.refuted} Refuted
        </span>

        {typeof summary?.sourcesCount === 'number' && summary.sourcesCount > 0 && (
          <span
            data-testid="pill-sources"
            className="inline-flex items-center rounded-md border border-border-subtle bg-overlay-subtle px-2 py-0.5 text-text-muted"
          >
            {summary.sourcesCount} Sources
          </span>
        )}
      </div>
    </div>
  );
}
