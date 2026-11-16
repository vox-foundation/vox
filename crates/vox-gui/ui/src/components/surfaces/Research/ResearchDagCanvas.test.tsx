import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';
import { describe, expect, it } from 'vitest';
import { ResearchDagCanvas } from './ResearchDagCanvas';

describe('ResearchDagCanvas', () => {
  it('renders dynamic svg canvas with wave auto-positioning and wcag accessibility', () => {
    const nodes = [
      { id: '1', label: 'Node A', wave: 0, status: 'verified' as const },
      { id: '2', label: 'Node B', wave: 1, status: 'contradicted' as const },
    ];
    const edges = [{ srcId: '1', dstId: '2', relation: 'contradicts' as const }];

    render(<ResearchDagCanvas nodes={nodes} edges={edges} />);
    expect(screen.getByRole('region', { name: /epistemic research dag/i })).toBeInTheDocument();
    expect(screen.getByText('Node A')).toBeInTheDocument();
    expect(screen.getByText('Node B')).toBeInTheDocument();
  });

  it('renders status indicators with accessible labels', () => {
    const nodes = [
      { id: '1', label: 'Hypothesis A', wave: 0, status: 'verified' as const },
      { id: '2', label: 'Hypothesis B', wave: 1, status: 'pending' as const },
      { id: '3', label: 'Hypothesis C', wave: 2, status: 'contradicted' as const },
    ];
    const edges = [
      { srcId: '1', dstId: '2', relation: 'supports' as const },
      { srcId: '2', dstId: '3', relation: 'contradicts' as const },
    ];

    render(<ResearchDagCanvas nodes={nodes} edges={edges} />);
    expect(screen.getByText('Hypothesis A')).toBeInTheDocument();
    expect(screen.getByText('Hypothesis B')).toBeInTheDocument();
    expect(screen.getByText('Hypothesis C')).toBeInTheDocument();
  });

  it('opens node inspector drawer when clicking a hierarchical node', () => {
    const nodes = [
      { id: 'q0', label: 'Root Query', wave: 0, kind: 'query' as const, status: 'verified' as const },
      {
        id: 'c1',
        label: 'Claim 1',
        wave: 2,
        kind: 'claim' as const,
        status: 'verified' as const,
        confidence: 0.94,
        excerpt: 'Corroborated across 3 independent domains.',
      },
    ];
    const edges = [{ srcId: 'q0', dstId: 'c1', relation: 'supports' as const }];

    render(<ResearchDagCanvas nodes={nodes} edges={edges} />);
    expect(screen.queryByTestId('dag-node-inspector')).toBeNull();
    fireEvent.click(screen.getByTestId('dag-node-c1'));
    expect(screen.getByTestId('dag-node-inspector')).toBeInTheDocument();
    expect(screen.getByText(/Corroborated across 3 independent domains/i)).toBeInTheDocument();
  });

  it('clusters >80 nodes by wave initially and expands a wave on click', () => {
    const nodes = Array.from({ length: 250 }, (_, idx) => {
      const wave = idx % 5;
      return {
        id: `n-${idx}`,
        label: `Claim Node ${idx}`,
        wave,
        kind: 'claim' as const,
        status: (idx % 7 === 0 ? 'contradicted' : idx % 2 === 0 ? 'verified' : 'pending') as
          | 'verified'
          | 'contradicted'
          | 'pending',
        confidence: 0.85,
        excerpt: `Evidence excerpt ${idx}`,
      };
    });
    const edges = nodes.slice(1).map((n, idx) => ({
      srcId: nodes[idx]!.id,
      dstId: n.id,
      relation: (n.status === 'contradicted' ? 'contradicts' : 'supports') as 'supports' | 'contradicts',
    }));

    const { container } = render(<ResearchDagCanvas nodes={nodes} edges={edges} />);

    expect(screen.getByTestId('dag-clustered-container')).toBeInTheDocument();
    for (let w = 0; w < 5; w++) {
      expect(screen.getByTestId(`dag-wave-cluster-${w}`)).toBeInTheDocument();
    }
    expect(container.querySelectorAll('[data-testid^="dag-node-"]')).toHaveLength(0);

    fireEvent.click(screen.getByTestId('dag-wave-toggle-2'));
    expect(screen.getByTestId('dag-wave-toggle-2')).toHaveAttribute('aria-expanded', 'true');

    const mountedNodes = container.querySelectorAll('[data-testid^="dag-node-n-"]');
    expect(mountedNodes.length).toBe(50);

    fireEvent.click(screen.getByTestId('dag-node-n-2'));
    expect(screen.getByTestId('dag-node-inspector')).toBeInTheDocument();
    expect(screen.getByText(/Evidence excerpt 2/i)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('dag-collapse-all-waves'));
    expect(container.querySelectorAll('[data-testid^="dag-node-n-"]')).toHaveLength(0);
  });
});
