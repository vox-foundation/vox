import React, { useState } from 'react';

export interface ConsensusLegViewDto {
  modelId: string;
  status: 'pending' | 'streaming' | 'completed' | 'error';
  content: string;
  reasoning?: string;
  responseTimeMs?: number;
  qualityScore?: number;
  errorMessage?: string;
}

export interface ConsensusGridViewProps {
  legs: ConsensusLegViewDto[];
  defaultLayout?: 'grid' | 'stacked';
  synthesis?: string | null;
  onAdoptVariant?: (modelId: string) => void;
}

export const ConsensusGridView: React.FC<ConsensusGridViewProps> = ({
  legs,
  defaultLayout = 'grid',
  synthesis = null,
  onAdoptVariant,
}) => {
  const [layout, setLayout] = useState<'grid' | 'stacked'>(defaultLayout);
  const colCount = Math.min(Math.max(legs.length, 1), 4);

  return (
    <section
      data-testid="consensus-grid-view"
      aria-label="Multi-model consensus responses"
      className="flex flex-col gap-3 rounded-xl border border-zinc-800 bg-zinc-950/90 p-3"
    >
      <div className="flex items-center justify-between border-b border-zinc-800/80 pb-2">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold uppercase tracking-[0.08em] text-brass">
            Multi-Model Consensus ({legs.length} Models)
          </span>
        </div>
        <div className="flex items-center gap-1" role="group" aria-label="Consensus layout mode">
          <button
            type="button"
            data-testid="consensus-layout-grid"
            aria-pressed={layout === 'grid'}
            onClick={() => setLayout('grid')}
            className={`rounded px-2 py-1 text-xs font-medium focus:outline-none focus:ring-2 focus:ring-inset focus:ring-brass ${
              layout === 'grid'
                ? 'bg-brass/20 text-brass'
                : 'text-zinc-400 hover:text-zinc-200'
            }`}
          >
            Grid View
          </button>
          <button
            type="button"
            data-testid="consensus-layout-stacked"
            aria-pressed={layout === 'stacked'}
            onClick={() => setLayout('stacked')}
            className={`rounded px-2 py-1 text-xs font-medium focus:outline-none focus:ring-2 focus:ring-inset focus:ring-brass ${
              layout === 'stacked'
                ? 'bg-brass/20 text-brass'
                : 'text-zinc-400 hover:text-zinc-200'
            }`}
          >
            Stacked View
          </button>
        </div>
      </div>

      <div
        data-testid="consensus-legs-container"
        data-layout={layout}
        data-columns={layout === 'grid' ? colCount : 1}
        className={
          layout === 'grid'
            ? 'grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4'
            : 'flex flex-col gap-3'
        }
      >
        {legs.map((leg) => (
          <article
            key={leg.modelId}
            data-testid={`consensus-leg-${leg.modelId}`}
            className="flex flex-col justify-between rounded-lg border border-zinc-800/90 bg-zinc-900/70 p-3"
          >
            <div>
              <header className="mb-2 flex items-center justify-between gap-2">
                <span className="truncate font-mono text-xs font-semibold text-zinc-100">
                  {leg.modelId}
                </span>
                <span
                  data-testid={`consensus-status-${leg.modelId}`}
                  className="rounded bg-zinc-800 px-1.5 py-0.5 text-[11px] font-medium text-zinc-300"
                >
                  {leg.status}
                </span>
              </header>
              {leg.status === 'error' ? (
                <p className="text-xs text-(--color-status-fail)">{leg.errorMessage ?? 'Model error'}</p>
              ) : (
                <div className="whitespace-pre-wrap text-xs leading-relaxed text-zinc-200">
                  {leg.content}
                </div>
              )}
            </div>

            <footer className="mt-3 flex items-center justify-between border-t border-zinc-800/60 pt-2 text-[11px] text-zinc-400">
              <span>
                {leg.responseTimeMs !== undefined ? `${leg.responseTimeMs}ms` : 'In flight'}
              </span>
              {leg.qualityScore !== undefined && (
                <span
                  data-testid={`consensus-quality-${leg.modelId}`}
                  className="font-mono text-(--color-status-pass)"
                >
                  QA {leg.qualityScore.toFixed(1)}
                </span>
              )}
              {onAdoptVariant && (
                <button
                  type="button"
                  data-testid={`consensus-adopt-${leg.modelId}`}
                  onClick={() => onAdoptVariant(leg.modelId)}
                  className="rounded border border-brass/40 bg-brass/15 px-1.5 py-0.5 font-mono text-[11px] text-brass hover:bg-brass/25"
                >
                  Adopt Variant
                </button>
              )}
            </footer>
          </article>
        ))}
      </div>

      {synthesis && (
        <div
          data-testid="consensus-synthesis-card"
          className="rounded-lg border border-brass/35 bg-brass/6 p-3 text-xs leading-relaxed text-zinc-200 whitespace-pre-wrap"
        >
          {synthesis}
        </div>
      )}
    </section>
  );
};
