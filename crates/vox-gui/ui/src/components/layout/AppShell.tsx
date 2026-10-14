import React from 'react';
import { Backdrop } from '../ui/Backdrop';
import { Sidebar, type SidebarMode } from './Sidebar';
import { SurfaceScrollHost } from './SurfaceScrollHost';
import { BreadcrumbBar } from './BreadcrumbBar';
import { BottomStatusBar } from './BottomStatusBar';
import { SurfaceErrorBoundary } from '../ui/ErrorBoundary';
import type { DashboardData } from '../../types/dashboard';
import type { PolicyBadge } from './Sidebar';
import type { HudTilesConfig } from '../../hooks/useHudTiles';
import { useNarrowViewport } from '../../hooks/useNarrowViewport';
import type { RoutingHealth, RoutingSummary, Toast } from '../../types/tauri';
import type { MeshNode } from '../surfaces/Mesh/MeshView';
import { INITIAL_KPIS } from '../../data/initialState';
import type { ChatSession } from '../../lib/useChatSessions';
import type { Notice } from '../../lib/noticeStore';

type KpiState = typeof INITIAL_KPIS;

export interface AppShellProps {
  activeView: string;
  onNavigate: (view: string) => void;
  onOpenParent: (parentKey: string) => void;
  onOpenTab: (viewKey: string) => void;
  sidebarMode: SidebarMode;
  setSidebarMode: (mode: SidebarMode) => void;
  agentsCount: number;
  data: DashboardData;
  pushToast: (t: Toast) => void;
  appVersion: string;
  policyBadge: PolicyBadge;
  needsYouCount: number;
  needsYouDegraded?: string[];
  kpis: KpiState;
  onOpenCommandPalette: () => void;
  lastOrchEventAt: number | null;
  orchUsesPolling: boolean;
  liveFreshMs: number;
  surfaceKey: string;
  surfaceLabel: string;
  /** When false, Loquela / transcript stack is omitted (Chat surface hosts composer). */
  chatDocked: boolean;
  chatDock?: React.ReactNode;
  children: React.ReactNode;
  openrouterSpendUsd?: number | null;
  /** Global routing pick for the status bar's Routing card. */
  routingSummary?: RoutingSummary | null;
  /** Routing health for the Routing card's dot. */
  routingHealth?: RoutingHealth | null;
  /** This chat session's spend, for the Spend popover. */
  sessionSpentUsd?: number | null;
  gamifyEnabled?: boolean;
  onOpenAchievements?: () => void;
  onOpenResearchDrawer?: () => void;
  /** Notice store contents and its "mark all read", for the status bar's bell. */
  notices?: Notice[];
  onMarkAllNoticesRead?: () => void;
  hudTilesConfig: HudTilesConfig;
  onHudTilesChange: (config: HudTilesConfig) => void;
  meshNodes: MeshNode[] | undefined;
  chatSessions?: ChatSession[];
  activeSessionId?: string | null;
  chatTaskCounts?: Record<string, number>;
  archivedChatSessions?: ChatSession[];
  showArchivedChatSessions?: boolean;
  /** session_ids with at least one pending scientia_harness_issues row (App.tsx polls). */
  pendingHarnessIssueSessionIds?: Set<string>;
  onSessionChange?: (sessionId: string) => void;
  onCreateSession?: () => void;
  onRenameSession?: (sessionId: string, title: string) => void;
  onArchiveSession?: (sessionId: string) => void;
  onUnarchiveSession?: (sessionId: string) => void;
  onToggleArchivedSessions?: () => void;
  onTaskBadgeClick?: (sessionId: string) => void;
}

