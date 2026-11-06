// crates/vox-gui/ui/src/components/gamify/__tests__/LudusSandbox.test.tsx
import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

const workspaceTownScan = vi.fn();
const openLocator = vi.fn();
vi.mock('../../../transport', () => ({
  listenAgentEvents: vi.fn().mockRejectedValue(new Error('not in tauri')),
  voxTransport: {
    getLlmSpend: vi.fn().mockRejectedValue(new Error('unavailable')),
    workspaceTownScan: (...a: unknown[]) => workspaceTownScan(...a),
    openLocator: (...a: unknown[]) => openLocator(...a),
    harnessCiFleetStatus: vi.fn().mockRejectedValue(new Error('unavailable')),
    vcsTownStatus: vi.fn().mockRejectedValue(new Error('unavailable')),
    hopperList: vi.fn().mockRejectedValue(new Error('unavailable')),
    callTool: vi.fn().mockResolvedValue({}),
    executeCommand: vi.fn().mockResolvedValue({}),
  },
}));

import { LudusSandbox } from '../LudusSandbox';
import { URBS_REPO_PRESETS } from '../urbs/repoPresets';

beforeEach(() => {
  workspaceTownScan.mockReset();
  openLocator.mockReset();
  workspaceTownScan.mockResolvedValue(URBS_REPO_PRESETS.live.scan);
});

describe('LudusSandbox (Phase 264: Multi-Repo Simulation Presets, Focus Mode & Priority Dock)', () => {
  it('renders the Live Developer Priority Dock, Repo Preset Switcher, and Focus Mode Switcher', async () => {
    render(<LudusSandbox />);

    await waitFor(() => expect(screen.getByTestId('urbs-priority-dock')).toBeTruthy());
    expect(screen.getByTestId('urbs-repo-preset-select')).toBeTruthy();
    expect(screen.getByTestId('urbs-focus-mode-select')).toBeTruthy();

    // Priority dock surfaces Needs You gate, Error item, Active Agent item, and Git Diff item
    expect(screen.getAllByTestId('urbs-priority-item-gate').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByTestId('urbs-priority-item-error').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByTestId('urbs-priority-item-agent').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByTestId('urbs-priority-item-diff').length).toBeGreaterThanOrEqual(1);
  });

  it('switches dynamically across TypeScript, Python ML, and Go Microservices presets and opens ImperiumInspector on priority dock click', async () => {
    render(<LudusSandbox />);

    const presetSelect = await screen.findByTestId('urbs-repo-preset-select');
    fireEvent.change(presetSelect, { target: { value: 'ts_monorepo' } });

    await waitFor(() => {
      expect(screen.getAllByText(/TypeScript/i).length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText(/Approve webhook signature rotation migration/i)).toBeTruthy();
    });

    // Click the priority dock error item (`pricing_compiler.ts`) to focus & open ImperiumInspector
    const errBtn = screen.getAllByTestId('urbs-priority-item-error')[0]!;
    fireEvent.click(errBtn);

    await waitFor(() => {
      expect(screen.getByTestId('urbs-imperium-inspector')).toBeTruthy();
      expect(screen.getByText(/DISTRICT PREFECTURE: CHECKOUT-ENGINE/i)).toBeTruthy();
    });

    // Switch to Python FastAPI + ML Service preset
    fireEvent.change(presetSelect, { target: { value: 'python_ml' } });
    await waitFor(() => {
      expect(screen.getByText(/Confirm HNSW ef_construction index rebuild/i)).toBeTruthy();
    });

    // Switch to Go Distributed Microservices preset
    fireEvent.change(presetSelect, { target: { value: 'go_microservices' } });
    await waitFor(() => {
      expect(screen.getByText(/Approve Raft quorum snapshot interval change/i)).toBeTruthy();
    });
  });
});
