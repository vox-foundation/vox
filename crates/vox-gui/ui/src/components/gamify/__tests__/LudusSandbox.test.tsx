// crates/vox-gui/ui/src/components/gamify/__tests__/LudusSandbox.test.tsx
import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const workspaceTownScan = vi.fn();
const openLocator = vi.fn();
const mockSetSessionContextPins = vi.fn();
const mockSetGuiPreference = vi.fn();
const mockRecordGuiEvent = vi.fn();

vi.mock('../../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../transport')>();
  return {
    ...actual,
    listenAgentEvents: vi.fn().mockRejectedValue(new Error('not in tauri')),
    getSessionContextPins: vi.fn().mockResolvedValue([]),
    setSessionContextPins: (...a: unknown[]) => mockSetSessionContextPins(...a),
    voxTransport: {
      ...actual.voxTransport,
      getLlmSpend: vi.fn().mockRejectedValue(new Error('unavailable')),
      workspaceTownScan: (...a: unknown[]) => workspaceTownScan(...a),
      openLocator: (...a: unknown[]) => openLocator(...a),
      harnessCiFleetStatus: vi.fn().mockRejectedValue(new Error('unavailable')),
      vcsTownStatus: vi.fn().mockRejectedValue(new Error('unavailable')),
      hopperList: vi.fn().mockRejectedValue(new Error('unavailable')),
      callTool: vi.fn().mockResolvedValue({}),
      executeCommand: vi.fn().mockResolvedValue({}),
      getGuiPreference: vi.fn().mockResolvedValue(null),
      setGuiPreference: (...a: unknown[]) => mockSetGuiPreference(...a),
      recordGuiEvent: (...a: unknown[]) => mockRecordGuiEvent(...a),
    },
  };
});

import { LudusSandbox } from '../LudusSandbox';
import { URBS_REPO_PRESETS } from '../urbs/repoPresets';

beforeEach(() => {
  workspaceTownScan.mockReset();
  openLocator.mockReset();
  mockSetSessionContextPins.mockReset();
  mockSetGuiPreference.mockReset();
  mockRecordGuiEvent.mockReset();
  workspaceTownScan.mockResolvedValue(URBS_REPO_PRESETS.live.scan);
  mockSetSessionContextPins.mockResolvedValue([]);
  mockSetGuiPreference.mockResolvedValue(undefined);
  mockRecordGuiEvent.mockResolvedValue({ xpGranted: 50, lumensGranted: 8, achievementTitle: null });
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
    const user = userEvent.setup();
    render(<LudusSandbox />);

    const presetSelect = await screen.findByTestId('urbs-repo-preset-select');
    await user.click(presetSelect);
    await user.click(await screen.findByRole('option', { name: URBS_REPO_PRESETS.ts_monorepo.label }));

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
    await user.click(presetSelect);
    await user.click(await screen.findByRole('option', { name: URBS_REPO_PRESETS.python_ml.label }));
    await waitFor(() => {
      expect(screen.getByText(/Confirm HNSW ef_construction index rebuild/i)).toBeTruthy();
    });

    // Switch to Go Distributed Microservices preset
    await user.click(presetSelect);
    await user.click(await screen.findByRole('option', { name: URBS_REPO_PRESETS.go_microservices.label }));
    await waitFor(() => {
      expect(screen.getByText(/Approve Raft quorum snapshot interval change/i)).toBeTruthy();
    });
  });

  it('persists Pin Context via setSessionContextPins and Yield claim via setGuiPreference with rollback on failure', async () => {
    render(<LudusSandbox />);

    const errBtn = (await screen.findAllByTestId('urbs-priority-item-error'))[0]!;
    fireEvent.click(errBtn);

    const pinBtn = await screen.findByTestId('urbs-action-pin-context');
    fireEvent.click(pinBtn);
    await waitFor(() => {
      expect(mockSetSessionContextPins).toHaveBeenCalledWith(
        'default',
        expect.arrayContaining([expect.objectContaining({ scope: 'urbs' })]),
      );
    });

    const yieldBtn = await screen.findByTestId('urbs-action-claim-yield');
    expect(yieldBtn.textContent).toContain('250 XP');
    fireEvent.click(yieldBtn);
    await waitFor(() => {
      expect(mockSetGuiPreference).toHaveBeenCalledWith('vox.urbs.claimed_yield_xp.v1', '300');
      expect(yieldBtn.textContent).toContain('300 XP');
    });

    mockSetGuiPreference.mockRejectedValueOnce(new Error('pref store locked'));
    fireEvent.click(yieldBtn);
    await waitFor(() => {
      expect(yieldBtn.textContent).toContain('300 XP');
      expect(screen.getByTestId('urbs-dev-action-receipt').textContent).toMatch(
        /Yield claim failed:.*pref store locked/i,
      );
    });
  });
});
