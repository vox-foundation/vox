import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { AgentTrajectoryBraid } from './AgentTrajectoryBraid';

describe('AgentTrajectoryBraid (Concilium Compact Swimlane Embed)', () => {
  beforeEach(() => cleanup());

  it('renders the 8-slice braided SVG paths, W3C traceparent lanes, and 0 tok/s sandbox indicators', () => {
    const { container } = render(
      <AgentTrajectoryBraid
        trajectories={[
          {
            agentId: 'a-1',
            label: 'verifier-branch-1',
            slices: [0, 1, 1, 2, 2, 3, 2, 2],
            activeLane: 'VERIFY',
          },
          {
            agentId: 'a-2',
            label: 'fanout-worker-2',
            slices: [1, 1, 2, 1, 0, 1, 1, 3],
            activeLane: 'SANDBOX',
          },
        ]}
      />,
    );

    expect(screen.getByText(/AGENT ROLE TRAJECTORY FLOW/i)).toBeDefined();
    expect(screen.getByText('verifier-branch-1')).toBeDefined();
    expect(screen.getByText('0 tok/s')).toBeDefined();
    expect(container.querySelector('[data-slot="concilium-trajectory-braid"]')).not.toBeNull();
    const paths = container.querySelectorAll('svg path');
    expect(paths.length).toBe(2);
  });

  it('keeps 6 legend rows aligned with 6 SVG paths and clamps out-of-range slice indices', () => {
    const trajectories = Array.from({ length: 6 }, (_, i) => ({
      agentId: `a-${i}`,
      label: `worker-${i}`,
      slices: [0, 1, 2, 3, 2, 1, 0, i === 5 ? 99 : 2],
      activeLane: 'VERIFY' as const,
    }));
    const { container } = render(<AgentTrajectoryBraid trajectories={trajectories} />);
    expect(container.querySelectorAll('[data-slot="concilium-braid-lane"]').length).toBe(6);
    expect(container.querySelectorAll('svg path').length).toBe(6);
    expect(screen.queryByText('undefined')).toBeNull();
  });
});
