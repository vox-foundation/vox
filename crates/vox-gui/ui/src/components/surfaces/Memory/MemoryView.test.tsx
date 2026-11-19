import { describe, it, expect, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { LanguageProvider } from '../../../hooks/useLanguage';

const mockNodes = [
  {
    id: 'doc:123',
    label: 'Architecture Guide',
    snippet: 'This is the main architecture guide...',
    node_type: 'document',
    created_at: '2026-09-30 12:00:00',
  },
];

const mockHealth = {
  node_count: 42,
  edge_count: 88,
  fts_available: true,
  corpus_counts: {
    document: 20,
    research_synthesis: 12,
    web_research_source: 10,
  },
};

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args?: any) => {
    if (cmd === 'get_memory_status') {
      return Promise.resolve({
        corpus_counts: { memory: 100, knowledge: 200, chunk: 50 },
        shards: [],
        recent_recalls: [],
        embedding_dim: 768,
      });
    }
    if (cmd === 'list_knowledge_nodes') {
      return Promise.resolve(mockNodes);
    }
    if (cmd === 'delete_knowledge_node') {
      return Promise.resolve();
    }
    if (cmd === 'get_kb_health') {
      return Promise.resolve(mockHealth);
    }
    if (cmd === 'ingest_url') {
      return Promise.resolve({ finding_id: ingestUrlStatus === 'duplicate' ? null : 'f1', status: ingestUrlStatus });
    }
    if (cmd === 'ingest_text') {
      return Promise.resolve({ finding_id: 'f2', status: 'pending' });
    }
    if (cmd === 'vox_search_query') {
      return Promise.resolve({
        hits: [
          {
            source: 'knowledge',
            kind: 'policy',
            path: 'docs/src/architecture/cryptography-ssot-2026.md',
            title: 'Cryptography Policy',
            snippet: 'Application crypto must use vox-crypto with ed25519 signing.',
            score: 0.94,
            provenance: ['docs'],
            locator: { kind: 'file', value: 'docs/src/architecture/cryptography-ssot-2026.md' },
          },
        ],
        facets_by_source: [],
        facets_by_kind: [],
        total: 1,
        next_cursor: null,
        corpora: ['knowledge'],
      });
    }
    return Promise.resolve(null);
  }),
}));

let ingestUrlStatus = 'pending';

const noopToast = () => {};

import { MemoryView } from './MemoryView';

