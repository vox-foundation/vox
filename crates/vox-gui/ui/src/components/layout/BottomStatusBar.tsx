import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../ui/Glass';
import { Icon } from '../ui/Icons';
import { RomanTooltip } from '../ui/RomanTooltip';
import { formatSpend } from '../../config/budget';
import { useFreshness } from '../../hooks/useFreshness';
import {
  resolveVisibleHudTiles,
  toggleHudTile,
  reorderHudTile,
  defaultHudTiles,
  normalizeHudTiles,
  HUD_TILE_LABELS,
  type HudTilesConfig,
  type HudTileKind,
} from '../../hooks/useHudTiles';
import { INITIAL_KPIS } from '../../data/initialState';
import { WORKBENCH_TABBAR_TRAILING_SLOT_ID } from '../../lib/domIds';
import { routingCardValue, routingHealthProblems } from '../../lib/routingSummary';
import type { RoutingHealth, RoutingSummary } from '../../types/tauri';
import type { MeshNode } from '../surfaces/Mesh/MeshView';
import { StatusBarCluster } from '../common/StatusBarCluster';
import { NotificationCenter } from '../common/NotificationCenter';
import type { Notice } from '../../lib/noticeStore';

type KpiState = typeof INITIAL_KPIS;

export interface BottomStatusBarProps {
  kpis: KpiState;
  hudTilesConfig: HudTilesConfig;
  onHudTilesChange: (config: HudTilesConfig) => void;
  onNavigate: (view: string) => void;
  lastOrchEventAt: number | null;
  orchUsesPolling: boolean;
  liveFreshMs: number;
  /** Global routing pick (get_routing_summary_live); a version is shown only when catalog-resolved. */
  routingSummary?: RoutingSummary | null;
  /** Routing health (get_routing_health); the Routing card shows a dot only when it reports a problem. */
  routingHealth?: RoutingHealth | null;
  openrouterSpendUsd?: number | null;
  /** This chat session's spend (get_llm_spend sessionUsd), shown in the Spend popover. */
  sessionSpentUsd?: number | null;
  /** Approvals plus open questions (attention inbox `totalCount`). */
  needsYouCount?: number | null;
  /** Inbox sources that failed to load; the card never shows a clear 0 while any are unknown. */
  needsYouDegraded?: string[];
  meshNodes?: MeshNode[];
  gamifyEnabled?: boolean;
  onOpenAchievements?: () => void;
  onOpenResearchDrawer?: () => void;
  /** Every toast and engine problem the app has kept; the bell is the last item when both are given. */
  notices?: Notice[];
  onMarkAllNoticesRead?: () => void;
}

interface VcsStatus {
  branch: string;
  commit: string;
  dirtyCount: number;
  version: string;
}

const FALLBACK_VCS_STATUS: VcsStatus = {
  branch: 'main',
  commit: '98ea5d2',
  dirtyCount: 0,
  version: '0.6.0',
};

function freshnessClasses(tone: 'live' | 'poll' | 'stale') {
  if (tone === 'live') {
    return {
      pill: 'border-[#5B6B24]/65 bg-[#E9EFD5] text-[#2B370E] font-bold',
      dot: 'bg-[#4D5D1E]',
      label: 'Live',
      title: 'Live: receiving engine events',
    };
  }
  if (tone === 'poll') {
    return {
      pill: 'border-[#8C6825]/65 bg-[#FBF3DC] text-[#5C441E] font-bold',
      dot: 'bg-[#B8860B]',
      label: 'Poll',
      title: 'Polling: no event stream, refreshing on a timer',
    };
  }
  return {
    pill: 'border-[#9E7B2F]/45 bg-[#EAE0CE] text-[#5C441E]',
    dot: 'bg-[#8C6825]',
    label: 'Offline',
    title: 'Offline: no engine data recently',
  };
}

