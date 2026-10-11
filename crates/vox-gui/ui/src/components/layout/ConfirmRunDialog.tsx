import React from 'react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../ui/Dialog';

interface ConfirmRunDialogProps {
  open: boolean;
  commandText: string;
  rowId: string;
  onCancel: () => void;
  onConfirm: () => void;
}

/** RED stub: renders the shell only; buttons and wording land in the next commit. */
export function ConfirmRunDialog({ open, onCancel }: ConfirmRunDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(next) => { if (!next) onCancel(); }}>
      <DialogContent>
        <DialogTitle>Confirm</DialogTitle>
        <DialogDescription>pending</DialogDescription>
      </DialogContent>
    </Dialog>
  );
}
