import React, { useEffect, useMemo, useRef } from 'react';
import type { CapabilityIndexEntry } from '../../generated/capabilityIndex.generated';
import type { Toast } from '../../types/tauri';
import { buildExplainCard } from '../../lib/capabilityCard';
import { Glass } from '../ui/Glass';
import { Icon } from '../ui/Icons';

export interface ExplainDrawerProps {
  row: CapabilityIndexEntry | null;
  rows: readonly CapabilityIndexEntry[];
  domains: readonly { id: string; label: string }[];
  catalogPaths: ReadonlySet<string>;
  coverageAsOf?: { sha: string; date: string } | null;
  onClose: () => void;
  onOpenForm: (path: string[]) => void;
  onOpenDoc: (path: string) => void;
  pushToast: (t: Toast) => void;
}

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mb-4">
      <h3 className="mb-1 font-display text-[11px] uppercase tracking-[0.14em] text-text-muted">{title}</h3>
      <dl className="grid grid-cols-[7rem_1fr] gap-x-3 gap-y-1 text-[13px] text-text-primary">{children}</dl>
    </section>
  );
}

function Item({ label, children }: { label?: string; children: React.ReactNode }) {
  return (
    <>
      <dt className="text-text-muted">{label ?? ''}</dt>
      <dd className="min-w-0 break-words">{children}</dd>
    </>
  );
}

const actionButton =
  'rounded-md border border-border-subtle px-3 py-1.5 text-[12px] text-text-primary hover:bg-overlay-hover disabled:cursor-not-allowed disabled:opacity-50';

export function ExplainDrawer({
  row,
  rows,
  domains,
  catalogPaths,
  coverageAsOf = null,
  onClose,
  onOpenForm,
  onOpenDoc,
  pushToast,
}: ExplainDrawerProps) {
  const panelRef = useRef<HTMLDivElement | null>(null);
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);

  const card = useMemo(
    () => (row ? buildExplainCard(row, rows, domains, catalogPaths, coverageAsOf) : null),
    [row, rows, domains, catalogPaths, coverageAsOf],
  );

  useEffect(() => {
    if (!row) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [row, onClose]);

  // Modal focus behavior (same as DocViewerDrawer): focus in on open, trap Tab, restore on close.
  useEffect(() => {
    if (!row) return;
    previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
    closeButtonRef.current?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;
      const panel = panelRef.current;
      if (!panel) return;
      const focusable = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (el) => el.offsetParent !== null || el === document.activeElement,
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement;
      if (e.shiftKey) {
        if (active === first || !panel.contains(active)) {
          e.preventDefault();
          last.focus();
        }
      } else if (active === last || !panel.contains(active)) {
        e.preventDefault();
        first.focus();
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      previouslyFocusedRef.current?.focus();
      previouslyFocusedRef.current = null;
    };
  }, [row]);

  if (!card) return null;
  const { actions } = card;

  const copyMcp = async () => {
    const writeText = navigator.clipboard?.writeText?.bind(navigator.clipboard);
    if (!writeText || !actions.copyMcp) {
      pushToast({ tone: 'warn', title: 'Copy failed', body: 'clipboard unavailable', cause: 'backend-error' });
      return;
    }
    try {
      await writeText(actions.copyMcp);
      pushToast({ tone: 'ok', title: 'Copied', body: `${actions.copyMcp} to clipboard`, cause: 'clipboard' });
    } catch {
      pushToast({ tone: 'warn', title: 'Copy failed', cause: 'backend-error' });
    }
  };

  const { equivalents: eq, location: loc, status: st } = card;
  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <button type="button" aria-label="Close explain card" className="flex-1 bg-black/50" onClick={onClose} />
      <Glass
        role="dialog"
        aria-label={card.title}
        aria-modal="true"
        className="flex h-full w-full max-w-xl flex-col rounded-none border-l border-border-subtle shadow-2xl"
        inset={false}
      >
        <div ref={panelRef} className="flex h-full min-h-0 w-full flex-col">
          <div className="flex items-start justify-between gap-3 border-b border-border-subtle px-4 py-3">
            <div className="min-w-0">
              <div className="font-display text-[13px] uppercase tracking-[0.14em] text-text-primary">{card.title}</div>
              <div className="mt-0.5 text-[11px] text-text-muted">
                {card.kind} · {card.id}
              </div>
              {card.summary && <p className="mt-2 text-[13px] text-text-primary">{card.summary}</p>}
            </div>
            <button
              ref={closeButtonRef}
              type="button"
              aria-label="Close explain"
              onClick={onClose}
              className="flex size-7 shrink-0 items-center justify-center rounded-md text-text-muted hover:bg-overlay-hover hover:text-text-primary"
            >
              <Icon.x className="size-4" aria-hidden="true" />
            </button>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto p-4">
            <div className="mb-4 flex flex-wrap items-center gap-2">
              <button
                type="button"
                className={actionButton}
                disabled={!actions.openForm || !!actions.openForm.unavailable}
                onClick={() => actions.openForm && onOpenForm(actions.openForm.path)}
              >
                Open form
              </button>
              {actions.openForm?.unavailable && (
                <span className="text-[11px] text-text-muted">{actions.openForm.unavailable}</span>
              )}
              {actions.copyMcp && (
                <button type="button" className={actionButton} onClick={copyMcp}>
                  Copy MCP name
                </button>
              )}
              {actions.openDocs && (
                <button type="button" className={actionButton} onClick={() => onOpenDoc(actions.openDocs!)}>
                  Open docs
                </button>
              )}
            </div>
            <Group title="Equivalents">
              <Item label="CLI">{eq.cli.join(', ')}</Item>
              <Item label="MCP">{eq.mcp.join(', ')}</Item>
              <Item label="GUI">{eq.gui}</Item>
              <Item label="Catalog">{eq.catalog}</Item>
            </Group>
            <Group title="Location">
              <Item label="Crate">{loc.crate}</Item>
              <Item label="Source">{loc.source}</Item>
              <Item label="Docs">{loc.docs}</Item>
            </Group>
            <Group title="Status">
              <Item label="Coverage">{st.coverage}</Item>
              <Item label="Tier">{st.tier}</Item>
              <Item label="Side effects">{st.sideEffects}</Item>
              <Item label="Domain">{st.domain}</Item>
            </Group>
            <Group title="Related">
              {card.related.length === 0 ? (
                <Item>{'—'}</Item>
              ) : (
                card.related.map((r) => (
                  <Item key={r.id} label={r.id}>
                    {r.title}
                  </Item>
                ))
              )}
            </Group>
          </div>
        </div>
      </Glass>
    </div>
  );
}
