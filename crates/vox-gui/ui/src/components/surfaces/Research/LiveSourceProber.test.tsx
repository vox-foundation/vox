// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';
import { LiveSourceProber, type ProviderProbeResult } from './LiveSourceProber';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

describe('LiveSourceProber', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockReset();
  });

  it('renders the query input, provider selector, and probe button', () => {
    render(<LiveSourceProber initialQuery="Rust async" />);

    expect(screen.getByTestId('live-source-prober')).toBeTruthy();
    const queryInput = screen.getByLabelText('Probe query') as HTMLInputElement;
    expect(queryInput.value).toBe('Rust async');

    const providerSelect = screen.getByLabelText('Search provider') as HTMLSelectElement;
    expect(providerSelect.value).toBe('all');

    const probeBtn = screen.getByRole('button', { name: /probe provider/i });
    expect(probeBtn.getAttribute('type')).toBe('button');
  });

  it('invokes probe_all_search_providers when provider is all and displays latency and hit count', async () => {
    const mockResults: ProviderProbeResult[] = [
      {
        provider: 'duckduckgo',
        http_status: 200,
        latency_ms: 142,
        success: true,
        hit_count: 5,
        sample_titles: ['Title 1', 'Title 2'],
        error_message: null,
        remediation_tip: null,
      },
    ];

    invokeMock.mockResolvedValueOnce(mockResults);

    render(<LiveSourceProber initialQuery="Rust language" />);
    const probeBtn = screen.getByRole('button', { name: /probe provider/i });
    fireEvent.click(probeBtn);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('probe_all_search_providers', {
        query: 'Rust language',
      });
    });

    await waitFor(() => {
      expect(screen.getByText('duckduckgo')).toBeTruthy();
      expect(screen.getByText('142 ms')).toBeTruthy();
      expect(screen.getByText('5 hits')).toBeTruthy();
      expect(screen.getByText('Title 1')).toBeTruthy();
      expect(screen.getByText('Title 2')).toBeTruthy();
      expect(screen.getByText('HTTP 200')).toBeTruthy();
    });
  });

  it('invokes probe_search_provider when a specific provider is selected', async () => {
    const mockResult: ProviderProbeResult = {
      provider: 'tavily',
      http_status: 200,
      latency_ms: 88,
      success: true,
      hit_count: 3,
      sample_titles: ['Tavily Result 1'],
      error_message: null,
      remediation_tip: null,
    };

    invokeMock.mockResolvedValueOnce(mockResult);

    render(<LiveSourceProber initialQuery="Vox agent" />);

    const select = screen.getByLabelText('Search provider');
    fireEvent.change(select, { target: { value: 'tavily' } });

    const probeBtn = screen.getByRole('button', { name: /probe provider/i });
    fireEvent.click(probeBtn);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('probe_search_provider', {
        provider: 'tavily',
        query: 'Vox agent',
      });
    });

    await waitFor(() => {
      expect(screen.getByText('tavily')).toBeTruthy();
      expect(screen.getByText('88 ms')).toBeTruthy();
      expect(screen.getByText('3 hits')).toBeTruthy();
      expect(screen.getByText('Tavily Result 1')).toBeTruthy();
    });
  });

  it('displays remediation tip when provider is unconfigured or erroring', async () => {
    const unconfiguredResult: ProviderProbeResult = {
      provider: 'searxng',
      http_status: 0,
      latency_ms: 0,
      success: false,
      hit_count: 0,
      sample_titles: [],
      error_message: 'SearXNG URL is not configured',
      remediation_tip: 'Set VOX_SEARCH_SEARXNG_URL in environment or settings',
    };

    invokeMock.mockResolvedValueOnce([unconfiguredResult]);

    render(<LiveSourceProber initialQuery="Testing unconfigured" />);
    const probeBtn = screen.getByRole('button', { name: /probe provider/i });
    fireEvent.click(probeBtn);

    await waitFor(() => {
      expect(screen.getByText('searxng')).toBeTruthy();
      expect(screen.getByText('Unconfigured (HTTP 0)')).toBeTruthy();
      expect(
        screen.getByText(/Set VOX_SEARCH_SEARXNG_URL in environment or settings/i)
      ).toBeTruthy();
    });
  });

  it('handles backend invoke error gracefully', async () => {
    invokeMock.mockRejectedValueOnce('Failed to connect to backend probe');

    render(<LiveSourceProber initialQuery="Query failing" />);
    const probeBtn = screen.getByRole('button', { name: /probe provider/i });
    fireEvent.click(probeBtn);

    await waitFor(() => {
      expect(screen.getByText('Failed to connect to backend probe')).toBeTruthy();
    });
  });

  it('sanitizes internal IPC leak errors using sanitizeErrorForToast', async () => {
    invokeMock.mockRejectedValueOnce(new Error('TypeError: __TAURI_INTERNALS__ is undefined'));

    render(<LiveSourceProber initialQuery="Query leak" />);
    const probeBtn = screen.getByRole('button', { name: /probe provider/i });
    fireEvent.click(probeBtn);

    await waitFor(() => {
      expect(screen.getByText('An unexpected error occurred.')).toBeTruthy();
    });
  });
});
