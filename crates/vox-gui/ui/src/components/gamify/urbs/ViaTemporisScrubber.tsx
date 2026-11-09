import React from 'react';
import { Icon } from '../../ui/Icons';
import { RomanTooltip } from '../../ui/RomanTooltip';

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
  readonly milestones: readonly ViaTemporisMilestoneItem[];
  readonly selectedTurn: number;
  readonly activeHeadTurn: number;
  readonly busyRestore?: boolean;
  readonly onSelectTurn: (turnNumber: number) => void;
  readonly onCaptureCheckpoint: () => void;
  readonly onRollbackToTurn: (turnNumber: number) => void;
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
  if (num >= 1 && num <= ROMAN_NUMERALS.length) {
    return ROMAN_NUMERALS[num - 1];
  }
  return `M${num}`;
}

export const ViaTemporisScrubber: React.FC<ViaTemporisScrubberProps> = ({
  milestones,
  selectedTurn,
  activeHeadTurn,
  busyRestore = false,
  onSelectTurn,
  onCaptureCheckpoint,
  onRollbackToTurn,
}) => {
  const activeMilestone =
    milestones.find((m) => m.turnNumber === selectedTurn) ??
    milestones[milestones.length - 1];

  return (
    <div
      data-testid="urbs-via-temporis-scrubber"
      role="region"
      aria-label="Git Commit Timeline & Checkpoint Scrubber"
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 10,
        padding: '6px 12px',
        background: 'var(--color-bg-surface)',
        border: '1.5px solid var(--color-border-strong)',
        borderRadius: 10,
        boxShadow: '0 8px 20px rgba(28, 20, 12, 0.18), inset 0 1px 0 rgba(255, 255, 255, 0.25)',
        color: 'var(--color-text-primary)',
        fontSize: 11,
        flexWrap: 'wrap',
      }}
    >
      {/* Left Label: Plain-English Git Timeline */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <RomanTooltip
          title="Git Commit Timeline"
          purpose="Scrub through repository commit milestones and agent turn checkpoints to inspect diffs or restore a prior working state."
          actionHint="Select any milestone step to inspect its changes"
        >
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 5,
              fontSize: 10,
              fontWeight: 800,
              letterSpacing: '0.08em',
              color: 'var(--color-text-primary)',
              padding: '3px 8px',
              borderRadius: 6,
              background: 'var(--color-bg-elevated)',
              border: '1px solid var(--color-border-strong)',
              whiteSpace: 'nowrap',
            }}
          >
            <Icon.hourglassVia style={{ width: 12, height: 12, color: 'var(--color-accent-default)' }} />
            <span>GIT TIMELINE</span>
          </span>
        </RomanTooltip>
        <span
          style={{
            fontSize: 10,
            color: 'var(--color-text-secondary)',
            fontFamily: "var(--font-sans, 'Palatino', Georgia, serif)",
            whiteSpace: 'nowrap',
          }}
        >
          Step {selectedTurn}/{activeHeadTurn}
        </span>
      </div>

      {/* Center: Roman Stone Milestone Scrubber */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          flex: 1,
          minWidth: 180,
          justifyContent: 'center',
        }}
      >
        {milestones.map((m) => {
          const isSelected = m.turnNumber === selectedTurn;
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
                onClick={() => onSelectTurn(m.turnNumber)}
                title={`${m.refName} (+${m.addedLines}/-${m.deletedLines} LoC, ${m.testShieldStatus.toUpperCase()})`}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 5,
                  padding: '3px 9px',
                  borderRadius: 6,
                  cursor: 'pointer',
                  fontFamily: "var(--font-display, 'Cinzel', 'Palatino', Georgia, serif)",
                  fontSize: 10,
                  fontWeight: 800,
                  background: isSelected
                    ? 'var(--color-bg-elevated)'
                    : 'var(--color-bg-surface)',
                  border: isSelected
                    ? '1.5px solid var(--color-accent-default)'
                    : isFail
                      ? '1px solid var(--color-status-fail)'
                      : '1px solid var(--color-border-subtle)',
                  color: 'var(--color-text-primary)',
                  boxShadow: isSelected
                    ? '0 0 10px var(--color-overlay-hover)'
                    : 'none',
                }}
              >
                <Icon.romanColumn style={{ width: 11, height: 11, color: isSelected ? 'var(--color-accent-default)' : 'var(--color-text-muted)' }} />
                <span>Step {m.turnNumber} ({m.romanNumeral})</span>
                <span
                  style={{
                    fontSize: 9,
                    color: isFail ? 'var(--color-status-fail)' : 'var(--color-status-pass)',
                  }}
                >
                  {isFail ? 'FAIL' : 'PASS'}
                </span>
              </button>
            </RomanTooltip>
          );
        })}
      </div>

      {/* Right: Selected Milestone Delta & 1-Click Rollback / Capture */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        {activeMilestone && (
          <RomanTooltip
            title="Commit Diff Summary"
            purpose={`Lines added: +${activeMilestone.addedLines}, lines removed: -${activeMilestone.deletedLines}, compute budget units: ${activeMilestone.tokenCostBu} BU.`}
          >
            <span
              data-testid="urbs-scrubber-delta-pill"
              style={{
                fontSize: 10,
                fontFamily: "var(--font-sans, 'Palatino', Georgia, serif)",
                color: 'var(--color-text-primary)',
                background: 'var(--color-bg-elevated)',
                border: '1px solid var(--color-border-subtle)',
                borderRadius: 5,
                padding: '2px 7px',
                whiteSpace: 'nowrap',
              }}
            >
              +{activeMilestone.addedLines}/-{activeMilestone.deletedLines} LoC ·{' '}
              {activeMilestone.tokenCostBu} BU
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
            style={{
              padding: '3px 9px',
              fontSize: 10,
              fontWeight: 700,
              borderRadius: 6,
              cursor: 'pointer',
              background: 'var(--color-bg-elevated)',
              border: '1px solid var(--color-border-strong)',
              color: 'var(--color-text-primary)',
              whiteSpace: 'nowrap',
            }}
          >
            + Save Checkpoint
          </button>
        </RomanTooltip>
        <RomanTooltip
          title={`Restore to Step ${selectedTurn}`}
          purpose="Reverts workspace files back to the selected commit checkpoint."
          actionHint={`Click to restore working tree to Step ${selectedTurn} (${toRomanNumeral(selectedTurn)})`}
        >
          <button
            type="button"
            data-testid="urbs-scrubber-rollback-btn"
            disabled={busyRestore}
            onClick={() => onRollbackToTurn(selectedTurn)}
            style={{
              padding: '3px 9px',
              fontSize: 10,
              fontWeight: 700,
              borderRadius: 6,
              cursor: 'pointer',
              background: 'var(--color-bg-elevated)',
              border: '1px solid var(--color-status-fail)',
              color: 'var(--color-status-fail)',
              whiteSpace: 'nowrap',
            }}
          >
            Restore Step {selectedTurn} ({toRomanNumeral(selectedTurn)})
          </button>
        </RomanTooltip>
      </div>
    </div>
  );
};