function Segment({
  testId,
  hudTileId,
  label,
  value,
  onClick,
  onDoubleClick,
  expanded,
  buttonRef,
  ariaLabel,
  badge,
  tooltip,
}: {
  testId: string;
  hudTileId?: string;
  label: string;
  value: string;
  onClick: (e: React.MouseEvent<HTMLButtonElement>) => void;
  onDoubleClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  /** Overrides the accessible name (the visible label and value stay as they are). */
  ariaLabel?: string;
  badge?: React.ReactNode;
  /** Set only on a card that opens a popover. */
  expanded?: boolean;
  buttonRef?: React.Ref<HTMLButtonElement>;
  tooltip?: {
    title: string;
    purpose: string;
    actionHint?: string;
    statusBadge?: string;
  };
}) {
  const button = (
    <button
      ref={buttonRef}
      type="button"
      data-testid={testId}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      aria-label={ariaLabel}
      aria-haspopup={expanded === undefined ? undefined : 'dialog'}
      aria-expanded={expanded}
      className={`inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded px-1 xl:px-1.5 py-0.5 text-[9.5px] xl:text-[10.5px] font-mono transition ${
        expanded
          ? 'border border-[#8C6825] bg-[#251C0E] text-[#F3E5AB] shadow-[0_1px_6px_rgba(0,0,0,0.3)]'
          : 'text-[#2B2013] hover:border hover:border-[#9E7B2F]/60 hover:bg-[#DFD2BC]/85 hover:text-[#1C140C]'
      }`}
    >
      <span
        data-testid={hudTileId}
        data-card-label
        className={`font-display uppercase tracking-[0.06em] xl:tracking-[0.08em] font-bold text-[9px] xl:text-[9.5px] ${
          expanded ? 'text-[#D4AF37]' : 'text-[#6B4C12]'
        }`}
      >
        {label}
      </span>
      <span
        data-card-value
        title={value}
        className={`inline-block max-w-[48ch] truncate align-bottom font-mono tabular-nums font-medium ${
          expanded ? 'text-[#F3E5AB]' : 'text-[#1C140C]'
        }`}
      >
        {value}
      </span>
      {badge}
    </button>
  );

  if (tooltip) {
    return (
      <RomanTooltip
        title={tooltip.title}
        purpose={tooltip.purpose}
        actionHint={tooltip.actionHint ?? 'Click for live controls or jump to surface'}
        statusBadge={tooltip.statusBadge}
        placement="top"
        disabled={expanded}
      >
        {button}
      </RomanTooltip>
    );
  }

  return button;
}

