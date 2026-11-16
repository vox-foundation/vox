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
import { ArrowDown, ArrowUp, ArrowUpDown, ChevronDown, ChevronRight } from 'lucide-react';
import { cn } from '../../lib/cn';
import { Button } from './Button';

export interface ColumnDef<T> {
  key: string;
  header: string;
  width?: number;
  sortable?: boolean;
  render?: (row: T) => React.ReactNode;
}

export interface DataTableProps<T> {
  rows: T[];
  columns: ColumnDef<T>[];
  groupBy?: (row: T) => string;
  selectable?: boolean;
  onRowAction?: (id: string, action: string) => void;
  emptyState?: React.ReactNode;
  loading?: boolean;
  getRowId: (row: T) => string;
  density?: 'compact' | 'default' | 'comfortable';
}

const DENSITY_CLASS = {
  compact: 'px-2 py-1 text-[11px]',
  default: 'px-4 py-2 text-sm',
  comfortable: 'px-6 py-4 text-base',
};

const DATA_TABLE_FEATURES = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  sortFns: { alphanumeric: sortFn_alphanumeric },
});

export function DataTable<T>({
  rows,
  columns,
  groupBy,
  selectable = false,
  onRowAction,
  emptyState,
  loading = false,
  getRowId,
  density = 'default',
}: DataTableProps<T>) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());
  const [sorting, setSorting] = useState<SortingState>([]);

  const tanstackColumns = useMemo<TanStackColumnDef<typeof DATA_TABLE_FEATURES, T>[]>(
    () =>
      columns.map((col) => ({
        id: col.key,
        header: col.header,
        enableSorting: Boolean(col.sortable),
        accessorFn: (row: T) => {
          const record = row as Record<string, unknown>;
          const val = record[col.key];
          return val == null ? '' : String(val);
        },
      })),
    [columns]
  );

  const table = useTable({
    features: DATA_TABLE_FEATURES,
    data: rows,
    columns: tanstackColumns,
    getRowId,
    state: { sorting },
    onSortingChange: setSorting,
  });

  if (loading) {
    return (
      <div data-slot="data-table-skeleton" className="flex w-full flex-col gap-2 py-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="h-10 w-full animate-pulse rounded-lg border border-border-subtle bg-overlay-subtle" />
        ))}
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div data-slot="data-table-empty" className="w-full py-6">
        {emptyState || <div className="text-center text-sm text-text-muted">No data available</div>}
      </div>
    );
  }

  const toggleGroup = (group: string) => {
    setCollapsedGroups((curr) => {
      const next = new Set(curr);
      if (next.has(group)) next.delete(group);
      else next.add(group);
      return next;
    });
  };

  const toggleSelectAll = () => {
    setSelectedIds((curr) => {
      if (curr.size === rows.length) return new Set();
      return new Set(rows.map(getRowId));
    });
  };

  const toggleSelectRow = (id: string) => {
    setSelectedIds((curr) => {
      const next = new Set(curr);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const sortedRows = table.getRowModel().rows.map((r) => r.original);
  const grouped: Record<string, T[]> = {};
  if (groupBy) {
    sortedRows.forEach((r) => {
      const key = groupBy(r);
      (grouped[key] ??= []).push(r);
    });
  } else {
    grouped[''] = sortedRows;
  }

  return (
    <div
      data-slot="data-table"
      className="w-full overflow-x-auto rounded-xl border border-border-subtle bg-bg-surface/80 backdrop-blur-xl"
    >
      {selectable && selectedIds.size > 0 && (
        <div className="flex items-center justify-between border-b border-border-subtle bg-brass/10 px-4 py-2 text-xs text-brass">
          <span>{selectedIds.size} rows selected</span>
          <div className="flex items-center gap-2">
            <Button size="xs" variant="primary" onClick={() => onRowAction?.(Array.from(selectedIds).join(','), 'bulk-pause')}>
              Pause
            </Button>
            <Button size="xs" variant="danger" onClick={() => onRowAction?.(Array.from(selectedIds).join(','), 'bulk-cancel')}>
              Cancel
            </Button>
          </div>
        </div>
      )}
      <table className="w-full border-collapse text-left">
        <thead>
          <tr className="border-b border-border-subtle bg-overlay-subtle">
            {selectable && (
              <th className={cn('w-10', DENSITY_CLASS[density])}>
                <input
                  type="checkbox"
                  checked={selectedIds.size === rows.length && rows.length > 0}
                  onChange={toggleSelectAll}
                  className="rounded-sm border-border-subtle bg-bg-base text-brass focus:ring-brass/40"
                  aria-label="Select all rows"
                />
              </th>
            )}
            {columns.map((col) => {
              const tsCol = table.getColumn(col.key);
              const sortDir = tsCol?.getIsSorted();
              const ariaSort = col.sortable
                ? sortDir === 'asc'
                  ? 'ascending'
                  : sortDir === 'desc'
                    ? 'descending'
                    : 'none'
                : undefined;

              return (
                <th
                  key={col.key}
                  aria-sort={ariaSort}
                  className={cn('text-[10px] font-semibold tracking-wide text-text-muted uppercase', DENSITY_CLASS[density])}
                  style={{ width: col.width }}
                >
                  {col.sortable && tsCol ? (
                    <button
                      type="button"
                      data-slot="data-table-sort"
                      onClick={tsCol.getToggleSortingHandler()}
                      className="inline-flex items-center gap-1 text-left uppercase tracking-wide transition-colors hover:text-text-primary"
                    >
                      <span>{col.header}</span>
                      {sortDir === 'asc' ? (
                        <ArrowUp className="size-3 shrink-0" strokeWidth={1.75} aria-hidden="true" />
                      ) : sortDir === 'desc' ? (
                        <ArrowDown className="size-3 shrink-0" strokeWidth={1.75} aria-hidden="true" />
                      ) : (
                        <ArrowUpDown className="size-3 shrink-0 opacity-60" strokeWidth={1.5} aria-hidden="true" />
                      )}
                    </button>
                  ) : (
                    col.header
                  )}
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {Object.entries(grouped).map(([groupName, groupRows]) => {
            const isCollapsed = collapsedGroups.has(groupName);
            return (
              <React.Fragment key={groupName}>
                {groupBy && (
                  <tr className="border-b border-border-subtle bg-overlay-subtle">
                    <td colSpan={columns.length + (selectable ? 1 : 0)} className="px-3 py-1.5">
                      <button
                        type="button"
                        data-slot="data-table-group-toggle"
                        onClick={() => toggleGroup(groupName)}
                        className="flex items-center gap-1.5 font-mono text-[10px] tracking-widest text-text-muted uppercase transition-colors hover:text-text-secondary"
                        aria-expanded={!isCollapsed}
                      >
                        {isCollapsed ? (
                          <ChevronRight className="size-3.5 shrink-0" strokeWidth={1.75} aria-hidden="true" />
                        ) : (
                          <ChevronDown className="size-3.5 shrink-0" strokeWidth={1.75} aria-hidden="true" />
                        )}
                        <span>
                          {groupName} ({groupRows.length})
                        </span>
                      </button>
                    </td>
                  </tr>
                )}
                {!isCollapsed &&
                  groupRows.map((row) => {
                    const id = getRowId(row);
                    const isSelected = selectedIds.has(id);
                    const record = row as Record<string, React.ReactNode>;
                    return (
                      <tr
                        key={id}
                        className={cn(
                          'border-b border-border-subtle transition-colors last:border-0 hover:bg-overlay-subtle',
                          isSelected && 'bg-brass/5'
                        )}
                      >
                        {selectable && (
                          <td className={cn('w-10', DENSITY_CLASS[density])}>
                            <input
                              type="checkbox"
                              checked={isSelected}
                              onChange={() => toggleSelectRow(id)}
                              className="rounded-sm border-brass/40 bg-bg-base text-brass focus:ring-brass/40"
                              aria-label={`Select row ${id}`}
                            />
                          </td>
                        )}
                        {columns.map((col) => (
                          <td key={col.key} className={DENSITY_CLASS[density]}>
                            {col.render ? col.render(row) : record[col.key]}
                          </td>
                        ))}
                      </tr>
                    );
                  })}
              </React.Fragment>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
