import React from 'react';
import type { BranchDivergenceResult } from '../../../lib/chat/branchTree';

export interface BranchComparisonModalProps {
  readonly title?: string;
  readonly leftLabel: string;
  readonly rightLabel: string;
  readonly divergence: BranchDivergenceResult;
  readonly onClose?: () => void;
}

export const BranchComparisonModal: React.FC<BranchComparisonModalProps> = ({
  title = 'Branch Comparison',
  leftLabel,
  rightLabel,
  divergence,
  onClose,
}) => {
  return (
    <section
      aria-label={title}
      data-testid="branch-comparison-modal"
      className="flex flex-col gap-3 rounded-lg border border-zinc-800 bg-zinc-950 p-4 text-zinc-100"
    >
      <header className="flex items-center justify-between border-b border-zinc-800 pb-2">
        <div>
          <h3 className="text-sm font-semibold">{title}</h3>
          <p className="text-xs text-zinc-400" data-testid="branch-divergence-summary">
            Diverges after node: {divergence.divergenceAncestorId ?? 'root'} (
            {divergence.sharedPrefix.length} shared messages)
          </p>
        </div>
        {onClose ? (
          <button
            type="button"
            onClick={onClose}
            className="rounded border border-zinc-700 px-2 py-1 text-xs hover:bg-zinc-800"
          >
            Close
          </button>
        ) : null}
      </header>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <div data-testid="branch-left-column" className="rounded border border-zinc-800 p-2">
          <div className="mb-2 text-xs font-medium text-(--color-status-pass)">{leftLabel}</div>
          {divergence.leftExclusive.map((m) => (
            <div key={m.id} className="mb-1 text-xs text-zinc-200">
              <span className="font-mono text-zinc-400">[{m.role}]</span> {m.content}
            </div>
          ))}
        </div>
        <div data-testid="branch-right-column" className="rounded border border-zinc-800 p-2">
          <div className="mb-2 text-xs font-medium text-sky-400">{rightLabel}</div>
          {divergence.rightExclusive.map((m) => (
            <div key={m.id} className="mb-1 text-xs text-zinc-200">
              <span className="font-mono text-zinc-400">[{m.role}]</span> {m.content}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};
