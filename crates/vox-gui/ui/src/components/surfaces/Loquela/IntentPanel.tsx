import React from 'react';
import type { Effort, IntentFields } from '../../../lib/intentSpec';
import { VoxSelect } from '../../ui/Select';

interface IntentPanelProps {
  intent: IntentFields;
  onChange: (patch: Partial<IntentFields>) => void;
  /** Effort is a task-queue concern; hide it on Quick chat. */
  showEffort?: boolean;
}

const FIELD_CLS =
  'w-full rounded-md border border-border-subtle bg-overlay-subtle px-2 py-1.5 text-[12px] text-text-primary placeholder:text-text-muted focus:outline-hidden focus:ring-1 focus:ring-brass/40';
const LABEL_CLS = 'font-display text-[11px] uppercase tracking-[0.13em] text-text-muted';

export function IntentPanel({ intent, onChange, showEffort = true }: IntentPanelProps) {
  return (
    <div className="mt-2 grid grid-cols-1 gap-2 border-t border-border-subtle pt-2 sm:grid-cols-2" data-testid="intent-panel">
      <label className="flex flex-col gap-1 sm:col-span-2">
        <span className={LABEL_CLS}>Goal</span>
        <input aria-label="Goal" value={intent.goal} placeholder="What outcome should the agent achieve?"
          onChange={(e) => onChange({ goal: e.target.value })} className={FIELD_CLS} />
      </label>
      <label className="flex flex-col gap-1">
        <span className={LABEL_CLS}>Constraints</span>
        <textarea aria-label="Constraints" rows={2} value={intent.constraints} placeholder="Boundaries: don't touch X, keep API stable…"
          onChange={(e) => onChange({ constraints: e.target.value })} className={`${FIELD_CLS} resize-none`} />
      </label>
      <label className="flex flex-col gap-1">
        <span className={LABEL_CLS}>Acceptance criteria</span>
        <textarea aria-label="Acceptance criteria" rows={2} value={intent.acceptance} placeholder="How you'll judge the work is done"
          onChange={(e) => onChange({ acceptance: e.target.value })} className={`${FIELD_CLS} resize-none`} />
      </label>
      {showEffort && (
        <div className="flex flex-col gap-1">
          <label htmlFor="intent-effort-select" className={LABEL_CLS}>Effort</label>
          <VoxSelect
            id="intent-effort-select"
            aria-label="Effort"
            value={intent.effort}
            onValueChange={(v) => onChange({ effort: v as Effort })}
            options={[
              { value: '', label: 'default' },
              { value: 'background', label: 'background — when idle' },
              { value: 'normal', label: 'normal' },
              { value: 'urgent', label: 'urgent — jump the queue' },
            ]}
            className={FIELD_CLS}
          />
        </div>
      )}
    </div>
  );
}
