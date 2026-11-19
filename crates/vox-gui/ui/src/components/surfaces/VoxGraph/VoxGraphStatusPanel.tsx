import React, { useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { callHonestMcpTool } from '../../../transport/t3Wave3Shared';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useVoxGraphStatus, VOX_GRAPH_STATUS_QUERY_KEY } from '../../../hooks/useVoxGraphStatus';
import { useLabel } from '../../../hooks/useLanguage';
import { BentoCard, BentoGrid, BentoScrollRegion, MasterDetailWorkbench } from '../../ui/BentoGrid';
import { CodeAtlasCanvas } from './CodeAtlasCanvas';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';

/**
 * Render an RFC3339 `built_at` timestamp as a coarse relative time
 * ("3h ago"). Returns an em dash for missing/unparseable input.
 */
function relativeBuiltAt(iso: string | null): string {
  if (!iso) return '—';
  const ts = Date.parse(iso);
  if (!Number.isFinite(ts)) return '—';
  const deltaSec = Math.round((Date.now() - ts) / 1000);
  if (deltaSec < 0) return 'just now';
  if (deltaSec < 60) return `${deltaSec}s ago`;
  if (deltaSec < 3600) return `${Math.floor(deltaSec / 60)}m ago`;
  if (deltaSec < 86_400) return `${Math.floor(deltaSec / 3600)}h ago`;
  return `${Math.floor(deltaSec / 86_400)}d ago`;
}

function RebuildButton({ corpusId }: { corpusId: string }) {
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRebuild = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await callHonestMcpTool('vox_search_rebuild', { corpus: corpusId }, (d) => d ?? {});
      await queryClient.invalidateQueries({ queryKey: VOX_GRAPH_STATUS_QUERY_KEY });
    } catch (e) {
      setError(sanitizeErrorForToast((e as Error)?.message ?? e));
    } finally {
      setBusy(false);
    }
  }, [corpusId, queryClient]);

  return (
    <div className="mt-2 flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
      <button
        type="button"
        data-slot="button"
        disabled={busy}
        aria-label={`Rebuild ${corpusId}`}
        onClick={handleRebuild}
        className="inline-flex h-8 items-center rounded-md border border-(--color-status-warn)/40 bg-bg-elevated px-3 text-xs font-medium text-(--color-status-warn) transition-colors hover:border-(--color-status-warn) disabled:opacity-50"
      >
        {busy ? 'Rebuilding…' : 'Rebuild'}
      </button>
      {error && (
        <span role="alert" className="text-[11px] text-(--color-status-fail)">
          {error}
        </span>
      )}
    </div>
  );
}

const TTL_DAYS_MIN = 1;
const TTL_DAYS_MAX = 3650;

