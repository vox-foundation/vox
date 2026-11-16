import React, { useMemo, useState } from 'react';
import {
  createSortedRowModel,
  rowSortingFeature,
  sortFn_alphanumeric,
  tableFeatures,
  useTable,
  type ColumnDef as TanStackColumnDef,
  type SortingState,
} from '@tanstack/react-table';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { stripAnsi } from './sgrToSpans';
import { TERMINAL_MONO_STYLE } from './terminalTheme';

const MAX_STRUCTURED_CHARS = 1_048_576;
const MAX_KEY_SCAN_ROWS = 100;
const MAX_DISPLAY_ROWS = 200;
const MAX_COLUMNS = 12;

const LEDGER_TABLE_FEATURES = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  sortFns: { alphanumeric: sortFn_alphanumeric },
});

interface LedgerColumnSpec {
  key: string;
  header: string;
}

export interface JsonLedgerTableProps {
  data: Record<string, unknown>[] | Record<string, unknown>;
}

export function tryParseStructuredJson(
  raw: string,
): Record<string, unknown>[] | Record<string, unknown> | null {
  if (!raw || raw.length > MAX_STRUCTURED_CHARS) return null;
  const cleaned = stripAnsi(raw).trim();
  if (!cleaned || cleaned.length > MAX_STRUCTURED_CHARS) return null;
  const first = cleaned[0];
  const last = cleaned[cleaned.length - 1];
  if (!((first === '[' && last === ']') || (first === '{' && last === '}'))) return null;
  try {
    const parsed: unknown = JSON.parse(cleaned);
    if (Array.isArray(parsed)) {
      if (parsed.length === 0) return [];
      return parsed.map((entry, idx) =>
        entry !== null && typeof entry === 'object' && !Array.isArray(entry)
          ? (entry as Record<string, unknown>)
          : { index: idx, value: entry },
      );
    }
    if (parsed !== null && typeof parsed === 'object') {
      return parsed as Record<string, unknown>;
    }
    return null;
  } catch {
    return null;
  }
}

function serializeCellScalar(val: unknown): string {
  if (val === null || val === undefined) return '';
  if (typeof val === 'object') {
    try { return JSON.stringify(val); } catch { return String(val); }
  }
  return String(val);
}

function escapeCsvCell(raw: string): string {
  return raw.includes(',') || raw.includes('"') || raw.includes('\n') || raw.includes('\r')
    ? `"${raw.replace(/"/g, '""')}"`
    : raw;
}

function formatCsv(columns: LedgerColumnSpec[], rows: Record<string, unknown>[]): string {
  const headerLine = columns.map((c) => escapeCsvCell(c.key)).join(',');
  const bodyLines = rows.map((row) =>
    columns.map((c) => escapeCsvCell(serializeCellScalar(row[c.key]))).join(','),
  );
  return [headerLine, ...bodyLines].join('\n');
}

