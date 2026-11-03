import React from 'react';

export interface WaveStageInfo {
  waveIndex: number;
  title: string;
  description: string;
  status: 'pending' | 'running' | 'completed' | 'skipped';
}

export interface MultiWaveProgressTimelineProps {
  currentWave: number;
  totalWaves: number;
  stabilityScore?: number;
  unresolvedContradictions?: number;
  earlyExitTriggered?: boolean;
}

export function MultiWaveProgressTimeline({
  currentWave,
  totalWaves,
  stabilityScore,
  unresolvedContradictions = 0,
  earlyExitTriggered = false,
}: MultiWaveProgressTimelineProps) {
  const waves: WaveStageInfo[] = [
    {
      waveIndex: 1,
      title: 'Wave 1: Reconnaissance',
      description: 'Broad search & claim extraction',
      status:
        currentWave > 1
          ? 'completed'
          : currentWave === 1
          ? 'running'
          : 'pending',
    },
    {
      waveIndex: 2,
      title: 'Wave 2: Contradiction Isolation',
      description: 'Deep dives & conflict resolution',
      status:
        currentWave > 2
          ? 'completed'
          : currentWave === 2
          ? 'running'
          : earlyExitTriggered && currentWave <= 1
          ? 'skipped'
          : 'pending',
    },
    {
      waveIndex: 3,
      title: 'Wave 3: Adversarial Validation',
      description: 'Empirical sandbox & authority weighting',
      status:
        currentWave > 3
          ? 'completed'
          : currentWave === 3
          ? 'running'
          : earlyExitTriggered && currentWave <= 2
          ? 'skipped'
          : 'pending',
    },
  ];

  return (
    <div className="rounded-xl border border-border-subtle bg-overlay-subtle p-4 font-sans">
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs font-semibold uppercase tracking-wider text-text-primary">
            Multi-Wave Deep Research Engine
          </span>
          <span className="rounded bg-brass/10 px-2 py-0.5 font-mono text-[10px] text-brass ring-1 ring-brass/20">
            Wave {Math.min(currentWave, totalWaves)} of {totalWaves}
          </span>
        </div>

        {stabilityScore !== undefined && (
          <div className="flex items-center gap-2 font-mono text-xs">
            <span className="text-text-muted">Stability Metric:</span>
            <span
              className={`font-semibold ${
                stabilityScore >= 0.85
                  ? 'text-emerald-400'
                  : stabilityScore >= 0.60
                  ? 'text-amber-400'
                  : 'text-red-400'
              }`}
            >
              {Math.round(stabilityScore * 100)}%
            </span>
            {earlyExitTriggered && (
              <span className="rounded bg-emerald-500/15 px-1.5 py-0.5 text-[10px] text-emerald-300 ring-1 ring-emerald-500/30">
                Early Exit
              </span>
            )}
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        {waves.map((w) => {
          const isCurrent = w.status === 'running';
          const isDone = w.status === 'completed';
          const isSkipped = w.status === 'skipped';

          const borderColor = isCurrent
            ? 'border-brass ring-1 ring-brass/30'
            : isDone
            ? 'border-emerald-500/30 bg-emerald-500/5'
            : isSkipped
            ? 'border-border-subtle/50 opacity-60'
            : 'border-border-subtle';

          return (
            <div
              key={w.waveIndex}
              className={`flex flex-col justify-between rounded-lg border p-3 transition-all ${borderColor}`}
            >
              <div>
                <div className="flex items-center justify-between">
                  <span className="font-mono text-[11px] font-medium text-text-primary">
                    {w.title}
                  </span>
                  <span className="font-mono text-[10px]">
                    {isCurrent && <span className="animate-pulse text-brass">● Running</span>}
                    {isDone && <span className="text-emerald-400">✓ Complete</span>}
                    {isSkipped && <span className="text-text-muted">⊘ Skipped</span>}
                    {w.status === 'pending' && <span className="text-text-muted">○ Queued</span>}
                  </span>
                </div>
                <p className="mt-1 text-xs text-text-secondary">{w.description}</p>
              </div>

              {w.waveIndex === 2 && unresolvedContradictions > 0 && (
                <div className="mt-2 text-[11px] text-amber-400">
                  ⚠️ {unresolvedContradictions} contradiction{unresolvedContradictions > 1 ? 's' : ''} detected
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
