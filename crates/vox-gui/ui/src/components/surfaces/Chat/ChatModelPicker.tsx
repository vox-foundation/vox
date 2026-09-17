import React, { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { getAutoModelRecommendation, type AutoModelRecommendation } from '../../../transport';
import {
  filterPickerModels,
  isLocalProviderName,
  modelMatchesQuery,
  normalizeModelCard,
  unselectableReason,
  type PickerModel,
  type ProviderStatus,
} from '../../../lib/modelPicker';
import { MODEL_LIST_LIMIT } from '../../../config/constants';
import { ModelPickerSearch } from './ModelPickerSearch';

/** Chat-surface model pick. The pick is lifted to App state and threaded into
 *  the chat submit payload as the `model_override` enqueue hint — the one
 *  channel the daemon consumes (TaskEnqueueHints.model_override →
 *  AgentTask.model_override → StreamRoute::UserModelOverride). Deliberately
 *  NOT `set_active_model`, which only touches the GUI process (Resolved
 *  decision "Item 4"). `null` pick = auto-route (clear the override). */

function shortModelId(id: string): string {
  return id.split('/').pop() ?? id;
}

export function ChatModelPicker({
  activeModel,
  onApplied,
}: {
  activeModel?: string | null;
  onApplied?: (modelId: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  const [models, setModels] = useState<PickerModel[]>([]);
  const [statuses, setStatuses] = useState<ProviderStatus[]>([]);
  const [recommendation, setRecommendation] = useState<AutoModelRecommendation | null>(null);
  const [query, setQuery] = useState('');
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let mounted = true;
    getAutoModelRecommendation()
      .then(rec => {
        if (mounted && rec) {
          setRecommendation(rec);
        }
      })
      .catch(() => {
        // Recommendation is best-effort
      });
    return () => {
      mounted = false;
    };
  }, []);

  // Escape + outside-click close (Escape pattern mirrors ChatSurface's routing
  // drawer; outside-click mirrors ChatSessionRail's menu dismiss). Without
  // these, only re-toggling or a selection closes the listbox.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    const onPointerDown = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onPointerDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onPointerDown);
    };
  }, [open]);

  const load = async () => {
    try {
      const [cards, providerStatuses] = await Promise.all([
        invoke<Array<Record<string, unknown>>>('list_model_cards', { limit: MODEL_LIST_LIMIT }),
        invoke<ProviderStatus[]>('inference_provider_status'),
      ]);
      setModels(
        (Array.isArray(cards) ? cards : [])
          .map(c => normalizeModelCard(c))
          .filter((m): m is PickerModel => m != null),
      );
      setStatuses(Array.isArray(providerStatuses) ? providerStatuses : []);
      setError(null);
    } catch (e) {
      setError(sanitizeErrorForToast(e));
    }
  };

  const toggle = async () => {
    const next = !open;
    setOpen(next);
    if (next) {
      setQuery('');
      await load();
    }
  };

  const visible = useMemo(
    () => filterPickerModels(models, statuses, query),
    [models, statuses, query],
  );

  // Local models (e.g. a freshly-trained `mens/foo`) that filterPickerModels
  // drops for cause — server not running, or not yet in local_models — get
  // shown dimmed with their reason instead of silently vanishing.
  const unreachableLocal = useMemo(
    () =>
      models
        .filter(m => isLocalProviderName(m.provider) || isLocalProviderName(m.providerType))
        .filter(m => modelMatchesQuery(m, query))
        .map(m => ({ model: m, reason: unselectableReason(m, statuses) }))
        .filter((r): r is { model: PickerModel; reason: string } => r.reason != null),
    [models, statuses, query],
  );

  const apply = (id: string | null) => {
    onApplied?.(id);
    setOpen(false);
  };

  const goStartMensServer = () => {
    window.dispatchEvent(new CustomEvent('vox://navigate-surface', { detail: { view: 'mens' } }));
    setOpen(false);
  };

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => void toggle()}
        className="rounded-lg border border-border-subtle px-2 py-1 font-mono text-[10px] text-text-muted hover:text-brass"
      >
        model: {activeModel ?? 'auto-route'}
      </button>
      {open && (
        <div
          role="listbox"
          aria-label="Pick model for this chat"
          className="absolute bottom-full left-0 z-50 mb-1 w-80 rounded-lg border border-border-subtle bg-bg-base p-1"
        >
          <div className="sticky top-0 z-10 bg-bg-base">
            <ModelPickerSearch value={query} onChange={setQuery} />
          </div>
          <div className="max-h-72 overflow-y-auto overscroll-contain custom-scrollbar">
            <button
              type="button"
              role="option"
              aria-selected={activeModel == null}
              aria-label={
                recommendation
                  ? `Auto (Recommended: ${shortModelId(recommendation.selected_model_id)}) auto-route`
                  : 'auto-route (clear override)'
              }
              title={
                recommendation
                  ? `Detected VRAM: ${recommendation.detected_vram_gb.toFixed(1)} GB — ${recommendation.tier_reason}`
                  : undefined
              }
              onClick={() => apply(null)}
              className="flex w-full items-center justify-between truncate rounded-sm px-2 py-1 text-left font-mono text-[10px] text-text-secondary hover:bg-overlay-subtle"
            >
              <span className="truncate">
                {recommendation
                  ? `Auto (Recommended: ${shortModelId(recommendation.selected_model_id)})`
                  : 'auto-route (clear override)'}
              </span>
              {recommendation && (
                <span
                  title={`${recommendation.detected_vram_gb.toFixed(1)} GB detected VRAM`}
                  className="ml-2 shrink-0 rounded border border-border-subtle bg-overlay-subtle px-1 py-0.5 text-[9px] text-text-muted"
                >
                  {recommendation.detected_vram_gb.toFixed(1)} GB
                </span>
              )}
            </button>
            {visible.map(m => (
              <button
                key={m.id}
                type="button"
                role="option"
                aria-selected={m.id === activeModel}
                onClick={() => apply(m.id)}
                className="w-full truncate rounded px-2 py-1 text-left font-mono text-[10px] text-text-secondary hover:bg-overlay-subtle"
              >
                {m.id}
              </button>
            ))}
            {unreachableLocal.map(({ model: m, reason }) => (
              <div
                key={m.id}
                className="flex items-center justify-between gap-2 truncate rounded px-2 py-1 font-mono text-[10px] text-text-muted opacity-60"
              >
                <span className="min-w-0 flex-1 truncate" title={`${m.id} — ${reason}`}>
                  <span>{m.id}</span>{' — '}<span>{reason}</span>
                </span>
                <button
                  type="button"
                  onClick={goStartMensServer}
                  className="shrink-0 text-brass underline opacity-100 hover:text-brass/80"
                >
                  Start server
                </button>
              </div>
            ))}
            {visible.length === 0 && unreachableLocal.length === 0 && (
              <div className="px-2 py-1.5 font-mono text-[10px] text-text-muted">
                No keyed models match
              </div>
            )}
          </div>
        </div>
      )}
      {error && <div role="alert" className="mt-1 text-[10px] text-rose-400">{error}</div>}
    </div>
  );
}