function TtlEditor({
  ttlDays,
  effectiveTtlDays,
  envForced,
}: {
  ttlDays: number;
  effectiveTtlDays: number;
  envForced: boolean;
}) {
  const queryClient = useQueryClient();
  const [value, setValue] = useState(String(ttlDays));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [wrotePath, setWrotePath] = useState<string | null>(null);

  const handleSave = useCallback(async () => {
    const parsed = Number(value);
    if (!Number.isInteger(parsed) || parsed < TTL_DAYS_MIN || parsed > TTL_DAYS_MAX) {
      setError(`TTL must be a whole number between ${TTL_DAYS_MIN} and ${TTL_DAYS_MAX}`);
      return;
    }
    setBusy(true);
    setError(null);
    setWrotePath(null);
    try {
      const res = await callHonestMcpTool<
        { requires_commit?: boolean; contract_path?: string },
        { requires_commit?: boolean; contract_path?: string }
      >('vox_search_set_ttl', { ttl_days: parsed }, (d) => d ?? {});
      if (res.requires_commit) {
        setWrotePath(res.contract_path ?? null);
      }
      await queryClient.invalidateQueries({ queryKey: VOX_GRAPH_STATUS_QUERY_KEY });
    } catch (e) {
      setError(sanitizeErrorForToast((e as Error)?.message ?? e));
    } finally {
      setBusy(false);
    }
  }, [value, queryClient]);

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        <label
          htmlFor="vg-ttl-days"
          title="Maximum days before a graph shard is flagged stale and queued for incremental rebuild"
          className="text-[11px] uppercase tracking-[0.08em] text-text-muted cursor-help"
        >
          TTL (days)
        </label>
        <input
          id="vg-ttl-days"
          type="number"
          inputMode="numeric"
          min={TTL_DAYS_MIN}
          max={TTL_DAYS_MAX}
          value={value}
          disabled={busy}
          aria-label="Staleness TTL in days"
          onChange={(e) => {
            setValue(e.target.value);
            setWrotePath(null);
            setError(null);
          }}
          className="h-8 w-20 rounded-md border border-border-subtle bg-bg-elevated px-2.5 font-mono text-xs text-text-primary disabled:opacity-50"
        />
        <button
          type="button"
          data-slot="button"
          disabled={busy}
          aria-label="Save TTL"
          onClick={handleSave}
          className="inline-flex h-8 items-center rounded-md border border-border-subtle bg-bg-surface px-3 text-xs font-medium text-text-primary transition-colors hover:border-brass/45 hover:bg-bg-elevated disabled:opacity-50"
        >
          {busy ? 'Saving…' : 'Save'}
        </button>
      </div>
      {envForced && (
        <span className="text-[11px] text-(--color-status-warn)">
          VOX_GRAPHIFY_TTL_DAYS is set and overrides this value.
        </span>
      )}
      {effectiveTtlDays !== ttlDays && (
        <span className="text-[11px] text-text-secondary">
          Currently in force: {effectiveTtlDays} days.
        </span>
      )}
      {wrotePath && (
        <span className="text-[11px] text-text-secondary">
          Wrote <code className="font-mono">{wrotePath}</code> — commit it so the CLI and CI
          use this TTL too.
        </span>
      )}
      {error && (
        <span role="alert" className="text-[11px] text-(--color-status-fail)">
          {error}
        </span>
      )}
    </div>
  );
}

