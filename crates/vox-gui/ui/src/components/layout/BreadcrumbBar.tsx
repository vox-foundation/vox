import React from 'react';
import {
  breadcrumbsForView,
  type BreadcrumbSegment,
  resolveNavigation,
  CHILD_ORDER_BY_PARENT,
  labelForNavKey,
} from '../../lib/navigation';
import { useCurrentLang } from '../../hooks/useLanguage';
import { recordGamifyGuiEvent } from '../../lib/gamifyGuiEvents';

interface Props {
  viewKey: string;
  onNavigate?: (viewKey: string) => void;
  gamifyEnabled?: boolean;
}

export function BreadcrumbBar({ viewKey, onNavigate, gamifyEnabled }: Props) {
  const lang = useCurrentLang();
  const segments = breadcrumbsForView(viewKey, lang);
  if (segments.length === 0 || viewKey === 'chat') return null;

  const { parent, child } = resolveNavigation(viewKey);
  const siblingChildren = CHILD_ORDER_BY_PARENT[parent];

  const renderSegment = (seg: BreadcrumbSegment, index: number) => {
    const isLast = index === segments.length - 1;
    const content = (
      <span className={isLast ? 'text-text-secondary' : 'text-text-muted'}>{seg.label}</span>
    );
    if (!isLast && onNavigate) {
      return (
        <button
          key={seg.key}
          type="button"
          onClick={() => {
            void recordGamifyGuiEvent(
              'breadcrumb_navigation',
              { from: viewKey, to: seg.key },
              { enabled: gamifyEnabled },
            );
            onNavigate(seg.key);
          }}
          className="font-display text-[11px] uppercase tracking-[0.14em] hover:text-text-secondary transition"
          aria-label={`Navigate to ${seg.label}`}
        >
          {content}
        </button>
      );
    }
    return (
      <span
        key={seg.key}
        className="font-display text-[11px] uppercase tracking-[0.14em]"
        aria-current={isLast ? 'page' : undefined}
      >
        {content}
      </span>
    );
  };

  return (
    <nav aria-label="Breadcrumb" className="flex flex-wrap items-center gap-2 px-1 pb-2">
      <div className="flex items-center gap-2">
        {segments.map((seg, i) => (
          <React.Fragment key={`${i}:${seg.key}`}>
            {i > 0 && <span className="text-text-muted text-[10px]" aria-hidden="true">›</span>}
            {renderSegment(seg, i)}
          </React.Fragment>
        ))}
      </div>

      {siblingChildren && siblingChildren.length > 1 && (
        <div
          role="tablist"
          aria-label={`${labelForNavKey(parent)} subsections`}
          className="ml-3 flex items-center gap-1 border-l border-border-subtle pl-3 overflow-x-auto no-scrollbar"
        >
          {siblingChildren.map(subKey => {
            const isCurrent = child === subKey;
            return (
              <button
                key={subKey}
                type="button"
                role="tab"
                aria-selected={isCurrent}
                onClick={() => {
                  if (!isCurrent && onNavigate) {
                    void recordGamifyGuiEvent(
                      'breadcrumb_navigation',
                      { from: viewKey, to: subKey },
                      { enabled: gamifyEnabled },
                    );
                    onNavigate(subKey);
                  }
                }}
                className={`rounded-md px-2 py-0.5 font-display text-[10px] uppercase tracking-wider transition whitespace-nowrap ${
                  isCurrent
                    ? 'border border-brass/40 bg-brass/15 text-brass font-medium'
                    : 'text-text-muted hover:bg-overlay-hover hover:text-text-secondary'
                }`}
              >
                {labelForNavKey(subKey)}
              </button>
            );
          })}
        </div>
      )}
    </nav>
  );
}

