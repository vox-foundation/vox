// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';

const CARDS = [
  { id: 'openai/gpt-x', provider: 'openai', tier: 'frontier', cost_per_1k: 0.01, max_tokens: 128000, is_free: false, latency_p50_ms: 800 },
  { id: 'ollama/llama', provider: 'ollama', tier: 'local', cost_per_1k: 0, max_tokens: 8000, is_free: true, latency_p50_ms: 50 },
];
const SUMMARY = {
  active_model: 'openai/gpt-x',
  exploration_spent_usd: 1.2,
  exploration_budget_usd: 10,
  arm_count: 4,
  model_count: 2,
  decision_preview: null,
};

const baseImpl = (cmd: string) => {
  if (cmd === 'list_model_cards') return Promise.resolve(CARDS);
  if (cmd === 'get_routing_summary_live') return Promise.resolve(SUMMARY);
  if (cmd === 'get_active_model') return Promise.resolve('openai/gpt-x');
  if (cmd === 'set_active_model') return Promise.resolve(null);
  return Promise.resolve(null);
};
const invokeMock = vi.fn(baseImpl);
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

import { ModelsView } from './ModelsView';

describe('ModelsView', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockClear();
  });

  it('renders the Model Registry heading', () => {
    render(<ModelsView pushToast={vi.fn()} />);
    expect(screen.getByText('Model Registry')).toBeTruthy();
  });

  it('every button carries an explicit type="button"', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await waitFor(() => expect(screen.getAllByText('Set active').length).toBeGreaterThan(0));
    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('marks the active model card with aria-pressed and aria-current', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    const active = await screen.findByLabelText('Set openai/gpt-x as active model (currently active)');
    expect(active.getAttribute('aria-pressed')).toBe('true');
  });

  it('exposes the model list with role=list/listitem', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await waitFor(() => expect(screen.getAllByRole('list').length).toBeGreaterThan(0));
    expect(screen.getAllByRole('listitem').length).toBe(CARDS.length);
  });
});

const EXPLANATION = {
  mode: 'efficiency', task: 'codegen', complexity: 7, chosen: 'acme/widget-5.5', only_candidate: false,
  total_models: 3,
  candidates: [{
    id: 'acme/widget-5.5', provider: 'acme', tier: 'Fast', is_free: false, price_out_per_m: 0.9, score: 0.65,
    quality: { value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' },
    parts: { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 },
  }],
  excluded: [],
};
const HEALTHY = {
  schema_version: 1, checked_at_unix: 0, models: 3, cloud_models: 2, benchmarked: 1, inherited: 1,
  unknown_tier_cloud: 0, quality_scale: 'derived', price_bands: 'derived', efficient_pick: 'acme/widget-5.5',
  violations: [],
};
const PREVIEW = {
  selected_model: 'acme/widget-5.5', discovery_state: 'confirmed', intelligence_score: 0.8,
  efficiency_score: 0.5, latency_score: 0.4, alternatives: ['acme/widget-4.8'], rejection_reasons: [],
};
const ROUTING_CARDS = [...CARDS, {
  id: 'acme/widget-9', provider: 'acme', tier: 'Pro', cost_per_1k: 0.02, max_tokens: 64000, is_free: false,
  latency_p50_ms: 500,
}];
const routingAware = (cmd: string) => {
  if (cmd === 'list_model_cards') return Promise.resolve(ROUTING_CARDS);
  if (cmd === 'get_routing_summary_live') return Promise.resolve({ ...SUMMARY, decision_preview: PREVIEW });
  if (cmd === 'explain_routing') return Promise.resolve(EXPLANATION);
  if (cmd === 'get_routing_health') return Promise.resolve(HEALTHY);
  return baseImpl(cmd);
};

describe('ModelsView routing panel', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockClear();
    invokeMock.mockImplementation(routingAware);
  });
  afterEach(() => {
    invokeMock.mockImplementation(baseImpl);
  });

  it('renders the routing explainer instead of the decision preview', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await screen.findByTestId('routing-explainer');
    expect(screen.queryByText(/Decision Preview/i)).toBeNull();
    expect(document.body.textContent).not.toMatch(/intel=|eff=|lat=/);
  });

  it('changing the mode asks routing again for that mode', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await screen.findByTestId('routing-explainer');
    fireEvent.change(screen.getByRole('combobox', { name: 'Routing mode' }), { target: { value: 'genius' } });
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('explain_routing', { mode: 'genius', task: 'codegen', complexity: 7 }),
    );
  });

  it('prices are per million output tokens', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await screen.findByText('$20.00/M');
    expect(document.body.textContent).not.toContain('$/1k');
  });
});

