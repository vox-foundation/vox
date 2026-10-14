import React from 'react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../ui/Dialog';

interface ConfirmRunDialogProps {
  open: boolean;
  commandText: string;
  rowId: string;
  onCancel: () => void;
  onConfirm: () => void;
}

/** D-03: the only UI path that sends `confirmed: true`. Cancel owns the initial focus. */
export function ConfirmRunDialog({ open, commandText, rowId, onCancel, onConfirm }: ConfirmRunDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(next) => { if (!next) onCancel(); }}>
      <DialogContent className="max-w-md">
        <DialogTitle>Run a destructive command?</DialogTitle>
        <DialogDescription>
          {commandText} is classified destructive ({rowId}). It can delete or overwrite data.
        </DialogDescription>
        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            autoFocus
            onClick={onCancel}
            className="rounded-sm border border-border-subtle px-3 py-1.5 text-[12px] text-text-secondary"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className="rounded-sm border border-(--color-status-fail) px-3 py-1.5 text-[12px] text-(--color-status-fail)"
          >
            Run anyway
          </button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
