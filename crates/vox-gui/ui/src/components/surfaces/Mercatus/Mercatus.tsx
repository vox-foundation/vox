import React, { useEffect, useState, useCallback, useRef } from 'react';
import { voxTransport } from '../../../transport';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useLabel } from '../../../hooks/useLanguage';
import { BentoCard, BentoGrid, BentoScrollRegion, MasterDetailWorkbench } from '../../ui/BentoGrid';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';
import {
  MercatusInspectorPane,
  type PriceWatchConfig,
  type SourceMeta,
} from './MercatusInspectorPane';

type LoadState = 'loading' | 'ok' | 'error';
type MercatusPartFilter = 'all' | 'free' | 'priced';

const TARGET_USD_DEBOUNCE_MS = 250;

function parseTargetUsdDraft(raw: string): { valid: true; value: number | undefined } | { valid: false } {
  if (raw.trim() === '') return { valid: true, value: undefined };
  const n = Number(raw);
  return Number.isFinite(n) && n >= 0 ? { valid: true, value: n } : { valid: false };
}

export function Mercatus({ condensed }: { condensed?: boolean }) {
  const navLabel = useLabel('mercatus');
  const [cfg, setCfg] = useState<PriceWatchConfig | null>(null);
  const [draftTargets, setDraftTargets] = useState<Record<string, string>>({});
  const [state, setState] = useState<LoadState>('loading');
  const [err, setErr] = useState('');
  const [partFilter, setPartFilter] = useState<MercatusPartFilter>('all');
  const [selectedPartId, setSelectedPartId] = useState<string | null>(null);

  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);
  const writeSeqRef = useRef(0);
  const confirmedSeqRef = useRef(0);
  const confirmedCfgRef = useRef<PriceWatchConfig | null>(null);
  const latestCfgRef = useRef<PriceWatchConfig | null>(null);
  const draftTargetsRef = useRef<Record<string, string>>({});
  const debounceTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const reload = useCallback(() => {
    const reqId = ++loadSeqRef.current;
    setState('loading');
    setErr('');
    voxTransport.mercatusLoadConfig()
      .then((c) => {
        if (!mountedRef.current || reqId !== loadSeqRef.current) return;
        const loaded = (c as PriceWatchConfig) ?? null;
        confirmedCfgRef.current = loaded;
        latestCfgRef.current = loaded;
        draftTargetsRef.current = {};
        setDraftTargets({});
        setCfg(loaded);
        setState('ok');
      })
      .catch((e) => {
        if (!mountedRef.current || reqId !== loadSeqRef.current) return;
        setErr(sanitizeErrorForToast(e));
        setState('error');
      });
  }, []);

  const persistConfig = useCallback((nextCfg: PriceWatchConfig) => {
    const reqId = ++writeSeqRef.current;
    latestCfgRef.current = nextCfg;
    setErr('');
    setCfg(nextCfg);
    void voxTransport
      .mercatusSaveConfig(nextCfg)
      .then(() => {
        if (reqId > confirmedSeqRef.current) {
          confirmedSeqRef.current = reqId;
          confirmedCfgRef.current = nextCfg;
        }
      })
      .catch((e: unknown) => {
        if (reqId !== writeSeqRef.current || !mountedRef.current) return;
        latestCfgRef.current = confirmedCfgRef.current;
        setCfg(confirmedCfgRef.current);
        setErr(sanitizeErrorForToast(e));
      });
  }, []);

  const flushTargetUsd = useCallback(
    (partId: string) => {
      const timer = debounceTimersRef.current.get(partId);
      if (timer !== undefined) {
        clearTimeout(timer);
        debounceTimersRef.current.delete(partId);
      }
      if (!(partId in draftTargetsRef.current)) return;
      const raw = draftTargetsRef.current[partId];
      const { [partId]: _discarded, ...nextDrafts } = draftTargetsRef.current;
      draftTargetsRef.current = nextDrafts;
      if (mountedRef.current) {
        setDraftTargets(nextDrafts);
      }
      const base = latestCfgRef.current;
      const parsed = parseTargetUsdDraft(raw);
      if (!base || !parsed.valid) return;
      const currentVal = base.watchlist.find((p) => p.id === partId)?.target_usd;
      if (currentVal === parsed.value) return;
      const nextCfg: PriceWatchConfig = {
        ...base,
        watchlist: base.watchlist.map((p) => (p.id === partId ? { ...p, target_usd: parsed.value } : p)),
      };
      if (mountedRef.current) {
        persistConfig(nextCfg);
      } else {
        latestCfgRef.current = nextCfg;
        void voxTransport.mercatusSaveConfig(nextCfg).catch(() => {});
      }
    },
    [persistConfig],
  );

  useEffect(() => {
    const timers = debounceTimersRef.current;
    mountedRef.current = true;
    reload();
    return () => {
      mountedRef.current = false;
      for (const timer of timers.values()) {
        clearTimeout(timer);
      }
      timers.clear();
      for (const partId of Object.keys(draftTargetsRef.current)) {
        flushTargetUsd(partId);
      }
    };
  }, [reload, flushTargetUsd]);

  const handleUpdateTargetUsd = useCallback(
    (partId: string, raw: string) => {
      const nextDrafts = { ...draftTargetsRef.current, [partId]: raw };
      draftTargetsRef.current = nextDrafts;
      setDraftTargets(nextDrafts);
      const prevTimer = debounceTimersRef.current.get(partId);
      if (prevTimer !== undefined) {
        clearTimeout(prevTimer);
      }
      debounceTimersRef.current.set(
        partId,
        setTimeout(() => {
          flushTargetUsd(partId);
        }, TARGET_USD_DEBOUNCE_MS),
      );
    },
    [flushTargetUsd],
  );

  const handleToggleSource = useCallback(
    (src: string, meta: SourceMeta, nextChecked: boolean) => {
      const base = latestCfgRef.current;
      if (!base) return;
      const nextCfg: PriceWatchConfig = {
        ...base,
        _meta: {
          ...base._meta,
          sources: {
            ...base._meta.sources,
            [src]: { ...meta, enabled: nextChecked },
          },
        },
      };
      persistConfig(nextCfg);
    },
    [persistConfig],
  );

  const sources = cfg?._meta?.sources ?? {};
  const parts = cfg?.watchlist ?? [];
  const allSourceKeys = Object.keys(sources).filter((k) => sources[k].enabled);

  const models = parts;
  const freeCount = models.filter(
    (m) => (m.target_usd != null && m.target_usd === 0) || m.condition === 'free',
  ).length;
  const costs = models
    .map((m) => m.target_usd)
    .filter((c): c is number => typeof c === 'number' && c > 0);
  const cheapestCost = costs.length > 0 ? Math.min(...costs) : 0;
  const cheapest = costs.length > 0;
  const floorCostRatio =
    costs.length > 0
      ? Math.max(0.05, Math.min(1, 1 - cheapestCost / Math.max(...costs, 1)))
      : 0;
  const filteredParts = parts.filter((part) => {
    if (partFilter === 'free') {
      return (part.target_usd != null && part.target_usd === 0) || part.condition === 'free';
    }
    if (partFilter === 'priced') {
      return typeof part.target_usd === 'number' && part.target_usd > 0;
    }
    return true;
  });
  const selectedPart =
    filteredParts.find((p) => p.id === selectedPartId) ?? filteredParts[0] ?? parts[0] ?? null;
  const synced = state === 'ok';

  if (condensed) {
    return (
      <section className="space-y-2 text-[11px] text-text-secondary">
        <h2 className="font-display text-xs text-brass tracking-wider uppercase">{navLabel}</h2>
        {state === 'loading' && <div className="animate-pulse text-brass">Loading…</div>}
        {state === 'error' && <div className="text-destructive">{err}</div>}
        {state === 'ok' && (
          <div>
            {parts.length} parts · {allSourceKeys.length} enabled sources
          </div>
        )}
      </section>
    );
  }

  const masterPane = (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <BentoScrollRegion
        ariaLabel="Hardware and model price watchlist matrix"
        className="flex-1 rounded-xl border border-border-strong bg-bg-surface shadow-sm"
      >
        <table className="w-full border-collapse text-[11px]">
          <thead>
            <tr className="border-b border-border-subtle bg-bg-elevated/80">
              <th className="px-3 py-2 text-left font-display text-[10px] uppercase tracking-widest text-brass">
                Part
              </th>
              <th className="px-3 py-2 text-left font-display text-[10px] uppercase tracking-widest text-brass">
                Role
              </th>
              <th className="px-3 py-2 text-left font-display text-[10px] uppercase tracking-widest text-brass">
                Target
              </th>
              {allSourceKeys.map((src) => (
                <th
                  key={src}
                  className="px-2 py-2 text-center font-display text-[10px] uppercase tracking-widest text-brass"
                >
                  {src}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {filteredParts.map((part, i) => {
              const isSelected = selectedPart?.id === part.id;
              return (
                <tr
                  key={part.id}
                  data-testid={`mercatus-part-row-${part.id}`}
                  data-selected={isSelected || undefined}
                  onClick={() => setSelectedPartId(part.id)}
                  className={`cursor-pointer border-b border-border-subtle last:border-0 transition-colors ${
                    isSelected
                      ? 'bg-brass/15'
                      : i % 2 === 0
                        ? 'hover:bg-overlay-hover'
                        : 'bg-bg-elevated/40 hover:bg-overlay-hover'
                  }`}
                >
                  <td className="px-3 py-1.5 font-mono text-[11px] text-text-primary font-medium">
                    <button
                      type="button"
                      data-slot="row-select-trigger"
                      aria-pressed={isSelected}
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedPartId(part.id);
                      }}
                      className="text-left font-mono text-[11px] font-medium text-text-primary focus-visible:underline focus-visible:outline-hidden"
                    >
                      {part.model || part.id}
                    </button>
                  </td>
                  <td className="px-3 py-1.5 text-text-secondary">{part.role}</td>
                  <td
                    className="px-3 py-1.5 font-mono text-text-primary"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <div className="inline-flex items-center gap-1">
                      <span className="text-text-muted">$</span>
                      <input
                        type="number"
                        min={0}
                        step="0.05"
                        data-testid={`mercatus-target-usd-${part.id}`}
                        aria-label={`Target USD for ${part.model || part.id}`}
                        value={part.id in draftTargets ? draftTargets[part.id] : (part.target_usd ?? '')}
                        placeholder="—"
                        onChange={(e) => handleUpdateTargetUsd(part.id, e.target.value)}
                        onBlur={() => flushTargetUsd(part.id)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') flushTargetUsd(part.id);
                        }}
                        className="w-20 rounded border border-border-strong bg-bg-elevated px-1.5 py-0.5 font-mono text-[11px] text-text-primary focus:border-brass focus:outline-none"
                      />
                    </div>
                  </td>
                  {allSourceKeys.map((src) => {
                    const hasId = !!(part.ids ?? {})[src];
                    const inSources = (part.sources ?? []).includes(src);
                    const dot = hasId ? (
                      <Icon.circleDot className="size-3.5 inline-block" aria-hidden="true" />
                    ) : inSources ? (
                      <Icon.circle className="size-3.5 inline-block" aria-hidden="true" />
                    ) : (
                      '·'
                    );
                    const cls = hasId
                      ? 'text-brass'
                      : inSources
                        ? 'text-brass/70'
                        : 'text-text-muted/40';
                    return (
                      <td
                        key={src}
                        className={`px-2 py-1.5 text-center font-mono ${cls}`}
                        title={
                          hasId
                            ? `id: ${part.ids[src]}`
                            : inSources
                              ? 'in sources[], no id'
                              : 'not tracked'
                        }
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
      </BentoScrollRegion>

      <div className="text-[10px] text-text-muted flex flex-wrap gap-4 items-center">
        <span className="inline-flex items-center gap-1">
          <Icon.circleDot className="size-3 text-brass" aria-hidden="true" />
          <span>id pinned</span>
        </span>
        <span className="inline-flex items-center gap-1">
          <Icon.circle className="size-3 text-brass/70" aria-hidden="true" />
          <span>in sources[], no id</span>
        </span>
        <span>
          <span className="font-mono text-text-muted/40">·</span> not tracked
        </span>
        <span className="ml-auto font-display text-[9px] uppercase tracking-wider text-text-secondary">
          {parts.length} parts · {allSourceKeys.length} enabled sources
        </span>
      </div>

      <BentoGrid preset="triad" data-testid="mercatus-arbitrage-deck">
        <BentoCard className="border-border-strong">
          <div className="flex items-center justify-between border-b border-border-subtle pb-1.5">
            <span className="font-display text-[10px] font-bold tracking-widest text-brass uppercase">
              Spot GPU Arbitrage
            </span>
            <RomanWaxSealBadge label="QLoRA Ready" tone="gold" />
          </div>
          <p className="mt-2 text-[11px] leading-relaxed text-text-secondary">
            Tracks Vast.ai &amp; RunPod spot instance pricing against target hourly ceilings for MENS 24GB/48GB training runs.
          </p>
        </BentoCard>
        <BentoCard className="border-border-strong">
          <div className="flex items-center justify-between border-b border-border-subtle pb-1.5">
            <span className="font-display text-[10px] font-bold tracking-widest text-brass uppercase">
              Token Cost Band
            </span>
            <RomanWaxSealBadge label="Live Sync" tone="gold" />
          </div>
          <p className="mt-2 text-[11px] leading-relaxed text-text-secondary">
            Feeds OpenRouter &amp; frontier provider price-per-million shifts directly into{' '}
            <code className="rounded bg-bg-elevated px-1 py-0.5 text-text-primary">vox-orchestrator</code>{' '}
            model routing.
          </p>
        </BentoCard>
        <BentoCard className="border-border-strong">
          <div className="flex items-center justify-between border-b border-border-subtle pb-1.5">
            <span className="font-display text-[10px] font-bold tracking-widest text-brass uppercase">
              Probe Cadence
            </span>
            <RomanWaxSealBadge label={`${allSourceKeys.length} ACTIVE FEEDS`} tone="emerald" />
          </div>
          <p className="mt-2 text-[11px] leading-relaxed text-text-secondary">
            Automated background sweeps respect per-source cadence windows and daily probe cost caps.
          </p>
        </BentoCard>
      </BentoGrid>
    </div>
  );

  return (
    <section className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-4 p-4">
      <RomanSurfaceHeader
        domainTag="MARKET · MERCATUS"
        title={`${navLabel} — Token & Compute Exchange Ledger`}
        subtitle="Real-time model pricing, context window capacity, free-tier availability, and budget burn telemetry."
        waxSealBadge={synced ? 'LIVE SYNC' : 'OFFLINE'}
        icon={<Icon.denariusCoin className="size-5 text-brass" />}
        gauges={[
          {
            label: 'Catalog',
            valueText: `${models.length} Models`,
            ratio: Math.min(1, models.length / 20),
            tone: 'gold',
            active: partFilter === 'all',
            onClick: () => setPartFilter('all'),
          },
          {
            label: 'Free Tiers',
            valueText: `${freeCount} Active`,
            ratio: Math.min(1, freeCount / Math.max(1, models.length)),
            tone: 'emerald',
            active: partFilter === 'free',
            onClick: () => setPartFilter('free'),
          },
          {
            label: 'Floor Cost',
            valueText: cheapest ? `$${cheapestCost.toFixed(2)}/1M` : '$0.00',
            ratio: floorCostRatio,
            tone: 'gold',
            active: partFilter === 'priced',
            onClick: () => setPartFilter('priced'),
          },
        ]}
        actions={
          <button
            type="button"
            onClick={reload}
            className="rounded-lg border border-border-strong bg-bg-elevated px-3 py-1 font-display text-[10px] uppercase tracking-wider text-text-primary hover:bg-overlay-hover transition-colors shadow-sm"
          >
            {state === 'loading' ? 'loading…' : 'refresh'}
          </button>
        }
      />

      {(state === 'error' || Boolean(err)) && (
        <div
          data-testid="mercatus-error-banner"
          className="rounded-xl border border-dashed border-destructive/40 bg-destructive/10 px-4 py-3 text-[11px] text-destructive"
        >
          {err}
        </div>
      )}

      {state === 'ok' && !cfg && (
        <div className="rounded-xl border border-border-strong bg-bg-surface p-6 text-center text-xs text-text-muted">
          No price-watch watchlist configured in this workspace yet.
        </div>
      )}

      {state === 'ok' && cfg && (
        <MasterDetailWorkbench
          storageKey="vox.layout.mercatus.v4"
          detailTitle="Watchlist & Source Config"
          detailDescription="Selected model/GPU target ceiling, source coverage breakdown, and live provider feed switches."
          hasActiveDetail={selectedPartId !== null}
          defaultMasterPct={56}
          master={masterPane}
          detail={
            <MercatusInspectorPane
              cfg={cfg}
              selectedPart={selectedPart}
              cheapestCost={cheapestCost}
              draftTargets={draftTargets}
              onUpdateTargetUsd={handleUpdateTargetUsd}
              onFlushTargetUsd={flushTargetUsd}
              onToggleSource={handleToggleSource}
            />
          }
        />
      )}

      {state === 'loading' && (
        <div className="text-[11px] text-text-muted animate-pulse">Loading config…</div>
      )}
    </section>
  );
}
