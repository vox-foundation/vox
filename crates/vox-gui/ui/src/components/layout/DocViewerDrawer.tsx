import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../ui/Glass';
import { Icon } from '../ui/Icons';
import { DocReader } from '../surfaces/DocReader/DocReader';
import type { ActiveDoc } from '../../hooks/useDocViewer';

export interface DocViewerDrawerProps {
  doc: ActiveDoc | null;
  onClose: () => void;
}

type DockWidthMode = 'split' | 'wide' | 'full';

const WIDTH_CLASS: Record<DockWidthMode, string> = {
  split: 'w-[min(46rem,92vw)]',
  wide: 'w-[min(64rem,94vw)]',
  full: 'w-[min(86rem,98vw)]',
};

const WIDTH_LABEL: Record<DockWidthMode, string> = {
  split: 'Width: Split',
  wide: 'Width: Wide',
  full: 'Width: Full',
};

export function DocViewerDrawer({ doc, onClose }: DocViewerDrawerProps) {
  const panelRef = useRef<HTMLDivElement | null>(null);
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const [widthMode, setWidthMode] = useState<DockWidthMode>('split');
  const [minimized, setMinimized] = useState(false);

  useEffect(() => {
    if (!doc) return;
    setMinimized(false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [doc, onClose]);

  // Non-modal dock focus ergonomics: move focus into the pane on open and restore
  // focus on close without trapping Tab/pointer access to the rest of the workspace.
  useEffect(() => {
    if (!doc) return;
    previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
    closeButtonRef.current?.focus();

    return () => {
      previouslyFocusedRef.current?.focus();
      previouslyFocusedRef.current = null;
    };
  }, [doc]);

  if (!doc) return null;

  const cycleWidth = () => {
    const order: DockWidthMode[] = ['split', 'wide', 'full'];
    setWidthMode((prev) => order[(order.indexOf(prev) + 1) % order.length]!);
  };

  return (
    <div
      className={`fixed right-2.5 bottom-9 z-30 flex max-w-[98vw] flex-col pointer-events-none ${
        minimized ? 'w-80' : `top-12 ${WIDTH_CLASS[widthMode]}`
      }`}
    >
      <Glass
        role="dialog"
        aria-label={doc.title}
        aria-modal="false"
        data-testid="doc-viewer-dock"
        className="pointer-events-auto flex h-full w-full flex-col rounded-xl border border-(--color-gold-500)/60 shadow-2xl overflow-hidden"
        inset={false}
      >
        <div ref={panelRef} className="flex h-full min-h-0 w-full flex-col">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-strong bg-bg-elevated px-3.5 py-2">
            <div className="min-w-0 flex items-center gap-2">
              <span className="font-display text-xs font-bold uppercase tracking-[0.14em] text-text-primary truncate">
                {doc.title}
              </span>
              <span className="hidden sm:inline rounded border border-border-subtle bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] text-text-muted">
                Docked · Non-Modal
              </span>
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              {!minimized && (
                <button
                  type="button"
                  data-slot="button"
                  data-roman-plaque="true"
                  data-testid="doc-dock-width-btn"
                  title="Cycle docked width (Split -> Wide -> Full)"
                  onClick={cycleWidth}
                  className="vox-brass-plaque-btn inline-flex h-7 items-center rounded px-2 text-[11px] transition-colors"
                >
                  <span>{WIDTH_LABEL[widthMode]}</span>
                </button>
              )}
              <button
                type="button"
                data-slot="button"
                data-roman-plaque="true"
                data-testid="doc-dock-minimize-btn"
                title={minimized ? 'Expand document workspace' : 'Collapse dock bar to bottom corner'}
                onClick={() => setMinimized((m) => !m)}
                className="vox-brass-plaque-btn inline-flex h-7 items-center rounded px-2 text-[11px] transition-colors"
              >
                <span>{minimized ? 'Expand' : 'Minimize'}</span>
              </button>
              <button
                ref={closeButtonRef}
                type="button"
                data-slot="button"
                aria-label="Close doc"
                onClick={onClose}
                className="flex size-7 items-center justify-center rounded-md border border-border-subtle bg-bg-surface text-text-muted hover:border-brass hover:text-text-primary transition-colors"
              >
                <Icon.x className="size-4" aria-hidden="true" />
              </button>
            </div>
          </div>
          {!minimized && (
            <div className="min-h-0 flex-1 overflow-y-auto p-3">
              <DocReader
                path={doc.path}
                initialContent={doc.initialContent}
                initialMode={doc.initialMode}
              />
            </div>
          )}
        </div>
      </Glass>
    </div>
  );
}
