import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import React from 'react';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({ display_name: 'operator@vox' }),
}));

vi.mock('../../generated/surfaceRegistry.generated', () => ({
  SURFACE_REGISTRY: [
    { viewKey: 'dashboard', navLabel: 'Dashboard', parentSurface: 'agents', tier: 'surface' },
    { viewKey: 'flow', navLabel: 'Flow', parentSurface: 'agents', tier: 'surface' },
    { viewKey: 'memory', navLabel: 'Memory', parentSurface: 'search', tier: 'surface' },
    { viewKey: 'settings', navLabel: 'Settings', parentSurface: null, tier: 'surface' },
    { viewKey: 'explorer', navLabel: 'Explorer', parentSurface: null, tier: 'surface' },
  ],
}));

import { Sidebar } from './Sidebar';
import { LanguageProvider } from '../../hooks/useLanguage';

const baseProps = {
  view: 'dashboard',
  onOpenParent: vi.fn(),
  onOpenTab: vi.fn(),
  agentsCount: 2,
  data: {
    agents: [],
    stream: [],
    alerts: [],
    skills: [],
    peers: [],
    kpis: {} as any,
    contextChips: [],
  },
  mode: 'default' as const,
  setMode: vi.fn(),
  pushToast: vi.fn(),
  appVersion: '0.6.0',
};

function renderSidebar(extraProps: Partial<typeof baseProps> = {}) {
  return render(
    <LanguageProvider>
      <Sidebar {...baseProps} {...extraProps} />
    </LanguageProvider>,
  );
}

describe('Sidebar badges', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  it('includes needs-you count in Runs nav aria-label', () => {
    renderSidebar({ needsYouCount: 3 });
    expect(screen.getByLabelText('Review, 3 items need you')).toBeDefined();
  });

  it('uses default Runs aria-label when nothing needs attention', () => {
    renderSidebar({ needsYouCount: 0 });
    expect(screen.getByRole('button', { name: 'Review' })).toBeDefined();
  });

  it('includes failing count in Settings nav aria-label', () => {
    renderSidebar({ policyBadge: { count: 2, status: 'fail' } });
    expect(screen.getByRole('button', { name: /Settings.*2 policy failures/i })).toBeDefined();
  });

  it('uses default Settings aria-label when nothing is failing', () => {
    renderSidebar({ policyBadge: { count: 0, status: 'pass' } });
    expect(screen.getByRole('button', { name: 'Settings' })).toBeDefined();
  });

  it('exposes Explorer shortcut with browse everything Vox can do aria-label', () => {
    renderSidebar();
    expect(screen.getByRole('button', { name: /Explorer.*browse everything Vox can do/i })).toBeDefined();
  });
});

describe('Sidebar has no nav filter', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  it('renders no nav filter input or toggle', () => {
    renderSidebar();
    expect(screen.queryByTestId('sidebar-nav-filter')).toBeNull();
    expect(screen.queryByRole('button', { name: /filter navigation/i })).toBeNull();
  });

  it('still renders top-level nav items', () => {
    renderSidebar();
    expect(screen.getByRole('button', { name: 'Chat' })).toBeDefined();
    expect(screen.getByRole('button', { name: /^Agents/i })).toBeDefined();
  });

  it('workspace nav calls onOpenParent with workspace key', () => {
    const onOpenParent = vi.fn();
    renderSidebar({ onOpenParent });
    fireEvent.click(screen.getByRole('button', { name: /^Workspace$/i }));
    expect(onOpenParent).toHaveBeenCalledWith('workspace');
  });
});

describe('Sidebar orchestrator freshness dot', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.spyOn(Date, 'now').mockRestore();
  });

  it('uses live styling when orchestrator events are fresh', () => {
    vi.spyOn(Date, 'now').mockReturnValue(10_000);
    renderSidebar({ lastOrchEventAt: 9_500, orchUsesPolling: false, liveFreshMs: 1_000 });
    const dot = screen.getByTestId('sidebar-orch-freshness-dot');
    expect(dot.className).toMatch(/bg-accent-secondary/);
    expect(dot.className).not.toMatch(/bg-text-muted/);
  });

  it('uses stale styling when orchestrator events are stale', () => {
    vi.spyOn(Date, 'now').mockReturnValue(10_000);
    renderSidebar({ lastOrchEventAt: 5_000, orchUsesPolling: false, liveFreshMs: 1_000 });
    const dot = screen.getByTestId('sidebar-orch-freshness-dot');
    expect(dot.className).toMatch(/bg-text-muted/);
    expect(dot.className).not.toMatch(/bg-accent-secondary/);
  });
});

