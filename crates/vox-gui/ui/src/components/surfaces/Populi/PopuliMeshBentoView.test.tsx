import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { __resetBackendAvailabilityForTests } from '../../../lib/backendGuard';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { PopuliMeshBentoView } from './PopuliMeshBentoView';
import { surfaceDecorators } from '../decoratorRegistry';

const MOCK_NODES = [
  {
    id: 'node-alpha',
    status: 'online',
    vram_gb: 24,
    host_triple: 'aarch64-apple-darwin',
    gpu_summary: 'Apple M3 Max · 24 GB Unified',
    trust_tier: 'verified',
    advertised_models: ['mens-8b', 'mens/e2e-smoke-metal'],
    last_seen_unix_ms: Date.now() - 10_000,
  },
  {
    id: 'node-beta',
    status: 'online',
    vram_gb: 16,
    host_triple: 'x86_64-unknown-linux-gnu',
    gpu_summary: 'NVIDIA RTX 4080 · 16 GB VRAM',
    trust_tier: 'trusted',
    advertised_models: ['qwen-coder-7b'],
    last_seen_unix_ms: Date.now() - 25_000,
  },
  {
    id: 'node-gamma',
    status: 'quarantined',
    vram_gb: 8,
    host_triple: 'x86_64-pc-windows-msvc',
    gpu_summary: 'RTX 3070 · 8 GB VRAM',
    trust_tier: 'probation',
    advertised_models: [],
    last_seen_unix_ms: Date.now() - 120_000,
  },
];

describe('PopuliMeshBentoView (Slice 435-A · AXIS-BENTO-01)', () => {
  beforeEach(() => {
    window.localStorage.clear();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_gui_preference') return Promise.resolve(null);
      if (cmd === 'set_gui_preference') return Promise.resolve();
      if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_mesh_nodes') {
        return Promise.resolve({
          is_error: false,
          result: {
            source: 'control_plane',
            queue_depth: 1,
            nodes: MOCK_NODES,
          },
        });
      }
      if (cmd === 'execute_command') {
        const path = (args?.path as string[] | undefined)?.join(' ');
        if (path === 'populi status') {
          return Promise.resolve({
            exit_code: 0,
            stdout: 'mesh: 2/3 online | overlay: quic-tls13 healthy',
            stderr: '',
          });
        }
        if (path === 'populi registry-snapshot') {
          return Promise.resolve({
            exit_code: 0,
            stdout: 'quorum: 2/3 | attestation: ed25519-sealed',
            stderr: '',
          });
        }
        return Promise.resolve({ exit_code: 0, stdout: 'ok', stderr: '' });
      }
      return Promise.resolve(null);
    });
  });

  it('registers PopuliMeshBentoView on surfaceDecorators.populi and renders SVG QUIC chart + MasterDetailWorkbench', async () => {
    expect(surfaceDecorators.populi).toBe(PopuliMeshBentoView);
    const pushToast = vi.fn();
    const { container } = render(<PopuliMeshBentoView pushToast={pushToast} />);

    expect(screen.getByRole('heading', { name: 'Vox Populi' })).toBeInTheDocument();
    const chartSvg = screen.getByTestId('populi-quic-topology-svg');
    expect(chartSvg).toHaveAttribute('data-cv-chart', 'true');
    expect(screen.getByRole('region', { name: 'Populi mesh peer matrix' })).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('2/3 UP')).toBeInTheDocument();
      expect(screen.getByText('40 GB')).toBeInTheDocument();
      expect(screen.getByText('2/2 OK')).toBeInTheDocument();
      expect(screen.getByTestId('populi-peer-row-node-alpha')).toBeInTheDocument();
      expect(screen.getByTestId('populi-peer-row-node-beta')).toBeInTheDocument();
      expect(screen.getByTestId('populi-peer-row-node-gamma')).toBeInTheDocument();
    });

    expect(container.querySelectorAll('pre').length).toBe(0);
    expect(screen.getByTestId('populi-snapshot-summary')).toHaveTextContent('ed25519-sealed');
    expect(screen.getByTestId('populi-hybrid-compute-grid')).toBeInTheDocument();
    expect(screen.getByTestId('populi-lock-matrix')).toBeInTheDocument();
    expect(chartSvg.querySelector('rect')?.getAttribute('fill')).toBe('var(--color-bg-surface)');
  });

  it('selects a peer when clicked in the SVG QUIC topology and filters peers via RomanBrassGauge dials', async () => {
    const pushToast = vi.fn();
    render(<PopuliMeshBentoView pushToast={pushToast} />);

    await waitFor(() => {
      expect(screen.getByTestId('populi-topology-node-node-beta')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('populi-topology-node-node-beta'));

    await waitFor(() => {
      const inspector = screen.getByTestId('populi-peer-inspector');
      expect(inspector).toHaveTextContent('node-beta');
      expect(inspector).toHaveTextContent('x86_64-unknown-linux-gnu');
      expect(inspector).toHaveTextContent('NVIDIA RTX 4080 · 16 GB VRAM');
      expect(inspector).toHaveTextContent('qwen-coder-7b');
    });

    // Click Probes gauge to filter degraded peers (node-gamma only)
    const probesGauge = screen.getByRole('button', { name: /Probes/i });
    fireEvent.click(probesGauge);

    expect(screen.getByTestId('populi-mesh-bento-view')).toHaveAttribute('data-peer-filter', 'degraded');
    expect(screen.queryByTestId('populi-peer-row-node-alpha')).toBeNull();
    expect(screen.getByTestId('populi-peer-row-node-gamma')).toBeInTheDocument();
  });

  it('persists gossip cadence knob with rollback on error and re-runs overlay probes on button click', async () => {
    const pushToast = vi.fn();
    render(<PopuliMeshBentoView pushToast={pushToast} />);

    await waitFor(() => expect(screen.getByText('2/2 OK')).toBeInTheDocument());

    const cadenceKnob = screen.getByRole('slider', { name: 'Populi gossip heartbeat cadence' });
    expect(cadenceKnob).toHaveAttribute('aria-valuenow', '250');

    fireEvent.keyDown(cadenceKnob, { key: 'ArrowUp' });
    await waitFor(() => {
      expect(cadenceKnob).toHaveAttribute('aria-valuenow', '300');
      expect(invokeMock).toHaveBeenCalledWith('set_gui_preference', {
        key: 'vox.populi.gossip_cadence_ms.v1',
        value: '300',
      });
    });

    invokeMock.mockImplementationOnce(() =>
      Promise.reject(new TypeError('window.__TAURI_INTERNALS__ broken')),
    );
    fireEvent.keyDown(cadenceKnob, { key: 'ArrowUp' });
    await waitFor(() => {
      expect(cadenceKnob).toHaveAttribute('aria-valuenow', '300');
      expect(pushToast).toHaveBeenCalledWith(
        expect.objectContaining({
          tone: 'warn',
          title: 'Gossip cadence rollback',
          body: 'An unexpected error occurred.',
        }),
      );
    });

    const callsBefore = invokeMock.mock.calls.filter((c) => c[0] === 'execute_command').length;
    fireEvent.click(screen.getByTestId('populi-run-probe-btn'));
    await waitFor(() => {
      const callsAfter = invokeMock.mock.calls.filter((c) => c[0] === 'execute_command').length;
      expect(callsAfter).toBeGreaterThanOrEqual(callsBefore + 2);
    });
  });
});
