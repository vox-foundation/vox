import React from 'react';

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
      aria-label="Via Temporis Milestone Road Time-Scrubber"
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 10,
        padding: '6px 12px',
        background:
          'linear-gradient(90deg, rgba(16, 13, 10, 0.95) 0%, rgba(26, 21, 15, 0.96) 50%, rgba(16, 13, 10, 0.95) 100%)',
        border: '1px solid rgba(212, 175, 55, 0.55)',
        borderRadius: 8,
        boxShadow: '0 8px 24px rgba(0, 0, 0, 0.65)',
        color: '#F5EFE0',
        fontSize: 11,
        flexWrap: 'wrap',
      }}
    >
      {/* Left Label: VIA TEMPORIS */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span
          style={{
            fontSize: 10,
            fontWeight: 800,
            letterSpacing: '0.08em',
            color: '#E6C259',
            padding: '2px 6px',
            borderRadius: 4,
            background: 'rgba(212, 175, 55, 0.16)',
            border: '1px solid rgba(212, 175, 55, 0.42)',
            whiteSpace: 'nowrap',
          }}
        >
          ⏳ VIA TEMPORIS · PORTA GIT
        </span>
        <span
          style={{
            fontSize: 10,
            color: '#C7BCA8',
            fontFamily: 'var(--font-mono, JetBrains Mono, monospace)',
            whiteSpace: 'nowrap',
          }}
        >
          Turn {selectedTurn}/{activeHeadTurn}
        </span>
      </div>

      {/* Center: Roman Stone Milestone Road & Sundial Scrubber */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          flex: 1,
          minWidth: 220,
          justifyContent: 'center',
        }}
      >
        {milestones.map((m) => {
          const isSelected = m.turnNumber === selectedTurn;
          const isFail = m.testShieldStatus === 'fail';
          return (
            <button
              key={m.turnNumber}
              type="button"
              data-testid={`urbs-milestone-stone-${m.turnNumber}`}
              onClick={() => onSelectTurn(m.turnNumber)}
              title={`${m.refName} (+${m.addedLines}/-${m.deletedLines} LoC, ${m.testShieldStatus.toUpperCase()})`}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 4,
                padding: '3px 8px',
                borderRadius: 6,
                cursor: 'pointer',
                fontFamily: 'var(--font-mono, JetBrains Mono, monospace)',
                fontSize: 10,
                fontWeight: 800,
                background: isSelected
                  ? 'linear-gradient(180deg, rgba(212, 175, 55, 0.38) 0%, rgba(161, 98, 7, 0.42) 100%)'
                  : 'rgba(30, 27, 22, 0.85)',
                border: isSelected
                  ? '1.5px solid #FACC15'
                  : isFail
                    ? '1px solid rgba(248, 113, 113, 0.7)'
                    : '1px solid rgba(16, 185, 129, 0.45)',
                color: isSelected ? '#FEF08A' : '#E7DEC8',
                boxShadow: isSelected
                  ? '0 0 10px rgba(250, 204, 21, 0.38)'
                  : 'none',
              }}
            >
              <span>{isSelected ? '☀' : '🪨'}</span>
              <span>{m.romanNumeral}</span>
              <span
                style={{
                  fontSize: 9,
                  color: isFail ? '#F87171' : '#34D399',
                }}
              >
                {isFail ? '✗' : '🛡'}
              </span>
            </button>
          );
        })}
      </div>

      {/* Right: Selected Milestone Delta & 1-Click Rollback / Capture */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        {activeMilestone && (
          <span
            data-testid="urbs-scrubber-delta-pill"
            style={{
              fontSize: 10,
              fontFamily: 'var(--font-mono, JetBrains Mono, monospace)',
              color: '#A7F3D0',
              background: 'rgba(16, 185, 129, 0.12)',
              border: '1px solid rgba(16, 185, 129, 0.35)',
              borderRadius: 4,
              padding: '2px 6px',
              whiteSpace: 'nowrap',
            }}
          >
            +{activeMilestone.addedLines}/-{activeMilestone.deletedLines} LoC ·{' '}
            {activeMilestone.tokenCostBu} BU
          </span>
        )}
        <button
          type="button"
          data-testid="urbs-scrubber-capture-btn"
          onClick={onCaptureCheckpoint}
          style={{
            padding: '3px 8px',
            fontSize: 10,
            fontWeight: 700,
            borderRadius: 5,
            cursor: 'pointer',
            background: 'rgba(212, 175, 55, 0.18)',
            border: '1px solid rgba(212, 175, 55, 0.55)',
            color: '#FDE68A',
            whiteSpace: 'nowrap',
          }}
        >
          + Milestone
        </button>
        <button
          type="button"
          data-testid="urbs-scrubber-rollback-btn"
          disabled={busyRestore}
          onClick={() => onRollbackToTurn(selectedTurn)}
          style={{
            padding: '3px 8px',
            fontSize: 10,
            fontWeight: 700,
            borderRadius: 5,
            cursor: 'pointer',
            background: 'rgba(239, 68, 68, 0.2)',
            border: '1px solid rgba(248, 113, 113, 0.65)',
            color: '#FECACA',
            whiteSpace: 'nowrap',
          }}
        >
          ↺ Rollback to {toRomanNumeral(selectedTurn)}
        </button>
      </div>
    </div>
  );
};
