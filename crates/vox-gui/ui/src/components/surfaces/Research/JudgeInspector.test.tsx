// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { JudgeInspector, type JudgeClaim } from './JudgeInspector';

describe('JudgeInspector', () => {
  beforeEach(() => {
    cleanup();
  });

  it('renders metric badges: Confidence Tier, Source Count, Citation Precision', () => {
    render(
      <JudgeInspector
        confidenceTier="DeepResearch"
        sourceCount={7}
        citationPrecision={0.95}
      />
    );

    expect(screen.getByTestId('judge-inspector')).toBeTruthy();
    expect(screen.getByTestId('metric-confidence-tier').textContent).toBe('DeepResearch');
    expect(screen.getByTestId('metric-source-count').textContent).toBe('7');
    expect(screen.getByTestId('metric-citation-precision').textContent).toBe('95%');
  });

  it('renders claims verdict distribution accurately for supported, contested, and refuted claims', () => {
    const claims: JudgeClaim[] = [
      { claim_id: 'c1', text: 'Supported fact 1', verdict: 'Supported', confidence: 0.95 },
      { claim_id: 'c2', text: 'Verified fact 2', verdict: 'Verified', confidence: 0.92 },
      { claim_id: 'c3', text: 'Contested fact 3', verdict: 'Contested', confidence: 0.6 },
      { claim_id: 'c4', text: 'Refuted assertion 4', verdict: 'Refuted', confidence: 0.3 },
      { claim_id: 'c5', text: 'Contradicted claim 5', verdict: 'Contradicted', confidence: 0.2 },
    ];

    render(
      <JudgeInspector
        confidenceTier="Light"
        sourceCount={4}
        citationPrecision={0.88}
        claims={claims}
      />
    );

    expect(screen.getByTestId('count-supported').textContent).toBe('2');
    expect(screen.getByTestId('count-contested').textContent).toBe('1');
    expect(screen.getByTestId('count-refuted').textContent).toBe('2');

    expect(screen.getByText('Supported fact 1')).toBeTruthy();
    expect(screen.getByText('Contested fact 3')).toBeTruthy();
    expect(screen.getByText('Refuted assertion 4')).toBeTruthy();
  });

  it('renders chain-of-thought rationale and grounding details', () => {
    const claims: JudgeClaim[] = [
      { claim_id: 'c1', text: 'Claim A', verdict: 'Supported', confidence: 0.99, resample_stability: 0.9 },
    ];

    render(
      <JudgeInspector
        confidenceTier="DeepResearch"
        sourceCount={5}
        citationPrecision={1.0}
        claims={claims}
      />
    );

    const rationale = screen.getByTestId('judge-rationale');
    expect(rationale.textContent).toContain('Epistemic Judge evaluated 1 claim(s)');
    expect(rationale.textContent).toContain('5 independent source domain(s)');
    expect(rationale.textContent).toContain('100% citation precision');
  });

  it('displays custom rationale when provided in props', () => {
    render(
      <JudgeInspector
        confidenceTier="Direct"
        rationale="Custom judge chain-of-thought rationale verifying single-pass source."
      />
    );

    const rationale = screen.getByTestId('judge-rationale');
    expect(rationale.textContent).toBe(
      'Custom judge chain-of-thought rationale verifying single-pass source.'
    );
  });

  it('outputs appropriate rationale when claims array is empty', () => {
    render(
      <JudgeInspector
        confidenceTier="Direct"
        sourceCount={0}
        claims={[]}
      />
    );

    const rationale = screen.getByTestId('judge-rationale');
    expect(rationale.textContent).toContain('No discrete claims were extracted for epistemic evaluation.');
  });
});
