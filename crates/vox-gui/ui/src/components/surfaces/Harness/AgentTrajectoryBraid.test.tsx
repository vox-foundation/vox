import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { AgentTrajectoryBraid } from './AgentTrajectoryBraid';

describe('AgentTrajectoryBraid', () => {
  beforeEach(() => cleanup());

  it('renders the 8-slice braided SVG paths and agent labels', () => {
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
            slices: [1, 1, 2, 1, 0, 1, 1, 1],
            activeLane: 'FAN_OUT',
          },
        ]}
      />,
    );

    expect(screen.getByText(/AGENT ROLE TRAJECTORY FLOW/i)).toBeDefined();
    expect(screen.getByText('verifier-branch-1')).toBeDefined();
    const paths = container.querySelectorAll('svg path');
    expect(paths.length).toBe(2);
  });
});
