import React, { useState } from 'react';
import { Popover } from '../../ui/Popover';
import { Icon } from '../../ui/Icons';

export type ResearchMode = 'auto' | 'fast' | 'deep' | 'off';

export interface ResearchOption {
  id: ResearchMode;
  label: string;
  shortLabel: string;
  hint: string;
  badge?: string;
}

export const RESEARCH_OPTIONS: ResearchOption[] = [
  {
    id: 'auto',
    label: 'Auto (Socrates)',
    shortLabel: 'Auto',
    hint: 'Evidence-weighted; Socrates decides whether & how deep to research',
    badge: 'Socrates',
  },
  {
    id: 'fast',
    label: 'Fast Web',
    shortLabel: 'Fast',
    hint: '1-hop shallow web retrieval across search providers',
  },
  {
    id: 'deep',
    label: 'Deep Research',
    shortLabel: 'Deep',
    hint: 'Multi-hop iterative exploration, cross-corpus synthesis & self-verification',
  },
  {
    id: 'off',
    label: 'Off',
    shortLabel: 'Off',
    hint: 'No external search; answers strictly from local context & model',
  },
];

export function ResearchModeDropdown({
  mode,
  onChange,
  className = '',
}: {
  mode: ResearchMode;
  onChange: (next: ResearchMode) => void;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const selected = RESEARCH_OPTIONS.find(o => o.id === mode) ?? RESEARCH_OPTIONS[0];

  return (
    <div className={`relative ${className}`}>
      <button
        type="button"
        aria-expanded={open}
        aria-label={`Research mode: ${selected.label}`}
        onClick={() => setOpen(o => !o)}
        title={selected.hint}
        className="inline-flex items-center gap-1.5 rounded-md border border-border-subtle bg-overlay-subtle px-2 py-1 text-text-secondary hover:border-white/20 transition-colors"
      >
        {selected.id === 'auto' ? (
          <Icon.brain className="size-3 text-brass" />
        ) : selected.id === 'fast' ? (
          <Icon.bolt className="size-3 text-amber-300" />
        ) : selected.id === 'deep' ? (
          <Icon.globe className="size-3 text-cyan-300" />
        ) : (
          <Icon.search className="size-3 text-text-muted" />
        )}
        <span className="text-[11px] text-text-muted">Research:</span>
        <span className="text-[11px] text-text-primary font-medium">{selected.shortLabel}</span>
        {selected.badge && (
          <span className="rounded bg-brass/15 px-1 py-0.5 font-mono text-[8px] text-brass">
            {selected.badge}
          </span>
        )}
        <Icon.chevR className="size-2.5 text-text-muted rotate-90" />
      </button>

      <Popover open={open}>
        <div className="w-64 p-1">
          {RESEARCH_OPTIONS.map(opt => {
            const isSelected = opt.id === mode;
            return (
              <button
                key={opt.id}
                type="button"
                aria-label={`Set research mode: ${opt.label}`}
                onClick={() => {
                  onChange(opt.id);
                  setOpen(false);
                }}
                className={`flex w-full items-start gap-2 rounded-sm px-2 py-1.5 text-left hover:bg-overlay-subtle transition-colors ${
                  isSelected ? 'bg-overlay-subtle border-l-2 border-brass' : ''
                }`}
              >
                <div className="mt-0.5 shrink-0">
                  {opt.id === 'auto' ? (
                    <Icon.brain className="size-3.5 text-brass" />
                  ) : opt.id === 'fast' ? (
                    <Icon.bolt className="size-3.5 text-amber-300" />
                  ) : opt.id === 'deep' ? (
                    <Icon.globe className="size-3.5 text-cyan-300" />
                  ) : (
                    <Icon.search className="size-3.5 text-text-muted" />
                  )}
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-1.5">
                    <span className={`text-[11px] ${isSelected ? 'text-brass font-medium' : 'text-text-primary'}`}>
                      {opt.label}
                    </span>
                    {opt.badge && (
                      <span className="rounded bg-brass/15 px-1 font-mono text-[8px] text-brass">
                        {opt.badge}
                      </span>
                    )}
                  </div>
                  <div className="font-mono text-[9px] text-text-muted leading-tight mt-0.5">
                    {opt.hint}
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      </Popover>
    </div>
  );
}
