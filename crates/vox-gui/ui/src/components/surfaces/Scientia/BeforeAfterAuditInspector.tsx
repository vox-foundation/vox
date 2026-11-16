import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { getV76AuditDiff, type V76AuditAxisDelta, type V76AuditDiffDto } from '../../../api/scientia';
import { Icon } from '../../ui/Icons';

export interface BeforeAfterAuditInspectorProps {
  initialData?: V76AuditDiffDto | null;
  compact?: boolean;
}

const DEFAULT_VIEWPORT_HEIGHT = 280;
const ROW_HEIGHT = 28;

function formatMetricNumber(val: number): string {
  return Number.isInteger(val) ? String(val) : Number(val.toFixed(4)).toString();
}

function formatSignedDelta(val: number): string {
  const formatted = formatMetricNumber(Math.abs(val));
  return val > 0 ? `+${formatted}` : val < 0 ? `-${formatted}` : '0';
}

function niVerdictBadgeLabel(verdict: string): string {
  if (verdict === 'non_inferior') return 'NonInferior';
  if (verdict === 'inferior') return 'Inferior';
  return 'Inconclusive';
}

function isFavorableDelta(axis: V76AuditAxisDelta): boolean {
  return axis.higher_is_better ? axis.delta >= 0 : axis.delta <= 0;
}

