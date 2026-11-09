import React, { useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { voxTransport } from '../../../transport';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useVoxGraphStatus, VOX_GRAPH_STATUS_QUERY_KEY } from '../../../hooks/useVoxGraphStatus';
import { useLabel } from '../../../hooks/useLanguage';
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
      await voxTransport.invokeMcpTool('vox_search_rebuild', { corpus: corpusId });
      await queryClient.invalidateQueries({ queryKey: VOX_GRAPH_STATUS_QUERY_KEY });
    } catch (e) {
      setError(sanitizeErrorForToast((e as Error)?.message ?? e));
    } finally {
      setBusy(false);
    }
  }, [corpusId, queryClient]);

  return (
    <div className="mt-2 flex items-center gap-2">
      <button
        type="button"
        disabled={busy}
        aria-label={`Rebuild ${corpusId}`}
        onClick={handleRebuild}
        className="rounded-md border border-(--color-status-warn)/30 bg-(--color-status-warn)/10 px-2.5 py-1 text-[11px] font-medium text-(--color-status-warn) transition hover:bg-(--color-status-warn)/20 disabled:opacity-50"
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

  const [lastTtlDays, setLastTtlDays] = useState(ttlDays);
  if (ttlDays !== lastTtlDays) {
    setLastTtlDays(ttlDays);
    setValue(String(ttlDays)); // gui-safe: numeric input state conversion
  }

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
      const res = (await voxTransport.invokeMcpTool('vox_search_set_ttl', {
        ttl_days: parsed,
      })) as {
        result?: {
          success?: boolean;
          error?: string;
          data?: { requires_commit?: boolean; contract_path?: string };
        };
      };
      if (res?.result?.success === false) {
        setError(sanitizeErrorForToast(res.result.error ?? 'Failed to set TTL'));
        return;
      }
      if (res?.result?.data?.requires_commit) {
        setWrotePath(res.result.data.contract_path ?? null);
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
        <label htmlFor="vg-ttl-days" className="text-[11px] uppercase tracking-[0.08em] text-text-muted">
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
          className="h-7 w-20 rounded-md border border-border-subtle bg-bg-elevated px-2 font-mono text-[11px] text-text-primary disabled:opacity-50"
        />
        <button
          type="button"
          disabled={busy}
          aria-label="Save TTL"
          onClick={handleSave}
          className="h-7 rounded-md border border-border-subtle bg-bg-surface px-3 text-[11px] font-medium text-text-primary transition hover:bg-overlay-subtle disabled:opacity-50"
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

  if (condensed) {
    if (isLoading) {
      return <div className="p-2 text-[11px] text-text-secondary animate-pulse">Loading…</div>;
    }
    if (isError) {
      return <div className="p-2 text-[11px] text-(--color-status-fail)" role="alert">Code graph status unavailable</div>;
    }
    if (!data) return <div className="p-2 text-[11px] text-text-secondary">No code graph data</div>;
    const freshCount = data.corpora.filter(c => c.is_fresh).length;
    return (
      <div className="p-2 text-[11px] text-text-secondary">
        <div className="mb-1 text-xs font-semibold uppercase tracking-[0.08em] text-text-primary">{corpusHealthLabel}</div>
        <div>{freshCount}/{data.corpora.length} fresh</div>
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

  return (
    <div className="flex flex-col gap-4 p-4">
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE"
        title={corpusHealthLabel}
        subtitle="Hybrid lexical + semantic retrieval corpora, staleness TTL governance, and architectural atlas."
        actions={
          <div className="flex items-center gap-4">
            {typeof data.ttl_days === 'number' && (
              <TtlEditor
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

      <div className="grid gap-3 sm:grid-cols-1 md:grid-cols-2">
        {data.corpora.map((c) => (
          <div
            key={c.corpus_id}
            className={`vox-marble-tablet group rounded-lg border p-4 transition-colors duration-200 ${
              c.is_fresh
                ? 'border-(--color-status-pass)/20 bg-(--color-status-pass)/4 hover:border-(--color-status-pass)/40'
                : 'border-(--color-status-warn)/20 bg-(--color-status-warn)/4 hover:border-(--color-status-warn)/40'
            }`}
          >
            <div className="flex items-start justify-between">
              <div>
                <h3 className="font-medium text-text-primary">{c.title}</h3>
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
                <span
                  className="font-semibold text-text-primary"
                  title={c.built_at ?? undefined}
                >
                  {relativeBuiltAt(c.built_at)}
                </span>
              </div>
            </div>

            {!c.is_fresh && (
              <div className="mt-3 space-y-2 border-t border-border-subtle pt-3">
                <div className="text-[11px] text-text-secondary">
                  <span className="text-text-muted text-[11px] block uppercase">Stale Reasons</span>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {c.stale_reasons.map((r) => (
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
              </div>
            )}
          </div>
        ))}
      </div>

      <CodeAtlasCanvas />
    </div>
  );
}
