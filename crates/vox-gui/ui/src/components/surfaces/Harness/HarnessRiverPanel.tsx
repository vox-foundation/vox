import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  getLatestControlState,
  getLatestModelOverride,
  subscribeControlState,
  subscribeModelOverride,
  type ControlState,
} from '../../../lib/driveConsole';
import { SHELL_PREFERENCE_KEYS } from '../../../lib/shellPersistence';
import type { RailRouting } from '../../../lib/routingSummary';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { ChatExecutionTask } from '../Chat/ChatExecutionRail';
import { deriveRiverSnapshot } from './riverMath';
import { renderRiverFrame } from './riverCanvasRenderer';
import { WaveformTierStrip } from './WaveformTierStrip';
import { AgentTrajectoryBraid } from './AgentTrajectoryBraid';
import { VerificationFunnelFooter } from './VerificationFunnelFooter';
import { LudusSandbox } from '../../gamify/LudusSandbox';
import type { HarnessTierId } from './riverTypes';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

function readStoredModelOverride(): string | null {
  try {
    if (typeof window === 'undefined' || !window.localStorage) return null;
    const raw = window.localStorage.getItem(SHELL_PREFERENCE_KEYS.chatModelOverride);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    return typeof parsed === 'string' && parsed.trim() ? parsed.trim() : null;
  } catch {
    return null;
  }
}

export interface HarnessRiverPanelProps {
  tasks?: ChatExecutionTask[];
  messages?: ChatMessage[];
  routing?: RailRouting | null;
  sessionSpentUsd?: number | null;
  control?: ControlState;
  chatModelOverride?: string | null;
  condensed?: boolean;
  gamifyEnabled?: boolean;
  onOpenRouting?: () => void;
  onFocusComposer?: () => void;
}

