import React from 'react';
import { breadcrumbsForView, type BreadcrumbSegment } from '../../lib/navigation';
import { useCurrentLang } from '../../hooks/useLanguage';
import { recordGamifyGuiEvent } from '../../lib/gamifyGuiEvents';
import { surfaceAvailability, useBackendStatus } from '../../hooks/useBackendStatus';
import { UNAVAILABLE_REASON_LABEL } from './SurfaceUnavailable';

interface Props {
  viewKey: string;
  onNavigate?: (viewKey: string) => void;
  gamifyEnabled?: boolean;
}

export function BreadcrumbBar({ viewKey, onNavigate, gamifyEnabled }: Props) {
  const lang = useCurrentLang();
  const segments = breadcrumbsForView(viewKey, lang);
  const { status } = useBackendStatus();
  const availability = surfaceAvailability(status, viewKey);
  if (segments.length === 0 || viewKey === 'chat') return null;

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
    <nav aria-label="Breadcrumb" className="flex items-center gap-2 px-1 pb-2">
      {segments.map((seg, i) => (
        <React.Fragment key={`${i}:${seg.key}`}>
          {i > 0 && <span className="text-text-muted text-[10px]" aria-hidden="true">›</span>}
          {renderSegment(seg, i)}
        </React.Fragment>
      ))}
      {!availability.available && availability.reason && (
        <span data-testid="breadcrumb-unavailable" className="font-display text-[10px] uppercase tracking-[0.14em] text-amber-300/80">
          · {UNAVAILABLE_REASON_LABEL[availability.reason]}
        </span>
      )}
    </nav>
  );
}
