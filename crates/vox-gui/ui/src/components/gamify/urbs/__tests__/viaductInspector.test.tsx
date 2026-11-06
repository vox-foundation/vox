import React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { hitTestViaductEdge, layoutTown } from '../layout';
import { DEFAULT_LIVE_FALLBACK_SCAN } from '../repoPresets';
import { ImperiumInspector } from '../ImperiumInspector';

describe('Viaduct Contract Inspector & Quarantine Lock (Phases 273 & 274)', () => {
  const layout = layoutTown(DEFAULT_LIVE_FALLBACK_SCAN, new Set(['core-compiler']));

  it('hitTestViaductEdge resolves a dependency viaduct between two districts', () => {
    const edge = DEFAULT_LIVE_FALLBACK_SCAN.edges![0]!;
    const fromD = layout.districts.find((d) => d.name === edge.from_district_id)!;
    const toD = layout.districts.find((d) => d.name === edge.to_district_id)!;
    const midX = ((fromD.x0 + fromD.x1) / 2 + (toD.x0 + toD.x1) / 2) / 2;
    const midY = ((fromD.y0 + fromD.y1) / 2 + (toD.y0 + toD.y1) / 2) / 2;

    const hit = hitTestViaductEdge(midX, midY, layout.districts, DEFAULT_LIVE_FALLBACK_SCAN.edges!);
    expect(hit).toBeTruthy();
    expect(hit?.from_district_id).toBe(edge.from_district_id);
    expect(hit?.to_district_id).toBe(edge.to_district_id);
  });

  it('renders viaduct contract details and layer violation alert in ImperiumInspector', () => {
    render(
      <ImperiumInspector
        selection={{
          kind: 'viaduct',
          edge: {
            from_district_id: 'core-compiler',
            to_district_id: 'studio-gui',
            from_layer: 1,
            to_layer: 3,
            imported_symbols: ['GuiWindowHandle'],
            call_site_count: 4,
            layer_violation: true,
          },
        }}
        layout={layout}
        harness={{ ci: null, vcs: null, queueLen: null, mcp: null }}
        buildings={{}}
        agentTasks={{}}
        scanRoot="/workspace"
        onSelect={vi.fn()}
        onJumpToTile={vi.fn()}
        onRefreshHarness={vi.fn()}
      />,
    );

    const panel = screen.getByTestId('urbs-viaduct-contract-panel');
    expect(panel.textContent).toContain('core-compiler (L1) → studio-gui (L3)');
    expect(screen.getByTestId('urbs-viaduct-layer-violation-banner').textContent).toContain(
      'LAYER VIOLATION',
    );
  });

  it('toggles Clausura Ferrea district quarantine from ImperiumInspector', () => {
    const onToggleQuarantine = vi.fn();
    render(
      <ImperiumInspector
        selection={{ kind: 'district', name: 'core-compiler' }}
        layout={layout}
        harness={{ ci: null, vcs: null, queueLen: null, mcp: null }}
        buildings={{}}
        agentTasks={{}}
        scanRoot="/workspace"
        quarantinedDistricts={['core-compiler']}
        onToggleQuarantine={onToggleQuarantine}
        onSelect={vi.fn()}
        onJumpToTile={vi.fn()}
        onRefreshHarness={vi.fn()}
      />,
    );

    const btn = screen.getByTestId('urbs-district-quarantine-btn');
    expect(btn.textContent).toContain('🔒 Lock');
    fireEvent.click(btn);
    expect(onToggleQuarantine).toHaveBeenCalledWith('core-compiler', false);
  });
});
