import React, { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import {
  filterPickerModels,
  normalizeModelCard,
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
  const [query, setQuery] = useState('');
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

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

  const apply = (id: string | null) => {
    onApplied?.(id);
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
              onClick={() => apply(null)}
              className="w-full truncate rounded-sm px-2 py-1 text-left font-mono text-[10px] text-text-secondary hover:bg-overlay-subtle"
            >
              auto-route (clear override)
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
            {visible.length === 0 && (
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
