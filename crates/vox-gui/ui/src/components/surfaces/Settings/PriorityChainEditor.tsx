import React, { useEffect, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { MODEL_LIST_LIMIT } from '../../../config/constants';
import { voxTransport } from '../../../transport';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';
import { Slider } from '../../ui/Slider';
import type { Toast } from '../../../types/tauri';

// --- SelectionPolicy JSON shape (mirrors vox_orchestrator::models::SelectionPolicy) ---
// The Rust enums are externally-tagged serde with snake_case variant names:
//   EmphasizeAxis{axis,weight} -> { "emphasize_axis": { "axis": "intelligence", "weight": 90 } }
//   PinModel(String)           -> { "pin_model": "model-id" }
//   PreferFree                 -> "prefer_free"
//   FallbackWhen{condition,then}
//        -> { "fallback_when": { "condition": <cond>, "then": <step> } }
// FallbackCondition snake_case; CostAboveUsdPerCall(f64) is a newtype:
//   "out_of_tokens" | "no_candidate" | "provider_unavailable"
//   | { "cost_above_usd_per_call": 1.5 }

type AxisKind = 'intelligence' | 'efficiency' | 'responsiveness';

type FallbackCondition =
  | 'out_of_tokens'
  | 'no_candidate'
  | 'provider_unavailable'
  | { cost_above_usd_per_call: number };

type SelectionStep =
  | { emphasize_axis: { axis: AxisKind; weight: number } }
  | { pin_model: string }
  | 'prefer_free'
  | { fallback_when: { condition: FallbackCondition; then: SelectionStep } };

interface SelectionPolicy {
  steps: SelectionStep[];
}

const AXES: AxisKind[] = ['intelligence', 'efficiency', 'responsiveness'];
const SIMPLE_CONDITIONS: FallbackCondition[] = [
  'out_of_tokens',
  'no_candidate',
  'provider_unavailable',
];

// --- summarisation helpers for chip rendering ---
function conditionLabel(c: FallbackCondition): string {
  if (typeof c === 'object') return `cost > $${c.cost_above_usd_per_call}/call`;
  return c.replace(/_/g, ' ');
}

function stepLabel(s: SelectionStep): string {
  if (s === 'prefer_free') return 'Prefer free model';
  if (typeof s === 'object') {
    if ('emphasize_axis' in s) {
      const { axis, weight } = s.emphasize_axis;
      return `Emphasize ${axis} (${weight})`;
    }
    if ('pin_model' in s) return `Pin model: ${s.pin_model}`;
    if ('fallback_when' in s) {
      const { condition, then } = s.fallback_when;
      return `When ${conditionLabel(condition)} → ${stepLabel(then)}`;
    }
  }
  return 'unknown step';
}

function stepIcon(s: SelectionStep): React.ReactNode {
  if (s === 'prefer_free') return <Icon.circle className="size-3 shrink-0" aria-hidden="true" />;
  if (typeof s === 'object') {
    if ('emphasize_axis' in s) return <Icon.trendUp className="size-3 shrink-0" aria-hidden="true" />;
    if ('pin_model' in s) return <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />;
    if ('fallback_when' in s) return <Icon.refresh className="size-3 shrink-0" aria-hidden="true" />;
  }
  return '•';
}

const BTN =
  'rounded-sm border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[10px] text-text-secondary hover:bg-overlay-subtle disabled:opacity-40';

interface Props {
  pushToast: (t: Toast) => void;
}

export function PriorityChainEditor({ pushToast }: Props) {
  const [steps, setSteps] = useState<SelectionStep[]>([]);
  const [models, setModels] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState(false);
  const dragIndex = useRef<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);
  // Monotonic write-id so a slower, older setSelectionPolicy write can't clobber
  // a newer one. lastConfirmed holds the most recent successfully-persisted state
  // to revert to on failure of the latest write.
  const writeId = useRef(0);
  const lastConfirmed = useRef<SelectionStep[]>([]);

  // Hydrate from persisted policy + load model list (reuse Models surface source).
  useEffect(() => {
    (async () => {
      try {
        const json = await voxTransport.getSelectionPolicy();
        const parsed = JSON.parse(json) as SelectionPolicy;
        const hydrated = Array.isArray(parsed.steps) ? parsed.steps : [];
        setSteps(hydrated);
        lastConfirmed.current = hydrated;
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Could not load priority chain', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      } finally {
        setLoading(false);
      }
      try {
        const cards = await voxTransport.listModels(MODEL_LIST_LIMIT);
        setModels((cards as any[]).map((c) => c.id).filter(Boolean));
      } catch {
        // model list is optional; PinModel falls back to a free-text input.
      }
    })();
  }, []);

  // Persist the chain. Called on every mutation (save-on-change). Writes are
  // versioned: only the latest write's result is honoured, so a slower older
  // write can't overwrite newer UI/state. On failure of the latest write we
  // revert to the last-confirmed state and surface the error.
  const persist = async (next: SelectionStep[]) => {
    setSteps(next);
    const id = ++writeId.current;
    try {
      await voxTransport.setSelectionPolicy(JSON.stringify({ steps: next }));
      if (id !== writeId.current) return; // a newer write superseded this one
      lastConfirmed.current = next;
    } catch (err) {
      if (id !== writeId.current) return; // stale failure; a newer write owns state
      setSteps(lastConfirmed.current);
      pushToast({ tone: 'warn', title: 'Priority chain save failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const removeStep = (i: number) => persist(steps.filter((_, idx) => idx !== i));

  const move = (from: number, to: number) => {
    if (to < 0 || to >= steps.length || from === to) return;
    const next = steps.slice();
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    persist(next);
  };

  // HTML5 drag-and-drop reorder.
  const onDrop = (target: number) => {
    const from = dragIndex.current;
    dragIndex.current = null;
    setDragOver(null);
    if (from == null) return;
    // splice semantics already handle index shift via move().
    move(from, target);
  };

  const addStep = (s: SelectionStep) => {
    setAdding(false);
    persist([...steps, s]);
  };

  if (loading) {
    return <div className="mt-4 text-[12px] text-text-muted">Loading priority chain…</div>;
  }

  return (
    <div className="mt-5 rounded-xl border border-border-subtle bg-overlay-subtle p-3">
      <div className="flex items-center justify-between">
        <div className="font-display text-[12px] tracking-[0.12em] uppercase text-text-secondary">
          Model priority chain
        </div>
        <button type="button" className={BTN} onClick={() => setAdding((a) => !a)} aria-expanded={adding}>
          {adding ? 'close' : '+ add step'}
        </button>
      </div>
      <p className="mt-1 text-[11px] text-text-muted">
        Evaluated top-to-bottom; first match wins. Changes apply on next orchestrator restart.
      </p>

      {adding && <AddStepMenu models={models} onAdd={addStep} />}

      {steps.length === 0 ? (
        <div className="mt-3 rounded-md border border-dashed border-border-subtle p-3 text-center text-[11px] text-text-muted">
          No steps — the orchestrator uses its default selection cascade. Add a step to override.
        </div>
      ) : (
        <ul className="mt-3 space-y-1.5">
          {steps.map((s, i) => (
            <li
              key={i}
              draggable
              onDragStart={() => {
                dragIndex.current = i;
              }}
              onDragOver={(e) => {
                e.preventDefault();
                setDragOver(i);
              }}
              onDragLeave={() => setDragOver((d) => (d === i ? null : d))}
              onDrop={(e) => {
                e.preventDefault();
                onDrop(i);
              }}
              className={`flex items-center gap-2 rounded-md border bg-overlay-subtle p-2 transition ${
                dragOver === i ? 'border-brass/50 bg-brass/5' : 'border-border-subtle'
              }`}
            >
              <span aria-hidden="true" className="inline-flex cursor-grab select-none items-center text-text-muted" title="Drag to reorder">
                <Icon.gripVertical className="size-3.5" aria-hidden="true" />
              </span>
              <span aria-hidden="true" className="inline-flex items-center font-mono text-[11px] text-brass">{stepIcon(s)}</span>
              <span className="font-mono text-[10px] text-text-muted">{i + 1}.</span>
              <span className="flex-1 text-[12px] text-text-secondary">{stepLabel(s)}</span>
              <button type="button" className={`${BTN} inline-flex items-center justify-center`} disabled={i === 0} onClick={() => move(i, i - 1)} aria-label={`Move step ${i + 1} up`} title="Move up">
                <Icon.arrowUp className="size-3" aria-hidden="true" />
              </button>
              <button
                type="button"
                className={`${BTN} inline-flex items-center justify-center`}
                disabled={i === steps.length - 1}
                onClick={() => move(i, i + 1)}
                aria-label={`Move step ${i + 1} down`}
                title="Move down"
              >
                <Icon.arrowDown className="size-3" aria-hidden="true" />
              </button>
              <button
                type="button"
                className="inline-flex items-center justify-center rounded-sm border border-rose-500/20 bg-rose-500/4 px-2 py-1 font-mono text-[10px] text-rose-300 hover:bg-rose-500/10"
                onClick={() => removeStep(i)}
                aria-label={`Remove step ${i + 1}`}
                title="Remove step"
              >
                <Icon.close className="size-3" aria-hidden="true" />
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

// --- Add-step menu: builds one SelectionStep then hands it back via onAdd ---
function AddStepMenu({
  models,
  onAdd,
}: {
  models: string[];
  onAdd: (s: SelectionStep) => void;
}) {
  const [kind, setKind] = useState<'emphasize_axis' | 'pin_model' | 'prefer_free' | 'fallback_when'>(
    'emphasize_axis',
  );
  const [axis, setAxis] = useState<AxisKind>('intelligence');
  const [weight, setWeight] = useState(50);
  const [pin, setPin] = useState(models[0] ?? '');
  const [fbCond, setFbCond] = useState<'out_of_tokens' | 'no_candidate' | 'provider_unavailable' | 'cost'>(
    'out_of_tokens',
  );
  const [fbCost, setFbCost] = useState(0.05);
  // Nested step for FallbackWhen — kept simple: a non-recursive leaf step.
  const [thenKind, setThenKind] = useState<'prefer_free' | 'pin_model' | 'emphasize_axis'>('prefer_free');
  const [thenAxis, setThenAxis] = useState<AxisKind>('efficiency');
  const [thenWeight, setThenWeight] = useState(50);
  const [thenPin, setThenPin] = useState(models[0] ?? '');

  // models may resolve after mount (listModels() is async); fill the pin defaults
  // once it arrives, but only if the user hasn't already chosen something.
  useEffect(() => {
    if (models.length > 0) {
      setPin((c) => c || models[0]);
      setThenPin((c) => c || models[0]);
    }
  }, [models]);

  const buildThen = (): SelectionStep => {
    if (thenKind === 'prefer_free') return 'prefer_free';
    if (thenKind === 'pin_model') return { pin_model: thenPin };
    return { emphasize_axis: { axis: thenAxis, weight: thenWeight } };
  };

  const build = (): SelectionStep | null => {
    if (kind === 'emphasize_axis') return { emphasize_axis: { axis, weight } };
    if (kind === 'pin_model') {
      if (!pin.trim()) return null;
      return { pin_model: pin.trim() };
    }
    if (kind === 'prefer_free') return 'prefer_free';
    const condition: FallbackCondition =
      fbCond === 'cost' ? { cost_above_usd_per_call: fbCost } : fbCond;
    return { fallback_when: { condition, then: buildThen() } };
  };

  const sel =
    'rounded-sm border border-border-subtle bg-black/30 px-2 py-1 font-mono text-[11px] text-text-primary focus:border-brass/40 focus:outline-hidden';

  return (
    <div className="mt-3 space-y-2 rounded-md border border-border-subtle bg-black/20 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-display text-[10px] uppercase tracking-widest text-text-muted">Step type</span>
        <VoxSelect
          size="xs"
          aria-label="Step type"
          className={`w-40 ${sel}`}
          value={kind}
          onValueChange={(v) => setKind(v as typeof kind)}
          options={[
            { value: 'emphasize_axis', label: 'Emphasize axis' },
            { value: 'pin_model', label: 'Pin model' },
            { value: 'prefer_free', label: 'Prefer free' },
            { value: 'fallback_when', label: 'Fallback when…' },
          ]}
        />
      </div>

      {kind === 'emphasize_axis' && (
        <div className="flex flex-wrap items-center gap-2">
          <VoxSelect
            size="xs"
            aria-label="Axis"
            className={`w-36 ${sel}`}
            value={axis}
            onValueChange={(v) => setAxis(v as AxisKind)}
            options={AXES.map((a) => ({ value: a, label: a }))}
          />
          <Slider
            ariaLabel="Axis weight"
            min={0}
            max={100}
            step={1}
            value={[weight]}
            onValueChange={(vals) => {
              if (vals[0] !== undefined) setWeight(vals[0]);
            }}
            className="w-40"
          />
          <span className="w-8 font-mono text-[11px] text-text-secondary">{weight}</span>
        </div>
      )}

      {kind === 'pin_model' && (
        <div className="flex flex-wrap items-center gap-2">
          {models.length > 0 ? (
            <VoxSelect
              size="xs"
              aria-label="Model to pin"
              className={`w-64 ${sel}`}
              value={pin}
              onValueChange={setPin}
              options={models.map((m) => ({ value: m, label: m }))}
            />
          ) : (
            <input
              className={`${sel} min-w-[16rem]`}
              aria-label="Model id to pin"
              placeholder="model id"
              value={pin}
              onChange={(e) => setPin(e.target.value)}
            />
          )}
        </div>
      )}

      {kind === 'prefer_free' && (
        <div className="text-[11px] text-text-muted">Selects the best eligible free-tier model.</div>
      )}

      {kind === 'fallback_when' && (
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-display text-[10px] uppercase tracking-widest text-text-muted">When</span>
            <VoxSelect
              size="xs"
              aria-label="Fallback condition"
              className={`w-48 ${sel}`}
              value={fbCond}
              onValueChange={(v) => setFbCond(v as typeof fbCond)}
              options={[
                ...SIMPLE_CONDITIONS.map((c) => ({ value: String(c), label: conditionLabel(c) })),
                { value: 'cost', label: 'cost above $/call…' },
              ]}
            />
            {fbCond === 'cost' && (
              <input
                type="number"
                step={0.01}
                min={0}
                value={fbCost}
                onChange={(e) => setFbCost(Number(e.target.value))}
                className={`${sel} w-24`}
              />
            )}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-display text-[10px] uppercase tracking-widest text-text-muted">Then</span>
            <VoxSelect
              size="xs"
              aria-label="Fallback action"
              className={`w-36 ${sel}`}
              value={thenKind}
              onValueChange={(v) => setThenKind(v as typeof thenKind)}
              options={[
                { value: 'prefer_free', label: 'Prefer free' },
                { value: 'pin_model', label: 'Pin model' },
                { value: 'emphasize_axis', label: 'Emphasize axis' },
              ]}
            />
            {thenKind === 'pin_model' &&
              (models.length > 0 ? (
                <VoxSelect
                  size="xs"
                  aria-label="Fallback model to pin"
                  className={`w-56 ${sel}`}
                  value={thenPin}
                  onValueChange={setThenPin}
                  options={models.map((m) => ({ value: m, label: m }))}
                />
              ) : (
                <input
                  className={`${sel} min-w-56`}
                  aria-label="Fallback model id to pin"
                  placeholder="model id"
                  value={thenPin}
                  onChange={(e) => setThenPin(e.target.value)}
                />
              ))}
            {thenKind === 'emphasize_axis' && (
              <>
                <VoxSelect
                  size="xs"
                  aria-label="Fallback axis"
                  className={`w-36 ${sel}`}
                  value={thenAxis}
                  onValueChange={(v) => setThenAxis(v as AxisKind)}
                  options={AXES.map((a) => ({ value: a, label: a }))}
                />
                <Slider
                  ariaLabel="Fallback axis weight"
                  min={0}
                  max={100}
                  step={1}
                  value={[thenWeight]}
                  onValueChange={(vals) => {
                    if (vals[0] !== undefined) setThenWeight(vals[0]);
                  }}
                  className="w-32"
                />
                <span className="w-8 font-mono text-[11px] text-text-secondary">{thenWeight}</span>
              </>
            )}
          </div>
        </div>
      )}

      <div className="flex justify-end">
        <button
          type="button"
          className={BTN}
          onClick={() => {
            const s = build();
            if (s) onAdd(s);
          }}
        >
          add to chain
        </button>
      </div>
    </div>
  );
}
