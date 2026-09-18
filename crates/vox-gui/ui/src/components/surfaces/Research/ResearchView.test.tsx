// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import React from 'react';

const SESSIONS = [
  { id: 1, status: 'completed', query_text: 'What is Vox?', started_at_ms: 0, finished_at_ms: 1 },
];

const DETAIL_NO_CLAIMS = {
  session: SESSIONS[0],
  report_markdown: 'The sky is blue.',
  artifact_json: null,
};

const DETAIL_WITH_CLAIMS = {
  session: SESSIONS[0],
  report_markdown: 'The sky is blue.',
  artifact_json: null,
  confidence_tier: 'DeepResearch',
  source_count: 3,
  citation_precision: 1.0,
  claims: [
    {
      claim_id: 'c1',
      text: 'The sky is blue.',
      verdict: 'Supported',
      confidence: 0.9,
      resample_stability: 0.8,
      citation_urls: ['https://example.com/a'],
      corroboration_count: 1,
    },
  ],
};

let detailResponse: unknown = DETAIL_NO_CLAIMS;

const invokeMock = vi.fn((cmd: string) => {
  if (cmd === 'list_research_sessions') return Promise.resolve(SESSIONS);
  if (cmd === 'get_research_session_detail') return Promise.resolve(detailResponse);
  return Promise.resolve(null);
});
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { LanguageProvider } from '../../../hooks/useLanguage';
import { ResearchView } from './ResearchView';

