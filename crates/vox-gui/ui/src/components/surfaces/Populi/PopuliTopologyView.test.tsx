import React from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetBackendAvailabilityForTests } from '../../../lib/backendGuard';
import type { PopuliPlacementDto } from '../../../types/concilium';

const { invokeMock, listenHandlers } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenHandlers: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, handler: (e: { payload: unknown }) => void) => {
    listenHandlers.set(event, handler);
    return Promise.resolve(() => {
      listenHandlers.delete(event);
    });
  }),
}));

import { PopuliTopologyView } from './PopuliTopologyView';

function makePlacement(overrides?: Partial<PopuliPlacementDto>): PopuliPlacementDto {
  return {
    generated_at_ms: 1700000000000,
    head_seq: 122,
    gossip_cadence_ms: 250,
    source: 'control_plane',
    queue_depth: 1,
    nodes: [
      {
        node_id: 'local-mens-gpu',
        display_name: 'Local Host MENS GPU',
        tier: 'local_mens_gpu',
        status: 'online',
        host_triple: 'aarch64-apple-darwin',
        gpu_summary: 'Apple M3 Max 24GB',
        vram_used_mb: 12288,
        vram_total_mb: 24576,
        kv_cache_warm_prefixes: 4,
        advertised_models: ['mens-8b'],
        docked_workers: [
          {
            agent_id: 'agent-coder-1',
            roster_name: 'p455-populi',
            status: 'working',
            tok_per_sec: 48.2,
            workspace_mode: 'share',
            file_lock_prefix: 'crates/vox-gui/ui/src/components/surfaces/Populi/',
          },
        ],
      },
      {
        node_id: 'quic-peer-beta',
        display_name: 'Populi Peer Beta',
        tier: 'populi_quic_peer',
        status: 'online',
        host_triple: 'x86_64-unknown-linux-gnu',
        gpu_summary: 'RTX 4090 24GB',
        vram_total_mb: 24576,
        rtt_ms: 7,
        advertised_models: ['qwen-coder-32b'],
        docked_workers: [
          {
            agent_id: 'agent-verifier-1',
            roster_name: 'verifier',
            status: 'parked',
            tok_per_sec: 0,
            workspace_mode: 'inherit',
            file_lock_prefix: null,
          },
        ],
      },
      {
        node_id: 'cloud-bandit-router',
        display_name: 'Cloud Frontier Router',
        tier: 'cloud_bandit',
        status: 'online',
        gpu_summary: 'Thompson Bandit Multi-Provider',
        bandit_weight: 0.72,
        advertised_models: ['claude-sonnet', 'gemini-pro'],
        docked_workers: [
          {
            agent_id: 'agent-arch-1',
            roster_name: 'coordinator',
            status: 'working',
            tok_per_sec: 84.5,
            workspace_mode: 'branch',
            file_lock_prefix: '.planning/',
          },
        ],
      },
    ],
    lock_matrix: [
      {
        prefix: 'crates/vox-gui/ui/src/components/surfaces/Populi/',
        holder_agent: 'p455-populi',
        exclusive: true,
        compute_tier: 'local_mens_gpu',
        workspace_mode: 'share',
        waiters: ['verifier'],
        contended: true,
        acquired_seq: 122,
      },
      {
        prefix: 'crates/vox-gui/ui/src/types/',
        holder_agent: 'coordinator',
        exclusive: false,
        compute_tier: 'cloud_bandit',
        workspace_mode: 'inherit',
        waiters: [],
        contended: false,
        acquired_seq: 118,
      },
    ],
    ...overrides,
  };
}

