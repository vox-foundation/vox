import React from 'react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../../ui/Dialog';

interface TaskDiffDialogProps {
  open: boolean;
  /** True while `get_task_diff` is in flight; never claims "no changes" until it has answered. */
  loading: boolean;
  text: string;
  onClose: () => void;
}

/** The `/diff` slash command's result: the diff the agent staged, as the engine produced it. */
export function TaskDiffDialog({ open, loading, text, onClose }: TaskDiffDialogProps) {
  const hasDiff = text.trim() !== '';
  return (
    <Dialog open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <DialogContent className="max-w-3xl">
        <DialogTitle>Pending diff</DialogTitle>
        <DialogDescription>What the agent has staged and not yet committed.</DialogDescription>
        <div className="mt-3 max-h-[60vh] overflow-auto rounded-lg border border-border-subtle bg-bg-base p-3">
          {loading ? (
            <p className="text-xs text-text-muted">Loading the pending diff…</p>
          ) : hasDiff ? (
            <pre data-testid="task-diff-text" className="whitespace-pre font-mono text-[11px] text-text-secondary">{text}</pre>
          ) : (
            <p className="text-xs text-text-muted">No pending changes.</p>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