export function AppShell({
  activeView,
  onNavigate,
  onOpenParent,
  onOpenTab,
  sidebarMode,
  setSidebarMode,
  agentsCount,
  data,
  pushToast,
  appVersion,
  policyBadge,
  needsYouCount,
  needsYouDegraded,
  kpis,
  onOpenCommandPalette,
  lastOrchEventAt,
  orchUsesPolling,
  liveFreshMs,
  surfaceKey,
  surfaceLabel,
  chatDocked,
  chatDock,
  children,
  openrouterSpendUsd,
  routingSummary,
  routingHealth,
  sessionSpentUsd,
  gamifyEnabled,
  onOpenAchievements,
  onOpenResearchDrawer,
  notices,
  onMarkAllNoticesRead,
  hudTilesConfig,
  onHudTilesChange,
  meshNodes,
  chatSessions,
  activeSessionId,
  chatTaskCounts,
  archivedChatSessions,
  showArchivedChatSessions,
  pendingHarnessIssueSessionIds,
  onSessionChange,
  onCreateSession,
  onRenameSession,
  onArchiveSession,
  onUnarchiveSession,
  onToggleArchivedSessions,
  onTaskBadgeClick,
}: AppShellProps) {
  // Below 640px the 212px/280px sidebar leaves the status bar no room: show the rail, keep the stored mode.
  const narrow = useNarrowViewport(640);
  const mainPaddingBottom = chatDocked ? 'pb-[180px]' : 'pb-5';

  return (
    <div className="flex flex-1 min-h-0 w-screen flex-col bg-bg-base text-text-muted font-sans selection:bg-brass/30 selection:text-text-primary overflow-hidden">
      <Backdrop />

      <div className="flex flex-1 min-h-0">
        <Sidebar
          view={activeView}
          onOpenParent={onOpenParent}
          onOpenTab={onOpenTab}
          agentsCount={agentsCount}
          data={data}
          mode={narrow ? 'rail' : sidebarMode}
          setMode={setSidebarMode}
          pushToast={pushToast}
          appVersion={appVersion}
          policyBadge={policyBadge}
          needsYouCount={needsYouCount}
          needsYouDegraded={needsYouDegraded}
          lastOrchEventAt={lastOrchEventAt}
          orchUsesPolling={orchUsesPolling}
          liveFreshMs={liveFreshMs}
          onOpenCommandPalette={onOpenCommandPalette}
          chatSessions={chatSessions}
          activeSessionId={activeSessionId}
          chatTaskCounts={chatTaskCounts}
          archivedChatSessions={archivedChatSessions}
          showArchivedChatSessions={showArchivedChatSessions}
          pendingHarnessIssueSessionIds={pendingHarnessIssueSessionIds}
          onSessionChange={onSessionChange}
          onCreateSession={onCreateSession}
          onRenameSession={onRenameSession}
          onArchiveSession={onArchiveSession}
          onUnarchiveSession={onUnarchiveSession}
          onToggleArchivedSessions={onToggleArchivedSessions}
          onTaskBadgeClick={onTaskBadgeClick}
        />

        {/* data-view: stable hook for e2e to assert which surface is mounted (the workbench tab bar that used to expose this was removed in #460). */}
        <main className="flex-1 flex flex-col min-w-0 relative" data-testid="active-surface" data-view={activeView}>
          {/* The page's one h1 (axe page-has-heading-one): surfaces carry section headings only, so a surface docked inside another cannot add a second. */}
          <h1 className="sr-only">{surfaceLabel}</h1>
          <div className="px-4 pt-3 pb-0">
            <BreadcrumbBar viewKey={activeView} onNavigate={onNavigate} gamifyEnabled={gamifyEnabled} />
          </div>

          <div className={`flex-1 min-h-0 flex flex-col overflow-hidden p-5 max-[639px]:px-2 ${mainPaddingBottom}`}>
            <SurfaceErrorBoundary key={surfaceKey} surface={surfaceLabel}>
              <SurfaceScrollHost>{children}</SurfaceScrollHost>
            </SurfaceErrorBoundary>
          </div>

          {chatDocked && chatDock != null && (
            <div className="p-4 pt-0 mt-auto" data-testid="loquela-dock">
              {chatDock}
            </div>
          )}
        </main>
      </div>

      <BottomStatusBar
        kpis={kpis}
        hudTilesConfig={hudTilesConfig}
        onHudTilesChange={onHudTilesChange}
        onNavigate={onNavigate}
        lastOrchEventAt={lastOrchEventAt}
        orchUsesPolling={orchUsesPolling}
        liveFreshMs={liveFreshMs}
        routingSummary={routingSummary}
        routingHealth={routingHealth}
        openrouterSpendUsd={openrouterSpendUsd}
        sessionSpentUsd={sessionSpentUsd}
        needsYouCount={needsYouCount}
        needsYouDegraded={needsYouDegraded}
        meshNodes={meshNodes}
        gamifyEnabled={gamifyEnabled}
        onOpenAchievements={onOpenAchievements}
        onOpenResearchDrawer={onOpenResearchDrawer}
        notices={notices}
        onMarkAllNoticesRead={onMarkAllNoticesRead}
      />
    </div>
  );
}
