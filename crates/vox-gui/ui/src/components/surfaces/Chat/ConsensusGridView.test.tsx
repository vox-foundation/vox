import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ConsensusGridView } from './ConsensusGridView';

describe('ConsensusGridView', () => {
  const sampleLegs = [
    {
      modelId: 'anthropic/claude-sonnet',
      status: 'completed' as const,
      content: 'Response from Claude Sonnet',
      responseTimeMs: 410,
      qualityScore: 91.4,
    },
    {
      modelId: 'mens-qwen-coder-7b',
      status: 'completed' as const,
      content: 'Response from local MENS model',
      responseTimeMs: 185,
      qualityScore: 88.2,
    },
  ];

  it('renders multi-model legs and toggles between Grid View and Stacked View', () => {
    render(<ConsensusGridView legs={sampleLegs} />);
    const container = screen.getByTestId('consensus-legs-container');
    expect(container.getAttribute('data-layout')).toBe('grid');
    expect(container.getAttribute('data-columns')).toBe('2');
    expect(screen.getByTestId('consensus-quality-anthropic/claude-sonnet').textContent).toContain(
      'QA 91.4',
    );

    fireEvent.click(screen.getByTestId('consensus-layout-stacked'));
    expect(container.getAttribute('data-layout')).toBe('stacked');
    expect(container.getAttribute('data-columns')).toBe('1');
  });
});
