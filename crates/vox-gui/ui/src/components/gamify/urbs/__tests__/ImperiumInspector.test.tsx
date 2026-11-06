// crates/vox-gui/ui/src/components/gamify/urbs/__tests__/ImperiumInspector.test.tsx
import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ImperiumInspector } from '../ImperiumInspector';
import { layoutTown } from '../layout';
import type { TownScan } from '../types';

vi.mock('../../../../transport', () => ({
  voxTransport: {
    hopperList: vi.fn().mockResolvedValue([]),
    callTool: vi.fn().mockResolvedValue({}),
    executeCommand: vi.fn().mockResolvedValue({}),
    setRoutingPriority: vi.fn().mockResolvedValue({}),
    openLocator: vi.fn().mockResolvedValue({}),
    doubtTask: vi.fn().mockResolvedValue({}),
  },
}));

const mockCapture = vi.fn().mockResolvedValue({ checkpoint_id: 'ckpt-99' });
const mockRestore = vi.fn().mockResolvedValue({ restored: true });

vi.mock('../../../../transport/t3Wave3Rpc', () => ({
  vcsCheckpointCapture: (...args: unknown[]) => mockCapture(...args),
  vcsCheckpointRestore: (...args: unknown[]) => mockRestore(...args),
}));

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

  it('renders repo-agnostic DEPENDENCY & BLAST-RADIUS DAG, upstream providers, downstream blast radius, and git diff lines', () => {
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
    expect(screen.getByText(/⬆ core-types/i)).toBeTruthy();
    expect(screen.getByText(/⚡ checkout-api/i)).toBeTruthy();
    expect(screen.getByText(/⚡ storefront-web/i)).toBeTruthy();

    // Per-file git diff badge
    expect(screen.getByText(/M \+28\/-6/i)).toBeTruthy();

    // Triage buttons
    const testImpactedBtn = screen.getByRole('button', { name: /Test Impacted \(2\)/i });
    fireEvent.click(testImpactedBtn);
    expect(screen.getByText(/Testing 3 impacted module\(s\): auth-engine, checkout-api, storefront-web/i)).toBeTruthy();
  });

  it('renders PORTA GIT Stone Milestone Checkpoints and triggers vcsCheckpointCapture / vcsCheckpointRestore', () => {
    const layout = layoutTown(polyglotScan, new Set(['auth-engine']));
    const onRefreshHarness = vi.fn();

    render(
      <ImperiumInspector
        selection={{ kind: 'landmark', id: 'PORTA' }}
        layout={layout}
        scanRoot={polyglotScan.root}
        harness={{
          ci: null,
          vcs: { branches: [{ name: 'feat/vitruvian', isHead: true }], prs: [] },
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
    expect(mockCapture).toHaveBeenCalledWith('default', 'milestone', {
      note: 'PORTA GIT milestone on feat/vitruvian',
    });

    fireEvent.click(screen.getByTestId('urbs-restore-checkpoint-ckpt-1'));
    expect(mockRestore).toHaveBeenCalledWith('default', 'ckpt-1');
  });
});
