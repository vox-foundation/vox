// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import { ResearchTracePanel, type ResearchTrace } from './ResearchTracePanel';
// Recorded live daemon replies (see each fixture's `source`).
import quickOk from '../../../../e2e/fixtures/research-trace/quick-ok.json';
import quickErrors from '../../../../e2e/fixtures/research-trace/quick-provider-errors.json';
import budget from '../../../../e2e/fixtures/research-trace/quick-budget-exhausted.derived.json';
import deepOk from '../../../../e2e/fixtures/research-trace/deep-ok.json';
import deepFailed from '../../../../e2e/fixtures/research-trace/deep-failed.json';
import noResearch from '../../../../e2e/fixtures/research-trace/no-research.json';

const traceOf = (f: { reply: { events: unknown[] } }) => f.reply.events[0] as ResearchTrace;
const open = () => fireEvent.click(screen.getByTestId('research-trace-toggle'));
const providerRow = (provider: string) =>
  screen.getAllByTestId('research-provider').find((r) => r.getAttribute('data-provider') === provider)!;

describe('ResearchTracePanel', () => {
  it('shows a collapsed header with mode, source count, model, time and status', () => {
    render(<ResearchTracePanel trace={traceOf(quickOk)} />);
    const header = screen.getByTestId('research-trace-toggle');
    expect(header).toHaveAttribute('aria-expanded', 'false');
    expect(header).toHaveTextContent('Quick research · 8 sources · google/gemini-3.8-flash · 11.0s · ok');
    expect(screen.queryByTestId('research-stage-retrieval')).toBeNull();
  });

  it('expands to every stage, the provider table, Tavily credits and source links', () => {
    render(<ResearchTracePanel trace={traceOf(quickOk)} />);
    open();
    for (const s of ['detection', 'preamble', 'queries', 'retrieval', 'sources', 'citation_check']) {
      expect(screen.getByTestId(`research-stage-${s}`)).toBeInTheDocument();
    }
    expect(screen.getAllByTestId('research-provider').map((r) => r.getAttribute('data-provider')))
      .toEqual(['arxiv', 'openalex', 'wikipedia', 'searxng', 'tavily']);
    expect(providerRow('tavily')).toHaveTextContent('ok · 5 hits');
    expect(screen.getByTestId('research-tavily-credits')).toHaveTextContent('Tavily credits: 1/50 used · 49 left');
    expect(screen.getByTestId('research-source-5'))
      .toHaveAttribute('href', 'https://openrouter.ai/google/gemini-3.8-flash');
  });

  it('renders error, circuit_open and not_configured provider outcomes honestly', () => {
    render(<ResearchTracePanel trace={traceOf(quickErrors)} />);
    open();
    expect(providerRow('openalex')).toHaveAttribute('data-state', 'error');
    expect(providerRow('openalex')).toHaveTextContent('429 Too Many Requests');
    expect(providerRow('searxng')).toHaveTextContent('circuit open');
    expect(providerRow('tavily')).toHaveTextContent('not configured');
  });

  it('makes an exhausted Tavily budget visible', () => {
    render(<ResearchTracePanel trace={traceOf(budget)} />);
    open();
    expect(providerRow('tavily')).toHaveAttribute('data-state', 'budget_exhausted');
    expect(providerRow('tavily')).toHaveTextContent('budget exhausted');
    expect(screen.getByTestId('research-tavily-credits')).toHaveTextContent('50/50 used · 0 left');
  });

  it('labels every ProviderStatus variant, including calls from a deep run', () => {
    const states = [
      { state: 'ok', hits: 3 }, { state: 'timeout' }, { state: 'error', message: 'boom' },
      { state: 'not_configured' }, { state: 'disabled' }, { state: 'circuit_open' }, { state: 'budget_exhausted' },
    ];
    const trace: ResearchTrace = {
      ...traceOf(deepOk),
      stages: [{
        stage: 'retrieval', status: 'degraded', summary: 's',
        detail: { providers: states.map((status, i) => ({ provider: `p${i}`, status, elapsed_ms: 1, calls: 2 })) },
      }],
    };
    render(<ResearchTracePanel trace={trace} />);
    open();
    const text = screen.getAllByTestId('research-provider').map((r) => r.textContent);
    ['ok · 3 hits', 'timeout', 'error', 'not configured', 'disabled', 'circuit open', 'budget exhausted']
      .forEach((label, i) => expect(text[i]).toContain(label));
    expect(text[2]).toContain('boom');
    expect(text[0]).toContain('×2 calls');
  });

  it('shows the deep pipeline stages, including the claims summary', () => {
    render(<ResearchTracePanel trace={traceOf(deepOk)} />);
    expect(screen.getByTestId('research-trace-toggle')).toHaveTextContent('Deep research · 40 sources');
    open();
    expect(screen.getByTestId('research-stage-claims')).toHaveTextContent('2 claims: 1 supported');
    expect(screen.getByTestId('research-stage-citation_audit')).toHaveAttribute('data-status', 'degraded');
  });

  it('marks a failed deep pipeline visibly with its error', () => {
    render(<ResearchTracePanel trace={traceOf(deepFailed)} />);
    expect(screen.getByTestId('research-trace')).toHaveAttribute('data-status', 'failed');
    expect(screen.getByTestId('research-trace-toggle')).toHaveTextContent('no model');
    open();
    const stage = screen.getByTestId('research-stage-deep_pipeline');
    expect(stage).toHaveAttribute('data-status', 'failed');
    expect(within(stage).getByText(/No API key available/)).toBeInTheDocument();
  });

  it('shows a compact "No research" line naming the detector decision', () => {
    const t = traceOf(noResearch);
    expect(t.mode).toBe('none');
    render(<ResearchTracePanel trace={t} />);
    const header = screen.getByTestId('research-trace-toggle');
    expect(screen.getByTestId('research-trace')).toHaveAttribute('data-mode', 'none');
    expect(header).toHaveTextContent('No research · skip: greeting / small talk');
    expect(header).not.toHaveTextContent('sources');
  });
});