export function BottomStatusBar({
  kpis,
  hudTilesConfig,
  onHudTilesChange,
  onNavigate,
  lastOrchEventAt,
  orchUsesPolling,
  liveFreshMs,
  routingSummary = null,
  routingHealth = null,
  openrouterSpendUsd = null,
  sessionSpentUsd = null,
  needsYouCount = null,
  needsYouDegraded = [],
  meshNodes,
  gamifyEnabled = false,
  onOpenAchievements,
  onOpenResearchDrawer,
  notices,
  onMarkAllNoticesRead,
}: BottomStatusBarProps) {
  const tone = useFreshness(lastOrchEventAt, {
    freshMs: liveFreshMs,
    usesPolling: orchUsesPolling,
  });
  const fresh = freshnessClasses(tone);
  const visible = resolveVisibleHudTiles(hudTilesConfig);

  // Active popover panel
  type PanelKind =
    | 'configure'
    | 'spend'
    | 'repo_version'
    | 'active_agents'
    | 'mesh_peers'
    | 'routing_decision'
    | 'attention_debt';

  const [openPanel, setOpenPanel] = useState<PanelKind | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const spendTriggerRef = useRef<HTMLButtonElement>(null);

  // VCS and App Version status
  const [repoStatus, setRepoStatus] = useState<VcsStatus>(FALLBACK_VCS_STATUS);

  const fetchRepoStatus = async () => {
    try {
      const w = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : undefined;
      const internals = w?.__TAURI_INTERNALS__ as { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> } | undefined;
      const tauriCore = (w?.__TAURI__ as { core?: { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> } } | undefined)?.core;
      const inv = internals?.invoke ?? tauriCore?.invoke;

      if (typeof inv === 'function') {
        const [vcsRaw, verRaw] = await Promise.all([
          inv('get_vcs_status').catch(() => null),
          inv('get_app_version').catch(() => null),
        ]);

        const vcs = vcsRaw as Record<string, unknown> | null;
        const ver = verRaw as Record<string, unknown> | string | null;

        const branch =
          typeof vcs?.branch === 'string'
            ? vcs.branch
            : typeof vcsRaw === 'string'
            ? vcsRaw
            : FALLBACK_VCS_STATUS.branch;

        const commit =
          typeof vcs?.commit === 'string'
            ? vcs.commit
            : typeof vcs?.short_sha === 'string'
            ? vcs.short_sha
            : typeof vcs?.sha === 'string'
            ? vcs.sha
            : FALLBACK_VCS_STATUS.commit;

        const dirtyCount =
          typeof vcs?.dirtyCount === 'number'
            ? vcs.dirtyCount
            : typeof vcs?.dirty_count === 'number'
            ? vcs.dirty_count
            : vcs?.dirty
            ? 1
            : FALLBACK_VCS_STATUS.dirtyCount;

        const version =
          typeof ver === 'string'
            ? ver
            : typeof (ver as Record<string, unknown>)?.version === 'string'
            ? ((ver as Record<string, unknown>).version as string)
            : FALLBACK_VCS_STATUS.version;

        setRepoStatus({ branch, commit, dirtyCount, version });
        return;
      }
    } catch {
      // Safe fallback
    }
    setRepoStatus(FALLBACK_VCS_STATUS);
  };

  useEffect(() => {
    void fetchRepoStatus();
    const interval = setInterval(() => {
      void fetchRepoStatus();
    }, 15_000);
    return () => clearInterval(interval);
  }, []);

  // Sync / normalize stored localStorage configs on mount
  useEffect(() => {
    try {
      if (typeof localStorage !== 'undefined') {
        const stored = localStorage.getItem('vox.hud.tiles.v2');
        if (stored) {
          const parsed = JSON.parse(stored);
          const normalized = normalizeHudTiles(parsed);
          const currentIds = hudTilesConfig.tiles.map((t) => t.id).join(',');
          const normalizedIds = normalized.tiles.map((t) => t.id).join(',');
          if (currentIds !== normalizedIds) {
            onHudTilesChange(normalized);
          }
        }
      }
    } catch {
      // Ignore localStorage parse errors
    }
  }, []);

  useEffect(() => {
    if (openPanel === null) return;
    const activeTrigger = openPanel === 'spend' ? spendTriggerRef : triggerRef;
    const onOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (panelRef.current?.contains(target)) return;
      if (activeTrigger.current?.contains(target)) return;
      setOpenPanel(null);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpenPanel(null);
        activeTrigger.current?.focus();
      }
    };
    document.addEventListener('mousedown', onOutside);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onOutside);
      document.removeEventListener('keydown', onKey);
    };
  }, [openPanel]);

  const persistConfig = (next: HudTilesConfig) => {
    onHudTilesChange(next);
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('vox.hud.tiles.v2', JSON.stringify(next));
      }
    } catch {
      // Ignore quota errors
    }
  };

  const handleToggleTile = (id: string, enabled: boolean) => {
    const next = toggleHudTile(hudTilesConfig, id, enabled);
    persistConfig(next);
  };

  const handleReorderTile = (from: number, to: number) => {
    const next = reorderHudTile(hudTilesConfig, from, to);
    persistConfig(next);
  };

  const handleResetDefaults = () => {
    const next = defaultHudTiles();
    persistConfig(next);
  };

  const budget = kpis.budgetBurn;
  // A cap only when the daemon reported a positive one: never `/ $0`, never the fallback placeholder.
  const spendValue = formatSpend(budget.value, budget.source === 'daemon' ? budget.cap : null);
  const usd = (v: number | null) => (v == null || Number.isNaN(v) ? 'unknown' : `$${v.toFixed(2)}`);
  const agentsN = kpis.activeAgents.value;
  const engineValue = `${agentsN} ${agentsN === 1 ? 'agent' : 'agents'} · ${kpis.queueDepth.value} queued`;
  // Mesh has one source (vox_mesh_nodes via useMeshNodes); until it answers, show a dash, not a second count.
  const meshValue =
    meshNodes == null
      ? '—'
      : `${meshNodes.filter((n) => n.status === 'online').length}/${meshNodes.length} online`;

  const routingProblems = routingHealthProblems(routingHealth);

  const renderSegment = (kind: HudTileKind): React.ReactNode => {
    const label = HUD_TILE_LABELS[kind];
    switch (kind) {
      case 'repo_version': {
        const shortSha = repoStatus.commit.slice(0, 7);
        const repoValue = `vox@${repoStatus.branch} · ${shortSha} · v${repoStatus.version}${
          repoStatus.dirtyCount > 0 ? ` · ${repoStatus.dirtyCount} dirty` : ''
        }`;
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-repo"
            hudTileId="hud-tile-repo_version"
            label={label}
            value={repoValue}
            expanded={openPanel === 'repo_version'}
            onClick={(e) => {
              if (e.shiftKey) {
                onNavigate('vox-git-view');
              } else {
                setOpenPanel((p) => (p === 'repo_version' ? null : 'repo_version'));
              }
            }}
            onDoubleClick={() => onNavigate('vox-git-view')}
            tooltip={{
              title: 'Repository & Engine Version',
              purpose: 'Tracks current branch, commit SHA, app version, and workspace cleanliness.',
              actionHint: 'Click for live controls or jump to surface',
              statusBadge: `v${repoStatus.version}`,
            }}
          />
        );
      }
      case 'active_agents':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-engine"
            hudTileId="hud-tile-active_agents"
            label={label}
            value={engineValue}
            expanded={openPanel === 'active_agents'}
            onClick={(e) => {
              if (e.shiftKey) {
                onNavigate('vox-operations-hub');
              } else {
                setOpenPanel((p) => (p === 'active_agents' ? null : 'active_agents'));
                onNavigate('agents');
              }
            }}
            onDoubleClick={() => onNavigate('vox-operations-hub')}
            tooltip={{
              title: 'Autonomous Execution Engine',
              purpose: 'Monitors autonomous orchestrator agents and pending queue depth.',
              actionHint: 'Click for live controls or jump to surface',
              statusBadge: fresh.label,
            }}
          />
        );
      case 'budget_burn':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-spend"
            hudTileId="hud-tile-budget_burn"
            label={label}
            value={spendValue}
            expanded={openPanel === 'spend'}
            buttonRef={spendTriggerRef}
            onClick={(e) => {
              if (e.shiftKey) {
                onNavigate('vox-cost-analytics');
              } else {
                setOpenPanel((p) => (p === 'spend' ? null : 'spend'));
              }
            }}
            onDoubleClick={() => onNavigate('vox-cost-analytics')}
            tooltip={{
              title: 'Spend & Budget Burn',
              purpose: 'Tracks token expenditure across engine, sessions, and OpenRouter.',
              actionHint: 'Click for live controls or jump to surface',
              statusBadge: spendValue,
            }}
          />
        );
      case 'mesh_peers':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-mesh"
            hudTileId="hud-tile-mesh_peers"
            label={label}
            value={meshValue}
            expanded={openPanel === 'mesh_peers'}
            onClick={(e) => {
              if (e.shiftKey) {
                onNavigate('vox-mesh-topology');
              } else {
                setOpenPanel((p) => (p === 'mesh_peers' ? null : 'mesh_peers'));
                onNavigate('mesh');
              }
            }}
            onDoubleClick={() => onNavigate('vox-mesh-topology')}
            tooltip={{
              title: 'Distributed Mesh Peers',
              purpose: 'Live node cluster topology and peer connectivity status.',
              actionHint: 'Click for live controls or jump to surface',
              statusBadge: meshValue,
            }}
          />
        );
      case 'routing_decision':
      case 'active_model':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-routing"
            hudTileId="hud-tile-routing_decision"
            label={label}
            value={routingCardValue(routingSummary)}
            ariaLabel="Open routing details"
            expanded={openPanel === 'routing_decision'}
            badge={
              routingProblems > 0 ? (
                <span
                  role="img"
                  data-testid="bottom-status-bar-routing-health"
                  aria-label={`Routing health: ${routingProblems} ${routingProblems === 1 ? 'problem' : 'problems'}`}
                  className="size-1.5 shrink-0 rounded-full"
                  style={{ background: 'var(--color-status-warn)' }}
                />
              ) : null
            }
            onClick={(e) => {
              if (e.shiftKey) {
                onNavigate('vox-model-router');
              } else {
                setOpenPanel((p) => (p === 'routing_decision' ? null : 'routing_decision'));
                onNavigate('models');
                window.setTimeout(() => document.getElementById('routing-panel')?.scrollIntoView?.({ block: 'start' }), 150);
              }
            }}
            onDoubleClick={() => onNavigate('vox-model-router')}
            tooltip={{
              title: 'Model Routing & Spoke Selection',
              purpose: 'Active model routing decision, dispatch policies, and MENS weights.',
              actionHint: 'Click for live controls or jump to surface',
              statusBadge: routingProblems > 0 ? `${routingProblems} issue(s)` : 'Nominal',
            }}
          />
        );
      case 'attention_debt':
      case 'pending_approvals':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-needs-you"
            hudTileId="hud-tile-attention_debt"
            label={label}
            value={
              needsYouDegraded.length > 0
                ? `${(needsYouCount ?? 0) > 0 ? needsYouCount : '—'} · couldn't load ${needsYouDegraded.join(', ')}`
                : String(needsYouCount ?? 0)
            }
            expanded={openPanel === 'attention_debt'}
            onClick={(e) => {
              if (e.shiftKey) {
                onNavigate('vox-approvals-center');
              } else {
                setOpenPanel((p) => (p === 'attention_debt' ? null : 'attention_debt'));
                onNavigate('needs-you');
              }
            }}
            onDoubleClick={() => onNavigate('vox-approvals-center')}
            tooltip={{
              title: 'Attention Debt & Approvals',
              purpose: 'Pending human-in-the-loop approvals, questions, and policy gates.',
              actionHint: 'Click for live controls or jump to surface',
              statusBadge: `${needsYouCount ?? 0} pending`,
            }}
          />
        );
      default:
        return null;
    }
  };

  return (
    <Glass
      data-testid="bottom-status-bar"
      role="status"
      aria-label="Operator status"
      style={{
        backgroundImage:
          'linear-gradient(90deg, rgba(248,243,233,0.96) 0%, rgba(238,229,211,0.94) 50%, rgba(248,243,233,0.96) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}
      className="flex h-7 w-full items-center gap-1 p-0 px-2.5 rounded-none border-x-0 border-b-0 shadow-[0_-2px_10px_rgba(0,0,0,0.35),inset_0_1px_0_rgba(255,255,255,0.92)] text-[10.5px] border-t-2 border-[#C59B27] text-[#1C140C]"
    >
      <div data-testid="status-bar" className="contents">
        <div className="relative flex min-w-0 flex-1">
          <div className="flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto no-scrollbar">
            {visible.map((kind) => renderSegment(kind))}
          </div>

          {/* Interactive Popover Cards */}
          {openPanel && openPanel !== 'configure' && (
            <div
              ref={panelRef}
              role="dialog"
              aria-label={
                openPanel === 'spend'
                  ? 'Spend detail'
                  : openPanel === 'repo_version'
                  ? 'Repository detail'
                  : openPanel === 'active_agents'
                  ? 'Engine detail'
                  : openPanel === 'mesh_peers'
                  ? 'Mesh detail'
                  : openPanel === 'routing_decision'
                  ? 'Routing detail'
                  : 'Attention detail'
              }
              data-testid={
                openPanel === 'spend'
                  ? 'bottom-status-bar-spend-popover'
                  : `bottom-status-bar-${openPanel}-popover`
              }
              className="absolute bottom-7 left-2 z-50 mb-1 w-80 rounded-lg border-2 border-[#D4AF37]/65 bg-linear-to-b from-[#1E1912] to-[#0E0C08] p-3 text-[#F3E5AB] shadow-2xl"
            >
              {/* Spend Popover */}
              {openPanel === 'spend' && (
                <div data-testid="hud-spend-popover">
                  <div className="flex items-center justify-between border-b border-[#D4AF37]/30 pb-1.5 mb-2">
                    <span className="font-display text-[10px] uppercase tracking-[0.12em] text-[#D4AF37] font-semibold">
                      Spend Telemetry
                    </span>
                    <span className="font-mono text-[10px] text-[#F3E5AB]">{spendValue}</span>
                  </div>
                  <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
                    <dt className="text-[#C8B99E]">Engine total</dt>
                    <dd className="font-mono tabular-nums text-[#F3E5AB]">{spendValue}</dd>
                    <dt className="text-[#C8B99E]">OpenRouter</dt>
                    <dd className="font-mono tabular-nums text-[#F3E5AB]">{usd(openrouterSpendUsd)}</dd>
                    <dt className="text-[#C8B99E]">This session</dt>
                    <dd className="font-mono tabular-nums text-[#F3E5AB]">{usd(sessionSpentUsd)}</dd>
                    <dt className="text-[#C8B99E]">Local models</dt>
                    <dd className="font-mono tabular-nums text-[#F3E5AB]">not metered</dd>
                  </dl>
                  <div className="mt-3 flex flex-col gap-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('settings');
                      }}
                      className="w-full rounded border border-[#D4AF37]/40 bg-[#1D1811] px-2 py-1 text-[11px] text-[#F3E5AB] hover:border-[#D4AF37] hover:bg-[#282117] transition"
                    >
                      Budget settings
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-cost-analytics');
                      }}
                      className="w-full rounded border border-[#D4AF37]/50 bg-linear-to-r from-[#2A2114] to-[#1E180E] px-2 py-1 text-[11px] font-semibold text-[#D4AF37] hover:border-[#F3E5AB] hover:text-[#FFF8DC] transition inline-flex items-center justify-center gap-1"
                    >
                      Open Cost Analytics ↗
                    </button>
                  </div>
                </div>
              )}

              {/* Repo Version Popover */}
              {openPanel === 'repo_version' && (
                <div>
                  <div className="flex items-center justify-between border-b border-[#D4AF37]/30 pb-1.5 mb-2">
                    <span className="font-display text-[10px] uppercase tracking-[0.12em] text-[#D4AF37] font-semibold">
                      Repository & Local Version
                    </span>
                    <span className="rounded border border-[#D4AF37]/40 bg-[#D4AF37]/15 px-1.5 py-0.5 font-mono text-[9px] text-[#F3E5AB]">
                      v{repoStatus.version}
                    </span>
                  </div>
                  <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
                    <dt className="text-[#C8B99E]">Branch</dt>
                    <dd className="font-mono text-[#F3E5AB]">{repoStatus.branch}</dd>
                    <dt className="text-[#C8B99E]">Commit SHA</dt>
                    <dd className="font-mono text-[#F3E5AB]">{repoStatus.commit}</dd>
                    <dt className="text-[#C8B99E]">App Version</dt>
                    <dd className="font-mono text-[#F3E5AB]">v{repoStatus.version}</dd>
                    <dt className="text-[#C8B99E]">Working Tree</dt>
                    <dd className="font-mono text-[#F3E5AB]">
                      {repoStatus.dirtyCount > 0 ? `${repoStatus.dirtyCount} dirty file(s)` : 'Clean'}
                    </dd>
                  </dl>
                  <div className="mt-3 flex flex-col gap-1.5">
                    <button
                      type="button"
                      onClick={() => void fetchRepoStatus()}
                      className="w-full rounded border border-[#D4AF37]/40 bg-[#1D1811] px-2 py-1 text-[11px] text-[#F3E5AB] hover:border-[#D4AF37] hover:bg-[#282117] transition"
                    >
                      Refresh Repo Status
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-git-view');
                      }}
                      className="w-full rounded border border-[#D4AF37]/50 bg-linear-to-r from-[#2A2114] to-[#1E180E] px-2 py-1 text-[11px] font-semibold text-[#D4AF37] hover:border-[#F3E5AB] hover:text-[#FFF8DC] transition inline-flex items-center justify-center gap-1"
                    >
                      Open Repository Surface ↗
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-mens-training');
                      }}
                      className="w-full rounded border border-[#D4AF37]/30 bg-[#14100B] px-2 py-1 text-[11px] text-[#C8B99E] hover:border-[#D4AF37] hover:text-[#F3E5AB] transition inline-flex items-center justify-center gap-1"
                    >
                      Open MENS Ladder ↗
                    </button>
                  </div>
                </div>
              )}

              {/* Engine (Active Agents) Popover */}
              {openPanel === 'active_agents' && (
                <div>
                  <div className="flex items-center justify-between border-b border-[#D4AF37]/30 pb-1.5 mb-2">
                    <span className="font-display text-[10px] uppercase tracking-[0.12em] text-[#D4AF37] font-semibold">
                      Engine Telemetry
                    </span>
                    <span className="font-mono text-[10px] text-[#F3E5AB]">{fresh.label}</span>
                  </div>
                  <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
                    <dt className="text-[#C8B99E]">Active Agents</dt>
                    <dd className="font-mono tabular-nums text-[#F3E5AB]">{agentsN}</dd>
                    <dt className="text-[#C8B99E]">Queued Tasks</dt>
                    <dd className="font-mono tabular-nums text-[#F3E5AB]">{kpis.queueDepth.value}</dd>
                    <dt className="text-[#C8B99E]">Freshness Status</dt>
                    <dd className="font-mono text-[#F3E5AB]">{fresh.label}</dd>
                  </dl>
                  <div className="mt-3 flex flex-col gap-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-operations-hub');
                      }}
                      className="w-full rounded border border-[#D4AF37]/50 bg-linear-to-r from-[#2A2114] to-[#1E180E] px-2 py-1 text-[11px] font-semibold text-[#D4AF37] hover:border-[#F3E5AB] hover:text-[#FFF8DC] transition inline-flex items-center justify-center gap-1"
                    >
                      Open Operations Hub ↗
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-task-list');
                      }}
                      className="w-full rounded border border-[#D4AF37]/30 bg-[#14100B] px-2 py-1 text-[11px] text-[#C8B99E] hover:border-[#D4AF37] hover:text-[#F3E5AB] transition inline-flex items-center justify-center gap-1"
                    >
                      Open Task Queue ↗
                    </button>
                  </div>
                </div>
              )}

              {/* Mesh Popover */}
              {openPanel === 'mesh_peers' && (
                <div>
                  <div className="flex items-center justify-between border-b border-[#D4AF37]/30 pb-1.5 mb-2">
                    <span className="font-display text-[10px] uppercase tracking-[0.12em] text-[#D4AF37] font-semibold">
                      Mesh Topology
                    </span>
                    <span className="font-mono text-[10px] text-[#F3E5AB]">{meshValue}</span>
                  </div>
                  <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
                    <dt className="text-[#C8B99E]">Peers Status</dt>
                    <dd className="font-mono text-[#F3E5AB]">{meshValue}</dd>
                    <dt className="text-[#C8B99E]">Mesh Topology</dt>
                    <dd className="font-mono text-[#F3E5AB]">
                      {meshNodes ? `${meshNodes.length} nodes registered` : 'Nominal'}
                    </dd>
                  </dl>
                  <div className="mt-3 flex flex-col gap-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-mesh-topology');
                      }}
                      className="w-full rounded border border-[#D4AF37]/50 bg-linear-to-r from-[#2A2114] to-[#1E180E] px-2 py-1 text-[11px] font-semibold text-[#D4AF37] hover:border-[#F3E5AB] hover:text-[#FFF8DC] transition inline-flex items-center justify-center gap-1"
                    >
                      Open Mesh Topology ↗
                    </button>
                  </div>
                </div>
              )}

              {/* Routing Popover */}
              {openPanel === 'routing_decision' && (
                <div>
                  <div className="flex items-center justify-between border-b border-[#D4AF37]/30 pb-1.5 mb-2">
                    <span className="font-display text-[10px] uppercase tracking-[0.12em] text-[#D4AF37] font-semibold">
                      Routing & Spoke Selection
                    </span>
                    <span className="font-mono text-[10px] text-[#F3E5AB]">
                      {routingProblems > 0 ? `${routingProblems} issue(s)` : 'Nominal'}
                    </span>
                  </div>
                  <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
                    <dt className="text-[#C8B99E]">Active Route</dt>
                    <dd className="font-mono text-[#F3E5AB] truncate max-w-[20ch]">
                      {routingCardValue(routingSummary)}
                    </dd>
                    <dt className="text-[#C8B99E]">Spoke Selection</dt>
                    <dd className="font-mono text-[#F3E5AB] truncate max-w-[20ch]">
                      {routingSummary?.reason ?? 'Autonomous multi-tier router'}
                    </dd>
                    <dt className="text-[#C8B99E]">Route Health</dt>
                    <dd className="font-mono text-[#F3E5AB]">
                      {routingProblems > 0 ? `${routingProblems} problem(s)` : 'Nominal'}
                    </dd>
                  </dl>
                  <div className="mt-3 flex flex-col gap-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-model-router');
                      }}
                      className="w-full rounded border border-[#D4AF37]/50 bg-linear-to-r from-[#2A2114] to-[#1E180E] px-2 py-1 text-[11px] font-semibold text-[#D4AF37] hover:border-[#F3E5AB] hover:text-[#FFF8DC] transition inline-flex items-center justify-center gap-1"
                    >
                      Open Model Router ↗
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-mens-training');
                      }}
                      className="w-full rounded border border-[#D4AF37]/30 bg-[#14100B] px-2 py-1 text-[11px] text-[#C8B99E] hover:border-[#D4AF37] hover:text-[#F3E5AB] transition inline-flex items-center justify-center gap-1"
                    >
                      Open MENS Studio ↗
                    </button>
                  </div>
                </div>
              )}

              {/* Attention Debt Popover */}
              {openPanel === 'attention_debt' && (
                <div>
                  <div className="flex items-center justify-between border-b border-[#D4AF37]/30 pb-1.5 mb-2">
                    <span className="font-display text-[10px] uppercase tracking-[0.12em] text-[#D4AF37] font-semibold">
                      Attention Debt & Policy Gates
                    </span>
                    <span className="font-mono text-[10px] text-[#F3E5AB]">
                      {needsYouCount ?? 0} pending
                    </span>
                  </div>
                  <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
                    <dt className="text-[#C8B99E]">Pending Approvals</dt>
                    <dd className="font-mono text-[#F3E5AB]">{needsYouCount ?? 0}</dd>
                    <dt className="text-[#C8B99E]">Verification Gates</dt>
                    <dd className="font-mono text-[#F3E5AB]">
                      {needsYouDegraded.length > 0 ? `Degraded (${needsYouDegraded.join(', ')})` : 'Nominal'}
                    </dd>
                  </dl>
                  <div className="mt-3 flex flex-col gap-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        setOpenPanel(null);
                        onNavigate('vox-approvals-center');
                      }}
                      className="w-full rounded border border-[#D4AF37]/50 bg-linear-to-r from-[#2A2114] to-[#1E180E] px-2 py-1 text-[11px] font-semibold text-[#D4AF37] hover:border-[#F3E5AB] hover:text-[#FFF8DC] transition inline-flex items-center justify-center gap-1"
                    >
                      Open Review & Approvals ↗
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {gamifyEnabled && onOpenAchievements && (
          <button
            type="button"
            data-testid="achievements-trigger"
            aria-label="Open achievements"
            onClick={onOpenAchievements}
            className="inline-flex shrink-0 items-center justify-center rounded px-1.5 py-0.5 text-brass hover:bg-overlay-subtle hover:text-[#F3E5AB] transition"
          >
            <Icon.trophy className="size-3.5" aria-hidden="true" />
          </button>
        )}

        <StatusBarCluster onOpenDrawer={onOpenResearchDrawer} />

        {/* Configure Status Bar Dropdown */}
        <div className="relative shrink-0">
          <button
            ref={triggerRef}
            type="button"
            data-testid="hud-configure-trigger"
            onClick={() => setOpenPanel((p) => (p === 'configure' ? null : 'configure'))}
            aria-expanded={openPanel === 'configure'}
            aria-label="Configure status bar"
            className="rounded border border-[#8C6825] bg-[#251C0E] px-2 py-0.5 text-[10px] font-mono text-[#F3E5AB] hover:border-[#D4AF37] hover:bg-[#382B14] hover:text-white transition"
          >
            <span className="max-[639px]:sr-only">Configure </span>▾
          </button>
          {openPanel === 'configure' ? (
            <div
              ref={panelRef}
              data-testid="hud-configure-popover"
              className="absolute bottom-full right-0 z-50 mb-1 w-64 rounded-lg border-2 border-[#D4AF37]/65 bg-linear-to-b from-[#1E1912] to-[#0E0C08] p-3 text-[#F3E5AB] shadow-2xl"
            >
              <div className="flex items-center justify-between border-b border-[#D4AF37]/30 pb-1.5 mb-2">
                <span className="font-display text-[10px] uppercase tracking-[0.12em] text-[#D4AF37] font-semibold">
                  Configure Status Bar
                </span>
                <button
                  type="button"
                  onClick={handleResetDefaults}
                  className="text-[9px] font-mono text-[#C8B99E] hover:text-[#F3E5AB] underline"
                >
                  Reset Defaults
                </button>
              </div>
              <div className="flex flex-col gap-1 max-h-60 overflow-y-auto">
                {hudTilesConfig.tiles.map((tile, idx) => (
                  <div
                    key={tile.id}
                    className="flex items-center justify-between gap-1 rounded px-1.5 py-1 text-[11px] text-[#F3E5AB] hover:bg-[#282117]/60"
                  >
                    <label className="flex items-center gap-2 cursor-pointer select-none">
                      <input
                        type="checkbox"
                        checked={tile.enabled}
                        onChange={(e) => handleToggleTile(tile.id, e.target.checked)}
                        className="accent-[#D4AF37] rounded border-[#D4AF37]/40 bg-[#0E0C08] text-[#D4AF37] focus:ring-[#D4AF37]/40 size-3.5"
                      />
                      <span className="font-medium">{HUD_TILE_LABELS[tile.kind] ?? tile.kind}</span>
                    </label>
                    <div className="flex items-center gap-0.5">
                      <button
                        type="button"
                        aria-label={`Move ${HUD_TILE_LABELS[tile.kind] ?? tile.kind} up`}
                        disabled={idx === 0}
                        onClick={() => handleReorderTile(idx, idx - 1)}
                        className="px-1 text-[10px] text-[#C8B99E] hover:text-[#F3E5AB] disabled:opacity-30 disabled:hover:text-[#C8B99E]"
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        aria-label={`Move ${HUD_TILE_LABELS[tile.kind] ?? tile.kind} down`}
                        disabled={idx === hudTilesConfig.tiles.length - 1}
                        onClick={() => handleReorderTile(idx, idx + 1)}
                        className="px-1 text-[10px] text-[#C8B99E] hover:text-[#F3E5AB] disabled:opacity-30 disabled:hover:text-[#C8B99E]"
                      >
                        ↓
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : null}
        </div>

        {/* Engine Freshness Pill */}
        <div
          data-testid="bottom-status-bar-freshness"
          title={fresh.title}
          className={`ml-auto inline-flex shrink-0 items-center gap-1.5 rounded border px-2 py-0.5 text-[10px] font-mono ${fresh.pill}`}
        >
          <span data-testid="hud-Connection-badge" className={`size-1.5 rounded-full ${fresh.dot}`} />
          <span className="uppercase tracking-[0.08em] max-[639px]:sr-only">{fresh.label}</span>
        </div>

        {/* Trailing slot portal anchor */}
        <div
          id={WORKBENCH_TABBAR_TRAILING_SLOT_ID}
          data-testid={WORKBENCH_TABBAR_TRAILING_SLOT_ID}
          className="ml-2 flex shrink-0 items-center"
        />

        {notices && onMarkAllNoticesRead ? (
          <NotificationCenter notices={notices} onMarkAllRead={onMarkAllNoticesRead} />
        ) : null}
      </div>
    </Glass>
  );
}
