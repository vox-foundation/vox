import React, { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { defaultRangeExtractor, type Range } from '@tanstack/react-virtual';
import { Glass } from '../../ui/Glass';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { cn } from '../../../lib/cn';
import { useVirtualList } from '../../../hooks/useVirtualList';
import { decideCliActivation } from '../../../lib/capabilityRun';
import { docKey } from '../../../lib/omnibarFacets';
import type { CommandCatalogEntry } from '../../../types/catalog';
import { CapabilityBadge, CapabilityTag } from './CapabilityBadge';
import { isOverdue, localIsoDate, type FlattenedRow, type StatusMixCounts } from './explorerModel';

export interface ExplorerTreeProps {
  rows: readonly FlattenedRow[];
  onToggle: (key: string) => void;
  activeKey: string | null;
  onActivate: (key: string) => void;
  treeId?: string;
  narrow?: boolean;
  onNavigate?: (viewKey: string) => void;
  onOpenForm?: (path: string[]) => void;
  onExplain?: (rowId: string) => void;
  onOpenDoc?: (docKeyStr: string) => void;
  commandCatalog?: readonly CommandCatalogEntry[];
}

type ActionSlot = 'primary' | 'explain' | 'docs';

function capitalizeFirst(s: string): string {
  return s.length > 0 ? s.charAt(0).toUpperCase() + s.slice(1) : s;
}

function DomainRollupBar({ mix }: { mix?: StatusMixCounts }) {
  const total = mix?.total ?? 0;
  if (!mix || total <= 0) return <div aria-hidden="true" className="h-1 w-full rounded-full bg-overlay-subtle" />;
  const pct = (n: number) => `${(n / total) * 100}%`;
  return (
    <div aria-hidden="true" className="flex h-1 w-full overflow-hidden rounded-full bg-overlay-subtle">
      {mix.dedicated > 0 && <span className="h-full bg-(--color-status-pass)" style={{ width: pct(mix.dedicated) }} />}
      {mix.generic_form > 0 && <span className="h-full bg-(--color-status-info)" style={{ width: pct(mix.generic_form) }} />}
      {mix.cli_only > 0 && <span className="h-full bg-(--color-text-secondary)" style={{ width: pct(mix.cli_only) }} />}
      {mix.mcp_only > 0 && <span className="h-full bg-(--color-status-warn)" style={{ width: pct(mix.mcp_only) }} />}
    </div>
  );
}

export function ExplorerTree({
  rows,
  onToggle,
  activeKey,
  onActivate,
  treeId = 'capability-explorer-treegrid',
  narrow = false,
  onNavigate,
  onOpenForm,
  onExplain,
  onOpenDoc,
  commandCatalog,
}: ExplorerTreeProps) {
  const containerRef = useRef<HTMLDivElement>(null!);
  const rowRefs = useRef<Map<string, HTMLDivElement>>(new Map());
  const pendingFocusRef = useRef<{ rowKey: string; actionSlot: ActionSlot | null } | null>(null);

  const foundIdx = activeKey ? rows.findIndex((r) => r.key === activeKey) : -1;
  const activeIndex = rows.length === 0 ? -1 : foundIdx >= 0 ? foundIdx : 0;
  const activeIndexRef = useRef<number>(activeIndex);
  activeIndexRef.current = activeIndex;

  const catalogByPath = useMemo(
    () => (commandCatalog ? new Map(commandCatalog.map((e) => [e.path.join(' '), e])) : null),
    [commandCatalog],
  );

  const estimateSize = useCallback(
    (index: number) => {
      const r = rows[index];
      if (!r) return 36;
      if (r.kind === 'group' || r.kind === 'mcp_group') return 48;
      if (r.kind === 'domain') return 40;
      if (r.kind === 'exemption_strip') return 88;
      return 36;
    },
    [rows],
  );

  const rangeExtractor = useCallback(
    (range: Range) => {
      const indices = defaultRangeExtractor(range);
      const idx = activeIndexRef.current >= 0 ? activeIndexRef.current : activeIndex;
      if (idx >= 0 && idx < range.count && !indices.includes(idx)) {
        indices.push(idx);
        indices.sort((a, b) => a - b);
      }
      return indices;
    },
    [activeIndex],
  );

  const { virtualizer, totalSize, virtualItems } = useVirtualList({
    containerRef,
    count: rows.length,
    estimateSize,
    overscan: 8,
    rangeExtractor,
  });

  const applyPendingFocus = useCallback(() => {
    const pending = pendingFocusRef.current;
    if (!pending) return;
    const rowEl = rowRefs.current.get(pending.rowKey);
    if (!rowEl) return;
    pendingFocusRef.current = null;
    if (pending.actionSlot) {
      const btn = rowEl.querySelector<HTMLButtonElement>(`button[data-action="${pending.actionSlot}"]`);
      if (btn) {
        btn.focus();
        return;
      }
    }
    rowEl.focus();
  }, []);

  useLayoutEffect(() => {
    applyPendingFocus();
  });

  const moveRow = (targetIdx: number, slot: ActionSlot | null) => {
    if (rows.length === 0) return;
    const clamped = Math.max(0, Math.min(rows.length - 1, targetIdx));
    const targetRow = rows[clamped];
    if (!targetRow) return;
    activeIndexRef.current = clamped;
    pendingFocusRef.current = { rowKey: targetRow.key, actionSlot: slot };
    onActivate(targetRow.key);
    virtualizer.scrollToIndex(clamped, { align: 'auto' });
    applyPendingFocus();
  };

  const today = localIsoDate(new Date());

  const getVerbDecision = (v: NonNullable<FlattenedRow['verb']>) => {
    const entry: CommandCatalogEntry | undefined = catalogByPath
      ? catalogByPath.get(v.path.join(' '))
      : {
          path: v.path,
          command: v.cli,
          about: v.summary ?? '',
          aliases: [],
          has_subcommands: false,
          compiled_in: v.catalogForm || v.status !== 'cli_only',
          source_group: v.groupName,
          feature_gate: null,
          tier: 'recommended' as const,
          arguments: [],
        };
    return decideCliActivation({ sideEffect: v.sideEffect }, entry);
  };

  const renderDocsBtn = (docsPath: string | null, capName: string) =>
    docsPath ? (
      <Button
        variant="outline"
        size="sm"
        tabIndex={-1}
        data-action="docs"
        aria-label={`Open docs for ${capName}`}
        icon={!narrow ? <Icon.book className="size-3.5" aria-hidden="true" /> : undefined}
        className="text-text-secondary"
        onClick={() => onOpenDoc?.(docKey(docsPath))}
      >
        {narrow ? <Icon.book className="size-3.5" aria-hidden="true" /> : 'Open docs'}
      </Button>
    ) : null;

  const renderExplainBtn = (targetId: string | null, capName: string) => {
    const disabled = !targetId;
    return (
      <Button
        variant="outline"
        size="sm"
        tabIndex={-1}
        data-action="explain"
        aria-label={`Explain capability ${capName}`}
        aria-disabled={disabled ? 'true' : undefined}
        className={cn('text-text-secondary', disabled && 'opacity-50 cursor-not-allowed')}
        onClick={disabled ? undefined : () => onExplain?.(targetId!)}
      >
        Explain capability
      </Button>
    );
  };

  const renderActions = (row: FlattenedRow) => {
    if (row.kind === 'group' && row.group) {
      const g = row.group;
      const capName = `vox ${g.name}`;
      return (
        <div className="flex items-center gap-2">
          {g.viewKey ? (
            <Button variant="outline" size="sm" tabIndex={-1} data-action="primary" className="text-text-primary" onClick={() => onNavigate?.(g.viewKey!)}>
              Open surface
            </Button>
          ) : (
            <Button variant="outline" size="sm" tabIndex={-1} data-action="primary" className="text-text-primary" onClick={() => onOpenForm?.([g.name])}>
              Open form
            </Button>
          )}
          {renderExplainBtn(g.rowId, capName)}
          {renderDocsBtn(g.docs, capName)}
        </div>
      );
    }
    if (row.kind === 'verb' && row.verb) {
      const v = row.verb;
      const decision = getVerbDecision(v);
      const reason = decision.kind === 'explain' ? capitalizeFirst(decision.reason) : null;
      const reasonId = reason ? `reason-${v.id.replace(/[^a-zA-Z0-9_-]/g, '-')}` : undefined;
      return (
        <div className="flex items-center gap-2">
          {reasonId && <span id={reasonId} className="sr-only">{reason}</span>}
          <Button
            variant="outline"
            size="sm"
            tabIndex={-1}
            data-action="primary"
            aria-disabled={reason ? 'true' : undefined}
            aria-describedby={reasonId}
            className={cn('text-text-primary', reason && 'opacity-50 cursor-not-allowed text-text-muted')}
            onClick={reason ? undefined : () => onOpenForm?.(v.path)}
          >
            Open form
          </Button>
          {renderExplainBtn(v.id, v.cli)}
          {renderDocsBtn(v.docs, v.cli)}
        </div>
      );
    }
    if (row.kind === 'mcp_tool' && row.mcpTool) {
      const t = row.mcpTool;
      return (
        <div className="flex items-center gap-2">
          {renderExplainBtn(t.id, t.mcp)}
          {renderDocsBtn(t.docs, t.mcp)}
        </div>
      );
    }
    return null;
  };

  return (
    <Glass inset={false} className="min-h-0 flex-1 overflow-hidden rounded-xl">
      <div
        id={treeId}
        ref={containerRef}
        role="treegrid"
        aria-label="Capabilities"
        aria-colcount={3}
        aria-rowcount={rows.length + 1}
        className="h-full max-h-[72vh] overflow-y-auto custom-scrollbar"
      >
        <div
          role="row"
          aria-rowindex={1}
          className="sticky top-0 z-10 grid grid-cols-[1fr_176px_304px] items-center border-b border-border-subtle bg-bg-surface px-4 py-2 text-[11px] font-medium uppercase tracking-[0.08em] text-text-muted"
        >
          <div role="columnheader">Capability</div>
          <div role="columnheader">Status</div>
          <div role="columnheader">Actions</div>
        </div>

        <div role="rowgroup" style={{ height: totalSize, position: 'relative' }}>
          {virtualItems.map((vItem) => {
            const row = rows[vItem.index];
            if (!row) return null;
            const isActive = vItem.index === activeIndex;
            const indentPx = (row.level - 1) * 24 + 16;

            return (
              <div
                key={row.key}
                ref={(el) => {
                  if (el) {
                    rowRefs.current.set(row.key, el);
                    virtualizer.measureElement(el);
                  } else {
                    rowRefs.current.delete(row.key);
                  }
                }}
                data-index={vItem.index}
                role="row"
                aria-level={row.level}
                aria-setsize={row.setsize}
                aria-posinset={row.posinset}
                aria-rowindex={row.rowIndex}
                aria-expanded={row.expandable ? Boolean(row.expanded) : undefined}
                tabIndex={isActive ? 0 : -1}
                style={{ position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${vItem.start}px)` }}
                onClick={() => {
                  onActivate(row.key);
                  if (row.expandable) onToggle(row.key);
                }}
                onKeyDown={(e) => {
                  const rowEl = rowRefs.current.get(row.key);
                  const actionBtn = (e.target as HTMLElement).closest<HTMLButtonElement>('button[data-action]');
                  const actionBtns = rowEl ? Array.from(rowEl.querySelectorAll<HTMLButtonElement>('button[data-action]')) : [];
                  const currentBtnIdx = actionBtn ? actionBtns.indexOf(actionBtn) : -1;

                  if (currentBtnIdx >= 0) {
                    if (e.key === 'ArrowRight') {
                      e.preventDefault();
                      e.stopPropagation();
                      const nextBtn = actionBtns[Math.min(actionBtns.length - 1, currentBtnIdx + 1)];
                      nextBtn?.focus();
                    } else if (e.key === 'ArrowLeft') {
                      e.preventDefault();
                      e.stopPropagation();
                      if (currentBtnIdx > 0) {
                        const prevBtn = actionBtns[currentBtnIdx - 1];
                        prevBtn?.focus();
                      } else {
                        rowEl?.focus();
                      }
                    } else if (e.key === 'Escape') {
                      e.preventDefault();
                      e.stopPropagation();
                      rowEl?.focus();
                    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                      e.preventDefault();
                      e.stopPropagation();
                      moveRow(vItem.index + (e.key === 'ArrowDown' ? 1 : -1), (actionBtn?.dataset.action as ActionSlot) ?? null);
                    } else if (e.key === 'Enter' || e.key === ' ') {
                      e.stopPropagation();
                      if (actionBtn?.getAttribute('aria-disabled') === 'true') e.preventDefault();
                    }
                    return;
                  }

                  if (e.key === 'ArrowDown') {
                    e.preventDefault();
                    moveRow(vItem.index + 1, null);
                  } else if (e.key === 'ArrowUp') {
                    e.preventDefault();
                    moveRow(vItem.index - 1, null);
                  } else if (e.key === 'Home') {
                    e.preventDefault();
                    moveRow(0, null);
                  } else if (e.key === 'End') {
                    e.preventDefault();
                    moveRow(rows.length - 1, null);
                  } else if (e.key === 'PageDown') {
                    e.preventDefault();
                    moveRow(vItem.index + 10, null);
                  } else if (e.key === 'PageUp') {
                    e.preventDefault();
                    moveRow(vItem.index - 10, null);
                  } else if (e.key === 'ArrowRight') {
                    e.preventDefault();
                    if (row.expandable && !row.expanded) {
                      onToggle(row.key);
                    } else if (actionBtns.length > 0) {
                      actionBtns[0].focus();
                    }
                  } else if (e.key === 'ArrowLeft') {
                    e.preventDefault();
                    if (row.expandable && row.expanded) {
                      onToggle(row.key);
                    } else if (row.parentKey !== null) {
                      const parentIdx = rows.findIndex((r) => r.key === row.parentKey);
                      if (parentIdx >= 0) moveRow(parentIdx, null);
                    }
                  } else if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    if (row.expandable) {
                      onToggle(row.key);
                    } else if (row.kind === 'verb' && row.verb && getVerbDecision(row.verb).kind !== 'explain') {
                      onOpenForm?.(row.verb.path);
                    }
                  }
                }}
                className={cn(
                  'border-b border-border-subtle transition-colors hover:bg-overlay-hover',
                  row.kind === 'exemption_strip' ? 'p-2' : 'grid grid-cols-[1fr_176px_304px] items-center pr-4',
                  row.kind === 'domain' && 'min-h-[40px] bg-overlay-subtle/40',
                  (row.kind === 'group' || row.kind === 'mcp_group') && 'min-h-[48px]',
                  (row.kind === 'verb' || row.kind === 'mcp_tool') && 'min-h-[36px]',
                )}
              >
                {row.kind === 'exemption_strip' && row.exemption ? (
                  <div role="gridcell" aria-colspan={3} style={{ paddingLeft: `${indentPx}px` }} className="pr-2">
                    <dl className="grid grid-cols-[7rem_1fr] gap-x-3 gap-y-1 rounded-lg bg-overlay-subtle p-4 text-[13px] text-text-primary">
                      <dt className="text-text-muted">Owner</dt>
                      <dd className="min-w-0 break-words">{row.exemption.owner}</dd>
                      <dt className="text-text-muted">Reason</dt>
                      <dd className="min-w-0 break-words">{row.exemption.reason}</dd>
                      <dt className="text-text-muted">Review by</dt>
                      <dd className="min-w-0 break-words">
                        {isOverdue(row.exemption.reviewBy, today)
                          ? `${row.exemption.reviewBy}, overdue`
                          : row.exemption.reviewBy}
                      </dd>
                    </dl>
                  </div>
                ) : (
                  <>
                    <div role="gridcell" style={{ paddingLeft: `${indentPx}px` }} className="flex min-w-0 items-center gap-2 pr-4">
                      {row.expandable ? (
                        <Icon.chevR aria-hidden="true" className={cn('size-3.5 shrink-0 text-text-muted transition-transform', row.expanded && 'rotate-90')} />
                      ) : (
                        <span aria-hidden="true" className="w-6 shrink-0" />
                      )}

                      {row.kind === 'domain' && (
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="truncate font-display text-[13px] uppercase tracking-[0.14em] text-text-primary">{row.domainLabel}</span>
                          <span className="font-mono text-[11px] text-text-muted">{row.domainLeafCount ?? 0}</span>
                        </div>
                      )}

                      {row.kind === 'group' && row.group && (
                        <div className="min-w-0 flex-1 py-1">
                          <div className="truncate font-mono text-[13px] text-text-primary">vox {row.group.name}</div>
                          <div className="flex items-center gap-1 truncate text-[11px] text-text-muted">
                            {row.group.exemption && (
                              <>
                                <CapabilityTag tag="exempt" />
                                <span>review by {row.group.exemption.reviewBy}</span>
                                {isOverdue(row.group.exemption.reviewBy, today) && <CapabilityTag tag="overdue" />}
                                <span aria-hidden="true">·</span>
                              </>
                            )}
                            {row.group.crate && (
                              <>
                                {row.group.crateRowId ? (
                                  <button
                                    type="button"
                                    tabIndex={-1}
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      onExplain?.(row.group!.crateRowId!);
                                    }}
                                    className="text-text-secondary underline-offset-2 hover:underline focus-visible:underline"
                                  >
                                    {row.group.crate}
                                    {row.group.layer !== null ? ` · L${row.group.layer}` : ''}
                                  </button>
                                ) : (
                                  <span>
                                    {row.group.crate}
                                    {row.group.layer !== null ? ` · L${row.group.layer}` : ''}
                                  </span>
                                )}
                                <span aria-hidden="true">·</span>
                              </>
                            )}
                            <span>{row.group.verbCount} verbs</span>
                            {row.group.summary && (
                              <>
                                <span aria-hidden="true">·</span>
                                <span className="truncate">{row.group.summary}</span>
                              </>
                            )}
                          </div>
                        </div>
                      )}

                      {row.kind === 'mcp_group' && row.mcpGroup && (
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="truncate font-sans text-[13px] text-text-primary">MCP-only tools</span>
                          <span className="font-mono text-[11px] text-text-muted">({row.mcpGroup.tools.length})</span>
                        </div>
                      )}

                      {row.kind === 'verb' && row.verb && (
                        <div className="flex min-w-0 items-center gap-2 truncate">
                          <span className="truncate font-mono text-[13px] text-text-secondary">{row.verb.cli}</span>
                          {row.verb.summary && <span className="truncate text-[13px] text-text-muted">{row.verb.summary}</span>}
                        </div>
                      )}

                      {row.kind === 'mcp_tool' && row.mcpTool && (
                        <div className="flex min-w-0 items-center gap-2 truncate">
                          <span className="truncate font-mono text-[13px] text-text-secondary">{row.mcpTool.mcp}</span>
                          {row.mcpTool.summary && <span className="truncate text-[13px] text-text-muted">{row.mcpTool.summary}</span>}
                        </div>
                      )}
                    </div>

                    <div role="gridcell" className="pr-4">
                      {row.kind === 'domain' && <DomainRollupBar mix={row.domainMix} />}
                      {row.kind === 'group' && row.group && <CapabilityBadge status={row.group.status} />}
                      {row.kind === 'mcp_group' && <CapabilityBadge status="mcp_only" />}
                      {row.kind === 'verb' && row.verb && <CapabilityBadge status={row.verb.status} />}
                      {row.kind === 'mcp_tool' && <CapabilityBadge status="mcp_only" />}
                    </div>

                    <div role="gridcell" onClick={(e) => e.stopPropagation()}>
                      {renderActions(row)}
                    </div>
                  </>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </Glass>
  );
}
