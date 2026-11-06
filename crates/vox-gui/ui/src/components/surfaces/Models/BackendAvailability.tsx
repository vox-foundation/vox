import React from 'react';
import { Glass } from '../../ui/Glass';

export interface ProviderStatus {
  provider: string;
  key_present: boolean;
  is_local: boolean;
  local_reachable: boolean | null;
  local_models: string[];
}

/** Per-backend live status strip for the Models surface (B9). */
export function BackendAvailability({ statuses }: { statuses: ProviderStatus[] }) {
  if (statuses.length === 0) return null;
  return (
    <Glass className="p-4">
      <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">
        Backend availability
      </div>
      <div role="list" className="mt-2 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
        {statuses.map(s => {
          const live = s.is_local ? s.local_reachable === true : s.key_present;
          const label = s.is_local
            ? s.local_reachable === true
              ? `online · ${s.local_models.length} models`
              : s.local_reachable === false
                ? 'offline'
                : 'probing…'
            : s.key_present
              ? 'key configured'
              : 'no key';
          return (
            <div
              key={s.provider}
              role="listitem"
              aria-label={s.provider}
              title={s.provider}
              className="flex items-center gap-2 rounded-lg border border-border-subtle bg-bg-base/40 px-2.5 py-1.5 min-w-0"
            >
              <span
                aria-hidden
                className={`size-2 shrink-0 rounded-full ${live ? 'bg-emerald-400' : 'bg-zinc-600'}`}
              />
              <span className="font-mono text-[10px] text-text-primary truncate min-w-0">{s.provider}</span>
              <span className="ml-auto shrink-0 whitespace-nowrap text-[9px] uppercase tracking-widest text-text-muted">{label}</span>
            </div>
          );
        })}
      </div>
    </Glass>
  );
}
