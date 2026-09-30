// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { within } from '@testing-library/react';
import type { ComponentProps } from 'react';
import { BottomStatusBar } from './BottomStatusBar';
import { defaultHudTiles } from '../../hooks/useHudTiles';
import { INITIAL_KPIS } from '../../data/initialState';
import { WORKBENCH_TABBAR_TRAILING_SLOT_ID } from '../../lib/domIds';

describe('BottomStatusBar', () => {
  it('renders every enabled tile as a compact one-line segment', () => {
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onNavigate={vi.fn()}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    expect(screen.getByTestId('bottom-status-bar')).toBeInTheDocument();
    expect(screen.getByText('Engine')).toBeInTheDocument();
    expect(screen.getByText('Mesh')).toBeInTheDocument();
  });

  it('a disabled tile in hudTilesConfig does not render', () => {
    const config = defaultHudTiles();
    config.tiles = config.tiles.map((t) =>
      t.kind === 'mesh_peers' ? { ...t, enabled: false } : t,
    );
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={config}
        onNavigate={vi.fn()}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    expect(screen.queryByText('Mesh')).not.toBeInTheDocument();
  });

  it('clicking the agents segment navigates to the agents view', () => {
    const onNavigate = vi.fn();
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onNavigate={onNavigate}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    fireEvent.click(screen.getByText('Engine').closest('button')!);
    expect(onNavigate).toHaveBeenCalledWith('agents');
  });

  it('clicking the mesh segment navigates to the real mesh view, not the Compute default', () => {
    const onNavigate = vi.fn();
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onNavigate={onNavigate}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    fireEvent.click(screen.getByText('Mesh').closest('button')!);
    expect(onNavigate).toHaveBeenCalledWith('mesh');
  });

  it('the configure trigger opens a live-apply checkbox menu that stays open across toggles', () => {
    const onHudTilesChange = vi.fn();
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onHudTilesChange={onHudTilesChange}
        onNavigate={vi.fn()}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /configure/i }));
    const meshCheckbox = screen.getByRole('checkbox', { name: /^mesh$/i });
    expect(meshCheckbox).toBeChecked();
    fireEvent.click(meshCheckbox);
    expect(onHudTilesChange).toHaveBeenCalledTimes(1);
    const budgetCheckbox = screen.getByRole('checkbox', { name: /^spend$/i });
    fireEvent.click(budgetCheckbox);
    expect(onHudTilesChange).toHaveBeenCalledTimes(2);
  });

  it('renders the workbench-tabbar-trailing-slot portal target', () => {
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onHudTilesChange={vi.fn()}
        onNavigate={vi.fn()}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    expect(document.getElementById(WORKBENCH_TABBAR_TRAILING_SLOT_ID)).toBeInTheDocument();
  });

  it('mesh segment shows online/total node count from real mesh data, not a bare peer count', () => {
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onHudTilesChange={vi.fn()}
        onNavigate={vi.fn()}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
        meshNodes={[
          { id: 'n1', status: 'online' },
          { id: 'n2', status: 'online' },
          { id: 'n3', status: 'quarantined' },
        ]}
      />,
    );
    expect(screen.getByTestId('bottom-status-bar-mesh')).toHaveTextContent('2/3 online');
  });

  it('mesh card shows "—" until the mesh node list arrives (one source: useMeshNodes)', () => {
    render(
      <BottomStatusBar
        kpis={INITIAL_KPIS}
        hudTilesConfig={defaultHudTiles()}
        onHudTilesChange={vi.fn()}
        onNavigate={vi.fn()}
        lastOrchEventAt={null}
        orchUsesPolling={false}
        liveFreshMs={10_000}
      />,
    );
    expect(screen.getByTestId('bottom-status-bar-mesh').textContent).toBe('Mesh—');
  });
});

