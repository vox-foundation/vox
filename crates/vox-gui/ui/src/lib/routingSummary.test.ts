import { describe, it, expect } from 'vitest';
import { modelStateHint, railRoutingFromSummary, routingCardValue } from './routingSummary';

const summary = (over: Record<string, unknown> = {}, preview: Record<string, unknown> = {}) =>
  ({
    active_model: null,
    exploration_spent_usd: 0,
    exploration_budget_usd: 1,
    routing_priority: { efficiency: 50, precision: 50, latency: 50, availability: 50, balance: 50, mobile: 50 },
    arm_count: 1,
    model_count: 1,
    decision_preview: {
      selected_model: 'acme/widget-flash-4.8',
      discovery_state: 'confirmed',
      alternatives: [],
      rejection_reasons: [],
      intelligence_score: 0.5,
      efficiency_score: 0.5,
      latency_score: 0.5,
      ...preview,
    },
    family: 'acme/widget-flash',
    reason: 'lowest cost that fits the mode',
    ...over,
  }) as never;

describe('routingCardValue (global pick, via routingModelLabel)', () => {
  it('catalog pick shows the concrete id', () => {
    expect(routingCardValue(summary({ resolved_from: 'catalog' }))).toBe('Auto → acme/widget-flash-4.8');
  });

  it('bootstrap pick shows the family plus "(offline)", never the version', () => {
    const value = routingCardValue(summary({ resolved_from: 'bootstrap' }));
    expect(value).toBe('Auto → acme/widget-flash (offline)');
    expect(value).not.toContain('4.8');
  });

  it('a daemon without family/resolved_from is treated as offline and never shows a version', () => {
    expect(routingCardValue(summary({ family: undefined, resolved_from: undefined }))).toBe(
      'Auto → acme/widget-flash (offline)',
    );
  });

  it('a locally served pick shows its local id with "(local)"', () => {
    expect(
      routingCardValue(summary({ resolved_from: 'local', family: 'mens/finetune' }, { selected_model: 'mens/finetune-run' })),
    ).toBe('Auto → mens/finetune-run (local)');
  });

  it('reads plain "Auto" with no summary, no decision, or a blank pick — even when a family is present', () => {
    expect(routingCardValue(null)).toBe('Auto');
    expect(routingCardValue(summary({ decision_preview: null, resolved_from: 'bootstrap' }))).toBe('Auto');
    expect(routingCardValue(summary({ resolved_from: 'bootstrap' }, { selected_model: '  ' }))).toBe('Auto');
  });
});

describe('railRoutingFromSummary', () => {
  it('returns null without a summary or a decision, even when a family is present', () => {
    expect(railRoutingFromSummary(null)).toBeNull();
    expect(railRoutingFromSummary(summary({ decision_preview: null, resolved_from: 'bootstrap' }))).toBeNull();
  });

  it('off-catalog: alternatives become family keys, deduped, chosen family excluded, at most 3', () => {
    const r = railRoutingFromSummary(
      summary(
        { resolved_from: 'bootstrap' },
        {
          alternatives: [
            'acme/widget-flash-4.7',
            'acme/gizmo-pro-2',
            'acme/gizmo-pro-3',
            'acme/zeta-1',
            'acme/omega-9',
          ],
        },
      ),
    );
    expect(r).toEqual({
      model: 'acme/widget-flash (offline)',
      reason: 'lowest cost that fits the mode',
      state: 'confirmed',
      alternatives: ['acme/gizmo-pro', 'acme/zeta', 'acme/omega'],
    });
  });

  it('catalog: alternatives stay concrete ids', () => {
    const r = railRoutingFromSummary(
      summary({ resolved_from: 'catalog' }, { alternatives: ['acme/gizmo-pro-2', 'acme/zeta-1'] }),
    );
    expect(r?.model).toBe('acme/widget-flash-4.8');
    expect(r?.alternatives).toEqual(['acme/gizmo-pro-2', 'acme/zeta-1']);
  });

  it('a blank reason or state becomes null', () => {
    const r = railRoutingFromSummary(summary({ resolved_from: 'bootstrap', reason: '  ' }, { discovery_state: '' }));
    expect(r?.reason).toBeNull();
    expect(r?.state).toBeNull();
  });
});

describe('modelStateHint', () => {
  it('explains the discovery states the server sends (ModelConfidence)', () => {
    expect(modelStateHint('confirmed')).toMatch(/eligible for routing/);
    expect(modelStateHint('Provisional')).toMatch(/not yet measured/);
    expect(modelStateHint('shadowed')).toMatch(/not routed yet/);
    expect(modelStateHint('deprecated')).toMatch(/retired/);
  });

  it('returns null for anything else, including words the server never sends and inherited keys', () => {
    expect(modelStateHint('exploit')).toBeNull();
    expect(modelStateHint('constructor')).toBeNull();
    expect(modelStateHint(null)).toBeNull();
  });
});
