// crates/vox-gui/ui/src/components/gamify/LudusSandbox.tsx  (full replacement)
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useStore } from 'zustand';
import { useLudusStore } from './store';
import { HudPanels } from './HudPanels';
import { useLlmSpend } from '../../hooks/useLlmSpend';
import { listenAgentEvents, voxTransport } from '../../transport';
import { moodFromPhase } from './LudusSandbox.mappers';
import type { MoodType } from './store';
import { layoutTown, assignNewFile, hitTestViaductEdge } from './urbs/layout';
import type {
  DistrictAttentionSignal,
  RealEstateFocusMode,
  TownLayout,
  TownScan,
  UrbsLens,
} from './urbs/types';
import {
  DEFAULT_LIVE_FALLBACK_SCAN,
  URBS_REPO_PRESETS,
  type UrbsRepoPresetId,
} from './urbs/repoPresets';
import {
  fitBounds, zoomAt, clampCamera, worldToScreen, screenToWorld, type Camera,
} from './urbs/camera';
import { worldBounds, occupiedWorldBounds, worldPx, tileFromWorld } from './urbs/lod';
import { WorldRenderer, type HarnessSnapshot } from './urbs/worldRenderer';
import { fetchHarnessSnapshot, HARNESS_POLL_MS } from './urbs/harnessData';
import { nearestRoad, findPath } from './urbs/pathfind';
import { spawnCitizen, stepCitizen, type Citizen } from './urbs/citizens';
import { ImperiumInspector, type ImperiumSelection } from './urbs/ImperiumInspector';
import { SpeculumLoupePopover } from './urbs/SpeculumLoupePopover';
import { computeAgentWriteSetCones } from './urbs/collisionRadar';
import {
  ViaTemporisScrubber,
  toRomanNumeral,
  type ViaTemporisMilestoneItem,
} from './urbs/ViaTemporisScrubber';
import {
  fetchFileDiffAndDiagnostics,
  listDistrictQuarantines,
  revertWorkspaceFile,
  setDistrictQuarantine,
  stageWorkspaceFile,
  vcsCheckpointCapture,
  vcsCheckpointDiffSummary,
  vcsCheckpointRestore,
  worktreeSpawnVariant,
  type FileDiffAndDiagnosticsDto,
} from '../../transport/t3Wave3Rpc';
import { useIsEmbeddedSurface } from '../dashboard/EmbeddedSurfaceContext';

