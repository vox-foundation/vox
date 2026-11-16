// crates/vox-gui/ui/src/components/gamify/urbs/__tests__/ImperiumInspector.test.tsx
import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ImperiumInspector } from '../ImperiumInspector';
import { layoutTown } from '../layout';
import type { TownScan } from '../types';

const mockSetGuiPreference = vi.fn().mockResolvedValue(undefined);
const mockGetGuiPreference = vi.fn().mockResolvedValue(null);
const mockInvokeMcpTool = vi.fn().mockResolvedValue({ is_error: false, result: { approvals: [] } });

vi.mock('../../../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../../transport')>();
  return {
    ...actual,
    voxTransport: {
      ...actual.voxTransport,
      hopperList: vi.fn().mockResolvedValue([]),
      callTool: vi.fn().mockResolvedValue({}),
      executeCommand: vi.fn().mockResolvedValue({}),
      setRoutingPriority: vi.fn().mockResolvedValue({}),
      openLocator: vi.fn().mockResolvedValue({}),
      doubtTask: vi.fn().mockResolvedValue({}),
      getGuiPreference: (...args: unknown[]) => mockGetGuiPreference(...args),
      setGuiPreference: (...args: unknown[]) => mockSetGuiPreference(...args),
      invokeMcpTool: (...args: unknown[]) => mockInvokeMcpTool(...args),
    },
  };
});

const mockCapture = vi.fn().mockResolvedValue({
  thread_id: 'default',
  turn_count: 3,
  ref_name: 'refs/vox/checkpoints/default/restore-point/porta-feat/vitruvian',
  commit_oid: 'abc1234',
  is_restore_point: true,
});
const mockRestore = vi.fn().mockResolvedValue({
  thread_id: 'default',
  restored_turn: 1,
  ref_name: 'refs/vox/checkpoints/default/turn/1',
});

vi.mock('../../../../transport/t3Wave3Rpc', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../../transport/t3Wave3Rpc')>();
  return {
    ...actual,
    vcsCheckpointCapture: (...args: unknown[]) => mockCapture(...args),
    vcsCheckpointRestore: (...args: unknown[]) => mockRestore(...args),
  };
});

