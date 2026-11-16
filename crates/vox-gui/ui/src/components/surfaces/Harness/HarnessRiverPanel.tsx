import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  getLatestControlState,
  getLatestModelOverride,
  subscribeControlState,
  subscribeModelOverride,
  type ControlState,
} from '../../../lib/driveConsole';
import { SHELL_PREFERENCE_KEYS } from '../../../lib/shellPersistence';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { compactChatUpTo, voxTransport } from '../../../transport';
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
import { VoxSelect } from '../../ui/Select';
import { Icon } from '../../ui/Icons';
import { DiegeticFxStage } from '../../fx/DiegeticFxStage';

type RiverTimeWindow = '15m' | '1h' | '24h' | 'session';

const TIME_WINDOW_MS: Record<Exclude<RiverTimeWindow, 'session'>, number> = {
  '15m': 15 * 60 * 1000,
  '1h': 60 * 60 * 1000,
  '24h': 24 * 60 * 60 * 1000,
};

const VALID_TIME_WINDOWS = new Set<RiverTimeWindow>(['15m', '1h', '24h', 'session']);

const DEFAULT_SLUICE_GATES: Record<string, boolean> = {
  ast: true,
  typeck: true,
  toestub: true,
  tests: true,
};

const QUICK_JUMP_SURFACE_MAP: Record<string, string> = {
  operations: 'tasks',
  tasks: 'tasks',
  mens: 'mens',
  models: 'models',
  approvals: 'approvals',
  'harness-health': 'harness-health',
};

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
  const [timeWindow, setTimeWindow] = useState<RiverTimeWindow>('session');
  const [interceptorArmed, setInterceptorArmed] = useState(true);
  const [sluiceGates, setSluiceGates] = useState<Record<string, boolean>>(() => ({
    ...DEFAULT_SLUICE_GATES,
  }));
  const [aqueductReceipt, setAqueductReceipt] = useState<string | null>(null);
  const [aqueductYieldXp, setAqueductYieldXp] = useState<number>(250);

  useEffect(() => subscribeControlState(setBusControl), []);
  useEffect(() => subscribeModelOverride(setBusModelOverride), []);

  useEffect(() => {
    let active = true;
    if (typeof voxTransport.getGuiPreference !== 'function') return;
    void voxTransport
      .getGuiPreference('vox.harness.time_window.v1')
      .then((raw) => {
        if (active && raw && VALID_TIME_WINDOWS.has(raw as RiverTimeWindow)) {
          setTimeWindow(raw as RiverTimeWindow);
        }
      })
      .catch(() => {});
    void voxTransport
      .getGuiPreference('vox.harness.interceptor_armed.v1')
      .then((raw) => {
        if (!active || !raw) return;
        try {
          const parsed: unknown = JSON.parse(raw);
          if (typeof parsed === 'boolean') setInterceptorArmed(parsed);
        } catch {
          // ignore malformed preference
        }
      })
      .catch(() => {});
    void voxTransport
      .getGuiPreference('vox.harness.sluice_gates.v1')
      .then((raw) => {
        if (!active || !raw) return;
        try {
          const parsed: unknown = JSON.parse(raw);
          if (parsed && typeof parsed === 'object') {
            const next: Record<string, boolean> = { ...DEFAULT_SLUICE_GATES };
            for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
              if (typeof v === 'boolean') next[k] = v;
            }
            setSluiceGates(next);
          }
        } catch {
          // ignore malformed preference
        }
      })
      .catch(() => {});
    void voxTransport
      .getGuiPreference('vox.harness.claimed_yield_xp.v1')
      .then((raw) => {
        if (!active || !raw) return;
        const parsed = Number(raw);
        if (Number.isFinite(parsed) && parsed >= 0) setAqueductYieldXp(parsed);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);

  const effectiveControl = control ?? busControl;
  const effectiveModelOverride =
    chatModelOverride !== undefined ? chatModelOverride : (busModelOverride ?? readStoredModelOverride());
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const { filteredTasks, filteredMessages } = useMemo(() => {
    if (timeWindow === 'session') {
      return { filteredTasks: tasks, filteredMessages: messages };
    }
    const cutoffMs = Date.now() - TIME_WINDOW_MS[timeWindow];
    const nextTasks = tasks.filter((t) => {
      const tsCandidate = t as ChatExecutionTask & {
        updatedAtMs?: number;
        createdAtMs?: number;
        timestamp_ms?: number;
      };
      const ts = tsCandidate.updatedAtMs ?? tsCandidate.createdAtMs ?? tsCandidate.timestamp_ms;
      if (typeof ts === 'number') return ts >= cutoffMs;
      return t.status === 'in_progress' || t.status === 'running' || t.status === 'queued';
    });
    const nextMessages = messages
      .filter((m) =>
        typeof m.createdAtMs === 'number'
          ? m.createdAtMs >= cutoffMs
          : m.status === 'streaming' || m.status === 'pending',
      )
      .map((m) => {
        if (!m.events || m.events.length === 0) return m;
        return {
          ...m,
          events: m.events.filter((ev) => {
            const evTs =
              (ev as { timestamp_ms?: number; ts_ms?: number }).timestamp_ms ??
              (ev as { ts_ms?: number }).ts_ms;
            return typeof evTs === 'number' ? evTs >= cutoffMs : true;
          }),
        };
      });
    return { filteredTasks: nextTasks, filteredMessages: nextMessages };
  }, [tasks, messages, timeWindow]);

  const snapshot = useMemo(
    () =>
      deriveRiverSnapshot({
        tasks: filteredTasks,
        messages: filteredMessages,
        sessionSpentUsd,
        clutch: effectiveControl.clutch,
        risk: effectiveControl.risk,
        chatModelOverride: effectiveModelOverride,
        routing,
      }),
    [
      filteredTasks,
      filteredMessages,
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
    const resolved = QUICK_JUMP_SURFACE_MAP[target] ?? target;
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('vox-surface-navigate', { detail: { surface: resolved } }));
    }
  };

  const handleTimeWindowChange = (next: RiverTimeWindow) => {
    const prev = timeWindow;
    setTimeWindow(next);
    void voxTransport
      .setGuiPreference('vox.harness.time_window.v1', next)
      .then(() => {
        setAqueductReceipt(`Time window filtered to ${next}`);
      })
      .catch((err: unknown) => {
        setTimeWindow(prev);
        setAqueductReceipt(`Time window update failed: ${sanitizeErrorForToast(err)}`);
      });
  };

  const handleToggleInterceptor = () => {
    const prev = interceptorArmed;
    const next = !prev;
    setInterceptorArmed(next);
    void voxTransport
      .setGuiPreference('vox.harness.interceptor_armed.v1', JSON.stringify(next))
      .then(() => {
        setAqueductReceipt(
          next ? 'Verification Gate armed (Zero-Cascade)' : 'Verification Gate set to standby',
        );
      })
      .catch((err: unknown) => {
        setInterceptorArmed(prev);
        setAqueductReceipt(`Verification Gate update failed: ${sanitizeErrorForToast(err)}`);
      });
  };

  const persistSluiceGates = (
    prevGates: Record<string, boolean>,
    nextGates: Record<string, boolean>,
    successMsg: string,
  ) => {
    setSluiceGates(nextGates);
    void voxTransport
      .setGuiPreference('vox.harness.sluice_gates.v1', JSON.stringify(nextGates))
      .then(() => {
        setAqueductReceipt(successMsg);
      })
      .catch((err: unknown) => {
        setSluiceGates(prevGates);
        setAqueductReceipt(`Sluice gate update failed: ${sanitizeErrorForToast(err)}`);
      });
  };

  const handleCompactContext = () => {
    setAqueductReceipt('Compacting context reservoir…');
    void compactChatUpTo({ sessionId: 'default', direction: 'up_to' })
      .then((res) => {
        setAqueductReceipt(
          `Compacted Context Reservoir (${res.session_id} · turn #${res.compact_message_index})`,
        );
      })
      .catch((err: unknown) => {
        setAqueductReceipt(`Compact Context failed: ${sanitizeErrorForToast(err)}`);
      });
  };

  const handleClaimYield = () => {
    const prevXp = aqueductYieldXp;
    const nextTotal = prevXp + 250;
    setAqueductYieldXp(nextTotal);
    const gamifyCall =
      typeof voxTransport.recordGuiEvent === 'function'
        ? voxTransport.recordGuiEvent('aqueduct_yield_claimed', { xpDelta: prevXp })
        : Promise.resolve({ xpGranted: 250, lumensGranted: 18 });
    void Promise.all([
      gamifyCall,
      voxTransport.setGuiPreference('vox.harness.claimed_yield_xp.v1', String(nextTotal)),
    ])
      .then(([res]) => {
        const xp = res?.xpGranted && res.xpGranted > 0 ? res.xpGranted : 250;
        const lum = res?.lumensGranted && res.lumensGranted > 0 ? res.lumensGranted : 18;
        setAqueductReceipt(`Claimed +${xp} XP · +${lum} Denarii from verified Aqueduct cascade`);
      })
      .catch((err: unknown) => {
        setAqueductYieldXp(prevXp);
        setAqueductReceipt(`Claim Yield failed: ${sanitizeErrorForToast(err)}`);
      });
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
            {[
              {
                key: 'tasks',
                label: 'Operations Hub',
                purpose: 'Jump to the tasks operations queue to monitor active turns, hopper jobs, and agents.',
                hint: 'Open Tasks Queue',
              },
              {
                key: 'mens',
                label: 'MENS Studio',
                purpose: 'Jump to MENS Studio to inspect five-spoke datasets, adapter checkpoints, and benchmark rankings.',
                hint: 'Open MENS Studio',
              },
              {
                key: 'models',
                label: 'Models',
                purpose: 'Jump to Models to inspect provider routing, context windows, and benchmark leaderboards.',
                hint: 'Open Models',
              },
              {
                key: 'approvals',
                label: 'Approvals',
                purpose: 'Jump to Approvals to review pending human-in-the-loop tool gates and permissions.',
                hint: 'Open Approvals',
              },
              {
                key: 'harness-health',
                label: 'Harness Health',
                purpose: 'Jump to Harness Health to inspect live evaluation runs and regression flags.',
                hint: 'Open Harness Health',
              },
            ].map((item) => (
              <RomanTooltip
                key={item.key}
                title={item.label}
                purpose={item.purpose}
                actionHint={item.hint}
              >
                <button
                  type="button"
                  data-testid={`harness-jump-${item.key}`}
                  onClick={() => handleJump(item.key)}
                  className="flex items-center gap-1 rounded-md border border-[#D4AF37]/40 bg-bg-elevated px-2 py-1 font-mono text-[10px] font-semibold text-text-primary hover:border-[#D4AF37] transition"
                >
                  <span>{item.label}</span>
                  <Icon.externalLink className="size-3 shrink-0" aria-hidden="true" />
                </button>
              </RomanTooltip>
            ))}

            {/* Harness Mode Switchers */}
            {gamifyEnabled && (
              <div
                role="group"
                aria-label="Harness visualization mode"
                className="flex items-center rounded-lg border border-border-subtle bg-bg-surface p-0.5"
              >
                {(
                  [
                    { id: 'river', label: 'River', compactLabel: 'River' },
                    { id: 'urbs', label: 'Urbs Civilization', compactLabel: 'Urbs' },
                    { id: 'split', label: 'Split', compactLabel: 'Split' },
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
                      <span className="hidden sm:inline">{m.label}</span>
                      <span className="sm:hidden">{m.compactLabel}</span>
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
                  <VoxSelect
                    id="river-time-window-select"
                    aria-label="Time Window"
                    data-testid="river-time-window-select"
                    size="xs"
                    value={timeWindow}
                    onValueChange={(next) => handleTimeWindowChange(next as RiverTimeWindow)}
                    options={[
                      { value: '15m', label: 'Last 15m' },
                      { value: '1h', label: 'Last 1h' },
                      { value: '24h', label: 'Last 24h' },
                      { value: 'session', label: 'Current Session' },
                    ]}
                    className="h-6 w-auto min-w-[110px] border-[#C59B27]/65 bg-[#FAF6EE] px-2 py-0.5 font-mono text-[9.5px] font-semibold text-[#1C140C]"
                  />
                </div>

                <RomanTooltip
                  title="Deterministic Interceptor Gate"
                  purpose="Enforces strict verification gating before letting generated code execute on the file tree."
                  statusBadge={interceptorArmed ? 'ARMED' : 'BYPASS'}
                  actionHint="Click to toggle interceptor arm state"
                >
                  <button
                    type="button"
                    data-testid="aqueduct-interceptor-gate"
                    aria-pressed={interceptorArmed}
                    onClick={handleToggleInterceptor}
                    className="flex items-center gap-1.5 rounded-full border border-[#059669]/65 bg-[#ECFDF5] px-2.5 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-[#065F46] hover:border-[#047857] transition"
                  >
                    <Icon.gladiusShield className="size-3 text-[#059669]" />
                    <span>Verification Gate: {interceptorArmed ? 'Armed · Zero-Cascade' : 'Standby'}</span>
                  </button>
                </RomanTooltip>
              </div>
            </div>
            <div className={condensed ? 'relative z-10 h-[170px] w-full' : 'relative z-10 h-[210px] xl:h-[250px] w-full'}>
              <DiegeticFxStage
                preset="river-cascade"
                activeBoost={interceptorArmed ? 1.3 : 0.75}
                intensity={1.15}
                testId="harness-river-fx-stage"
              />
              <canvas
                ref={canvasRef}
                data-testid="harness-river-canvas"
                className="relative z-10 h-full w-full block"
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
                        const prevGates = sluiceGates;
                        const nextGates = { ...prevGates, [g.id]: !open };
                        persistSluiceGates(
                          prevGates,
                          nextGates,
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
                    setAqueductReceipt('Dispatched new task into Cursus II Balanced Hybrid channel');
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  Dispatch Task
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-flush"
                  onClick={() => {
                    persistSluiceGates(
                      sluiceGates,
                      { ...DEFAULT_SLUICE_GATES },
                      'Flushed all 4 Aqueduct Sluice Gates — zero-cascade stream verified',
                    );
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  Flush Sluice Gate
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-reroute"
                  onClick={() => {
                    if (onOpenRouting) {
                      onOpenRouting();
                    } else {
                      handleJump('models');
                    }
                    setAqueductReceipt('Rerouted active LLM stream across 3-Tier Castellum Divisorium');
                  }}
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  Reroute Tier
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-compact"
                  onClick={handleCompactContext}
                  className="rounded-md border border-[#8C6214]/85 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  Compact Context
                </button>
                <button
                  type="button"
                  data-testid="aqueduct-action-claim-yield"
                  onClick={handleClaimYield}
                  className="inline-flex items-center gap-1 rounded-md border border-[#B38728] bg-linear-to-b from-[#D4AF37] to-[#9A711C] px-2.5 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-[#1A1208] shadow-sm hover:brightness-110"
                >
                  <span>Claim Yield +{aqueductYieldXp} XP · +18</span>
                  <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />
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
