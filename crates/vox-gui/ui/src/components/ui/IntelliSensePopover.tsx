import React, { useMemo, useState } from 'react';
import type { CompletionCategory, IntelliSenseItem } from '../../lib/intellisenseEngine';

interface Props {
  open: boolean;
  items: IntelliSenseItem[];
  activeIndex: number;
  onSelectIndex: (index: number) => void;
  onAcceptItem: (item: IntelliSenseItem) => void;
  onClose?: () => void;
  position?: 'above' | 'below';
}

const CATEGORY_TABS: Array<{ id: CompletionCategory; label: string }> = [
  { id: 'all', label: 'All' },
  { id: 'command', label: 'Commands' },
  { id: 'slash', label: 'Slash & Skills' },
  { id: 'term', label: 'Terminal @term' },
  { id: 'file', label: 'Files' },
  { id: 'mcp', label: 'MCP & Symbols' },
  { id: 'nu', label: 'Nushell' },
];

function matchesTab(item: IntelliSenseItem, tab: CompletionCategory): boolean {
  if (tab === 'all') return true;
  if (tab === 'slash') return item.category === 'slash' || item.category === 'skill';
  if (tab === 'mcp') return item.category === 'mcp' || item.category === 'symbol';
  return item.category === tab;
}

export function IntelliSensePopover({
  open,
  items,
  activeIndex,
  onSelectIndex,
  onAcceptItem,
  position = 'above',
}: Props) {
  const [activeTab, setActiveTab] = useState<CompletionCategory>('all');

  const visibleItems = useMemo(
    () => items.filter((it) => matchesTab(it, activeTab)).slice(0, 14),
    [items, activeTab],
  );

  if (!open || items.length === 0) return null;

  const clampedIndex = Math.min(
    Math.max(0, activeIndex),
    Math.max(0, visibleItems.length - 1),
  );
  const highlighted = visibleItems[clampedIndex] ?? items[0];

  const posClass =
    position === 'above'
      ? 'bottom-[calc(100%+6px)] left-0'
      : 'top-[calc(100%+6px)] left-0';

  return (
    <div
      data-testid="intellisense-popover"
      role="listbox"
      aria-label="IntelliSense suggestions"
      className={`absolute ${posClass} z-50 w-[480px] max-w-[calc(100vw-2rem)] rounded-lg border border-border-subtle bg-bg-base/95 p-1.5 backdrop-blur-xl shadow-[0_24px_60px_-20px_rgba(0,0,0,0.92)]`}
    >
      <div className="mb-1 flex flex-wrap items-center gap-1 border-b border-white/10 px-1 pb-1">
        {CATEGORY_TABS.map((tab) => {
          const count = items.filter((it) => matchesTab(it, tab.id)).length;
          if (tab.id !== 'all' && count === 0) return null;
          const isCurrent = activeTab === tab.id;
          return (
            <button
              type="button"
              key={tab.id}
              data-testid={`intellisense-tab-${tab.id}`}
              onClick={() => {
                setActiveTab(tab.id);
                onSelectIndex(0);
              }}
              className={`rounded px-1.5 py-0.5 font-mono text-[10px] transition ${
                isCurrent
                  ? 'border border-brass/40 bg-brass/15 text-brass'
                  : 'text-text-muted hover:bg-overlay-subtle hover:text-text-secondary'
              }`}
            >
              {tab.label} <span className="opacity-65">({count})</span>
            </button>
          );
        })}
      </div>

      <div className="max-h-[240px] overflow-y-auto overscroll-contain">
        {visibleItems.map((item, idx) => {
          const isActive = idx === clampedIndex;
          return (
            <button
              type="button"
              key={item.id}
              aria-current={isActive ? 'true' : undefined}
              data-active={isActive ? 'true' : 'false'}
              data-testid={
                item.category === 'term' ? 'loquela-at-term-option' : 'intellisense-option'
              }
              onMouseEnter={() => onSelectIndex(idx)}
              onMouseDown={(e) => {
                // Prevent input blur before click handler fires
                e.preventDefault();
              }}
              onClick={() => onAcceptItem(item)}
              className={`flex w-full items-center gap-2 rounded px-2 py-1.5 text-left transition ${
                isActive ? 'bg-overlay-subtle ring-1 ring-brass/30' : 'hover:bg-overlay-subtle/60'
              }`}
            >
              <span className="shrink-0 rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-wider text-brass">
                {item.badge}
              </span>
              <span className="truncate font-mono text-[11px] font-medium text-text-primary">
                {item.label}
              </span>
              <span className="ml-auto truncate pl-2 text-[11px] text-text-muted max-w-[210px]">
                {item.description}
              </span>
            </button>
          );
        })}
      </div>

      {highlighted && (
        <div
          data-testid="intellisense-footer"
          className="mt-1 flex flex-col gap-0.5 border-t border-white/10 px-2 pt-1.5 text-[10px]"
        >
          <div className="flex items-center justify-between gap-2">
            <code className="truncate font-mono text-[10px] text-cyan-300">
              {highlighted.signature ?? highlighted.label}
            </code>
            <span className="shrink-0 font-mono text-[9px] text-text-muted">
              ↑↓ Navigate · Tab Accept · Enter Run · Esc Close
            </span>
          </div>
          <div className="truncate text-text-muted">{highlighted.description}</div>
        </div>
      )}
    </div>
  );
}