describe('Sidebar language labels', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  it('shows English nav label by default', () => {
    renderSidebar();
    expect(screen.getByRole('button', { name: 'Market' })).toBeDefined();
  });

  it('shows Latin nav label when vox.lang=la', () => {
    window.localStorage.setItem('vox.lang', 'la');
    renderSidebar();
    expect(screen.getByRole('button', { name: 'Mercatus' })).toBeDefined();
  });
});

describe('Sidebar accordion (default and wide modes)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  it('expands the parent containing the active view in wide mode', () => {
    renderSidebar({ view: 'flow', mode: 'wide' });
    expect(screen.getByRole('button', { name: /^flow$/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^tasks$/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^policies$/i })).not.toBeInTheDocument();
  });

  it('expands the parent containing the active view in default mode as well', () => {
    renderSidebar({ view: 'flow', mode: 'default' });
    expect(screen.getByRole('button', { name: /^flow$/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^tasks$/i })).toBeInTheDocument();
  });

  it('names the runs child "Runs" under the "Review" parent, in the active language', () => {
    renderSidebar({ view: 'runs', mode: 'wide' });
    expect(screen.getByRole('button', { name: /^runs$/i })).toBeInTheDocument();
    cleanup();
    window.localStorage.setItem('vox.lang', 'la');
    renderSidebar({ view: 'runs', mode: 'wide' });
    expect(screen.getByRole('button', { name: /^cursus$/i })).toBeInTheDocument();
  });

  it('does not render a child tree in rail mode', () => {
    renderSidebar({ view: 'flow', mode: 'rail' });
    expect(screen.queryByRole('button', { name: /^tasks$/i })).not.toBeInTheDocument();
  });

  it('renders a portaled flyout menu for subsections in rail mode on hover or click and navigates', () => {
    const onOpenTab = vi.fn();
    renderSidebar({ view: 'flow', mode: 'rail', onOpenTab });
    // In rail mode, the inline tree is collapsed
    expect(screen.queryByRole('button', { name: /^tasks$/i })).not.toBeInTheDocument();
    // Clicking the parent reveals the portaled flyout menu
    const agentsBtn = screen.getByRole('button', { name: /Agents/ });
    fireEvent.click(agentsBtn);
    expect(screen.getByTestId('rail-flyout-agents')).toBeInTheDocument();
    const taskItem = screen.getByRole('menuitem', { name: /^tasks$/i });
    expect(taskItem).toBeInTheDocument();
    fireEvent.click(taskItem);
    expect(onOpenTab).toHaveBeenCalledWith('tasks');
  });

  it('clicking a child calls onOpenTab with that child key', () => {
    const onOpenTab = vi.fn();
    renderSidebar({ view: 'flow', mode: 'wide', onOpenTab });
    fireEvent.click(screen.getByRole('button', { name: /^tasks$/i }));
    expect(onOpenTab).toHaveBeenCalledWith('tasks');
  });

  it('the peek chevron expands a parent without navigating (does not call onOpenParent/onOpenTab)', () => {
    const onOpenParent = vi.fn();
    const onOpenTab = vi.fn();
    renderSidebar({ view: 'flow', mode: 'wide', onOpenParent, onOpenTab });
    fireEvent.click(screen.getByRole('button', { name: /expand knowledge/i }));
    expect(onOpenParent).not.toHaveBeenCalled();
    expect(onOpenTab).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: /^memory$/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^tasks$/i })).not.toBeInTheDocument();
  });

  it('clicking the active parent\'s own chevron collapses its default-expanded children', () => {
    renderSidebar({ view: 'flow', mode: 'wide' });
    expect(screen.getByRole('button', { name: /^tasks$/i })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /collapse agents/i }));
    expect(screen.queryByRole('button', { name: /^tasks$/i })).not.toBeInTheDocument();
  });
});