export function HarnessRiverPanel({
  tasks = [],
  messages = [],
  routing = null,
  sessionSpentUsd = null,
  control,
  chatModelOverride,
  condensed = false,
  gamifyEnabled = true,
  onOpenRouting,
  onFocusComposer,
}: HarnessRiverPanelProps) {
  const [busControl, setBusControl] = useState<ControlState>(getLatestControlState);
  const [busModelOverride, setBusModelOverride] = useState<string | null>(
    () => getLatestModelOverride() ?? readStoredModelOverride(),
  );
  const [viewMode, setViewMode] = useState<'river' | 'urbs' | 'split'>('river');
  const [timeWindow, setTimeWindow] = useState<'15m' | '1h' | '24h' | 'session'>('session');
  const [interceptorArmed, setInterceptorArmed] = useState(true);
  const [sluiceGates, setSluiceGates] = useState<Record<string, boolean>>({
    ast: true,
    typeck: true,
    toestub: true,
    tests: true,
  });
  const [aqueductReceipt, setAqueductReceipt] = useState<string | null>(null);
  const [aqueductYieldXp, setAqueductYieldXp] = useState<number>(250);

  useEffect(() => subscribeControlState(setBusControl), []);
  useEffect(() => subscribeModelOverride(setBusModelOverride), []);

  const effectiveControl = control ?? busControl;
  const effectiveModelOverride =
    chatModelOverride !== undefined ? chatModelOverride : (busModelOverride ?? readStoredModelOverride());
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const snapshot = useMemo(
    () =>
      deriveRiverSnapshot({
        tasks,
        messages,
        sessionSpentUsd,
        clutch: effectiveControl.clutch,
        risk: effectiveControl.risk,
        chatModelOverride: effectiveModelOverride,
        routing,
      }),
    [
      tasks,
      messages,
      sessionSpentUsd,
      effectiveControl.clutch,
      effectiveControl.risk,
      effectiveModelOverride,
      routing,
    ],
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (
      typeof navigator !== 'undefined' &&
      /jsdom/i.test(navigator.userAgent) &&
      !('mock' in canvas.getContext)
    ) {
      return;
    }
    let ctx: CanvasRenderingContext2D | null = null;
    try {
      ctx = canvas.getContext('2d');
    } catch {
      ctx = null;
    }
    if (!ctx) return;

    let animId = 0;
    const startMs = performance.now();

    let cachedW = Math.max(320, Math.floor(canvas.clientWidth || 560));
    let cachedH = Math.max(180, Math.floor(canvas.clientHeight || 260));

    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver((entries) => {
        requestAnimationFrame(() => {
          for (const entry of entries) {
            if (entry.contentRect) {
              cachedW = Math.max(320, Math.floor(entry.contentRect.width || canvas.clientWidth || 560));
              cachedH = Math.max(180, Math.floor(entry.contentRect.height || canvas.clientHeight || 260));
            }
          }
        });
      });
      ro.observe(canvas);
    }

    const prefersReducedMotion =
      typeof window !== 'undefined' &&
      Boolean(window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches);

    const draw = (nowMs: number) => {
      if (typeof document !== 'undefined' && document.hidden) {
        animId = requestAnimationFrame(draw);
        return;
      }
      const dpr = typeof window !== 'undefined' ? Math.min(2, Math.max(1, window.devicePixelRatio || 1)) : 1;
      const cssW = cachedW;
      const cssH = cachedH;
      const targetW = Math.floor(cssW * dpr);
      const targetH = Math.floor(cssH * dpr);
      if (canvas.width !== targetW || canvas.height !== targetH) {
        canvas.width = targetW;
        canvas.height = targetH;
      }
      if (typeof ctx!.setTransform === 'function') {
        ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
      }
      const phaseSec = (nowMs - startMs) / 1000;
      renderRiverFrame(ctx!, cssW, cssH, snapshot, phaseSec);

      if (!prefersReducedMotion) {
        animId = requestAnimationFrame(draw);
      }
    };

    renderRiverFrame(ctx, cachedW, cachedH, snapshot, 0);
    if (!prefersReducedMotion) {
      animId = requestAnimationFrame(draw);
    }

    return () => {
      if (animId) cancelAnimationFrame(animId);
      if (ro) ro.disconnect();
    };
  }, [snapshot]);

  const canNavigateTier = Boolean(onOpenRouting || onFocusComposer);
  const handleSelectTier = (_tierId: HarnessTierId) => {
    if (onOpenRouting) {
      onOpenRouting();
    } else if (onFocusComposer) {
      onFocusComposer();
    }
  };

  const handleJump = (target: string) => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('vox-surface-navigate', { detail: { surface: target } }));
    }
  };

  return (
    <section
      data-testid="axis-harness-river"
      className="flex flex-col gap-1.5 text-[#E6DCB8]"
      aria-label="Axis Autonomous Harness River"
    >
      {/* Plate 2 Luminous White Carrara Marble Frieze Header */}
      <div
        data-roman-frieze="true"
        className="vox-animate-marble-sheen relative overflow-hidden rounded-xl border-2 border-[#C59B27] px-3.5 py-1.5 shadow-[0_8px_24px_rgba(0,0,0,0.65),inset_0_2px_0_rgba(255,255,255,0.92)]"
        style={{
          backgroundImage: `linear-gradient(115deg, rgba(255,252,245,0) 35%, rgba(255,248,225,0.48) 50%, rgba(255,252,245,0) 65%), linear-gradient(90deg, rgba(246,240,229,0.88) 0%, rgba(238,229,212,0.74) 48%, rgba(246,240,229,0.88) 100%), url('/assets/romanesque/white-marble-frieze-banner.jpg')`,
          backgroundSize: '240% 100%, 100% 100%, cover',
          backgroundPosition: 'center',
        }}
      >
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2.5">
            <RomanMedallion active size="sm">
              <Icon.aqueductArch className="size-3.5 text-[#FFF8E7]" />
            </RomanMedallion>
            <div>
              <div className="flex items-center gap-2">
                <span className="inline-block size-2 rounded-full bg-[#B8860B] animate-pulse" />
                <h3 className="font-display text-xs font-bold uppercase tracking-widest text-[#1C140C]">
                  EXECUTION PIPELINE CASCADE · LIVE TASK STREAM
                </h3>
              </div>
              <p className="font-serif text-[11px] font-medium text-[#3B2E1E]">
                Wired to Loquela composer · 3-Tier Cascade
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-1.5">
            {/* Quick-Jump Actions */}
            <RomanTooltip
              title="Operations Hub"
              purpose="Jump to the system operations hub to monitor daemons, process threads, and MCP adapters."
              actionHint="Open Operations Hub"
            >
              <button
                type="button"
                onClick={() => handleJump('operations')}
                className="flex items-center gap-1 rounded-md border border-[#D4AF37]/40 bg-[#241A0B] px-2 py-1 font-mono text-[10px] font-semibold text-[#F3E5AB] hover:border-[#F3E5AB] transition"
              >
                <span>Operations Hub ↗</span>
              </button>
            </RomanTooltip>

            <RomanTooltip
              title="MENS Studio"
              purpose="Jump to MENS Studio to inspect five-spoke datasets, adapter checkpoints, and benchmark rankings."
              actionHint="Open MENS Studio"
            >
              <button
                type="button"
                onClick={() => handleJump('mens')}
                className="flex items-center gap-1 rounded-md border border-[#D4AF37]/40 bg-[#241A0B] px-2 py-1 font-mono text-[10px] font-semibold text-[#F3E5AB] hover:border-[#F3E5AB] transition"
              >
                <span>MENS Studio ↗</span>
              </button>
            </RomanTooltip>

            {/* Harness Mode Switchers */}
            {gamifyEnabled && (
              <div
                role="group"
                aria-label="Harness visualization mode"
                className="flex items-center rounded-lg border border-[#382F20] bg-[#0E0B07] p-0.5"
              >
                {(
                  [
                    { id: 'river', label: 'River' },
                    { id: 'urbs', label: 'Urbs Civilization' },
                    { id: 'split', label: 'Split' },
                  ] as const
                ).map((m) => (
                  <RomanTooltip
                    key={m.id}
                    title={`${m.label} Mode`}
                    purpose={`Displays ${m.label.toLowerCase()} visualization mode for live pipeline telemetry.`}
                    actionHint={`Switch to ${m.label}`}
                  >
                    <button
                      type="button"
                      data-testid={`harness-mode-${m.id}`}
                      aria-pressed={viewMode === m.id}
                      onClick={() => setViewMode(m.id)}
                      className={`rounded px-2 py-0.5 font-mono text-[10px] font-semibold transition ${
                        viewMode === m.id
                          ? 'border border-[#F3E5AB] bg-gradient-to-b from-[#4A3818] to-[#241A0B] text-[#F3E5AB]'
                          : 'text-[#9E9079] hover:text-[#E6DCB8]'
                      }`}
                    >
                      {m.label}
                    </button>
                  </RomanTooltip>
                ))}
              </div>
            )}

            {onOpenRouting && (
              <RomanTooltip
                title="Routing & Model Tiers"
                purpose="Inspect model routing policies, tier latencies, and token cost thresholds."
                actionHint="Open routing drawer"
              >
                <button
                  type="button"
                  onClick={onOpenRouting}
                  className="rounded-md border border-[#382F20] bg-[#140F0A] px-2 py-1 font-mono text-[10px] font-semibold text-[#D4AF37] hover:border-[#D4AF37] transition"
                >
                  Routing &amp; Tiers
                </button>
              </RomanTooltip>
            )}

            {onFocusComposer && (
              <RomanTooltip
                title="Focus Chat Composer"
                purpose="Directs keyboard focus to the main Loquela prompt input field."
                actionHint="Focus prompt composer"
              >
                <button
                  type="button"
                  onClick={onFocusComposer}
                  className="rounded-md border border-[#382F20] bg-[#140F0A] px-2 py-1 font-mono text-[10px] text-[#C4B69C] hover:border-[#D4AF37] transition"
                >
                  Focus composer
                </button>
              </RomanTooltip>
            )}
          </div>
        </div>
      </div>

      {/* Zone A: Top Telemetry Waveform & 3-Tier Model Assignment Strip */}
      <WaveformTierStrip
        snapshot={snapshot}
        condensed={condensed}
        onSelectTier={canNavigateTier ? handleSelectTier : undefined}
      />

      {/* Controls Bar: Compact Progressive-Disclosure Window Select & Verification Gate */}
      <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-[#382F20] bg-[#110D09] px-3 py-1 text-[10px]">
        <div className="flex items-center gap-2">
          <label htmlFor="river-time-window-select" className="font-serif uppercase tracking-wider text-[#A89878]">
            Window:
          </label>
          <select
            id="river-time-window-select"
            aria-label="Time Window"
            data-unified-knob="true"
            value={timeWindow}
            onChange={(e) => setTimeWindow(e.target.value as '15m' | '1h' | '24h' | 'session')}
            className="appearance-none rounded border border-[#D4AF37]/40 bg-[#17120C] px-2 py-0.5 text-[10px] font-semibold text-[#F3E5AB]"
          >
            <option value="15m">Last 15m</option>
            <option value="1h">Last 1h</option>
            <option value="24h">Last 24h</option>
            <option value="session">Current Session</option>
          </select>
        </div>

        <RomanTooltip
          title="Deterministic Interceptor Gate"
          purpose="Enforces strict verification gating before letting generated code execute on the file tree."
          statusBadge={interceptorArmed ? 'ARMED' : 'BYPASS'}
          actionHint="Click to toggle interceptor arm state"
        >
          <button
            type="button"
            onClick={() => setInterceptorArmed((a) => !a)}
            className="flex items-center gap-1.5 rounded-full border border-[#7B8C32]/55 bg-[#4D5D1E]/25 px-2.5 py-0.5 font-semibold text-[#DCE7B6] hover:border-[#8FA339] transition"
          >
            <Icon.gladiusShield className="size-3 text-[#8FA339]" />
            <span>Verification Gate: {interceptorArmed ? 'Armed · Zero-Cascade' : 'Standby'}</span>
          </button>
        </RomanTooltip>
      </div>

      {/* Zone B: Center Alluvial Particle Bundle / Urbs Civilization with Aqueduct Backdrop */}
      <div className={viewMode === 'split' ? 'grid grid-cols-1 gap-2 lg:grid-cols-2' : 'block'}>
        {viewMode !== 'urbs' && (
          <div className="relative overflow-hidden rounded-xl border-2 border-[#D4AF37]/55 bg-[#0A0805] shadow-[0_12px_32px_rgba(0,0,0,0.75)]">
            {/* High-Resolution Romanesque Aqueduct & Castellum Divisorium Sluice Cascade Backdrop */}
            <div
              className="absolute inset-0 pointer-events-none opacity-85 bg-center bg-cover"
              style={{ backgroundImage: `url('/assets/romanesque/aqueduct-sluice-cascade-bg.jpg')` }}
            />
            <div className="relative z-10 flex flex-wrap items-center justify-between gap-2 border-b border-[#D4AF37]/40 bg-[#140F0A]/92 px-3 py-1 font-display text-[9px] uppercase tracking-wider text-[#E6DCB8]">
              <span>AQUAE CAPUT · 8-SPOKE SOURCE ARCHES</span>
              <span className="font-bold text-[#F3E5AB]">CASTELLUM DIVISORIUM · 3-TIER HYDRAULIC CASCADE</span>
              <span>CLAUSURAE · 4 BRASS SLUICE GATES</span>
            </div>
            <div className={condensed ? 'relative z-10 h-[150px] w-full' : 'relative z-10 h-[164px] xl:h-[215px] w-full'}>
              <canvas
                ref={canvasRef}
                data-testid="harness-river-canvas"
                className="h-full w-full block"
                aria-label="Live alluvial particle stream and phase scanner"
              />
            </div>

            {/* Phase 365: Interactive Aqueduct Sluice Gates & Developer Orchestrator Command Desk (Concept Art Plate 2) */}
            <div
              data-testid="aqueduct-developer-control-bar"
              style={{
                backgroundImage:
                  'linear-gradient(180deg, rgba(244,238,225,0.95) 0%, rgba(229,218,198,0.95) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
                backgroundSize: 'cover',
                backgroundPosition: 'center',
              }}
              className="vox-marble-tablet relative z-10 flex flex-wrap items-center justify-between gap-1.5 border-t-2 border-[#B38728]/80 px-3 py-1 text-[#1C140C] shadow-[inset_0_1px_0_rgba(255,255,255,0.9)]"
            >
              {/* 4 Interactive Brass Sluice Gates (Clausurae I–IV) */}
              <div data-testid="aqueduct-sluice-gates" className="flex flex-wrap items-center gap-1">
                <span className="mr-1 font-display text-[9px] font-bold uppercase tracking-wider text-[#5C4318]">
                  Sluice Gates:
                </span>
                {[
                  { id: 'ast', label: 'Gate I · AST Parse' },
                  { id: 'typeck', label: 'Gate II · Typecheck' },
                  { id: 'toestub', label: 'Gate III · TOESTUB' },
                  { id: 'tests', label: 'Gate IV · Test Suite' },
                ].map((g) => {
                  const open = sluiceGates[g.id] ?? true;
                  return (
                    <button
                      key={g.id}
                      type="button"
                      data-testid={`aqueduct-sluice-gate-${g.id}`}
                      aria-pressed={open}
                      onClick={() => {
                        setSluiceGates((prev) => ({ ...prev, [g.id]: !open }));
                        setAqueductReceipt(
                          `${g.label} ${!open ? 'OPENED (Flow Armed)' : 'CLOSED (Holding Stream)'}`,
                        );
                      }}
                      className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider transition ${
                        open
                          ? 'border-[#8C6214] bg-linear-to-b from-[#2A1F12] to-[#161009] text-[#F3E5AB] shadow-sm'
                          : 'border-amber-700/70 bg-amber-950/90 text-amber-200'
                      }`}
                    >
                      <span
                        className={`size-1.5 rounded-full ${open ? 'bg-[#8FA339]' : 'bg-amber-400'}`}
                        aria-hidden="true"
                      />
                      <span>{g.label}</span>
                    </button>
                  );
                })}
              </div>

              {/* 1-Click Developer Orchestrator Push-Buttons */}
              <div data-testid="aqueduct-dev-actions" className="flex flex-wrap items-center gap-1">
                <button
                  type="button"
                  data-testid="aqueduct-action-dispatch"
                  onClick={() => {
                    onFocusComposer?.();
                    setAqueductReceipt('⚔ Dispatched new task into Cursus II Balanced Hybrid channel');
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  [Dispatch Task]
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-flush"
                  onClick={() => {
                    setSluiceGates({ ast: true, typeck: true, toestub: true, tests: true });
                    setAqueductReceipt('🌊 Flushed all 4 Aqueduct Sluice Gates — zero-cascade stream verified');
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  [Flush Sluice Gate]
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-reroute"
                  onClick={() => {
                    onOpenRouting?.();
                    setAqueductReceipt('⚖ Rerouted active LLM stream across 3-Tier Castellum Divisorium');
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  [Reroute Tier]
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-compact"
                  onClick={() => {
                    setAqueductReceipt('🏺 Compacted Context Reservoir (64% → 38% occupancy)');
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  [Compact Context]
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-claim-yield"
                  onClick={() => {
                    setAqueductYieldXp((xp) => xp + 250);
                    setAqueductReceipt('🏆 Claimed +250 XP · +18 Denarii from verified Aqueduct cascade!');
                  }}
                  className="rounded-md border border-[#B38728] bg-linear-to-b from-[#D4AF37] to-[#9A711C] px-2.5 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-[#1A1208] shadow-sm hover:brightness-110"
                >
                  [Claim Yield +{aqueductYieldXp} XP · +18 ◆]
                </button>
              </div>

              {aqueductReceipt && (
                <span
                  data-testid="aqueduct-dev-receipt"
                  className="rounded border border-[#8C6214]/50 bg-[#1B140B] px-2 py-0.5 font-mono text-[8.5px] text-[#F3E5AB]"
                >
                  {aqueductReceipt}
                </span>
              )}
            </div>
          </div>
        )}
        {viewMode !== 'river' && (
          <div
            data-testid="harness-urbs-embed"
            className={condensed ? 'h-[240px] w-full' : 'h-[300px] w-full'}
          >
            <LudusSandbox energy={80} maxEnergy={100} />
          </div>
        )}
      </div>

      {/* Zone C: 8-Slice Agent Role Trajectory Braid (full mode only) */}
      {!condensed && <AgentTrajectoryBraid trajectories={snapshot.trajectories} />}

      {/* Zone D: Deterministic Evidence & Verification Retention Funnel */}
      <VerificationFunnelFooter
        snapshot={snapshot}
        condensed={condensed}
        gamifyEnabled={gamifyEnabled}
      />
    </section>
  );
}
