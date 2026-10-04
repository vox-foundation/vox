import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import { Glass } from '../../ui/Glass';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { BackendAvailability, type ProviderStatus } from './BackendAvailability';
import type { Toast } from '../../../types/tauri';
import { RoutingExplainer } from './RoutingExplainer';
import { useRoutingExplanation } from '../../../hooks/useRoutingExplanation';
import { modeLabel } from '../../../lib/turnEvents';
import { priceLabel } from '../../../lib/routingLabels';

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

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [cards, routing, active, statuses] = await Promise.all([
        invoke<ModelCard[]>('list_model_cards', { limit: 120 }),
        invoke<RoutingSummary>('get_routing_summary_live'),
        invoke<string | null>('get_active_model'),
        invoke<ProviderStatus[]>('inference_provider_status').catch(() => [] as ProviderStatus[]),
      ]);
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
      await invoke('set_active_model', { modelId: id });
      setActiveModel(id);
      void recordGamifyGuiEvent('model_activated', { model_id: id }, { enabled: gamifyEnabled });
      pushToast({ tone: 'ok', title: 'Active model set', body: id, cause: 'backend-ok' });
    } catch (err) {
      pushToast({ tone: 'error', title: 'Set active failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const hosted = models
    .filter(m => !m.id.includes('ollama') && !m.id.startsWith('mesh/') && !m.id.startsWith('mens/'))
    .filter(m => !freeOnly || m.is_free);
  const local = models
    .filter(m => m.id.includes('ollama') || m.id.startsWith('mesh/') || m.id.startsWith('mens/'))
    .filter(m => !freeOnly || m.is_free);

  return (
    <div className="flex flex-col gap-5">
      <Glass className="p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="font-display text-sm tracking-widest text-text-secondary uppercase">Model Registry</div>
            <div className="text-xs text-text-muted mt-1">
              {summary ? routingSummaryLine(summary) : 'Loading routing summary…'}
            </div>
          </div>
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-[11px] text-text-muted">
              <input
                type="checkbox"
                role="checkbox"
                aria-label="Free only"
                checked={freeOnly}
                onChange={e => setFreeOnly(e.target.checked)}
              />
              Free only
            </label>
            <div className="text-right">
              <div className="text-[10px] uppercase tracking-widest text-text-muted">Active</div>
              <div className="font-mono text-xs text-brass">{activeModel ?? 'auto-route'}</div>
            </div>
          </div>
        </div>
      </Glass>
      <RoutingPanel />
      <BackendAvailability statuses={providerStatuses} />
      {loading && models.length === 0 ? (
        <Glass className="p-8 text-center text-text-muted text-sm">Loading model catalog…</Glass>
      ) : (
        <>
          <ModelGrid title="Hosted" items={hosted} activeModel={activeModel} onSetDefault={setDefault} />
          <ModelGrid title="Local / Mesh / MENS" items={local} activeModel={activeModel} onSetDefault={setDefault} />
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
    <Glass id="routing-panel" className="p-4 flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3 text-xs">
        <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">Routing</div>
        <label className="flex items-center gap-1">
          Routing mode
          <select aria-label="Routing mode" value={mode} onChange={e => setMode(e.target.value)}>
            {MODES.map(m => <option key={m} value={m}>{modeLabel(m)}</option>)}
          </select>
        </label>
        <label className="flex items-center gap-1">
          Task
          <select aria-label="Task" value={task} onChange={e => setTask(e.target.value)}>
            {TASKS.map(t => <option key={t} value={t}>{t}</option>)}
          </select>
        </label>
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
    <section>
      <div className="mb-2 font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">{title}</div>
      <div role="list" className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
        {items.slice(0, 48).map(m => {
          const isActive = activeModel === m.id;
          return (
            <Glass key={m.id} role="listitem" className={`p-4 flex flex-col gap-3 ${isActive ? 'ring-1 ring-brass/40' : ''}`}>
              <div className="flex justify-between gap-2">
                <div className="min-w-0">
                  <div className="font-mono text-xs text-text-primary truncate" title={m.id}>{m.id}</div>
                  <div className="text-[10px] text-text-muted">{m.provider} · {m.tier}</div>
                </div>
                {m.is_free && <span className="text-[9px] uppercase tracking-widest" style={{ color: 'var(--color-status-pass)' }}>free</span>}
              </div>
              <div className="grid grid-cols-4 gap-2 text-[10px] font-mono text-text-muted">
                <div><span className="text-text-muted">ctx</span> {Math.round(m.max_tokens / 1000)}k</div>
                <div>{priceLabel(m.is_free ? 0 : m.cost_per_1k > 0 ? m.cost_per_1k * 1000 : null, m.is_free)}</div>
                <div><span className="text-text-muted">p50</span> {m.latency_p50_ms ?? '—'}</div>
                <div><span className="text-text-muted">qual</span> {m.quality_score != null ? m.quality_score.toFixed(2) : '—'}</div>
              </div>
              <button
                type="button"
                onClick={() => onSetDefault(m.id)}
                aria-pressed={isActive}
                aria-current={isActive ? 'true' : undefined}
                aria-label={`Set ${m.id} as active model${isActive ? ' (currently active)' : ''}`}
                className="mt-auto rounded-lg border border-border-subtle px-3 py-1.5 text-[10px] uppercase tracking-widest hover:bg-overlay-subtle"
              >
                {isActive ? 'Active' : 'Set active'}
              </button>
            </Glass>
          );
        })}
      </div>
    </section>
  );
}
