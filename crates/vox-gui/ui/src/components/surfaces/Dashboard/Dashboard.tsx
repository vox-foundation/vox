import React, { useState, useEffect, useRef } from 'react';
import { LudusSandbox } from '../../gamify/LudusSandbox';
import { Glass } from '../../ui/Glass';
import { Icon } from '../../ui/Icons';
import { StreamCard } from './StreamCard';
import { AgentRow } from './AgentRow';
import { LudusBanner } from './LudusBanner';
import { DashboardData, Agent, StreamItem, LudusAlert } from '../../../types/dashboard';
import { Skeleton } from '../../ui/Skeleton';
import { defaultDashboardLayout, type DashboardWidget, addWidgetToLayout, resetDashboardLayout } from '../../../lib/dashboardLayout';
import { DashboardGrid, persistDashboardLayout } from '../../dashboard/DashboardGrid';
import { WidgetPickerDrawer } from '../../dashboard/WidgetPickerDrawer';
import { useLocalStorage } from '../../../hooks/useLocalStorage';
import { labelFor } from '../../../lib/lexicon';
import { useLang } from '../../../hooks/useLanguage';
import { SHELL_PREFERENCE_KEYS } from '../../../lib/shellPersistence';
import { loadDashboardLayout } from '../../dashboard/DashboardGrid';
import {
  useMetricSeries,
  metricSeriesFromSpark,
  shouldAppendMetricFromKpi,
} from '../../../hooks/useMetricSeries';
import { AreaChartWidget } from '../../dashboard/widgets/AreaChartWidget';
import { LineChartWidget } from '../../dashboard/widgets/LineChartWidget';
import { BarChartWidget } from '../../dashboard/widgets/BarChartWidget';
import { Kpi } from '../../ui/Kpi';
import { AttentionBudgetMeter } from '../AttentionBudgetMeter';
import type { AttentionBudgetSnapshot } from '../../../types/tauri';
import { ResourcesWidget } from './ResourcesWidget';
import { useAgentApprovals } from '../../../hooks/useAgentApprovals';
import { surfaceKeyOf } from '../../../lib/dashboardLayout';
import { resolveWidget } from '../../dashboard/dashboardWidgetRegistry';
import { SurfaceMiniRender } from '../../dashboard/SurfaceMiniRender';
import { EmbeddedSurfaceProvider } from '../../dashboard/EmbeddedSurfaceContext';
import { WidgetErrorBoundary } from '../../dashboard/WidgetErrorBoundary';
import { childRenderer, type SurfaceProps } from '../../layout/surfaceComponents';
import type { Toast } from '../../../types/tauri';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';

/** Consistent empty-state hint for a panel with no data yet. */
function EmptyHint({ icon, title, hint }: { icon?: React.ReactNode; title: string; hint?: string }) {
  return (
    <div role="status" className="flex flex-col items-center justify-center gap-1.5 rounded-lg border border-dashed border-border-subtle py-8 text-center">
      {icon && <div className="text-text-muted">{icon}</div>}
      <div className="text-[12px] text-text-muted">{title}</div>
      {hint && <div className="text-[11px] text-text-muted">{hint}</div>}
    </div>
  );
}

interface DashboardProps {
  data: DashboardData;
  loading?: boolean;
  onPause: (a: Agent) => void;
  onResume: (a: Agent) => void;
  onDoubt?: (item: StreamItem) => void;
  onOverrule?: (item: StreamItem) => void;
  onAckLudus: (note: LudusAlert) => void;
  filterKind: string;
  setFilterKind: (k: string) => void;
  onOpenInConsole?: (a: Agent) => void;
  onOpenChat?: () => void;
  onNavigate?: (viewKey: string) => void;
  attention_budget?: AttentionBudgetSnapshot | null;
  /** Real toast handler, threaded into mini-rendered surfaces (which call pushToast). */
  pushToast?: (toast: Toast) => void;
}

