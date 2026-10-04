import React from 'react';
import { cn } from '../../lib/cn';

export type PhaseKind = 
  | 'Verifying' 
  | 'Executing' 
  | 'Planning' 
  | 'Paused' 
  | 'Validated' 
  | 'Doubted' 
  | 'Speculative' 
  | 'Active' 
  | 'Root';

export const PHASE_TONE: Record<PhaseKind, { dot: string; ring: string; text: string; glow: string }> = {
  Verifying:   { dot: "bg-violet-400", ring: "ring-violet-400/30", text: "text-violet-300",  glow: "shadow-[0_0_18px_-4px_rgba(167,139,250,0.55)]" },
  Executing:   { dot: "bg-brass",      ring: "ring-brass/30",      text: "text-brass",       glow: "" },
  // Was cyan-400 (a blue chip) — the only PhaseKind tones that broke from
  // the app's brass/gold accent language used everywhere else (Approvals,
  // Discovery, Repository). Reuses the `brass` token, matching Executing,
  // per tokens.ts's STATUS_TONE (the SSOT these two maps mirror).
  Planning:    { dot: "bg-brass",      ring: "ring-brass/30",      text: "text-brass",       glow: "" },
  Paused:      { dot: "bg-text-muted",   ring: "ring-border-subtle/30",   text: "text-text-secondary",    glow: "" },
  Validated:   { dot: "bg-(--color-status-pass)", ring: "ring-(--color-status-pass)/30", text: "text-(--color-status-pass)", glow: "" },
  Doubted:     { dot: "bg-(--color-status-warn)", ring: "ring-(--color-status-warn)/30", text: "text-(--color-status-warn)", glow: "" },
  Speculative: { dot: "bg-violet-400", ring: "ring-violet-400/30", text: "text-violet-300",  glow: "" },
  Active:      { dot: "bg-brass",      ring: "ring-brass/30",      text: "text-brass",       glow: "" },
  Root:        { dot: "bg-white",      ring: "ring-white/30",      text: "text-white",       glow: "shadow-[0_0_22px_-2px_rgba(255,255,255,0.5)]" },
};

interface PillProps {
  phase: PhaseKind | string;
  label?: string;
  className?: string;
}

export function Pill({ phase, label, className = "" }: PillProps) {
  const t = PHASE_TONE[phase as PhaseKind] || PHASE_TONE.Paused;
  return (
    <span className={cn(
      "inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[11px] font-medium tracking-[0.08em] uppercase ring-1 bg-overlay-subtle",
      t.ring,
      t.text,
      className
    )}>
      <span className={cn("relative inline-block size-1.5 rounded-full", t.dot)}>
        {phase !== "Paused" && (
          <span className={cn("absolute inset-0 rounded-full animate-vox-ping opacity-60", t.dot)} />
        )}
      </span>
      {label || phase}
    </span>
  );
}
