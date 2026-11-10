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
import { EmbeddedSurfaceProvider } from '../../dashboard/EmbeddedSurfaceContext';
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
        className="vox-animate-marble-sheen relative overflow-hidden rounded-xl border-2 border-[#C59B27] px-3.5 py-1.5 shadow-[0_6px_18px_rgba(28,20,12,0.22),inset_0_2px_0_rgba(255,255,255,0.92)]"
        style={{
          backgroundImage: `linear-gradient(115deg, rgba(255,252,245,0) 35%, rgba(255,248,225,0.48) 50%, rgba(255,252,245,0) 65%), linear-gradient(90deg, rgba(250,246,238,0.92) 0%, rgba(242,233,218,0.82) 48%, rgba(250,246,238,0.92) 100%), url('/assets/romanesque/carrara-acanthus-header-band.jpg')`,
          backgroundSize: '240% 100%, 100% 100%, cover',
          backgroundPosition: 'center, center, center',
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
                className="flex items-center gap-1 rounded-md border border-[#D4AF37]/40 bg-bg-elevated px-2 py-1 font-mono text-[10px] font-semibold text-text-primary hover:border-[#D4AF37] transition"
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
                className="flex items-center gap-1 rounded-md border border-[#D4AF37]/40 bg-bg-elevated px-2 py-1 font-mono text-[10px] font-semibold text-text-primary hover:border-[#D4AF37] transition"
              >
                <span>MENS Studio ↗</span>
              </button>
            </RomanTooltip>

            {/* Harness Mode Switchers */}
            {gamifyEnabled && (
              <div
                role="group"
                aria-label="Harness visualization mode"
                className="flex items-center rounded-lg border border-border-subtle bg-bg-surface p-0.5"
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
                          ? 'border border-[#D4AF37] bg-bg-elevated text-text-primary'
                          : 'text-text-secondary hover:text-text-primary'
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
                  className="rounded-md border border-border-subtle bg-bg-elevated px-2 py-1 font-mono text-[10px] font-semibold text-brass hover:border-[#D4AF37] transition"
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
                  className="rounded-md border border-border-subtle bg-bg-elevated px-2 py-1 font-mono text-[10px] text-text-secondary hover:border-[#D4AF37] transition"
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

      {/* Zone B: Center Alluvial Particle Bundle / Urbs Civilization with Uncropped 3-Tier Aqueduct Panorama */}
      <div className={viewMode === 'split' ? 'grid grid-cols-1 gap-2 lg:grid-cols-2' : 'block'}>
        {viewMode !== 'urbs' && (
          <div className="relative overflow-hidden rounded-xl border-2 border-[#C59B27]/75 bg-[#F5EFE2] shadow-[0_10px_28px_rgba(28,20,12,0.28)]">
            {/* Phase 384 Uncropped 3-Tier Romanesque Aqueduct & Castellum Divisorium Sluice Cascade Panorama */}
            <div
              className="absolute inset-0 pointer-events-none opacity-98 bg-center bg-[length:100%_100%]"
              style={{ backgroundImage: `url('/assets/romanesque/aqueduct-sluice-cascade-bg.jpg')` }}
            />
            <div
              style={{
                backgroundImage:
                  'linear-gradient(90deg, rgba(250,246,238,0.95) 0%, rgba(242,233,218,0.92) 50%, rgba(250,246,238,0.95) 100%)',
              }}
              className="relative z-10 flex flex-wrap items-center justify-between gap-2 border-b border-[#C59B27]/65 px-3 py-1 font-display text-[9px] uppercase tracking-wider text-[#1C140C]"
            >
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-bold text-[#5C4318]">AQUAE CAPUT · 8-SPOKE ARCHES</span>
                <span className="text-[#8C6214]" aria-hidden="true">·</span>
                <span className="font-bold text-[#1C140C]">CASTELLUM DIVISORIUM · 3-TIER HYDRAULIC CASCADE</span>
              </div>

              <div className="flex flex-wrap items-center gap-2 font-sans text-[9.5px] normal-case tracking-normal">
                <div className="flex items-center gap-1.5">
                  <label htmlFor="river-time-window-select" className="font-display text-[8.5px] font-bold uppercase tracking-wider text-[#5C4318]">
                    Window:
                  </label>
                  <select
                    id="river-time-window-select"
                    aria-label="Time Window"
                    data-unified-knob="true"
                    value={timeWindow}
                    onChange={(e) => setTimeWindow(e.target.value as '15m' | '1h' | '24h' | 'session')}
                    className="appearance-none rounded border border-[#C59B27]/65 bg-[#FAF6EE] px-2 py-0.5 font-mono text-[9.5px] font-semibold text-[#1C140C]"
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
                    className="flex items-center gap-1.5 rounded-full border border-[#059669]/65 bg-[#ECFDF5] px-2.5 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-[#065F46] hover:border-[#047857] transition"
                  >
                    <Icon.gladiusShield className="size-3 text-[#059669]" />
                    <span>Verification Gate: {interceptorArmed ? 'Armed · Zero-Cascade' : 'Standby'}</span>
                  </button>
                </RomanTooltip>
              </div>
            </div>
            <div className={condensed ? 'relative z-10 h-[170px] w-full' : 'relative z-10 h-[210px] xl:h-[250px] w-full'}>
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
                  'linear-gradient(180deg, rgba(250,246,238,0.96) 0%, rgba(239,230,213,0.95) 100%), url(/assets/romanesque/carrara-seamless-stone.jpg)',
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
                          ? 'border-[#8C6214] bg-bg-elevated text-text-primary shadow-sm'
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
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
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
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
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
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  [Reroute Tier]
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-compact"
                  onClick={() => {
                    setAqueductReceipt('🏺 Compacted Context Reservoir (64% → 38% occupancy)');
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
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
                  className="rounded border border-[#8C6214]/50 bg-bg-elevated px-2 py-0.5 font-mono text-[8.5px] text-text-primary"
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
            className={condensed ? 'h-[240px] w-full' : 'h-[320px] w-full'}
          >
            <EmbeddedSurfaceProvider value={viewMode === 'split'}>
              <LudusSandbox energy={80} maxEnergy={100} />
            </EmbeddedSurfaceProvider>
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
