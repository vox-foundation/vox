// RED stub: replaced in the GREEN commit.
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../ui/Dialog';
export function ShortcutSheet({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Dialog open={open} onOpenChange={(n) => { if (!n) onClose(); }}>
      <DialogContent><DialogTitle>Keyboard shortcuts</DialogTitle><DialogDescription>stub</DialogDescription></DialogContent>
    </Dialog>
  );
}
