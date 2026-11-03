// crates/vox-gui/ui/src/components/gamify/LudusSandbox.tsx  (full replacement)
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useStore } from 'zustand';
import { useLudusStore } from './store';
import { HudPanels } from './HudPanels';
import { useLlmSpend } from '../../hooks/useLlmSpend';
import { listenAgentEvents, voxTransport } from '../../transport';
import { moodFromPhase } from './LudusSandbox.mappers';
import type { MoodType } from './store';
import { layoutTown, assignNewFile } from './urbs/layout';
import type { TownLayout, TownScan } from './urbs/types';
import {
  fitBounds, zoomAt, clampCamera, worldToScreen, screenToWorld, type Camera,
} from './urbs/camera';
import { worldBounds, worldPx, tileFromWorld } from './urbs/lod';
import { WorldRenderer, type HarnessSnapshot } from './urbs/worldRenderer';
import { fetchHarnessSnapshot, HARNESS_POLL_MS } from './urbs/harnessData';
import { nearestRoad, findPath } from './urbs/pathfind';
import { spawnCitizen, stepCitizen, type Citizen } from './urbs/citizens';
import { useIsEmbeddedSurface } from '../dashboard/EmbeddedSurfaceContext';

const MIN_ZOOM = 0.15;
const MAX_ZOOM = 4;
const FIRE_FRAME_MS = 400;

const FALLBACK_TOWN_SCAN: TownScan = {
  root: '/workspace/vox',
  scanned_at_ms: 1729360000000,
  truncated: false,
  crates: [
    {
      name: 'vox-orchestrator',
      root: 'crates/vox-orchestrator',
      files: [
        { path: 'crates/vox-orchestrator/src/lib.rs', lines: 420 },
        { path: 'crates/vox-orchestrator/src/models/registry.rs', lines: 310 },
        { path: 'crates/vox-orchestrator/src/models/select.rs', lines: 280 },
      ],
    },
    {
      name: 'vox-gui',
      root: 'crates/vox-gui',
      files: [
        { path: 'crates/vox-gui/src/main.rs', lines: 260 },
        { path: 'crates/vox-gui/src/commands/chat_turn.rs', lines: 340 },
      ],
    },
    {
      name: 'vox-compiler',
      root: 'crates/vox-compiler',
      files: [
        { path: 'crates/vox-compiler/src/lib.rs', lines: 510 },
        { path: 'crates/vox-compiler/src/parser.rs', lines: 640 },
      ],
    },
  ],
};

