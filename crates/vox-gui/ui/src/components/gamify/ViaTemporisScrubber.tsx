import React from 'react';
import { Icon } from '../ui/Icons';
import { RomanTooltip } from '../ui/RomanTooltip';

export interface ViaTemporisMilestoneItem {
  readonly turnNumber: number;
  readonly romanNumeral: string;
  readonly refName: string;
  readonly commitOidShort: string;
  readonly addedLines: number;
  readonly deletedLines: number;
  readonly modifiedFiles: readonly string[];
  readonly testShieldStatus: 'pass' | 'fail' | 'running';
  readonly tokenCostBu: number;
}

export interface ViaTemporisScrubberProps {
  readonly milestones?: readonly ViaTemporisMilestoneItem[];
  readonly checkpoints?: readonly ViaTemporisMilestoneItem[];
  readonly selectedTurn?: number;
  readonly activeTurn?: number;
  readonly activeHeadTurn?: number;
  readonly busyRestore?: boolean;
  readonly onSelectTurn?: (turnNumber: number) => void;
  readonly onCaptureCheckpoint?: () => void;
  readonly onRollbackToTurn?: (turnNumber: number) => void;
  readonly onClose?: () => void;
}

const ROMAN_NUMERALS = [
  'I',
  'II',
  'III',
  'IV',
  'V',
  'VI',
  'VII',
  'VIII',
  'IX',
  'X',
  'XI',
  'XII',
];

export function toRomanNumeral(num: number): string {
  if (num === 4) return 'IV';
  if (num === 9) return 'IX';
  if (num >= 1 && num <= ROMAN_NUMERALS.length) {
    return ROMAN_NUMERALS[num - 1];
  }
  return `M${num}`;
}

/**
 * Concept Art Plate 2: Git Commit Timeline & Checkpoints tablet.
 * Features a miniature crimson wax seal medallion, laurel wreath badge,
 * and a thin golden progress bar showing active turn position.
 */
