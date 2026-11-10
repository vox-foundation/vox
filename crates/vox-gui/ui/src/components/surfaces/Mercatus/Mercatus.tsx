import React, { useEffect, useState, useCallback } from 'react';
import { voxTransport } from '../../../transport';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useLabel } from '../../../hooks/useLanguage';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';

interface SourceMeta {
  enabled: boolean;
  costUsd: number;
  cadenceHours: number;
  tier: string;
}

interface WatchlistPart {
  id: string;
  role: string;
  model: string;
  sources: string[];
  ids: Record<string, string>;
  target_usd?: number;
  condition?: string;
}

interface PriceWatchConfig {
  _meta: { sources: Record<string, SourceMeta> };
  watchlist: WatchlistPart[];
}

type LoadState = 'loading' | 'ok' | 'error';

export function Mercatus({ condensed }: { condensed?: boolean }) {
  const navLabel = useLabel('mercatus');
  const [cfg, setCfg] = useState<PriceWatchConfig | null>(null);
  const [state, setState] = useState<LoadState>('loading');
  const [err, setErr] = useState('');

  const reload = useCallback(() => {
    setState('loading');
    voxTransport.mercatusLoadConfig()
      .then((c) => { setCfg(c as PriceWatchConfig); setState('ok'); })
      .catch((e) => { setErr(sanitizeErrorForToast(e)); setState('error'); });
  }, []);

  useEffect(() => { reload(); }, [reload]);

  const sources = cfg?._meta?.sources ?? {};
  const parts = cfg?.watchlist ?? [];
  const allSourceKeys = Object.keys(sources).filter(k => sources[k].enabled);

  const models = parts;
  const freeCount = models.filter(m => (m.target_usd != null && m.target_usd === 0) || m.condition === 'free').length;
  const costs = models.map(m => m.target_usd).filter((c): c is number => typeof c === 'number' && c > 0);
  const cheapestCost = costs.length > 0 ? Math.min(...costs) : 0;
  const cheapest = costs.length > 0;
  const synced = state === 'ok';

  if (condensed) {
    return (
      <section className="space-y-2 text-[11px] text-[#C5BCAC]">
        <h2 className="font-display text-xs text-[#F3E5AB] tracking-wider uppercase">{navLabel}</h2>
        {state === 'loading' && <div className="animate-pulse text-[#D4AF37]">Loading…</div>}
        {state === 'error' && <div className="text-[#FCA5A5]">{err}</div>}
        {state === 'ok' && <div>{parts.length} parts · {allSourceKeys.length} enabled sources</div>}
      </section>
    );
  }

  return (
    <section className="space-y-4">
      <RomanSurfaceHeader
        domainTag="MARKET · MERCATUS"
        title={`${navLabel} — Token & Compute Exchange Ledger`}
        subtitle="Real-time model pricing, context window capacity, free-tier availability, and budget burn telemetry."
        waxSealBadge={synced ? 'LIVE SYNC' : 'OFFLINE'}
        icon={<Icon.denariusCoin className="size-5 text-[#F3E5AB]" />}
        gauges={[
          { label: 'Catalog', valueText: `${models.length} Models`, ratio: Math.min(1, models.length / 20), tone: 'gold' },
          { label: 'Free Tiers', valueText: `${freeCount} Active`, ratio: Math.min(1, freeCount / Math.max(1, models.length)), tone: 'emerald' },
          { label: 'Floor Cost', valueText: cheapest ? `$${cheapestCost.toFixed(2)}/1M` : '$0.00', ratio: 0.85, tone: 'gold' },
        ]}
        actions={
          <button
            type="button"
            onClick={reload}
            className="rounded-lg border border-[#C59B27]/40 bg-bg-elevated px-3 py-1 font-display text-[10px] uppercase tracking-wider text-text-primary hover:bg-[#D4AF37]/20 transition-colors shadow-sm"
          >
            {state === 'loading' ? 'loading…' : 'refresh'}
          </button>
        }
      />

      {state === 'error' && (
        <div className="rounded-xl border border-dashed border-[#FCA5A5]/40 bg-[#7F1D1D]/15 px-4 py-3 text-[11px] text-[#FCA5A5]">
          {err}
        </div>
      )}

      {state === 'ok' && !cfg && (
        <div className="rounded-xl border border-[#C59B27]/35 bg-bg-surface p-6 text-center text-xs text-text-muted">
          No price-watch watchlist configured in this workspace yet.
        </div>
      )}

      {state === 'ok' && cfg && (
        <>
          {/* Coverage matrix */}
          <div className="overflow-x-auto rounded-xl border border-[#C59B27]/35 bg-bg-surface shadow-sm">
            <table className="w-full border-collapse text-[11px]">
              <thead>
                <tr className="border-b border-[#C59B27]/25 bg-bg-elevated/80">
                  <th className="px-3 py-2 text-left font-display text-[10px] uppercase tracking-widest text-[#D4AF37]">
                    Part
                  </th>
                  <th className="px-3 py-2 text-left font-display text-[10px] uppercase tracking-widest text-[#D4AF37]">
                    Role
                  </th>
                  <th className="px-3 py-2 text-left font-display text-[10px] uppercase tracking-widest text-[#D4AF37]">
                    Target
                  </th>
                  {allSourceKeys.map((src) => (
                    <th
                      key={src}
                      className="px-2 py-2 text-center font-display text-[10px] uppercase tracking-widest text-[#D4AF37]"
                    >
                      {src}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {parts.map((part, i) => {
                  return (
                    <tr
                      key={part.id}
                      className={`border-b border-[#D4AF37]/15 last:border-0 hover:bg-[#D4AF37]/5 transition-colors ${
                        i % 2 === 0 ? '' : 'bg-bg-elevated/40'
                      }`}
                    >
                      <td className="px-3 py-1.5 font-mono text-[11px] text-text-primary font-medium">
                        {part.model || part.id}
                      </td>
                      <td className="px-3 py-1.5 text-text-secondary">{part.role}</td>
                      <td className="px-3 py-1.5 font-mono text-text-primary">
                        {part.target_usd != null ? `$${part.target_usd}` : '—'}
                      </td>
                      {allSourceKeys.map((src) => {
                        const hasId = !!(part.ids ?? {})[src];
                        const inSources = (part.sources ?? []).includes(src);
                        const dot = hasId
                          ? '●'
                          : inSources
                          ? '○'
                          : '·';
                        const cls = hasId
                          ? 'text-[#E8BF4C]'
                          : inSources
                          ? 'text-[#D4AF37]/70'
                          : 'text-[#8C806B]/30';
                        return (
                          <td
                            key={src}
                            className={`px-2 py-1.5 text-center font-mono ${cls}`}
                            title={hasId ? `id: ${part.ids[src]}` : inSources ? 'in sources[], no id' : 'not tracked'}
                          >
                            {dot}
                          </td>
                        );
                      })}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <div className="text-[10px] text-[#8C806B] flex flex-wrap gap-4 items-center">
            <span><span className="font-mono text-[#E8BF4C]">●</span> id pinned</span>
            <span><span className="font-mono text-[#D4AF37]/70">○</span> in sources[], no id</span>
            <span><span className="font-mono text-[#8C806B]/30">·</span> not tracked</span>
            <span className="ml-auto font-display text-[9px] uppercase tracking-wider text-[#A69B89]">{parts.length} parts · {allSourceKeys.length} enabled sources</span>
          </div>

          {/* Source cost table */}
          <details open className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface shadow-[0_4px_16px_rgba(0,0,0,0.2)]">
            <summary className="cursor-pointer px-4 py-2.5 font-display text-[10px] uppercase tracking-widest text-brass hover:text-text-primary">
              Source registry ({Object.keys(sources).length})
            </summary>
            <div className="overflow-x-auto">
              <table className="w-full border-collapse text-[11px]">
                <thead>
                  <tr className="border-b border-[#C59B27]/25 bg-bg-elevated/80">
                    {['Source', 'Tier', 'Cost/run', 'Cadence', 'Enabled'].map(h => (
                      <th key={h} className="px-3 py-1.5 text-left font-display text-[10px] uppercase tracking-widest text-brass">{h}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {Object.entries(sources).map(([src, meta]) => (
                    <tr key={src} className="border-b border-[#D4AF37]/15 last:border-0 hover:bg-[#D4AF37]/5 transition-colors">
                      <td className="px-3 py-1.5 font-mono text-text-primary font-medium">{src}</td>
                      <td className="px-3 py-1.5 text-text-secondary">{meta.tier}</td>
                      <td className="px-3 py-1.5 font-mono text-text-primary">${meta.costUsd.toFixed(3)}</td>
                      <td className="px-3 py-1.5 text-text-secondary">{meta.cadenceHours}h</td>
                      <td className="px-3 py-1.5">
                        {meta.enabled ? (
                          <RomanWaxSealBadge label="ENABLED" tone="emerald" className="text-[8px] py-0 px-1.5" />
                        ) : (
                          <span className="font-mono text-[#8C806B]/50">no</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </details>

          <div
            data-testid="mercatus-arbitrage-deck"
            className="grid grid-cols-1 gap-3 sm:grid-cols-3"
          >
            <div
              style={{
                backgroundImage:
                  'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
                backgroundSize: 'cover',
                backgroundPosition: 'center',
              }}
              className="rounded-xl border-2 border-[#C59B27] p-4 text-[#1C140C] shadow-[0_6px_16px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.92)]"
            >
              <div className="flex items-center justify-between border-b border-[#9E7B2F]/45 pb-1.5">
                <span className="font-display text-[10.5px] font-bold tracking-widest text-[#1C140C] uppercase">Spot GPU Arbitrage</span>
                <RomanWaxSealBadge label="QLoRA Ready" tone="gold" />
              </div>
              <p className="mt-2 text-[11px] font-medium leading-relaxed text-[#2B2013]">
                Tracks Vast.ai &amp; RunPod spot instance pricing against target hourly ceilings for MENS 24GB/48GB training runs.
              </p>
            </div>
            <div
              style={{
                backgroundImage:
                  'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
                backgroundSize: 'cover',
                backgroundPosition: 'center',
              }}
              className="rounded-xl border-2 border-[#C59B27] p-4 text-[#1C140C] shadow-[0_6px_16px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.92)]"
            >
              <div className="flex items-center justify-between border-b border-[#9E7B2F]/45 pb-1.5">
                <span className="font-display text-[10.5px] font-bold tracking-widest text-[#1C140C] uppercase">Token Cost Band</span>
                <RomanWaxSealBadge label="Live Sync" tone="gold" />
              </div>
              <p className="mt-2 text-[11px] font-medium leading-relaxed text-[#2B2013]">
                Feeds OpenRouter &amp; frontier provider price-per-million shifts directly into <code className="rounded bg-bg-elevated px-1 py-0.5 text-text-primary">vox-orchestrator</code> model routing.
              </p>
            </div>
            <div
              style={{
                backgroundImage:
                  'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
                backgroundSize: 'cover',
                backgroundPosition: 'center',
              }}
              className="rounded-xl border-2 border-[#C59B27] p-4 text-[#1C140C] shadow-[0_6px_16px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.92)]"
            >
              <div className="flex items-center justify-between border-b border-[#9E7B2F]/45 pb-1.5">
                <span className="font-display text-[10.5px] font-bold tracking-widest text-[#1C140C] uppercase">Probe Cadence</span>
                <RomanWaxSealBadge label={`${allSourceKeys.length} ACTIVE FEEDS`} tone="emerald" />
              </div>
              <p className="mt-2 text-[11px] font-medium leading-relaxed text-[#2B2013]">
                Automated background sweeps respect per-source cadence windows and daily probe cost caps.
              </p>
            </div>
          </div>

          <div
            style={{
              backgroundImage:
                'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
              backgroundSize: 'cover',
              backgroundPosition: 'center',
            }}
            className="rounded-xl border-2 border-[#C59B27] p-4 text-[#1C140C] shadow-[0_6px_18px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)]"
          >
            <div className="mb-2 flex items-center justify-between border-b border-[#9E7B2F]/45 pb-2">
              <span className="font-display text-xs font-bold uppercase tracking-[0.14em] text-[#1C140C]">
                Treasury Denarius &amp; Prompt Cache Settlement Ledger
              </span>
              <RomanWaxSealBadge label="TREASURY GUARD" tone="gold" />
            </div>
            <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
              <div className="rounded-lg border border-[#9E7B2F] bg-bg-surface px-3 py-2 text-text-primary">
                <div className="font-display text-[9px] uppercase tracking-widest text-brass">Cache Hit Savings</div>
                <div className="mt-0.5 font-mono text-xs font-semibold text-text-primary">Up to 90% Off</div>
              </div>
              <div className="rounded-lg border border-[#9E7B2F] bg-bg-surface px-3 py-2 text-text-primary">
                <div className="font-display text-[9px] uppercase tracking-widest text-brass">Local MENS Cost</div>
                <div className="mt-0.5 font-mono text-xs font-semibold text-text-primary">$0.00 / 1M Tok</div>
              </div>
              <div className="rounded-lg border border-[#9E7B2F] bg-bg-surface px-3 py-2 text-text-primary">
                <div className="font-display text-[9px] uppercase tracking-widest text-brass">Budget Enforcement</div>
                <div className="mt-0.5 font-mono text-xs font-semibold text-text-primary">Hard Cap + Toast</div>
              </div>
              <div className="rounded-lg border border-[#9E7B2F] bg-bg-surface px-3 py-2 text-text-primary">
                <div className="font-display text-[9px] uppercase tracking-widest text-brass">Settlement Currency</div>
                <div className="mt-0.5 font-mono text-xs font-semibold text-text-primary">USD · Denarius</div>
              </div>
            </div>
          </div>
        </>
      )}

      {state === 'loading' && (
        <div className="text-[11px] text-[#A69B89] animate-pulse">Loading config…</div>
      )}
    </section>
  );
}
