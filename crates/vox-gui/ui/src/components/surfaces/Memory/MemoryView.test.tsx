// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
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
      return Promise.resolve('web:test-hash');
    }
    if (cmd === 'ingest_text') {
      return Promise.resolve('doc:test-hash');
    }
    return Promise.resolve(null);
  }),
}));

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
});
