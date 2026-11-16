import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';

const mockLoadConfig = vi.fn();
const mockSaveConfig = vi.fn();

vi.mock('../../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../transport')>();
  return {
    ...actual,
    voxTransport: {
      ...actual.voxTransport,
      mercatusLoadConfig: (...args: unknown[]) => mockLoadConfig(...args),
      mercatusSaveConfig: (...args: unknown[]) => mockSaveConfig(...args),
    },
  };
});

import { Mercatus } from './Mercatus';
import { LanguageProvider } from '../../../hooks/useLanguage';
import { labelFor } from '../../../lib/lexicon';

// Reproduces a live bug: the sidebar nav item for this surface is labeled
// via the lexicon ('Market' in English), but the surface's own header
// hardcoded a different string ("Mercatus"), so the page you land on didn't
// match what you clicked.
describe('Mercatus', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLoadConfig.mockResolvedValue({ _meta: { sources: {} }, watchlist: [] });
    mockSaveConfig.mockResolvedValue(undefined);
  });

  it("headers with the same label the sidebar uses ('mercatus' lexicon key)", () => {
    render(
      <LanguageProvider>
        <Mercatus />
      </LanguageProvider>,
    );
    const expected = labelFor('mercatus', 'en');
    expect(screen.getByRole('heading', { name: new RegExp(expected) })).toBeInTheDocument();
  });

  it('condensed prop renders only the parts/sources summary line, not the coverage table', async () => {
    render(
      <LanguageProvider>
        <Mercatus condensed />
      </LanguageProvider>,
    );
    // Same "N parts · N enabled sources" line the full view already shows,
    // rendered on its own without the coverage matrix or source registry.
    expect(await screen.findByText(/0 parts · 0 enabled sources/i)).toBeInTheDocument();
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('filters watchlist rows when clicking Catalog / Free Tiers / Floor Cost gauges', async () => {
    mockLoadConfig.mockResolvedValue({
      _meta: {
        sources: {
          openrouter: { enabled: true, costUsd: 0.002, cadenceHours: 6, tier: 'frontier' },
        },
      },
      watchlist: [
        {
          id: 'mens-local',
          role: 'local-code',
          model: 'mens/qwen3-8b',
          sources: ['openrouter'],
          ids: { openrouter: 'mens/qwen3-8b' },
          target_usd: 0,
          condition: 'free',
        },
        {
          id: 'claude-sonnet',
          role: 'architect',
          model: 'anthropic/claude-sonnet',
          sources: ['openrouter'],
          ids: { openrouter: 'anthropic/claude-sonnet' },
          target_usd: 3.0,
        },
      ],
    });

    render(
      <LanguageProvider>
        <Mercatus />
      </LanguageProvider>,
    );

    expect(await screen.findByTestId('mercatus-part-row-mens-local')).toBeInTheDocument();
    expect(screen.getByTestId('mercatus-part-row-claude-sonnet')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Free Tiers/i }));
    expect(screen.getByTestId('mercatus-part-row-mens-local')).toBeInTheDocument();
    expect(screen.queryByTestId('mercatus-part-row-claude-sonnet')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: /Floor Cost/i }));
    expect(screen.queryByTestId('mercatus-part-row-mens-local')).toBeNull();
    expect(screen.getByTestId('mercatus-part-row-claude-sonnet')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Catalog/i }));
    expect(screen.getByTestId('mercatus-part-row-mens-local')).toBeInTheDocument();
    expect(screen.getByTestId('mercatus-part-row-claude-sonnet')).toBeInTheDocument();
  });

  it('persists source Switch toggles and target_usd threshold edits via mercatusSaveConfig and rolls back on failure', async () => {
    mockLoadConfig.mockResolvedValue({
      _meta: {
        sources: {
          openrouter: { enabled: true, costUsd: 0.002, cadenceHours: 6, tier: 'frontier' },
          vastai: { enabled: false, costUsd: 0.005, cadenceHours: 12, tier: 'gpu-spot' },
        },
      },
      watchlist: [
        {
          id: 'claude-sonnet',
          role: 'architect',
          model: 'anthropic/claude-sonnet',
          sources: ['openrouter'],
          ids: { openrouter: 'anthropic/claude-sonnet' },
          target_usd: 3.0,
        },
      ],
    });

    render(
      <LanguageProvider>
        <Mercatus />
      </LanguageProvider>,
    );

    const vastSwitch = await screen.findByTestId('mercatus-source-switch-vastai');
    expect(vastSwitch.getAttribute('data-state')).toBe('unchecked');

    fireEvent.click(vastSwitch);
    await waitFor(() => {
      expect(mockSaveConfig).toHaveBeenCalledWith(
        expect.objectContaining({
          _meta: {
            sources: expect.objectContaining({
              vastai: expect.objectContaining({ enabled: true }),
            }),
          },
        }),
      );
      expect(vastSwitch.getAttribute('data-state')).toBe('checked');
    });

    // Reject next source switch toggle -> snapshot rollback + sanitized error banner
    mockSaveConfig.mockRejectedValueOnce(new Error('price-watch config read-only'));
    fireEvent.click(vastSwitch);
    await waitFor(() => {
      expect(vastSwitch.getAttribute('data-state')).toBe('checked');
      expect(screen.getByTestId('mercatus-error-banner').textContent).toContain(
        'price-watch config read-only',
      );
    });

    // Inline target_usd edit -> save + rollback on rejection
    const targetInput = screen.getByTestId('mercatus-target-usd-claude-sonnet') as HTMLInputElement;
    expect(targetInput.value).toBe('3');

    fireEvent.change(targetInput, { target: { value: '2.45' } });
    await waitFor(() => {
      expect(mockSaveConfig).toHaveBeenLastCalledWith(
        expect.objectContaining({
          watchlist: [expect.objectContaining({ id: 'claude-sonnet', target_usd: 2.45 })],
        }),
      );
      expect(targetInput.value).toBe('2.45');
    });

    mockSaveConfig.mockRejectedValueOnce(new Error('disk quota exceeded'));
    fireEvent.change(targetInput, { target: { value: '1.8' } });
    await waitFor(() => {
      expect(targetInput.value).toBe('2.45');
      expect(screen.getByTestId('mercatus-error-banner').textContent).toContain(
        'disk quota exceeded',
      );
    });
  });
});
