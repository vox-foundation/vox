import React, { useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
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
  { id: 'duckduckgo', label: 'DuckDuckGo' },
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
          className="rounded border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-mono text-emerald-400"
        >
          HTTP {status}
        </span>
      );
    }
    if (status === 0) {
      return (
        <span
          data-testid="status-badge-unconfigured"
          className="rounded border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[11px] font-mono text-amber-400"
        >
          Unconfigured (HTTP 0)
        </span>
      );
    }
    return (
      <span
        data-testid="status-badge-error"
        className="rounded border border-red-500/30 bg-red-500/10 px-2 py-0.5 text-[11px] font-mono text-red-400"
      >
        Error (HTTP {status})
      </span>
    );
  };

  return (
    <div
      data-testid="live-source-prober"
      className="rounded-lg border border-border-subtle bg-overlay-subtle p-3 space-y-3"
    >
      <div className="flex items-center justify-between">
        <h3 className="font-display text-[12px] uppercase tracking-wide text-text-primary">
          Live Source Prober
        </h3>
        <span className="text-[11px] text-text-muted">
          Inspect source engine latency and availability
        </span>
      </div>

      <div className="flex flex-wrap gap-2">
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
          className="min-w-[200px] flex-1 rounded-lg border border-border-subtle bg-black/40 px-3 py-1.5 text-sm text-text-secondary outline-hidden focus:border-brass/40"
        />

        <select
          value={provider}
          onChange={(e) => setProvider(e.target.value)}
          aria-label="Search provider"
          data-testid="probe-provider-select"
          className="rounded-lg border border-border-subtle bg-black/40 px-3 py-1.5 text-sm text-text-secondary outline-hidden focus:border-brass/40"
        >
          {SEARCH_PROVIDERS.map((p) => (
            <option key={p.id} value={p.id} className="bg-neutral-900 text-text-secondary">
              {p.label}
            </option>
          ))}
        </select>

        <button
          type="button"
          data-testid="probe-btn"
          onClick={handleProbe}
          disabled={loading || !query.trim()}
          className="rounded-lg border border-brass/30 bg-brass/10 px-4 py-1.5 text-sm text-brass hover:bg-brass/20 disabled:opacity-50"
        >
          {loading ? 'Probing…' : 'Probe Provider'}
        </button>
      </div>

      {errorMessage && (
        <div className="rounded border border-red-500/30 bg-red-500/10 p-2 text-[12px] text-red-300">
          {errorMessage}
        </div>
      )}

      {results.length > 0 && (
        <div className="space-y-2 mt-2">
          {results.map((r, i) => (
            <div
              key={`${r.provider}-${i}`}
              className="rounded-lg border border-border-subtle bg-black/30 p-3 space-y-2"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm font-semibold capitalize text-text-primary">
                    {r.provider}
                  </span>
                  {getStatusBadge(r.http_status)}
                </div>
                <div className="flex items-center gap-2">
                  <span
                    data-testid="latency-badge"
                    className="rounded border border-border-subtle bg-black/40 px-2 py-0.5 font-mono text-[11px] text-text-secondary"
                  >
                    {r.latency_ms} ms
                  </span>
                  <span
                    data-testid="hit-count-badge"
                    className="rounded border border-border-subtle bg-black/40 px-2 py-0.5 font-mono text-[11px] text-text-secondary"
                  >
                    {r.hit_count} hits
                  </span>
                </div>
              </div>

              {(r.remediation_tip || r.http_status === 0 || !r.success || r.hit_count === 0) && (
                <div
                  data-testid="remediation-banner"
                  className="rounded border border-amber-500/30 bg-amber-500/10 p-2 text-[11px] text-amber-300"
                >
                  <span className="font-semibold">Remediation: </span>
                  {r.remediation_tip ??
                    r.error_message ??
                    'Provider returned 0 hits. Check provider configuration or try a broader query.'}
                </div>
              )}

              {r.sample_titles && r.sample_titles.length > 0 && (
                <div className="pt-1">
                  <div className="text-[10px] font-mono uppercase tracking-wider text-text-muted mb-1">
                    Sample Titles
                  </div>
                  <ul className="list-disc list-inside space-y-0.5" data-testid="sample-titles-list">
                    {r.sample_titles.map((title, idx) => (
                      <li key={idx} className="truncate text-[11px] text-text-secondary">
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
