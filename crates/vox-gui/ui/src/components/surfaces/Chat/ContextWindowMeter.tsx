import React from 'react';

export type ContextSliceKey = 'system' | 'kb' | 'ast' | 'history';

export interface ContextSliceBreakdown {
  systemTokens: number;
  kbTokens: number;
  astTokens: number;
  historyTokens: number;
}

export interface ContextWindowMeterProps {
  /** Estimated tokens currently in the active context window. */
  usedTokens: number;
  /** Maximum tokens the model can hold (from CompactionConfig). */
  maxTokens: number;
  /** Token count at which compaction will trigger. */
  thresholdTokens: number;
  /** Human-readable strategy: "aggressive", "balanced", or "conservative". */
  strategy: string;
  /** Optional callback to cycle or update compaction strategy inline. */
  onStrategyChange?: (nextStrategy: string) => void;
  /** Optional 1-click surgical sub-transcript tool-result clearing action (`CONTEXT-SURGERY-03`). */
  onClearToolOutputs?: () => void;
  /** Cumulative BPE tokens reclaimed via surgical tool-result clearing in this session. */
  reclaimedToolTokens?: number;
  /** Empirical prompt-cache read tokens reported by the active turn/session. */
  cacheReadTokens?: number;
  /** Empirical prompt-cache hit percentage (0..100) if pre-computed upstream. */
  cacheHitPct?: number;
  /** Empirical per-segment BPE token breakdown when available from the harness. */
  sliceBreakdown?: ContextSliceBreakdown;
  /** Controlled active slice filter for inspecting a specific context segment. */
  activeSliceFilter?: ContextSliceKey | null;
  /** Callback invoked when the operator clicks a context slice pill. */
  onSliceSelect?: (slice: ContextSliceKey | null) => void;
}

type Zone = 'safe' | 'warn' | 'danger';

function getZone(pct: number): Zone {
  if (pct >= 90) return 'danger';
  if (pct >= 70) return 'warn';
  return 'safe';
}

const ZONE_FILL: Record<Zone, string> = {
  safe:   'bg-[oklch(0.7_0.15_140)]',
  warn:   'bg-[oklch(0.75_0.15_60)]',
  danger: 'bg-[oklch(0.65_0.2_25)]',
};

const ZONE_TEXT: Record<Zone, string> = {
  safe:   'text-[oklch(0.7_0.15_140)]',
  warn:   'text-[oklch(0.75_0.15_60)]',
  danger: 'text-[oklch(0.65_0.2_25)]',
};

