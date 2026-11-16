import React, { useEffect, useRef } from 'react';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { TERMINAL_MONO_FONT_STACK } from './terminalTheme';

export interface TermSearchBarProps {
  open: boolean;
  query: string;
  activeMatchIndex: number;
  totalMatches: number;
  onQueryChange: (next: string) => void;
  onNext: () => void;
  onPrev: () => void;
  onClose: () => void;
}

export function TermSearchBar({
  open, query, activeMatchIndex, totalMatches, onQueryChange, onNext, onPrev, onClose,
}: TermSearchBarProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  if (!open) return null;

  const countLabel =
    query.length > 0 && totalMatches === 0
      ? '0 of 0 matches - try another pattern or press Esc to clear'
      : `${activeMatchIndex} of ${totalMatches}`;

  return (
    <div
      data-testid="term-search-bar"
      className="flex h-8 shrink-0 items-center gap-2 border-b border-border-subtle bg-bg-elevated px-2 text-xs"
    >
      <Icon.search className="size-4 shrink-0 text-text-muted" aria-hidden="true" />
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            onClose();
          } else if (e.key === 'Enter') {
            e.preventDefault();
            if (e.shiftKey) onPrev();
            else onNext();
          }
        }}
        data-testid="term-search-input"
        data-terminal-mono="true"
        aria-label="Search terminal blocks and buffer"
        placeholder="Find in terminal..."
        style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}
        className="h-6 min-w-0 flex-1 rounded border border-border-subtle bg-bg-base px-2 text-xs text-text-primary placeholder:text-text-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      />
      <span data-testid="term-search-count" className="shrink-0 text-[11px] leading-[1.25] font-semibold text-text-muted">
        {countLabel}
      </span>
      <Button size="xs" variant="ghost" data-testid="term-search-prev" aria-label="Prev search match" onClick={onPrev} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold">
        <Icon.arrowUp className="size-3" aria-hidden="true" />
        <span>Prev</span>
      </Button>
      <Button size="xs" variant="ghost" data-testid="term-search-next" aria-label="Next search match" onClick={onNext} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold">
        <Icon.arrowDown className="size-3" aria-hidden="true" />
        <span>Next</span>
      </Button>
      <Button size="xs" variant="ghost" data-testid="term-search-close" aria-label="Esc close terminal search" onClick={onClose} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold">
        <Icon.close className="size-3" aria-hidden="true" />
        <span>Esc</span>
      </Button>
    </div>
  );
}