export function JsonLedgerTable({ data }: JsonLedgerTableProps) {
  const [sorting, setSorting] = useState<SortingState>([]);
  const [filterQuery, setFilterQuery] = useState('');
  const [expandedCells, setExpandedCells] = useState<Set<string>>(() => new Set());
  const [copiedFormat, setCopiedFormat] = useState<'csv' | 'json' | null>(null);

  const { columns, normalizedRows, totalSourceRows, totalSourceCols } = useMemo(() => {
    if (Array.isArray(data)) {
      const capped = data.slice(0, MAX_DISPLAY_ROWS);
      const seenKeys = new Set<string>();
      for (const row of capped.slice(0, MAX_KEY_SCAN_ROWS)) {
        for (const k of Object.keys(row)) seenKeys.add(k);
      }
      const cols = Array.from(seenKeys).slice(0, MAX_COLUMNS).map((k) => ({ key: k, header: k }));
      return { columns: cols, normalizedRows: capped, totalSourceRows: data.length, totalSourceCols: seenKeys.size };
    }
    const entries = Object.entries(data);
    const rows = entries.slice(0, MAX_DISPLAY_ROWS).map(([key, value]) => ({ key, value }));
    const cols = [{ key: 'key', header: 'Field' }, { key: 'value', header: 'Value' }];
    return { columns: cols, normalizedRows: rows, totalSourceRows: entries.length, totalSourceCols: 2 };
  }, [data]);

  const filteredRows = useMemo(() => {
    const q = filterQuery.trim().toLowerCase();
    if (!q) return normalizedRows;
    return normalizedRows.filter((row) =>
      columns.some((col) => serializeCellScalar(row[col.key]).toLowerCase().includes(q)),
    );
  }, [normalizedRows, columns, filterQuery]);

  const tanstackColumns = useMemo<
    TanStackColumnDef<typeof LEDGER_TABLE_FEATURES, Record<string, unknown>>[]
  >(
    () =>
      columns.map((col) => ({
        id: col.key,
        header: col.header,
        enableSorting: true,
        sortingFn: 'alphanumeric',
        accessorFn: (row) => serializeCellScalar(row[col.key]),
      })),
    [columns],
  );

  const getRowId = (_row: Record<string, unknown>, idx: number) => String(idx);
  const table = useTable({
    features: LEDGER_TABLE_FEATURES,
    data: filteredRows,
    columns: tanstackColumns,
    getRowId,
    state: { sorting },
    onSortingChange: setSorting,
  });

  const sortedRows = table.getRowModel().rows;

  const toggleCell = (cellKey: string) => {
    setExpandedCells((prev) => {
      const next = new Set(prev);
      if (next.has(cellKey)) next.delete(cellKey);
      else next.add(cellKey);
      return next;
    });
  };

  const copyPayload = async (kind: 'csv' | 'json') => {
    const rowsForExport = sortedRows.map((r) => r.original);
    const text =
      kind === 'csv'
        ? formatCsv(columns, rowsForExport)
        : JSON.stringify(
            Array.isArray(data)
              ? rowsForExport
              : Object.fromEntries(rowsForExport.map((r) => [String(r.key ?? ''), r.value])),
            null,
            2,
          );
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      setCopiedFormat(kind);
    }
  };

  const copyBtnCls =
    'h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold text-text-secondary hover:text-text-primary';

  return (
    <div data-testid="term-ledger-table" data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="flex flex-col rounded-md border border-border-subtle bg-bg-surface">
      <div className="flex min-h-8 flex-wrap items-center justify-between gap-2 border-b border-border-subtle bg-bg-elevated px-2 py-1">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <Icon.search className="size-4 shrink-0 text-text-muted" aria-hidden="true" />
          <input
            type="search" data-slot="inline-search-input" data-terminal-mono="true" data-testid="term-ledger-filter" value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Filter rows..." aria-label="Filter table rows"
            className="h-6 min-w-0 flex-1 bg-transparent text-xs font-normal text-text-primary placeholder:text-text-muted outline-none"
          />
          {filterQuery.length > 0 && (
            <Button size="xs" variant="ghost" aria-label="Clear filter" onClick={() => setFilterQuery('')} className="h-6 px-2 py-1 text-[11px] font-semibold">
              Clear Filter
            </Button>
          )}
        </div>
        <div className="flex items-center gap-2">
          <span className="rounded border border-border-subtle bg-bg-surface px-2 py-1 text-[11px] leading-[1.25] font-semibold text-text-muted">
            {`${filteredRows.length} of ${totalSourceRows} rows`}
          </span>
          {copiedFormat && <span role="status" className="text-[11px] font-semibold text-accent">Copied</span>}
          <Button size="xs" variant="ghost" onClick={() => void copyPayload('csv')} className={copyBtnCls}>Copy CSV</Button>
          <Button size="xs" variant="ghost" onClick={() => void copyPayload('json')} className={copyBtnCls}>Copy JSON</Button>
        </div>
      </div>

      {totalSourceRows === 0 ? (
        <div data-testid="term-ledger-empty" className="flex flex-col gap-1 p-4 text-center text-xs font-normal text-text-muted">
          <div className="font-semibold text-text-secondary">Empty JSON collection</div>
          <div>
            {Array.isArray(data)
              ? 'Command returned 0 records ([]). Switch to JSON view to inspect raw output.'
              : 'Command returned an empty object ({}). Switch to JSON view to inspect raw output.'}
          </div>
        </div>
      ) : (
      <div className="max-h-80 overflow-auto">
        <table role="grid" className="w-full border-collapse text-left">
          <thead className="sticky top-0 z-10 border-b border-border-subtle bg-bg-elevated">
            <tr>
              {columns.map((col) => {
                const tsCol = table.getColumn(col.key);
                const sortDir = tsCol?.getIsSorted();
                const ariaSort: 'ascending' | 'descending' | 'none' =
                  sortDir === 'asc' ? 'ascending' : sortDir === 'desc' ? 'descending' : 'none';
                return (
                  <th key={col.key} aria-sort={ariaSort} className="px-2 py-1 text-[11px] leading-[1.25] font-semibold uppercase tracking-wide text-text-secondary">
                    <button
                      type="button" aria-label={`${col.header} sort column`}
                      onClick={tsCol?.getToggleSortingHandler()}
                      className="inline-flex items-center gap-1 text-left uppercase transition-colors hover:text-text-primary"
                    >
                      <span>{col.header}</span>
                      {sortDir === 'asc' ? <Icon.arrowUp className="size-3 shrink-0 text-accent" aria-hidden="true" />
                        : sortDir === 'desc' ? <Icon.arrowDown className="size-3 shrink-0 text-accent" aria-hidden="true" />
                        : <Icon.arrowUpDown className="size-3 shrink-0 text-text-muted" aria-hidden="true" />}
                    </button>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {sortedRows.length === 0 ? (
              <tr>
                <td colSpan={Math.max(columns.length, 1)} className="p-4 text-center text-xs font-normal text-text-muted">
                  <div className="font-semibold text-text-secondary">No matching rows</div>
                  <div>{`0 rows match "${filterQuery.trim()}" - click Clear Filter to restore all ${normalizedRows.length} rows.`}</div>
                </td>
              </tr>
            ) : (
              sortedRows.map((tsRow) => {
                const row = tsRow.original;
                return (
                  <tr key={tsRow.id} className="border-b border-border-subtle bg-bg-base transition-colors last:border-0 hover:bg-overlay-subtle">
                    {columns.map((col) => {
                      const val = row[col.key];
                      const cellId = `${tsRow.id}:${col.key}`;
                      const isNested = val !== null && typeof val === 'object';
                      const isExpanded = expandedCells.has(cellId);
                      return (
                        <td key={col.key} className={`px-2 py-1 align-top text-xs leading-[1.45] font-normal text-text-primary ${typeof val === 'number' ? 'tabular-nums text-right' : ''}`}>
                          {isNested ? (
                            <div className="flex flex-col items-start gap-1">
                              <button
                                type="button" aria-expanded={isExpanded} onClick={() => toggleCell(cellId)}
                                className="rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-accent hover:border-border-strong"
                              >
                                {Array.isArray(val) ? `[${val.length}]` : '{...}'}
                              </button>
                              {isExpanded && (
                                <pre data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="max-w-[320px] whitespace-pre-wrap break-words rounded border border-border-subtle bg-bg-surface p-2 text-[11px] leading-[1.45] text-text-primary">
                                  {JSON.stringify(val, null, 2)}
                                </pre>
                              )}
                            </div>
                          ) : (
                            <span title={val == null ? '' : String(val)} className="inline-block max-w-[280px] truncate">
                              {val === null ? 'null' : val === undefined ? '' : String(val)}
                            </span>
                          )}
                        </td>
                      );
                    })}
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
      )}

      {(totalSourceRows > MAX_DISPLAY_ROWS || totalSourceCols > MAX_COLUMNS) && (
        <div className="border-t border-border-subtle bg-bg-elevated px-3 py-1 text-[11px] font-semibold text-text-muted">
          {totalSourceRows > MAX_DISPLAY_ROWS && totalSourceCols > MAX_COLUMNS
            ? `Showing first ${MAX_DISPLAY_ROWS} of ${totalSourceRows} rows and ${MAX_COLUMNS} of ${totalSourceCols} columns`
            : totalSourceRows > MAX_DISPLAY_ROWS
              ? `Showing first ${MAX_DISPLAY_ROWS} of ${totalSourceRows} rows`
              : `Showing first ${MAX_COLUMNS} of ${totalSourceCols} columns`}
        </div>
      )}
    </div>
  );
}
