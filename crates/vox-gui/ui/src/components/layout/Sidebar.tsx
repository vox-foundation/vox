import React, { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { Glass } from '../ui/Glass';
import { Icon } from '../ui/Icons';
import { AxisMark } from '../ui/AxisMark';
import { DashboardData } from '../../types/dashboard';
import { SURFACE_REGISTRY } from '../../generated/surfaceRegistry.generated';
import { TOP_LEVEL_VIEWS, resolveNavigation, CHILD_ORDER_BY_PARENT, childLabelFor } from '../../lib/navigation';
import { STATUS_BADGE_CLASS, STATUS_RAIL_BADGE_CLASS } from '../../styles/tokens';
import { useFreshness } from '../../hooks/useFreshness';
import { useLang } from '../../hooks/useLanguage';
import { LEXICON, labelFor, sidebarParentLabel } from '../../lib/lexicon';
import { SessionSidebarSection } from './SessionSidebarSection';
import type { ChatSession } from '../../lib/useChatSessions';
import type { Toast } from '../../types/tauri';
import { surfaceAvailability, useBackendStatus } from '../../hooks/useBackendStatus';
import { BackendStatusPill } from './BackendStatusPill';
import { UNAVAILABLE_COPY } from './SurfaceUnavailable';
import { RomanTooltip, RomanMedallion } from '../ui/RomanTooltip';

export type SidebarMode = 'hidden' | 'rail' | 'default' | 'wide';

type PolicyBadgeStatus = 'pass' | 'fail' | 'warn' | 'not_run';
export interface PolicyBadge { count: number; status: PolicyBadgeStatus; }

const PARENT_NAV_PURPOSE: Record<string, { purpose: string; action: string }> = {
  chat: {
    purpose: 'Interactive AI pair-programming console with live orchestrator spoke routing, context pinning, and plan execution.',
    action: 'Click to open Chat & manage conversation sessions',
  },
  runs: {
    purpose: 'Human-in-the-loop review center for pending tool approvals, security policy gates, and verification checks.',
    action: 'Click to inspect items needing your review',
  },
  agents: {
    purpose: 'Executive mission control for active agents, workflow flow, task queues, mesh topology, and sub-agent trees.',
    action: 'Click to open Agents Dashboard & subsections',
  },
  workspace: {
    purpose: 'Source code repository timeline, interactive 3D/2.5D Workspace Map, Arena quests, and pipeline cascade.',
    action: 'Click to inspect Repository, Workspace Map & Arena',
  },
  knowledge: {
    purpose: 'Scientia deep research, indexed documentation facets, vector memory, and architectural knowledge base.',
    action: 'Click to search and manage workspace knowledge',
  },
  commands: {
    purpose: 'Unified CLI command catalog, installed Agent Skills, MCP tool inspector, and automation scripts.',
    action: 'Click to browse and invoke commands, skills & tools',
  },
  compute: {
    purpose: 'MENS local QLoRA model training studio, model router policy, and Populi distributed compute nodes.',
    action: 'Click to configure MENS training & model routing',
  },
  mercatus: {
    purpose: 'Token spend analytics, prompt cache efficiency, provider cost breakdown, and budget thresholds.',
    action: 'Click to inspect token budgets and cost telemetry',
  },
  settings: {
    purpose: 'Global Vox Axis configuration, Clavis secret keys (Hugging Face, OpenRouter), and policy enforcement.',
    action: 'Click to configure keys, preferences & policies',
  },
  explorer: {
    purpose: 'Complete directory of all 32 Vox Axis surfaces, capabilities, and diagnostic tools in one place.',
    action: 'Click to browse every surface in Vox Axis',
  },
};

interface NavItemProps {
  active: boolean;
  icon: React.ReactNode;
  label: string;
  onClick: (e?: React.MouseEvent<HTMLButtonElement>) => void;
  badge?: number | string | null;
  badgeClass?: string;
  railBadgeClass?: string;
  collapsed: boolean;
  innerRef?: React.Ref<HTMLButtonElement>;
  ariaLabel?: string;
  tooltipPurpose?: string;
  tooltipAction?: string;
}

function NavItem({ active, icon, label, onClick, badge, badgeClass, railBadgeClass, collapsed, innerRef, ariaLabel, tooltipPurpose, tooltipAction }: NavItemProps) {
  const effectiveAriaLabel = ariaLabel ?? (collapsed ? label : undefined);
  const btn = (
    <button type="button" ref={innerRef} onClick={onClick} title={collapsed ? label : undefined} aria-label={effectiveAriaLabel}
      className={`group relative flex w-full items-center ${collapsed ? "justify-center px-0" : "gap-2 px-2"} py-1.5 rounded-xl border transition-all duration-150 ${
        active
          ? "border-[#9E7B2F] bg-linear-to-r from-[#DFD0B5] via-[#EBE0C9] to-[#F7F1E5] text-[#1C140C] font-bold shadow-[0_3px_10px_rgba(68,50,20,0.22),inset_0_1px_0_rgba(255,255,255,0.92)]"
          : "border-transparent text-[#2B2013] hover:border-[#C59B27]/55 hover:bg-[#EFE6D4]/85 hover:text-[#1C140C]"
      }`}>
      {active && <span className="absolute left-0 top-1/2 -translate-y-1/2 h-5 w-[3.5px] rounded-r bg-[#B8860B] shadow-[0_0_8px_rgba(184,134,11,0.65)]" />}
      <RomanMedallion active={active} size="md">{icon}</RomanMedallion>
      {!collapsed && <span className="flex-1 min-w-0 text-left font-display text-[10.5px] font-bold tracking-[0.09em] uppercase whitespace-nowrap overflow-hidden text-ellipsis">{label}</span>}
      {!collapsed && badge != null && <span className={`rounded-full border border-[#8C6825] px-1.5 py-0.5 font-mono text-[9px] font-semibold ${badgeClass ?? 'bg-[#251C0E] text-[#F3E5AB]'}`}>{badge}</span>}
      {collapsed && badge != null && <span className={`absolute right-1 top-1 rounded-full px-1 font-mono text-[8px] ${railBadgeClass ?? 'bg-[#8C6825] text-[#FFF8E7] font-bold'}`}>{badge}</span>}
    </button>
  );

  if (tooltipPurpose) {
    return (
      <RomanTooltip
        title={label}
        purpose={tooltipPurpose}
        actionHint={tooltipAction}
        statusBadge={badge != null ? String(badge) : undefined}
        placement="right"
        className="flex w-full"
      >
        {btn}
      </RomanTooltip>
    );
  }
  return btn;
}

const SIDEBAR_WIDTHS: Record<SidebarMode, number> = { hidden: 0, rail: 64, default: 184, wide: 248 };
const SIDEBAR_ORDER: SidebarMode[] = ["rail", "default", "wide"];

// Curated nav icons & 5-domain consolidation headers (Concept-Art v4.0: Operate/Chat, Agents & Runs, Workspace, Knowledge, Compute & System).
const TOP_NAV_ICON: Record<string, string> = {
  chat: 'message',
  runs: 'shield',
  agents: 'users',
  knowledge: 'book',
  workspace: 'folder',
  commands: 'terminal',
  compute: 'cpu',
  mercatus: 'scale',
  settings: 'settings',
};

const SIDEBAR_DOMAIN_HEADER_BEFORE: Record<string, { id: string; label: string }> = {
  chat: { id: 'operate-chat', label: 'Operate · Chat' },
  runs: { id: 'agents-runs', label: 'Agents & Runs' },
  workspace: { id: 'workspace', label: 'Workspace' },
  knowledge: { id: 'knowledge', label: 'Knowledge' },
  compute: { id: 'compute-system', label: 'Compute & System' },
};

const navLabelFor = (key: string, lang: 'en' | 'la') => sidebarParentLabel(key, lang);

interface SidebarProps {
  view: string;
  onOpenParent: (parentKey: string) => void;
  onOpenTab: (viewKey: string) => void;
  agentsCount: number;
  data: DashboardData;
  mode: SidebarMode;
  setMode: (m: SidebarMode) => void;
  pushToast: (t: Toast) => void;
  appVersion?: string;
  policyBadge?: PolicyBadge | null;
  needsYouCount?: number;
  needsYouDegraded?: string[];
  lastOrchEventAt?: number | null;
  orchUsesPolling?: boolean;
  liveFreshMs?: number;
  onOpenCommandPalette?: () => void;
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

export function Sidebar({
  view,
  onOpenParent,
  onOpenTab,
  agentsCount,
  mode,
  setMode,
  appVersion,
  policyBadge,
  needsYouCount,
  needsYouDegraded,
  lastOrchEventAt = null,
  orchUsesPolling = false,
  liveFreshMs = 10_000,
  onOpenCommandPalette,
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
}: SidebarProps) {
  const lastVisibleModeRef = useRef<Exclude<SidebarMode, 'hidden'>>(
    mode === 'hidden' ? 'default' : mode,
  );
  const [localHidden, setLocalHidden] = useState(mode === 'hidden');
  useEffect(() => {
    if (mode === 'hidden') {
      setLocalHidden(true);
    } else {
      lastVisibleModeRef.current = mode;
      setLocalHidden(false);
    }
  }, [mode]);

  const isHidden = mode === 'hidden' || localHidden;
  const effectiveMode: SidebarMode = isHidden ? 'hidden' : mode;
  const w = SIDEBAR_WIDTHS[effectiveMode];
  const collapsed = effectiveMode === "rail";
  const { parent: activeParent } = resolveNavigation(view);
  const [identity, setIdentity] = useState('operator@vox');
  const tone = useFreshness(lastOrchEventAt, {
    freshMs: liveFreshMs,
    usesPolling: orchUsesPolling,
  });
  const orchDotClass =
    tone === 'live' ? 'bg-accent-secondary' : tone === 'poll' ? 'bg-brass' : 'bg-text-muted';

  useEffect(() => {
    invoke<{ display_name: string }>('get_identity_summary')
      .then(i => setIdentity(i.display_name))
      .catch(() => {});
  }, []);

  const cycle = (dir: number) => {
    const i = SIDEBAR_ORDER.indexOf(effectiveMode);
    const ni = Math.max(0, Math.min(SIDEBAR_ORDER.length - 1, i + dir)) as number;
    setMode(SIDEBAR_ORDER[ni]);
  };

  const handleHideSidebar = () => {
    if (mode !== 'hidden') {
      lastVisibleModeRef.current = mode;
    }
    setLocalHidden(true);
    setMode('hidden');
  };

  const handleRestoreSidebar = () => {
    const target = lastVisibleModeRef.current || 'default';
    setLocalHidden(false);
    setMode(target);
  };

  const { lang } = useLang();
  const { status: backendStatus } = useBackendStatus();
  const activeRef = useRef<HTMLButtonElement>(null);

  const visibleTopLevel = TOP_LEVEL_VIEWS.filter(k => k !== 'settings');

  // null = no override (show active parent's children).
  // '' (empty string) = user explicitly collapsed the active parent.
  const [peekedParent, setPeekedParent] = useState<string | null>(null);
  const [railMenuOpenKey, setRailMenuOpenKey] = useState<string | null>(null);
  const [railMenuPos, setRailMenuPos] = useState<{ top: number; left: number } | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setPeekedParent(null);
  }, [activeParent]);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    };
  }, []);

  const handleRailMouseEnter = (key: string, e: React.MouseEvent<HTMLElement>) => {
    if (!collapsed) return;
    if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    const rect = e.currentTarget.getBoundingClientRect();
    setRailMenuPos({ top: rect.top, left: rect.right + 6 });
    setRailMenuOpenKey(key);
  };

  const handleRailMouseLeave = () => {
    if (!collapsed) return;
    if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    closeTimerRef.current = setTimeout(() => {
      setRailMenuOpenKey(null);
      setRailMenuPos(null);
    }, 150);
  };

  const handleFlyoutMouseEnter = () => {
    if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
  };

  const handleFlyoutMouseLeave = () => {
    if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    closeTimerRef.current = setTimeout(() => {
      setRailMenuOpenKey(null);
      setRailMenuPos(null);
    }, 150);
  };

  const expandedParent = peekedParent === '' ? null : peekedParent ?? activeParent;

  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: 'nearest' });
  }, [view]);

  const settingsEntry = SURFACE_REGISTRY.find(e => e.viewKey === 'settings');
  const explorerEntry = SURFACE_REGISTRY.find(e => e.viewKey === 'explorer');

  if (isHidden) {
    return (
      <aside
        aria-label="Sidebar"
        data-sidebar-hidden="true"
        className="shrink-0 flex flex-col transition-[width] duration-200 ease-out h-full overflow-visible sticky top-0 z-50"
        style={{ width: 0 }}
      >
        <RomanTooltip
          title="Restore Navigation Sidebar"
          purpose="Re-opens the Vox Axis navigation rail to switch between Chat, Agents, Workspace, Knowledge, and Compute surfaces."
          actionHint="Click to expand the Sidebar"
          placement="bottom"
        >
          <button
            type="button"
            data-testid="sidebar-restore-button"
            onClick={handleRestoreSidebar}
            aria-label="Show sidebar"
            aria-expanded={false}
            title="Show sidebar"
            className="fixed left-3 top-2.5 z-50 inline-flex items-center gap-1.5 rounded-lg border-2 border-[#D4AF37]/65 bg-linear-to-b from-[#2A2011] to-[#120E08] px-2.5 py-1.5 font-display text-[10px] font-semibold uppercase tracking-[0.14em] text-[#F3E5AB] shadow-[0_6px_18px_rgba(0,0,0,0.75),inset_0_1px_0_rgba(243,229,171,0.3)] transition hover:border-[#F3E5AB] hover:text-white"
          >
            <Icon.romanColumn className="size-3.5 text-[#D4AF37]" aria-hidden="true" />
            <span>Sidebar</span>
            <Icon.chevR className="size-3 text-[#D4AF37]" aria-hidden="true" />
          </button>
        </RomanTooltip>
      </aside>
    );
  }

  return (
    <aside aria-label="Sidebar" className="shrink-0 flex flex-col transition-[width] duration-200 ease-out h-full overflow-hidden sticky top-0" style={{ width: w }}>
      <Glass
        style={{
          backgroundImage:
            'linear-gradient(180deg, rgba(248,243,233,0.95) 0%, rgba(238,229,211,0.93) 50%, rgba(229,217,195,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="flex h-full flex-col p-2 rounded-none border-y-0 border-l-0 border-r-2 border-[#C59B27] shadow-[4px_0_18px_rgba(0,0,0,0.45),inset_-1px_0_0_rgba(255,255,255,0.85)]"
      >
        <div
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(252,248,240,0.94) 0%, rgba(236,226,207,0.9) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className={`flex items-center ${collapsed ? "justify-center p-1.5" : "justify-between px-2 py-1.5"} mb-1.5 rounded-lg border-2 border-[#C59B27] shadow-[0_3px_10px_rgba(68,50,20,0.25),inset_0_1px_0_rgba(255,255,255,0.95)] shrink-0`}
        >
          {collapsed && (
            <div className="grid size-6 place-items-center rounded-full border border-[#8C6825] bg-[#251C0E] shrink-0">
              <AxisMark className="size-4 text-[#F3E5AB]" />
            </div>
          )}
          {!collapsed && (
            <div className="flex items-center gap-1.5 min-w-0">
              <RomanMedallion active size="sm">
                <AxisMark size={14} />
              </RomanMedallion>
              <div className="leading-tight min-w-0">
                <div className="font-display text-[10.5px] font-bold tracking-[0.16em] text-[#1C140C] truncate">AXIS</div>
                <div className="font-display text-[7.5px] font-semibold uppercase tracking-wider text-[#5C441E] truncate">Control Studio</div>
              </div>
            </div>
          )}
          <div className={`flex items-center ${collapsed ? "flex-col gap-1" : "gap-0.5"}`}>
            <button type="button" onClick={() => cycle(-1)} disabled={effectiveMode === "rail"} title="Collapse to icon rail" aria-label="Collapse sidebar"
              className={`flex size-5.5 items-center justify-center rounded-md border border-[#9E7B2F]/65 bg-[#231B12] ${effectiveMode === "rail" ? "opacity-30 cursor-not-allowed" : "hover:border-[#D4AF37] text-[#F3E5AB] hover:text-white"}`}>
              <Icon.chevL className="size-3" aria-hidden="true"/>
            </button>
            <button type="button" onClick={() => cycle(1)} disabled={effectiveMode === "wide"} title="Expand sidebar width" aria-label="Expand sidebar"
              className={`flex size-5.5 items-center justify-center rounded-md border border-[#9E7B2F]/65 bg-[#231B12] ${effectiveMode === "wide" ? "opacity-30 cursor-not-allowed" : "hover:border-[#D4AF37] text-[#F3E5AB] hover:text-white"}`}>
              <Icon.chevR className="size-3" aria-hidden="true"/>
            </button>
            <button
              type="button"
              data-testid="sidebar-hide-button"
              onClick={handleHideSidebar}
              title="Hide sidebar"
              aria-label="Hide sidebar"
              aria-expanded={true}
              className="flex size-5.5 items-center justify-center rounded-md border border-[#9E7B2F]/65 bg-[#231B12] text-[#F3E5AB] transition hover:border-[#D4AF37] hover:text-white"
            >
              <Icon.x className="size-3" aria-hidden="true" />
            </button>
          </div>
        </div>

        {onOpenCommandPalette && (
          <RomanTooltip
            title="Command & Surface Search"
            purpose="Instantly jump to any of the 32 Vox Axis surfaces, run a CLI command, or search indexed documentation."
            actionHint="Click or press ⌘K to open OmniSearch"
            shortcut="⌘K"
            placement="right"
            className="flex w-full shrink-0"
          >
            <button
              type="button"
              data-testid="omnisearch-trigger"
              onClick={onOpenCommandPalette}
              title={collapsed ? 'Search or jump…' : undefined}
              aria-label="Search or jump to…"
              className={`group relative flex w-full items-center ${collapsed ? "justify-center px-0" : "gap-2 px-2"} py-1.5 mb-1.5 rounded-xl border border-[#9E7B2F]/65 bg-linear-to-b from-[#FAF6EE] to-[#EAE0CC] text-[#1C140C] shadow-[0_2px_6px_rgba(68,50,20,0.16),inset_0_1px_0_rgba(255,255,255,0.92)] transition hover:border-[#8C6825] hover:bg-[#EFE6D4] shrink-0`}
            >
              <RomanMedallion size="md">
                <Icon.search className="size-3.5" aria-hidden="true" />
              </RomanMedallion>
              {!collapsed && (
                <>
                  <span className="flex-1 min-w-0 text-left font-display text-[10.5px] font-bold tracking-[0.1em] uppercase whitespace-nowrap overflow-hidden text-ellipsis text-[#1C140C]">
                    Search
                  </span>
                  <span className="rounded border border-[#8C6825] bg-[#231B12] px-1.5 py-0.5 font-mono text-[8.5px] tracking-widest text-[#F3E5AB]">⌘K</span>
                </>
              )}
            </button>
          </RomanTooltip>
        )}

        <nav aria-label="Primary navigation" className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden custom-scrollbar flex flex-col gap-0.5 -mr-1 pr-1">
          {visibleTopLevel.map(key => {
            const label = navLabelFor(key, lang);
            const IconCmp = (Icon as Record<string, any>)[TOP_NAV_ICON[key] ?? 'file'] ?? Icon.file;
            const isActive = activeParent === key;
            const isExpanded = expandedParent === key && mode !== 'rail';
            const children = CHILD_ORDER_BY_PARENT[key];
            const hasReviewItems = key === 'runs' && needsYouCount != null && needsYouCount > 0;
            const couldNotLoad =
              key === 'runs' && (needsYouDegraded?.length ?? 0) > 0 ? `couldn't load ${needsYouDegraded!.join(', ')}` : '';
            const badge =
              key === 'agents' ? agentsCount
              : hasReviewItems ? needsYouCount
              : couldNotLoad ? '!'
              : undefined;
            const navAriaLabel =
              key === 'runs'
                ? hasReviewItems
                  ? `Review, ${needsYouCount} items need you${couldNotLoad ? ` (${couldNotLoad})` : ''}`
                  : couldNotLoad
                    ? `Review, ${couldNotLoad}`
                    : 'Review'
                : undefined;
            const domainHeader = !collapsed ? SIDEBAR_DOMAIN_HEADER_BEFORE[key] : undefined;
            const navMeta = PARENT_NAV_PURPOSE[key];
            return (
              <React.Fragment key={key}>
                {domainHeader && (
                  <div
                    data-testid={`sidebar-domain-group-${domainHeader.id}`}
                    className="mx-1.5 mt-1.5 mb-0.5 border-b border-[#9E7B2F]/45 pb-0.5 font-display text-[8.5px] font-bold uppercase tracking-[0.18em] text-[#5C441E]"
                  >
                    {domainHeader.label}
                  </div>
                )}
                <div
                  className="relative flex items-center gap-0.5"
                  onMouseEnter={(e) => collapsed && children && handleRailMouseEnter(key, e)}
                  onMouseLeave={handleRailMouseLeave}
                >
                  <div className="flex-1 min-w-0">
                    <NavItem
                      innerRef={isActive ? activeRef : undefined}
                      collapsed={collapsed}
                      active={isActive}
                      onClick={(e) => {
                        onOpenParent(key);
                        if (collapsed && children) {
                          if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
                          if (railMenuOpenKey === key) {
                            setRailMenuOpenKey(null);
                            setRailMenuPos(null);
                          } else {
                            const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
                            setRailMenuPos({ top: rect.top, left: rect.right + 6 });
                            setRailMenuOpenKey(key);
                          }
                        }
                      }}
                      icon={<IconCmp className="size-3.5" />}
                      label={label}
                      badge={badge}
                      ariaLabel={navAriaLabel}
                      tooltipPurpose={navMeta?.purpose}
                      tooltipAction={navMeta?.action}
                    />
                  </div>
                  {/* chat has no child views but expands to its session list, so it needs the toggle too */}
                  {(children || key === 'chat') && mode !== 'rail' && (
                    <button
                      type="button"
                      aria-label={`${isExpanded ? 'Collapse' : 'Expand'} ${label}`}
                      aria-expanded={isExpanded}
                      onClick={() => setPeekedParent(isExpanded ? '' : key)}
                      className="flex size-5.5 shrink-0 items-center justify-center rounded-md text-[#4A3718] hover:bg-[#DFD2BC] hover:text-[#1C140C]"
                    >
                      <Icon.chevR className={`size-3 transition-transform ${isExpanded ? 'rotate-90 text-[#8C6825]' : ''}`} aria-hidden="true" />
                    </button>
                  )}
                  {collapsed && railMenuOpenKey === key && children && typeof document !== 'undefined' && createPortal(
                    <div
                      role="menu"
                      aria-label={`${label} subsections`}
                      data-testid={`rail-flyout-${key}`}
                      onMouseEnter={handleFlyoutMouseEnter}
                      onMouseLeave={handleFlyoutMouseLeave}
                      onKeyDown={(e) => {
                        if (e.key === 'Escape') {
                          setRailMenuOpenKey(null);
                          setRailMenuPos(null);
                        }
                      }}
                      style={railMenuPos ? { top: `${railMenuPos.top}px`, left: `${railMenuPos.left}px` } : undefined}
                      className="fixed z-[99999] min-w-[190px] rounded-xl border-2 border-[#C59B27] bg-linear-to-b from-[#F8F3E8] to-[#E6DAC4] p-1.5 shadow-2xl animate-in fade-in-0 zoom-in-95 pointer-events-auto"
                    >
                      <div className="px-2.5 py-1 text-[10px] font-display font-bold uppercase tracking-widest text-[#1C140C] border-b border-[#9E7B2F]/45 mb-1">
                        {label}
                      </div>
                      <div className="flex flex-col gap-0.5">
                        {children.map(childKey => (
                          <button
                            key={childKey}
                            type="button"
                            role="menuitem"
                            onClick={(e) => {
                              e.stopPropagation();
                              onOpenTab(childKey);
                              setRailMenuOpenKey(null);
                              setRailMenuPos(null);
                            }}
                            className={`flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs transition ${
                              view === childKey
                                ? 'border border-[#8C6825] bg-[#251C0E] text-[#F3E5AB] font-semibold'
                                : 'text-[#2B2013] hover:bg-[#DFD2BC] hover:text-[#1C140C]'
                            }`}
                          >
                            {childLabelFor(childKey, lang)}
                          </button>
                        ))}
                      </div>
                    </div>,
                    document.body
                  )}
                </div>
                {isExpanded && key === 'chat' && chatSessions && (
                  <div className="ml-3 border-l-2 border-[#9E7B2F]/55 pl-1.5 max-h-[50vh] overflow-y-auto custom-scrollbar rounded-r bg-[#231B12]/90 py-1 pr-1">
                    <SessionSidebarSection
                      sessions={chatSessions}
                      activeSessionId={activeSessionId ?? null}
                      taskCounts={chatTaskCounts ?? {}}
                      archivedSessions={archivedChatSessions ?? []}
                      showArchived={showArchivedChatSessions ?? false}
                      pendingIssueSessionIds={pendingHarnessIssueSessionIds}
                      onSessionChange={onSessionChange ?? (() => {})}
                      onCreateSession={onCreateSession ?? (() => {})}
                      onRenameSession={onRenameSession ?? (() => {})}
                      onArchiveSession={onArchiveSession ?? (() => {})}
                      onUnarchiveSession={onUnarchiveSession ?? (() => {})}
                      onToggleArchivedView={onToggleArchivedSessions ?? (() => {})}
                      onTaskBadgeClick={onTaskBadgeClick ?? (() => {})}
                    />
                  </div>
                )}
                {isExpanded && key !== 'chat' && children && (
                  <div
                    data-testid="sidebar-child-list"
                    className={
                      peekedParent === key
                        ? 'ml-3 flex flex-col gap-0.5 border-l-2 border-[#9E7B2F]/55 pl-2 py-0.5'
                        : 'sr-only'
                    }
                  >
                    {children.map(childKey => {
                      const availability = surfaceAvailability(backendStatus, childKey);
                      return (
                        <button
                          key={childKey}
                          type="button"
                          onClick={() => onOpenTab(childKey)}
                          aria-current={view === childKey ? 'page' : undefined}
                          data-unavailable={availability.available ? undefined : availability.reason ?? undefined}
                          title={availability.available || !availability.reason ? undefined : UNAVAILABLE_COPY[availability.reason].title}
                          className={`w-full rounded-lg px-2 py-1 text-left font-display text-[10px] tracking-widest uppercase transition ${
                            view === childKey
                              ? 'border border-[#8C6825] bg-[#251C0E] text-[#F3E5AB] font-bold'
                              : 'text-[#2B2013] hover:bg-[#DFD2BC] hover:text-[#1C140C]'
                          } ${availability.available ? '' : 'opacity-50'}`}
                        >
                          {childLabelFor(childKey, lang)}
                        </button>
                      );
                    })}
                  </div>
                )}
              </React.Fragment>
            );
          })}
        </nav>

        <div className="flex flex-col gap-1 pt-1.5 shrink-0">
          <BackendStatusPill collapsed={collapsed} />
          {settingsEntry && (
            <div className="flex flex-col gap-0.5 border-t border-[#9E7B2F]/45 pt-1">
              <NavItem
                collapsed={collapsed}
                active={activeParent === 'settings'}
                onClick={() => onOpenTab('settings')}
                icon={<Icon.settings className="size-3.5" />}
                label={labelFor('settings', lang)}
                badge={policyBadge && policyBadge.count > 0 ? policyBadge.count : undefined}
                badgeClass={policyBadge ? STATUS_BADGE_CLASS[policyBadge.status] : undefined}
                railBadgeClass={policyBadge ? STATUS_RAIL_BADGE_CLASS[policyBadge.status] : undefined}
                ariaLabel={
                  policyBadge && policyBadge.count > 0
                    ? `Settings, ${policyBadge.count} policy failures`
                    : 'Settings'
                }
                tooltipPurpose={PARENT_NAV_PURPOSE.settings.purpose}
                tooltipAction={PARENT_NAV_PURPOSE.settings.action}
              />
              {explorerEntry && (
                <NavItem
                  collapsed={collapsed}
                  active={view === 'explorer'}
                  onClick={() => onOpenTab('explorer')}
                  icon={<Icon.branch className="size-3.5" />}
                  label={labelFor('explorer', lang)}
                  ariaLabel="Explorer, browse everything Vox can do"
                  tooltipPurpose={PARENT_NAV_PURPOSE.explorer.purpose}
                  tooltipAction={PARENT_NAV_PURPOSE.explorer.action}
                />
              )}
            </div>
          )}

          <div className={`flex items-center ${collapsed ? "justify-center" : "gap-2 px-2"} rounded-lg border border-[#9E7B2F]/65 bg-linear-to-r from-[#261D10] to-[#1A130A] py-1.5 shadow-[inset_0_1px_0_rgba(243,229,171,0.22)]`}>
            <div className="relative size-6 shrink-0 rounded-full border border-[#D4AF37]/80 bg-linear-to-br from-[#6B5022] to-[#23190B] grid place-items-center text-[#F3E5AB] font-display text-[10px] font-bold">
              V
              <span
                data-testid="sidebar-orch-freshness-dot"
                aria-hidden="true"
                className={`absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full ring-2 ring-[#1A130A] ${orchDotClass}`}
              />
            </div>
            {!collapsed && (
              <div className="flex-1 leading-tight overflow-hidden">
                <div className="font-display text-[10px] font-semibold text-[#F3E5AB] truncate">{identity}</div>
                <div className="font-mono text-[8px] text-[#D8C7A8] truncate">Vox Axis · v{appVersion ?? '0.6.0'}</div>
              </div>
            )}
          </div>
        </div>
      </Glass>
    </aside>
  );
}
