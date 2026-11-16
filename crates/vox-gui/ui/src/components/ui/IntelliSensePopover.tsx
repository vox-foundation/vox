import React, { useMemo, useState } from 'react';
import type { CompletionCategory, IntelliSenseItem } from '../../lib/intellisenseEngine';
import { TERMINAL_MONO_FONT_STACK } from '../surfaces/Console/terminalTheme';
import { KeycapIcon } from './Icons';

export interface IntelliSensePopoverProps {
  open: boolean;
  items: Array<IntelliSenseItem & { badge?: string }>;
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
  { id: 'history', label: 'History' },
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
}: IntelliSensePopoverProps) {
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
      ? 'bottom-[calc(100%+8px)] left-0'
      : 'top-[calc(100%+8px)] left-0';

  return (
    <div
      data-testid="intellisense-popover"
      className={`absolute ${posClass} z-50 w-[480px] max-w-[calc(100vw-32px)] rounded-lg border border-border-strong bg-bg-surface p-2 shadow-lg`}
    >
      <div className="mb-1 flex flex-wrap items-center gap-1 border-b border-border-subtle px-1 pb-1">
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
              style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}
              className={`rounded px-2 py-1 text-[11px] font-semibold transition ${
                isCurrent
                  ? 'border border-brass/40 bg-brass/15 text-accent'
                  : 'text-text-muted hover:bg-overlay-subtle hover:text-text-secondary'
              }`}
            >
              {tab.label} <span className="opacity-70">({count})</span>
            </button>
          );
        })}
      </div>

      <div
        role="listbox"
        aria-label="IntelliSense suggestions"
        className="flex max-h-60 flex-col gap-1 overflow-y-auto overscroll-contain"
      >
        {visibleItems.map((item, idx) => {
          const isActive = idx === clampedIndex;
          const isAccentBadge =
            item.badge === 'MACRO' || item.badge === 'SLASH' || item.category === 'term';
          return (
            <button
              type="button"
              role="option"
              aria-selected={isActive}
              key={item.id}
              aria-current={isActive ? 'true' : undefined}
              data-active={isActive ? 'true' : 'false'}
              data-testid={
                item.category === 'term' ? 'loquela-at-term-option' : 'intellisense-option'
              }
              onMouseEnter={() => onSelectIndex(idx)}
              onMouseDown={(e) => {
                e.preventDefault();
              }}
              onClick={() => onAcceptItem(item)}
              className={`flex w-full items-center gap-2 rounded px-2 py-1 text-left transition ${
                isActive ? 'bg-overlay-subtle ring-1 ring-brass/40' : 'hover:bg-overlay-subtle/60'
              }`}
            >
              <span
                style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}
                className={`shrink-0 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold uppercase tracking-wider ${
                  isAccentBadge
                    ? 'bg-brass/15 text-accent'
                    : 'bg-bg-elevated text-text-secondary'
                }`}
              >
                {item.badge}
              </span>
              <span
                style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}
                className="truncate text-xs font-normal text-text-primary"
              >
                {item.label}
              </span>
              <span className="ml-auto max-w-[208px] truncate pl-2 text-xs font-normal text-text-muted">
                {item.description}
              </span>
            </button>
          );
        })}
      </div>

      {highlighted && (
        <div
          data-testid="intellisense-footer"
          className="mt-1 flex flex-col gap-1 border-t border-border-subtle px-2 pt-2 text-xs"
        >
          <div className="flex items-center justify-between gap-2">
            <code
              style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}
              className="truncate text-xs font-normal text-accent"
            >
              {highlighted.signature ?? highlighted.label}
            </code>
            <span
              style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}
              className="inline-flex shrink-0 items-center gap-1 text-[11px] font-semibold text-text-muted"
            >
              <span className="inline-flex items-center gap-1">
                <KeycapIcon keyName="Up" className="size-3" />
                <KeycapIcon keyName="Down" className="size-3" />
              </span>
              <span>Navigate - Tab Insert - Enter Run - Esc Dismiss</span>
            </span>
          </div>
          <div className="truncate text-xs font-normal text-text-muted">
            {highlighted.description}
          </div>
        </div>
      )}
    </div>
  );
}
