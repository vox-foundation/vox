import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../ui/Glass';
import { Icon } from '../ui/Icons';
import { formatSpend } from '../../config/budget';
import { useFreshness } from '../../hooks/useFreshness';
import {
  resolveVisibleHudTiles,
  toggleHudTile,
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

function freshnessClasses(tone: 'live' | 'poll' | 'stale') {
  if (tone === 'live') {
    return {
      pill: 'border-emerald-400/20 bg-emerald-400/4 text-emerald-300',
      dot: 'bg-emerald-400',
      label: 'Live',
      title: 'Live: receiving engine events',
    };
  }
  if (tone === 'poll') {
    return {
      pill: 'border-amber-400/20 bg-amber-400/4 text-amber-300',
      dot: 'bg-amber-400',
      label: 'Poll',
      title: 'Polling: no event stream, refreshing on a timer',
    };
  }
  return {
    pill: 'border-border-subtle bg-overlay-subtle text-text-muted',
    dot: 'bg-text-muted',
    label: 'Offline',
    title: 'Offline: no engine data recently',
  };
}

function Segment({
  testId,
  label,
  value,
  onClick,
  expanded,
  buttonRef,
  ariaLabel,
  badge,
}: {
  testId: string;
  label: string;
  value: string;
  onClick: () => void;
  /** Overrides the accessible name (the visible label and value stay as they are). */
  ariaLabel?: string;
  badge?: React.ReactNode;
  /** Set only on a card that opens a popover. */
  expanded?: boolean;
  buttonRef?: React.Ref<HTMLButtonElement>;
}) {
  return (
    <button
      ref={buttonRef}
      type="button"
      data-testid={testId}
      onClick={onClick}
      aria-label={ariaLabel}
      aria-haspopup={expanded === undefined ? undefined : 'dialog'}
      aria-expanded={expanded}
      className="inline-flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-[10px] text-text-muted hover:bg-overlay-subtle hover:text-text-secondary transition"
    >
      <span data-card-label className="uppercase tracking-[0.14em] text-text-muted">{label}</span>
      <span
        data-card-value
        title={value}
        className="inline-block max-w-[28ch] truncate align-bottom font-mono tabular-nums text-text-secondary"
      >
        {value}
      </span>
      {badge}
    </button>
  );
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

  // One popover at a time: the Configure menu or the Spend card's detail.
  const [openPanel, setOpenPanel] = useState<'configure' | 'spend' | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const spendTriggerRef = useRef<HTMLButtonElement>(null);

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
      case 'active_agents':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-engine"
            label={label}
            value={engineValue}
            onClick={() => onNavigate('agents')}
          />
        );
      case 'budget_burn':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-spend"
            label={label}
            value={spendValue}
            expanded={openPanel === 'spend'}
            buttonRef={spendTriggerRef}
            onClick={() => setOpenPanel((p) => (p === 'spend' ? null : 'spend'))}
          />
        );
      case 'mesh_peers':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-mesh"
            label={label}
            value={meshValue}
            onClick={() => onNavigate('mesh')}
          />
        );
      case 'active_model':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-routing"
            label={label}
            value={routingCardValue(routingSummary)}
            ariaLabel="Open routing details"
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
            onClick={() => {
              onNavigate('models');
              // ponytail: one timed retry for a surface that mounts after navigation; a ref handshake if it ever misses.
              window.setTimeout(() => document.getElementById('routing-panel')?.scrollIntoView?.({ block: 'start' }), 150);
            }}
          />
        );
      case 'pending_approvals':
        return (
          <Segment
            key={kind}
            testId="bottom-status-bar-needs-you"
            label={label}
            value={
              needsYouDegraded.length > 0
                ? `${(needsYouCount ?? 0) > 0 ? needsYouCount : '—'} · couldn't load ${needsYouDegraded.join(', ')}`
                : String(needsYouCount ?? 0)
            }
            onClick={() => onNavigate('needs-you')}
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
      className="flex h-7 w-full items-center gap-1 p-0 px-3 rounded-none border-x-0 border-b-0 shadow-none text-[10px] text-text-muted"
    >
      <div className="relative flex min-w-0 flex-1">
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
          {visible.map((kind) => renderSegment(kind))}
        </div>
        {openPanel === 'spend' ? (
          <div
            ref={panelRef}
            role="dialog"
            aria-label="Spend detail"
            data-testid="bottom-status-bar-spend-popover"
            className="absolute bottom-full left-0 z-50 mb-1 w-64 rounded-lg border border-border-subtle bg-bg-base p-3 shadow-2xl"
          >
            <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 text-[11px]">
              <dt className="text-text-muted">Engine total</dt>
              <dd className="font-mono tabular-nums text-text-secondary">{spendValue}</dd>
              <dt className="text-text-muted">OpenRouter</dt>
              <dd className="font-mono tabular-nums text-text-secondary">{usd(openrouterSpendUsd)}</dd>
              <dt className="text-text-muted">This session</dt>
              <dd className="font-mono tabular-nums text-text-secondary">{usd(sessionSpentUsd)}</dd>
              <dt className="text-text-muted">Local models</dt>
              <dd className="font-mono tabular-nums text-text-secondary">not metered</dd>
            </dl>
            <button
              type="button"
              onClick={() => {
                setOpenPanel(null);
                onNavigate('settings');
              }}
              className="mt-2 w-full rounded-sm border border-border-subtle px-2 py-1 text-[11px] text-text-secondary hover:bg-overlay-subtle"
            >
              Budget settings
            </button>
          </div>
        ) : null}
      </div>
      {gamifyEnabled && onOpenAchievements && (
        <button
          type="button"
          data-testid="achievements-trigger"
          aria-label="Open achievements"
          onClick={onOpenAchievements}
          className="inline-flex shrink-0 items-center justify-center rounded-sm px-1.5 py-0.5 text-amber-300/80 hover:bg-overlay-subtle hover:text-amber-200 transition"
        >
          <Icon.trophy className="size-3.5" aria-hidden="true" />
        </button>
      )}
      <StatusBarCluster onOpenDrawer={onOpenResearchDrawer} />
      <div className="relative shrink-0">
        <button
          ref={triggerRef}
          type="button"
          onClick={() => setOpenPanel((p) => (p === 'configure' ? null : 'configure'))}
          aria-expanded={openPanel === 'configure'}
          aria-label="Configure status bar"
          className="rounded-sm px-1.5 py-0.5 text-[10px] text-text-muted hover:bg-overlay-subtle hover:text-text-secondary transition"
        >
          Configure ▾
        </button>
        {openPanel === 'configure' ? (
          <div
            ref={panelRef}
            className="absolute bottom-full right-0 z-50 mb-1 w-56 rounded-lg border border-border-subtle bg-bg-base p-2 shadow-2xl"
          >
            {hudTilesConfig.tiles.map((tile) => (
              <label
                key={tile.id}
                className="flex items-center gap-2 rounded-sm px-2 py-1 text-[11px] text-text-secondary hover:bg-overlay-subtle"
              >
                <input
                  type="checkbox"
                  checked={tile.enabled}
                  onChange={(e) =>
                    onHudTilesChange(toggleHudTile(hudTilesConfig, tile.id, e.target.checked))
                  }
                  className="rounded-sm border-border-subtle bg-bg-base text-brass focus:ring-brass/40 focus:ring-offset-bg-base size-3.5"
                />
                {HUD_TILE_LABELS[tile.kind]}
              </label>
            ))}
          </div>
        ) : null}
      </div>
      <div
        data-testid="bottom-status-bar-freshness"
        title={fresh.title}
        className={`ml-auto inline-flex shrink-0 items-center gap-1.5 rounded-sm border px-2 py-0.5 ${fresh.pill}`}
      >
        <span className={`size-1.5 rounded-full ${fresh.dot}`} />
        <span className="uppercase tracking-[0.14em]">{fresh.label}</span>
      </div>

      {/* Fixed home for surface-level chrome that needs to sit inline with
          persistent app chrome rather than in the per-surface content area
          (e.g. Chat's "Panels ▾" dock-visibility menu, portaled in here from
          ChatSurface). BottomStatusBar is a single, non-wrapping row rendered
          once in the app shell's footer — unlike WorkbenchTabBar's tablist,
          it never grows to multiple lines as more tabs open, so anything
          docked here stays put and reachable regardless of how many
          top-level tabs are open or how the tab bar wraps. It's also
          independent of the tab bar's own lifecycle: the tab bar is slated
          for eventual removal, this slot is not. `shrink-0` (and the KPI
          segments' own scroll region above) keeps it pinned at the right
          edge even when the window itself is too narrow to fit everything. */}
      <div
        id={WORKBENCH_TABBAR_TRAILING_SLOT_ID}
        data-testid={WORKBENCH_TABBAR_TRAILING_SLOT_ID}
        className="ml-2 flex shrink-0 items-center"
      />
      {notices && onMarkAllNoticesRead ? (
        <NotificationCenter notices={notices} onMarkAllRead={onMarkAllNoticesRead} />
      ) : null}
    </Glass>
  );
}
