import React, { useRef, useState } from 'react';
import { Glass } from '../../ui/Glass';
import { cn } from '../../../lib/cn';
import type { DomainCountEntry, StatusMixCounts } from './explorerModel';

export interface ExplorerRailProps {
  domains: readonly DomainCountEntry[];
  allMix: StatusMixCounts;
  selected: string | null;
  onSelect: (domainId: string | null) => void;
  narrow?: boolean;
}

interface RailItem {
  id: string | null;
  label: string;
  mix: StatusMixCounts;
}

function StatusMixBar({ mix }: { mix: StatusMixCounts }) {
  const total = mix.total;
  if (total <= 0) {
    return <div aria-hidden="true" className="h-1 w-full rounded-full bg-overlay-subtle" />;
  }
  const pct = (n: number) => `${(n / total) * 100}%`;
  return (
    <div aria-hidden="true" className="flex h-1 w-full overflow-hidden rounded-full bg-overlay-subtle">
      {mix.dedicated > 0 && (
        <span className="h-full bg-(--color-status-pass)" style={{ width: pct(mix.dedicated) }} />
      )}
      {mix.generic_form > 0 && (
        <span className="h-full bg-(--color-status-info)" style={{ width: pct(mix.generic_form) }} />
      )}
      {mix.cli_only > 0 && (
        <span className="h-full bg-(--color-text-secondary)" style={{ width: pct(mix.cli_only) }} />
      )}
      {mix.mcp_only > 0 && (
        <span className="h-full bg-(--color-status-warn)" style={{ width: pct(mix.mcp_only) }} />
      )}
    </div>
  );
}

export function ExplorerRail({
  domains,
  allMix,
  selected,
  onSelect,
  narrow = false,
}: ExplorerRailProps) {
  const items: RailItem[] = [
    { id: null, label: 'All domains', mix: allMix },
    ...domains.map((d) => ({ id: d.id, label: d.label, mix: d.mix })),
  ];

  const selectedIdx = Math.max(
    0,
    items.findIndex((item) => item.id === selected),
  );
  const [focusedIdx, setFocusedIdx] = useState<number>(selectedIdx);
  const btnRefs = useRef<(HTMLButtonElement | null)[]>([]);

  if (narrow) {
    return (
      <select
        aria-label="Domain"
        value={selected ?? ''}
        onChange={(e) => onSelect(e.target.value === '' ? null : e.target.value)}
        className="h-9 w-full rounded-lg border border-border-subtle bg-bg-surface px-3 text-[13px] text-text-primary"
      >
        {items.map((item) => (
          <option key={item.id ?? '__all__'} value={item.id ?? ''}>
            {item.label} ({item.mix.total})
          </option>
        ))}
      </select>
    );
  }

  const moveFocus = (nextIdx: number) => {
    const clamped = Math.max(0, Math.min(items.length - 1, nextIdx));
    setFocusedIdx(clamped);
    btnRefs.current[clamped]?.focus();
  };

  return (
    <Glass
      inset={false}
      className="w-[240px] shrink-0 rounded-xl p-2 max-h-full overflow-y-auto custom-scrollbar"
    >
      <nav aria-label="Domains" className="space-y-1">
        {items.map((item, idx) => {
          const isSelected = item.id === selected;
          const isZero = item.mix.total === 0;
          const activeTabStop = Math.min(focusedIdx, items.length - 1) === idx;
          const srText = `${item.label}, ${item.mix.total} ${
            item.mix.total === 1 ? 'capability' : 'capabilities'
          }: ${item.mix.generic_form} Generic form, ${item.mix.dedicated} Dedicated surface, ${
            item.mix.cli_only
          } CLI-only, ${item.mix.mcp_only} MCP-only / orphan`;

          return (
            <button
              key={item.id ?? '__all__'}
              ref={(el) => {
                btnRefs.current[idx] = el;
              }}
              type="button"
              tabIndex={activeTabStop ? 0 : -1}
              aria-current={isSelected ? 'true' : undefined}
              title={item.label}
              onClick={() => {
                setFocusedIdx(idx);
                onSelect(item.id);
              }}
              onKeyDown={(e) => {
                if (e.key === 'ArrowDown') {
                  e.preventDefault();
                  moveFocus(idx + 1);
                } else if (e.key === 'ArrowUp') {
                  e.preventDefault();
                  moveFocus(idx - 1);
                } else if (e.key === 'Home') {
                  e.preventDefault();
                  moveFocus(0);
                } else if (e.key === 'End') {
                  e.preventDefault();
                  moveFocus(items.length - 1);
                }
              }}
              className={cn(
                'flex h-10 w-full flex-col justify-center gap-1 rounded-lg px-2 text-left transition-colors',
                isSelected
                  ? 'border-l-2 border-(--color-accent-default) bg-overlay-subtle text-(--color-accent-default)'
                  : isZero
                    ? 'text-text-muted hover:bg-overlay-hover'
                    : 'text-text-secondary hover:bg-overlay-hover',
              )}
            >
              <span className="sr-only">{srText}</span>
              <span aria-hidden="true" className="flex items-center justify-between gap-2 w-full">
                <span className="truncate text-[13px]">{item.label}</span>
                <span className="shrink-0 font-mono text-[11px] text-text-muted">
                  {item.mix.total}
                </span>
              </span>
              <StatusMixBar mix={item.mix} />
            </button>
          );
        })}
      </nav>
    </Glass>
  );
}