describe('MemoryView', () => {
  it('renders the Mnemosyne heading', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    expect(screen.getByText(/Mnemosyne/i)).toBeDefined();
  });

  it('renders the navigation tabs (P2.3)', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    expect(screen.getByTestId('tab-search')).toBeDefined();
    expect(screen.getByTestId('tab-browse')).toBeDefined();
    expect(screen.getByTestId('tab-ingest')).toBeDefined();
    expect(screen.getByTestId('tab-health')).toBeDefined();
  });

  it('switches to Browse tab and renders nodes with delete action (P2.3)', async () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    fireEvent.click(screen.getByTestId('tab-browse'));

    await waitFor(() => {
      expect(screen.getByText('Knowledge Graph Nodes')).toBeDefined();
      expect(screen.getByText('Architecture Guide')).toBeDefined();
      expect(screen.getByTestId('delete-node-doc:123')).toBeDefined();
    });
  });

  it('switches to Ingest tab and renders URL and text forms (P2.3)', async () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    fireEvent.click(screen.getByTestId('tab-ingest'));

    await waitFor(() => {
      expect(screen.getByTestId('ingest-url-btn')).toBeDefined();
      expect(screen.getByTestId('ingest-text-btn')).toBeDefined();
    });
  });

  it('switches to Health tab and renders health stats (P2.3)', async () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    fireEvent.click(screen.getByTestId('tab-health'));

    await waitFor(() => {
      expect(screen.getByText('Knowledge Graph Health & Topology')).toBeDefined();
      expect(screen.getByText('42')).toBeDefined();
      expect(screen.getByText('88')).toBeDefined();
      expect(screen.getByText(/Ready & Available/i)).toBeDefined();
    });
  });

  it('renders the Recent recalls section heading', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    expect(screen.getByText(/Recent recalls/i)).toBeDefined();
  });

  it('renders the Memory shards section heading', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    expect(screen.getByText(/Memory shards/i)).toBeDefined();
  });

  it('renders the Recall button', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    expect(screen.getByText('Recall')).toBeDefined();
  });

  it('every button carries an explicit type="button"', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('labels the recall search input (no placeholder-as-label)', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    expect(screen.getByLabelText('Recall query')).toBeDefined();
  });

  it('exposes the Auto-recall toggle with aria-pressed', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    const toggle = screen.getByRole('button', { name: /auto-recall/i });
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
  });

  it('marks the citations region as a polite live region', () => {
    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    const region = screen.getByLabelText('Recall citations');
    expect(region.getAttribute('aria-live')).toBe('polite');
  });

  async function submitIngestUrl(): Promise<{ title: string; body: string }[]> {
    const toasts: { title: string; body: string }[] = [];
    render(<LanguageProvider><MemoryView pushToast={(t: any) => toasts.push(t)} /></LanguageProvider>);
    fireEvent.click(screen.getByTestId('tab-ingest'));
    const input = await screen.findByPlaceholderText("https://example.com/article");
    fireEvent.change(input, { target: { value: 'https://example.com/post' } });
    fireEvent.click(screen.getByTestId('ingest-url-btn'));
    await waitFor(() => expect(toasts.length).toBeGreaterThan(0));
    return toasts;
  }

  it('Ingest URL toasts that the page was filed for review and is not yet searchable', async () => {
    ingestUrlStatus = 'pending';
    const [t] = await submitIngestUrl();
    expect(t.title).toBe('Filed for review');
    expect(t.body).toMatch(/pending review/i);
    expect(t.body).toMatch(/not yet searchable/i);
  });

  it('Ingest URL toasts "Already pending review" for a duplicate finding', async () => {
    ingestUrlStatus = 'duplicate';
    const [t] = await submitIngestUrl();
    expect(t.title).toBe('Already pending review');
  });

  it('toggles Ingest target scope between repo_local and global_library without cream-on-marble hex literals', () => {
    const src = readFileSync(join(import.meta.dirname, 'MemoryView.tsx'), 'utf8');
    expect(src).not.toContain('#F3E5AB');
    expect(src).not.toContain('#D6CEBF');
    expect(src).not.toContain('#78350F');

    render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
    fireEvent.click(screen.getByTestId('tab-ingest'));
    const scopeBtn = screen.getByTestId('memory-ingest-scope-toggle');
    expect(scopeBtn.textContent).toContain('Repo-Local');
    fireEvent.click(scopeBtn);
    expect(scopeBtn.textContent).toContain('Global Library');
  });

  it('switches 3-layer Federated KB scope selector and emits Research handoff navigation', () => {
    const events: CustomEvent[] = [];
    const onNav = (e: Event) => events.push(e as CustomEvent);
    window.addEventListener('vox://navigate-surface', onNav);
    try {
      render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);
      expect(screen.getByTestId('kb-layer-scope-bar')).toBeTruthy();

      const canonBtn = screen.getByTestId('kb-layer-vox_baseline');
      fireEvent.click(canonBtn);
      expect(canonBtn.getAttribute('aria-pressed')).toBe('true');

      const researchBtn = screen.getByTestId('memory-to-research-btn');
      fireEvent.click(researchBtn);
      expect(events.some((ev) => ev.detail?.view === 'research')).toBe(true);
    } finally {
      window.removeEventListener('vox://navigate-surface', onNav);
    }
  });

  it('expands/collapses hits inline, omits duplicate header Top-K/chips, and hands off hits to Research (v8.5)', async () => {
    const events: CustomEvent[] = [];
    const onNav = (e: Event) => events.push(e as CustomEvent);
    window.addEventListener('vox://navigate-surface', onNav);
    try {
      render(<LanguageProvider><MemoryView pushToast={noopToast} /></LanguageProvider>);

      // Click-tax removal: duplicate inline CorpusChips removed from search header
      expect(document.querySelectorAll('[data-slot="chip"]').length).toBe(0);

      const input = screen.getByLabelText('Recall query');
      fireEvent.change(input, { target: { value: 'ed25519' } });
      fireEvent.click(screen.getByText('Recall'));

      const expandBtn = await screen.findByTestId('expand-hit-0');
      expect(expandBtn.getAttribute('aria-expanded')).toBe('false');
      expect(expandBtn.textContent).toContain('Expand');
      fireEvent.click(expandBtn);
      expect(expandBtn.getAttribute('aria-expanded')).toBe('true');
      expect(expandBtn.textContent).toContain('Collapse');

      const researchHitBtn = screen.getByTestId('research-hit-0');
      fireEvent.click(researchHitBtn);
      expect(events.some((ev) => ev.detail?.view === 'research')).toBe(true);
      expect(window.localStorage.getItem('vox_research_seed_query')).toBe('ed25519');

      const docEvents: CustomEvent[] = [];
      const onOpenDoc = (e: Event) => docEvents.push(e as CustomEvent);
      window.addEventListener('vox://open-doc', onOpenDoc);
      try {
        fireEvent.click(screen.getByRole('button', { name: 'Open' }));
        expect(docEvents.some((ev) => ev.detail?.path === 'docs/src/architecture/cryptography-ssot-2026.md')).toBe(true);
      } finally {
        window.removeEventListener('vox://open-doc', onOpenDoc);
      }
    } finally {
      window.removeEventListener('vox://navigate-surface', onNav);
      window.localStorage.removeItem('vox_research_seed_query');
    }
  });
});
