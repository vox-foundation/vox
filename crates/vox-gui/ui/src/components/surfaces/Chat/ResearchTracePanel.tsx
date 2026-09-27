import React, { useState } from 'react';

/** Mirrors Rust `research_turn.rs` — `ResearchTrace::to_event()` (events[0] of a chat reply). */
type Stage = { stage: string; status: string; elapsed_ms?: number | null; summary: string; detail?: unknown };
type Src = { n: number; url: string; title: string; engine: string };
/** Rust `ProviderStatus` (serde tag = "state"). */
type ProviderState = { state: string; hits?: number; message?: string };
/** Quick: `ProviderOutcome`; deep: `ProviderCallSummary` (adds `calls`). */
type Provider = { provider: string; status: ProviderState; elapsed_ms: number; calls?: number };
type TavilyCredits = { used: number; remaining: number };
export type ResearchTrace = {
  kind: 'research_trace';
  mode: 'none' | 'quick' | 'deep' | string;
  status: string;
  explicit?: boolean;
  reasons?: string[];
  query?: string;
  stages: Stage[];
  sources: Src[];
  source_count: number;
  model?: string | null;
  total_ms?: number;
};

const FAIL = 'text-[var(--color-status-fail)]';
const WARN = 'text-[var(--color-status-warn)]';
const STATUS_CLASS: Record<string, string> = {
  ok: 'text-text-secondary', empty: 'text-text-muted', skipped: 'text-text-muted',
  degraded: WARN, failed: FAIL,
};
/** Every `ProviderStatus` variant, labelled; unknown future states fall through verbatim. */
// The theme's warn and fail tokens are close in hue, so each state also gets a
// glyph — severity never relies on colour alone.
const PROVIDER_STATE: Record<string, { label: string; cls: string; glyph: string }> = {
  ok: { label: 'ok', cls: 'text-text-secondary', glyph: '✓' },
  timeout: { label: 'timeout', cls: FAIL, glyph: '✕' },
  error: { label: 'error', cls: FAIL, glyph: '✕' },
  not_configured: { label: 'not configured', cls: 'text-text-muted', glyph: '–' },
  disabled: { label: 'disabled', cls: 'text-text-muted', glyph: '–' },
  circuit_open: { label: 'circuit open', cls: WARN, glyph: '!' },
  budget_exhausted: { label: 'budget exhausted', cls: WARN, glyph: '!' },
};
const MODE_LABEL: Record<string, string> = { quick: 'Quick research', deep: 'Deep research' };

function detailOf(detail: unknown): { providers: Provider[]; credits: TavilyCredits | null } {
  const d = (detail && typeof detail === 'object' ? detail : {}) as { providers?: unknown; tavily_credits?: unknown };
  const providers = Array.isArray(d.providers) ? (d.providers as Provider[]) : [];
  const c = d.tavily_credits as TavilyCredits | null | undefined;
  const credits = c && typeof c.used === 'number' && typeof c.remaining === 'number' ? c : null;
  return { providers, credits };
}

function ProviderTable({ providers }: { providers: Provider[] }) {
  return (
    <div className="ml-4 mt-0.5">
      {providers.map((p, i) => {
        const st = PROVIDER_STATE[p.status.state] ?? { label: p.status.state, cls: 'text-text-secondary', glyph: '?' };
        return (
          <div key={`${p.provider}-${p.status.state}-${i}`} data-testid="research-provider"
               data-provider={p.provider} data-state={p.status.state} className={st.cls}>
            <div className="flex gap-3 whitespace-nowrap">
              <span className="w-[9ch] shrink-0">{p.provider}</span>
              <span className="w-[20ch] shrink-0">
                {st.glyph} {st.label}{p.status.hits != null && ` · ${p.status.hits} hits`}
              </span>
              <span className="w-[8ch] shrink-0 text-right tabular-nums">{p.elapsed_ms}ms</span>
              {p.calls != null && p.calls > 1 && <span>×{p.calls} calls</span>}
            </div>
            {p.status.message && <div className="ml-[12ch] break-words">{p.status.message}</div>}
          </div>
        );
      })}
    </div>
  );
}

/**
 * Collapsible per-turn research trace under an assistant reply: header with
 * mode / source count / model / time, expanding to every pipeline stage, the
 * per-provider outcome table, Tavily credits and the numbered source links.
 * A turn the classifier did not research (`mode: 'none'`) gets a compact
 * "No research · <reason>" line, so automatic detection stays visible.
 */
export function ResearchTracePanel({ trace }: { trace: ResearchTrace }) {
  const [open, setOpen] = useState(false);
  const seconds = trace.total_ms != null ? `${(trace.total_ms / 1000).toFixed(1)}s` : '';
  const header = trace.mode === 'none'
    ? ['No research', trace.reasons?.[0]].filter(Boolean).join(' · ')
    : [
        MODE_LABEL[trace.mode] ?? trace.mode,
        `${trace.source_count} sources`,
        trace.model ?? 'no model',
        seconds,
        trace.status,
      ].filter(Boolean).join(' · ');
  return (
    <div data-testid="research-trace" data-status={trace.status} data-mode={trace.mode}
         className="w-full max-w-[720px] rounded-md border border-border-subtle bg-overlay-subtle text-left font-mono text-[11px]">
      <button type="button" data-testid="research-trace-toggle" aria-expanded={open}
              onClick={() => setOpen((o) => !o)}
              className={`w-full px-2 py-1 text-left ${STATUS_CLASS[trace.status] ?? 'text-text-secondary'}`}>
        {open ? '▾' : '▸'} {header}
      </button>
      {open && (
        <div className="space-y-1 border-t border-border-subtle px-2 py-1">
          {trace.stages.map((s, i) => {
            const { providers, credits } = detailOf(s.detail);
            return (
              <div key={`${s.stage}-${i}`} data-testid={`research-stage-${s.stage}`} data-status={s.status}
                   className={`break-words ${STATUS_CLASS[s.status] ?? 'text-text-secondary'}`}>
                <span className="font-semibold">{s.stage}</span> · {s.status}
                {s.elapsed_ms != null && ` · ${s.elapsed_ms}ms`} — {s.summary}
                {providers.length > 0 && <ProviderTable providers={providers} />}
                {credits && (
                  <div data-testid="research-tavily-credits"
                       className={`ml-4 ${credits.remaining === 0 ? WARN : 'text-text-muted'}`}>
                    Tavily credits: {credits.used}/{credits.used + credits.remaining} used · {credits.remaining} left
                  </div>
                )}
              </div>
            );
          })}
          {trace.sources.length > 0 && (
            <ol className="ml-4 list-none text-text-secondary">
              {trace.sources.map((s) => (
                <li key={s.n} className="break-words">
                  [{s.n}]{' '}
                  <a data-testid={`research-source-${s.n}`} href={s.url} target="_blank" rel="noreferrer"
                     className="underline">{s.title || s.url}</a>{' '}
                  <span className="text-text-muted">({s.engine})</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      )}
    </div>
  );
}
