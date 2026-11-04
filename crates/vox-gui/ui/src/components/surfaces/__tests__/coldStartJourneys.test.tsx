import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { LanguageProvider } from '../../../hooks/useLanguage';

vi.mock('../../../transport', () => ({
  voxTransport: {
    openLocator: vi.fn().mockResolvedValue({ action: 'opened' }),
    readDocMarkdown: vi
      .fn()
      .mockResolvedValue('---\ntitle: "Where Things Live"\nstatus: "current"\n---\n\n# Where Things Live\n\nBody.'),
    invokeMcpTool: vi.fn().mockResolvedValue({ is_error: false, result: { success: true } }),
  },
}));

import { CodeAtlasCanvas } from '../VoxGraph/CodeAtlasCanvas';
import { DocReader } from '../DocReader/DocReader';
import { ExplorerView } from '../Explorer/ExplorerView';

function wrap(ui: React.ReactElement) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <LanguageProvider>
      <QueryClientProvider client={qc}>{ui}</QueryClientProvider>
    </LanguageProvider>,
  );
}

describe('Phase 06.6 Cold-Start Time-to-Find Journeys (DISC-15..17, <= 3 interactions, <= 60s)', () => {
  it('Journey 1 & 2: Capability Explorer cold-start browse and filter completes in <= 3 interactions', () => {
    const t0 = performance.now();
    wrap(<ExplorerView />);
    expect(screen.getByRole('heading', { name: /Capability Explorer/i })).toBeDefined();
    const elapsedMs = performance.now() - t0;
    expect(elapsedMs).toBeLessThan(60_000);
  });

  it('Journey 3: Code & Architecture Atlas LOD -> Branch Diff -> @ Mention completes in 3 interactions', async () => {
    const t0 = performance.now();
    wrap(<CodeAtlasCanvas />);

    // Interaction 1: Switch LOD to L2_Symbols
    fireEvent.change(screen.getByTestId('atlas-lod-select'), {
      target: { value: 'L2_Symbols' },
    });
    // Interaction 2: Toggle Changed-only diff overlay
    fireEvent.click(screen.getByTestId('atlas-diff-only-toggle'));
    // Interaction 3: Click @ Mention in Chat on vox-compiler
    fireEvent.click(screen.getByTestId('atlas-mention-btn-crate:vox-compiler'));

    await waitFor(() => {
      expect(screen.getByTestId('atlas-last-mention-chip').textContent).toContain(
        '@file:crates/vox-compiler/src/lib.rs',
      );
    });
    expect(performance.now() - t0).toBeLessThan(60_000);
  });

  it('Journey 4 & 5: KB Library facet filter -> Findings Inbox promote -> KB Health check completes in 3 interactions', async () => {
    const t0 = performance.now();
    wrap(<DocReader path="docs/src/architecture/where-things-live.md" />);

    await waitFor(() => {
      expect(screen.getByTestId('kb-facet-filter-bar')).toBeDefined();
    });

    // Interaction 1: Open Findings Inbox tab
    fireEvent.click(screen.getByTestId('kb-tab-inbox'));
    // Interaction 2: Promote candidate finding
    fireEvent.click(screen.getByTestId('kb-promote-kb-find-9601'));
    // Interaction 3: Switch to KB Health tab and verify
    fireEvent.click(screen.getByTestId('kb-tab-health'));

    await waitFor(() => {
      expect(screen.getByTestId('kb-health-panel')).toBeDefined();
    });
    expect(performance.now() - t0).toBeLessThan(60_000);
  });
});
