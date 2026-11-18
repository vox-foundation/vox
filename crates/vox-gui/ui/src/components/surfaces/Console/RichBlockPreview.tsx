import React from 'react';
import type { TermBlockDto } from '../../../transport';

export interface VoxFrameSpec {
  kind: 'frame';
  title?: string;
  columns: string[];
  rows: Array<Array<string | number | boolean | null>>;
}

export interface VoxPlotSpec {
  kind: 'plot';
  title?: string;
  chartType?: 'bar' | 'line';
  points: Array<{ label: string; value: number }>;
}

export type VoxRichPreviewSpec = VoxFrameSpec | VoxPlotSpec;

export type TelemetryBlockLike = TermBlockDto & {
  cpu_ms?: number | null;
  max_rss_kb?: number | null;
};

export function formatBlockRusageBadge(block: TelemetryBlockLike): string | null {
  const parts: string[] = [];
  if (typeof block.cpu_ms === 'number' && Number.isFinite(block.cpu_ms)) {
    parts.push(`${block.cpu_ms}ms cpu`);
  }
  if (typeof block.max_rss_kb === 'number' && Number.isFinite(block.max_rss_kb)) {
    const mb = block.max_rss_kb >= 1024 ? `${(block.max_rss_kb / 1024).toFixed(1)}MiB` : `${block.max_rss_kb}KiB`;
    parts.push(`${mb} rss`);
  }
  return parts.length > 0 ? parts.join(' / ') : null;
}

export function tryParseVoxFrameOrPlot(text: string): VoxRichPreviewSpec | null {
  const trimmed = text.trim();
  if (!trimmed.startsWith('{')) return null;
  try {
    const obj = JSON.parse(trimmed) as Record<string, unknown>;
    const frameObj = (obj.__vox_frame__ ?? (obj.kind === 'frame' ? obj : null)) as Record<string, unknown> | null;
    if (frameObj && Array.isArray(frameObj.columns) && Array.isArray(frameObj.rows)) {
      return {
        kind: 'frame',
        title: typeof frameObj.title === 'string' ? frameObj.title : undefined,
        columns: frameObj.columns.map((c) => String(c)),
        rows: frameObj.rows.slice(0, 50).map((r) => (Array.isArray(r) ? r.map((v) => (v as string | number | boolean | null)) : [])),
      };
    }
    const plotObj = (obj.__vox_plot__ ?? (obj.kind === 'plot' ? obj : null)) as Record<string, unknown> | null;
    if (plotObj && Array.isArray(plotObj.points)) {
      const points = plotObj.points
        .filter((p): p is { label: unknown; value: unknown } => Boolean(p && typeof p === 'object'))
        .map((p) => ({ label: String(p.label ?? ''), value: Number(p.value ?? 0) }))
        .filter((p) => Number.isFinite(p.value));
      if (points.length > 0) {
        return {
          kind: 'plot',
          title: typeof plotObj.title === 'string' ? plotObj.title : undefined,
          chartType: plotObj.chartType === 'line' ? 'line' : 'bar',
          points,
        };
      }
    }
  } catch {
    return null;
  }
  return null;
}

export function RichBlockPreview({ spec }: { spec: VoxRichPreviewSpec }) {
  if (spec.kind === 'frame') {
    return (
      <div
        data-testid="term-std-frame-card"
        className="overflow-x-auto rounded border border-border-subtle bg-bg-elevated p-2 text-xs"
      >
        <div className="mb-1 flex items-center justify-between text-[11px] font-semibold text-accent">
          <span>{spec.title ?? 'std.frame'}</span>
          <span className="text-text-muted">{`${spec.rows.length} rows x ${spec.columns.length} cols`}</span>
        </div>
        <table className="w-full border-collapse text-left font-mono text-[11px]">
          <thead>
            <tr className="border-b border-border-subtle text-text-secondary">
              {spec.columns.map((col, idx) => (
                <th key={idx} className="px-2 py-1 font-semibold">
                  {col}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {spec.rows.map((row, rIdx) => (
              <tr key={rIdx} className="border-b border-border-subtle/50 last:border-b-0">
                {spec.columns.map((_, cIdx) => (
                  <td key={cIdx} className="px-2 py-1 text-text-primary">
                    {row[cIdx] == null ? '—' : String(row[cIdx])}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  const maxVal = Math.max(1, ...spec.points.map((p) => Math.abs(p.value)));
  const width = Math.max(240, spec.points.length * 44 + 24);
  const height = 100;
  return (
    <div
      data-testid="term-std-plot-card"
      className="rounded border border-border-subtle bg-bg-elevated p-2 text-xs"
    >
      <div className="mb-1 flex items-center justify-between text-[11px] font-semibold text-accent">
        <span>{spec.title ?? 'std.plot'}</span>
        <span className="text-text-muted">{`${spec.points.length} pts`}</span>
      </div>
      <svg
        role="img"
        aria-label={spec.title ?? 'std.plot chart'}
        viewBox={`0 0 ${width} ${height}`}
        className="h-24 w-full max-w-full"
      >
        {spec.points.map((pt, i) => {
          const barH = Math.max(4, Math.round((Math.abs(pt.value) / maxVal) * 68));
          const x = 14 + i * 44;
          const y = 76 - barH;
          return (
            <g key={i}>
              <rect x={x} y={y} width={28} height={barH} rx={2} className="fill-accent opacity-85" />
              <text x={x + 14} y={92} textAnchor="middle" className="fill-text-muted text-[9px]">
                {pt.label.slice(0, 6)}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}
