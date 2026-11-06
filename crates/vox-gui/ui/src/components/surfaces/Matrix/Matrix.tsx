import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import { Glass } from '../../ui/Glass';
import { Pill, PHASE_TONE } from '../../ui/Pill';
import { phaseFill, phaseStroke } from '../../../lib/visualTokens';
import { useLabel } from '../../../hooks/useLanguage';
import { MATRIX_POLL_MS } from '../../../config/constants';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import type { Toast } from '../../../types/tauri';

/** One routing-priority axis projected onto the hex grid (mirrors the Rust
 *  `RoutingIntentionDto`). */
interface RoutingIntention {
  id: string;
  parent: string;
  branch: string;
  phase: string;
  conf: number;
  note: string;
}

function AxisWeightRow({
  intention,
  onSelect,
  selected,
}: {
  intention: RoutingIntention;
  onSelect: (id: string) => void;
  selected: boolean;
}) {
  const conf = intention.conf;
  const pct = Math.round(conf * 100);
  const stroke = phaseStroke(intention.phase === 'Active' ? 'Active' : intention.phase);
  const phaseTone = {
    stroke,
    fill: phaseFill(stroke, conf),
    text:
      (PHASE_TONE as Record<string, { text: string }>)[intention.phase]?.text ??
      PHASE_TONE.Active.text,
  };

  return (
    <button
      type="button"
      onClick={() => onSelect(intention.id)}
      aria-pressed={selected}
      aria-label={`${intention.branch} routing axis (${intention.parent}, ${pct}% weight, ${intention.phase})`}
      className={`group relative flex w-full flex-col gap-2 rounded-lg border p-3.5 text-left transition focus:outline-hidden ${
        selected
          ? 'border-brass/60 bg-overlay-medium shadow-sm'
          : 'border-border-subtle bg-overlay-subtle hover:border-border-subtle/80 hover:bg-overlay-hover/50'
      }`}
    >
      <div className="flex w-full items-center justify-between gap-2 whitespace-nowrap">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className="size-2 shrink-0 rounded-full"
            style={{ backgroundColor: phaseTone.stroke }}
            aria-hidden="true"
          />
          <span className={`truncate font-display text-[13px] font-semibold tracking-tight ${phaseTone.text}`}>
            {intention.branch}
          </span>
          <span className="rounded border border-border-subtle bg-bg-base/60 px-1.5 py-0.5 font-mono text-[10px] text-text-muted">
            {intention.phase}
          </span>
        </div>
        <span className="font-mono text-[14px] font-semibold tabular-nums text-text-primary">
          {pct}%
        </span>
      </div>

      {/* Real SVG horizontal bullet & confidence bar (no polygons) */}
      <svg
        viewBox="0 0 240 12"
        className="h-2.5 w-full overflow-visible"
        aria-hidden="true"
      >
        <rect x="0" y="3" width="240" height="6" rx="3" fill="rgba(255,255,255,0.06)" />
        <rect
          x="0"
          y="3"
          width={Math.max(6, Math.round(conf * 240))}
          height="6"
          rx="3"
          fill={phaseTone.stroke}
          fillOpacity="0.85"
        />
        <line
          x1={Math.round(conf * 240)}
          y1="1"
          x2={Math.round(conf * 240)}
          y2="11"
          stroke={phaseTone.stroke}
          strokeWidth="1.5"
          strokeLinecap="round"
        />
      </svg>

      <div className="truncate text-[11px] text-text-muted">{intention.note}</div>
    </button>
  );
}

interface MatrixProps {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
}

