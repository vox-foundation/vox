import React, { useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Icon } from '../../ui/Icons';

export type FrameCellValue = string | number | boolean | null | Record<string, unknown> | unknown[];

export interface FrameChunkPage {
  stream_id?: string;
  columns: string[];
  rows: FrameCellValue[][];
  offset: number;
  limit: number;
  has_more: boolean;
  total_rows?: number | null;
}

export interface VirtualizedFrameChunkTableProps {
  page: FrameChunkPage;
  loadingMore?: boolean;
  onLoadNextChunk?: (nextOffset: number, limit: number) => void;
  viewportHeight?: number;
  rowHeight?: number;
  overscan?: number;
}

const DEFAULT_VIEWPORT_HEIGHT = 320;
const DEFAULT_ROW_HEIGHT = 36;
const DEFAULT_OVERSCAN = 8;

function formatCell(val: FrameCellValue | undefined): string {
  if (val === null || val === undefined) return '-';
  if (typeof val === 'string') return val;
  if (typeof val === 'number' || typeof val === 'boolean') return String(val);
  try {
    return JSON.stringify(val);
  } catch {
    return '[object]';
  }
}

export function VirtualizedFrameChunkTable({
  page,
  loadingMore = false,
  onLoadNextChunk,
  viewportHeight = DEFAULT_VIEWPORT_HEIGHT,
  rowHeight = DEFAULT_ROW_HEIGHT,
  overscan = DEFAULT_OVERSCAN,
}: VirtualizedFrameChunkTableProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const { columns, rows, offset, limit, has_more, total_rows } = page;

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan,
    initialRect: { width: 800, height: viewportHeight },
    observeElementRect: (instance, cb) => {
      const el = instance.scrollElement;
      if (!el) return;
      const emit = () => {
        const rect = el.getBoundingClientRect();
        const width = el.clientWidth || el.offsetWidth || rect.width || 800;
        const height = el.clientHeight || el.offsetHeight || rect.height || viewportHeight;
        cb({ width: Math.round(width), height: Math.round(height) });
      };
      emit();
      if (typeof ResizeObserver === 'undefined') return;
      const ro = new ResizeObserver(() => emit());
      ro.observe(el);
      return () => ro.disconnect();
    },
  });

  const virtualItems = virtualizer.getVirtualItems();
  const colTemplate =
    columns.length > 0 ? `56px repeat(${columns.length}, minmax(110px, 1fr))` : '56px 1fr';

  return (
    <div
      data-testid="virtualized-frame-chunk-table"
      className="rounded-xl border border-border-subtle bg-overlay-subtle p-3 space-y-2"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
        <div className="flex items-center gap-2">
          <Icon.matrix className="size-3.5 text-brass shrink-0" aria-hidden="true" />
          <span className="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-primary">
            DataFrame Stream Preview
          </span>
          {page.stream_id && (
            <span className="rounded bg-black/30 px-1.5 py-0.5 font-mono text-[10px] text-text-muted">
              {page.stream_id}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2 font-mono text-[10px] text-text-muted">
          <span>
            {rows.length} loaded
            {total_rows != null ? ` / ${total_rows} total` : ''}
          </span>
          {has_more && onLoadNextChunk && (
            <button
              type="button"
              data-testid="frame-chunk-load-more"
              disabled={loadingMore}
              onClick={() => onLoadNextChunk(offset + rows.length, limit)}
              className="rounded border border-brass/40 bg-brass/10 px-2 py-0.5 text-[10px] text-brass hover:bg-brass/20 disabled:opacity-50"
            >
              {loadingMore ? 'Streaming…' : 'Load next chunk'}
            </button>
          )}
        </div>
      </div>

      {rows.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border-subtle py-6 text-center font-mono text-[11px] text-text-muted">
          No frame rows available in stream chunk.
        </div>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-border-subtle bg-black/30">
          <div
            role="row"
            style={{ gridTemplateColumns: colTemplate }}
            className="grid border-b border-border-subtle bg-black/50 px-2 py-1.5 font-mono text-[10px] font-semibold uppercase tracking-wider text-text-muted"
          >
            <div>#</div>
            {columns.map((col) => (
              <div key={col} className="truncate pr-2" title={col}>
                {col}
              </div>
            ))}
          </div>

          <div
            ref={scrollRef}
            role="region"
            tabIndex={0}
            aria-label="Virtualized frame chunk rows"
            data-testid="virtualized-frame-viewport"
            style={{ height: Math.min(viewportHeight, Math.max(rowHeight, rows.length * rowHeight)) }}
            className="overflow-y-auto custom-scrollbar"
          >
            <div
              style={{
                height: `${virtualizer.getTotalSize()}px`,
                position: 'relative',
                width: '100%',
              }}
            >
              {virtualItems.map((vItem) => {
                const row = rows[vItem.index];
                if (!row) return null;
                return (
                  <div
                    key={vItem.index}
                    role="row"
                    data-testid="virtualized-frame-row"
                    data-index={vItem.index}
                    style={{
                      position: 'absolute',
                      top: 0,
                      left: 0,
                      width: '100%',
                      height: `${vItem.size}px`,
                      transform: `translateY(${vItem.start}px)`,
                      gridTemplateColumns: colTemplate,
                    }}
                    className="grid items-center border-b border-border-subtle/40 px-2 font-mono text-[11px] text-text-secondary hover:bg-white/[0.03]"
                  >
                    <div className="text-[10px] text-text-muted">{offset + vItem.index}</div>
                    {columns.map((_, colIdx) => {
                      const text = formatCell(row[colIdx]);
                      return (
                        <div key={colIdx} className="truncate pr-2" title={text}>
                          {text}
                        </div>
                      );
                    })}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
