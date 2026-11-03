import React from 'react';
import type { ChatVerbosity } from '../../../hooks/useChatVerbosity';

const OPTIONS: ReadonlyArray<{ value: ChatVerbosity; label: string; hint: string }> = [
  { value: 'quiet', label: 'Quiet', hint: 'Traces stay collapsed; only things that need you show' },
  { value: 'normal', label: 'Normal', hint: 'A trace opens when a receipt failed or something needs you' },
  { value: 'verbose', label: 'Verbose', hint: 'Every trace opens' },
];

/** How much of each turn's trace opens by default (`buildTurnTrace`). */
export function ChatVerbosityControl({
  value,
  onChange,
}: {
  value: ChatVerbosity;
  onChange: (next: ChatVerbosity) => void;
}) {
  return (
    <div
      role="radiogroup"
      aria-label="Trace detail"
      data-testid="chat-verbosity-control"
      className="flex items-center gap-1 self-end font-mono text-[11px]"
    >
      {OPTIONS.map((o) => {
        const active = value === o.value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={active}
            title={o.hint}
            onClick={() => onChange(o.value)}
            className={`rounded-md border px-2 py-0.5 ${
              active ? 'border-brass/40 text-text-primary' : 'border-transparent text-text-muted hover:text-text-secondary'
            }`}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