/** Color-coded token usage progress bar for the ChatExecutionRail. */
export function ContextWindowMeter({
  usedTokens,
  maxTokens,
  thresholdTokens,
  strategy,
  onStrategyChange,
  onClearToolOutputs,
  reclaimedToolTokens,
  cacheReadTokens,
  cacheHitPct,
  sliceBreakdown,
  activeSliceFilter,
  onSliceSelect,
}: ContextWindowMeterProps) {
  const [internalSlice, setInternalSlice] = React.useState<ContextSliceKey | null>(null);
  const selectedSlice = activeSliceFilter !== undefined ? activeSliceFilter : internalSlice;

  const handleSliceToggle = (key: ContextSliceKey) => {
    const next = selectedSlice === key ? null : key;
    if (activeSliceFilter === undefined) {
      setInternalSlice(next);
    }
    onSliceSelect?.(next);
  };

  const safeMax = Math.max(1, maxTokens);
  const safeUsed = Math.max(0, usedTokens);
  const clampedNow = Math.min(safeMax, safeUsed);
  const pct = Math.min(100, maxTokens <= 0 ? 0 : Math.round((safeUsed / maxTokens) * 100));
  const thresholdPct = maxTokens <= 0 ? 80 : Math.round((thresholdTokens / maxTokens) * 100);
  const zone = getZone(pct);
  const strategies = ['balanced', 'aggressive', 'conservative'];

  // Empirical prompt-cache hit percentage (never fake 86%/92% when 0 tokens are cached/used)
  const kvCacheHitPct = (() => {
    if (typeof cacheHitPct === 'number' && Number.isFinite(cacheHitPct)) {
      return Math.max(0, Math.min(100, Math.round(cacheHitPct)));
    }
    if (typeof cacheReadTokens === 'number' && Number.isFinite(cacheReadTokens) && safeUsed > 0) {
      const safeCache = Math.max(0, cacheReadTokens);
      return Math.min(100, Math.round((safeCache / (safeUsed + safeCache)) * 100));
    }
    if (reclaimedToolTokens && reclaimedToolTokens > 0 && safeUsed + reclaimedToolTokens > 0) {
      return Math.min(100, Math.round((reclaimedToolTokens / (safeUsed + reclaimedToolTokens)) * 100));
    }
    return 0;
  })();

  // Empirical or threshold/reclaim-bounded slice breakdown
  const { sysSliceTokens, kbSliceTokens, astSliceTokens, histSliceTokens } = (() => {
    if (safeUsed === 0) {
      return { sysSliceTokens: 0, kbSliceTokens: 0, astSliceTokens: 0, histSliceTokens: 0 };
    }
    if (sliceBreakdown) {
      return {
        sysSliceTokens: Math.max(0, Math.round(sliceBreakdown.systemTokens)),
        kbSliceTokens: Math.max(0, Math.round(sliceBreakdown.kbTokens)),
        astSliceTokens: Math.max(0, Math.round(sliceBreakdown.astTokens)),
        histSliceTokens: Math.max(0, Math.round(sliceBreakdown.historyTokens)),
      };
    }
    const reserveSpan = maxTokens > thresholdTokens && thresholdTokens > 0 ? maxTokens - thresholdTokens : 0;
    const sys =
      reserveSpan > 0 && maxTokens > 0
        ? Math.min(safeUsed, Math.round((reserveSpan / maxTokens) * safeUsed))
        : 0;
    const afterSys = Math.max(0, safeUsed - sys);
    const kb = Math.min(afterSys, Math.max(0, Math.round(cacheReadTokens ?? 0)));
    const afterKb = Math.max(0, afterSys - kb);
    const ast = Math.min(afterKb, Math.max(0, Math.round(reclaimedToolTokens ?? 0)));
    const hist = Math.max(0, afterKb - ast);
    return { sysSliceTokens: sys, kbSliceTokens: kb, astSliceTokens: ast, histSliceTokens: hist };
  })();

  const sliceSpecs: Array<{
    key: ContextSliceKey;
    shortLabel: string;
    fullLabel: string;
    tokens: number;
    fill: string;
  }> = [
    { key: 'system', shortLabel: 'Sys', fullLabel: 'Instructions & Reserve', tokens: sysSliceTokens, fill: 'var(--color-accent-secondary)' },
    { key: 'kb', shortLabel: 'KB', fullLabel: 'Pinned Files & Cache', tokens: kbSliceTokens, fill: 'rgb(var(--brass))' },
    { key: 'ast', shortLabel: 'AST', fullLabel: 'Code & Tool AST', tokens: astSliceTokens, fill: 'var(--color-status-pass)' },
    { key: 'history', shortLabel: 'Hist', fullLabel: 'Turn History', tokens: histSliceTokens, fill: 'var(--color-status-info)' },
  ];
  const totalSliceTokens = sliceSpecs.reduce((acc, s) => acc + s.tokens, 0);
  const activeSpec = selectedSlice ? sliceSpecs.find((s) => s.key === selectedSlice) ?? null : null;

  const strategyLabels: Record<string, string> = {
    balanced: 'Balanced',
    aggressive: 'Compact',
    conservative: 'Keep more',
  };

  return (
    <div className="flex flex-col gap-1.5 px-2 py-1">
      <div
        className="flex flex-col gap-1"
        role="meter"
        aria-valuenow={clampedNow}
        aria-valuemin={0}
        aria-valuemax={safeMax}
        aria-label={`Context window: ${pct}% full`}
      >
        {/* Label row */}
        <div className="flex items-center justify-between gap-2 whitespace-nowrap">
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] uppercase tracking-[0.08em] text-text-muted">Context</span>
            <span
              data-testid="context-meter-kv-cache"
              title="Empirical prompt cache / reclaimed token rate"
              className="rounded border border-border-subtle bg-overlay-subtle px-1 py-0.2 font-mono text-[11px] text-brass"
            >
              {kvCacheHitPct}% cached
            </span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="font-mono text-[11px] tabular-nums text-text-muted">
              {usedTokens.toLocaleString()} / {maxTokens.toLocaleString()}
            </span>
            <span className={`font-mono text-[11px] tabular-nums ${ZONE_TEXT[zone]}`}>
              {pct}%
            </span>
          </div>
        </div>

        {/* Progress bar track */}
        <div className="relative h-1.5 w-full overflow-hidden rounded-full bg-overlay-subtle">
          {/* Fill */}
          <div
            data-zone={zone}
            className={`absolute inset-y-0 left-0 rounded-full transition-[width] duration-500 ${ZONE_FILL[zone]}`}
            style={{ width: `${pct}%` }}
          />
          {/* Threshold marker */}
          <div
            className="absolute inset-y-0 w-px bg-text-primary/35"
            style={{ left: `${thresholdPct}%` }}
            title={`Compaction triggers at ${thresholdTokens.toLocaleString()} tokens`}
          />
        </div>

        {/* Empirical SVG Stacked Context Memory Breakdown Chart */}
        {(() => {
          const fillRatio = maxTokens > 0 && safeUsed > 0 ? Math.min(1, safeUsed / maxTokens) : 0;
          const nonZeroCount = sliceSpecs.filter((s) => s.tokens > 0).length;
          const gapTotal = Math.max(0, (nonZeroCount - 1) * 2);
          const usableW =
            totalSliceTokens > 0 && fillRatio > 0
              ? Math.max(nonZeroCount * 4, Math.round(fillRatio * (240 - gapTotal)))
              : 0;
          let cursorX = 0;
          const rects = sliceSpecs.map((s) => {
            if (s.tokens <= 0 || totalSliceTokens <= 0 || usableW <= 0) {
              return { ...s, x: cursorX, w: 0 };
            }
            const w = Math.max(4, Math.round((s.tokens / totalSliceTokens) * usableW));
            const x = cursorX;
            cursorX += w + 2;
            return { ...s, x, w };
          });
          return (
            <svg
              data-testid="context-meter-bpe-slices"
              viewBox="0 0 240 10"
              className="h-2 w-full overflow-visible"
              aria-hidden="true"
            >
              <title>{`Context Slices — Instructions: ${sysSliceTokens.toLocaleString()} · Pinned Files: ${kbSliceTokens.toLocaleString()} · Code: ${astSliceTokens.toLocaleString()} · History: ${histSliceTokens.toLocaleString()}`}</title>
              <rect x="0" y="2" width="240" height="6" rx="1.5" fill="var(--color-overlay-subtle)" />
              {rects.map((r) =>
                r.w > 0 ? (
                  <rect
                    key={r.key}
                    x={r.x}
                    y="2"
                    width={r.w}
                    height="6"
                    rx="1.5"
                    style={{ fill: r.fill }}
                    fillOpacity={selectedSlice === null ? 0.88 : selectedSlice === r.key ? 1 : 0.3}
                  />
                ) : null,
              )}
              <line
                x1={Math.round((thresholdPct / 100) * 240)}
                y1="0"
                x2={Math.round((thresholdPct / 100) * 240)}
                y2="10"
                stroke="var(--color-text-muted)"
                strokeWidth="1.2"
              />
            </svg>
          );
        })()}
      </div>

      {/* Interactive slice filter pills (sibling of role="meter" so axe nested-interactive stays 0) */}
      <div className="flex flex-wrap items-center gap-1" role="group" aria-label="Context token slices">
        {sliceSpecs.map((s) => {
          const isSelected = selectedSlice === s.key;
          return (
            <button
              key={s.key}
              type="button"
              data-testid={`context-meter-slice-${s.key}`}
              aria-pressed={isSelected}
              onClick={() => handleSliceToggle(s.key)}
              className={`inline-flex items-center gap-1 rounded border px-1.5 py-0.2 font-mono text-[11px] transition-colors ${
                isSelected
                  ? 'border-brass/60 bg-brass/20 text-brass'
                  : 'border-border-subtle bg-overlay-subtle text-text-muted hover:text-text-secondary'
              }`}
            >
              <span>{s.shortLabel}</span>
              <span className="tabular-nums">{s.tokens.toLocaleString()}</span>
            </button>
          );
        })}
      </div>

      {activeSpec && (
        <div
          data-testid="context-meter-slice-inspector"
          className="flex items-center justify-between gap-2 rounded border border-brass/35 bg-brass/10 px-2 py-0.5 font-mono text-[11px] text-text-secondary"
        >
          <span className="truncate text-brass">{activeSpec.fullLabel}</span>
          <span className="shrink-0 tabular-nums">
            {activeSpec.tokens.toLocaleString()} tok (
            {totalSliceTokens > 0 ? Math.round((activeSpec.tokens / totalSliceTokens) * 100) : 0}%)
          </span>
        </div>
      )}

      {/* Strategy label / interactive cycle (sibling of role="meter" so axe nested-interactive is 0) */}
      <div className="flex flex-wrap items-center justify-between gap-1">
        {onStrategyChange ? (
          <div className="flex items-center gap-1 min-w-0">
            <div className="inline-flex rounded border border-border-subtle bg-overlay-subtle p-0.5" role="group" aria-label="Select compaction strategy">
              {strategies.map((s) => (
                <button
                  key={s}
                  type="button"
                  data-testid={`context-meter-strategy-${s}`}
                  aria-pressed={strategy === s}
                  onClick={() => onStrategyChange(s)}
                  className={`whitespace-nowrap rounded-xs px-1 py-0.2 font-mono text-[11px] transition ${
                    strategy === s
                      ? 'bg-brass/20 text-brass'
                      : 'text-text-muted hover:text-text-secondary'
                  }`}
                >
                  {strategyLabels[s] ?? s}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <span className="text-[11px] text-text-muted">{strategy}</span>
        )}
        <div className="flex items-center gap-1">
          {onClearToolOutputs && (
            <button
              type="button"
              data-testid="context-meter-clear-tools-btn"
              title="Surgically clear stale intermediate tool outputs while keeping user/assistant reasoning and recent tool outputs"
              onClick={onClearToolOutputs}
              className="whitespace-nowrap rounded border border-brass/30 bg-brass/10 px-1 py-0.2 font-mono text-[11px] text-brass transition hover:bg-brass/20"
            >
              Clear Tools
              {reclaimedToolTokens && reclaimedToolTokens > 0
                ? ` (-${reclaimedToolTokens.toLocaleString()})`
                : ''}
            </button>
          )}
          <span className="font-mono text-[11px] text-text-muted">
            @{thresholdPct}%
          </span>
        </div>
      </div>
    </div>
  );
}
