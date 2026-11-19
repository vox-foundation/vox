import React, { useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { VoxSelect } from '../../ui/Select';
import {
  probeSearchProvider,
  probeAllSearchProviders,
  type ProviderProbeResult,
} from './researchActions';

export type { ProviderProbeResult };

export interface LiveSourceProberProps {
  initialQuery?: string;
}

export const SEARCH_PROVIDERS = [
  { id: 'all', label: 'All Providers' },
  { id: 'searxng', label: 'SearXNG' },
  { id: 'tavily', label: 'Tavily' },
  { id: 'openalex', label: 'OpenAlex' },
  { id: 'arxiv', label: 'arXiv' },
  { id: 'wikipedia', label: 'Wikipedia' },
] as const;

export function LiveSourceProber({ initialQuery = '' }: LiveSourceProberProps) {
  const [query, setQuery] = useState(initialQuery);
  const [provider, setProvider] = useState<string>('all');
  const [loading, setLoading] = useState(false);
  const [results, setResults] = useState<ProviderProbeResult[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const handleProbe = async () => {
    const trimmed = query.trim();
    if (!trimmed) return;

    setLoading(true);
    setErrorMessage(null);

    try {
      if (provider === 'all') {
        const res = await probeAllSearchProviders(trimmed);
        setResults(res);
      } else {
        const res = await probeSearchProvider(provider, trimmed);
        setResults([res]);
      }
    } catch (err) {
      setErrorMessage(sanitizeErrorForToast(err));
    } finally {
      setLoading(false);
    }
  };

  const getStatusBadge = (status: number) => {
    if (status === 200) {
      return (
        <span
          data-testid="status-badge-200"
          className="rounded border border-(--color-status-pass)/35 bg-bg-elevated px-2 py-0.5 text-[11px] font-mono text-(--color-status-pass)"
        >
          HTTP {status}
        </span>
      );
    }
    if (status === 0) {
      return (
        <span
          data-testid="status-badge-unconfigured"
          className="rounded border border-(--color-status-warn)/35 bg-bg-elevated px-2 py-0.5 text-[11px] font-mono text-(--color-status-warn)"
        >
          Unconfigured (HTTP 0)
        </span>
      );
    }
    return (
      <span
        data-testid="status-badge-error"
        className="rounded border border-(--color-status-fail)/35 bg-bg-elevated px-2 py-0.5 text-[11px] font-mono text-(--color-status-fail)"
      >
        Error (HTTP {status})
      </span>
    );
  };

  return (
    <div
      data-testid="live-source-prober"
      className="rounded-lg border border-border-subtle bg-bg-surface p-3 space-y-3"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="font-display text-xs uppercase tracking-wide text-text-primary">
          Live Source Prober
        </h3>
        <span className="text-xs text-text-muted">
          Inspect source engine latency and availability
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Probe query..."
          aria-label="Probe query"
          data-testid="probe-query-input"
          onKeyDown={(e) => {
            if (e.key === 'Enter') void handleProbe();
          }}
          className="h-9 min-w-[200px] flex-1 rounded-lg border border-border-subtle bg-bg-elevated px-3 text-sm text-text-primary outline-hidden focus:border-brass/50"
        />

        <VoxSelect
          value={provider}
          onValueChange={setProvider}
          aria-label="Search provider"
          data-testid="probe-provider-select"
          options={SEARCH_PROVIDERS.map((p) => ({
            value: p.id,
            label: p.label,
          }))}
          className="h-9 w-40 rounded-lg border-border-subtle bg-bg-elevated text-sm text-text-primary"
        />

        <button
          type="button"
          data-slot="button"
          data-testid="probe-submit-btn"
          onClick={handleProbe}
          disabled={loading || !query.trim()}
          className="h-9 rounded-lg border border-brass/40 bg-bg-elevated px-4 text-xs font-semibold text-brass hover:bg-brass/15 disabled:opacity-50"
        >
          {loading ? 'Probing…' : 'Probe Provider'}
        </button>
      </div>

      {errorMessage && (
        <div className="rounded border border-(--color-status-fail)/35 bg-bg-elevated p-2 text-xs text-(--color-status-fail)">
          {errorMessage}
        </div>
      )}

      {results.length > 0 && (
        <div className="space-y-2 mt-2">
          {results.map((r, i) => (
            <div
              key={`${r.provider}-${i}`}
              className="rounded-lg border border-border-subtle bg-bg-panel p-3 space-y-2"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="font-sans text-sm font-semibold capitalize text-text-primary">
                    {r.provider}
                  </span>
                  {getStatusBadge(r.http_status)}
                </div>
                <div className="flex items-center gap-2">
                  <span
                    data-testid="latency-badge"
                    className="rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[11px] text-text-secondary"
                  >
                    {r.latency_ms} ms
                  </span>
                  <span
                    data-testid="hit-count-badge"
                    className="rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[11px] text-text-secondary"
                  >
                    {r.hit_count} hits
                  </span>
                </div>
              </div>

              {(r.remediation_tip || r.http_status === 0 || !r.success || r.hit_count === 0) && (
                <div
                  data-testid="remediation-banner"
                  className="rounded border border-(--color-status-warn)/35 bg-bg-elevated p-2 text-xs text-(--color-status-warn)"
                >
                  <span className="font-semibold">Remediation: </span>
                  {r.remediation_tip ??
                    r.error_message ??
                    'Provider returned 0 hits. Check provider configuration or try a broader query.'}
                </div>
              )}

              {r.sample_titles && r.sample_titles.length > 0 && (
                <div className="pt-1">
                  <div className="text-[11px] font-medium uppercase tracking-wider text-text-muted mb-1">
                    Sample Titles
                  </div>
                  <ul className="list-disc list-inside space-y-0.5" data-testid="sample-titles-list">
                    {r.sample_titles.map((title, idx) => (
                      <li key={idx} className="truncate text-xs text-text-secondary">
                        {title}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
