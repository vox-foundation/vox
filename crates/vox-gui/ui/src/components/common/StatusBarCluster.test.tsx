// @vitest-environment jsdom
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { StatusBarCluster } from './StatusBarCluster';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'get_research_engine_status') {
      return {
        active_lane: 'fast',
        fast_timeout_ms: 2500,
        deep_timeout_ms: 15000,
        providers: [
          {
            id: 'tavily',
            name: 'Tavily',
            is_keyless: false,
            is_enabled: true,
            has_key: true,
            quota_usage: { units_spent: 250, units_limit: 1000 },
          },
        ],
        free_key_offers: [],
      };
    }
    return null;
  }),
}));

describe('StatusBarCluster', () => {
  it('toggles the research popover on trigger click', async () => {
    render(<StatusBarCluster />);
    const trigger = screen.getByTestId('status-bar-cluster-trigger');
    expect(screen.queryByTestId('status-bar-cluster-popover')).not.toBeInTheDocument();

    fireEvent.click(trigger);
    expect(await screen.findByTestId('status-bar-cluster-popover')).toBeInTheDocument();
    expect(await screen.findByText('Providers')).toBeInTheDocument();
    expect(screen.getByText('Lane')).toBeInTheDocument();
  });

  it('closes popover on Escape and restores focus to trigger', async () => {
    render(<StatusBarCluster />);
    const trigger = screen.getByTestId('status-bar-cluster-trigger');
    fireEvent.click(trigger);
    expect(await screen.findByTestId('status-bar-cluster-popover')).toBeInTheDocument();

    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => {
      expect(screen.queryByTestId('status-bar-cluster-popover')).not.toBeInTheDocument();
    });
    expect(document.activeElement).toBe(trigger);
  });

  it('invokes onOpenDrawer when Configure button is clicked', async () => {
    const onOpenDrawer = vi.fn();
    render(<StatusBarCluster onOpenDrawer={onOpenDrawer} />);
    const trigger = screen.getByTestId('status-bar-cluster-trigger');
    fireEvent.click(trigger);
    const configBtn = await screen.findByRole('button', { name: /Configure Engines & Keys/i });
    fireEvent.click(configBtn);
    expect(onOpenDrawer).toHaveBeenCalled();
  });
});

describe('StatusBarCluster honesty (plan 3a)', () => {
  const provider = (id: string, name: string, over: Record<string, unknown> = {}) => ({
    id,
    name,
    is_keyless: true,
    is_enabled: true,
    has_key: false,
    quota_usage: null,
    ...over,
  });
  const status = (over: Record<string, unknown> = {}) => ({
    active_lane: 'fast',
    fast_timeout_ms: 4000,
    deep_timeout_ms: 15000,
    providers: [],
    free_key_offers: [],
    ...over,
  });

  it('lists each provider exactly as the engine reports it, with no hardcoded health', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async () =>
      status({
        providers: [
          provider('wikipedia', 'Wikipedia'),
          provider('arxiv', 'arXiv', { is_enabled: false }),
          provider('tavily', 'Tavily Search', { is_keyless: false, has_key: false }),
        ],
      }),
    );
    render(<StatusBarCluster />);
    fireEvent.click(screen.getByTestId('status-bar-cluster-trigger'));
    expect(await screen.findByTestId('status-bar-cluster-provider-wikipedia')).toHaveTextContent('Wikipediaon');
    expect(screen.getByTestId('status-bar-cluster-provider-arxiv')).toHaveTextContent('arXivoff');
    expect(screen.getByTestId('status-bar-cluster-provider-tavily')).toHaveTextContent('Tavily Searchon · no key');
    expect(screen.queryByTestId('status-bar-cluster-provider-openalex')).toBeNull();
    const popover = screen.getByTestId('status-bar-cluster-popover');
    expect(popover).not.toHaveTextContent('✓');
    expect(popover).not.toHaveTextContent(/online/i);
    expect(popover).not.toHaveTextContent(/plaintext/i);
    expect(popover).toHaveTextContent('Fast (≤4s)');
  });

  it('a failed status fetch reads "unknown" and drops an earlier reading (no stale green)', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockImplementation(async () => status({ providers: [provider('wikipedia', 'Wikipedia')] }));
    render(<StatusBarCluster />);
    const trigger = screen.getByTestId('status-bar-cluster-trigger');
    fireEvent.click(trigger);
    expect(await screen.findByTestId('status-bar-cluster-provider-wikipedia')).toBeInTheDocument();
    fireEvent.click(trigger);
    vi.mocked(invoke).mockImplementation(async () => {
      throw new Error('daemon down');
    });
    fireEvent.click(trigger);
    expect(await screen.findByTestId('status-bar-cluster-unknown')).toBeInTheDocument();
    expect(screen.queryByTestId('status-bar-cluster-provider-wikipedia')).toBeNull();
    expect(screen.getByTestId('status-bar-cluster-popover')).not.toHaveTextContent('✓');
    expect(trigger).toHaveTextContent('unknown');
  });
});

