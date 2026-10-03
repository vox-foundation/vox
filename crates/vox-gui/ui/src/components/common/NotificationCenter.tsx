import React, { useEffect, useRef, useState } from 'react';
import { Icon } from '../ui/Icons';
import { unreadProblems, type Notice } from '../../lib/noticeStore';

const SEVERITY_COLOR: Record<Notice['severity'], string> = {
  error: 'var(--color-status-fail)',
  warning: 'var(--color-status-warn)',
  success: 'var(--color-status-pass)',
  info: 'var(--color-status-info)',
};

interface Props {
  notices: Notice[];
  onMarkAllRead(): void;
}

/**
 * The notification bell and its drawer (docs/src/architecture/gui-observability-ssot-2026.md).
 * Popover behaviour — outside click, document-level Escape, focus return — mirrors StatusBarCluster.
 */
export function NotificationCenter({ notices, onMarkAllRead }: Props) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState<'all' | 'problems'>('all');
  const panelRef = useRef<HTMLElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const unread = unreadProblems(notices);
  const unreadError = notices.some(n => !n.read && n.severity === 'error');

  useEffect(() => {
    if (!open) return;
    panelRef.current?.focus();
    const handleOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (panelRef.current?.contains(target) || triggerRef.current?.contains(target)) return;
      setOpen(false);
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener('mousedown', handleOutside);
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('mousedown', handleOutside);
      document.removeEventListener('keydown', handleKey);
    };
  }, [open]);

  const shown = filter === 'all' ? notices : notices.filter(n => n.severity === 'warning' || n.severity === 'error');
  return (
    <div className="relative inline-flex items-center">
      <button
        ref={triggerRef}
        type="button"
        onClick={() => setOpen(o => !o)}
        aria-expanded={open}
        aria-controls="notification-center"
        aria-label={unread > 0 ? `Notifications, ${unread} need attention` : 'Notifications'}
        className="flex items-center gap-1 px-2 text-text-muted hover:text-text-primary"
      >
        <Icon.bell className="size-3.5" aria-hidden="true" />
        {/* The slot is always rendered at a fixed width so the bar does not shift when the count appears. */}
        <span data-slot="count" data-testid={unread > 0 ? 'notification-unread' : undefined}
          className="min-w-[3ch] font-mono text-[11px] tabular-nums"
          style={{ color: unreadError ? 'var(--color-status-fail)' : 'var(--color-status-warn)' }}>
          {unread > 0 ? unread : ''}
        </span>
      </button>
      {/* Engine problems never toast, so announce the count politely for screen-reader users. */}
      <span className="sr-only" aria-live="polite">{unread > 0 ? `${unread} notifications need attention` : ''}</span>
      {open && (
        <section
          ref={panelRef}
          id="notification-center"
          role="dialog"
          aria-label="Notifications"
          tabIndex={-1}
          className="absolute bottom-full right-0 z-50 mb-1 flex max-h-[60vh] w-[360px] max-w-[calc(100vw-2rem)] flex-col rounded-xl border border-border-subtle bg-bg-base p-3 text-xs shadow-2xl"
        >
          <header className="mb-2 flex items-center gap-2">
            <span className="flex-1 text-text-secondary">Notifications</span>
            <span role="radiogroup" aria-label="Show" className="flex gap-1">
              {(['all', 'problems'] as const).map(v => (
                <label key={v} className="flex items-center gap-1 text-text-muted">
                  <input type="radio" name="notice-filter" checked={filter === v} onChange={() => setFilter(v)} />
                  {v === 'all' ? 'All' : 'Problems'}
                </label>
              ))}
            </span>
            <button type="button" onClick={onMarkAllRead} className="text-text-muted hover:text-text-primary">
              Mark all read
            </button>
          </header>
          {shown.length === 0 ? (
            <p className="py-6 text-center text-text-muted">Nothing to report</p>
          ) : (
            <ul className="flex min-h-0 flex-col gap-1 overflow-y-auto">
              {shown.map(n => (
                <li key={n.id} data-severity={n.severity} data-read={n.read ? 'true' : undefined}
                  className="rounded-lg border-l-2 px-2 py-1"
                  style={{ borderColor: SEVERITY_COLOR[n.severity] }}>
                  <div className="flex items-baseline gap-1">
                    <span className={n.read ? 'text-text-muted' : 'text-text-primary'}>{n.title}</span>
                    {n.count > 1 && <span className="font-mono text-[11px] text-text-muted">×{n.count}</span>}
                    <span className="ml-auto text-[11px] text-text-muted">{n.source}</span>
                  </div>
                  {n.body && <div className="break-words text-text-muted">{n.body}</div>}
                  {n.cmd && <div className="font-mono text-[11px] text-text-muted">▸ {n.cmd}</div>}
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </div>
  );
}