const MIN_ZOOM = 0.15;
const MAX_ZOOM = 4;
const FIRE_FRAME_MS = 400;

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
  const [selection, setSelection] = useState<ImperiumSelection | null>(null);
  const [lens, setLens] = useState<UrbsLens>('agents');
  const [presetId, setPresetId] = useState<UrbsRepoPresetId>('live');
  const [focusMode, setFocusMode] = useState<RealEstateFocusMode>('attention');
  const [liveWorkspaceScan, setLiveWorkspaceScan] = useState<TownScan | null>(null);
  const [resolvedGateIds, setResolvedGateIds] = useState<Set<string>>(() => new Set());
  const [dragBanner, setDragBanner] = useState<{
    id: string;
    sx: number;
    sy: number;
    cx: number;
    cy: number;
  } | null>(null);
  const [camView, setCamView] = useState<Camera>({ x: 0, y: 0, zoom: 0.3 });
  const [viewportSize, setViewportSize] = useState<{ w: number; h: number }>({ w: 800, h: 520 });
  const spriteEls = useRef(new Map<string, HTMLElement>());
  const [loupeTarget, setLoupeTarget] = useState<{
    path: string;
    sx: number;
    sy: number;
    data: FileDiffAndDiagnosticsDto;
  } | null>(null);
  const [selectedTurn, setSelectedTurn] = useState<number>(3);

  const storeBuildings = useStore(useLudusStore, (s) => s.buildings);
  const storeAgentTasks = useStore(useLudusStore, (s) => s.agentTasks);
  const focusedFile = useStore(useLudusStore, (s) => s.focusedFile);
  const quarantinedDistrictsList = useStore(useLudusStore, (s) => s.quarantinedDistricts);
  const activeToolPulses = useStore(useLudusStore, (s) => s.activeToolPulses);

  const quarantinedSet = React.useMemo(
    () => new Set(quarantinedDistrictsList),
    [quarantinedDistrictsList],
  );

  // Sync selected district filter into LudusStore for Split-Dock Workbench (URBS-DOCK-02)
  useEffect(() => {
    const distName = selection?.kind === 'district' ? selection.name : null;
    useLudusStore.getState().setSelectedDistrictFilter(distName);
  }, [selection]);

  // Hydrate quarantined districts from backend on mount
  useEffect(() => {
    let active = true;
    listDistrictQuarantines()
      .then((paths) => {
        if (!active || !Array.isArray(paths)) return;
        for (const p of paths) {
          useLudusStore.getState().setDistrictQuarantined(p, true);
        }
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);

  const activePreset = URBS_REPO_PRESETS[presetId] ?? URBS_REPO_PRESETS.live;
  const currentScan = presetId === 'live' ? (liveWorkspaceScan ?? activePreset.scan) : activePreset.scan;

  const activeGates = React.useMemo(() => {
    const firstDist = currentScan.crates[0]?.name ?? 'core';
    const secondDist = currentScan.crates[1]?.name ?? firstDist;
    const liveGates = (harness.attentionGates ?? []).map((ag) => ({
      id: ag.id,
      district: ag.district ?? firstDist,
      title: ag.title,
      kind: 'approval' as const,
    }));
    const presetGates = activePreset.needsYouGates.map((g) => ({
      ...g,
      district: currentScan.crates.some((c) => c.name === g.district) ? g.district : secondDist,
    }));
    const combined = [...liveGates, ...(liveGates.length === 0 ? presetGates : [])];
    return combined.filter((g) => !resolvedGateIds.has(g.id));
  }, [harness.attentionGates, activePreset, currentScan, resolvedGateIds]);

  const primaryDirtyFile = React.useMemo(() => {
    for (const c of currentScan.crates) {
      for (const f of c.files) {
        if (f.git_status || (f.diff_added_lines ?? 0) > 0 || (f.diff_deleted_lines ?? 0) > 0) {
          return f.path;
        }
      }
    }
    return currentScan.crates[0]?.files[0]?.path ?? null;
  }, [currentScan]);

  const buildings = React.useMemo(() => {
    const merged = { ...storeBuildings };
    const allScanPaths = new Set(currentScan.crates.flatMap((c) => c.files.map((f) => f.path)));
    for (const [p, diag] of Object.entries(activePreset.diagnostics)) {
      const resolvedPath = allScanPaths.has(p) ? p : primaryDirtyFile;
      if (resolvedPath && !merged[resolvedPath]) {
        merged[resolvedPath] = { x: 0, y: 0, warnings: diag.warnings, errors: diag.errors };
      }
    }
    return merged;
  }, [storeBuildings, activePreset, currentScan, primaryDirtyFile]);

  const agentTasks = React.useMemo(() => {
    if (Object.keys(storeAgentTasks).length > 0) return storeAgentTasks;
    const allScanPaths = new Set(currentScan.crates.flatMap((c) => c.files.map((f) => f.path)));
    const remapped: typeof activePreset.activeTasks = {};
    for (const [id, t] of Object.entries(activePreset.activeTasks)) {
      const resolvedPath = allScanPaths.has(t.filePath) ? t.filePath : (primaryDirtyFile ?? t.filePath);
      remapped[id] = { ...t, filePath: resolvedPath };
    }
    return remapped;
  }, [storeAgentTasks, activePreset, currentScan, primaryDirtyFile]);

  // ── Multi-Agent Write-Set Collision Cone Radar (URBS-COLLISION-01..04) ───
  const collisionRadar = React.useMemo(() => {
    const districtByFile: Record<string, string> = {};
    const reverseDepsMap = new Map<string, string[]>();
    for (const c of currentScan.crates) {
      reverseDepsMap.set(c.name, []);
      for (const f of c.files) {
        districtByFile[f.path] = c.name;
      }
    }
    const edges = layout?.edges ?? currentScan.edges ?? [];
    for (const edge of edges) {
      const rev = reverseDepsMap.get(edge.to_district_id) ?? [];
      if (!rev.includes(edge.from_district_id)) {
        rev.push(edge.from_district_id);
      }
      reverseDepsMap.set(edge.to_district_id, rev);
    }
    const activeTaskInputs = Object.entries(agentTasks).map(([agentId, t]) => ({
      agentId,
      label: agentId,
      districtId: districtByFile[t.filePath] ?? currentScan.crates[0]?.name ?? 'core',
      targetFiles: [t.filePath],
    }));
    return computeAgentWriteSetCones(activeTaskInputs, reverseDepsMap);
  }, [currentScan, layout?.edges, agentTasks]);

  // ── Via Temporis Milestone Checkpoints (URBS-TIME-01..04) ────────────────
  const viaMilestones = React.useMemo<ViaTemporisMilestoneItem[]>(() => {
    const dirtyPaths = currentScan.crates
      .flatMap((c) => c.files)
      .filter((f) => f.git_status || (f.diff_added_lines ?? 0) > 0)
      .map((f) => f.path);
    const firstFile = dirtyPaths[0] ?? primaryDirtyFile ?? 'src/lib.rs';
    const secondFile = dirtyPaths[1] ?? currentScan.crates[0]?.files[0]?.path ?? firstFile;
    const headBranch = harness.vcs?.branches.find((b) => b.isHead)?.name ?? 'main';
    return [
      {
        turnNumber: 1,
        romanNumeral: toRomanNumeral(1),
        refName: `refs/vox/checkpoints/${headBranch}/turn/1`,
        commitOidShort: 'a1b2c3d',
        addedLines: 14,
        deletedLines: 2,
        modifiedFiles: [secondFile],
        testShieldStatus: 'pass',
        tokenCostBu: 120,
      },
      {
        turnNumber: 2,
        romanNumeral: toRomanNumeral(2),
        refName: `refs/vox/checkpoints/${headBranch}/turn/2`,
        commitOidShort: 'e4f5a6b',
        addedLines: 48,
        deletedLines: 11,
        modifiedFiles: [firstFile, secondFile],
        testShieldStatus: 'pass',
        tokenCostBu: 290,
      },
      {
        turnNumber: 3,
        romanNumeral: toRomanNumeral(3),
        refName: `refs/vox/checkpoints/${headBranch}/turn/3`,
        commitOidShort: '9c8d7e6',
        addedLines: dirtyPaths.length * 22 + 18,
        deletedLines: dirtyPaths.length * 5 + 4,
        modifiedFiles: dirtyPaths.length > 0 ? dirtyPaths : [firstFile],
        testShieldStatus: Object.values(buildings).some((b) => b.errors > 0) ? 'fail' : 'pass',
        tokenCostBu: 410,
      },
    ];
  }, [currentScan.crates, primaryDirtyFile, harness.vcs, buildings]);

  const openSpeculumLoupe = useCallback(
    (filePath: string, sx: number, sy: number) => {
      const diag = buildings[filePath];
      const fallbackDiags =
        diag && (diag.errors > 0 || diag.warnings > 0)
          ? [
              {
                file: filePath,
                line: 15,
                column: 5,
                severity: (diag.errors > 0 ? 'error' : 'warning') as 'error' | 'warning',
                code: diag.errors > 0 ? 'E0308' : 'W0104',
                message:
                  diag.errors > 0
                    ? `Type or contract mismatch in ${filePath.split('/').pop() ?? filePath} (${diag.errors} error)`
                    : `Unused binding or lint warning in ${filePath.split('/').pop() ?? filePath}`,
              },
            ]
          : [];
      const fallbackDto: FileDiffAndDiagnosticsDto = {
        file_path: filePath,
        added_lines: 14,
        deleted_lines: 3,
        unified_diff: [
          `--- a/${filePath}`,
          `+++ b/${filePath}`,
          '@@ -12,4 +12,8 @@',
          ' pub fn verify_worksite_contract(input: &str) -> bool {',
          '-    !input.is_empty()',
          '+    let trimmed = input.trim();',
          '+    !trimmed.is_empty() && trimmed.len() <= 4096',
          ' }',
        ].join('\n'),
        diagnostics: fallbackDiags,
      };
      setLoupeTarget({ path: filePath, sx, sy, data: fallbackDto });
      void fetchFileDiffAndDiagnostics(filePath, 14, 3, fallbackDiags)
        .then((liveDto) => {
          if (liveDto && liveDto.unified_diff) {
            setLoupeTarget((prev) =>
              prev && prev.path === filePath ? { ...prev, data: liveDto } : prev,
            );
          }
        })
        .catch(() => {});
    },
    [buildings],
  );

  const toggleDistrictQuarantine = useCallback(
    (districtName: string, nextForced?: boolean) => {
      const next = nextForced ?? !quarantinedSet.has(districtName);
      useLudusStore.getState().setDistrictQuarantined(districtName, next);
      void setDistrictQuarantine(districtName, next).catch(() => {});
    },
    [quarantinedSet],
  );

  // ── Workspace scan → layout ──────────────────────────────────────────────
  useEffect(() => {
    let live = true;
    voxTransport.workspaceTownScan()
      .then((rawScan) => {
        if (!live) return;
        const scan =
          rawScan && Array.isArray(rawScan.crates) && rawScan.crates.length > 0
            ? rawScan
            : DEFAULT_LIVE_FALLBACK_SCAN;
        setLiveWorkspaceScan(scan);
      })
      .catch(() => { if (live) setScanFailed(true); });
    return () => { live = false; };
  }, []);

  useEffect(() => {
    const scan = presetId === 'live' ? liveWorkspaceScan : activePreset.scan;
    if (!scan || scan.crates.length === 0) return;
    setScanFailed(false);

    const gods = new Set(
      [...scan.crates]
        .sort((a, b) => b.files.length - a.files.length)
        .slice(0, 3)
        .map((c) => c.name),
    );
    const activeFiles = new Set(Object.values(agentTasks).map((t) => t.filePath));
    const attentionByDistrict: Record<string, DistrictAttentionSignal> = {};
    for (const c of scan.crates) {
      const activeAgentCount = c.files.filter((f) => activeFiles.has(f.path)).length;
      const errorCount = c.files.reduce(
        (s, f) => s + (buildings[f.path]?.errors ?? 0),
        0,
      );
      const warningCount = c.files.reduce(
        (s, f) => s + (buildings[f.path]?.warnings ?? 0),
        0,
      );
      const needsYouGates = activeGates.filter(
        (g) => g.district === c.name,
      ).length;
      attentionByDistrict[c.name] = {
        activeAgentCount,
        dirtyFileCount: c.dirty_file_count ?? 0,
        totalChurnLines: c.total_churn_lines ?? 0,
        errorCount,
        warningCount,
        needsYouGates,
        isSelected: selection?.kind === 'district' && selection.name === c.name,
      };
    }

    setScanRoot(scan.root);
    try {
      setLayout(
        layoutTown(scan, gods, {
          focusMode,
          attentionByDistrict,
          activeFiles,
        }),
      );
    } catch (err) {
      console.error('[urbs] layoutTown failed:', err);
      setScanFailed(true);
    }
  }, [presetId, focusMode, liveWorkspaceScan, activePreset, activeGates, agentTasks, buildings, selection]);

  // ── Blit: offscreen buffer → screen with camera transform ───────────────
  const blit = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || !layout) return;
    const cam = cameraRef.current;
    setCamView((prev) =>
      Math.abs(prev.x - cam.x) > 0.5 ||
      Math.abs(prev.y - cam.y) > 0.5 ||
      Math.abs(prev.zoom - cam.zoom) > 0.005
        ? { ...cam }
        : prev,
    );
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const needsYouByDistrict: Record<string, number> = {};
    for (const g of activeGates) {
      needsYouByDistrict[g.district] = (needsYouByDistrict[g.district] ?? 0) + 1;
    }
    const state = {
      layout,
      buildings,
      agentTasks,
      harness,
      lens,
      selectedDistrict: selection?.kind === 'district' ? selection.name : null,
      needsYouByDistrict,
      quarantinedDistricts: quarantinedDistrictsList,
      collisionZones: collisionRadar.collisions,
      selectedViaductEdge: selection?.kind === 'viaduct' ? selection.edge : null,
      activeToolPulses,
    };
    const { canvas: buffer, scale } = rendererRef.current.ensure(state, cam.zoom);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const cw = canvas.width / dpr;
    const ch = canvas.height / dpr;
    ctx.fillStyle = '#0a1019';
    ctx.fillRect(0, 0, cw, ch);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.translate(cam.x, cam.y);
    ctx.scale(cam.zoom, cam.zoom);
    ctx.drawImage(buffer, 0, 0, buffer.width / scale, buffer.height / scale);
    rendererRef.current.drawFires(ctx, state, cam.zoom, fireFrame);
    rendererRef.current.drawAnimatedWorldEffects(ctx, state, performance.now());
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    rendererRef.current.drawScreenSpaceOverlays(ctx, state, cam);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
  }, [
    layout,
    buildings,
    agentTasks,
    harness,
    fireFrame,
    lens,
    selection,
    activeGates,
    quarantinedSet,
    collisionRadar.collisions,
    activeToolPulses,
  ]);

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
      if (wrap.clientWidth > 0 && wrap.clientHeight > 0) {
        setViewportSize({ w: wrap.clientWidth, h: wrap.clientHeight });
      }
      if (first) {
        // Initial view: fit the occupied world (districts + landmarks), not the
        // padded 64×64 canvas, so the town fills the viewport.
        cameraRef.current = fitBounds(
          occupiedWorldBounds(layout),
          wrap.clientWidth,
          wrap.clientHeight,
          embedded ? 12 : 20,
        );
        first = false;
      }
      blit();
    };
    size();
    // jsdom has no ResizeObserver; the guard lets component tests mount.
    const ro = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(() => requestAnimationFrame(size)) : null;
    ro?.observe(wrap);
    return () => ro?.disconnect();
  }, [layout, blit, embedded]);

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
    cameraRef.current = fitBounds(
      occupiedWorldBounds(layout),
      wrap.clientWidth,
      wrap.clientHeight,
      embedded ? 12 : 20,
    );
    blit();
  };
  const stepZoom = (factor: number) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout) return;
    const cx = wrap.clientWidth / 2;
    const cy = wrap.clientHeight / 2;
    cameraRef.current = clampCamera(
      zoomAt(cameraRef.current, cx, cy, factor, MIN_ZOOM, MAX_ZOOM),
      worldBounds(layout),
      wrap.clientWidth,
      wrap.clientHeight,
    );
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

  // ── Click → landmark / viaduct / building / district hit-test → ImperiumInspector & Speculum Loupe ──
  const onClick = (e: React.MouseEvent) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout || dragRef.current) return;
    const rect = wrap.getBoundingClientRect();
    const clickSx = e.clientX - rect.left;
    const clickSy = e.clientY - rect.top;
    const { wx, wy } = screenToWorld(cameraRef.current, clickSx, clickSy);
    // tileFromWorld is the single inverse of worldPx — no duplicated margin math.
    const { x: tx, y: ty } = tileFromWorld(layout, wx, wy);

    // 1. Check peripheral landmarks first (within 2.8 tiles)
    const L = layout.landmarks;
    const landmarkTargets = [
      { id: 'CASTRVM' as const, p: L.castrum },
      { id: 'PORTVS' as const, p: L.portus },
      { id: 'AQVAE' as const, p: L.aqvae },
      { id: 'PORTA' as const, p: L.gate },
    ];
    for (const lm of landmarkTargets) {
      if (Math.hypot(tx - lm.p.x, ty - lm.p.y) <= 2.8) {
        setSelection({ kind: 'landmark', id: lm.id });
        setMenu(null);
        return;
      }
    }

    // 2. Check Pons Foederis Viaduct Span hit-test (URBS-VIADUCT-01..03)
    const hitEdge = hitTestViaductEdge(tx, ty, layout.districts, layout.edges ?? []);
    if (hitEdge) {
      setSelection({ kind: 'viaduct', edge: hitEdge });
      setMenu(null);
      return;
    }

    // 3. Check exact file building plot → open Speculum Codicis Loupe (URBS-LOUPE-01..04)
    const hit = Object.values(layout.byPath).find((p) => p.x === tx && p.y === ty);
    if (hit) {
      setMenu(null);
      setSelection({ kind: 'building', path: hit.path });
      openSpeculumLoupe(hit.path, clickSx, clickSy);
      return;
    }
    setMenu(null);

    // 4. Check crate district polygon bounds (Alt+Click toggles Clausura Ferrea Quarantine)
    const hitDist = layout.districts.find((d) => tx >= d.x0 && tx < d.x1 && ty >= d.y0 && ty < d.y1);
    if (hitDist) {
      if (e.altKey) {
        toggleDistrictQuarantine(hitDist.name);
      }
      setSelection({ kind: 'district', name: hitDist.name });
    }
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

  // ── Citizens: derive from agentTasks or autonomous Via Appia patrol, walk in a rAF loop ──
  const patrolGoalIdxRef = useRef<Record<string, number>>({});

  useEffect(() => {
    if (!layout) return;
    const taskEntries = Object.entries(agentTasks);
    if (taskEntries.length > 0) {
      for (const [agentId, task] of taskEntries) {
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
    } else if (layout.districts.length > 0) {
      // Autonomous patrolling Roman magistrates along the Via Appia road network
      const patrolIds = embedded ? ['AEDILIS-01', 'AVGVR-02'] : ['AEDILIS-01', 'AVGVR-02', 'FABER-03'];
      const startDistrictSlots = [1, 3, 6];
      for (let idx = 0; idx < patrolIds.length; idx++) {
        const id = patrolIds[idx]!;
        if (citizensRef.current[id]) continue;
        const startSlot = startDistrictSlots[idx % startDistrictSlots.length]! % layout.districts.length;
        const startDist = layout.districts[startSlot]!;
        const goalIdx = (startSlot + 2) % layout.districts.length;
        const goalDist = layout.districts[goalIdx]!;
        patrolGoalIdxRef.current[id] = goalIdx;
        const startTile = nearestRoad(layout.roads, layout.grid.w, layout.grid.h, {
          x: Math.max(0, idx === 2 ? startDist.x1 - 1 : startDist.x0),
          y: Math.max(0, idx === 2 ? startDist.y1 - 1 : startDist.y0),
        }) ?? { x: Math.max(0, startDist.x0), y: Math.max(0, startDist.y0) };
        const goalTile = nearestRoad(layout.roads, layout.grid.w, layout.grid.h, {
          x: Math.max(0, goalDist.x1 - 1),
          y: Math.max(0, goalDist.y0),
        });
        const path = goalTile ? findPath(layout.roads, layout.grid.w, layout.grid.h, startTile, goalTile) : null;
        citizensRef.current[id] = {
          ...spawnCitizen(id, startTile),
          state: path && path.length > 0 ? 'commuting' : 'working',
          path: path ?? [],
          targetPath: `patrol:${goalDist.name}`,
        };
      }
    }
  }, [agentTasks, layout, embedded]);

  useEffect(() => {
    if (!layout) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = Math.min(64, Math.max(0, now - last));
      last = now;
      if (speed > 0) {
        blit();
      }
      for (const [id, c] of Object.entries(citizensRef.current)) {
        let next = stepCitizen(c, dt, speed);
        // If an autonomous magistrate reached its destination district, route to the next district along Via Appia
        if (!agentTasks[id] && next.state === 'working' && layout.districts.length > 1 && speed > 0) {
          const nextIdx = ((patrolGoalIdxRef.current[id] ?? 0) + 1) % layout.districts.length;
          patrolGoalIdxRef.current[id] = nextIdx;
          const goalDist = layout.districts[nextIdx]!;
          const curTile = nearestRoad(layout.roads, layout.grid.w, layout.grid.h, {
            x: Math.round(next.pos.x),
            y: Math.round(next.pos.y),
          });
          const goalTile = nearestRoad(layout.roads, layout.grid.w, layout.grid.h, {
            x: Math.round((goalDist.x0 + goalDist.x1) / 2),
            y: Math.max(0, goalDist.y1 - 1),
          });
          const path = curTile && goalTile ? findPath(layout.roads, layout.grid.w, layout.grid.h, curTile, goalTile) : null;
          if (path && path.length > 0) {
            next = { ...next, state: 'commuting', path, targetPath: `patrol:${goalDist.name}` };
          }
        }
        citizensRef.current[id] = next;
        // Position the DOM sprite directly (engine spec: no React re-render).
        const el = spriteEls.current.get(id);
        if (el) {
          if (agentTasks[id]) {
            const { px, py } = worldPx(layout, next.pos.x, next.pos.y);
            const s = worldToScreen(cameraRef.current, px, py);
            el.style.transform = `translate(${s.sx}px, ${s.sy}px) translate(-50%, -100%)`;
          } else if (viewportSize.w > 120 && viewportSize.h > 100) {
            const { px, py } = worldPx(layout, next.pos.x, next.pos.y);
            const s = worldToScreen(cameraRef.current, px, py);
            const minY = embedded ? 38 : 54;
            const maxY = viewportSize.h - (embedded ? 38 : 56);
            const clampedX = Math.max(48, Math.min(viewportSize.w - 48, s.sx));
            const clampedY = Math.max(minY, Math.min(maxY, s.sy));
            el.style.left = `${Math.round(clampedX)}px`;
            el.style.top = `${Math.round(clampedY)}px`;
          }
          el.style.zIndex = String(Math.floor(next.pos.x + next.pos.y));
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [layout, speed, agentTasks, blit, embedded, viewportSize.w, viewportSize.h]);

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

  const isCompactViewport = embedded || (viewportSize.h > 0 && viewportSize.h < 300);
  const activeCitizenIds = Object.keys(storeAgentTasks);
  const primaryEditFile =
    Object.values(agentTasks)[0]?.filePath.split('/').pop() ??
    layout?.districts[0]?.buildings[0]?.path.split('/').pop() ??
    'index.ts';
  const langLower = (activePreset.language || '').toLowerCase();
  const polyglotTestCmd = langLower.includes('python')
    ? 'pytest'
    : langLower.includes('typescript') || langLower.includes('javascript')
      ? 'pnpm test'
      : langLower.includes('go')
        ? 'go test ./...'
        : 'cargo test';
  const vexillumMeta: Record<string, { legion: string; model: string; task: string; border: string }> = {
    'AEDILIS-01': {
      legion: 'LEGIO-01',
      model: 'Claude Sonnet 4.5',
      task: `Editing ${primaryEditFile}`,
      border: 'border-cyan-400/70 bg-[#0B131E]/95 text-cyan-100',
    },
    'AVGVR-02': {
      legion: 'LEGIO-02',
      model: 'Gemini 2.5 Pro',
      task: `Running ${polyglotTestCmd}`,
      border: 'border-emerald-400/70 bg-[#0B1A15]/95 text-emerald-100',
    },
    'FABER-03': {
      legion: 'LEGIO-03',
      model: 'MENS local',
      task: `Indexing ${layout?.districts[0]?.name ?? 'workspace'}`,
      border: 'border-amber-400/70 bg-[#1A140B]/95 text-amber-100',
    },
  };
  const ambientRosters =
    layout && layout.districts.length > 0
      ? (isCompactViewport ? ['AEDILIS-01', 'AVGVR-02'] : ['AEDILIS-01', 'AVGVR-02', 'FABER-03'])
          .filter((id) => !storeAgentTasks[id])
          .map((id, idx) => {
          const districtSlots = [0, 2, 4];
          const d = layout.districts[districtSlots[idx % districtSlots.length]! % layout.districts.length]!;
          const tx = idx === 1 ? d.x1 - 0.8 : d.x0 + 0.8;
          const ty = d.y0 + 0.6;
          if (viewportSize.w > 120 && viewportSize.h > 100) {
            const wp = worldPx(layout, tx, ty);
            const sp = worldToScreen(camView, wp.px, wp.py - 28);
            const minY = embedded ? 38 : 82;
            const maxY = viewportSize.h - (embedded ? 38 : 72);
            const clampedX = Math.max(76, Math.min(viewportSize.w - 76, sp.sx));
            const clampedY = Math.max(minY, Math.min(maxY, sp.sy));
            return { id, left: `${Math.round(clampedX)}px`, top: `${Math.round(clampedY)}px` };
          }
          const fallbacks = [
            { left: '29%', top: '69%' },
            { left: '49%', top: '52%' },
            { left: '56%', top: '65%' },
          ];
          return { id, ...fallbacks[idx % fallbacks.length]! };
        })
      : [];

  const handleBannerDrop = (agentId: string, clientX: number, clientY: number) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout) return;
    const rect = wrap.getBoundingClientRect();
    const { wx, wy } = screenToWorld(cameraRef.current, clientX - rect.left, clientY - rect.top);
    const { x: tx, y: ty } = tileFromWorld(layout, wx, wy);
    const hitPlot = Object.values(layout.byPath).find((p) => p.x === tx && p.y === ty);
    const hitDist = layout.districts.find((d) => tx >= d.x0 && tx < d.x1 && ty >= d.y0 && ty < d.y1);
    const targetPath = hitPlot?.path ?? hitDist?.buildings[0]?.path;
    if (targetPath) {
      useLudusStore.getState().updateAgentTask(agentId, {
        taskId: agentId,
        filePath: targetPath,
        status: `Assigned via Vexillum drop → ${targetPath}`,
      });
      if (hitDist) setSelection({ kind: 'district', name: hitDist.name });
      void voxTransport
        .callTool('vox_submit_task', {
          description: `Fix & inspect ${targetPath}`,
          file_affinity: [targetPath],
        })
        .catch(() => {});
    }
  };

  const occBounds = layout ? occupiedWorldBounds(layout) : null;
  const mapW = 180;
  const mapH = 104;
  const toRadarX = (wx: number) =>
    occBounds
      ? ((wx - occBounds.minX) / Math.max(1, occBounds.maxX - occBounds.minX)) * (mapW - 20) + 10
      : 90;
  const toRadarY = (wy: number) =>
    occBounds
      ? ((wy - occBounds.minY) / Math.max(1, occBounds.maxY - occBounds.minY)) * (mapH - 18) + 9
      : 52;

  const frustumRect = (() => {
    if (!layout || !occBounds) return { x: 36, y: 22, w: 108, h: 60 };
    const tl = screenToWorld(camView, 0, 0);
    const br = screenToWorld(camView, viewportSize.w, viewportSize.h);
    const rx0 = Math.max(6, Math.min(mapW - 24, toRadarX(tl.wx)));
    const ry0 = Math.max(6, Math.min(mapH - 20, toRadarY(tl.wy)));
    const rx1 = Math.max(rx0 + 24, Math.min(mapW - 6, toRadarX(br.wx)));
    const ry1 = Math.max(ry0 + 18, Math.min(mapH - 6, toRadarY(br.wy)));
    return { x: rx0, y: ry0, w: rx1 - rx0, h: ry1 - ry0 };
  })();

  const onMinimapClick = (e: React.MouseEvent<SVGSVGElement>) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout || !occBounds) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const u = Math.max(0, Math.min(1, (e.clientX - rect.left - 10) / Math.max(1, rect.width - 20)));
    const v = Math.max(0, Math.min(1, (e.clientY - rect.top - 9) / Math.max(1, rect.height - 18)));
    const wx = occBounds.minX + u * (occBounds.maxX - occBounds.minX);
    const wy = occBounds.minY + v * (occBounds.maxY - occBounds.minY);
    const zoom = cameraRef.current.zoom;
    cameraRef.current = clampCamera(
      { zoom, x: wrap.clientWidth / 2 - wx * zoom, y: wrap.clientHeight / 2 - wy * zoom },
      worldBounds(layout),
      wrap.clientWidth,
      wrap.clientHeight,
    );
    blit();
  };

  // ── Render ───────────────────────────────────────────────────────────────
  if (scanFailed) {
    return (
      <div className="flex h-full w-full items-center justify-center rounded-2xl border border-border-subtle bg-[#0c0a09] text-sm text-text-muted">
        Workspace scan unavailable — the town cannot render.
      </div>
    );
  }
  return (
    <div ref={wrapRef} className="relative h-full w-full overflow-hidden rounded-2xl border border-border-subtle bg-[#090b10]">
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
          className={`pointer-events-auto absolute ${
            embedded
              ? 'left-2 top-2 px-2 py-0.5 text-[9px]'
              : 'left-1/2 top-2.5 w-max max-w-[98%] -translate-x-1/2 px-2.5 py-1 text-[10px]'
          } z-10 flex flex-nowrap items-center justify-center gap-1.5 overflow-x-auto rounded-xl border border-brass/40 bg-[#0A0C12]/90 font-mono shadow-2xl backdrop-blur-md`}
        >
          {[
            {
              id: 'FORVM',
              label:
                layout.districts.find((dist) => dist.archetype === 'acropolis_engine' || dist.landmark)
                  ?.civicTitle ?? 'FORVM ACROPOLIS',
              lit: true,
              detail: undefined,
              onClick: () => {
                fitWorld();
                const d =
                  layout.districts.find(
                    (dist) => dist.archetype === 'acropolis_engine' || dist.landmark,
                  ) ?? layout.districts[0];
                setSelection(d ? { kind: 'district', name: d.name } : { kind: 'landmark', id: 'FORVM' });
              },
            },
            {
              id: 'PRAETORIVM',
              label:
                layout.districts.find(
                  (dist) => dist.archetype === 'praetorium_hub' || (dist.layer ?? 1) === 2,
                )?.civicTitle ?? 'PRAETORIVM HUB',
              lit: true,
              detail: undefined,
              onClick: () => {
                const d =
                  layout.districts.find(
                    (dist) => dist.archetype === 'praetorium_hub' || (dist.layer ?? 1) === 2,
                  ) ?? layout.districts[0];
                if (d) jumpToTile((d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
                setSelection({ kind: 'landmark', id: 'PRAETORIVM' });
              },
            },
            {
              id: 'AQVAE',
              label: 'AQVAE MCP',
              lit: Boolean(harness.mcp),
              detail: harness.mcp ? `${harness.mcp.filter((m) => m.ok).length}/${harness.mcp.length}` : 'offline',
              onClick: () => {
                jumpToTile(layout.landmarks.aqvae.x, layout.landmarks.aqvae.y);
                setSelection({ kind: 'landmark', id: 'AQVAE' });
              },
            },
            {
              id: 'CASTRVM',
              label: 'CASTRVM CI',
              lit: Boolean(harness.ci),
              detail: harness.ci ? `${harness.ci.runners.length}/${harness.ci.runners.length}` : 'offline',
              onClick: () => {
                jumpToTile(layout.landmarks.castrum.x, layout.landmarks.castrum.y);
                setSelection({ kind: 'landmark', id: 'CASTRVM' });
              },
            },
            {
              id: 'PORTVS',
              label: 'PORTVS QUEUE',
              lit: harness.queueLen !== null,
              detail: harness.queueLen !== null ? `${harness.queueLen}` : 'offline',
              onClick: () => {
                jumpToTile(layout.landmarks.portus.x, layout.landmarks.portus.y);
                setSelection({ kind: 'landmark', id: 'PORTVS' });
              },
            },
            {
              id: 'PORTA',
              label: 'PORTA GIT',
              lit: Boolean(harness.vcs),
              detail: harness.vcs ? `${harness.vcs.branches.find((b) => b.isHead)?.name ?? 'main'}` : 'offline',
              onClick: () => {
                jumpToTile(layout.landmarks.gate.x, layout.landmarks.gate.y);
                setSelection({ kind: 'landmark', id: 'PORTA' });
              },
            },
          ].map((lm) => (
            <button
              key={lm.id}
              type="button"
              data-urbs-landmark={lm.id}
              data-urbs-landmark-state={lm.lit ? 'lit' : 'unlit'}
              onClick={lm.onClick}
              className={`inline-flex items-center gap-1.5 whitespace-nowrap rounded-md border px-2 py-0.5 transition ${
                lm.lit
                  ? 'border-amber-500/40 bg-[#141722]/90 text-amber-100 hover:border-amber-400 hover:bg-amber-500/20'
                  : 'border-zinc-800 bg-zinc-900/70 text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <span className={`size-1.5 rounded-full ${lm.lit ? 'bg-emerald-400 shadow-[0_0_6px_#34d399]' : 'bg-zinc-600'}`} aria-hidden="true" />
              <span className="font-semibold tracking-wide">{ embedded ? lm.id : lm.label }</span>
              {!embedded && lm.detail && <span className="text-[9.5px] text-amber-300/90">· {lm.detail}</span>}
            </button>
          ))}
        </div>
      )}
      {layout && !embedded && (
        <div
          data-testid="urbs-lens-bar"
          className="pointer-events-auto absolute left-1/2 top-11 z-10 flex w-max max-w-[98%] -translate-x-1/2 flex-nowrap items-center gap-1.5 overflow-x-auto rounded-lg border border-white/10 bg-[#0A0C12]/88 px-2 py-0.5 font-mono text-[9.5px] shadow-xl backdrop-blur-md"
        >
          <span className="mr-0.5 text-[9px] font-semibold uppercase tracking-widest text-brass">Repo:</span>
          <select
            data-testid="urbs-repo-preset-select"
            aria-label="Repository simulation preset"
            value={presetId}
            onChange={(e) => {
              setPresetId(e.target.value as UrbsRepoPresetId);
              setSelection(null);
            }}
            className="rounded border border-amber-500/40 bg-[#141722] px-1.5 py-0.5 font-mono text-[9.5px] text-amber-100 focus:border-amber-400 focus:outline-none"
          >
            {Object.values(URBS_REPO_PRESETS).map((preset) => (
              <option key={preset.id} value={preset.id}>
                {preset.label}
              </option>
            ))}
          </select>

          <span className="ml-1 text-[9px] font-semibold uppercase tracking-widest text-brass">Focus:</span>
          <select
            data-testid="urbs-focus-mode-select"
            aria-label="Real-estate attention focus mode"
            value={focusMode}
            onChange={(e) => setFocusMode(e.target.value as RealEstateFocusMode)}
            className="rounded border border-cyan-500/40 bg-[#141722] px-1.5 py-0.5 font-mono text-[9.5px] text-cyan-100 focus:border-cyan-400 focus:outline-none"
          >
            <option value="attention">Attention Weighted (65%+ Worksite)</option>
            <option value="worksite_only">Worksite Only (Compact Dormant)</option>
            <option value="proportional">Proportional Raw LoC</option>
          </select>

          <span className="ml-1 text-[9px] font-semibold uppercase tracking-widest text-brass">Lens:</span>
          {(
            [
              { id: 'architecture', label: '🏛 Architecture & Layers' },
              { id: 'health', label: '🔥 Health & Diagnostics' },
              { id: 'agents', label: '⚔ Live Agents & Locks' },
              { id: 'pipeline', label: '⛵ Task Pipeline' },
              { id: 'cost', label: '🪙 Token Spend Heatmap' },
            ] as const
          ).map((item) => {
            const active = lens === item.id;
            return (
              <button
                key={item.id}
                type="button"
                data-urbs-lens={item.id}
                onClick={() => setLens(item.id)}
                className={`whitespace-nowrap rounded border px-2 py-0.5 transition ${
                  active
                    ? 'border-amber-400/80 bg-amber-500/20 font-semibold text-amber-100 shadow-[0_0_10px_rgba(245,158,11,0.25)]'
                    : 'border-zinc-800/80 bg-zinc-900/60 text-zinc-400 hover:border-zinc-700 hover:text-zinc-200'
                }`}
              >
                {item.label}
                {active ? ' (ACTIVE)' : ''}
              </button>
            );
          })}
        </div>
      )}
      {layout && !embedded && (
        <div
          data-testid="urbs-priority-dock"
          className="pointer-events-auto absolute right-2 top-20 z-10 w-[238px] rounded-xl border border-amber-500/40 bg-[#0A0C14]/92 p-2.5 text-zinc-100 shadow-2xl backdrop-blur-md"
        >
          <div className="mb-1.5 flex items-center justify-between border-b border-white/10 pb-1">
            <span className="font-mono text-[9px] font-semibold uppercase tracking-widest text-amber-300">
              ⚡ Developer Priority Dock
            </span>
            <span className="font-mono text-[8.5px] text-zinc-400">
              {activePreset.language}
            </span>
          </div>
          <div className="max-h-48 space-y-1 overflow-y-auto pr-0.5">
            {activeGates.map((gate) => {
              const dist = layout.districts.find((d) => d.name === gate.district) ?? layout.districts[0];
              return (
                <div
                  key={`gate-${gate.id}`}
                  className="flex w-full items-center justify-between gap-1 rounded border border-amber-400/60 bg-amber-500/15 px-2 py-1 font-mono text-[9px] text-amber-100"
                >
                  <button
                    type="button"
                    data-testid="urbs-priority-item-gate"
                    onClick={() => {
                      if (dist) {
                        jumpToTile((dist.x0 + dist.x1) / 2, (dist.y0 + dist.y1) / 2);
                        setSelection({ kind: 'district', name: dist.name });
                      }
                    }}
                    className="min-w-0 flex-1 truncate text-left hover:text-amber-200"
                  >
                    🔔 {gate.title}
                  </button>
                  <button
                    type="button"
                    data-testid={`urbs-approve-gate-${gate.id}`}
                    onClick={() => {
                      setResolvedGateIds((prev) => new Set([...prev, gate.id]));
                    }}
                    className="shrink-0 rounded border border-emerald-400/60 bg-emerald-500/20 px-1.5 py-0.5 text-[8px] font-semibold text-emerald-200 hover:bg-emerald-500/35"
                    title="Approve decision gate & raise Portcullis"
                  >
                    ✓ RAISE
                  </button>
                </div>
              );
            })}
            {Object.entries(buildings)
              .filter(([, diag]) => diag.errors > 0)
              .map(([path, diag]) => {
                const plot = layout.byPath[path];
                const dist = layout.districts.find((d) => d.buildings.some((b) => b.path === path));
                const shortName = path.split('/').pop() ?? path;
                return (
                  <div
                    key={`err-${path}`}
                    className="flex w-full items-center justify-between gap-1 rounded border border-rose-500/60 bg-rose-500/15 px-2 py-1 font-mono text-[9px] text-rose-100"
                  >
                    <button
                      type="button"
                      data-testid="urbs-priority-item-error"
                      onClick={() => {
                        if (plot) jumpToTile(plot.x, plot.y);
                        if (dist) setSelection({ kind: 'district', name: dist.name });
                      }}
                      className="min-w-0 flex-1 truncate text-left hover:text-rose-200"
                    >
                      🔥 {shortName} ({diag.errors} ERR)
                    </button>
                    <button
                      type="button"
                      data-testid="urbs-priority-open-loupe"
                      onClick={() => {
                        openSpeculumLoupe(path, Math.round(viewportSize.w * 0.52), Math.round(viewportSize.h * 0.36));
                      }}
                      className="shrink-0 rounded border border-amber-400/60 bg-amber-500/20 px-1.5 py-0.5 text-[8px] font-semibold text-amber-200 hover:bg-amber-500/35"
                      title="Open Speculum Codicis Diff & Diagnostic Loupe"
                    >
                      🔍 LOUPE
                    </button>
                  </div>
                );
              })}
            {Object.entries(agentTasks).map(([agentId, task]) => {
              const plot = layout.byPath[task.filePath];
              const dist = layout.districts.find((d) =>
                d.buildings.some((b) => b.path === task.filePath),
              );
              const shortName = task.filePath.split('/').pop() ?? task.filePath;
              return (
                <button
                  key={`task-${agentId}-${task.taskId}`}
                  type="button"
                  data-testid="urbs-priority-item-agent"
                  onClick={() => {
                    if (plot) jumpToTile(plot.x, plot.y);
                    if (dist) setSelection({ kind: 'district', name: dist.name });
                  }}
                  className="flex w-full items-center justify-between gap-1 rounded border border-cyan-500/50 bg-cyan-500/15 px-2 py-1 text-left font-mono text-[9px] text-cyan-100 hover:bg-cyan-500/25"
                >
                  <span className="truncate">
                    ⚙ {agentId} → {shortName}
                  </span>
                  <span className="shrink-0 text-[8.5px] text-cyan-300">ACTIVE</span>
                </button>
              );
            })}
            {layout.districts
              .flatMap((d) =>
                d.buildings
                  .filter((b) => b.isTouched)
                  .map((b) => ({ district: d.name, plot: b })),
              )
              .slice(0, 4)
              .map(({ district, plot }) => {
                const shortName = plot.path.split('/').pop() ?? plot.path;
                return (
                  <button
                    key={`diff-${plot.path}`}
                    type="button"
                    data-testid="urbs-priority-item-diff"
                    onClick={() => {
                      jumpToTile(plot.x, plot.y);
                      setSelection({ kind: 'district', name: district });
                      openSpeculumLoupe(plot.path, Math.round(viewportSize.w * 0.52), Math.round(viewportSize.h * 0.36));
                    }}
                    className="flex w-full items-center justify-between gap-1 rounded border border-emerald-500/40 bg-emerald-500/10 px-2 py-1 text-left font-mono text-[9px] text-emerald-100 hover:bg-emerald-500/20"
                  >
                    <span className="truncate">
                      ✎ {district}/{shortName}
                    </span>
                    <span className="shrink-0 text-[8.5px] text-emerald-300">
                      +{plot.diffAdded ?? 12}/-{plot.diffDeleted ?? 3}
                    </span>
                  </button>
                );
              })}
          </div>
        </div>
      )}
      {layout && !embedded && selection && (
        <ImperiumInspector
          selection={selection}
          layout={layout}
          harness={harness}
          buildings={buildings}
          agentTasks={agentTasks}
          scanRoot={scanRoot}
          quarantinedDistricts={quarantinedDistrictsList}
          onToggleQuarantine={toggleDistrictQuarantine}
          collisionZones={collisionRadar.collisions}
          onSerializeCollision={(zone) => {
            if (zone.agentB) {
              useLudusStore.getState().updateAgentTask(zone.agentB, null);
            }
          }}
          onSplitWorktreeCollision={(zone) => {
            void worktreeSpawnVariant(`urbs-split-${zone.districtId}`, [
              zone.agentA,
              zone.agentB,
            ]).catch(() => {});
          }}
          onSelect={setSelection}
          onJumpToTile={jumpToTile}
          onRefreshHarness={() => {
            fetchHarnessSnapshot().then(setHarness).catch(() => {});
          }}
        />
      )}
      {loupeTarget && !embedded && (
        <div
          className="pointer-events-auto absolute z-30"
          style={{
            left: Math.max(12, Math.min(viewportSize.w - 408, loupeTarget.sx)),
            top: Math.max(54, Math.min(viewportSize.h - 260, loupeTarget.sy)),
          }}
        >
          <SpeculumLoupePopover
            diffData={loupeTarget.data}
            onClose={() => setLoupeTarget(null)}
            onStageFile={(path) => {
              void stageWorkspaceFile(path).catch(() => {});
              setLoupeTarget(null);
            }}
            onRevertFile={(path) => {
              void revertWorkspaceFile(path).catch(() => {});
              setLoupeTarget(null);
            }}
            onDispatchFix={(path, firstDiag) => {
              const detail = firstDiag
                ? `${firstDiag.code ?? 'error'} at ${path}:${firstDiag.line}:${firstDiag.column} — ${firstDiag.message}`
                : `Fix diagnostics in ${path}`;
              void voxTransport
                .callTool('vox_submit_task', {
                  description: `Fix compiler diagnostic: ${detail}`,
                  file_affinity: [path],
                })
                .catch(() => {});
              setLoupeTarget(null);
            }}
          />
        </div>
      )}
      {/* Citizens & Draggable Roman Legion Vexillum Standard Banners */}
      <div className="pointer-events-none absolute inset-0">
        {activeCitizenIds.map((id) => (
          <div
            key={id}
            ref={(el) => { if (el) spriteEls.current.set(id, el); else spriteEls.current.delete(id); }}
            className="absolute will-change-transform"
          >
            <div className="flex flex-col items-center">
              <RomanCitizenSvg />
              <span className="rounded-sm border border-cyan-400/60 bg-[#0B131E]/95 px-1.5 py-0.5 font-mono text-[9px] text-cyan-100 shadow-md">
                {id} · {agentTasks[id]?.filePath.split('/').pop() ?? 'active'}
              </span>
            </div>
          </div>
        ))}
        {ambientRosters.map((c) => {
          const meta = vexillumMeta[c.id] ?? {
            legion: c.id,
            model: 'Claude Sonnet 4.5',
            task: 'Patrolling Via Appia',
            border: 'border-amber-400/70 bg-[#0D0F17]/95 text-amber-100',
          };
          return (
            <div
              key={c.id}
              data-testid="urbs-vexillum-banner"
              ref={(el) => { if (el) spriteEls.current.set(c.id, el); else spriteEls.current.delete(c.id); }}
              style={{ left: c.left, top: c.top }}
              onPointerDown={(ev) => {
                if (isCompactViewport) return;
                ev.stopPropagation();
                (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
                const rect = wrapRef.current?.getBoundingClientRect();
                const sx = rect ? ev.clientX - rect.left : 0;
                const sy = rect ? ev.clientY - rect.top : 0;
                setDragBanner({ id: c.id, sx, sy, cx: sx, cy: sy });
              }}
              onPointerMove={(ev) => {
                if (!dragBanner || dragBanner.id !== c.id) return;
                ev.stopPropagation();
                const rect = wrapRef.current?.getBoundingClientRect();
                if (!rect) return;
                setDragBanner({
                  ...dragBanner,
                  cx: ev.clientX - rect.left,
                  cy: ev.clientY - rect.top,
                });
              }}
              onPointerUp={(ev) => {
                if (!dragBanner || dragBanner.id !== c.id) return;
                ev.stopPropagation();
                handleBannerDrop(c.id, ev.clientX, ev.clientY);
                setDragBanner(null);
              }}
              className={`absolute -translate-x-1/2 -translate-y-full transition-[left,top] duration-150 ease-linear ${
                isCompactViewport ? 'pointer-events-none' : 'pointer-events-auto cursor-grab active:cursor-grabbing'
              }`}
            >
              <div className="group flex flex-col items-center">
                {!isCompactViewport && (
                  <div className="mb-0.5 flex items-center gap-1">
                    {/* Hanging Crimson & Brass Roman Vexillum Standard Flag (Concept Art B) */}
                    <svg viewBox="0 0 22 30" className="h-6 w-4 shrink-0 drop-shadow-[0_2px_4px_rgba(0,0,0,0.85)]" aria-hidden="true">
                      <line x1="11" y1="1" x2="11" y2="29" stroke="#d4af37" strokeWidth="1.6" />
                      <line x1="2" y1="4" x2="20" y2="4" stroke="#f59e0b" strokeWidth="2" />
                      <path d="M3 4H19V22L15 20L11 23L7 20L3 22V4Z" fill="#991b1b" stroke="#fbbf24" strokeWidth="1.1" />
                      <circle cx="11" cy="12" r="3.6" fill="none" stroke="#fde68a" strokeWidth="1.1" />
                      <path d="M9 10.5L13 13.5M13 10.5L9 13.5" stroke="#fef08a" strokeWidth="1" />
                    </svg>
                    <div className={`rounded border px-1.5 py-0.5 font-mono shadow-lg backdrop-blur-sm ${meta.border}`}>
                      <div className="leading-tight">
                        <div className="text-[8.5px] font-bold tracking-wide">
                          {meta.legion} · {meta.model}
                        </div>
                        <div className="hidden text-[8px] opacity-90 group-hover:block">{meta.task}</div>
                      </div>
                    </div>
                  </div>
                )}
                <RomanCitizenSvg />
              </div>
            </div>
          );
        })}
        {dragBanner && (
          <svg className="pointer-events-none absolute inset-0 h-full w-full">
            <line
              x1={dragBanner.sx}
              y1={dragBanner.sy}
              x2={dragBanner.cx}
              y2={dragBanner.cy}
              stroke="#fbbf24"
              strokeWidth="2.2"
              strokeDasharray="5 3"
            />
            <circle
              cx={dragBanner.cx}
              cy={dragBanner.cy}
              r="18"
              fill="rgba(251, 191, 36, 0.18)"
              stroke="#fbbf24"
              strokeWidth="2"
            />
          </svg>
        )}
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
      {layout && !embedded && (
        <div
          data-testid="urbs-minimap-radar"
          className="pointer-events-auto absolute bottom-2 right-2 z-10 w-[196px] rounded-xl border border-brass/35 bg-[#0D0F17]/92 p-2 shadow-2xl backdrop-blur-md"
        >
          <div className="flex items-center justify-between px-0.5 pb-1">
            <span className="font-mono text-[9px] font-semibold uppercase tracking-widest text-brass">
              Strategic Minimap Radar
            </span>
            <span className="font-mono text-[9px] text-zinc-400">
              {layout.districts.length} districts
            </span>
          </div>
          <svg
            viewBox={`0 0 ${mapW} ${mapH}`}
            onClick={onMinimapClick}
            className="h-[104px] w-full cursor-crosshair rounded-lg border border-white/10 bg-[#07090E]"
            aria-label="Strategic Minimap Radar"
          >
            {/* Thermal & Activity Heatmap Blobs (Concept Art B Circular Radar) */}
            <defs>
              <radialGradient id="urbs-radar-heat-hot" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stopColor="#ef4444" stopOpacity="0.65" />
                <stop offset="55%" stopColor="#f59e0b" stopOpacity="0.32" />
                <stop offset="100%" stopColor="#10b981" stopOpacity="0" />
              </radialGradient>
              <radialGradient id="urbs-radar-heat-cyan" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stopColor="#22d3ee" stopOpacity="0.55" />
                <stop offset="100%" stopColor="#22d3ee" stopOpacity="0" />
              </radialGradient>
            </defs>
            <circle cx={mapW * 0.62} cy={mapH * 0.56} r="28" fill="url(#urbs-radar-heat-hot)" />
            <circle cx={mapW * 0.44} cy={mapH * 0.36} r="24" fill="url(#urbs-radar-heat-cyan)" />
            {/* Cyan AQVAE Aqueduct & Harbor Water Channel */}
            <path
              d={`M 14 14 Q ${mapW * 0.55} 24 ${mapW - 14} 42 M 12 ${mapH - 26} Q 48 ${mapH - 12} 78 ${mapH - 8}`}
              stroke="#22d3ee"
              strokeWidth="2"
              strokeOpacity="0.65"
              fill="none"
            />
            {/* Isometric District Polygons & Building Blips */}
            {layout.districts.map((d) => {
              const p0 = worldPx(layout, d.x0, d.y0);
              const p1 = worldPx(layout, d.x1, d.y0);
              const p2 = worldPx(layout, d.x1, d.y1);
              const p3 = worldPx(layout, d.x0, d.y1);
              const pts = [p0, p1, p2, p3]
                .map((p) => `${toRadarX(p.px).toFixed(1)},${toRadarY(p.py).toFixed(1)}`)
                .join(' ');
              return (
                <g key={d.name}>
                  <polygon
                    points={pts}
                    fill={d.landmark ? 'rgba(229, 169, 60, 0.16)' : 'rgba(148, 163, 184, 0.10)'}
                    stroke={d.landmark ? '#e5a93c' : 'rgba(148, 163, 184, 0.45)'}
                    strokeWidth="1"
                  />
                  {d.buildings.slice(0, 6).map((b) => {
                    const bp = worldPx(layout, b.x, b.y);
                    return (
                      <circle
                        key={b.path}
                        cx={toRadarX(bp.px)}
                        cy={toRadarY(bp.py)}
                        r={b.tier >= 2 ? 1.8 : 1.3}
                        fill="#fbbf24"
                      />
                    );
                  })}
                </g>
              );
            })}
            {/* Live Patrolling Citizen Blips on Strategic Minimap Radar */}
            {layout.districts.slice(0, 3).map((d, idx) => {
              const citizen = Object.values(citizensRef.current)[idx];
              const wp = citizen
                ? worldPx(layout, citizen.pos.x, citizen.pos.y)
                : worldPx(layout, (d.x0 + d.x1) / 2, d.y1 - 0.6);
              return (
                <circle
                  key={`radar-cit-${idx}`}
                  cx={toRadarX(wp.px)}
                  cy={toRadarY(wp.py)}
                  r="2.4"
                  fill="#34d399"
                  stroke="#064e3b"
                  strokeWidth="0.8"
                >
                  <animate attributeName="r" values="2;3.2;2" dur="1.6s" repeatCount="indefinite" />
                </circle>
              );
            })}
            {/* Camera Viewport Frustum Box */}
            <rect
              data-testid="urbs-minimap-frustum"
              x={frustumRect.x}
              y={frustumRect.y}
              width={frustumRect.w}
              height={frustumRect.h}
              rx="2"
              fill="rgba(229, 169, 60, 0.10)"
              stroke="#e5a93c"
              strokeWidth="1.2"
            />
          </svg>
        </div>
      )}
      {layout && !embedded && !selection && (
        <div className="pointer-events-auto absolute bottom-11 left-1/2 z-10 -translate-x-1/2">
          <ViaTemporisScrubber
            milestones={viaMilestones}
            selectedTurn={selectedTurn}
            activeHeadTurn={viaMilestones[viaMilestones.length - 1]?.turnNumber ?? 3}
            onSelectTurn={(turnNum) => {
              setSelectedTurn(turnNum);
              void vcsCheckpointDiffSummary('default', turnNum)
                .then((summary) => {
                  const firstPath = summary.files[0]?.file_path;
                  const plot = firstPath ? layout.byPath[firstPath] : undefined;
                  if (plot) jumpToTile(plot.x, plot.y);
                })
                .catch(() => {});
            }}
            onCaptureCheckpoint={() => {
              void vcsCheckpointCapture('default', viaMilestones.length + 1, 'via-temporis').catch(
                () => {},
              );
            }}
            onRollbackToTurn={(turnNum) => {
              void vcsCheckpointRestore('default', turnNum).catch(() => {});
            }}
          />
        </div>
      )}
      <div
        className={`absolute ${
          embedded ? 'bottom-1.5 left-2' : 'bottom-2 left-1/2 -translate-x-1/2'
        } z-10 flex items-center gap-1.5`}
      >
        <HudPanels
          treasuryUsd={treasuryUsd}
          energy={energy}
          maxEnergy={maxEnergy}
          speed={speed}
          onSetSpeed={setSpeed}
          embedded={embedded}
        />
        <button
          type="button"
          onClick={fitWorld}
          className="pointer-events-auto rounded-md border border-brass/30 bg-[#0D0F17]/90 px-2 py-1 font-mono text-[10px] text-amber-200 hover:bg-brass/20"
        >
          ⌂ fit
        </button>
        {!embedded && (
          <>
            <button
              type="button"
              aria-label="Zoom out"
              onClick={() => stepZoom(1 / 1.2)}
              className="pointer-events-auto rounded-md border border-zinc-800 bg-[#0D0F17]/90 px-2 py-1 font-mono text-[10px] text-zinc-300 hover:border-brass/40 hover:text-amber-200"
            >
              −
            </button>
            <button
              type="button"
              aria-label="Zoom in"
              onClick={() => stepZoom(1.2)}
              className="pointer-events-auto rounded-md border border-zinc-800 bg-[#0D0F17]/90 px-2 py-1 font-mono text-[10px] text-zinc-300 hover:border-brass/40 hover:text-amber-200"
            >
              +
            </button>
          </>
        )}
      </div>
    </div>
  );
};
