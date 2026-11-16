import React from 'react';
import { BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Switch } from '../../ui/Switch';

export interface SourceMeta {
  enabled: boolean;
  costUsd: number;
  cadenceHours: number;
  tier: string;
}

export interface WatchlistPart {
  id: string;
  role: string;
  model: string;
  sources: string[];
  ids: Record<string, string>;
  target_usd?: number;
  condition?: string;
}

export interface PriceWatchConfig {
  _meta: { sources: Record<string, SourceMeta> };
  watchlist: WatchlistPart[];
}

interface MercatusInspectorPaneProps {
  cfg: PriceWatchConfig;
  selectedPart: WatchlistPart | null;
  cheapestCost: number;
  draftTargets?: Record<string, string>;
  onUpdateTargetUsd: (partId: string, raw: string) => void;
  onFlushTargetUsd?: (partId: string) => void;
  onToggleSource: (src: string, meta: SourceMeta, nextChecked: boolean) => void;
}

export function MercatusInspectorPane({
  cfg,
  selectedPart,
  cheapestCost,
  draftTargets,
  onUpdateTargetUsd,
  onFlushTargetUsd,
  onToggleSource,
}: MercatusInspectorPaneProps) {
  const sources = cfg._meta?.sources ?? {};
  const isFree =
    selectedPart != null &&
    ((selectedPart.target_usd != null && selectedPart.target_usd === 0) ||
      selectedPart.condition === 'free');
  const partTarget = typeof selectedPart?.target_usd === 'number' ? selectedPart.target_usd : null;
  const floorDelta =
    partTarget !== null && partTarget > 0 && cheapestCost > 0 ? partTarget - cheapestCost : 0;

  return (
    <BentoScrollRegion
      ariaLabel="Selected watchlist part and source configuration"
      data-testid="mercatus-inspector-pane"
      className="flex-1 pr-1"
    >
      <div className="flex flex-col gap-3">
        {selectedPart ? (
          <div className="rounded-xl border border-border-strong bg-bg-surface p-3.5 space-y-3">
            <div className="flex items-start justify-between gap-2">
              <div>
                <div className="font-display text-[10px] tracking-widest text-brass uppercase">
                  Selected Exchange Entry
                </div>
                <div className="font-mono text-xs font-semibold text-text-primary">
                  {`Entry: ${selectedPart.model || selectedPart.id}`}
                </div>
                <div className="font-mono text-[11px] text-text-muted">
                  {`Role: ${selectedPart.role} · ID: ${selectedPart.id}`}
                </div>
              </div>
              <RomanWaxSealBadge
                label={isFree ? 'FREE TIER' : 'PRICED'}
                tone={isFree ? 'emerald' : 'gold'}
              />
            </div>

            <div className="grid grid-cols-2 gap-2 rounded-lg border border-border-subtle bg-bg-elevated/60 p-2.5 font-mono text-[11px]">
              <div>
                <span className="block text-[10px] uppercase text-text-muted">Target Ceiling</span>
                <span className="font-semibold text-text-primary">
                  {partTarget !== null ? `$${partTarget.toFixed(2)}` : 'Unbounded'}
                </span>
              </div>
              <div>
                <span className="block text-[10px] uppercase text-text-muted">Delta vs Floor</span>
                <span className="font-semibold text-brass">
                  {isFree
                    ? '$0.00 (Free)'
                    : partTarget !== null
                      ? `+$${Math.max(0, floorDelta).toFixed(2)}`
                      : '—'}
                </span>
              </div>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border-subtle pt-2.5">
              <label
                htmlFor="mercatus-selected-target-input"
                className="font-display text-[10px] tracking-widest text-text-secondary uppercase"
              >
                Ceiling Threshold (USD)
              </label>
              <div className="inline-flex items-center gap-1 font-mono text-xs">
                <span className="text-text-muted">$</span>
                <input
                  id="mercatus-selected-target-input"
                  type="number"
                  min={0}
                  step="0.05"
                  data-testid="mercatus-selected-target-usd"
                  aria-label={`Inspector target ceiling for ${selectedPart.model || selectedPart.id}`}
                  value={
                    draftTargets && selectedPart.id in draftTargets
                      ? draftTargets[selectedPart.id]
                      : (selectedPart.target_usd ?? '')
                  }
                  placeholder="—"
                  onChange={(e) => onUpdateTargetUsd(selectedPart.id, e.target.value)}
                  onBlur={() => onFlushTargetUsd?.(selectedPart.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') onFlushTargetUsd?.(selectedPart.id);
                  }}
                  className="w-24 rounded border border-border-strong bg-bg-elevated px-2 py-1 font-mono text-[11px] text-text-primary focus:border-brass focus:outline-hidden"
                />
              </div>
            </div>

            {Object.keys(selectedPart.ids ?? {}).length > 0 && (
              <div className="border-t border-border-subtle pt-2 font-mono text-[10px] text-text-secondary">
                <div className="mb-1 text-text-muted uppercase">Pinned Provider Slugs</div>
                <div className="flex flex-wrap gap-1.5">
                  {Object.entries(selectedPart.ids).map(([provider, slug]) => (
                    <span
                      key={provider}
                      className="rounded border border-border-subtle bg-bg-elevated px-1.5 py-0.5"
                    >
                      {`${provider}: ${slug}`}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="rounded-xl border border-border-subtle bg-bg-surface p-3.5 text-xs text-text-muted">
            Select a hardware or model watchlist entry to inspect price deltas and provider slugs.
          </div>
        )}

        <details
          open
          className="rounded-xl border border-border-strong bg-bg-surface shadow-sm"
        >
          <summary className="cursor-pointer px-4 py-2.5 font-display text-[10px] uppercase tracking-widest text-brass hover:text-text-primary">
            Source registry ({Object.keys(sources).length})
          </summary>
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-[11px]">
              <thead>
                <tr className="border-b border-border-subtle bg-bg-elevated/80">
                  {['Source', 'Tier', 'Cost/run', 'Cadence', 'Enabled'].map((h) => (
                    <th
                      key={h}
                      className="px-3 py-1.5 text-left font-display text-[10px] uppercase tracking-widest text-brass"
                    >
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {Object.entries(sources).map(([src, meta]) => (
                  <tr
                    key={src}
                    className="border-b border-border-subtle last:border-0 hover:bg-overlay-hover transition-colors"
                  >
                    <td className="px-3 py-1.5 font-mono text-text-primary font-medium">{src}</td>
                    <td className="px-3 py-1.5 text-text-secondary">{meta.tier}</td>
                    <td className="px-3 py-1.5 font-mono text-text-primary">
                      ${meta.costUsd.toFixed(3)}
                    </td>
                    <td className="px-3 py-1.5 text-text-secondary">{meta.cadenceHours}h</td>
                    <td className="px-3 py-1.5">
                      <div className="inline-flex items-center gap-2">
                        <Switch
                          checked={Boolean(meta.enabled)}
                          data-testid={`mercatus-source-switch-${src}`}
                          aria-label={`Toggle source ${src}`}
                          onCheckedChange={(nextChecked) => onToggleSource(src, meta, nextChecked)}
                        />
                        {meta.enabled ? (
                          <RomanWaxSealBadge
                            label="ENABLED"
                            tone="emerald"
                            className="text-[8px] py-0 px-1.5"
                          />
                        ) : (
                          <span className="font-mono text-text-muted/60">no</span>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </details>

        <div className="rounded-xl border border-border-strong bg-bg-surface p-3.5">
          <div className="mb-2 flex items-center justify-between border-b border-border-subtle pb-2">
            <span className="font-display text-[10px] font-bold uppercase tracking-widest text-brass">
              Treasury Denarius &amp; Prompt Cache Settlement
            </span>
            <RomanWaxSealBadge label="TREASURY GUARD" tone="gold" />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div className="rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-2">
              <div className="font-display text-[9px] uppercase tracking-widest text-brass">
                Cache Hit Savings
              </div>
              <div className="mt-0.5 font-mono text-xs font-semibold text-text-primary">
                Up to 90% Off
              </div>
            </div>
            <div className="rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-2">
              <div className="font-display text-[9px] uppercase tracking-widest text-brass">
                Local MENS Cost
              </div>
              <div className="mt-0.5 font-mono text-xs font-semibold text-text-primary">
                $0.00 / 1M Tok
              </div>
            </div>
          </div>
        </div>
      </div>
    </BentoScrollRegion>
  );
}