describe('PopuliTopologyView (CONC-07 / CONC-08)', () => {
  beforeEach(() => {
    cleanup();
    window.localStorage.clear();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    listenHandlers.clear();
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function setupDefaultMocks(placement: PopuliPlacementDto) {
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_gui_preference') return Promise.resolve(null);
      if (cmd === 'set_gui_preference') return Promise.resolve();
      if (cmd === 'concilium_placement') return Promise.resolve(placement);
      if (cmd === 'concilium_pilot_action') {
        return Promise.resolve({
          ok: true,
          seq: 125,
          message: 'Swapped compute tier',
          forked_agent_id: null,
        });
      }
      if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_mesh_nodes') {
        return Promise.resolve({
          is_error: false,
          result: {
            source: 'control_plane',
            queue_depth: 1,
            nodes: [
              { id: 'local-mens-gpu', status: 'online', vram_gb: 24, host_triple: 'aarch64-apple-darwin', gpu_summary: 'Apple M3 Max 24GB', trust_tier: 'verified', advertised_models: ['mens-8b'], last_seen_unix_ms: Date.now() },
              { id: 'quic-peer-beta', status: 'online', vram_gb: 24, host_triple: 'x86_64-unknown-linux-gnu', gpu_summary: 'RTX 4090 24GB', trust_tier: 'trusted', advertised_models: ['qwen-coder-32b'], last_seen_unix_ms: Date.now() },
            ],
          },
        });
      }
      if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_mesh_queue_stats') {
        return Promise.resolve({ is_error: false, result: { pending_count: 1 } });
      }
      if (cmd === 'execute_command') {
        return Promise.resolve({ exit_code: 0, stdout: 'overlay: healthy', stderr: '' });
      }
      return Promise.resolve(null);
    });
  }

  it('renders 3-tier HybridComputeNodeCards with docked @agent pills and FileAffinityMap contention shields', async () => {
    setupDefaultMocks(makePlacement());
    render(<PopuliTopologyView pushToast={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId('populi-compute-node-local-mens-gpu')).toBeInTheDocument();
      expect(screen.getByTestId('populi-compute-node-quic-peer-beta')).toBeInTheDocument();
      expect(screen.getByTestId('populi-compute-node-cloud-bandit-router')).toBeInTheDocument();
    });

    const localCard = screen.getByTestId('populi-compute-node-local-mens-gpu');
    expect(localCard).toHaveTextContent('LOCAL HOST MENS GPU');
    expect(localCard).toHaveTextContent('4 warm');
    expect(localCard).toHaveTextContent('@p455-populi');
    expect(localCard).toHaveTextContent('working 48.2 tok/s');
    expect(localCard).toHaveTextContent('share');

    const peerCard = screen.getByTestId('populi-compute-node-quic-peer-beta');
    expect(peerCard).toHaveTextContent('POPULI QUIC PEER (iroh)');
    expect(peerCard).toHaveTextContent('7ms RTT');
    expect(peerCard).toHaveTextContent('@verifier');
    expect(peerCard).toHaveTextContent('parked 0 tok/s');
    expect(peerCard).toHaveTextContent('inherit');

    const cloudCard = screen.getByTestId('populi-compute-node-cloud-bandit-router');
    expect(cloudCard).toHaveTextContent('CLOUD FRONTIER ROUTER');
    expect(cloudCard).toHaveTextContent('72% weight');
    expect(cloudCard).toHaveTextContent('@coordinator');
    expect(cloudCard).toHaveTextContent('branch');

    const lockMatrix = screen.getByTestId('populi-lock-matrix');
    expect(lockMatrix).toHaveTextContent('exclusive');
    expect(lockMatrix).toHaveTextContent('shared');
    expect(lockMatrix).toHaveTextContent('#seq-122');
    expect(screen.getByTestId('lock-contention-shield-crates/vox-gui/ui/src/components/surfaces/Populi/')).toHaveTextContent('[FileLock] wait: @verifier');
  });

  it('hot-swaps a docked worker to another compute node via concilium_pilot_action', async () => {
    const pushToast = vi.fn();
    setupDefaultMocks(makePlacement());
    render(<PopuliTopologyView pushToast={pushToast} />);

    await waitFor(() => {
      expect(screen.getByTestId('docked-worker-pill-agent-coder-1')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('docked-worker-pill-agent-coder-1'));
    fireEvent.click(screen.getByTestId('migrate-worker-btn-quic-peer-beta'));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('concilium_pilot_action', {
        input: {
          action: 'swap_agent_compute',
          agent_id: 'agent-coder-1',
          target_tier: 'populi_quic_peer',
          target_node_id: 'quic-peer-beta',
        },
      });
    });

    expect(pushToast).toHaveBeenCalledWith(
      expect.objectContaining({ tone: 'ok', title: 'Worker compute migrated' }),
    );
  });

  it('refreshes placement reactively on listenAgentEvents with zero 5000ms polling loops', async () => {
    const intervalSpy = vi.spyOn(window, 'setInterval');
    let currentPlacement = makePlacement({ head_seq: 122 });
    setupDefaultMocks(currentPlacement);
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'concilium_placement') return Promise.resolve(currentPlacement);
      if (cmd === 'get_gui_preference') return Promise.resolve(null);
      if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_mesh_nodes') {
        return Promise.resolve({ is_error: false, result: { source: 'control_plane', nodes: [] } });
      }
      if (cmd === 'invoke_mcp_tool' && args?.tool === 'vox_mesh_queue_stats') {
        return Promise.resolve({ is_error: false, result: { pending_count: 0 } });
      }
      if (cmd === 'execute_command') {
        return Promise.resolve({ exit_code: 0, stdout: 'ok', stderr: '' });
      }
      return Promise.resolve(null);
    });

    render(<PopuliTopologyView pushToast={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('head #seq-122')).toBeInTheDocument());

    const calls5s = intervalSpy.mock.calls.filter((c) => c[1] === 5000);
    const appIntervals = intervalSpy.mock.calls.filter(
      (c) => !String(c[0]).includes('checkRealTimersCallback'),
    );
    expect(calls5s).toHaveLength(0);
    expect(appIntervals).toHaveLength(0);

    currentPlacement = makePlacement({ head_seq: 130 });
    const handler = listenHandlers.get('vox://agent-events');
    expect(handler).toBeDefined();

    await act(async () => {
      handler?.({ payload: { id: 1, timestamp_ms: Date.now(), kind: { type: 'task_started' } } });
    });

    await waitFor(() => {
      expect(screen.getByText('head #seq-130')).toBeInTheDocument();
    });
  });
});
