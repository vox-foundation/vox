// @vitest-environment jsdom
import { render, screen } from '@testing-library/react';
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
});