describe('Sidebar chat sessions section (Task 9)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  const chatSession = (overrides: Partial<{
    session_id: string; title: string; updated_at: string; message_count: number;
    conversation_id: number; repository_id: string | null;
  }> = {}) => ({
    session_id: 's1', title: 'First chat', updated_at: '2026-01-01T00:00:00Z',
    message_count: 1, conversation_id: 1, repository_id: 'repo-a', ...overrides,
  });

  it('renders SessionSidebarSection (not the generic static children list) when the chat parent is expanded in wide mode and chatSessions is provided', () => {
    renderSidebar({
      view: 'chat',
      mode: 'wide',
      chatSessions: [chatSession()],
      activeSessionId: 's1',
    });
    // SessionSidebarSection's own "+ New session" control is the reliable
    // marker that the real section rendered, not the generic per-parent
    // static children button list (which chat has none of).
    expect(screen.getByRole('button', { name: /new session/i })).toBeInTheDocument();
    expect(screen.getByText('First chat')).toBeInTheDocument();
  });

  it('does not render the session section when chatSessions is not provided, even with the chat parent expanded', () => {
    renderSidebar({ view: 'chat', mode: 'wide' });
    expect(screen.queryByRole('button', { name: /new session/i })).not.toBeInTheDocument();
  });

  it('clicking a session row calls onSessionChange with its session id', () => {
    const onSessionChange = vi.fn();
    renderSidebar({
      view: 'chat',
      mode: 'wide',
      chatSessions: [chatSession({ session_id: 's1', title: 'First chat' })],
      activeSessionId: null,
      onSessionChange,
    });
    fireEvent.click(screen.getByText('First chat'));
    expect(onSessionChange).toHaveBeenCalledWith('s1');
  });
});

describe('Sidebar degraded Review badge', () => {
  beforeEach(() => {
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  it('shows a degraded badge instead of all-clear when a source failed', () => {
    renderSidebar({ needsYouCount: 0, needsYouDegraded: ['approvals'] } as never);
    const review = screen.getByRole('button', { name: "Review, couldn't load approvals" });
    expect(review.textContent).toContain('!');
  });

  it('keeps the count and names the failed source when other sources have items', () => {
    renderSidebar({ needsYouCount: 2, needsYouDegraded: ['feedback'] } as never);
    expect(screen.getByRole('button', { name: "Review, 2 items need you (couldn't load feedback)" })).toBeDefined();
  });
});


import { StaticBackendStatusProvider } from '../../hooks/useBackendStatus';

describe('Sidebar remote gating', () => {
  it('marks unavailable child surfaces with their reason', () => {
    render(
      <LanguageProvider>
        <StaticBackendStatusProvider
          status={{ mode: 'remote', url: 'https://vox.lan:3921/', profile: null, surfaces: [{ surface: 'dashboard', available: true, reason: null }] }}
        >
          <Sidebar {...baseProps} mode="wide" view="dashboard" />
        </StaticBackendStatusProvider>
      </LanguageProvider>,
    );
    const flow = screen.getAllByRole('button').find((b) => b.getAttribute('data-unavailable') === 'not_in_profile');
    expect(flow).toBeTruthy();
    expect(flow?.getAttribute('title')).toMatch(/not available on this server/i);
  });
});

describe('Sidebar hide and restore button', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  it('hides the sidebar completely when Hide sidebar button is clicked and restores it via Show sidebar button', () => {
    const setMode = vi.fn();
    renderSidebar({ mode: 'default', setMode });

    const hideBtn = screen.getByTestId('sidebar-hide-button');
    expect(hideBtn).toHaveAttribute('aria-label', 'Hide sidebar');
    expect(hideBtn).toHaveAttribute('aria-expanded', 'true');

    fireEvent.click(hideBtn);
    expect(setMode).toHaveBeenCalledWith('hidden');

    const aside = screen.getByRole('complementary', { name: 'Sidebar' });
    expect(aside).toHaveAttribute('data-sidebar-hidden', 'true');
    expect(screen.queryByRole('navigation', { name: 'Primary navigation' })).toBeNull();

    const restoreBtn = screen.getByTestId('sidebar-restore-button');
    expect(restoreBtn).toHaveAttribute('aria-label', 'Show sidebar');
    expect(restoreBtn).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(restoreBtn);
    expect(setMode).toHaveBeenCalledWith('default');
    expect(screen.getByRole('navigation', { name: 'Primary navigation' })).toBeInTheDocument();
  });

  it('groups top-level navigation under the 5 v4.0 domain headers and collapses non-chat child list visually by default (NAV-CONSOLIDATE-01..02)', () => {
    renderSidebar({ mode: 'default', view: 'dashboard' });
    expect(screen.getByTestId('sidebar-domain-group-operate-chat')).toBeInTheDocument();
    expect(screen.getByTestId('sidebar-domain-group-agents-runs')).toBeInTheDocument();
    expect(screen.getByTestId('sidebar-domain-group-workspace')).toBeInTheDocument();
    expect(screen.getByTestId('sidebar-domain-group-knowledge')).toBeInTheDocument();
    expect(screen.getByTestId('sidebar-domain-group-compute-system')).toBeInTheDocument();

    const childList = screen.getByTestId('sidebar-child-list');
    expect(childList.className).toContain('sr-only');
  });
});
