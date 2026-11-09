import React from 'react';
import { RomanTooltip, RomanBrassGauge as BaseBrassGauge } from '../ui/RomanTooltip';
import { Icon } from '../ui/Icons';

export interface RomanBrassGaugeProps {
  label: string;
  valueText?: string;
  value?: string;
  ratio?: number;
  percent?: number;
  tone?: 'gold' | 'emerald' | 'crimson';
  accent?: 'gold' | 'emerald' | 'amber' | 'crimson';
  purpose?: string;
  onClick?: () => void;
}

/**
 * Animated Semicircular Brass Gauge matching Concept Art Plate 2 dials.
 */
export const RomanBrassGauge: React.FC<RomanBrassGaugeProps> = ({
  label,
  valueText,
  value,
  ratio,
  percent,
  tone,
  accent,
  purpose,
  onClick,
}) => {
  const displayVal = valueText ?? value ?? '';
  const effRatio =
    ratio !== undefined
      ? ratio
      : percent !== undefined
        ? percent / 100
        : 0.5;
  const effTone =
    tone ?? (accent === 'emerald' ? 'emerald' : accent === 'crimson' ? 'crimson' : 'gold');

  return (
    <BaseBrassGauge
      label={label}
      valueText={displayVal}
      ratio={effRatio}
      tone={effTone}
      purpose={purpose}
      onClick={onClick}
    />
  );
};

export interface HudPanelsProps {
  /** Real LLM spend USD (from get_llm_spend); null = unknown → render "—". */
  treasuryUsd?: number | null;
  spendUsd?: number | null;
  energy: number;
  maxEnergy: number;
  /** Animation speed multiplier: 0 = paused. View-only; not a simulation. */
  speed?: number;
  simSpeed?: number;
  simPaused?: boolean;
  onSetSpeed: (speed: number) => void;
  embedded?: boolean;
  onNavigate?: (viewKey: string) => void;
}