export function Matrix({ pushToast, gamifyEnabled = false }: MatrixProps) {
  const embedded = useIsEmbeddedSurface();
  const [intentions, setIntentions] = useState<RoutingIntention[]>([]);
  const [sel, setSel] = useState<string | undefined>(undefined);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const cells = await invoke<RoutingIntention[]>('get_routing_intentions');
      setIntentions(Array.isArray(cells) ? cells : []);
      setSel(prev => (prev && cells.some(c => c.id === prev) ? prev : cells[0]?.id));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Routing policies load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoading(false);
    }
  }, [pushToast]);

  useEffect(() => {
    refresh();
    // Embedded mini-render: one initial fetch only, no repeating poll.
    if (embedded) return;
    const id = setInterval(refresh, MATRIX_POLL_MS);
    return () => clearInterval(id);
  }, [refresh, embedded]);

  const nudge = useCallback(async (axis: RoutingIntention, direction: 'promote' | 'doubt') => {
    setBusy(true);
    try {
      await invoke('nudge_routing_intention', { axis: axis.id, direction });
      void recordGamifyGuiEvent(
        'palette_navigation',
        { axis: axis.id, direction, surface: 'matrix' },
        { enabled: gamifyEnabled },
      );
      pushToast({
        tone: direction === 'promote' ? 'ok' : 'warn',
        title: direction === 'promote' ? 'Axis promoted' : 'Axis doubted',
        body: `${axis.branch} routing weight ${direction === 'promote' ? 'increased' : 'reduced'}.`,
        cmd: `vox config routing ${axis.id} ${direction}`,
        cause: 'backend-ok',
      });
      await refresh();
    } catch (err) {
      pushToast({ tone: 'warn', title: `Routing ${direction} failed`, body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  }, [pushToast, refresh, gamifyEnabled]);

  const active = intentions.find(i => i.id === sel) || intentions[0];
  const routingTitle = useLabel('mat-routing');
  const axisTitle = useLabel('mat-axis');

  if (loading) return (
    <div className="p-8 text-center">
      <Glass className="p-12 inline-block">
        <p className="font-display uppercase tracking-[0.2em] text-text-muted">Loading routing policies…</p>
      </Glass>
    </div>
  );

  if (!active) return (
    <div className="p-8 text-center">
        <Glass className="p-12 inline-block">
            <p className="font-display uppercase tracking-[0.2em] text-text-muted">No routing policies active</p>
        </Glass>
    </div>
  );

  const groups: Record<string, RoutingIntention[]> = {};
  intentions.forEach((i) => { (groups[i.parent] = groups[i.parent] || []).push(i); });

  return (
    <div className="grid grid-cols-12 gap-5 p-5">
      <Glass className="col-span-12 xl:col-span-8 p-5">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="font-display text-[18px] font-semibold tracking-tight text-text-primary">{routingTitle}</h2>
            <p className="mt-0.5 text-[11px] text-text-muted">
              Choose how strongly the router favors quality, speed, and cost on each turn.
            </p>
          </div>
        </div>

        {/* Real SVG Comparative Weight Distribution Chart */}
        <div className="mt-4 rounded-lg border border-border-subtle bg-overlay-subtle/60 p-3">
          <div className="mb-1.5 flex items-center justify-between text-[10px] font-mono uppercase tracking-[0.08em] text-text-muted">
            <span>Relative Priority Distribution</span>
            <span>{intentions.length} active priorities</span>
          </div>
          <svg
            data-testid="matrix-comparative-chart-svg"
            viewBox={`0 0 520 ${Math.max(28, intentions.length * 22)}`}
            className="w-full"
            aria-label="Routing priority weight comparison chart"
          >
            {intentions.map((item, idx) => {
              const y = idx * 22 + 4;
              const barW = Math.max(8, Math.round(item.conf * 340));
              const isSelected = sel === item.id;
              return (
                <g key={item.id}>
                  <text
                    x="0"
                    y={y + 11}
                    fill={isSelected ? '#E5A93C' : '#9BA3B4'}
                    fontSize="10"
                    fontFamily="monospace"
                  >
                    {item.branch.slice(0, 16)}
                  </text>
                  <rect x="115" y={y + 3} width="340" height="8" rx="4" fill="rgba(255,255,255,0.06)" />
                  <rect
                    x="115"
                    y={y + 3}
                    width={barW}
                    height="8"
                    rx="4"
                    fill={isSelected ? '#E5A93C' : '#38BDF8'}
                    fillOpacity={isSelected ? '0.95' : '0.7'}
                  />
                  <text
                    x="465"
                    y={y + 11}
                    fill="#E6EAF2"
                    fontSize="10"
                    fontFamily="monospace"
                  >
                    {Math.round(item.conf * 100)}%
                  </text>
                </g>
              );
            })}
          </svg>
        </div>

        <div className="mt-4 space-y-5">
          {Object.entries(groups).map(([parent, items]) => (
            <div key={parent}>
              <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.14em] text-text-muted">
                <span className="h-px flex-1 bg-linear-to-r from-white/10 to-transparent" />
                <span className="text-text-muted">{parent}</span>
                <span className="h-px flex-1 bg-linear-to-l from-white/10 to-transparent" />
              </div>
              <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
                {items.map(i => (
                  <AxisWeightRow key={i.id} intention={i} onSelect={setSel} selected={sel === i.id} />
                ))}
              </div>
            </div>
          ))}
        </div>
      </Glass>

      <Glass className="col-span-12 xl:col-span-4 p-5">
        <div className="flex items-center justify-between">
          <h3 className="font-display text-[14px] font-semibold tracking-wide text-text-primary">{axisTitle}</h3>
          <Pill phase={active.phase} />
        </div>
        <div className="mt-3 rounded-xl border border-border-subtle bg-overlay-subtle p-4">
          <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-text-muted">{active.parent}</div>
          <div className="mt-1 font-display text-[20px] font-semibold tracking-tight text-text-primary">{active.branch}</div>
          <div className="mt-2 text-[12px] leading-relaxed text-text-muted">{active.note}</div>
          <div className="mt-4">
            <div className="flex items-center justify-between text-[10px] uppercase tracking-[0.14em] text-text-muted">
              <span>Weight</span><span className="font-mono text-text-secondary">{Math.round(active.conf * 100)}%</span>
            </div>
            <div
              className="mt-1.5 h-2 overflow-hidden rounded-full bg-overlay-subtle"
              role="progressbar"
              aria-valuenow={Math.round(active.conf * 100)}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-label={`${active.branch} routing weight`}
            >
              <div className="h-full rounded-full bg-linear-to-r from-violet-400 via-cyan-400 to-emerald-400" style={{ width: `${active.conf * 100}%` }} />
            </div>
          </div>
          <div className="mt-4 flex gap-2">
            <button
              type="button"
              disabled={busy}
              onClick={() => nudge(active, 'promote')}
              className="inline-flex flex-1 items-center justify-center whitespace-nowrap rounded-md border border-emerald-400/30 bg-emerald-400/10 px-3 py-1.5 font-mono text-[11px] font-medium text-emerald-300 transition hover:bg-emerald-400/20 disabled:opacity-40"
            >
              + Increase
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => nudge(active, 'doubt')}
              className="inline-flex flex-1 items-center justify-center whitespace-nowrap rounded-md border border-amber-400/30 bg-amber-400/10 px-3 py-1.5 font-mono text-[11px] font-medium text-amber-300 transition hover:bg-amber-400/20 disabled:opacity-40"
            >
              − Reduce
            </button>
          </div>
        </div>
      </Glass>
    </div>
  );
}