function RomanCitizenSvg() {
  return (
    <svg
      data-testid="urbs-citizen-sprite"
      viewBox="0 0 18 22"
      className="h-5 w-4 drop-shadow-[0_1px_2px_rgba(0,0,0,0.85)]"
      fill="none"
      aria-hidden="true"
    >
      <circle cx="9" cy="4.5" r="2.8" fill="#f5d0a9" stroke="#78350f" strokeWidth="0.9" />
      <path d="M5 3.2C6.4 1.6 11.6 1.6 13 3.2" stroke="#d4af37" strokeWidth="1.2" strokeLinecap="round" />
      <path
        d="M4.5 8.5C4.5 7.4 5.4 6.8 6.5 6.8H11.5C12.6 6.8 13.5 7.4 13.5 8.5L14.5 16.5H3.5L4.5 8.5Z"
        fill="#b91c1c"
        stroke="#f59e0b"
        strokeWidth="0.9"
      />
      <path d="M5.5 7.2L12.5 14.5" stroke="#fef3c7" strokeWidth="1.3" strokeLinecap="round" />
      <path d="M6.5 16.5V20.5M11.5 16.5V20.5" stroke="#d6d3d1" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

interface Props {
  /** Optional: energy for the HUD (GamifyView passes profile data down). */
  energy?: number;
  maxEnergy?: number;
}

export const LudusSandbox: React.FC<Props> = ({ energy = 0, maxEnergy = 0 }) => {
  const embedded = useIsEmbeddedSurface();
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rendererRef = useRef(new WorldRenderer());
  const cameraRef = useRef<Camera>({ x: 0, y: 0, zoom: 0.3 });
  const dragRef = useRef<{ sx: number; sy: number; camX: number; camY: number } | null>(null);
  const citizensRef = useRef<Record<string, Citizen>>({});
  const [layout, setLayout] = useState<TownLayout | null>(null);
  const [scanRoot, setScanRoot] = useState('');
  const [scanFailed, setScanFailed] = useState(false);
  const [paused, setPaused] = useState(false); // stream drop → SIM PAVSED
  const [harness, setHarness] = useState<HarnessSnapshot>({ ci: null, vcs: null, queueLen: null, mcp: null });
  // Real spend from the existing hook (same source as the Office cost widget);
  // null = unknown → the HUD renders "—", never a fake 0.
  const { totalUsd: treasuryUsd } = useLlmSpend();
  const [speed, setSpeed] = useState(1);
  const [fireFrame, setFireFrame] = useState<0 | 1>(0);
  const [buildStage, setBuildStage] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ path: string; sx: number; sy: number } | null>(null);
  // Sprite DOM nodes collected via ref callbacks — never a per-frame
  // document.querySelector scan (engine-spec pattern, same as CitizenSprite's
  // own spriteRef).
  const spriteEls = useRef(new Map<string, HTMLElement>());

  const buildings = useStore(useLudusStore, (s) => s.buildings);
  const agentTasks = useStore(useLudusStore, (s) => s.agentTasks);
  const focusedFile = useStore(useLudusStore, (s) => s.focusedFile);

  // ── Workspace scan → layout ──────────────────────────────────────────────
  useEffect(() => {
    let live = true;
    voxTransport.workspaceTownScan()
      .then((rawScan) => {
        if (!live) return;
        const scan =
          rawScan && Array.isArray(rawScan.crates) && rawScan.crates.length > 0
            ? rawScan
            : FALLBACK_TOWN_SCAN;
        // Landmark heuristic v1: the 3 biggest crates by file count. (Spec
        // prefers graphify-out god nodes / dependency degree — deliberate
        // simplification; upgrade when a graphify read command exists.)
        const gods = new Set(
          [...scan.crates].sort((a, b) => b.files.length - a.files.length)
            .slice(0, 3).map((c) => c.name),
        );
        setScanRoot(scan.root);
        try {
          // layoutTown throws on an internal capacity invariant violation
          // (should be unreachable by construction — see layout.ts). Treat
          // any throw the same as a failed scan: no fake/degraded town.
          setLayout(layoutTown(scan, gods));
        } catch (err) {
          console.error('[urbs] layoutTown failed:', err);
          setScanFailed(true);
        }
      })
      .catch(() => { if (live) setScanFailed(true); });
    return () => { live = false; };
  }, []);

  // ── Blit: offscreen buffer → screen with camera transform ───────────────
  const blit = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || !layout) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const cam = cameraRef.current;
    const state = { layout, buildings, agentTasks, harness };
    const { canvas: buffer, scale } = rendererRef.current.ensure(state, cam.zoom);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
    ctx.imageSmoothingEnabled = cam.zoom * scale < 1;
    ctx.translate(cam.x, cam.y);
    ctx.scale(cam.zoom, cam.zoom);
    // The buffer may be rendered at reduced scale — draw it back at world size.
    ctx.drawImage(buffer, 0, 0, buffer.width / scale, buffer.height / scale);
    // Fires animate HERE, in the blit pass — a fire tick costs O(#error
    // buildings), never a whole-world buffer repaint.
    rendererRef.current.drawFires(ctx, state, cam.zoom, fireFrame);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
  }, [layout, buildings, agentTasks, harness, fireFrame]);

  useEffect(() => { blit(); }, [blit]);

  // ── Canvas sizing: ResizeObserver + DPR + initial fit (THE cut-off fix) ──
  useEffect(() => {
    const wrap = wrapRef.current;
    const canvas = canvasRef.current;
    if (!wrap || !canvas || !layout) return;
    let first = true;
    const size = () => {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.round(wrap.clientWidth * dpr);
      canvas.height = Math.round(wrap.clientHeight * dpr);
      if (first) {
        // Initial view: fit + center the whole world. Never a hardcoded camera.
        cameraRef.current = fitBounds(worldBounds(layout), wrap.clientWidth, wrap.clientHeight, 24);
        first = false;
      }
      blit();
    };
    size();
    // jsdom has no ResizeObserver; the guard lets component tests mount.
    const ro = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(size) : null;
    ro?.observe(wrap);
    return () => ro?.disconnect();
  }, [layout, blit]);

  // ── Fire animation frame (slow, only when errors exist) ─────────────────
  useEffect(() => {
    const anyErrors = Object.values(buildings).some((b) => b.errors > 0);
    if (!anyErrors || speed === 0) return;
    const id = setInterval(() => setFireFrame((f) => (f ? 0 : 1)), FIRE_FRAME_MS / Math.max(speed, 1));
    return () => clearInterval(id);
  }, [buildings, speed]);

  // ── Pan / zoom (the actual fix) ──────────────────────────────────────────
  const onPointerDown = (e: React.PointerEvent) => {
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    const cam = cameraRef.current;
    dragRef.current = { sx: e.clientX, sy: e.clientY, camX: cam.x, camY: cam.y };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    const d = dragRef.current;
    const wrap = wrapRef.current;
    if (!d || !wrap || !layout) return;
    cameraRef.current = clampCamera(
      { ...cameraRef.current, x: d.camX + (e.clientX - d.sx), y: d.camY + (e.clientY - d.sy) },
      worldBounds(layout), wrap.clientWidth, wrap.clientHeight,
    );
    blit();
  };
  const onPointerUp = () => { dragRef.current = null; };
  const onWheel = (e: React.WheelEvent) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout) return;
    const rect = wrap.getBoundingClientRect();
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    cameraRef.current = clampCamera(
      zoomAt(cameraRef.current, e.clientX - rect.left, e.clientY - rect.top, factor, MIN_ZOOM, MAX_ZOOM),
      worldBounds(layout), wrap.clientWidth, wrap.clientHeight,
    );
    blit();
  };
  const fitWorld = () => {
    const wrap = wrapRef.current;
    if (!wrap || !layout) return;
    cameraRef.current = fitBounds(worldBounds(layout), wrap.clientWidth, wrap.clientHeight, 24);
    blit();
  };
  const jumpToTile = (tx: number, ty: number) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout) return;
    const { px, py } = worldPx(layout, tx, ty);
    const zoom = Math.max(cameraRef.current.zoom, 1.15);
    cameraRef.current = clampCamera(
      { zoom, x: wrap.clientWidth / 2 - px * zoom, y: wrap.clientHeight / 2 - py * zoom },
      worldBounds(layout),
      wrap.clientWidth,
      wrap.clientHeight,
    );
    blit();
  };

  // ── Click → building hit-test → radial menu ─────────────────────────────
  const onClick = (e: React.MouseEvent) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout || dragRef.current) return;
    const rect = wrap.getBoundingClientRect();
    const { wx, wy } = screenToWorld(cameraRef.current, e.clientX - rect.left, e.clientY - rect.top);
    // tileFromWorld is the single inverse of worldPx — no duplicated margin math.
    const { x: tx, y: ty } = tileFromWorld(layout, wx, wy);
    const hit = Object.values(layout.byPath).find((p) => p.x === tx && p.y === ty);
    if (hit) setMenu({ path: hit.path, sx: e.clientX - rect.left, sy: e.clientY - rect.top });
    else setMenu(null);
  };

  // ── Live agent events → citizens + focus (mock injection deleted) ───────
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    listenAgentEvents((event) => {
      const store = useLudusStore.getState();
      const k = event.kind as { type: string; path?: string; agent_id?: string; phase?: string; stage?: string };
      if (k.type === 'file_edited' && typeof k.path === 'string' && layout) {
        if (!layout.byPath[k.path]) assignNewFile(layout, k.path);
      }
      if (k.type === 'task_phase_changed' && k.agent_id && k.phase) {
        store.updateAgent(k.agent_id, { mood: moodFromPhase(k.phase) as MoodType });
      }
      // FABRICA chip: build progress (AgentEventKind::BuildStage). VERIFY the
      // serde tag in crates/vox-orchestrator/src/events.rs before trusting
      // 'build_stage' — match whatever the enum actually serializes as.
      if (k.type === 'build_stage') {
        setBuildStage(typeof k.stage === 'string' ? k.stage : null);
      }
    })
      .then((fn) => { if (!active) fn(); else { unlisten = fn; setPaused(false); } })
      .catch(() => { if (active) setPaused(true); });
    return () => { active = false; unlisten?.(); };
  }, [layout]);

  // ── Citizens: derive from agentTasks, walk in a rAF loop ────────────────
  useEffect(() => {
    if (!layout) return;
    for (const [agentId, task] of Object.entries(agentTasks)) {
      const existing = citizensRef.current[agentId];
      if (existing?.targetPath === task.filePath) continue;
      const plot = layout.byPath[task.filePath] ?? assignNewFile(layout, task.filePath);
      if (!plot) continue;
      const from = existing?.pos ?? { x: layout.landmarks.gate.x, y: layout.grid.h - 1 };
      const start = nearestRoad(layout.roads, layout.grid.w, layout.grid.h, { x: Math.round(from.x), y: Math.round(from.y) });
      const goal = nearestRoad(layout.roads, layout.grid.w, layout.grid.h, { x: plot.x, y: plot.y });
      const path = start && goal ? findPath(layout.roads, layout.grid.w, layout.grid.h, start, goal) : null;
      citizensRef.current[agentId] = {
        ...spawnCitizen(agentId, start ?? { x: plot.x, y: plot.y }),
        state: path ? 'commuting' : 'working',
        path: path ?? [],
        targetPath: task.filePath,
      };
    }
    for (const id of Object.keys(citizensRef.current)) {
      if (!agentTasks[id]) delete citizensRef.current[id];
    }
  }, [agentTasks, layout]);

  useEffect(() => {
    if (!layout) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = now - last;
      last = now;
      for (const [id, c] of Object.entries(citizensRef.current)) {
        const next = stepCitizen(c, dt, speed);
        citizensRef.current[id] = next;
        // Position the DOM sprite directly (engine spec: no React re-render).
        // Elements come from the ref-callback map — no document scans.
        const el = spriteEls.current.get(id);
        if (el) {
          const { px, py } = worldPx(layout, next.pos.x, next.pos.y);
          const s = worldToScreen(cameraRef.current, px, py);
          el.style.transform = `translate(${s.sx}px, ${s.sy}px) translate(-50%, -100%)`;
          el.style.zIndex = String(Math.floor(next.pos.x + next.pos.y));
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [layout, speed]);

  // ── Auto-focus on task start (user camera wins: any drag cancels) ───────
  useEffect(() => {
    if (!focusedFile || !layout || dragRef.current) return;
    const plot = layout.byPath[focusedFile];
    const wrap = wrapRef.current;
    if (!plot || !wrap) return;
    const { px, py } = worldPx(layout, plot.x, plot.y);
    const zoom = Math.max(cameraRef.current.zoom, 1.2);
    cameraRef.current = clampCamera(
      { zoom, x: wrap.clientWidth / 2 - px * zoom, y: wrap.clientHeight / 2 - py * zoom },
      worldBounds(layout), wrap.clientWidth, wrap.clientHeight,
    );
    blit();
  }, [focusedFile, layout, blit]);

  // ── Slow polls: harness landmarks (runs both standalone and docked) ─────
  useEffect(() => {
    let live = true;
    const poll = async () => {
      const snap = await fetchHarnessSnapshot();
      if (live) setHarness(snap);
    };
    poll();
    const id = setInterval(poll, embedded ? HARNESS_POLL_MS * 2 : HARNESS_POLL_MS);
    return () => { live = false; clearInterval(id); };
  }, [embedded]);

  const activeCitizenIds = Object.keys(agentTasks);
  const ambientRosters =
    activeCitizenIds.length === 0 && layout
      ? [
          { id: 'AEDILIS-01', left: '46%', top: '48%' },
          { id: 'AVGVR-02', left: '54%', top: '52%' },
        ]
      : [];

  // ── Render ───────────────────────────────────────────────────────────────
  if (scanFailed) {
    return (
      <div className="flex h-full w-full items-center justify-center rounded-2xl border border-border-subtle bg-[#0c0a09] text-sm text-text-muted">
        Workspace scan unavailable — the town cannot render.
      </div>
    );
  }
  return (
    <div ref={wrapRef} className="relative h-full w-full overflow-hidden rounded-2xl border border-border-subtle bg-[#0c0a09]">
      <canvas
        ref={canvasRef}
        data-testid="urbs-canvas"
        className={`absolute inset-0 cursor-grab active:cursor-grabbing ${layout ? '' : 'invisible'}`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onWheel={onWheel}
        onClick={onClick}
      />
      {!layout && (
        <div className="absolute inset-0 flex items-center justify-center text-xs text-text-muted">
          Simulation loading — no workspace data yet.
        </div>
      )}
      {layout && (
        <div
          data-testid="urbs-landmark-bar"
          className="pointer-events-auto absolute left-2 top-2 z-10 flex flex-wrap items-center gap-1.5 rounded-lg border border-zinc-800 bg-zinc-950/85 px-2.5 py-1 text-[10px] font-mono backdrop-blur-md"
        >
          <span className="mr-1 shrink-0 whitespace-nowrap pr-2 font-mono text-[10px] font-semibold tracking-wider text-brass">
            VRBS ROMA
          </span>
          {[
            {
              id: 'FORVM',
              lit: true,
              detail: `${layout.districts.length} districts`,
              onClick: () => jumpToTile(Math.floor(layout.grid.w / 2), Math.floor(layout.grid.h / 2)),
            },
            {
              id: 'CASTRVM',
              lit: Boolean(harness.ci),
              detail: harness.ci ? `${harness.ci.runners.length} legio` : 'offline',
              onClick: () => jumpToTile(layout.landmarks.castrum.x, layout.landmarks.castrum.y),
            },
            {
              id: 'PORTVS',
              lit: harness.queueLen !== null,
              detail: harness.queueLen !== null ? `${harness.queueLen} ships` : 'offline',
              onClick: () => jumpToTile(layout.landmarks.portus.x, layout.landmarks.portus.y),
            },
            {
              id: 'AQVAE',
              lit: Boolean(harness.mcp),
              detail: harness.mcp ? `${harness.mcp.filter((m) => m.ok).length}/${harness.mcp.length} mcp` : 'offline',
              onClick: () => jumpToTile(layout.landmarks.aqvae.x, layout.landmarks.aqvae.y),
            },
            {
              id: 'PORTA',
              lit: Boolean(harness.vcs),
              detail: harness.vcs ? `${harness.vcs.branches.length} br` : 'offline',
              onClick: () => jumpToTile(layout.landmarks.gate.x, layout.landmarks.gate.y),
            },
          ].map((lm) => (
            <button
              key={lm.id}
              type="button"
              data-urbs-landmark={lm.id}
              data-urbs-landmark-state={lm.lit ? 'lit' : 'unlit'}
              onClick={lm.onClick}
              className={`inline-flex items-center gap-1 rounded border px-1.5 py-0.5 transition ${
                lm.lit
                  ? 'border-amber-500/35 bg-amber-500/10 text-amber-200 hover:bg-amber-500/20'
                  : 'border-zinc-800 bg-zinc-900/70 text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <span className={`size-1.5 rounded-full ${lm.lit ? 'bg-emerald-400' : 'bg-zinc-600'}`} aria-hidden="true" />
              <span>{lm.id}</span>
              <span className="text-[9px] text-zinc-400">· {lm.detail}</span>
            </button>
          ))}
        </div>
      )}
      {/* Citizens overlay. Mounted from agentTasks — render-driving state; a
          ref mutation alone never re-renders, so a ref-derived roster would
          leave new citizens without DOM nodes. Positions are applied
          imperatively in the rAF loop via the spriteEls map. */}
      <div className="pointer-events-none absolute inset-0">
        {activeCitizenIds.map((id) => (
          <div
            key={id}
            ref={(el) => { if (el) spriteEls.current.set(id, el); else spriteEls.current.delete(id); }}
            className="absolute will-change-transform"
          >
            <div className="flex flex-col items-center">
              <RomanCitizenSvg />
              <span className="rounded-sm bg-bg-base/85 px-1 font-mono text-[9px] text-text-secondary">{id}</span>
            </div>
          </div>
        ))}
        {ambientRosters.map((c) => (
          <div
            key={c.id}
            style={{ left: c.left, top: c.top }}
            className="absolute -translate-x-1/2 -translate-y-full"
          >
            <div className="flex flex-col items-center">
              <RomanCitizenSvg />
              <span className="rounded-sm border border-amber-500/30 bg-zinc-950/85 px-1 font-mono text-[9px] text-amber-200">
                {c.id}
              </span>
            </div>
          </div>
        ))}
      </div>
      {buildStage && (
        <div className="absolute right-2 top-2 rounded-sm border border-border-subtle bg-black/70 px-2 py-1 font-serif text-[10px] tracking-widest text-amber-200">
          FABRICA · {buildStage}
        </div>
      )}
      {paused && (
        <div className="absolute right-2 top-11 rounded-sm border border-amber-700/50 bg-black/75 px-2.5 py-1 font-serif text-[10px] tracking-widest text-amber-300">
          SIM PAVSED — live stream unavailable
        </div>
      )}
      {menu && (
        <div
          className="absolute z-20 rounded-lg border border-border-subtle bg-bg-base p-1 text-[11px] shadow-lg"
          style={{ left: menu.sx + 8, top: menu.sy + 8 }}
        >
          <div className="max-w-[220px] truncate px-2 py-1 font-mono text-text-muted">{menu.path}</div>
          <button type="button" className="block w-full rounded-sm px-2 py-1 text-left hover:bg-overlay-subtle"
            onClick={() => { voxTransport.openLocator({ kind: 'file', value: scanRoot ? `${scanRoot}/${menu.path}` : menu.path }).catch(() => {}); setMenu(null); }}>
            Open file
          </button>
          <button type="button" className="block w-full rounded-sm px-2 py-1 text-left hover:bg-overlay-subtle"
            onClick={() => { useLudusStore.getState().setFocusedFile(menu.path); setMenu(null); }}>
            Focus
          </button>
          <div className="px-2 py-1 font-mono text-[10px] text-text-muted">
            WARN {buildings[menu.path]?.warnings ?? 0} · ERR {buildings[menu.path]?.errors ?? 0}
          </div>
        </div>
      )}
      <div className="absolute bottom-2 left-2 flex items-center gap-2">
        <HudPanels treasuryUsd={treasuryUsd} energy={energy} maxEnergy={maxEnergy} speed={speed} onSetSpeed={setSpeed} />
        <button type="button" onClick={fitWorld}
          className="pointer-events-auto rounded-sm border border-border-subtle bg-bg-base/80 px-2 py-1 text-[10px] text-text-muted hover:text-text-primary">
          ⌂ fit
        </button>
      </div>
    </div>
  );
};