describe('ImperiumInspector (Phase 263: Dependency Blast-Radius & Polyglot Architecture Inspector)', () => {
  const polyglotScan: TownScan = {
    root: '/workspace/ts-app',
    scanned_at_ms: 0,
    truncated: false,
    crates: [
      {
        name: 'core-types',
        root: 'packages/core-types',
        files: [{ path: 'packages/core-types/src/index.ts', lines: 140 }],
        deps: [],
      },
      {
        name: 'auth-engine',
        root: 'packages/auth-engine',
        files: [
          {
            path: 'packages/auth-engine/src/jwt.ts',
            lines: 260,
            git_status: 'M',
            diff_added_lines: 28,
            diff_deleted_lines: 6,
          },
        ],
        deps: ['core-types'],
        dirty_file_count: 1,
        total_churn_lines: 34,
      },
      {
        name: 'checkout-api',
        root: 'services/checkout-api',
        files: [{ path: 'services/checkout-api/src/router.ts', lines: 380 }],
        deps: ['auth-engine'],
      },
      {
        name: 'storefront-web',
        root: 'apps/storefront-web',
        files: [{ path: 'apps/storefront-web/src/App.tsx', lines: 420 }],
        deps: ['checkout-api'],
      },
    ],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockGetGuiPreference.mockResolvedValue(null);
    mockSetGuiPreference.mockResolvedValue(undefined);
    mockInvokeMcpTool.mockResolvedValue({ is_error: false, result: { approvals: [] } });
  });

  it('renders repo-agnostic DEPENDENCY & BLAST-RADIUS DAG, RomanRotaryKnob slider, and persists routing weight with rollback', async () => {
    const layout = layoutTown(polyglotScan, new Set(['auth-engine']));
    const onSelect = vi.fn();
    const onJumpToTile = vi.fn();

    render(
      <ImperiumInspector
        selection={{ kind: 'district', name: 'auth-engine' }}
        layout={layout}
        scanRoot={polyglotScan.root}
        harness={{ ci: null, vcs: null, queueLen: 0, mcp: null }}
        onSelect={onSelect}
        onJumpToTile={onJumpToTile}
        onRefreshHarness={vi.fn()}
      />,
    );

    expect(screen.getByText(/DISTRICT PREFECTURE: AUTH-ENGINE/i)).toBeTruthy();
    expect(screen.getByText(/DEPENDENCY & BLAST-RADIUS DAG/i)).toBeTruthy();
    expect(screen.getByText(/Upstream Providers: 1/i)).toBeTruthy();
    expect(screen.getByText(/Blast Radius: 2 downstream/i)).toBeTruthy();

    // Upstream provider pill & downstream blast-radius pills
    const coreTypesBtn = screen.getByRole('button', { name: /core-types/i });
    expect(coreTypesBtn.querySelector('[data-icon="arrowUp"]')).toBeTruthy();
    const checkoutBtn = screen.getByRole('button', { name: /checkout-api/i });
    expect(checkoutBtn.querySelector('svg')).toBeTruthy();
    const storefrontBtn = screen.getByRole('button', { name: /storefront-web/i });
    expect(storefrontBtn.querySelector('svg')).toBeTruthy();

    // Per-file git diff badge
    expect(screen.getByText(/M \+28\/-6/i)).toBeTruthy();

    // RomanRotaryKnob (role="slider") commit + rollback on failure
    const knob = screen.getByRole('slider', { name: /District Model Routing Tier/i });
    expect(knob.getAttribute('aria-valuenow')).toBe('65');
    fireEvent.keyDown(knob, { key: 'ArrowUp' });
    await waitFor(() => {
      expect(mockSetGuiPreference).toHaveBeenCalledWith('vox.routing.exploration_pct.v1', '70');
      expect(knob.getAttribute('aria-valuenow')).toBe('70');
    });

    mockSetGuiPreference.mockRejectedValueOnce(new Error('disk write denied'));
    fireEvent.keyDown(knob, { key: 'ArrowUp' });
    await waitFor(() => {
      expect(knob.getAttribute('aria-valuenow')).toBe('70');
      expect(screen.getByText(/Routing update failed:.*disk write denied/i)).toBeTruthy();
    });

    // Triage buttons
    const testImpactedBtn = screen.getByRole('button', { name: /Test Impacted \(2\)/i });
    fireEvent.click(testImpactedBtn);
    expect(screen.getByText(/Testing 3 impacted module\(s\): auth-engine, checkout-api, storefront-web/i)).toBeTruthy();
  });

  it('renders PORTA GIT Stone Milestone Checkpoints and triggers typed vcsCheckpointCapture / vcsCheckpointRestore', async () => {
    const layout = layoutTown(polyglotScan, new Set(['auth-engine']));
    const onRefreshHarness = vi.fn();

    render(
      <ImperiumInspector
        selection={{ kind: 'landmark', id: 'PORTA' }}
        layout={layout}
        scanRoot={polyglotScan.root}
        harness={{
          ci: null,
          vcs: { branches: [{ name: 'feat/vitruvian', track: 'origin/feat/vitruvian', isHead: true }], prs: [] },
          queueLen: 0,
          mcp: null,
          checkpoints: [
            { id: 'ckpt-1', turnId: 'turn-1', note: 'Before compiler refactor', createdAtMs: 1000 },
            { id: 'ckpt-2', turnId: 'turn-2', note: 'Green test suite', createdAtMs: 2000 },
          ],
        }}
        onSelect={vi.fn()}
        onJumpToTile={vi.fn()}
        onRefreshHarness={onRefreshHarness}
      />,
    );

    expect(screen.getByText(/PORTA GIT · Milestone Road & Portcullis/i)).toBeTruthy();
    expect(screen.getByText(/\[I\] Before compiler refactor/i)).toBeTruthy();
    expect(screen.getByText(/\[II\] Green test suite/i)).toBeTruthy();

    fireEvent.click(screen.getByTestId('urbs-capture-checkpoint-btn'));
    expect(mockCapture).toHaveBeenCalledWith('default', 3, 'porta-feat/vitruvian');
    await waitFor(() => expect(onRefreshHarness).toHaveBeenCalledTimes(1));

    fireEvent.click(screen.getByTestId('urbs-restore-checkpoint-ckpt-1'));
    expect(mockRestore).toHaveBeenCalledWith('default', 1);
    await waitFor(() => expect(onRefreshHarness).toHaveBeenCalledTimes(2));
  });

  it('wires PRAETORIVM Approve button to vox_resolve_approval and rolls back on failure', async () => {
    const layout = layoutTown(polyglotScan, new Set(['auth-engine']));
    const onRefreshHarness = vi.fn();
    mockInvokeMcpTool.mockImplementation(async (tool: string) => {
      if (tool === 'vox_pending_approvals') {
        return { is_error: false, result: { approvals: [] } };
      }
      if (tool === 'vox_resolve_approval') {
        throw new Error('approval daemon unreachable');
      }
      return { is_error: false, result: {} };
    });

    render(
      <ImperiumInspector
        selection={{ kind: 'landmark', id: 'PRAETORIVM' }}
        layout={layout}
        scanRoot={polyglotScan.root}
        harness={{
          ci: null,
          vcs: null,
          queueLen: 0,
          mcp: null,
          attentionGates: [{ id: 'gate-42', title: 'Approve schema migration' }],
        }}
        onSelect={vi.fn()}
        onJumpToTile={vi.fn()}
        onRefreshHarness={onRefreshHarness}
      />,
    );

    const approveBtn = await screen.findByTestId('urbs-praetorium-approve-gate-42');
    fireEvent.click(approveBtn);

    await waitFor(() => {
      expect(mockInvokeMcpTool).toHaveBeenCalledWith('vox_resolve_approval', {
        approval_id: 'gate-42',
        outcome: 'approved',
      });
      expect(screen.getByTestId('urbs-praetorium-approve-gate-42')).toBeTruthy();
      expect(screen.getByText(/Approval failed:.*approval daemon unreachable/i)).toBeTruthy();
    });
  });
});
