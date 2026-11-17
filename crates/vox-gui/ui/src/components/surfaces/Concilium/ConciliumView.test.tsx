import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { ConciliumView } from './ConciliumView';

const conciliumSnapshotMock = vi.fn();
const conciliumBoardMock = vi.fn();
const conciliumPilotActionMock = vi.fn();
const listenAgentEventsMock = vi.fn();

vi.mock('../../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../transport')>();
  return {
    ...actual,
    conciliumSnapshot: (...a: unknown[]) => conciliumSnapshotMock(...a),
    conciliumBoard: (...a: unknown[]) => conciliumBoardMock(...a),
    conciliumPilotAction: (...a: unknown[]) => conciliumPilotActionMock(...a),
    listenAgentEvents: (...a: unknown[]) => listenAgentEventsMock(...a),
  };
});

describe('ConciliumView (Mode 1: Vox Concilium)', () => {
  beforeEach(() => {
    conciliumSnapshotMock.mockReset();
    conciliumSnapshotMock.mockRejectedValue(new Error('offline fallback'));
    conciliumBoardMock.mockReset();
    conciliumBoardMock.mockRejectedValue(new Error('offline fallback'));
    conciliumPilotActionMock.mockReset();
    conciliumPilotActionMock.mockResolvedValue({
      ok: true,
      seq: 122,
      message: 'Pilot action applied',
    });
    listenAgentEventsMock.mockReset();
    listenAgentEventsMock.mockResolvedValue(() => {});
  });

  it('renders NowPlayingStrip, W3C CausalSwimlanePanel, SharedSequenceBoard, and LiveAgentLens', () => {
    const { container } = render(<ConciliumView />);
    expect(container.querySelector('[data-slot="concilium-now-playing"]')).not.toBeNull();
    expect(container.querySelector('[data-slot="concilium-swimlanes"]')).not.toBeNull();
    expect(container.querySelector('[data-slot="concilium-seq-board"]')).not.toBeNull();
    expect(container.querySelector('[data-slot="concilium-agent-lens"]')).not.toBeNull();

    expect(screen.getByText('CLOUD BANDIT')).toBeDefined();
    expect(screen.getByText('LOCAL MENS GPU')).toBeDefined();
    expect(screen.getByText('POPULI QUIC PEER')).toBeDefined();
    expect(screen.getByText(/0 tok\/s · Asleep on #seq-118/i)).toBeDefined();
    expect(screen.getByTestId('concilium-wakeup-arcs')).toBeDefined();
  });

  it('filters #seq sequence board by #channel and displays watchdog alerts and receipt badges', () => {
    render(<ConciliumView initialFocus="board" />);
    expect(screen.getByText('WATCHDOG ALERT')).toBeDefined();
    expect(screen.getByText('lease:verified')).toBeDefined();
    expect(screen.getByText('vitest:pass')).toBeDefined();

    fireEvent.click(screen.getByTestId('concilium-channel-verify'));
    expect(screen.getByText('#seq-120')).toBeDefined();
    expect(screen.queryByText('#seq-119')).toBeNull();

    fireEvent.click(screen.getByTestId('concilium-channel-all'));
    expect(screen.getByText('#seq-119')).toBeDefined();
  });

  it('invokes conciliumPilotAction for Park, Fork Warm Context, and inline @steer', async () => {
    render(<ConciliumView />);

    fireEvent.click(screen.getByTestId('pilot-park-agent'));
    await waitFor(() =>
      expect(conciliumPilotActionMock).toHaveBeenCalledWith(
        expect.objectContaining({ action: 'park_agent', agent_id: 'coordinator' }),
      ),
    );

    fireEvent.click(screen.getByTestId('pilot-fork-agent'));
    await waitFor(() =>
      expect(conciliumPilotActionMock).toHaveBeenCalledWith(
        expect.objectContaining({
          action: 'fork_agent',
          agent_id: 'coordinator',
          workspace_mode: 'share',
        }),
      ),
    );

    const steerInput = screen.getByTestId('concilium-board-steer-input');
    fireEvent.change(steerInput, { target: { value: 'Check W3C traceparent spans' } });
    fireEvent.submit(steerInput.closest('form')!);

    await waitFor(() =>
      expect(conciliumPilotActionMock).toHaveBeenCalledWith(
        expect.objectContaining({
          action: 'steer_agent',
          directive: 'Check W3C traceparent spans',
        }),
      ),
    );
  });

  it('streams live AgentEventFrames into the #seq board without polling loops', async () => {
    let pushedHandler: ((e: { id: number; timestamp_ms: number; kind: Record<string, unknown> }) => void) | undefined;
    listenAgentEventsMock.mockImplementation((cb) => {
      pushedHandler = cb;
      return Promise.resolve(() => {});
    });

    render(<ConciliumView />);
    await waitFor(() => expect(pushedHandler).toBeDefined());

    pushedHandler!({
      id: 99,
      timestamp_ms: 1731800099000,
      kind: {
        type: 'task_completed',
        agent_id: 'coder',
        channel: '#main',
        message: 'Completed causal swimlane slice',
        mentions: ['@verifier'],
      },
    });

    await waitFor(() =>
      expect(screen.getByText('Completed causal swimlane slice')).toBeDefined(),
    );
  });

  it('preserves earlier pilot mutations across repeated actions on legacyAgents props', async () => {
    render(
      <ConciliumView
        legacyAgents={[
          {
            id: 'a1',
            codename: 'Falcon',
            phase: 'Executing',
            task: 'compile',
            progress: 0.5,
            cost: 1,
            budget: 5,
            eta: '2m',
            skill: 'compiler',
          },
        ]}
      />,
    );

    fireEvent.click(screen.getByTestId('pilot-park-agent'));
    await waitFor(() => expect(screen.getByTestId('concilium-parked-workers')).toHaveTextContent('1'));

    fireEvent.click(screen.getByTestId('pilot-fork-agent'));
    await waitFor(() => expect(screen.getByTestId('swimlane-row-Falcon-fork')).toBeDefined());
    expect(screen.getByTestId('concilium-parked-workers')).toHaveTextContent('2');
  });
});
