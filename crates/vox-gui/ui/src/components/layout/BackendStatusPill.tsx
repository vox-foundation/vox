import React from 'react';
import { useBackendStatus } from '../../hooks/useBackendStatus';

export function BackendStatusPill({ collapsed = false }: { collapsed?: boolean }) {
  const { status } = useBackendStatus();
  if (status.mode === 'local') return null;
  let host = 'server';
  try {
    if (status.url) host = new URL(status.url).host;
  } catch {
    host = status.url ?? 'server';
  }
  const profiles = status.profile?.profiles.join(', ') ?? '';
  const label = `Connected to ${host}${profiles ? `, profile ${profiles}` : ''}`;
  return (
    <div
      data-testid="backend-status-pill"
      role="status"
      aria-label={label}
      title={label}
      className={`flex items-center ${collapsed ? 'justify-center' : 'gap-1.5 px-2'} rounded-md border border-(--color-status-pass)/40 bg-(--color-status-pass)/10 py-1 font-mono text-[10px] font-semibold text-text-secondary`}
    >
      <span className="size-1.5 shrink-0 rounded-full bg-(--color-status-pass) ring-1 ring-[#D4AF37]/80" aria-hidden="true" />
      {!collapsed && <span className="truncate">{host}</span>}
      {!collapsed && profiles && <span className="truncate text-text-muted">· {profiles}</span>}
    </div>
  );
}
