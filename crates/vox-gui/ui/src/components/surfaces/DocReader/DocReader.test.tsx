import { describe, it, expect, vi, beforeEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockInvokeMcpTool = vi.fn();

vi.mock('../../../transport', () => ({
  voxTransport: {
    openLocator: vi.fn().mockResolvedValue({ action: 'opened' }),
    readDocMarkdown: vi.fn(),
    writeDocMarkdown: vi.fn().mockResolvedValue(undefined),
    invokeMcpTool: (...args: unknown[]) => mockInvokeMcpTool(...args),
  },
}));

import { voxTransport } from '../../../transport';
import { DocReader } from './DocReader';

function renderDocReader(path: string, onNavigate?: (s: number) => void) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <DocReader path={path} onNavigateToResearch={onNavigate} />
    </QueryClientProvider>,
  );
}

describe('DocReader', () => {
  beforeEach(() => {
    vi.mocked(voxTransport.readDocMarkdown).mockReset();
    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue('# Title\n\nBody text');
    mockInvokeMcpTool.mockReset();
    mockInvokeMcpTool.mockImplementation((tool: string, args?: Record<string, unknown>) => {
      if (tool === 'vox_kb_search') {
        return Promise.resolve({
          is_error: false,
          result: {
            success: true,
            data: {
              hits: [
                {
                  doc_path: 'docs/src/architecture/where-things-live.md',
                  title: 'Where Things Live (Crate Lookup SSOT)',
                  topic: 'doc-governance',
                  domain: 'documentation',
                  trust_tier: 'verified',
                  score: 0.99,
                },
              ],
            },
          },
        });
      }
      if (tool === 'vox_kb_inbox_list') {
        return Promise.resolve({
          is_error: false,
          result: {
            success: true,
            data: {
              findings: [
                {
                  id: 'kb-find-9601',
                  claim: 'VoxDb schema baseline 96 adds kb_findings with mechanical citation checks.',
                  citation_anchor: 'crates/vox-search/src/kb_inbox.rs:L24-L88',
                  trust_tier: 'candidate',
                  confidence: 0.94,
                  topic: 'database',
                },
              ],
            },
          },
        });
      }
      if (tool === 'vox_kb_status') {
        return Promise.resolve({
          is_error: false,
          result: {
            success: true,
            data: { doc_count: 1130, chunk_count: 4200, drifting_count: 0, deficit_count: 0, mrr_at_5: 0.82, recall_at_5: 0.94 },
          },
        });
      }
      if (tool === 'vox_repo_catalog_refresh' && args?.action === 'wiki_status') {
        return Promise.resolve({
          is_error: false,
          result: {
            success: true,
            data: {
              initialized: true,
              page_count: 6,
              total_pages: 6,
              auto_region_count: 6,
              legacy_mermaid_blocks: 0,
              stale_pages: ['interfaces-and-apis.md'],
              pages: [{ rel_path: 'README.md', exists: true }, { rel_path: 'interfaces-and-apis.md', exists: true }],
            },
          },
        });
      }
      if (tool === 'vox_repo_catalog_refresh' && args?.action === 'wiki_heal_list') {
        return Promise.resolve({
          is_error: false,
          result: {
            success: true,
            data: {
              proposals: [
                {
                  id: 'heal-p1',
                  repository_id: 'vox-main',
                  doc_path: 'docs/wiki/interfaces-and-apis.md',
                  tier: 2,
                  status: 'pending',
                  summary: 'Refresh endpoint signature citation in Layer 2.',
                  unified_diff: '--- a/docs/wiki/interfaces-and-apis.md\n+++ b/docs/wiki/interfaces-and-apis.md\n@@ -1 +1 @@\n-old endpoint\n+new grounded endpoint',
                  original_hash: 'abc123',
                  patched_markdown: '# Interfaces\nnew grounded endpoint',
                  evidence_citations: ['[crates/vox-cli/src/lib.rs:L10-L24]'],
                  gate_report_json: {
                    rewrite_guard_passed: true,
                    doc_lint_passed: true,
                    vox_diagram_passed: true,
                    doctest_passed: true,
                  },
                },
              ],
            },
          },
        });
      }
      return Promise.resolve({ is_error: false, result: { success: true, data: {} } });
    });
  });

  it('loads and renders markdown for a doc path without hardcoded fallback arrays in DocReader.tsx', async () => {
    const src = readFileSync(join(import.meta.dirname, 'DocReader.tsx'), 'utf8');
    expect(src).not.toContain('DEFAULT_INBOX_FINDINGS');
    expect(src).not.toContain('DEFAULT_LIBRARY_RESULTS');

    renderDocReader('docs/src/reference/cli.md');
    await waitFor(() => {
      expect(screen.getByTestId('doc-reader')).toBeDefined();
      expect(screen.getByText(/Body text/)).toBeDefined();
    });
    expect(voxTransport.readDocMarkdown).toHaveBeenCalledWith('docs/src/reference/cli.md');
    expect(screen.queryByText(/Verified by Deep Research/i)).toBeNull();
  });

  it('renders EmpiricalVerificationBadge when document contains empirical provenance block', async () => {
    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue(
      '# Architecture Doc\n\n> **Empirical Provenance:** Generated by Vox Deep Research (Session #101, Stability $S = 0.95$).\n\nVerified findings.',
    );

    renderDocReader('docs/src/architecture/metal-optimization-2026.md');
    await waitFor(() => {
      expect(screen.getByText(/Verified by Deep Research/i)).toBeDefined();
      expect(screen.getByText(/\(S = 0.95\)/)).toBeDefined();
    });
  });

  it('forwards onNavigateToResearch when navigating from provenance drawer', async () => {
    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue(
      '# Architecture Doc\n\n> **Empirical Provenance:** Generated by Vox Deep Research (Session #101, Stability $S = 0.95$).\n\nVerified findings.',
    );
    const onNavigate = vi.fn();
    renderDocReader('docs/src/architecture/metal-optimization-2026.md', onNavigate);

    await waitFor(() => expect(screen.getByText(/Verified by Deep Research/i)).toBeDefined());
    screen.getByRole('button', { name: /Verified by Deep Research/i }).click();
    await waitFor(() => expect(screen.getByText(/Open Full Research Session #101/i)).toBeDefined());
    screen.getByRole('button', { name: /Open Full Research Session #101/i }).click();
    expect(onNavigate).toHaveBeenCalledWith(101);
  });

  it('renders status banner for research note and deprecated doc, but not current doc', async () => {
    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue('---\ntitle: "Research Doc"\nstatus: "research"\n---\n\n# Research Doc\n\nContent.');
    const { unmount } = renderDocReader('docs/src/architecture/some-research.md');
    await waitFor(() => {
      expect(screen.getByTestId('doc-status-banner')).toBeDefined();
      expect(screen.getByText(/Internals — research note:/i)).toBeDefined();
    });
    unmount();

    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue('---\ntitle: "Old API"\nstatus: "deprecated"\n---\n\n# Old API\n\nContent.');
    const { unmount: unmount2 } = renderDocReader('docs/src/reference/old.md');
    await waitFor(() => {
      expect(screen.getByTestId('doc-status-banner')).toBeDefined();
      expect(screen.getByText(/Deprecated:/i)).toBeDefined();
    });
    unmount2();

    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue('---\ntitle: "Stable API"\nstatus: "current"\n---\n\n# Stable API\n\nContent.');
    renderDocReader('docs/src/reference/stable.md');
    await waitFor(() => expect(screen.queryByTestId('doc-status-banner')).toBeNull());
  });

  it('supports live KB Library filtering, Inbox promotion with target_scope, Health, and OpenWiki Self-Heal diff studio', async () => {
    renderDocReader('docs/src/reference/cli.md');
    await waitFor(() => {
      expect(screen.getByTestId('kb-facet-filter-bar')).toBeDefined();
      expect(screen.getByTestId('kb-trust-tier-badge').textContent).toContain('Verified SSOT');
    });

    fireEvent.click(screen.getByTestId('kb-tab-inbox'));
    await waitFor(() => {
      expect(screen.getByTestId('kb-inbox-panel')).toBeDefined();
      expect(screen.getByTestId('kb-finding-row-kb-find-9601')).toBeDefined();
    });

    fireEvent.click(screen.getByTestId('kb-promote-kb-find-9601'));
    await waitFor(() => {
      expect(screen.getByTestId('kb-finding-row-kb-find-9601').textContent).toContain('Tier: verified_finding');
      expect(mockInvokeMcpTool).toHaveBeenCalledWith('vox_kb_review_finding', {
        finding_id: 'kb-find-9601',
        action: 'promote',
        target_scope: 'repo_local',
      });
    });

    fireEvent.click(screen.getByTestId('kb-tab-openwiki'));
    await waitFor(() => {
      expect(screen.getByTestId('openwiki-heal-studio')).toBeDefined();
      expect(screen.getByTestId('openwiki-diff-inspector')).toBeDefined();
    });

    fireEvent.click(screen.getByTestId('openwiki-apply-proposal-heal-p1'));
    await waitFor(() => {
      expect(mockInvokeMcpTool).toHaveBeenCalledWith('vox_repo_catalog_refresh', {
        action: 'wiki_heal_apply',
        proposal_id: 'heal-p1',
      });
    });

    fireEvent.click(screen.getByTestId('openwiki-heal-tier1-btn'));
    await waitFor(() => {
      expect(mockInvokeMcpTool).toHaveBeenCalledWith('vox_repo_catalog_refresh', {
        action: 'wiki_heal_tier1',
      });
    });

    fireEvent.click(screen.getByTestId('kb-tab-health'));
    await waitFor(() => {
      expect(screen.getByTestId('kb-health-panel')).toBeDefined();
      expect(screen.getByTestId('kb-stale-verify-count').textContent).toBe('0');
    });
    fireEvent.click(screen.getByTestId('kb-verify-stale-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('kb-stale-verify-count').textContent).toBe('1');
    });
  });

  it('parses GFM tables, frontmatter cards, kb:summary, callouts, checklists, structured formats, and Split Editor saving (v8.6)', async () => {
    const richDoc = [
      '---',
      'title: "Full Markdown Spec"',
      'category: "reference"',
      'status: "current"',
      '---',
      '',
      '# Full Markdown Spec',
      '',
      '<!-- kb:summary -->',
      'Executive summary from frontmatter.',
      '<!-- /kb:summary -->',
      '',
      '## Tables & Callouts',
      '',
      '> [!NOTE]',
      '> Grounded alert text.',
      '',
      '| Tool | Status |',
      '|---|---|',
      '| `vox check` | Ready |',
      '',
      '- [x] Verified parser',
    ].join('\n');

    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue(richDoc);
    const { unmount } = renderDocReader('docs/src/reference/rich.md');

    await waitFor(() => {
      expect(screen.getByTestId('doc-frontmatter-card')).toBeInTheDocument();
      expect(screen.getByTestId('doc-kb-summary-callout')).toHaveTextContent('Executive summary from frontmatter.');
      expect(screen.getByTestId('doc-callout-block')).toHaveTextContent('Grounded alert text.');
      expect(screen.getByTestId('doc-markdown-table')).toHaveTextContent('vox check');
      expect(screen.getByTestId('doc-task-list')).toHaveTextContent('Verified parser');
      expect(screen.getByTestId('doc-toc-bar')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('doc-mode-split'));
    const editor = screen.getByTestId('doc-editor-textarea') as HTMLTextAreaElement;
    expect(editor.value).toContain('Grounded alert text.');
    fireEvent.change(editor, { target: { value: `${richDoc}\n\n## Edited Section\nAdded line.` } });

    const saveBtn = screen.getByTestId('doc-save-btn');
    fireEvent.click(saveBtn);
    await waitFor(() => {
      expect(voxTransport.writeDocMarkdown).toHaveBeenCalledWith(
        'docs/src/reference/rich.md',
        expect.stringContaining('## Edited Section'),
      );
    });
    unmount();

    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue('{"schema":"v1","enabled":true}');
    const jsonRender = renderDocReader('docs/sample.json');
    await waitFor(() => expect(screen.getByTestId('doc-json-view')).toHaveTextContent('schema'));
    jsonRender.unmount();

    vi.mocked(voxTransport.readDocMarkdown).mockResolvedValue('name,tier\ncli,core');
    renderDocReader('docs/sample.csv');
    await waitFor(() => expect(screen.getByTestId('doc-csv-table')).toHaveTextContent('cli'));
  });

  it('supports seeded unsaved drafts, + New Doc templates, and custom workspace save paths (v8.7)', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <DocReader
          path="docs/src/architecture/draft-findings-2026.md"
          initialContent={'# Seeded Draft\n\nInitial synthesis text.'}
          initialMode="split"
        />
      </QueryClientProvider>,
    );

    expect(voxTransport.readDocMarkdown).not.toHaveBeenCalled();
    expect(screen.getByTestId('doc-editor-textarea')).toHaveValue('# Seeded Draft\n\nInitial synthesis text.');
    const pathInput = screen.getByTestId('doc-path-input') as HTMLInputElement;
    expect(pathInput.value).toBe('docs/src/architecture/draft-findings-2026.md');

    fireEvent.click(screen.getByTestId('doc-new-file-btn'));
    expect((screen.getByTestId('doc-path-input') as HTMLInputElement).value).toMatch(/^docs\/src\/architecture\/note-.*-draft\.md$/);
    expect((screen.getByTestId('doc-editor-textarea') as HTMLTextAreaElement).value).toContain('# New Workspace Note');

    fireEvent.change(screen.getByTestId('doc-path-input'), { target: { value: 'docs/wiki/custom-note.md' } });
    fireEvent.click(screen.getByTestId('doc-save-btn'));
    await waitFor(() => {
      expect(voxTransport.writeDocMarkdown).toHaveBeenCalledWith(
        'docs/wiki/custom-note.md',
        expect.stringContaining('# New Workspace Note'),
      );
    });
  });
});
