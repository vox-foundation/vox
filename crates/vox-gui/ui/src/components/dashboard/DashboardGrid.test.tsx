import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';
import { addWidgetToLayout, defaultDashboardLayout } from '../../lib/dashboardLayout';
import { __resetBackendAvailabilityForTests } from '../../lib/backendGuard';
import { SHELL_PREFERENCE_KEYS } from '../../lib/shellPersistence';
import { voxTransport } from '../../transport';
import {
  DashboardGrid,
  hydrateDashboardLayoutFromBackend,
  persistDashboardLayout,
} from './DashboardGrid';

describe('DashboardGrid', () => {
  beforeEach(() => {
    window.localStorage.clear();
    delete (globalThis as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetBackendAvailabilityForTests();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    delete (globalThis as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetBackendAvailabilityForTests();
  });
  it('renders widgets from layout in DOM order', () => {
    const layout = defaultDashboardLayout();
    render(
      <DashboardGrid
        layout={layout}
        customizeMode={false}
        onLayoutChange={vi.fn()}
        renderWidget={(widget) => (
          <div data-testid={`widget-${widget.id}`}>{widget.kind}</div>
        )}
      />,
    );

    const nodes = screen.getAllByTestId(/^widget-/);
    expect(nodes.map((n) => n.getAttribute('data-testid'))).toEqual([
      'widget-resources',
      'widget-agents',
      'widget-alerts',
      'widget-stream',
    ]);
  });

  it('shows drag handles when customizeMode is true', () => {
    const layout = defaultDashboardLayout();
    render(
      <DashboardGrid
        layout={layout}
        customizeMode
        onLayoutChange={vi.fn()}
        renderWidget={(widget) => <div>{widget.kind}</div>}
      />,
    );

    expect(screen.getAllByRole('button', { name: /drag to reorder/i }).length).toBe(
      layout.widgets.length,
    );
  });

  it('shows resize handles when customizeMode is true', () => {
    const layout = defaultDashboardLayout();
    render(
      <DashboardGrid
        layout={layout}
        customizeMode
        onLayoutChange={vi.fn()}
        renderWidget={(widget) => <div>{widget.kind}</div>}
      />,
    );

    expect(screen.getAllByRole('button', { name: /^resize widget$/i }).length).toBe(
      layout.widgets.length,
    );
  });

  it('does not show resize handles when customizeMode is false', () => {
    const layout = defaultDashboardLayout();
    render(
      <DashboardGrid
        layout={layout}
        customizeMode={false}
        onLayoutChange={vi.fn()}
        renderWidget={(widget) => <div>{widget.kind}</div>}
      />,
    );

    expect(screen.queryByRole('button', { name: /^resize widget$/i })).toBeNull();
  });

  it('persists dashboard layout to both localStorage and backend setGuiPreference when backendAvailable()', async () => {
    (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    const setPrefSpy = vi
      .spyOn(voxTransport, 'setGuiPreference')
      .mockResolvedValue(undefined);

    const custom = addWidgetToLayout(defaultDashboardLayout(), 'budget_burn');
    persistDashboardLayout(custom);

    expect(
      window.localStorage.getItem(SHELL_PREFERENCE_KEYS.dashboardLayout),
    ).toBe(JSON.stringify(custom));
    expect(setPrefSpy).toHaveBeenCalledWith(
      SHELL_PREFERENCE_KEYS.dashboardLayout,
      JSON.stringify(custom),
    );
  });

  it('hydrates and validates dashboard layout from backend getGuiPreference and syncs localStorage', async () => {
    (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    __resetBackendAvailabilityForTests();
    const expected = addWidgetToLayout(defaultDashboardLayout(), 'queue_depth');
    const getPrefSpy = vi
      .spyOn(voxTransport, 'getGuiPreference')
      .mockResolvedValueOnce(JSON.stringify(expected))
      .mockResolvedValueOnce('{"version":999}');

    const hydrated = await hydrateDashboardLayoutFromBackend(
      defaultDashboardLayout(),
    );
    expect(getPrefSpy).toHaveBeenCalledWith(
      SHELL_PREFERENCE_KEYS.dashboardLayout,
    );
    expect(hydrated).toEqual(expected);
    expect(
      window.localStorage.getItem(SHELL_PREFERENCE_KEYS.dashboardLayout),
    ).toBe(JSON.stringify(expected));

    const invalid = await hydrateDashboardLayoutFromBackend(
      defaultDashboardLayout(),
    );
    expect(invalid).toBeNull();
  });
});
