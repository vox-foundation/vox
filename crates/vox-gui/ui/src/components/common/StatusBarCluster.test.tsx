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
  it('toggles 4-quadrant popover on trigger click', async () => {
    render(<StatusBarCluster />);
    const trigger = screen.getByTestId('status-bar-cluster-trigger');
    expect(screen.queryByTestId('status-bar-cluster-popover')).not.toBeInTheDocument();

    fireEvent.click(trigger);
    expect(await screen.findByTestId('status-bar-cluster-popover')).toBeInTheDocument();
    expect(screen.getByText(/Keyless Engines/i)).toBeInTheDocument();
    expect(screen.getByText(/Search Quotas/i)).toBeInTheDocument();
    expect(screen.getByText(/Lane Routing/i)).toBeInTheDocument();
    expect(screen.getByText(/Clavis Vault/i)).toBeInTheDocument();
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
