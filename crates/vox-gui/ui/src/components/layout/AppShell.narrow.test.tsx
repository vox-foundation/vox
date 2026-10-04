// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import { AppShell } from './AppShell';
import { INITIAL_DATA, INITIAL_KPIS } from '../../data/initialState';
import type { DashboardData } from '../../types/dashboard';
import { defaultHudTiles } from '../../hooks/useHudTiles';

vi.mock('./Sidebar', () => ({
  Sidebar: ({ mode }: { mode: string }) => <nav data-testid="sidebar" data-mode={mode} aria-label="Primary" />,
  SidebarMode: {},
}));
vi.mock('./BreadcrumbBar', () => ({ BreadcrumbBar: () => null }));
vi.mock('./BottomStatusBar', () => ({ BottomStatusBar: () => <div role="status" aria-label="Operator status" /> }));
vi.mock('./SurfaceScrollHost', () => ({
  SurfaceScrollHost: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
vi.mock('../ui/Backdrop', () => ({ Backdrop: () => null }));
vi.mock('../ui/ErrorBoundary', () => ({ SurfaceErrorBoundary: ({ children }: { children: React.ReactNode }) => <>{children}</> }));

const props = {
  activeView: 'dashboard', onNavigate: vi.fn(), onOpenParent: vi.fn(), onOpenTab: vi.fn(),
  sidebarMode: 'wide' as const, setSidebarMode: vi.fn(), agentsCount: 0,
  data: INITIAL_DATA as DashboardData, pushToast: vi.fn(), appVersion: '0.6.0',
  policyBadge: { count: 0, status: 'not_run' as const }, needsYouCount: 0, pendingApprovals: 0,
  kpis: INITIAL_KPIS, onOpenCommandPalette: vi.fn(), lastOrchEventAt: null, orchUsesPolling: false,
  liveFreshMs: 30_000, surfaceKey: 'dashboard', surfaceLabel: 'Dashboard',
  hudTilesConfig: defaultHudTiles(), onHudTilesChange: vi.fn(), meshNodes: undefined, chatDocked: false,
};

function narrowMedia(narrow: boolean) {
  window.matchMedia = ((q: string) => ({
    matches: narrow && /max-width/.test(q),
    addEventListener: () => {}, removeEventListener: () => {},
  })) as unknown as typeof window.matchMedia;
}

describe('AppShell on a narrow viewport', () => {
  const original = window.matchMedia;
  afterEach(() => { window.matchMedia = original; });

  it('collapses the sidebar to its rail whatever mode is stored', () => {
    narrowMedia(true);
    render(<AppShell {...props}><div>surface</div></AppShell>);
    expect(screen.getByTestId('sidebar').getAttribute('data-mode')).toBe('rail');
  });

  it('keeps the stored sidebar mode on a wide viewport', () => {
    narrowMedia(false);
    render(<AppShell {...props}><div>surface</div></AppShell>);
    expect(screen.getByTestId('sidebar').getAttribute('data-mode')).toBe('wide');
  });
});