export function Dashboard({
  data,
  loading = false,
  onPause,
  onResume,
  onDoubt,
  onOverrule,
  onAckLudus,
  filterKind,
  setFilterKind,
  onOpenInConsole,
  onOpenChat,
  onNavigate,
  attention_budget,
  pushToast,
}: DashboardProps) {
  const { lang } = useLang();
  const filters = ["all", "validated", "in-progress", "doubted", "speculative"];
  const stream = data.stream.filter(s => filterKind === "all" ? true : s.kind === filterKind);
  const [customizeMode, setCustomizeMode] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [sandboxCollapsed, setSandboxCollapsed] = useState(false);
  const [activeBottomTab, setActiveBottomTab] = useState<'agents' | 'budget' | 'resources'>('agents');
  const [layout, setLayout] = useLocalStorage(
    SHELL_PREFERENCE_KEYS.dashboardLayout,
    loadDashboardLayout(defaultDashboardLayout()),
  );
  const approvals = useAgentApprovals(data.agents.map((a) => a.codename));
  const { series: budgetSeries, setSeries: setBudgetSeries, append: appendBudget } =
    useMetricSeries('budget_burn', []);
  const { series: queueSeries, setSeries: setQueueSeries, append: appendQueue } =
    useMetricSeries('queue_depth', []);
  const prevBudgetValue = useRef<number | undefined>(undefined);
  const prevQueueValue = useRef<number | undefined>(undefined);

  const budgetSpark = data.kpis.budgetBurn.spark;
  const budgetValue =
    typeof data.kpis.budgetBurn.value === 'number' ? data.kpis.budgetBurn.value : 0;
  const queueSpark = data.kpis.queueDepth.spark;
  const queueValue = data.kpis.queueDepth.value;

  useEffect(() => {
    if (budgetSpark.length > 0) {
      setBudgetSeries(metricSeriesFromSpark(budgetSpark));
      prevBudgetValue.current = budgetValue;
    }
  }, [budgetSpark, budgetValue, setBudgetSeries]);

  useEffect(() => {
    if (shouldAppendMetricFromKpi(budgetSpark, budgetValue, prevBudgetValue.current)) {
      prevBudgetValue.current = budgetValue;
      appendBudget(budgetValue);
    }
  }, [budgetSpark, budgetValue, appendBudget]);

  useEffect(() => {
    if (queueSpark.length > 0) {
      setQueueSeries(metricSeriesFromSpark(queueSpark));
      prevQueueValue.current = queueValue;
    }
  }, [queueSpark, queueValue, setQueueSeries]);

  useEffect(() => {
    if (shouldAppendMetricFromKpi(queueSpark, queueValue, prevQueueValue.current)) {
      prevQueueValue.current = queueValue;
      appendQueue(queueValue);
    }
  }, [queueSpark, queueValue, appendQueue]);

  function updateLayout(next: ReturnType<typeof defaultDashboardLayout>) {
    setLayout(next);
    persistDashboardLayout(next);
  }

  function handleAddWidget(kind: Parameters<typeof addWidgetToLayout>[1]) {
    updateLayout(addWidgetToLayout(layout, kind));
    setPickerOpen(false);
  }

  function handleAddSurface(surfaceKey: string) {
    const next = addWidgetToLayout(layout, 'surface_widget');
    const placed = {
      ...next,
      widgets: next.widgets.map((w, i) =>
        i === next.widgets.length - 1 ? { ...w, config: { ...(w.config ?? {}), surfaceKey } } : w,
      ),
    };
    updateLayout(placed);
    setPickerOpen(false);
  }

  function handleResetLayout() {
    updateLayout(resetDashboardLayout());
    setPickerOpen(false);
  }

  function chartTitle(widget: DashboardWidget, fallback: string): string {
    const configured = widget.config?.title;
    return typeof configured === 'string' && configured.trim() !== '' ? configured : fallback;
  }

  function surfaceLabel(surfaceKey: string): string {
    // synthetic 'cost' has no registry row → give it a real label, not the raw key
    if (surfaceKey === 'cost') return 'OpenRouter Spend';
    return labelFor(surfaceKey, lang);
  }

  // Build the SurfaceProps bag the mini-render mounts real surfaces with. Closes
  // over the Dashboard's destructured props; pushToast must be a real function
  // (mini-rendered surfaces call it during render/effects), so fall back to a
  // no-op handler when the Dashboard was not given one (honest: no fabricated data).
  // Props for an embedded mini-render. The action callbacks are deliberately
  // INERT no-ops: a thumbnail must never pause/resume an agent, doubt/overrule a
  // task, ack a Ludus note, or push a toast. (The mini still issues read-only
  // mount polls — see SurfaceMiniRender's docstring.) Click-through to the live
  // surface, where the real callbacks apply, is the parent's job (onOpen).
  const noop = () => {};
  function miniPropsFor(): SurfaceProps {
    return {
      data,
      pushToast: noop,
      onPause: noop,
      onResume: noop,
      onDoubt: noop,
      onOverrule: noop,
      onAckLudus: noop,
      filterKind,
      setFilterKind,
      onOpenInConsole: noop,
      onNavigate,
      attention_budget,
    } as SurfaceProps;
  }

  function renderSurfaceWidget(surfaceKey: string): React.ReactNode {
    const resolved = resolveWidget(surfaceKey);
    if (resolved.kind === 'purpose-built') {
      return <resolved.Component data={data} />;
    }
    return (
      <SurfaceMiniRender surfaceKey={surfaceKey} label={surfaceLabel(surfaceKey)}>
        {childRenderer(miniPropsFor(), surfaceKey)}
      </SurfaceMiniRender>
    );
  }

  function renderWidget(widget: DashboardWidget) {
    const isAgentsTab = activeBottomTab === 'agents' || customizeMode;
    const isResourcesTab = activeBottomTab === 'resources' || customizeMode;

    switch (widget.kind) {
      case 'stream':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <Glass className="h-full p-5">
              <div className="flex items-center justify-between">
                <div>
                  <div className="flex items-center gap-3">
                    <h2 className="font-display text-[18px] font-semibold tracking-tight text-text-primary">The Stream</h2>
                    <span className="rounded-full border border-border-subtle bg-overlay-subtle px-2 py-0.5 font-mono text-[10px] text-text-muted">{stream.length} events</span>
                  </div>
                  <p className="mt-0.5 text-[11px] text-text-muted">Mission-control feed · live agent telemetry</p>
                </div>
                <div className="flex gap-1 rounded-lg border border-border-subtle bg-overlay-subtle p-1">
                  {filters.map(f => (
                    <button
                      key={f}
                      type="button"
                      aria-pressed={filterKind === f}
                      onClick={() => setFilterKind(f)}
                      className={`rounded-md px-2.5 py-1 text-[10px] font-display uppercase tracking-wider transition ${
                        filterKind === f ? "bg-overlay-subtle text-text-primary" : "text-text-muted hover:text-text-secondary"
                      }`}
                    >
                      {f === "in-progress" ? "in-prog" : f}
                    </button>
                  ))}
                </div>
              </div>
              <div className="mt-4 flex flex-col gap-2.5">
                {stream.length === 0 ? (
                  <EmptyHint
                    icon={<Icon.flow className="size-5" />}
                    title={filterKind === 'all' ? 'No events yet' : `No ${filterKind} events`}
                    hint="Live agent telemetry streams here once tasks run. Open Chat to submit a task."
                  />
                ) : (
                  stream.map(s => <StreamCard key={s.id} item={s} onDoubt={onDoubt} onOverrule={onOverrule} />)
                )}
              </div>
            </Glass>
          </div>
        );
      case 'alerts':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <Glass className="h-full p-5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Icon.alert className="size-4 text-amber-300" />
                  <h2 className="font-display text-[14px] font-semibold tracking-wide text-text-primary">System · Telemetry & Alerts</h2>
                </div>
                <span className="font-mono text-[10px] text-text-muted">{data.alerts.length} open</span>
              </div>
              <div className="mt-3 flex flex-col gap-2">
                {data.alerts.length === 0 ? (
                  <div className="rounded-lg border border-dashed border-border-subtle py-4 text-center text-[11px] text-text-muted">
                    All clear — no open alerts.
                  </div>
                ) : (
                  data.alerts.map(n => <LudusBanner key={n.id} note={n} onAck={onAckLudus} />)
                )}
              </div>
            </Glass>
          </div>
        );
      case 'agents':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <Glass className="h-full p-5">
              <div className="flex items-center justify-between">
                <h2 className="font-display text-[14px] font-semibold tracking-wide text-text-primary">Active Agents</h2>
                <span className="font-mono text-[10px] text-text-muted">{data.agents.length} shards</span>
              </div>
              <div className="mt-3 flex flex-col gap-2">
                {data.agents.length === 0 ? (
                  <div className="rounded-lg border border-dashed border-border-subtle py-4 text-center text-[11px] text-text-muted">
                    No active agents — open Chat to submit a task.
                  </div>
                ) : (
                  data.agents.map((a) => {
                    const ap = approvals.approvalFor(a.codename);
                    return (
                      <AgentRow key={a.id} a={a} onPause={onPause} onResume={onResume} onOpenInConsole={onOpenInConsole}
                        pendingApprovalId={ap?.approval_id ?? null}
                        onApprove={(id) => approvals.resolve(id, 'approved')}
                        onReject={(id) => approvals.resolve(id, 'rejected')} />
                    );
                  })
                )}
              </div>
            </Glass>
          </div>
        );
      case 'budget_burn':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <AreaChartWidget
              title="Budget Burn"
              series={budgetSeries}
            />
          </div>
        );
      case 'queue_depth':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <BarChartWidget
              title="Queue Depth"
              series={queueSeries}
            />
          </div>
        );
      case 'line_chart':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <LineChartWidget
              title={chartTitle(widget, 'Metric')}
              series={budgetSeries}
            />
          </div>
        );
      case 'bar_chart':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <BarChartWidget
              title={chartTitle(widget, 'Metric')}
              series={queueSeries}
            />
          </div>
        );
      case 'area_chart':
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <AreaChartWidget
              title={chartTitle(widget, 'Metric')}
              series={budgetSeries}
            />
          </div>
        );
      case 'resources':
        return (
          <div className={isResourcesTab ? 'h-full' : 'sr-only'}>
            <ResourcesWidget data={data} />
          </div>
        );
      case 'surface_widget': {
        const key = surfaceKeyOf(widget);
        if (!key) return null;
        return (
          <div className={isAgentsTab ? 'h-full' : 'sr-only'}>
            <WidgetErrorBoundary label={surfaceLabel(key)}>
              {renderSurfaceWidget(key)}
            </WidgetErrorBoundary>
          </div>
        );
      }
      default:
        return null;
    }
  }

  if (loading) {
    return (
      <div className="grid grid-cols-12 gap-5 p-5" role="status" aria-label="Loading dashboard">
        <Glass className="col-span-12 xl:col-span-8 p-5">
          <Skeleton className="h-6 w-40 mb-4" />
          <Skeleton className="h-24 w-full mb-2" />
          <Skeleton className="h-24 w-full" />
        </Glass>
        <Glass className="col-span-12 xl:col-span-4 p-5">
          <Skeleton className="h-6 w-32 mb-4" />
          <Skeleton className="h-16 w-full mb-2" />
          <Skeleton className="h-16 w-full" />
        </Glass>
      </div>
    );
  }

  return (
    <div className="relative">
      {onOpenChat && (
        <div
          style={{
            backgroundImage:
              "linear-gradient(180deg, rgba(248, 243, 232, 0.96) 0%, rgba(235, 226, 208, 0.94) 100%), url('/assets/romanesque/travertine-parchment-tablet.jpg')",
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="vox-marble-tablet mx-5 mb-2 mt-1 flex items-center justify-between gap-3 rounded-xl border-2 border-[#C59B27]/85 px-3.5 py-1.5 shadow-[0_6px_18px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)]"
        >
          <div className="flex items-center gap-2 min-w-0">
            <span className="size-2 rounded-full bg-[#8C6825] shrink-0" aria-hidden="true" />
            <p className="font-display text-[11px] font-bold uppercase tracking-wider text-[#1C140C] shrink-0">Mission Control</p>
            <span className="text-[11px] font-medium text-[#4A381D] truncate">Dispatch new work or adjust posture in the Loquela composer.</span>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {customizeMode && (
              <>
                <button
                  type="button"
                  aria-expanded={pickerOpen}
                  onClick={() => setPickerOpen((v) => !v)}
                  className="rounded-md border border-[#A67C1E] bg-[#EFE6D2] px-2.5 py-1 text-[11px] font-semibold text-[#2B1F0F] hover:bg-white"
                >
                  Add widget
                </button>
                <button
                  type="button"
                  onClick={handleResetLayout}
                  className="rounded-md border border-[#A67C1E] bg-[#EFE6D2] px-2.5 py-1 text-[11px] font-semibold text-[#2B1F0F] hover:bg-white"
                >
                  Reset to default
                </button>
              </>
            )}
            <button
              type="button"
              aria-expanded={customizeMode}
              aria-pressed={customizeMode}
              onClick={() => {
                setCustomizeMode((v) => {
                  const next = !v;
                  if (!next) {
                    setPickerOpen(false);
                  }
                  return next;
                });
              }}
              className="rounded-md border border-[#A67C1E] bg-[#EFE6D2] px-2.5 py-1 text-[11px] font-semibold text-[#2B1F0F] hover:bg-white"
            >
              {customizeMode ? 'Done customizing' : 'Customize dashboard'}
            </button>
            <button
              type="button"
              data-testid="open-chat-cta"
              onClick={onOpenChat}
              className="shrink-0 rounded-lg border border-[#8C6825] bg-bg-elevated px-3 py-1 font-display text-[11px] font-semibold tracking-wide text-text-primary transition hover:border-[#D4AF37]"
            >
              Open Chat
            </button>
          </div>
        </div>
      )}
      {!onOpenChat && (
        <div className="mb-2 flex items-center justify-end gap-2 px-5">
          {customizeMode && (
            <>
              <button
                type="button"
                aria-expanded={pickerOpen}
                onClick={() => setPickerOpen((v) => !v)}
                className="rounded-md border border-border-subtle bg-bg-base/80 px-2.5 py-1 text-[11px] text-text-secondary hover:bg-overlay-subtle"
              >
                Add widget
              </button>
              <button
                type="button"
                onClick={handleResetLayout}
                className="rounded-md border border-border-subtle bg-bg-base/80 px-2.5 py-1 text-[11px] text-text-secondary hover:bg-overlay-subtle"
              >
                Reset to default
              </button>
            </>
          )}
          <button
            type="button"
            aria-expanded={customizeMode}
            aria-pressed={customizeMode}
            onClick={() => {
              setCustomizeMode((v) => {
                const next = !v;
                if (!next) {
                  setPickerOpen(false);
                }
                return next;
              });
            }}
            className="rounded-md border border-border-subtle bg-bg-base/80 px-2.5 py-1 text-[11px] text-text-secondary hover:bg-overlay-subtle"
          >
            {customizeMode ? 'Done customizing' : 'Customize dashboard'}
          </button>
        </div>
      )}
      <div
        data-testid="dashboard-kpi-strip"
        className="grid grid-cols-2 md:grid-cols-4 gap-2.5 mb-2.5 px-5"
      >
        <RomanTooltip
          title="Active Agents"
          purpose="Shows how many autonomous or sub-agent workers are actively executing turns right now."
          actionHint="Click to open Agent Operations Flow"
          statusBadge={`${data.agents.length} active`}
          placement="bottom"
          className="block"
        >
          <div onClick={() => onNavigate?.('vox-operations-hub')} className="cursor-pointer transition-transform hover:-translate-y-0.5">
            <Kpi label="Active Agents" value={data.agents.length} accent="cyan" sparkData={[2, 3, 2, 4, 3, 5, Math.max(2, data.agents.length)]} />
          </div>
        </RomanTooltip>
        <RomanTooltip
          title="Queue Depth"
          purpose="Shows the number of pending tasks waiting in the orchestrator hopper queue."
          actionHint="Click to open Task Queue & Hopper Ledger"
          statusBadge={`${data.kpis.queueDepth.value} queued`}
          placement="bottom"
          className="block"
        >
          <div onClick={() => onNavigate?.('vox-task-list')} className="cursor-pointer transition-transform hover:-translate-y-0.5">
            <Kpi label="Queue Depth" value={data.kpis.queueDepth.value} accent="amber" sparkData={data.kpis.queueDepth.spark?.length ? data.kpis.queueDepth.spark : [1, 2, 1, 3, 2, 1, Math.max(1, Number(data.kpis.queueDepth.value) || 2)]} />
          </div>
        </RomanTooltip>
        <RomanTooltip
          title="Budget Spent"
          purpose="Tracks cumulative token spend across cloud providers and local MENS inference for this session."
          actionHint="Click to open Cost & Token Analytics"
          statusBadge={typeof data.kpis.budgetBurn.value === 'number' ? `$${data.kpis.budgetBurn.value.toFixed(2)}` : String(data.kpis.budgetBurn.value)}
          placement="bottom"
          className="block"
        >
          <div onClick={() => onNavigate?.('vox-cost-analytics')} className="cursor-pointer transition-transform hover:-translate-y-0.5">
            <Kpi label="Budget Spent" value={typeof data.kpis.budgetBurn.value === 'number' ? `$${data.kpis.budgetBurn.value.toFixed(2)}` : data.kpis.budgetBurn.value} accent="brass" sparkData={data.kpis.budgetBurn.spark?.length ? data.kpis.budgetBurn.spark : [1.2, 2.4, 3.8, 5.1, 7.6, 9.8, 12.4]} />
          </div>
        </RomanTooltip>
        <RomanTooltip
          title="Mesh Peers"
          purpose="Displays connected Populi QUIC compute mesh nodes available for distributed agent and model jobs."
          actionHint="Click to open Compute Mesh Topology"
          statusBadge={`${data.peers.filter((p) => p.online).length} online`}
          placement="bottom"
          className="block"
        >
          <div onClick={() => onNavigate?.('vox-mesh-topology')} className="cursor-pointer transition-transform hover:-translate-y-0.5">
            <Kpi label="Mesh Peers" value={data.peers.filter((p) => p.online).length} accent="emerald" sparkData={[3, 3, 4, 4, 4, 5, Math.max(3, data.peers.filter((p) => p.online).length)]} />
          </div>
        </RomanTooltip>
      </div>

      {/* Workspace Architecture & Simulation Map (Plate 2 Romanesque Edition - immediately below KPI strip) */}
      <div className="mx-5 mb-2.5 rounded-xl border-2 border-[#C59B27] bg-bg-surface shadow-[0_10px_28px_rgba(0,0,0,0.25)] overflow-hidden">
        <div
          data-roman-frieze="true"
          style={{
            backgroundImage:
              'linear-gradient(115deg, rgba(255,252,245,0) 35%, rgba(255,248,225,0.48) 50%, rgba(255,252,245,0) 65%), linear-gradient(90deg, rgba(246,240,229,0.88) 0%, rgba(238,229,212,0.74) 48%, rgba(246,240,229,0.88) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
            backgroundSize: '240% 100%, 100% 100%, cover',
            backgroundPosition: 'center',
          }}
          className="vox-animate-marble-sheen flex flex-wrap items-center justify-between gap-2 px-3.5 py-2 text-xs border-b-2 border-[#C59B27]"
        >
          <div className="flex items-center gap-2 min-w-0">
            <RomanMedallion active size="sm">
              <Icon.romanColumn className="size-3.5 text-[#FFF8E7]" aria-hidden="true" />
            </RomanMedallion>
            <span className="font-display text-[11px] font-bold text-[#1C140C] uppercase tracking-[0.14em] truncate">
              Workspace Architecture Map
            </span>
            <span className="hidden sm:inline-block rounded border border-[#785312] bg-linear-to-b from-[#F2D67B] via-[#D4AF37] to-[#9A741C] px-2 py-0.5 font-display text-[9px] font-bold text-[#1A1205]">
              Click any building or subsystem to navigate
            </span>
          </div>
          <div className="flex items-center gap-1.5">
            <RomanTooltip
              title="Execution Pipeline Cascade"
              purpose="Inspect the 3-tier model cascade and laminar task verification stream from code generation to gated merge."
              actionHint="Click to open Pipeline Cascade (Harness River)"
              placement="bottom"
            >
              <button
                type="button"
                onClick={() => onNavigate?.('vox-harness-river')}
                className="inline-flex items-center gap-1.5 rounded-md border border-[#5B6B24] bg-bg-elevated px-2.5 py-1 font-display text-[10px] uppercase tracking-wider text-text-primary transition hover:border-[#8FA339]"
              >
                <Icon.aqueductArch className="size-3" aria-hidden="true" />
                <span>Pipeline Cascade</span>
              </button>
            </RomanTooltip>
            <RomanTooltip
              title="Full-Screen Workspace Map & Arena"
              purpose="Open the full-screen interactive architectural map with 7-region Astrolabe Radar, Git Commit Timeline, and Engineering Quests."
              actionHint="Click to open Full-Screen Workspace Map"
              placement="bottom"
            >
              <button
                type="button"
                onClick={() => onNavigate?.('vox-Ludus-view')}
                className="inline-flex items-center gap-1.5 rounded-md border border-[#D4AF37]/60 bg-bg-elevated px-2.5 py-1 font-display text-[10px] font-semibold uppercase tracking-wider text-text-primary transition hover:border-[#D4AF37]"
              >
                <Icon.astrolabeRadar className="size-3 text-brass" aria-hidden="true" />
                <span>Full-Screen Map</span>
              </button>
            </RomanTooltip>
            <RomanTooltip
              title={sandboxCollapsed ? 'Expand Workspace Map' : 'Collapse Workspace Map'}
              purpose="Toggle the embedded interactive Workspace Map strip on the Dashboard."
              actionHint={sandboxCollapsed ? 'Click to expand map strip' : 'Click to collapse map strip'}
              placement="bottom"
            >
              <button
                type="button"
                onClick={() => setSandboxCollapsed(!sandboxCollapsed)}
                className="rounded-md border border-[#D4AF37]/30 bg-bg-elevated px-2.5 py-1 font-display text-[10px] uppercase tracking-wider text-text-secondary hover:border-[#D4AF37] hover:text-text-primary"
              >
                {sandboxCollapsed ? 'Expand' : 'Collapse'}
              </button>
            </RomanTooltip>
          </div>
        </div>
        {!sandboxCollapsed && (
          <div className="h-[268px] relative">
            <EmbeddedSurfaceProvider value={true}>
              <LudusSandbox energy={80} maxEnergy={100} onNavigate={onNavigate} />
            </EmbeddedSurfaceProvider>
          </div>
        )}
      </div>

      {/* Progressive-Disclosure Tabbed Bottom Drawer (Concept Art Option B: Active Agents & Alerts | Attention Budget | System Resources) */}
      <div className="mx-5 mb-2.5">
        <div
          role="tablist"
          aria-label="Dashboard secondary panels"
          className="flex items-center gap-1.5 border-b border-[#D4AF37]/25 pb-2"
        >
          <button
            type="button"
            role="tab"
            aria-selected={activeBottomTab === 'agents'}
            onClick={() => setActiveBottomTab('agents')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-1 font-display text-[11px] font-semibold uppercase tracking-wider transition ${
              activeBottomTab === 'agents'
                ? 'border border-[#D4AF37]/60 bg-[#D4AF37]/15 text-text-primary shadow-[0_0_12px_rgba(212,175,55,0.15)]'
                : 'border border-transparent text-text-muted hover:border-border-subtle hover:bg-overlay-subtle hover:text-text-secondary'
            }`}
          >
            <span className="size-1.5 rounded-full bg-[#7B8C32]" aria-hidden="true" />
            <span>Active Agents &amp; Alerts</span>
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeBottomTab === 'budget'}
            onClick={() => setActiveBottomTab('budget')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-1 font-display text-[11px] font-semibold uppercase tracking-wider transition ${
              activeBottomTab === 'budget'
                ? 'border border-[#D4AF37]/60 bg-[#D4AF37]/15 text-text-primary shadow-[0_0_12px_rgba(212,175,55,0.15)]'
                : 'border border-transparent text-text-muted hover:border-border-subtle hover:bg-overlay-subtle hover:text-text-secondary'
            }`}
          >
            <span className="size-1.5 rounded-full bg-[#D4AF37]" aria-hidden="true" />
            <span>Attention Budget</span>
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeBottomTab === 'resources'}
            onClick={() => setActiveBottomTab('resources')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-1 font-display text-[11px] font-semibold uppercase tracking-wider transition ${
              activeBottomTab === 'resources'
                ? 'border border-[#D4AF37]/60 bg-[#D4AF37]/15 text-text-primary shadow-[0_0_12px_rgba(212,175,55,0.15)]'
                : 'border border-transparent text-text-muted hover:border-border-subtle hover:bg-overlay-subtle hover:text-text-secondary'
            }`}
          >
            <span className="size-1.5 rounded-full bg-[#C59B27]" aria-hidden="true" />
            <span>System Resources</span>
          </button>
        </div>
      </div>

      <div className={activeBottomTab === 'budget' && !customizeMode ? 'mx-5 mb-2.5 block' : 'sr-only'}>
        {attention_budget ? (
          <AttentionBudgetMeter budget={attention_budget} />
        ) : (
          <div className="rounded-xl border border-dashed border-[#D4AF37]/30 bg-bg-surface p-6 text-center">
            <p className="font-display text-xs text-text-secondary">No active attention budget constraint for current session.</p>
          </div>
        )}
      </div>

      {!layout.widgets.some((w) => w.kind === 'resources') && (
        <div
          className={
            activeBottomTab === 'resources' && !customizeMode
              ? 'mx-5 mb-2.5 block'
              : 'sr-only'
          }
        >
          <ResourcesWidget data={data} />
        </div>
      )}

      <WidgetPickerDrawer
        layout={layout}
        open={customizeMode && pickerOpen}
        onClose={() => setPickerOpen(false)}
        onAdd={handleAddWidget}
        onAddSurface={handleAddSurface}
      />
      <DashboardGrid
        layout={layout}
        customizeMode={customizeMode}
        onLayoutChange={updateLayout}
        renderWidget={renderWidget}
      />
    </div>
  );
}
