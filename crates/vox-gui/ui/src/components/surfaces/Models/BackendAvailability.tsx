import React from 'react';
import { Glass } from '../../ui/Glass';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';

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
    <Glass
      style={{
        backgroundImage:
          'linear-gradient(180deg, rgba(248,243,233,0.95) 0%, rgba(235,224,205,0.94) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}
      className="p-4 border-2 border-[#C59B27] text-[#1C140C] shadow-[0_6px_18px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.9)]"
    >
      <div className="flex items-center justify-between border-b border-[#9E7B2F]/45 pb-2">
        <div className="font-display text-[11px] font-bold tracking-[0.2em] uppercase text-[#1C140C]">
          Backend availability
        </div>
        <RomanWaxSealBadge label="MONITORED" tone="gold" />
      </div>
      <div role="list" className="mt-2.5 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5">
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
              className="flex items-center gap-2.5 rounded-lg border border-[#9E7B2F] bg-bg-surface px-3 py-2 min-w-0 shadow-sm transition-colors hover:border-[#D4AF37]"
            >
              <span
                aria-hidden
                className={`size-2 shrink-0 rounded-full ${
                  live
                    ? 'bg-[#8FA339] shadow-[0_0_8px_rgba(143,163,57,0.8)]'
                    : 'bg-[#78350F]'
                }`}
              />
              <span className="font-mono text-[10px] text-text-primary font-medium truncate min-w-0">{s.provider}</span>
              <span className="ml-auto shrink-0 whitespace-nowrap text-[9px] uppercase tracking-widest text-text-secondary">
                {live ? (
                  <RomanWaxSealBadge label={label} tone="emerald" className="text-[8px] py-0 px-1.5" />
                ) : (
                  <span className="font-display text-text-secondary">{label}</span>
                )}
              </span>
            </div>
          );
        })}
      </div>
    </Glass>
  );
}
