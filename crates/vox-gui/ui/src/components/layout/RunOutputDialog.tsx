import React from 'react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../ui/Dialog';
import type { ExecuteOutput } from '../../transport';
import { boundOutput } from '../../lib/capabilityRun';

interface RunOutputDialogProps {
  /** Null = closed. */
  run: { title: string; out: ExecuteOutput } | null;
  onClose: () => void;
}

/**
 * D-04: the stdout/stderr of a palette run, as text (never HTML). WR-09: the backend buffers
 * whole output, so the display is bounded; a truncated stream says so and offers the full text.
 */
export function RunOutputDialog({ run, onClose }: RunOutputDialogProps) {
  return (
    <Dialog open={run !== null} onOpenChange={(next) => { if (!next) onClose(); }}>
      <DialogContent className="max-w-3xl">
        <DialogTitle>{run?.title ?? 'Command output'}</DialogTitle>
        <DialogDescription>exit code {run?.out.exit_code}</DialogDescription>
        {(['stdout', 'stderr'] as const).map((stream) => {
          const full = run?.out[stream] ?? '';
          const shown = boundOutput(full);
          return (
            <div key={stream} className="mt-3">
              <div className="mb-1 flex items-center justify-between font-mono text-[10px] uppercase tracking-widest text-text-muted">
                <span>{stream}</span>
                {shown.truncated ? (
                  <span className="flex items-center gap-2 normal-case tracking-normal">
                    <span>
                      truncated: showing {shown.text.length.toLocaleString()} of {shown.total.toLocaleString()} characters
                    </span>
                    <button
                      type="button"
                      onClick={() => { void navigator.clipboard?.writeText?.(full)?.catch?.(() => {}); }}
                      className="rounded-sm border border-border-subtle px-1.5 py-px text-text-secondary hover:bg-overlay-subtle"
                    >
                      Copy full {stream}
                    </button>
                  </span>
                ) : null}
              </div>
              <pre
                tabIndex={0}
                role="region"
                aria-label={stream}
                className="max-h-[30vh] overflow-auto whitespace-pre-wrap rounded-lg border border-border-subtle bg-bg-base p-3 font-mono text-[11px] text-text-secondary"
              >
                {shown.text || '(empty)'}
              </pre>
            </div>
          );
        })}
      </DialogContent>
    </Dialog>
  );
}