describe('ResearchView', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockClear();
    detailResponse = DETAIL_NO_CLAIMS;
  });

  it('renders the Research heading', () => {
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    expect(screen.getByText('Research')).toBeTruthy();
  });

  it('every button carries an explicit type="button"', async () => {
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('the research query input is labeled', () => {
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    expect(screen.getByLabelText('Research question')).toBeTruthy();
  });

  it('exposes the session history as role=list', async () => {
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getAllByRole('list').length).toBeGreaterThan(0));
    expect(screen.getAllByRole('listitem').length).toBe(SESSIONS.length);
  });

  it('renders the raw report only when the detail has no claim/citation data (current backend shape)', async () => {
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    screen.getByText('What is Vox?').closest('button')!.click();
    await waitFor(() => expect(screen.getByText('The sky is blue.')).toBeTruthy());
    expect(screen.queryByText(/High confidence/i)).toBeNull();
    expect(screen.queryByText(/claims verified/i)).toBeNull();
  });

  it('renders the headline banner and claim accordion when the detail carries claim/citation data', async () => {
    detailResponse = DETAIL_WITH_CLAIMS;
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    screen.getByText('What is Vox?').closest('button')!.click();
    await waitFor(() => expect(screen.getByText(/Preliminary — Awaiting evidence corroboration/i)).toBeTruthy());
    expect(screen.getByText(/1 claim verified · 0 contested · 3 sources/i)).toBeTruthy();
    // The fixture's single citation has corroboration_count: 1 (< 2, so it
    // reads as uncorroborated) — the banner's "N corroborating sources"
    // figure reflects that (0), so it correctly shows Preliminary instead of false High confidence.
    expect(screen.getByText('The sky is blue.')).toBeTruthy();
  });

  it('counts only citations backed by real corroboration data in the headline banner and requires 2+ for high confidence', async () => {
    detailResponse = {
      ...DETAIL_WITH_CLAIMS,
      claims: [
        {
          claim_id: 'c1',
          text: 'A well-corroborated claim.',
          verdict: 'Supported',
          confidence: 0.9,
          resample_stability: 1.0,
          citation_urls: ['https://example.com/a', 'https://example.com/b'],
          corroboration_count: 3,
        },
      ],
    };
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    screen.getByText('What is Vox?').closest('button')!.click();
    await waitFor(() => expect(screen.getByText(/High confidence/i)).toBeTruthy());
    expect(screen.getByText(/High confidence — 2 corroborating sources, no contested claims/i)).toBeTruthy();
  });

  it('renders refuted warning banner when claims are contradicted', async () => {
    detailResponse = {
      ...DETAIL_WITH_CLAIMS,
      claims: [
        {
          claim_id: 'c1',
          text: 'A refuted claim.',
          verdict: 'Refuted',
          confidence: 0.9,
          resample_stability: 1.0,
          citation_urls: ['https://example.com/a', 'https://example.com/b'],
          corroboration_count: 3,
        },
      ],
    };
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    screen.getByText('What is Vox?').closest('button')!.click();
    await waitFor(() => expect(screen.getByText(/Refuted by Evidence/i)).toBeTruthy());
    expect(screen.getByText(/Refuted by Evidence — 1 of 1 claims contradicted by sources/i)).toBeTruthy();
  });

  it('clicking a citation button in the report highlights the matching claim row', async () => {
    detailResponse = {
      ...DETAIL_WITH_CLAIMS,
      report_markdown: 'The sky is blue [1].',
      claims: [
        {
          claim_id: 'c1',
          text: 'The sky is blue.',
          verdict: 'Supported',
          confidence: 0.9,
          resample_stability: 1.0,
          citation_urls: ['https://example.com/a', 'https://example.com/b'],
          corroboration_count: 3,
        },
      ],
    };
    const { container } = render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    screen.getByText('What is Vox?').closest('button')!.click();

    const citeButton = await screen.findByRole('button', { name: 'Citation 1' });
    expect(citeButton).toBeTruthy();
    citeButton.click();

    await waitFor(() => {
      const claimEl = container.querySelector('#claim-c1');
      expect(claimEl).toBeTruthy();
      expect(claimEl?.className).toContain('ring-2 ring-brass');
    });
  });

  it('renders epistemic DAG canvas and opens Sandbox REPL modal from detail view', async () => {
    detailResponse = DETAIL_WITH_CLAIMS;
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    screen.getByText('What is Vox?').closest('button')!.click();

    await waitFor(() => {
      expect(screen.getByRole('region', { name: /epistemic research dag/i })).toBeTruthy();
    });

    const replButton = screen.getByRole('button', { name: /sandbox repl/i });
    expect(replButton).toBeTruthy();
    replButton.click();

    await waitFor(() => {
      expect(screen.getByText('Sandbox REPL Probe')).toBeTruthy();
      expect(screen.getByRole('button', { name: /run compiler probe/i })).toBeTruthy();
    });
  });

  it('toggles Diagnostic Prober when clicking toggle-diagnostics-btn', async () => {
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);

    expect(screen.queryByTestId('live-source-prober')).toBeNull();

    const toggleBtn = screen.getByTestId('toggle-diagnostics-btn');
    expect(toggleBtn.getAttribute('type')).toBe('button');
    toggleBtn.click();

    await waitFor(() => {
      expect(screen.getByTestId('live-source-prober')).toBeTruthy();
    });

    toggleBtn.click();
    await waitFor(() => {
      expect(screen.queryByTestId('live-source-prober')).toBeNull();
    });
  });

  it('toggles Judge Inspector when clicking toggle-judge-btn in session detail', async () => {
    detailResponse = DETAIL_WITH_CLAIMS;
    render(<LanguageProvider><ResearchView pushToast={vi.fn()} /></LanguageProvider>);
    await waitFor(() => expect(screen.getByText('What is Vox?')).toBeTruthy());
    screen.getByText('What is Vox?').closest('button')!.click();

    await waitFor(() => {
      expect(screen.getByTestId('toggle-judge-btn')).toBeTruthy();
    });

    expect(screen.queryByTestId('judge-inspector')).toBeNull();

    const judgeBtn = screen.getByTestId('toggle-judge-btn');
    expect(judgeBtn.getAttribute('type')).toBe('button');
    judgeBtn.click();

    await waitFor(() => {
      expect(screen.getByTestId('judge-inspector')).toBeTruthy();
      expect(screen.getByTestId('metric-confidence-tier').textContent).toBe('DeepResearch');
      expect(screen.getByTestId('metric-source-count').textContent).toBe('3');
      expect(screen.getByTestId('count-supported').textContent).toBe('1');
    });

    judgeBtn.click();
    await waitFor(() => {
      expect(screen.queryByTestId('judge-inspector')).toBeNull();
    });
  });
});