export function VoxGraphStatusPanel({ condensed }: { condensed?: boolean } = {}) {
  const { data, isLoading, isError, error } = useVoxGraphStatus();
  const corpusHealthLabel = useLabel('vg-corpus-health');
  const [corpusFilter, setCorpusFilter] = useState<'all' | 'fresh' | 'stale'>('all');
  const [shardSearch, setShardSearch] = useState('');
  const [selectedCorpusId, setSelectedCorpusId] = useState<string | null>(null);

  if (condensed) {
    if (isLoading) {
      return <div className="p-2 text-[11px] text-text-secondary animate-pulse">Loading…</div>;
    }
    if (isError) {
      return <div className="p-2 text-[11px] text-(--color-status-fail)" role="alert">Code graph status unavailable</div>;
    }
    if (!data) return <div className="p-2 text-[11px] text-text-secondary">No code graph data</div>;
    const corporaList = data.corpora ?? [];
    const freshCount = corporaList.filter((c) => c.is_fresh).length;
    return (
      <div className="p-2 text-[11px] text-text-secondary">
        <div className="mb-1 text-xs font-semibold uppercase tracking-[0.08em] text-text-primary">{corpusHealthLabel}</div>
        <div>{freshCount}/{corporaList.length} fresh</div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center p-8 text-sm text-text-secondary">
        Loading code graph status…
      </div>
    );
  }

  if (isError) {
    return (
      <div className="p-4" role="alert">
        <div className="flex items-center gap-2 rounded-lg border border-(--color-status-fail)/50 bg-(--color-status-fail)/20 p-3 text-sm text-(--color-status-fail)">
          <span>Code graph status unavailable: {(error as Error)?.message ?? 'unknown error'}</span>
        </div>
      </div>
    );
  }

  if (!data) return <div className="p-4 text-text-secondary">No code graph data available</div>;

  const corpora = data.corpora ?? [];
  const totalCorpora = corpora.length;
  const freshCorporaCount = corpora.filter((c) => c.is_fresh).length;
  const staleCorporaCount = totalCorpora - freshCorporaCount;
  const totalNodes = corpora.reduce((acc, c) => acc + (c.node_count ?? 0), 0);
  const totalEdges = corpora.reduce((acc, c) => acc + (c.edge_count ?? 0), 0);
  const q = shardSearch.trim().toLowerCase();
  const displayedCorpora = corpora.filter((c) => {
    if (corpusFilter === 'fresh' && !c.is_fresh) return false;
    if (corpusFilter === 'stale' && c.is_fresh) return false;
    if (q && !(c.title ?? '').toLowerCase().includes(q) && !(c.corpus_id ?? '').toLowerCase().includes(q)) {
      return false;
    }
    return true;
  });
  const selectedCorpus =
    displayedCorpora.find((c) => c.corpus_id === selectedCorpusId) ??
    displayedCorpora[0] ??
    corpora[0] ??
    null;

  const masterPane = (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <input
          type="search"
          aria-label="Filter retrieval corpora"
          value={shardSearch}
          onChange={(e) => setShardSearch(e.target.value)}
          placeholder="Filter index shards by title or corpus ID…"
          className="h-8 w-full rounded-md border border-border-subtle bg-bg-surface px-3 font-mono text-xs text-text-primary placeholder:text-text-muted focus:border-brass focus:outline-hidden"
        />
      </div>

      <BentoScrollRegion ariaLabel="Graph and retrieval index shards" className="flex-1 pr-1">
        <div className="flex flex-col gap-3">
          {displayedCorpora.map((c) => {
            const isSelected = selectedCorpus?.corpus_id === c.corpus_id;
            return (
              <div
                key={c.corpus_id}
                data-testid={`vg-corpus-card-${c.corpus_id}`}
                onClick={() => setSelectedCorpusId(c.corpus_id)}
                className={`vox-marble-tablet group cursor-pointer rounded-lg border p-4 transition-colors duration-200 ${
                  c.is_fresh
                    ? 'border-(--color-gold-500)/45 bg-bg-elevated hover:border-(--color-gold-400)'
                    : 'border-(--color-status-warn)/55 bg-bg-elevated hover:border-(--color-status-warn)'
                } ${isSelected ? 'ring-1 ring-(--color-gold-400)' : ''}`}
              >
                <div className="flex items-start justify-between">
                  <div>
                    <h3 className="font-medium text-text-primary">
                      <button
                        type="button"
                        data-slot="shard-select-trigger"
                        aria-pressed={isSelected}
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedCorpusId(c.corpus_id);
                        }}
                        className="text-left focus-visible:underline focus-visible:outline-hidden"
                      >
                        {c.title}
                      </button>
                    </h3>
                    <p className="font-mono text-[11px] text-text-muted">{c.corpus_id}</p>
                  </div>
                  <span
                    className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-medium ${
                      c.is_fresh
                        ? 'bg-(--color-status-pass)/10 text-(--color-status-pass)'
                        : 'bg-(--color-status-warn)/10 text-(--color-status-warn)'
                    }`}
                  >
                    <span
                      className={`h-1.5 w-1.5 rounded-full ${
                        c.is_fresh ? 'bg-(--color-status-pass)' : 'bg-(--color-status-warn)'
                      }`}
                    />
                    {c.is_fresh ? 'Fresh' : 'Stale'}
                  </span>
                </div>

                <div className="mt-3 grid grid-cols-3 gap-2 border-t border-border-subtle pt-3 font-mono text-[11px] text-text-secondary">
                  <div>
                    <span className="text-text-muted block text-[11px] uppercase">Nodes</span>
                    <span className="font-semibold text-text-primary">
                      {c.node_count?.toLocaleString() ?? '—'}
                    </span>
                  </div>
                  <div>
                    <span className="text-text-muted block text-[11px] uppercase">Edges</span>
                    <span className="font-semibold text-text-primary">
                      {c.edge_count?.toLocaleString() ?? '—'}
                    </span>
                  </div>
                  <div>
                    <span className="text-text-muted block text-[11px] uppercase">Built</span>
                    <span className="font-semibold text-text-primary" title={c.built_at ?? undefined}>
                      {relativeBuiltAt(c.built_at)}
                    </span>
                  </div>
                </div>

                {!c.is_fresh && (
                  <details
                    open
                    onClick={(e) => e.stopPropagation()}
                    className="mt-3 space-y-2 border-t border-border-subtle pt-3"
                  >
                    <summary className="cursor-pointer font-sans text-[11px] font-semibold uppercase tracking-wider text-(--color-status-warn) select-none">
                      Stale Diagnostics &amp; Rebuild Actions
                    </summary>
                    <div className="pt-1 text-[11px] text-text-secondary">
                      <span className="text-text-muted text-[11px] block uppercase">Stale Reasons</span>
                      <div className="flex flex-wrap gap-1 mt-1">
                        {(c.stale_reasons ?? []).map((r) => (
                          <span
                            key={r}
                            className="rounded-sm bg-(--color-status-warn)/10 px-1.5 py-0.5 text-[11px] font-mono text-(--color-status-warn)"
                          >
                            {r}
                          </span>
                        ))}
                      </div>
                    </div>

                    <RebuildButton corpusId={c.corpus_id} />

                    <div className="relative mt-2 rounded-sm bg-bg-elevated p-2 border border-border-subtle">
                      <span className="text-[11px] text-text-muted block uppercase mb-1">Rebuild Command</span>
                      <code className="block select-all font-mono text-[11px] text-text-primary break-all leading-normal">
                        vox graph rebuild --corpus {c.corpus_id}
                      </code>
                    </div>
                  </details>
                )}
              </div>
            );
          })}
        </div>
      </BentoScrollRegion>
    </div>
  );

  const detailPane = (
    <BentoScrollRegion
      ariaLabel="Selected corpus diagnostics and code atlas"
      data-testid="vg-inspector-pane"
      className="flex-1 pr-1"
    >
      <div className="flex flex-col gap-3">
        {selectedCorpus && (
          <div className="vox-marble-tablet rounded-xl border border-(--color-gold-500)/50 bg-bg-elevated p-3.5 font-mono text-[11px]">
            <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-strong pb-2">
              <span className="font-display text-xs font-semibold tracking-[0.13em] text-text-primary uppercase">
                {`Selected Shard: ${selectedCorpus.title} (${selectedCorpus.corpus_id})`}
              </span>
              <div className="flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  data-slot="button"
                  data-roman-plaque="true"
                  data-testid="vg-search-in-memory-btn"
                  title="Open the Memory recall surface to query indexed artifacts from this shard"
                  onClick={() =>
                    window.dispatchEvent(
                      new CustomEvent('vox://navigate-surface', { detail: { view: 'memory' } }),
                    )
                  }
                  className="vox-brass-plaque-btn inline-flex h-7 items-center gap-1 rounded px-2.5 text-[11px] font-medium text-text-primary transition-colors"
                >
                  Search Shard in Memory
                </button>
                <span className="font-semibold text-text-primary">
                  {`Health: ${selectedCorpus.is_fresh ? 'Fresh' : 'Stale'}`}
                </span>
              </div>
            </div>
            <div
              data-testid="vg-shard-diagnostics-grid"
              className="mt-2.5 grid grid-cols-2 gap-2 text-text-secondary @xl/surface:grid-cols-4"
            >
              {selectedCorpus.graph_exists && selectedCorpus.manifest_exists ? (
                <>
                  <div>{`Graph File: Present`}</div>
                  <div>{`Manifest: Present`}</div>
                  <div>{`Manifest SHA: ${selectedCorpus.manifest_git_sha ?? 'unbuilt'}`}</div>
                  <div>{`HEAD SHA: ${selectedCorpus.head_git_sha ?? 'unknown'}`}</div>
                </>
              ) : (
                <>
                  <div>{'Disk Cache: Live RRF / Pending Rebuild'}</div>
                  <div>{`Live Nodes: ${(selectedCorpus.node_count ?? 0).toLocaleString()}`}</div>
                  <div>{`Live Edges: ${(selectedCorpus.edge_count ?? 0).toLocaleString()}`}</div>
                  <div>{`HEAD SHA: ${selectedCorpus.head_git_sha ?? 'unknown'}`}</div>
                </>
              )}
            </div>
            {(selectedCorpus.warnings ?? []).length > 0 && (
              <div className="mt-2 border-t border-border-subtle pt-2 text-(--color-status-warn)">
                {`Warnings: ${(selectedCorpus.warnings ?? []).join('; ')}`}
              </div>
            )}
          </div>
        )}
        <CodeAtlasCanvas />
      </div>
    </BentoScrollRegion>
  );

  return (
    <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-4 p-4">
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE"
        title={corpusHealthLabel}
        subtitle="Hybrid lexical + semantic retrieval corpora, staleness TTL governance, and architectural atlas."
        gauges={[
          {
            label: 'Shards',
            valueText: `${totalCorpora}`,
            ratio: totalCorpora > 0 ? displayedCorpora.length / totalCorpora : 0,
            tone: 'gold',
            purpose: 'Registered hybrid retrieval and code graph shards',
            actionHint: 'Click to show all registered retrieval shards',
            active: corpusFilter === 'all' && totalCorpora > 0,
            onClick: () => setCorpusFilter('all'),
          },
          {
            label: 'Healthy',
            valueText: `${freshCorporaCount}/${totalCorpora}`,
            ratio: totalCorpora > 0 ? freshCorporaCount / totalCorpora : 0,
            tone: freshCorporaCount === totalCorpora && totalCorpora > 0 ? 'emerald' : 'gold',
            purpose: 'Retrieval shards within contract staleness TTL',
            actionHint: corpusFilter === 'fresh' ? 'Click to show all shards' : 'Click to filter to fresh shards',
            active: corpusFilter === 'fresh',
            onClick: () => setCorpusFilter((f) => (f === 'fresh' ? 'all' : 'fresh')),
          },
          {
            label: 'Outdated',
            valueText: `${staleCorporaCount}`,
            ratio: totalCorpora > 0 ? staleCorporaCount / totalCorpora : 0,
            tone: staleCorporaCount > 0 ? 'crimson' : 'emerald',
            purpose: 'Shards requiring graph or index rebuild',
            actionHint: corpusFilter === 'stale' ? 'Click to show all shards' : 'Click to filter to stale shards',
            active: corpusFilter === 'stale',
            onClick: () => setCorpusFilter((f) => (f === 'stale' ? 'all' : 'stale')),
          },
        ]}
        actions={
          <div className="flex items-center gap-4">
            {typeof data.ttl_days === 'number' && (
              <TtlEditor
                key={String(data.ttl_days_contract ?? data.ttl_days)}
                ttlDays={data.ttl_days_contract ?? data.ttl_days}
                effectiveTtlDays={data.ttl_days}
                envForced={data.ttl_days_env_forced === true}
              />
            )}
            <span className="font-mono text-[11px] text-text-muted">
              Default: {data.default_corpus_id}
            </span>
          </div>
        }
      />

      <BentoGrid preset="triad">
        <BentoCard className="vox-marble-tablet border-(--color-gold-500)/45 bg-bg-elevated py-2.5">
          <div className="font-display text-[11px] font-semibold tracking-[0.13em] text-text-primary uppercase">
            Shard Coverage
          </div>
          <div className="mt-1 font-mono text-xs font-medium text-text-primary">
            {`${displayedCorpora.length} visible · ${freshCorporaCount} fresh · ${staleCorporaCount} stale`}
          </div>
        </BentoCard>
        <BentoCard className="vox-marble-tablet border-(--color-gold-500)/45 bg-bg-elevated py-2.5">
          <div className="font-display text-[11px] font-semibold tracking-[0.13em] text-text-primary uppercase">
            Graph Topology
          </div>
          <div className="mt-1 font-mono text-xs font-medium text-text-primary">
            {`${totalNodes.toLocaleString()} indexed nodes · ${totalEdges.toLocaleString()} edges`}
          </div>
        </BentoCard>
        <BentoCard className="vox-marble-tablet border-(--color-gold-500)/45 bg-bg-elevated py-2.5">
          <div className="font-display text-[11px] font-semibold tracking-[0.13em] text-(--color-status-pass) uppercase">
            Hybrid Retrieval
          </div>
          <div className="mt-1 font-mono text-xs font-medium text-text-primary">
            Tantivy BM25 + Vector RRF Fusion
          </div>
        </BentoCard>
      </BentoGrid>

      <MasterDetailWorkbench
        storageKey="vox.layout.vox-graph.v4"
        detailTitle="Graph & Index Inspector"
        detailDescription="Selected retrieval shard manifest diagnostics and interactive Code & Architecture Atlas."
        hasActiveDetail={selectedCorpusId !== null}
        defaultMasterPct={42}
        master={masterPane}
        detail={detailPane}
      />
    </div>
  );
}
