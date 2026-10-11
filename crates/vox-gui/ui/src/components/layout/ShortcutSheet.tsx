import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../ui/Dialog';
import { ACTION_REGISTRY, DEFAULT_BINDINGS, formatChord, isMacPlatform } from '../../lib/keybinds';
import { useKeybindings } from '../../hooks/useKeybinds';

interface ShortcutSheetProps { open: boolean; onClose: () => void }

/** D-11/D-12: generated from ACTION_REGISTRY + the live bindings (D-17), never a hand list. */
export function ShortcutSheet({ open, onClose }: ShortcutSheetProps) {
  const bindings = useKeybindings();
  const isMac = isMacPlatform();
  return (
    <Dialog open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <DialogContent className="max-w-md">
        <DialogTitle>Keyboard shortcuts</DialogTitle>
        <DialogDescription>
          Press ? anywhere outside a text field, or {formatChord(bindings['show-shortcuts'] ?? DEFAULT_BINDINGS['show-shortcuts'], isMac)}. Rebind in Settings → Keybinds.
        </DialogDescription>
        <ul className="mt-3 space-y-1.5">
          {ACTION_REGISTRY.map(a => (
            <li
              key={a.id}
              data-testid={`shortcut-row-${a.id}`}
              className="flex items-center justify-between rounded-md border border-border-subtle bg-overlay-subtle px-3 py-2"
            >
              <span className="text-[12px] text-text-secondary">{a.label}</span>
              <kbd className="rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 font-mono text-[10px] text-text-secondary">
                {formatChord(bindings[a.id] ?? DEFAULT_BINDINGS[a.id], isMac)}
              </kbd>
            </li>
          ))}
        </ul>
      </DialogContent>
    </Dialog>
  );
}
