import { describe, it, expect } from 'vitest';
import { RESEARCH_STAGE_EXPLAINERS } from './researchExplainerRegistry';

describe('researchExplainerRegistry', () => {
  it('defines all canonical research stages with plain English and actions', () => {
    const requiredStages = [
      'queued',
      'planning',
      'retrieving',
      'verifying_claims',
      'synthesizing',
      'auditing_citations',
      'persisting',
      'completed',
    ];
    for (const s of requiredStages) {
      const explainer = RESEARCH_STAGE_EXPLAINERS[s];
      expect(explainer).toBeDefined();
      expect(explainer.plainTitle.length).toBeGreaterThan(0);
      expect(explainer.whyItMatters.length).toBeGreaterThan(0);
      expect(explainer.plainSummary.length).toBeGreaterThan(0);
      expect(explainer.mathOrAlgorithm.length).toBeGreaterThan(0);
      expect(explainer.action.length).toBeGreaterThan(0);
    }
  });
});