export function BeforeAfterAuditInspector({
  initialData = null,
  compact = false,
}: BeforeAfterAuditInspectorProps) {
  const [data, setData] = useState<V76AuditDiffDto | null>(initialData);
  const [loading, setLoading] = useState<boolean>(!initialData);
  const [diffMode, setDiffMode] = useState<'all' | 'changes'>('all');
  const [axisFilter, setAxisFilter] = useState<string>('all');
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const loadDiff = useCallback(async () => {
    setLoading(true);
    try {
      setData(await getV76AuditDiff());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (initialData) {
      setData(initialData);
      setLoading(false);
      return;
    }
    void loadDiff();
  }, [initialData, loadDiff]);

  const axes = useMemo(() => data?.axes ?? [], [data]);
  const bugs = useMemo(() => data?.bugs ?? [], [data]);
  const hunks = useMemo(() => data?.hunks ?? [], [data]);
  const visibleAxes = useMemo(
    () => (axisFilter === 'all' ? axes : axes.filter((a) => a.axis === axisFilter)),
    [axes, axisFilter],
  );
  const activeAxisObj = useMemo(
    () => axes.find((a) => a.axis === axisFilter) ?? null,
    [axes, axisFilter],
  );

  const filteredHunks = useMemo(() => {
    return hunks.filter((h) => {
      if (diffMode === 'changes' && h.tag === 'equal') return false;
      if (axisFilter !== 'all') {
        const lower = h.text.toLowerCase();
        const matchesAxis = lower.includes(axisFilter.toLowerCase());
        const matchesMetric = activeAxisObj
          ? lower.includes(activeAxisObj.metric.toLowerCase())
          : false;
        return matchesAxis || matchesMetric;
      }
      return true;
    });
  }, [hunks, diffMode, axisFilter, activeAxisObj]);

  const viewportHeight = compact ? 200 : DEFAULT_VIEWPORT_HEIGHT;
  const virtualizer = useVirtualizer({
    count: filteredHunks.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 8,
    initialRect: { width: 800, height: viewportHeight },
    observeElementRect: (instance, cb) => {
      const el = instance.scrollElement;
      if (!el) return;
      const emit = () => {
        const rect = el.getBoundingClientRect();
        cb({
          width: Math.round(el.clientWidth || el.offsetWidth || rect.width || 800),
          height: Math.round(el.clientHeight || el.offsetHeight || rect.height || viewportHeight),
        });
      };
      emit();
      if (typeof ResizeObserver === 'undefined') return;
      const ro = new ResizeObserver(() => emit());
      ro.observe(el);
      return () => ro.disconnect();
    },
  });

  const virtualItems = virtualizer.getVirtualItems();
  const resolvedBugsCount = useMemo(() => bugs.filter((b) => b.resolved).length, [bugs]);
  const lessons = data?.lessons_summary ?? null;

  return (
    <div
      data-testid="before-after-audit-inspector"
      data-slot="before-after-audit-inspector"
      className="rounded-xl border border-border-subtle bg-overlay-subtle p-4 space-y-4"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-display text-xs uppercase tracking-[0.16em] text-text-primary">
            v7.6 Before / After Empirical Audit Closure
          </span>
          <span
            data-testid="audit-stage-pill"
            className="inline-flex items-center gap-1 rounded-full border border-brass/40 bg-brass/10 px-2.5 py-0.5 font-mono text-[10px] text-brass"
          >
            <Icon.scale className="size-3 shrink-0" aria-hidden="true" />
            <span>{data?.before_stage ?? 'before'} to {data?.after_stage ?? 'after'}</span>
          </span>
          <span
            data-testid="audit-bug-closure-pill"
            className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 font-mono text-[10px] ${
              bugs.length > 0 && resolvedBugsCount === bugs.length
                ? 'border-emerald-400/40 bg-emerald-400/10 text-emerald-200'
                : 'border-amber-400/40 bg-amber-400/10 text-amber-200'
            }`}
          >
            <Icon.check className="size-3 shrink-0" aria-hidden="true" />
            <span>{resolvedBugsCount}/{bugs.length} Live Bugs Closed</span>
          </span>
        </div>
        <button
          type="button"
          data-testid="refresh-audit-diff-btn"
          disabled={loading}
          onClick={() => { void loadDiff(); }}
          className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2.5 py-1 font-mono text-[10px] text-text-secondary hover:text-text-primary hover:border-brass/40 disabled:opacity-50"
        >
          <Icon.refresh className="size-3 shrink-0" aria-hidden="true" />
          <span>{loading ? 'Refreshing...' : 'Refresh'}</span>
        </button>
      </div>

      {lessons && (
        <div
          data-testid="lessons-summary-bar"
          className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-subtle bg-black/30 px-3 py-2 font-mono text-[11px]"
        >
          <div className="flex flex-wrap items-center gap-3 text-text-secondary">
            <span>Quarantined Findings: <strong className="text-text-primary">{lessons.quarantined_findings_count}</strong></span>
            <span>Corroborated Promotions: <strong className="text-emerald-300">{lessons.corroborated_promotions_count}</strong></span>
            <span>Proposed Rules (.proposed): <strong className="text-brass">{lessons.proposed_effort_rules_count}</strong></span>
            <span>Auto-DPO Pairs: <strong className="text-text-primary">{lessons.auto_dpo_pairs_minted}</strong></span>
          </div>
          <span
            data-testid="unreviewed-ssot-mutations-pill"
            className={`inline-flex items-center gap-1 rounded border px-2 py-0.5 text-[10px] ${
              lessons.unreviewed_ssot_mutations === 0
                ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
                : 'border-rose-500/40 bg-rose-500/10 text-rose-300'
            }`}
          >
            <Icon.shield className="size-3 shrink-0" aria-hidden="true" />
            <span>unreviewed_ssot_mutations: {lessons.unreviewed_ssot_mutations}</span>
          </span>
        </div>
      )}

      <div className="flex flex-wrap items-center justify-between gap-2">
        <div role="group" aria-label="Filter diff hunks" className="flex flex-wrap items-center gap-1">
          {(['all', 'changes'] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              data-testid={`diff-filter-${mode}`}
              aria-pressed={diffMode === mode}
              onClick={() => setDiffMode(mode)}
              className={`rounded px-2.5 py-1 font-mono text-[10px] uppercase tracking-wider transition-colors ${
                diffMode === mode
                  ? 'border border-brass/50 bg-brass/20 text-brass font-semibold'
                  : 'border border-border-subtle bg-black/20 text-text-muted hover:text-text-secondary'
              }`}
            >
              {mode === 'all' ? 'All' : 'Changes Only'}
            </button>
          ))}
        </div>
        <div role="group" aria-label="Filter by evaluation axis" className="flex flex-wrap items-center gap-1">
          <button
            type="button"
            data-testid="axis-filter-all"
            aria-pressed={axisFilter === 'all'}
            onClick={() => setAxisFilter('all')}
            className={`rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider transition-colors ${
              axisFilter === 'all'
                ? 'border border-brass/50 bg-brass/20 text-brass'
                : 'border border-border-subtle bg-black/20 text-text-muted hover:text-text-secondary'
            }`}
          >
            All Axes
          </button>
          {axes.map((a) => (
            <button
              key={a.axis}
              type="button"
              data-testid={`axis-filter-${a.axis}`}
              aria-pressed={axisFilter === a.axis}
              onClick={() => setAxisFilter(a.axis)}
              className={`rounded px-2 py-0.5 font-mono text-[10px] transition-colors ${
                axisFilter === a.axis
                  ? 'border border-brass/50 bg-brass/20 text-brass'
                  : 'border border-border-subtle bg-black/20 text-text-muted hover:text-text-secondary'
              }`}
            >
              {a.axis}
            </button>
          ))}
        </div>
      </div>

      <div data-testid="audit-axes-grid" className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-3">
        {visibleAxes.map((axis) => {
          const favorable = isFavorableDelta(axis);
          return (
            <div
              key={axis.axis}
              data-testid={`audit-axis-card-${axis.axis}`}
              className="rounded-lg border border-border-subtle bg-black/30 p-3 space-y-1.5"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="truncate font-display text-[11px] font-semibold text-text-primary">{axis.label}</span>
                <div className="flex items-center gap-1 shrink-0">
                  <span
                    data-testid={`axis-ni-pill-${axis.axis}`}
                    className={`rounded border px-1.5 py-0.5 font-mono text-[9px] uppercase ${
                      axis.ni_verdict === 'non_inferior'
                        ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
                        : axis.ni_verdict === 'inferior'
                          ? 'border-rose-500/40 bg-rose-500/10 text-rose-300'
                          : 'border-amber-500/40 bg-amber-500/10 text-amber-300'
                    }`}
                  >
                    {niVerdictBadgeLabel(axis.ni_verdict)}
                  </span>
                  {axis.superior && (
                    <span
                      data-testid={`axis-superior-pill-${axis.axis}`}
                      className="rounded border border-brass/50 bg-brass/20 px-1.5 py-0.5 font-mono text-[9px] uppercase text-brass"
                    >
                      Superior
                    </span>
                  )}
                </div>
              </div>
              <div className="font-mono text-[10px] text-text-muted truncate">{axis.metric}</div>
              <div className="flex items-center justify-between font-mono text-xs">
                <span className="text-text-secondary">
                  {formatMetricNumber(axis.before_value)} to {formatMetricNumber(axis.after_value)}
                </span>
                <span className={favorable ? 'text-emerald-300 font-semibold' : 'text-rose-300'}>
                  {formatSignedDelta(axis.delta)}
                </span>
              </div>
              <div data-testid={`axis-ci-bounds-${axis.axis}`} className="font-mono text-[10px] text-text-muted">
                95% CI [{formatMetricNumber(axis.ci_low)}, {formatMetricNumber(axis.ci_high)}]
              </div>
            </div>
          );
        })}
      </div>

      <div data-testid="audit-bug-matrix" className="space-y-1.5">
        <div className="font-mono text-[10px] uppercase tracking-wider text-text-muted">
          9-Probe Live Bug Reproduction Matrix
        </div>
        <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2 lg:grid-cols-3">
          {bugs.map((bug) => (
            <div
              key={bug.id}
              data-testid={`bug-status-${bug.id}`}
              className="flex items-center justify-between gap-2 rounded border border-border-subtle bg-black/25 px-2.5 py-1.5 font-mono text-[10px]"
            >
              <div className="min-w-0 truncate">
                <div className="truncate text-text-secondary" title={bug.id}>{bug.id}</div>
                <div className="text-[9px] text-text-muted">{String(bug.before)} to {String(bug.after)}</div>
              </div>
              <span
                className={`inline-flex shrink-0 items-center gap-1 rounded border px-1.5 py-0.5 text-[9px] uppercase ${
                  bug.resolved
                    ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
                    : 'border-rose-500/40 bg-rose-500/10 text-rose-300'
                }`}
              >
                {bug.resolved ? (
                  <Icon.check className="size-2.5 shrink-0" aria-hidden="true" />
                ) : (
                  <Icon.alert className="size-2.5 shrink-0" aria-hidden="true" />
                )}
                <span>{bug.resolved ? 'Resolved' : 'Open'}</span>
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="space-y-1.5">
        <div className="flex items-center justify-between font-mono text-[10px] text-text-muted">
          <span className="uppercase tracking-wider">Virtualized Receipt Diff Hunks</span>
          <span>Showing {filteredHunks.length} of {hunks.length} lines</span>
        </div>
        <div
          ref={scrollRef}
          role="region"
          tabIndex={0}
          aria-label="Virtualized audit receipt diff hunks"
          data-testid="audit-diff-viewport"
          style={{ height: Math.min(viewportHeight, Math.max(ROW_HEIGHT * 2, filteredHunks.length * ROW_HEIGHT)) }}
          className="overflow-y-auto rounded-lg border border-border-subtle bg-black/40 font-mono text-[11px]"
        >
          <div style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative', width: '100%' }}>
            {virtualItems.map((vItem) => {
              const line = filteredHunks[vItem.index];
              if (!line) return null;
              const prefix = line.tag === 'delete' ? '-' : line.tag === 'insert' ? '+' : ' ';
              const rowTone =
                line.tag === 'delete'
                  ? 'bg-rose-500/15 text-rose-200'
                  : line.tag === 'insert'
                    ? 'bg-emerald-500/15 text-emerald-200'
                    : 'text-text-secondary';
              return (
                <div
                  key={line.index}
                  data-testid="audit-diff-hunk-row"
                  data-tag={line.tag}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${vItem.size}px`,
                    transform: `translateY(${vItem.start}px)`,
                  }}
                  className={`flex items-center gap-2 px-2.5 whitespace-pre overflow-hidden ${rowTone}`}
                >
                  <span className="w-8 shrink-0 text-right text-[9px] text-text-muted select-none">{line.old_line ?? ''}</span>
                  <span className="w-8 shrink-0 text-right text-[9px] text-text-muted select-none">{line.new_line ?? ''}</span>
                  <span className="w-3 shrink-0 font-bold select-none">{prefix}</span>
                  <span className="truncate">{line.text}</span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