const cardKpis = (budget: { value: number; cap: number; source: 'daemon' | 'fallback' }) => ({
  ...INITIAL_KPIS,
  activeAgents: { ...INITIAL_KPIS.activeAgents, value: 9 },
  queueDepth: { ...INITIAL_KPIS.queueDepth, value: 44 },
  budgetBurn: { ...INITIAL_KPIS.budgetBurn, ...budget },
});

const summary = (over: Record<string, unknown> = {}, selected = 'acme/widget-flash-4.8') => ({
  active_model: null,
  exploration_spent_usd: 0,
  exploration_budget_usd: 1,
  routing_priority: { efficiency: 50, precision: 50, latency: 50, availability: 50, balance: 50, mobile: 50 },
  arm_count: 1,
  model_count: 1,
  decision_preview: {
    selected_model: selected,
    discovery_state: 'confirmed',
    alternatives: [],
    rejection_reasons: [],
    intelligence_score: 0.5,
    efficiency_score: 0.5,
    latency_score: 0.5,
  },
  family: 'acme/widget-flash',
  reason: 'lowest cost that fits the mode',
  ...over,
});

function renderBar(over: Partial<ComponentProps<typeof BottomStatusBar>> = {}) {
  return render(
    <BottomStatusBar
      kpis={cardKpis({ value: 12.34, cap: 50, source: 'daemon' })}
      hudTilesConfig={defaultHudTiles()}
      onHudTilesChange={vi.fn()}
      onNavigate={vi.fn()}
      lastOrchEventAt={null}
      orchUsesPolling={false}
      liveFreshMs={10_000}
      {...over}
    />,
  );
}

