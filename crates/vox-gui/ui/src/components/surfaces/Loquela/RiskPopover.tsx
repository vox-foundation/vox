import React, { useEffect, useRef } from 'react';
import { RISK_POSTURES, type RiskId, type ControlState } from '../../../lib/driveConsole';

const COPY: Record<RiskId, string> = {
  high: 'Break things — auto-approve more, gates shadow-only, fewer safety tokens.',
  moderate: 'Confirm, and check replies. Balanced safety.',
  low: 'Enforce verification, check replies, raise approval, spend safety tokens, lean model up.',
};

interface RiskPopoverProps {
  risk: RiskId;
  open: boolean;
  onChange: (next: Partial<ControlState>) => void;
  onClose: () => void;
  children?: React.ReactNode;
}

export function RiskPopover({ risk, open, onChange, onClose, children }: RiskPopoverProps) {
  const activeBtnRef = useRef<HTMLButtonElement>(null);

  // Move focus into the dialog when it opens so keyboard users land on the
  // currently-selected posture (Escape handling already returns focus out).
  useEffect(() => {
    if (open) activeBtnRef.current?.focus();
  }, [open]);

  if (!open) return null;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') onClose();
  };

  return (
    <div
      role="dialog"
      aria-label="Configure acceptable risk"
      onKeyDown={handleKeyDown}
      className="absolute bottom-full right-0 z-[99999] mb-1 w-64 whitespace-normal rounded-lg border border-border-subtle bg-bg-base/98 p-2.5 text-[11px] text-text-primary shadow-2xl backdrop-blur-xl"
    >
      <div className="mb-1.5 text-[10px] uppercase tracking-[0.08em] text-text-muted">Acceptable risk</div>
      {RISK_POSTURES.map(p => (
        <button
          key={p.id}
          type="button"
          ref={risk === p.id ? activeBtnRef : undefined}
          onClick={() => { onChange({ risk: p.id }); }}
          aria-pressed={risk === p.id}
          className={`mb-1 flex w-full flex-col rounded-md border px-2 py-1 text-left ${
            risk === p.id ? 'border-brass/40 bg-brass/10' : 'border-border-subtle hover:border-brass/30'
          }`}
        >
          <span className="font-medium capitalize text-text-primary">{p.label} risk</span>
          <span className="text-[10px] leading-snug text-text-secondary">{COPY[p.id]}</span>
        </button>
      ))}
      {children ? (
        <div className="mt-1.5 border-t border-border-subtle pt-1.5">
          <div className="mb-1 text-[10px] uppercase tracking-[0.08em] text-text-muted">After each reply</div>
          {children}
        </div>
      ) : null}
    </div>
  );
}
