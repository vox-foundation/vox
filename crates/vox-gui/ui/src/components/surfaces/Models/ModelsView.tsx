import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invokeCmd } from '../../../transport';
import { Glass } from '../../ui/Glass';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { BackendAvailability, type ProviderStatus } from './BackendAvailability';
import { MensServePanel } from './MensServePanel';
import { ModelLeaderboardPanel } from './ModelLeaderboard';
import type { Toast } from '../../../types/tauri';
import { RoutingExplainer } from './RoutingExplainer';
import { useRoutingExplanation } from '../../../hooks/useRoutingExplanation';
import { modeLabel } from '../../../lib/turnEvents';
import { priceLabel } from '../../../lib/routingLabels';
import { BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';

interface ModelCard {
  id: string;
  provider: string;
  tier: string;
  cost_per_1k: number;
  max_tokens: number;
  is_free: boolean;
  latency_p50_ms?: number | null;
  success_rate?: number | null;
  quality_score?: number | null;
}

interface RoutingSummary {
  active_model?: string | null;
  exploration_spent_usd: number;
  exploration_budget_usd: number;
  arm_count: number;
  model_count: number;
}

type ModelsSnapshot = {
  cards: ModelCard[];
  routing: RoutingSummary;
  active_model: string | null;
  providers: ProviderStatus[];
};

/** The header line from whatever the daemon reported; a field it did not send is left out, never printed as "undefined". */
function routingSummaryLine(s: Partial<RoutingSummary>): string {
  const parts = [
    typeof s.model_count === 'number' ? `${s.model_count} models` : null,
    typeof s.arm_count === 'number' ? `${s.arm_count} routing arms` : null,
    typeof s.exploration_spent_usd === 'number' && typeof s.exploration_budget_usd === 'number'
      ? `explore $${s.exploration_spent_usd.toFixed(2)} / $${s.exploration_budget_usd.toFixed(0)}`
      : null,
  ].filter((p): p is string => p !== null);
  return parts.length > 0 ? parts.join(' · ') : 'Routing summary unavailable';
}

interface ModelsViewProps {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
}

export function ModelsView({ pushToast, gamifyEnabled = false }: ModelsViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [models, setModels] = useState<ModelCard[]>([]);
  const [summary, setSummary] = useState<RoutingSummary | null>(null);
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [providerStatuses, setProviderStatuses] = useState<ProviderStatus[]>([]);
  const [freeOnly, setFreeOnly] = useState(false);
  const [tierFilter, setTierFilter] = useState<'all' | 'hosted' | 'local'>('all');

  const ModelIcon = (Icon as unknown as Record<string, React.FC<React.SVGProps<SVGSVGElement>>>).models ?? Icon.cpu;

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const snap = await invokeCmd<ModelsSnapshot | null>('get_models_snapshot', { limit: 120 });
      const cards = Array.isArray(snap?.cards) ? snap.cards : [];
      const routing = snap?.routing ?? null;
      const active = snap?.active_model ?? null;
      const statuses = snap?.providers;
      setModels(cards);
      setSummary(routing);
      setActiveModel(active);
      // Harden against a RESOLVED null (the e2e variant mocks and any future backend
      // change resolve unknown commands to null — `.catch` never fires for that, and
      // `statuses.length` would then TypeError inside BackendAvailability).
      setProviderStatuses(Array.isArray(statuses) ? statuses : []);
    } catch (err) {
      pushToast({ tone: 'error', title: 'Models load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoading(false);
    }
  }, [pushToast]);

  useEffect(() => {
    refresh();
    // Embedded mini-render: one initial fetch only, no repeating poll.
    if (embedded) return;
    const id = setInterval(refresh, 8000);
    return () => clearInterval(id);
  }, [refresh, embedded]);

  const setDefault = async (id: string) => {
    try {
      await invokeCmd('set_active_model', { modelId: id });
      setActiveModel(id);
      void recordGamifyGuiEvent('model_activated', { model_id: id }, { enabled: gamifyEnabled });
      pushToast({ tone: 'ok', title: 'Active model set', body: id, cause: 'backend-ok' });
    } catch (err) {
      pushToast({ tone: 'error', title: 'Set active failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const allHostedCount = models.filter(
    m => !m.id.includes('ollama') && !m.id.startsWith('mesh/') && !m.id.startsWith('mens/')
  ).length;
  const allLocalCount = models.filter(
    m => m.id.includes('ollama') || m.id.startsWith('mesh/') || m.id.startsWith('mens/')
  ).length;
  const freeModelCount = models.filter(m => m.is_free).length;
  const hosted = models
    .filter(m => !m.id.includes('ollama') && !m.id.startsWith('mesh/') && !m.id.startsWith('mens/'))
    .filter(m => !freeOnly || m.is_free);
  const local = models
    .filter(m => m.id.includes('ollama') || m.id.startsWith('mesh/') || m.id.startsWith('mens/'))
    .filter(m => !freeOnly || m.is_free);

  return (
    <div className="@container/surface flex min-w-0 flex-col gap-4">
      <RomanSurfaceHeader
        domainTag="COMPUTE · MODELS"
        title="Model Registry, Leaderboard & Routing Posture"
        subtitle="Compare active frontier and local MENS models, inspect live routing decisions, and verify backend availability."
        waxSealBadge="ROUTER LIVE"
        icon={<ModelIcon className="size-5 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'Hosted',
            valueText: `${hosted.length}/${models.length}`,
            ratio: models.length > 0 ? Math.max(0, Math.min(1, allHostedCount / models.length)) : 0,
            tone: 'gold',
            purpose: 'Hosted frontier models in catalog',
            actionHint: tierFilter === 'hosted' ? 'Show all model tiers' : 'Filter to hosted frontier models',
            active: tierFilter === 'hosted',
            onClick: () => setTierFilter(prev => (prev === 'hosted' ? 'all' : 'hosted')),
          },
          {
            label: 'Local MENS',
            valueText: `${local.length} Local`,
            ratio: models.length > 0 ? Math.max(0, Math.min(1, allLocalCount / models.length)) : 0,
            tone: local.length > 0 ? 'emerald' : 'gold',
            purpose: 'Local Ollama, Mesh, and MENS models',
            actionHint: tierFilter === 'local' ? 'Show all model tiers' : 'Filter to local / Mesh / MENS models',
            active: tierFilter === 'local',
            onClick: () => setTierFilter(prev => (prev === 'local' ? 'all' : 'local')),
          },
          {
            label: 'Free Tier',
            valueText: `${freeModelCount} Free`,
            ratio: models.length > 0 ? Math.max(0, Math.min(1, freeModelCount / models.length)) : 0,
            tone: freeOnly ? 'emerald' : 'gold',
            purpose: 'Zero-cost catalog models eligible for free routing',
            actionHint: 'Toggle free-only model filter',
            active: freeOnly,
            onClick: () => setFreeOnly(prev => !prev),
          },
        ]}
      />
      <Glass
        style={{
          backgroundImage:
            'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(238,229,211,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="p-4 border-2 border-[#C59B27] text-[#1C140C] shadow-[0_6px_18px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.9)]"
      >
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="font-display text-sm font-bold tracking-widest text-[#1C140C] uppercase">Model Registry</div>
            <div className="text-xs font-medium text-[#4A3718] mt-1">
              {summary ? routingSummaryLine(summary) : 'Loading routing summary…'}
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <label className="flex items-center gap-2 text-[11px] font-semibold text-[#2B2013]">
              <input
                type="checkbox"
                role="checkbox"
                aria-label="Free only"
                checked={freeOnly}
                onChange={e => setFreeOnly(e.target.checked)}
                className="accent-[#8C6825]"
              />
              Free only
            </label>
            <div className="rounded-lg border border-[#8C6825] bg-bg-elevated px-2.5 py-1 text-right">
              <div className="text-[9px] uppercase tracking-widest text-text-secondary">Active</div>
              <div className="font-mono text-xs text-text-primary font-semibold">{activeModel ?? 'auto-route'}</div>
            </div>
          </div>
        </div>
      </Glass>
      <BentoGrid preset="hero" className="items-start">
        <div className="flex min-h-0 min-w-0 flex-col gap-3 @3xl/surface:col-span-7 @5xl/surface:col-span-8">
          <RoutingPanel />
          <ModelLeaderboardPanel />
        </div>
        <div className="flex min-h-0 min-w-0 flex-col gap-3 @3xl/surface:col-span-5 @5xl/surface:col-span-4">
          <BackendAvailability statuses={providerStatuses} />
          <MensServePanel pushToast={pushToast} />
        </div>
      </BentoGrid>
      {loading && models.length === 0 ? (
        <Glass className="p-8 text-center text-text-secondary text-sm border-[#D4AF37]/35 bg-bg-surface">
          Loading model catalog…
        </Glass>
      ) : (
        <>
          {tierFilter !== 'local' && (
            <ModelGrid title="Hosted" items={hosted} activeModel={activeModel} onSetDefault={setDefault} />
          )}
          {tierFilter !== 'hosted' && (
            <ModelGrid title="Local / Mesh / MENS" items={local} activeModel={activeModel} onSetDefault={setDefault} />
          )}
        </>
      )}
    </div>
  );
}

const MODES = ['efficiency', 'balanced', 'genius', 'free'] as const;
const TASKS = ['codegen', 'research', 'review', 'general'] as const;

function RoutingPanel() {
  const [mode, setMode] = useState<string>('efficiency');
  const [task, setTask] = useState<string>('codegen');
  const { explanation, health, loading } = useRoutingExplanation(mode, task, 7);
  return (
    <Glass id="routing-panel" className="p-4 flex flex-col gap-3 border-[#D4AF37]/35 bg-bg-surface shadow-[0_4px_20px_rgba(0,0,0,0.2)]">
      <div className="flex flex-wrap items-center justify-between gap-3 text-xs">
        <div className="flex flex-wrap items-center gap-3">
          <div className="font-display text-[11px] tracking-[0.2em] uppercase text-brass">Routing</div>
          <div className="flex items-center gap-1.5 text-text-secondary">
            <label htmlFor="models-routing-mode">Routing mode</label>
            <VoxSelect
              id="models-routing-mode"
              size="xs"
              aria-label="Routing mode"
              value={mode}
              onValueChange={setMode}
              options={MODES.map(m => ({ value: m, label: modeLabel(m) }))}
              className="h-7 w-auto border-[#D4AF37]/30 bg-bg-elevated font-mono text-[11px] text-text-primary"
            />
          </div>
          <div className="flex items-center gap-1.5 text-text-secondary">
            <label htmlFor="models-routing-task">Task</label>
            <VoxSelect
              id="models-routing-task"
              size="xs"
              aria-label="Task"
              value={task}
              onValueChange={setTask}
              options={TASKS.map(t => ({ value: t, label: t }))}
              className="h-7 w-auto border-[#D4AF37]/30 bg-bg-elevated font-mono text-[11px] text-text-primary"
            />
          </div>
        </div>
        <RomanWaxSealBadge label="ACTIVE DISPATCH" tone="gold" />
      </div>
      <RoutingExplainer explanation={explanation} health={health} loading={loading} />
    </Glass>
  );
}

function ModelGrid({ title, items, activeModel, onSetDefault }: {
  title: string; items: ModelCard[]; activeModel: string | null; onSetDefault: (id: string) => void;
}) {
  if (items.length === 0) return null;
  return (
    <section className="min-w-0">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <div className="font-display text-[11px] tracking-[0.2em] uppercase text-brass">{title}</div>
        <span className="font-mono text-[10px] text-text-muted">{items.length} models</span>
      </div>
      <BentoScrollRegion ariaLabel={`${title} model catalog`} className="max-h-[36rem] pr-1">
        <div role="list" className="grid grid-cols-1 gap-3 @xl/surface:grid-cols-2 @4xl/surface:grid-cols-3 @6xl/surface:grid-cols-4">
          {items.slice(0, 48).map(m => {
            const isActive = activeModel === m.id;
            return (
              <Glass
                key={m.id}
                role="listitem"
                className={`p-4 flex min-w-0 flex-col gap-3 border-[#D4AF37]/35 bg-bg-surface shadow-[0_4px_16px_rgba(0,0,0,0.2)] ${
                  isActive ? 'ring-1 ring-[#D4AF37] border-[#D4AF37]' : ''
                }`}
              >
                <div className="flex flex-wrap justify-between gap-2">
                  <div className="min-w-0">
                    <div className="font-mono text-xs text-text-primary truncate font-medium" title={m.id}>{m.id}</div>
                    <div className="text-[10px] text-text-secondary">{m.provider} · {m.tier}</div>
                  </div>
                  {m.is_free && <RomanWaxSealBadge label="FREE" tone="emerald" />}
                </div>
                <div className="grid grid-cols-2 gap-2 text-[10px] font-mono text-[#D6CEBF] @xs/surface:grid-cols-4">
                  <div><span className="text-[#8C806B]">ctx</span> {Math.round(m.max_tokens / 1000)}k</div>
                  <div>{priceLabel(m.is_free ? 0 : m.cost_per_1k > 0 ? m.cost_per_1k * 1000 : null, m.is_free)}</div>
                  <div><span className="text-[#8C806B]">p50</span> {m.latency_p50_ms ?? '—'}</div>
                  <div><span className="text-[#8C806B]">qual</span> {m.quality_score != null ? m.quality_score.toFixed(2) : '—'}</div>
                </div>
                <button
                  type="button"
                  onClick={() => onSetDefault(m.id)}
                  aria-pressed={isActive}
                  aria-current={isActive ? 'true' : undefined}
                  aria-label={`Set ${m.id} as active model${isActive ? ' (currently active)' : ''}`}
                  className={`mt-auto rounded-lg border px-3 py-1.5 text-[10px] uppercase tracking-widest transition-colors ${
                    isActive
                      ? 'border-[#D4AF37] bg-[#D4AF37]/20 text-[#F3E5AB] font-semibold shadow-[0_0_12px_rgba(212,175,55,0.3)]'
                      : 'border-[#D4AF37]/30 text-[#D6CEBF] hover:bg-[#D4AF37]/10 hover:text-[#F3E5AB]'
                  }`}
                >
                  {isActive ? 'Active' : 'Set active'}
                </button>
              </Glass>
            );
          })}
        </div>
      </BentoScrollRegion>
    </section>
  );
}
