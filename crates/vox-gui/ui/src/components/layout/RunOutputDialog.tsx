import React from 'react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../ui/Dialog';
import type { ExecuteOutput } from '../../transport';

interface RunOutputDialogProps {
  /** Null = closed. */
  run: { title: string; out: ExecuteOutput } | null;
  onClose: () => void;
}

/** D-04: the complete stdout/stderr of a palette run, as text (never HTML). */
export function RunOutputDialog({ run, onClose }: RunOutputDialogProps) {
  return (
    <Dialog open={run !== null} onOpenChange={(next) => { if (!next) onClose(); }}>
      <DialogContent className="max-w-3xl">
        <DialogTitle>{run?.title ?? 'Command output'}</DialogTitle>
        <DialogDescription>exit code {run?.out.exit_code}</DialogDescription>
        {(['stdout', 'stderr'] as const).map((stream) => (
          <div key={stream} className="mt-3">
            <div className="mb-1 font-mono text-[10px] uppercase tracking-widest text-text-muted">{stream}</div>
            <pre
              aria-label={stream}
              className="max-h-[30vh] overflow-auto whitespace-pre-wrap rounded-lg border border-border-subtle bg-bg-base p-3 font-mono text-[11px] text-text-secondary"
            >
              {run?.out[stream] || '(empty)'}
            </pre>
          </div>
        ))}
      </DialogContent>
    </Dialog>
  );
}
