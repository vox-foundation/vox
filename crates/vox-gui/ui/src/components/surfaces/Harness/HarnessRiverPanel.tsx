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
import type { HarnessTierId } from './riverTypes';

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

    // Cache canvas CSS width/height to avoid calling getBoundingClientRect() every frame (60fps)
    let cachedW = Math.max(320, Math.floor(canvas.clientWidth || 560));
    let cachedH = Math.max(180, Math.floor(canvas.clientHeight || 260));

    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver((entries) => {
        for (const entry of entries) {
          if (entry.contentRect) {
            cachedW = Math.max(320, Math.floor(entry.contentRect.width || canvas.clientWidth || 560));
            cachedH = Math.max(180, Math.floor(entry.contentRect.height || canvas.clientHeight || 260));
          }
        }
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

  return (
    <section
      data-testid="axis-harness-river"
      className="flex flex-col gap-2.5 text-text-primary"
      aria-label="Axis Autonomous Harness River"
    >
      {/* Header Bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-white/10 bg-[#09090b] px-3 py-2 font-mono">
        <div className="flex items-center gap-2">
          <span className="inline-block size-2 rounded-full bg-cyan animate-pulse" />
          <span className="text-xs font-bold uppercase tracking-wider text-zinc-100">
            AXIS HARNESS RIVER
          </span>
          <span className="text-[10px] text-text-muted">
            Wired to Loquela composer · 3-Tier Cascade
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          {onOpenRouting && (
            <button
              type="button"
              onClick={onOpenRouting}
              className="rounded border border-white/15 bg-white/5 px-2 py-1 text-[10px] text-cyan hover:bg-white/10"
            >
              Routing & Tiers
            </button>
          )}
          {onFocusComposer && (
            <button
              type="button"
              onClick={onFocusComposer}
              className="rounded border border-white/15 bg-white/5 px-2 py-1 text-[10px] text-zinc-200 hover:bg-white/10"
            >
              Focus composer
            </button>
          )}
        </div>
      </div>

      {/* Zone A: Top Telemetry Waveform & 3-Tier Model Assignment Strip */}
      <WaveformTierStrip
        snapshot={snapshot}
        condensed={condensed}
        onSelectTier={canNavigateTier ? handleSelectTier : undefined}
      />

      {/* Zone B: Center Alluvial Particle Bundle + Singularity Chokepoint + Sweeping Laser */}
      <div className="relative overflow-hidden rounded-lg border border-white/10 bg-[#070709]">
        <div className="flex items-center justify-between border-b border-white/10 px-3 py-1 font-mono text-[9px] uppercase tracking-wider text-text-muted">
          <span>SOURCE WORKTREES & AST HASHES</span>
          <span className="text-cyan">INTERCEPTOR SINGULARITY</span>
          <span>VERIFIED PATCH TARGETS</span>
        </div>
        <div className={condensed ? 'h-[210px] w-full' : 'h-[270px] w-full'}>
          <canvas
            ref={canvasRef}
            data-testid="harness-river-canvas"
            className="h-full w-full block"
            aria-label="Live alluvial particle stream and phase scanner"
          />
        </div>
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