export const ViaTemporisScrubber: React.FC<ViaTemporisScrubberProps> = ({
  milestones,
  checkpoints,
  selectedTurn,
  activeTurn,
  activeHeadTurn,
  busyRestore = false,
  onSelectTurn,
  onCaptureCheckpoint,
  onRollbackToTurn,
  onClose,
}) => {
  const items = milestones ?? checkpoints ?? [];
  const currentTurn = selectedTurn ?? activeTurn ?? (items[items.length - 1]?.turnNumber ?? 1);
  const headTurn = activeHeadTurn ?? (items[items.length - 1]?.turnNumber ?? currentTurn);
  const activeIdx = Math.max(0, items.findIndex((m) => m.turnNumber === currentTurn));
  const progressPct = items.length > 0 ? ((activeIdx + 1) / items.length) * 100 : 100;
  const activeMilestone = items.find((m) => m.turnNumber === currentTurn) ?? items[items.length - 1];

  return (
    <div
      data-testid="urbs-via-temporis-scrubber"
      role="region"
      aria-label="Git Commit Timeline & Checkpoint Scrubber"
      className="vox-marble-tablet flex flex-col gap-1 rounded-xl border border-border-strong bg-bg-surface px-2.5 py-1 text-xs text-text-primary shadow-[0_8px_20px_rgba(28,20,12,0.18),inset_0_1px_0_rgba(255,255,255,0.25)] select-none max-w-full"
    >
      {/* Top Row: Wax seal, badge, and step indicators */}
      <div className="flex flex-nowrap items-center justify-between gap-2 overflow-x-auto no-scrollbar">
        <div className="flex items-center gap-1.5">
          {/* Miniature Crimson Wax Seal Medallion */}
          <div className="relative size-5 shrink-0 overflow-hidden rounded-full border border-border-strong shadow-[0_0_8px_rgba(212,175,55,0.35)]">
            <img
              src="/assets/romanesque/wax-seal-medallion.jpg"
              alt=""
              aria-hidden="true"
              className="size-full object-cover"
            />
          </div>

          <RomanTooltip
            title="Git Timeline & Checkpoints"
            purpose="Scrub through repository commit milestones and agent turn checkpoints to inspect diffs or restore a prior working state."
            actionHint="Select any milestone step to inspect its changes"
          >
            <span className="inline-flex items-center gap-1 rounded border border-border-strong bg-bg-elevated px-2 py-0.5 font-display text-[9px] font-bold tracking-wider text-text-primary">
              <Icon.laurelWreath className="size-3 text-(--color-accent-default)" aria-hidden="true" />
              <span>GIT TIMELINE · CHECKPOINTS</span>
            </span>
          </RomanTooltip>

          <span className="font-mono text-[9px] text-text-secondary whitespace-nowrap">
            Step {currentTurn}/{headTurn}
          </span>
        </div>

        {/* Milestone Steps Stones */}
        <div className="flex items-center gap-1 overflow-x-auto no-scrollbar">
          {items.map((m) => {
            const isSelected = m.turnNumber === currentTurn;
            const isFail = m.testShieldStatus === 'fail';
            return (
              <RomanTooltip
                key={m.turnNumber}
                title={`Commit Step ${m.turnNumber} (${m.romanNumeral}) · ${m.commitOidShort}`}
                purpose={`Branch/Ref: ${m.refName} · Changed +${m.addedLines}/-${m.deletedLines} lines across ${m.modifiedFiles.length || 1} files. Verification: ${m.testShieldStatus.toUpperCase()}.`}
                actionHint="Click to preview this commit checkpoint on the map"
                statusBadge={m.testShieldStatus.toUpperCase()}
              >
                <button
                  type="button"
                  data-testid={`urbs-milestone-stone-${m.turnNumber}`}
                  onClick={() => onSelectTurn?.(m.turnNumber)}
                  title={`${m.refName} (+${m.addedLines}/-${m.deletedLines} LoC, ${m.testShieldStatus.toUpperCase()})`}
                  className={`inline-flex items-center gap-1 rounded px-2 py-0.5 font-mono text-[9px] font-bold transition ${
                    isSelected
                      ? 'border border-(--color-accent-default) bg-bg-elevated text-text-primary shadow-[0_0_8px_rgba(212,175,55,0.35)]'
                      : isFail
                        ? 'border border-(--color-status-fail)/70 bg-bg-surface text-text-primary hover:border-(--color-status-fail)'
                        : 'border border-border-subtle bg-bg-surface text-text-secondary hover:border-border-strong'
                  }`}
                >
                  <Icon.romanColumn className={`size-2.5 ${isSelected ? 'text-(--color-accent-default)' : 'text-text-muted'}`} />
                  <span>Step {m.turnNumber} ({m.romanNumeral})</span>
                  <span className={`text-[8.5px] ${isFail ? 'text-(--color-status-fail)' : 'text-(--color-status-pass)'}`}>
                    {isFail ? 'FAIL' : 'PASS'}
                  </span>
                </button>
              </RomanTooltip>
            );
          })}
        </div>

        {/* Diff summary & Actions */}
        <div className="flex items-center gap-1.5">
          {activeMilestone && (
            <RomanTooltip
              title="Commit Diff Summary"
              purpose={`Lines added: +${activeMilestone.addedLines}, lines removed: -${activeMilestone.deletedLines}, compute budget units: ${activeMilestone.tokenCostBu} BU.`}
            >
              <span
                data-testid="urbs-scrubber-delta-pill"
                className="rounded border border-(--color-status-pass)/40 bg-(--color-status-pass)/15 px-1.5 py-0.5 font-mono text-[9px] text-(--color-status-pass) whitespace-nowrap"
              >
                +{activeMilestone.addedLines}/-{activeMilestone.deletedLines} LoC · {activeMilestone.tokenCostBu} BU
              </span>
            </RomanTooltip>
          )}

          <RomanTooltip
            title="Save Git Checkpoint"
            purpose="Creates a new snapshot milestone of your current working tree so you can safely experiment or roll back at any time."
            actionHint="Click to record a new Git checkpoint"
          >
            <button
              type="button"
              data-testid="urbs-scrubber-capture-btn"
              onClick={onCaptureCheckpoint}
              className="rounded border border-border-strong bg-bg-elevated px-2 py-0.5 font-display text-[9px] font-bold text-text-primary hover:border-(--color-accent-default) transition whitespace-nowrap"
            >
              + Checkpoint
            </button>
          </RomanTooltip>

          <RomanTooltip
            title={`Restore to Step ${currentTurn}`}
            purpose="Reverts workspace files back to the selected commit checkpoint."
            actionHint={`Click to restore working tree to Step ${currentTurn} (${toRomanNumeral(currentTurn)})`}
          >
            <button
              type="button"
              data-testid="urbs-scrubber-rollback-btn"
              disabled={busyRestore}
              onClick={() => onRollbackToTurn?.(currentTurn)}
              className="rounded border border-(--color-status-fail)/60 bg-(--color-status-fail)/15 px-2 py-0.5 font-display text-[9px] font-bold text-(--color-status-fail) hover:border-(--color-status-fail) transition whitespace-nowrap disabled:opacity-50"
            >
              Restore Step {currentTurn} ({toRomanNumeral(currentTurn)})
            </button>
          </RomanTooltip>

          {onClose && (
            <RomanTooltip
              title="Collapse Git Timeline"
              purpose="Minimize the Git Milestone Scrubber back to the compact Crimson Wax Seal trigger pill."
              actionHint="Click to collapse timeline"
            >
              <button
                type="button"
                data-testid="urbs-scrubber-close-btn"
                onClick={onClose}
                aria-label="Collapse Git Timeline"
                className="rounded border border-border-subtle bg-bg-elevated px-1.5 py-0.5 font-mono text-[9px] text-text-secondary hover:border-border-strong hover:text-text-primary transition"
              >
                ✕
              </button>
            </RomanTooltip>
          )}
        </div>
      </div>

      {/* Thin Golden Progress Bar showing active turn position */}
      <div className="h-1 w-full overflow-hidden rounded-full bg-bg-elevated border border-border-subtle">
        <div
          data-testid="urbs-scrubber-progress-bar"
          className="h-full bg-(--color-accent-default) transition-all duration-300 shadow-[0_0_6px_rgba(212,175,55,0.6)]"
          style={{ width: `${progressPct}%` }}
        />
      </div>
    </div>
  );
};
