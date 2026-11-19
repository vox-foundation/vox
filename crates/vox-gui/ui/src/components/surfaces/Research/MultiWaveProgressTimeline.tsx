import React from 'react';
import { Icon } from '../../ui/Icons';

export interface WaveStageInfo {
  waveIndex: number;
  title: string;
  description: string;
  status: 'pending' | 'running' | 'completed' | 'skipped';
}

export interface MultiWaveProgressTimelineProps {
  currentWave: number;
  totalWaves?: number;
  stabilityScore?: number;
  unresolvedContradictions?: number;
  earlyExitTriggered?: boolean;
}

const WAVE_TEMPLATES: ReadonlyArray<{ title: string; description: string }> = [
  {
    title: 'Wave 1: Reconnaissance',
    description: 'Broad search & claim extraction',
  },
  {
    title: 'Wave 2: Contradiction Isolation',
    description: 'Deep dives & conflict resolution',
  },
  {
    title: 'Wave 3: Adversarial Validation',
    description: 'Empirical sandbox & authority weighting',
  },
  {
    title: 'Wave 4: Deep Corroboration',
    description: 'Cross-domain replication & citation audit',
  },
  {
    title: 'Wave 5: Final Synthesis',
    description: 'Epistemic convergence & verdict lock',
  },
];

function gridColsClass(waveCount: number): string {
  switch (waveCount) {
    case 1:
      return 'grid-cols-1';
    case 2:
      return 'grid-cols-1 sm:grid-cols-2';
    case 3:
      return 'grid-cols-1 md:grid-cols-3';
    case 4:
      return 'grid-cols-1 md:grid-cols-2 lg:grid-cols-4';
    default:
      return 'grid-cols-1 md:grid-cols-3 lg:grid-cols-5';
  }
}

export function MultiWaveProgressTimeline({
  currentWave,
  totalWaves = 3,
  stabilityScore,
  unresolvedContradictions = 0,
  earlyExitTriggered = false,
}: MultiWaveProgressTimelineProps) {
  const clampedTotal = Math.min(5, Math.max(1, Math.round(totalWaves)));
  const waves: WaveStageInfo[] = Array.from({ length: clampedTotal }, (_, idx) => {
    const waveIndex = idx + 1;
    const tpl = WAVE_TEMPLATES[idx] ?? {
      title: `Wave ${waveIndex}: Deep Pass`,
      description: 'Iterative evidence refinement',
    };
    const status: WaveStageInfo['status'] =
      currentWave > waveIndex
        ? 'completed'
        : currentWave === waveIndex
          ? 'running'
          : earlyExitTriggered && currentWave < waveIndex
            ? 'skipped'
            : 'pending';

    return {
      waveIndex,
      title: tpl.title,
      description: tpl.description,
      status,
    };
  });

  const contradictionWaveIndex = clampedTotal >= 2 ? 2 : 1;

  return (
    <div
      data-testid="multi-wave-progress-timeline"
      className="rounded-xl border border-border-subtle bg-bg-surface p-4 font-sans"
    >
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
            Multi-Wave Deep Research Engine
          </span>
          <span
            data-testid="multi-wave-counter"
            className="rounded border border-brass/30 bg-bg-elevated px-2 py-0.5 font-mono text-[11px] text-brass"
          >
            Wave {Math.max(1, Math.min(currentWave, clampedTotal))} of {clampedTotal}
          </span>
        </div>

        {stabilityScore !== undefined && (
          <div className="flex items-center gap-2 font-mono text-xs">
            <span className="text-text-muted">Stability Metric:</span>
            <span
              className={`font-semibold ${
                stabilityScore >= 0.85
                  ? 'text-(--color-status-pass)'
                  : stabilityScore >= 0.6
                    ? 'text-(--color-status-warn)'
                    : 'text-(--color-status-fail)'
              }`}
            >
              {Math.round(stabilityScore * 100)}%
            </span>
            {earlyExitTriggered && (
              <span className="rounded border border-(--color-status-pass)/35 bg-bg-elevated px-2 py-0.5 text-[11px] text-(--color-status-pass)">
                Early Exit
              </span>
            )}
          </div>
        )}
      </div>

      <div className={`grid gap-3 ${gridColsClass(clampedTotal)}`}>
        {waves.map((w) => {
          const isCurrent = w.status === 'running';
          const isDone = w.status === 'completed';
          const isSkipped = w.status === 'skipped';

          const borderColor = isCurrent
            ? 'border-brass bg-bg-elevated ring-1 ring-brass/30'
            : isDone
              ? 'border-(--color-status-pass)/35 bg-bg-elevated'
              : isSkipped
                ? 'border-border-subtle/50 bg-bg-panel opacity-65'
                : 'border-border-subtle bg-bg-panel';

          return (
            <div
              key={w.waveIndex}
              data-testid={`wave-stage-${w.waveIndex}`}
              data-status={w.status}
              className={`flex flex-col justify-between rounded-lg border p-3 transition-colors ${borderColor}`}
            >
              <div>
                <div className="flex items-center justify-between gap-1">
                  <span className="font-sans text-xs font-semibold text-text-primary">
                    {w.title}
                  </span>
                  <span className="font-mono text-[11px] shrink-0">
                    {isCurrent && (
                      <span className="inline-flex items-center gap-1 text-brass">
                        <Icon.circleDot className="size-3 shrink-0" aria-hidden="true" />
                        <span>Running</span>
                      </span>
                    )}
                    {isDone && (
                      <span className="inline-flex items-center gap-1 text-(--color-status-pass)">
                        <Icon.check className="size-3 shrink-0" aria-hidden="true" />
                        <span>Complete</span>
                      </span>
                    )}
                    {isSkipped && (
                      <span className="inline-flex items-center gap-1 text-text-muted">
                        <Icon.ban className="size-3 shrink-0" aria-hidden="true" />
                        <span>Skipped</span>
                      </span>
                    )}
                    {w.status === 'pending' && (
                      <span className="inline-flex items-center gap-1 text-text-muted">
                        <Icon.circle className="size-3 shrink-0" aria-hidden="true" />
                        <span>Queued</span>
                      </span>
                    )}
                  </span>
                </div>
                <p className="mt-1 text-xs leading-relaxed text-text-secondary">{w.description}</p>
              </div>

              {w.waveIndex === contradictionWaveIndex && unresolvedContradictions > 0 && (
                <div className="mt-2 inline-flex items-center gap-1 text-[11px] text-(--color-status-warn)">
                  <Icon.alert className="size-3.5 shrink-0" aria-hidden="true" />
                  <span>
                    {unresolvedContradictions} contradiction{unresolvedContradictions > 1 ? 's' : ''} detected
                  </span>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