export const HudPanels: React.FC<HudPanelsProps> = ({
  treasuryUsd,
  spendUsd,
  energy,
  maxEnergy,
  speed,
  simSpeed,
  simPaused = false,
  onSetSpeed,
  embedded = false,
  onNavigate,
}) => {
  const rawSpend = spendUsd !== undefined ? spendUsd : (treasuryUsd ?? null);
  const spendText = rawSpend === null ? '—' : `$${rawSpend.toFixed(2)}`;
  const currentSpeed = simPaused ? 0 : (simSpeed !== undefined ? simSpeed : (speed ?? 1));
  const isPaused = currentSpeed === 0;

  if (embedded) {
    return (
      <div
        data-testid="urbs-hud-embedded"
        className="flex items-center gap-2 rounded-lg border border-[#D4AF37]/50 bg-linear-to-r from-[#18130D]/95 via-[#1F1810]/95 to-[#18130D]/95 px-2.5 py-1 text-[10px] text-[#F3E5AB] pointer-events-auto select-none shadow-[0_6px_18px_rgba(0,0,0,0.75)]"
      >
        <RomanTooltip
          title="Session Budget Spend"
          purpose="Live cumulative LLM token spend across cloud and local MENS inference."
          actionHint="Click to open Cost & Token Analytics"
        >
          <button
            type="button"
            onClick={() => onNavigate?.('vox-cost-analytics')}
            className="flex items-center gap-1.5 border-r border-[#D4AF37]/25 pr-2 hover:text-white transition"
          >
            <Icon.denariusCoin className="size-3 text-[#D4AF37]" aria-hidden="true" />
            <span className="text-[#C8B99E] font-display text-[9px] uppercase tracking-wider">Budget</span>
            <span data-testid="hud-value" className="text-[#DCE7B6] font-bold font-mono text-[10.5px]">
              {spendText}
            </span>
          </button>
        </RomanTooltip>

        <RomanTooltip
          title="Agent Capacity & Energy"
          purpose="Remaining execution capacity before attention budget throttling activates."
          actionHint="Click to inspect Arena & Agent Capacity"
        >
          <button
            type="button"
            onClick={() => onNavigate?.('vox-Ludus-view')}
            className="flex items-center gap-1.5 border-r border-[#D4AF37]/25 pr-2 hover:text-white transition"
          >
            <Icon.brazierTorch className="size-3 text-amber-400" aria-hidden="true" />
            <span className="text-[#C8B99E] font-display text-[9px] uppercase tracking-wider">Energy</span>
            <span data-testid="hud-energy" className="text-[#F3E5AB] font-bold font-mono text-[10.5px]">
              {energy}/{maxEnergy}
            </span>
          </button>
        </RomanTooltip>

        <div className="flex items-center gap-1">
          <span className="text-[#C8B99E] font-display text-[9px] uppercase tracking-wider">Speed</span>
          <div className="flex items-center gap-0.5">
            {[0, 1, 3].map((s) => (
              <RomanTooltip
                key={s}
                title={s === 0 ? 'Pause simulation' : `${s}x speed`}
                purpose={s === 0 ? 'Pause map animations and particle flow.' : `Run workspace map activity animations at ${s}x speed.`}
                actionHint={`Click to set animation speed to ${s}x`}
              >
                <button
                  type="button"
                  title={s === 0 ? 'Pause simulation' : `${s}x speed`}
                  onClick={() => onSetSpeed(s)}
                  className={`px-1.5 py-0.5 rounded font-mono text-[9.5px] transition-colors ${
                    currentSpeed === s
                      ? 'border border-[#D4AF37] bg-[#3A2C14] text-[#F3E5AB] font-semibold'
                      : 'border border-[#3A3022] bg-[#120E09] text-[#B8A88A] hover:border-[#D4AF37]/50 hover:text-[#F3E5AB]'
                  }`}
                >
                  {s}x
                </button>
              </RomanTooltip>
            ))}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="urbs-stone-telemetry-console"
      style={{
        backgroundImage:
          'linear-gradient(180deg, rgba(28,22,14,0.96) 0%, rgba(14,11,7,0.98) 100%), url(/assets/romanesque/marble-frieze-banner.jpg)',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}
      className="flex flex-nowrap items-center gap-1.5 rounded-xl border-2 border-[#D4AF37]/65 px-2 py-0.5 text-xs text-[#F3E5AB] pointer-events-auto select-none whitespace-nowrap shadow-[0_12px_32px_rgba(0,0,0,0.88),inset_0_1px_0_rgba(243,229,171,0.3)]"
    >
      {/* Gauge 1: Budget Spend */}
      <div className="relative">
        <RomanTooltip
          title="Session Budget Spend"
          purpose="Live cumulative LLM token spend across cloud providers and local MENS models."
          actionHint="Click to open Cost & Token Analytics"
        >
          <div>
            <RomanBrassGauge
              label="Budget"
              value={spendText}
              valueText={spendText}
              percent={rawSpend === null ? 12 : Math.min(100, Math.max(12, rawSpend * 20))}
              accent="gold"
              onClick={() => onNavigate?.('vox-cost-analytics')}
            />
          </div>
        </RomanTooltip>
        <span data-testid="hud-value" className="sr-only">
          {spendText}
        </span>
      </div>

      {/* Column divider I: subtle carved stone pillar with Roman numeral I */}
      <div
        data-testid="stone-column-divider-1"
        className="flex flex-col items-center justify-center px-1.5 py-0.5 border-x border-[#D4AF37]/30 bg-gradient-to-b from-[#241E15]/70 to-[#120F0A]/70 rounded-xs select-none"
        aria-hidden="true"
      >
        <span className="font-serif text-[6.5px] text-[#8C7A5E]">▲</span>
        <span className="font-display text-[9px] font-bold text-[#D4AF37]/70 tracking-widest">I</span>
        <span className="font-serif text-[6.5px] text-[#8C7A5E]">▼</span>
      </div>

      {/* Gauge 2: Energy Capacity */}
      <div className="relative">
        <RomanTooltip
          title="Agent Capacity & Energy"
          purpose="Available execution stamina and concurrency pool for autonomous agent work before throttling."
          actionHint="Click to inspect Arena & Agent Capacity"
        >
          <div>
            <RomanBrassGauge
              label="Energy"
              value={`${energy}/${maxEnergy}`}
              valueText={`${energy}/${maxEnergy}`}
              percent={maxEnergy > 0 ? (energy / maxEnergy) * 100 : 80}
              accent="emerald"
              tone="emerald"
              onClick={() => onNavigate?.('vox-operations-hub')}
            />
          </div>
        </RomanTooltip>
        <span data-testid="hud-energy" className="sr-only">
          {energy}/{maxEnergy}
        </span>
      </div>

      {/* Column divider III: subtle carved stone pillar with Roman numeral III */}
      <div
        data-testid="stone-column-divider-3"
        className="flex flex-col items-center justify-center px-1.5 py-0.5 border-x border-[#D4AF37]/30 bg-gradient-to-b from-[#241E15]/70 to-[#120F0A]/70 rounded-xs select-none"
        aria-hidden="true"
      >
        <span className="font-serif text-[6.5px] text-[#8C7A5E]">▲</span>
        <span className="font-display text-[9px] font-bold text-[#D4AF37]/70 tracking-widest">III</span>
        <span className="font-serif text-[6.5px] text-[#8C7A5E]">▼</span>
      </div>

      {/* Gauge 3: Speed dial with compact 0x, 1x, 3x buttons */}
      <div className="flex items-center gap-2">
        <RomanTooltip
          title="Simulation Clock Speed"
          purpose="Simulation clock speed for architectural animations, citizen movement, and hydraulic particle flows."
          actionHint="Use speed buttons beside dial to adjust rate"
        >
          <div>
            <RomanBrassGauge
              label="Speed"
              value={isPaused ? '0x' : `${currentSpeed}x`}
              valueText={isPaused ? '0x' : `${currentSpeed}x`}
              percent={isPaused ? 5 : currentSpeed === 1 ? 50 : 95}
              accent="amber"
            />
          </div>
        </RomanTooltip>
        <div className="flex flex-col gap-0.5">
          <span className="font-display text-[7.5px] uppercase tracking-wider text-[#A89878]">Clock</span>
          <div className="flex items-center gap-1">
            {[0, 1, 3].map((s) => (
              <RomanTooltip
                key={s}
                title={s === 0 ? 'Pause simulation' : `${s}x speed`}
                purpose={s === 0 ? 'Pause map animations and hydraulic flow.' : `Run live architectural map animations at ${s}x speed.`}
                actionHint={`Click to set speed to ${s}x`}
              >
                <button
                  type="button"
                  title={s === 0 ? 'Pause simulation' : `${s}x speed`}
                  onClick={() => onSetSpeed(s)}
                  className={`px-1.5 py-0.5 rounded font-mono text-[9.5px] transition-all ${
                    currentSpeed === s
                      ? 'border border-[#F3E5AB] bg-linear-to-b from-[#4A3818] to-[#241B0B] text-[#F3E5AB] font-bold shadow-[0_0_8px_rgba(212,175,55,0.4)]'
                      : 'border border-[#3A3022] bg-[#120E09] text-[#B8A88A] hover:border-[#D4AF37]/50 hover:text-[#F3E5AB]'
                  }`}
                >
                  {s}x
                </button>
              </RomanTooltip>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export const ResourceBar = HudPanels;