describe('BottomStatusBar cards (chat-surfaces plan 3a)', () => {
  it('Engine reads "9 agents · 44 queued" and opens Agents', () => {
    const onNavigate = vi.fn();
    renderBar({ onNavigate });
    const card = screen.getByTestId('bottom-status-bar-engine');
    expect(card).toHaveTextContent('Engine9 agents · 44 queued');
    fireEvent.click(card);
    expect(onNavigate).toHaveBeenCalledWith('agents');
  });

  it('Engine says "1 agent", not "1 agents"', () => {
    renderBar({
      kpis: { ...cardKpis({ value: 0, cap: 50, source: 'daemon' }), activeAgents: { ...INITIAL_KPIS.activeAgents, value: 1 } },
    });
    expect(screen.getByTestId('bottom-status-bar-engine')).toHaveTextContent('1 agent · 44 queued');
  });

  it('Spend shows a daemon cap as "$12.34 / $50.00"', () => {
    renderBar();
    expect(screen.getByTestId('bottom-status-bar-spend')).toHaveTextContent('Spend$12.34 / $50.00');
  });

  it('Spend never renders a zero cap or the fallback placeholder cap', () => {
    const { unmount } = renderBar({ kpis: cardKpis({ value: 12.34, cap: 0, source: 'daemon' }) });
    expect(screen.getByTestId('bottom-status-bar-spend').textContent).toBe('Spend$12.34');
    unmount();
    renderBar({ kpis: cardKpis({ value: 12.34, cap: 50, source: 'fallback' }) });
    expect(screen.getByTestId('bottom-status-bar-spend').textContent).toBe('Spend$12.34');
  });

  it('the Spend popover splits engine total, OpenRouter, this session and local models', () => {
    renderBar({ openrouterSpendUsd: 1.5, sessionSpentUsd: 0.25 });
    fireEvent.click(screen.getByTestId('bottom-status-bar-spend'));
    const dialog = screen.getByRole('dialog', { name: 'Spend detail' });
    const row = (label: string) => within(dialog).getByText(label).nextElementSibling?.textContent;
    expect(row('Engine total')).toBe('$12.34 / $50.00');
    expect(row('OpenRouter')).toBe('$1.50');
    expect(row('This session')).toBe('$0.25');
    expect(row('Local models')).toBe('not metered');
    expect(screen.getByTestId('bottom-status-bar-spend')).toHaveAttribute('aria-expanded', 'true');
  });

  it('an unknown OpenRouter or session figure reads "unknown", never $0.00', () => {
    renderBar({ openrouterSpendUsd: null, sessionSpentUsd: null });
    fireEvent.click(screen.getByTestId('bottom-status-bar-spend'));
    const dialog = screen.getByRole('dialog', { name: 'Spend detail' });
    expect(within(dialog).getByText('OpenRouter').nextElementSibling?.textContent).toBe('unknown');
    expect(within(dialog).getByText('This session').nextElementSibling?.textContent).toBe('unknown');
  });

  it('Escape closes the Spend popover and returns focus to the Spend card', () => {
    renderBar();
    const card = screen.getByTestId('bottom-status-bar-spend');
    fireEvent.click(card);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Spend detail' })).toBeNull();
    expect(document.activeElement).toBe(card);
  });

  it('Routing shows the family and "(offline)" when the pick was not read from the catalog', () => {
    renderBar({ routingSummary: summary({ resolved_from: 'bootstrap' }) });
    const card = screen.getByTestId('bottom-status-bar-routing');
    expect(card).toHaveTextContent('RoutingAuto → acme/widget-flash (offline)');
    expect(card.textContent).not.toContain('4.8');
  });

  it('Routing shows a concrete version only when resolved_from is "catalog"', () => {
    renderBar({ routingSummary: summary({ resolved_from: 'catalog' }) });
    expect(screen.getByTestId('bottom-status-bar-routing')).toHaveTextContent('Auto → acme/widget-flash-4.8');
  });

  it('a long routing value is truncated in the bar and kept whole in its title', () => {
    const longId = `acme/${'widget-'.repeat(12)}flash-20260901`;
    renderBar({ routingSummary: summary({ resolved_from: 'catalog' }, longId) });
    const value = screen.getByTestId('bottom-status-bar-routing').querySelector('[data-card-value]')!;
    expect(value.className).toContain('truncate');
    expect(value).toHaveAttribute('title', `Auto → ${longId}`);
  });

  it('Routing reads "Auto" before any summary arrives', () => {
    renderBar();
    expect(screen.getByTestId('bottom-status-bar-routing').textContent).toBe('RoutingAuto');
  });

  it('Needs you counts approvals plus questions and opens Needs you', () => {
    const onNavigate = vi.fn();
    renderBar({ needsYouCount: 3, onNavigate });
    const card = screen.getByTestId('bottom-status-bar-needs-you');
    expect(card).toHaveTextContent('Needs you3');
    fireEvent.click(card);
    expect(onNavigate).toHaveBeenCalledWith('needs-you');
  });

  it('no retired segment renders', () => {
    renderBar({ openrouterSpendUsd: 1.5 });
    for (const id of ['agents', 'queue', 'budget', 'model', 'openrouter', 'approvals']) {
      expect(screen.queryByTestId(`bottom-status-bar-${id}`)).toBeNull();
    }
  });

  it('every Configure checkbox label equals the label on the card it toggles', () => {
    renderBar();
    const cardLabels = Array.from(document.querySelectorAll('[data-card-label]')).map((el) => el.textContent);
    fireEvent.click(screen.getByRole('button', { name: /configure status bar/i }));
    const menuLabels = screen.getAllByRole('checkbox').map((cb) => cb.closest('label')?.textContent?.trim());
    expect(cardLabels).toEqual(['Engine', 'Spend', 'Mesh', 'Routing', 'Needs you']);
    expect(menuLabels).toEqual(cardLabels);
  });

  it('the freshness pill explains Polling in its tooltip', () => {
    renderBar({ orchUsesPolling: true });
    expect(screen.getByTestId('bottom-status-bar-freshness')).toHaveAttribute(
      'title',
      expect.stringMatching(/^Polling/),
    );
  });
});
