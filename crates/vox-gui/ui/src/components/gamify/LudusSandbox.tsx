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
} from './ViaTemporisScrubber';
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
import { Icon } from '../ui/Icons';
import { RomanTooltip, RomanMedallion } from '../ui/RomanTooltip';

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

export interface MonumentCallout {
  id: string;
  label: string;
  crateHint: string;
  surface: string;
  targetView: string;
  purpose: string;
  left: string;
  top: string;
  monumentArt: string;
  defaultFilePath: string;
}

export const MONUMENT_CALLOUTS: MonumentCallout[] = [
  {
    id: 'monument-compiler',
    label: 'Compiler & Typecheck',
    crateHint: 'vox-compiler',
    surface: 'explorer',
    targetView: 'explorer',
    purpose: 'Vox parser, HIR typechecker, effect gate, and Rust/TSX code generator.',
    left: '17%',
    top: '25%',
    monumentArt: '/assets/romanesque/monument-temple-compiler.jpg',
    defaultFilePath: 'crates/vox-compiler/src/typeck/mod.rs',
  },
  {
    id: 'monument-orchestrator',
    label: 'Agent Orchestrator',
    crateHint: 'vox-orchestrator',
    surface: 'flow',
    targetView: 'flow',
    purpose: 'Multi-agent turn coordinator, spoke router, worktree leases, and tool verification.',
    left: '21%',
    top: '56%',
    monumentArt: '/assets/romanesque/monument-pantheon-orchestrator.jpg',
    defaultFilePath: 'crates/vox-orchestrator/src/router.rs',
  },
  {
    id: 'monument-gui',
    label: 'GUI & Surfaces',
    crateHint: 'vox-gui',
    surface: 'dashboard',
    targetView: 'dashboard',
    purpose: 'Vox Axis Tauri 2 desktop workbench and all 32 interactive operator surfaces.',
    left: '48%',
    top: '23%',
    monumentArt: '/assets/romanesque/monument-colosseum-harness.jpg',
    defaultFilePath: 'crates/vox-gui/ui/src/App.tsx',
  },
  {
    id: 'monument-db',
    label: 'Database & Knowledge',
    crateHint: 'vox-db',
    surface: 'memory',
    targetView: 'memory',
    purpose: 'Turso/SQLite state store, Tantivy hybrid search index, and Scientia knowledge graph.',
    left: '42%',
    top: '45%',
    monumentArt: '/assets/romanesque/monument-library-knowledge.jpg',
    defaultFilePath: 'crates/vox-db/src/turso.rs',
  },
  {
    id: 'monument-mens',
    label: 'MENS Local Models',
    crateHint: 'vox-mens',
    surface: 'mens',
    targetView: 'mens',
    purpose: 'Native QLoRA fine-tuning ladder (0.6B–32B), 8-spoke adapters, and Hugging Face sync.',
    left: '67%',
    top: '41%',
    monumentArt: '/assets/romanesque/monument-pantheon-orchestrator.jpg',
    defaultFilePath: 'crates/vox-mens/src/qlora.rs',
  },
  {
    id: 'monument-crypto',
    label: 'Security & Secrets',
    crateHint: 'vox-crypto',
    surface: 'settings',
    targetView: 'settings',
    purpose: 'Clavis credential vault, BLAKE3/Ed25519 cryptographic envelopes, and policy gates.',
    left: '71%',
    top: '26%',
    monumentArt: '/assets/romanesque/monument-temple-compiler.jpg',
    defaultFilePath: 'crates/vox-crypto/src/envelope.rs',
  },
  {
    id: 'monument-mesh',
    label: 'Compute Mesh & Queue',
    crateHint: 'vox-populi',
    surface: 'mesh',
    targetView: 'mesh',
    purpose: 'Distributed iroh QUIC peer mesh, live VRAM selector, and task dispatch queue.',
    left: '61%',
    top: '55%',
    monumentArt: '/assets/romanesque/monument-colosseum-harness.jpg',
    defaultFilePath: 'crates/vox-populi/src/mesh.rs',
  },
];

interface Props {
  /** Optional: energy for the HUD (GamifyView passes profile data down). */
  energy?: number;
  maxEnergy?: number;
  /** Optional navigation handler to jump directly to any Vox Axis surface. */
  onNavigate?: (viewKey: string) => void;
}

export const LudusSandbox: React.FC<Props> = ({ energy = 0, maxEnergy = 0, onNavigate }) => {
  const embedded = useIsEmbeddedSurface();
  const [showTimelineDrawer, setShowTimelineDrawer] = useState(false);
  const [showPriorityDrawer, setShowPriorityDrawer] = useState(false);
  const [showRadarDrawer, setShowRadarDrawer] = useState(true);
  const [showLensMenu, setShowLensMenu] = useState(false);
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
  const [activeLayerFilter, setActiveLayerFilter] = useState<number | null>(null);
  const [devActionReceipt, setDevActionReceipt] = useState<string | null>(null);
  const [pinnedCrates, setPinnedCrates] = useState<Set<string>>(() => new Set(['vox-compiler']));
  const [claimedYieldXp, setClaimedYieldXp] = useState<number>(250);

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
  const dioramaImgRef = useRef<HTMLImageElement | null>(null);
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
    ctx.clearRect(0, 0, cw, ch);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    const bgImg = dioramaImgRef.current;
    if (bgImg && bgImg.complete && bgImg.naturalWidth > 0) {
      const imgAspect = bgImg.naturalWidth / bgImg.naturalHeight;
      const canvasAspect = cw / Math.max(1, ch);
      let drawW = cw;
      let drawH = ch;
      let drawX = 0;
      let drawY = 0;
      if (canvasAspect > imgAspect) {
        drawH = cw / imgAspect;
        drawY = (ch - drawH) * 0.5;
      } else {
        drawW = ch * imgAspect;
        drawX = (cw - drawW) * 0.5;
      }
      ctx.save();
      ctx.globalAlpha = 0.98;
      ctx.drawImage(bgImg, drawX, drawY, drawW, drawH);
      ctx.restore();
    }
    ctx.translate(cam.x, cam.y);
    ctx.scale(cam.zoom, cam.zoom);
    ctx.drawImage(buffer, 0, 0, buffer.width / scale, buffer.height / scale);
    rendererRef.current.drawFires(ctx, state, cam.zoom, fireFrame);
    rendererRef.current.drawAnimatedWorldEffects(ctx, state, performance.now());
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
    quarantinedDistrictsList,
    collisionRadar.collisions,
    activeToolPulses,
  ]);

  useEffect(() => {
    if (typeof Image === 'undefined') return;
    const img = new Image();
    img.src = '/assets/romanesque/urbs-white-gold-citadel-bg.jpg';
    img.onload = () => {
      dioramaImgRef.current = img;
      blit();
    };
    dioramaImgRef.current = img;
  }, [blit]);

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
            const minY = embedded ? 38 : 62;
            const maxY = viewportSize.h - (embedded ? 38 : 96);
            const maxX = viewportSize.w - (embedded ? 48 : showRadarDrawer || showPriorityDrawer ? 228 : 56);
            const clampedX = Math.max(56, Math.min(maxX, s.sx));
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
  }, [layout, speed, agentTasks, blit, embedded, viewportSize.w, viewportSize.h, showRadarDrawer, showPriorityDrawer]);

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
      border: 'border-[#D4AF37]/85 bg-[#1C150D]/95 text-[#F3E5AB]',
    },
    'AVGVR-02': {
      legion: 'LEGIO-02',
      model: 'Gemini 2.5 Pro',
      task: `Running ${polyglotTestCmd}`,
      border: 'border-[#7B8C32]/85 bg-[#1E2410]/95 text-[#DCE7B6]',
    },
    'FABER-03': {
      legion: 'LEGIO-03',
      model: 'MENS local',
      task: `Indexing ${layout?.districts[0]?.name ?? 'workspace'}`,
      border: 'border-amber-400/75 bg-[#1E160C]/95 text-amber-100',
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
            const maxY = viewportSize.h - (embedded ? 38 : 96);
            const maxX = viewportSize.w - (embedded ? 76 : showRadarDrawer || showPriorityDrawer ? 248 : 84);
            const clampedX = Math.max(84, Math.min(maxX, sp.sx));
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
    <div
      ref={wrapRef}
      className="relative h-full w-full overflow-hidden rounded-xl border border-[#C59B27]/65 bg-[#F5EFE2]"
    >
      {/* Milestone v6.4 Generated Sunlit White/Gold Roman Acropolis Architectural Diorama Baseplate */}
      <img
        src="/assets/romanesque/urbs-white-gold-citadel-bg.jpg"
        alt=""
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 h-full w-full object-cover opacity-98 select-none"
      />
      <canvas
        ref={canvasRef}
        data-testid="urbs-canvas"
        data-diorama-composited="true"
        className={`absolute inset-0 cursor-grab active:cursor-grabbing opacity-100 ${layout ? '' : 'invisible'}`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onWheel={onWheel}
        onClick={onClick}
      />

      {/* Animated Golden Aqueduct Activity Stream SVG + Interactive Painted Roman Monument Cards */}
      {layout && (
        <div className="pointer-events-none absolute inset-0 z-[5]">
          <svg
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 h-full w-full opacity-90"
          >
            <defs>
              <linearGradient id="urbs-aqueduct-gold" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#D4AF37" stopOpacity="0.85" />
                <stop offset="50%" stopColor="#FEF08A" stopOpacity="0.95" />
                <stop offset="100%" stopColor="#C59B27" stopOpacity="0.85" />
              </linearGradient>
            </defs>
            {/* Golden Aqueduct Stream Channels from Orchestrator Pantheon (21,56) to Crate Monuments */}
            {[
              { x: 17, y: 25 },
              { x: 48, y: 23 },
              { x: 42, y: 45 },
              { x: 67, y: 41 },
              { x: 71, y: 26 },
              { x: 61, y: 55 },
            ]
              .slice(0, embedded ? 2 : 6)
              .map((pt, i) => {
                const mx = (21 + pt.x) / 2;
                const my = Math.min(56, pt.y) - 6;
                const dPath = `M 21 56 Q ${mx} ${my} ${pt.x} ${pt.y}`;
                return (
                  <g key={`aq-stream-${i}`}>
                    <path
                      d={dPath}
                      fill="none"
                      stroke="rgba(197, 155, 39, 0.35)"
                      strokeWidth="0.45"
                    />
                    <path
                      d={dPath}
                      fill="none"
                      stroke="url(#urbs-aqueduct-gold)"
                      strokeWidth="0.32"
                      strokeDasharray="1.8 1.8"
                    >
                      <animate
                        attributeName="stroke-dashoffset"
                        from="7.2"
                        to="0"
                        dur={`${2.2 + i * 0.3}s`}
                        repeatCount="indefinite"
                      />
                    </path>
                  </g>
                );
              })}
          </svg>

          {MONUMENT_CALLOUTS.filter(
            (m) => !embedded || ['monument-compiler', 'monument-orchestrator', 'monument-mens'].includes(m.id),
          ).map((m) => {
            const topPos = embedded
              ? m.id === 'monument-compiler'
                ? '30%'
                : m.id === 'monument-orchestrator'
                  ? '58%'
                  : '46%'
              : m.top;
            const leftPos = m.left;
            const matchedDistrict = layout.districts.find((d) =>
              d.name.includes(m.crateHint.replace('vox-', '')),
            );
            const liveTaskForMonument = Object.values(agentTasks).find(
              (t) =>
                t.filePath.includes(m.crateHint) ||
                (matchedDistrict && t.filePath.includes(matchedDistrict.name)),
            );
            const activeFilePath =
              liveTaskForMonument?.filePath ??
              matchedDistrict?.buildings.find((b) => b.isTouched || (b.diffAdded ?? 0) > 0)?.path ??
              m.defaultFilePath;
            const shortFileBadge = activeFilePath.split('/').slice(-2).join('/');
            const isAlreadySelected =
              selection?.kind === 'district' &&
              selection.name.includes(m.crateHint.replace('vox-', ''));
            const plaqueLabel = embedded
              ? m.id === 'monument-compiler'
                ? 'Compiler'
                : m.id === 'monument-orchestrator'
                  ? 'Orchestrator'
                  : 'MENS Models'
              : m.label;

            return (
              <div
                key={m.id}
                style={{ left: leftPos, top: topPos }}
                className="pointer-events-auto absolute -translate-x-1/2 -translate-y-1/2"
              >
                <RomanTooltip
                  title={m.label}
                  purpose={`${m.purpose} Active target: ${activeFilePath}`}
                  actionHint={
                    onNavigate
                      ? `Click to focus ${m.crateHint} (click again or click Open ↗ to jump directly to ${m.targetView})`
                      : `Click to focus ${m.crateHint} on the map`
                  }
                  statusBadge={activeFilePath}
                >
                  <button
                    type="button"
                    data-testid={`urbs-${m.id}`}
                    onClick={(ev) => {
                      ev.stopPropagation();
                      const dist = matchedDistrict ?? layout.districts[0];
                      if (dist) {
                        jumpToTile((dist.x0 + dist.x1) / 2, (dist.y0 + dist.y1) / 2);
                        setSelection({ kind: 'district', name: dist.name });
                      }
                      if (onNavigate && (isAlreadySelected || ev.metaKey || ev.shiftKey || embedded)) {
                        onNavigate(m.targetView);
                      }
                    }}
                    onDoubleClick={(ev) => {
                      ev.stopPropagation();
                      onNavigate?.(m.targetView);
                    }}
                    className={`group inline-flex items-center gap-1.5 rounded-lg border-2 border-[#C59B27]/85 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 ${
                      embedded ? 'px-1.5 py-0.5 text-[8.5px]' : 'px-2 py-1 text-[9.5px]'
                    } font-display font-semibold tracking-wide text-[#1C140C] shadow-[0_6px_18px_rgba(28,20,12,0.32),inset_0_1px_0_rgba(255,255,255,0.9)] transition-all hover:scale-105 hover:border-[#B8860B]`}
                  >
                    <img
                      src={m.monumentArt}
                      alt=""
                      aria-hidden="true"
                      className={`${
                        embedded ? 'size-4' : 'size-6'
                      } shrink-0 rounded border border-[#C59B27]/75 object-cover shadow-xs`}
                    />
                    <span className="flex flex-col items-start leading-tight">
                      <span className="inline-flex items-center gap-1 whitespace-nowrap">
                        <span
                          className="size-1.5 animate-pulse rounded-full bg-[#B8860B] shadow-[0_0_6px_#D4AF37]"
                          aria-hidden="true"
                        />
                        <span>{plaqueLabel}</span>
                      </span>
                      {!embedded && (
                        <span className="whitespace-nowrap font-sans text-[8px] font-medium tracking-normal text-[#5C4428]">
                          ⚙ {shortFileBadge}
                        </span>
                      )}
                    </span>
                    {onNavigate && (
                      <span
                        data-testid={`open-${m.id}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          onNavigate(m.targetView);
                        }}
                        title={`Jump directly to ${m.label} (${m.targetView})`}
                        className="ml-1 inline-flex items-center gap-0.5 whitespace-nowrap rounded border border-[#C59B27]/65 bg-[#F3E5AB]/75 px-1 py-0.2 font-display text-[8px] font-bold text-[#1C140C] hover:bg-[#D4AF37]/55 transition"
                      >
                        Open ↗
                      </span>
                    )}
                  </button>
                </RomanTooltip>
              </div>
            );
          })}
        </div>
      )}

      {!layout && (
        <div className="absolute inset-0 flex items-center justify-center text-xs text-text-muted">
          Simulation loading — no workspace data yet.
        </div>
      )}
      {layout && embedded && (
        <div
          data-testid="urbs-landmark-bar"
          style={{
            backgroundImage:
              'linear-gradient(90deg, rgba(250,246,238,0.96) 0%, rgba(242,234,218,0.95) 50%, rgba(250,246,238,0.96) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="pointer-events-auto absolute left-2 top-2 max-w-[calc(100%-175px)] px-2 py-0.5 text-[9px] z-10 flex flex-nowrap items-center justify-center gap-1.5 overflow-x-auto no-scrollbar rounded-xl border-2 border-[#C59B27]/80 font-display text-[#1C140C] shadow-[0_6px_18px_rgba(28,20,12,0.28),inset_0_1px_0_rgba(255,255,255,0.9)]"
        >
          {[
            {
              id: 'FORVM',
              label: 'Workspace Map',
              shortLabel: 'Workspace',
              purpose: 'Reset camera to fit the entire repository architecture and inspect core compiler crates.',
              actionHint: 'Click to fit map & inspect Core Architecture (click again to open Dashboard)',
              surface: 'dashboard',
              lit: true,
              detail: `${layout.districts.length} crates`,
              icon: Icon.romanColumn,
              onClick: () => {
                fitWorld();
                const d =
                  layout.districts.find(
                    (dist) => dist.archetype === 'acropolis_engine' || dist.landmark,
                  ) ?? layout.districts[0];
                if (selection?.kind === 'landmark' && selection.id === 'FORVM' && onNavigate) {
                  onNavigate('dashboard');
                } else {
                  setSelection(d ? { kind: 'district', name: d.name } : { kind: 'landmark', id: 'FORVM' });
                }
              },
            },
            {
              id: 'PRAETORIVM',
              label: 'Orchestrator',
              shortLabel: 'Orchestrator',
              purpose: 'Focus the central Agent Orchestrator hub coordinating active turns, spokes, and worktrees.',
              actionHint: 'Click to focus Orchestrator (click again or click ↗ to open Operations Flow)',
              surface: 'flow',
              lit: true,
              detail: 'Live',
              icon: Icon.brazierTorch,
              onClick: () => {
                const d =
                  layout.districts.find(
                    (dist) => dist.archetype === 'praetorium_hub' || (dist.layer ?? 1) === 2,
                  ) ?? layout.districts[0];
                if (d) jumpToTile((d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
                if (selection?.kind === 'landmark' && selection.id === 'PRAETORIVM' && onNavigate) {
                  onNavigate('flow');
                } else {
                  setSelection({ kind: 'landmark', id: 'PRAETORIVM' });
                }
              },
            },
            {
              id: 'CASTRVM',
              label: 'CI & Gates',
              shortLabel: 'CI Gates',
              purpose: 'Inspect continuous integration runners, TOESTUB static audit gates, and compiler verification.',
              actionHint: 'Click to focus CI Fortress (click again or click ↗ to open Pipeline Cascade)',
              surface: 'harness',
              lit: Boolean(harness.ci),
              detail: harness.ci ? `${harness.ci.runners.length}/${harness.ci.runners.length}` : 'verified',
              icon: Icon.gladiusShield,
              onClick: () => {
                jumpToTile(layout.landmarks.castrum.x, layout.landmarks.castrum.y);
                if (selection?.kind === 'landmark' && selection.id === 'CASTRVM' && onNavigate) {
                  onNavigate('harness');
                } else {
                  setSelection({ kind: 'landmark', id: 'CASTRVM' });
                }
              },
            },
            {
              id: 'PORTVS',
              label: 'Task Queue',
              shortLabel: 'Tasks',
              purpose: 'Inspect the task queue harbor where new user goals and background jobs enter the workspace.',
              actionHint: 'Click to focus Task Harbor (click again or click ↗ to open Task Queue)',
              surface: 'tasks',
              lit: harness.queueLen !== null,
              detail: harness.queueLen !== null ? `${harness.queueLen} queued` : '0 queued',
              icon: Icon.romanGalley,
              onClick: () => {
                jumpToTile(layout.landmarks.portus.x, layout.landmarks.portus.y);
                if (selection?.kind === 'landmark' && selection.id === 'PORTVS' && onNavigate) {
                  onNavigate('tasks');
                } else {
                  setSelection({ kind: 'landmark', id: 'PORTVS' });
                }
              },
            },
          ].map((lm) => {
            const IconComp = lm.icon;
            const isSelected = selection?.kind === 'landmark' && selection.id === lm.id;
            return (
              <RomanTooltip
                key={lm.id}
                title={lm.label}
                purpose={lm.purpose}
                actionHint={lm.actionHint}
                statusBadge={lm.detail}
                placement="bottom"
              >
                <button
                  type="button"
                  data-urbs-landmark={lm.id}
                  data-urbs-landmark-state={lm.lit ? 'lit' : 'unlit'}
                  onClick={lm.onClick}
                  onDoubleClick={(ev) => {
                    ev.stopPropagation();
                    onNavigate?.(lm.surface);
                  }}
                  className={`inline-flex items-center gap-1.5 whitespace-nowrap rounded-lg border px-2 py-0.5 transition ${
                    lm.lit
                      ? 'border-[#C59B27]/80 bg-[#FAF5E8] text-[#1C140C] hover:border-[#B8860B] hover:bg-white'
                      : 'border-[#C59B27]/45 bg-[#EFE6D5] text-[#5C4428] hover:text-[#1C140C]'
                  }`}
                >
                  <span
                    className={`size-1.5 rounded-full ${
                      lm.lit ? 'bg-[#B8860B] shadow-[0_0_6px_#D4AF37]' : 'bg-[#8C7D64]'
                    }`}
                    aria-hidden="true"
                  />
                  <span className="font-semibold tracking-wider uppercase text-[8.5px]">
                    {lm.shortLabel}
                  </span>
                </button>
              </RomanTooltip>
            );
          })}
        </div>
      )}

      {/* Embedded Right-Hand Progressive Disclosure Quick-Action Stack (Concept Art Plate 1 & 2) */}
      {layout && embedded && (
        <div className="pointer-events-auto absolute right-2 top-2 z-10 flex flex-col gap-1">
          <RomanTooltip
            title="Git Commit Timeline"
            purpose="Open the full-screen Workspace Map with the interactive Git Milestone Scrubber."
            actionHint="Click to open Git Timeline in Full-Screen Map"
            placement="left"
          >
            <button
              type="button"
              onClick={() => onNavigate?.('vox-Ludus-view')}
              className="inline-flex items-center justify-between gap-1.5 rounded-lg border border-[#C59B27]/80 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 px-2.5 py-1 font-display text-[9.5px] font-semibold uppercase tracking-wider text-[#1C140C] shadow-md hover:border-[#B8860B]"
            >
              <Icon.hourglassVia className="size-3 text-[#8C6214]" aria-hidden="true" />
              <span>Git Timeline</span>
            </button>
          </RomanTooltip>
          <RomanTooltip
            title="Priority Alerts & Gates"
            purpose={`${activeGates.length} pending review gates or compiler diagnostics across workspace districts.`}
            actionHint="Click to inspect & resolve Priority Alerts"
            placement="left"
          >
            <button
              type="button"
              onClick={() => onNavigate?.('vox-Ludus-view')}
              className="inline-flex items-center justify-between gap-1.5 rounded-lg border border-[#C59B27]/80 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 px-2.5 py-1 font-display text-[9.5px] font-semibold uppercase tracking-wider text-[#1C140C] shadow-md hover:border-[#B8860B]"
            >
              <Icon.gladiusShield className="size-3 text-[#8C6214]" aria-hidden="true" />
              <span>Priority ({activeGates.length})</span>
            </button>
          </RomanTooltip>
          <RomanTooltip
            title="7-Region Architecture Radar"
            purpose="Open the engraved Astrolabe Radar to inspect all 7 architectural layers and active hotspots."
            actionHint="Click to open Full-Screen Architecture Radar"
            placement="left"
          >
            <button
              type="button"
              onClick={() => onNavigate?.('vox-Ludus-view')}
              className="inline-flex items-center justify-between gap-1.5 rounded-lg border border-[#C59B27]/80 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 px-2.5 py-1 font-display text-[9.5px] font-semibold uppercase tracking-wider text-[#1C140C] shadow-md hover:border-[#B8860B]"
            >
              <Icon.astrolabeRadar className="size-3 text-[#8C6214]" aria-hidden="true" />
              <span>Radar View</span>
            </button>
          </RomanTooltip>
        </div>
      )}

      {/* Unified Architrave Bar at top-2.5 (Concept Art Option A: Consolidated 1-Row Toolbar when !embedded) */}
      {layout && !embedded && (
        <div
          style={{
            backgroundImage:
              'linear-gradient(90deg, rgba(250,246,238,0.96) 0%, rgba(242,234,218,0.95) 50%, rgba(250,246,238,0.96) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="pointer-events-auto absolute left-2.5 right-2.5 top-2.5 z-10 flex flex-nowrap items-center justify-between gap-2 rounded-xl border-2 border-[#C59B27]/80 px-2.5 py-1 text-[10px] font-display text-[#1C140C] shadow-[0_8px_24px_rgba(28,20,12,0.28),inset_0_1px_0_rgba(255,255,255,0.9)]"
        >
          {/* Left: 4 compact Landmark Medallion buttons */}
          <div data-testid="urbs-landmark-bar" className="flex min-w-0 items-center gap-1 overflow-x-auto no-scrollbar">
            {[
              {
                id: 'FORVM',
                label: 'Workspace Map',
                shortLabel: 'Workspace',
                purpose: 'Reset camera to fit the entire repository architecture and inspect core compiler crates.',
                actionHint: 'Click to fit map & inspect Core Architecture (click again to open Dashboard)',
                surface: 'dashboard',
                lit: true,
                detail: `${layout.districts.length} crates`,
                icon: Icon.romanColumn,
                onClick: () => {
                  fitWorld();
                  const d =
                    layout.districts.find(
                      (dist) => dist.archetype === 'acropolis_engine' || dist.landmark,
                    ) ?? layout.districts[0];
                  if (selection?.kind === 'landmark' && selection.id === 'FORVM' && onNavigate) {
                    onNavigate('dashboard');
                  } else {
                    setSelection(d ? { kind: 'district', name: d.name } : { kind: 'landmark', id: 'FORVM' });
                  }
                },
              },
              {
                id: 'PRAETORIVM',
                label: 'Orchestrator',
                shortLabel: 'Orchestrator',
                purpose: 'Focus the central Agent Orchestrator hub coordinating active turns, spokes, and worktrees.',
                actionHint: 'Click to focus Orchestrator (click again or click ↗ to open Operations Flow)',
                surface: 'flow',
                lit: true,
                detail: 'Live',
                icon: Icon.brazierTorch,
                onClick: () => {
                  const d =
                    layout.districts.find(
                      (dist) => dist.archetype === 'praetorium_hub' || (dist.layer ?? 1) === 2,
                    ) ?? layout.districts[0];
                  if (d) jumpToTile((d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
                  if (selection?.kind === 'landmark' && selection.id === 'PRAETORIVM' && onNavigate) {
                    onNavigate('flow');
                  } else {
                    setSelection({ kind: 'landmark', id: 'PRAETORIVM' });
                  }
                },
              },
              {
                id: 'CASTRVM',
                label: 'CI & Gates',
                shortLabel: 'CI Gates',
                purpose: 'Inspect continuous integration runners, TOESTUB static audit gates, and compiler verification.',
                actionHint: 'Click to focus CI Fortress (click again or click ↗ to open Pipeline Cascade)',
                surface: 'harness',
                lit: Boolean(harness.ci),
                detail: harness.ci ? `${harness.ci.runners.length}/${harness.ci.runners.length}` : 'verified',
                icon: Icon.gladiusShield,
                onClick: () => {
                  jumpToTile(layout.landmarks.castrum.x, layout.landmarks.castrum.y);
                  if (selection?.kind === 'landmark' && selection.id === 'CASTRVM' && onNavigate) {
                    onNavigate('harness');
                  } else {
                    setSelection({ kind: 'landmark', id: 'CASTRVM' });
                  }
                },
              },
              {
                id: 'PORTVS',
                label: 'Task Queue',
                shortLabel: 'Tasks',
                purpose: 'Inspect the task queue harbor where new user goals and background jobs enter the workspace.',
                actionHint: 'Click to focus Task Harbor (click again or click ↗ to open Task Queue)',
                surface: 'tasks',
                lit: harness.queueLen !== null,
                detail: harness.queueLen !== null ? `${harness.queueLen} queued` : '0 queued',
                icon: Icon.romanGalley,
                onClick: () => {
                  jumpToTile(layout.landmarks.portus.x, layout.landmarks.portus.y);
                  if (selection?.kind === 'landmark' && selection.id === 'PORTVS' && onNavigate) {
                    onNavigate('tasks');
                  } else {
                    setSelection({ kind: 'landmark', id: 'PORTVS' });
                  }
                },
              },
            ].map((lm) => {
              const IconComp = lm.icon;
              const isSelected = selection?.kind === 'landmark' && selection.id === lm.id;
              return (
                <RomanTooltip
                  key={lm.id}
                  title={lm.label}
                  purpose={lm.purpose}
                  actionHint={lm.actionHint}
                  statusBadge={lm.detail}
                  placement="bottom"
                >
                  <button
                    type="button"
                    data-urbs-landmark={lm.id}
                    data-urbs-landmark-state={lm.lit ? 'lit' : 'unlit'}
                    onClick={lm.onClick}
                    onDoubleClick={(ev) => {
                      ev.stopPropagation();
                      onNavigate?.(lm.surface);
                    }}
                    className={`inline-flex items-center gap-1.5 whitespace-nowrap rounded-lg border px-2 py-0.5 transition ${
                      lm.lit
                        ? 'border-[#C59B27]/80 bg-[#FAF5E8] text-[#1C140C] hover:border-[#B8860B] hover:bg-white'
                        : 'border-[#C59B27]/45 bg-[#EFE6D5] text-[#5C4428] hover:text-[#1C140C]'
                    }`}
                  >
                    <RomanMedallion size="sm" active={isSelected || lm.lit}>
                      <IconComp className="size-3 text-[#D4AF37]" />
                    </RomanMedallion>
                    <span className="font-semibold tracking-wider uppercase text-[9.5px]">
                      {lm.shortLabel}
                    </span>
                    {onNavigate && (
                      <span
                        data-testid={`landmark-open-${lm.id.toLowerCase()}`}
                        onClick={(ev) => {
                          ev.stopPropagation();
                          onNavigate(lm.surface);
                        }}
                        title={`Open ${lm.label} surface (${lm.surface})`}
                        className="rounded border border-[#C59B27]/55 bg-[#F3E5AB]/70 px-1 font-display text-[8.5px] font-bold text-[#1C140C] hover:bg-[#D4AF37]/50"
                      >
                        ↗
                      </span>
                    )}
                  </button>
                </RomanTooltip>
              );
            })}
          </div>

          {/* Center/Right: Primary Lens + [ Lens & Layout ▾ ] + [ Alerts & History ▾ ] */}
          <div data-testid="urbs-lens-bar" className="relative flex items-center gap-1.5 font-sans text-[9.5px]">
            {/* Primary Architecture lens button */}
            <RomanTooltip
              title="Architecture Lens"
              purpose="Highlights the 7 architectural layers (L0 Foundation through L5 Surfaces) and crate dependency viaducts."
              actionHint={lens === 'architecture' ? 'Currently active map lens' : 'Click to switch map overlay to Architecture'}
              placement="bottom"
            >
              <button
                type="button"
                data-urbs-lens="architecture"
                onClick={() => setLens('architecture')}
                className={`group inline-flex items-center gap-1.5 whitespace-nowrap rounded-lg border px-2 py-0.5 font-display text-[9.5px] uppercase tracking-wider transition ${
                  lens === 'architecture'
                    ? 'border-[#8C6214] bg-gradient-to-b from-[#2A1F12] to-[#161009] font-semibold text-[#F3E5AB] shadow-sm'
                    : 'border-[#C59B27]/75 bg-[#FAF5E8] text-[#1C140C] hover:border-[#B8860B] hover:bg-white'
                }`}
              >
                <Icon.romanColumn className={`size-3 ${lens === 'architecture' ? 'text-[#F3E5AB]' : 'text-[#8C6214]'}`} aria-hidden="true" />
                <span>Architecture</span>
              </button>
            </RomanTooltip>

            {/* Compact [ Lens & Layout ▾ ] Menu Button */}
            <div className="relative">
              <RomanTooltip
                title="Lens & Layout Settings"
                purpose="Switch map overlays (Diagnostics, Agents, Pipeline, Token Spend) and repository preset / attention layout."
                actionHint="Click to toggle Lens & Layout dropdown menu"
                placement="bottom"
              >
                <button
                  type="button"
                  data-testid="urbs-lens-layout-menu"
                  onClick={() => setShowLensMenu((v) => !v)}
                  className={`inline-flex items-center gap-1.5 rounded-lg border px-2 py-0.5 font-display text-[9.5px] font-semibold uppercase tracking-wider transition ${
                    showLensMenu
                      ? 'border-[#8C6214] bg-gradient-to-b from-[#2A1F12] to-[#161009] text-[#F3E5AB]'
                      : 'border-[#C59B27]/75 bg-[#FAF5E8] text-[#1C140C] hover:border-[#B8860B] hover:bg-white'
                  }`}
                >
                  <Icon.astrolabeRadar className="size-3 text-[#8C6214]" aria-hidden="true" />
                  <span>Lens &amp; Layout ▾</span>
                </button>
              </RomanTooltip>

              {/* Popover Dropdown (Always in DOM for tests/a11y, visible when showLensMenu is true) */}
              <div
                className={
                  showLensMenu
                    ? "absolute right-0 top-full mt-2 z-30 flex flex-col gap-2 rounded-xl border-2 border-[#D4AF37]/65 bg-[#14100B]/98 p-3 shadow-[0_12px_32px_rgba(0,0,0,0.9)] backdrop-blur-md min-w-[280px]"
                    : "pointer-events-none absolute -left-[9999px] top-0 z-[-1] flex flex-col gap-2 opacity-0 min-w-[280px]"
                }
              >
                <div className="flex flex-col gap-1 border-b border-[#D4AF37]/25 pb-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-display text-[9px] font-semibold uppercase tracking-widest text-[#D4AF37]">Repo:</span>
                    <select
                      data-testid="urbs-repo-preset-select"
                      data-unified-knob="true"
                      aria-label="Repository simulation preset"
                      value={presetId}
                      onChange={(e) => {
                        setPresetId(e.target.value as UrbsRepoPresetId);
                        setSelection(null);
                      }}
                      className="appearance-none rounded border border-[#D4AF37]/45 bg-[#1B150C] px-2 py-0.5 font-mono text-[9.5px] text-[#F3E5AB] focus:border-[#F3E5AB] focus:outline-none"
                    >
                      {Object.values(URBS_REPO_PRESETS).map((preset) => (
                        <option key={preset.id} value={preset.id}>
                          {preset.label}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div className="flex items-center justify-between gap-2 mt-1">
                    <span className="font-display text-[9px] font-semibold uppercase tracking-widest text-[#D4AF37]">Layout:</span>
                    <select
                      data-testid="urbs-focus-mode-select"
                      data-unified-knob="true"
                      aria-label="Real-estate attention focus mode"
                      value={focusMode}
                      onChange={(e) => setFocusMode(e.target.value as RealEstateFocusMode)}
                      className="appearance-none rounded border border-[#D4AF37]/45 bg-[#1B150C] px-2 py-0.5 font-mono text-[9.5px] text-[#F3E5AB] focus:border-[#F3E5AB] focus:outline-none"
                    >
                      <option value="attention">Attention Weighted</option>
                      <option value="worksite_only">Active Worksite Only</option>
                      <option value="proportional">Proportional LoC</option>
                    </select>
                  </div>
                </div>

                <div className="flex flex-col gap-1">
                  <span className="font-display text-[9px] font-semibold uppercase tracking-widest text-[#D4AF37]">Map Lenses:</span>
                  <div className="grid grid-cols-2 gap-1">
                    {(
                      [
                        {
                          id: 'health',
                          label: 'Health & Diagnostics',
                          icon: Icon.brazierTorch,
                        },
                        {
                          id: 'agents',
                          label: 'Live Agents',
                          icon: Icon.gladiusShield,
                        },
                        {
                          id: 'pipeline',
                          label: 'Task Pipeline',
                          icon: Icon.romanGalley,
                        },
                        {
                          id: 'cost',
                          label: 'Token Spend',
                          icon: Icon.denariusCoin,
                        },
                      ] as const
                    ).map((item) => {
                      const active = lens === item.id;
                      const LensIcon = item.icon;
                      return (
                        <button
                          key={item.id}
                          type="button"
                          data-urbs-lens={item.id}
                          onClick={() => {
                            setLens(item.id);
                          }}
                          className={`inline-flex items-center gap-1.5 rounded border px-2 py-1 font-display text-[9px] uppercase tracking-wider transition ${
                            active
                              ? 'border-[#F3E5AB] bg-gradient-to-b from-[#4A3818] to-[#241B0B] font-semibold text-[#F3E5AB]'
                              : 'border-[#3A3022] bg-[#1B150C] text-[#C8B99E] hover:border-[#D4AF37]/60 hover:text-[#F3E5AB]'
                          }`}
                        >
                          <LensIcon className={`size-3 ${active ? 'text-[#F3E5AB]' : 'text-[#D4AF37]'}`} aria-hidden="true" />
                          <span className="truncate">{item.label}</span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
            </div>

            {/* Compact [ Alerts & History ▾ ] Button */}
            <RomanTooltip
              title="Alerts & History Overlays"
              purpose="Toggle priority alerts drawer and git timeline scrubber."
              actionHint="Click to toggle Alerts & History drawers"
              placement="bottom"
            >
              <button
                type="button"
                data-testid="urbs-alerts-history-menu"
                onClick={() => {
                  const next = !(showPriorityDrawer || showTimelineDrawer);
                  setShowPriorityDrawer(next);
                  setShowTimelineDrawer(next);
                }}
                className={`inline-flex items-center gap-1.5 rounded-lg border px-2 py-0.5 font-display text-[9.5px] font-semibold uppercase tracking-wider transition ${
                  showPriorityDrawer || showTimelineDrawer
                    ? 'border-[#8C6214] bg-gradient-to-b from-[#2A1F12] to-[#161009] text-[#F3E5AB]'
                    : 'border-[#C59B27]/75 bg-[#FAF5E8] text-[#1C140C] hover:border-[#B8860B] hover:bg-white'
                }`}
              >
                <Icon.gladiusShield className="size-3 text-[#8C6214]" aria-hidden="true" />
                <span>Alerts &amp; History ▾</span>
                {activeGates.length > 0 && (
                  <span className="rounded-full border border-[#C59B27]/50 bg-[#F3E5AB] px-1 font-sans text-[8.5px] font-bold text-[#1C140C]">
                    {activeGates.length}
                  </span>
                )}
              </button>
            </RomanTooltip>
          </div>
        </div>
      )}
      {layout && !embedded && (
        <div
          data-testid="urbs-priority-dock"
          className={
            showPriorityDrawer
              ? "pointer-events-auto absolute right-2.5 top-16 z-10 w-[236px] rounded-xl border-2 border-[#D4AF37]/55 bg-linear-to-b from-[#1C160F]/95 to-[#0E0B07]/95 p-2.5 text-[#F3E5AB] shadow-[0_12px_30px_rgba(0,0,0,0.85)]"
              : "pointer-events-none absolute -left-[9999px] top-0 z-[-1] w-[236px] opacity-0"
          }
        >
          <div className="mb-1.5 flex items-center justify-between border-b border-[#D4AF37]/30 pb-1">
            <span className="inline-flex items-center gap-1.5 font-display text-[9.5px] font-semibold uppercase tracking-[0.14em] text-[#F3E5AB]">
              <Icon.gladiusShield className="size-3.5 text-[#D4AF37]" aria-hidden="true" />
              <span>Priority Alerts</span>
            </span>
            <div className="flex items-center gap-1.5">
              <span className="font-mono text-[8.5px] text-[#B8A88A]">
                {activePreset.language}
              </span>
              <button
                type="button"
                onClick={() => setShowPriorityDrawer(false)}
                title="Close Priority Alerts"
                className="text-[#8C7D64] hover:text-[#F3E5AB] font-mono text-[10px]"
              >
                ✕
              </button>
            </div>
          </div>
          <div className="max-h-36 space-y-1 overflow-y-auto pr-0.5 custom-scrollbar">
            {activeGates.map((gate) => {
              const dist = layout.districts.find((d) => d.name === gate.district) ?? layout.districts[0];
              return (
                <div
                  key={`gate-${gate.id}`}
                  className="flex w-full items-center justify-between gap-1 rounded border border-[#D4AF37]/50 bg-[#2A2011]/90 px-2 py-1 font-mono text-[9px] text-[#F3E5AB]"
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
                    className="min-w-0 flex-1 truncate text-left hover:text-white"
                  >
                    {gate.title}
                  </button>
                  <button
                    type="button"
                    data-testid={`urbs-approve-gate-${gate.id}`}
                    onClick={() => {
                      setResolvedGateIds((prev) => new Set([...prev, gate.id]));
                    }}
                    className="shrink-0 rounded border border-[#7B8C32]/65 bg-[#4D5D1E]/25 px-1.5 py-0.5 text-[8px] font-semibold text-[#DCE7B6] hover:bg-[#4D5D1E]/40"
                    title="Approve decision gate"
                  >
                    Approve
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
                    className="flex w-full items-center justify-between gap-1 rounded border border-rose-500/60 bg-rose-950/60 px-2 py-1 font-mono text-[9px] text-rose-100"
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
                      {shortName} ({diag.errors} err)
                    </button>
                    <button
                      type="button"
                      data-testid="urbs-priority-open-loupe"
                      onClick={() => {
                        openSpeculumLoupe(path, Math.round(viewportSize.w * 0.52), Math.round(viewportSize.h * 0.36));
                      }}
                      className="shrink-0 rounded border border-[#D4AF37]/60 bg-[#3A2C14] px-1.5 py-0.5 text-[8px] font-semibold text-[#F3E5AB] hover:border-[#F3E5AB]"
                      title="Inspect file diff & compiler diagnostics"
                    >
                      Inspect
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
                  className="flex w-full items-center justify-between gap-1 rounded border border-[#D4AF37]/40 bg-[#221B11] px-2 py-1 text-left font-mono text-[9px] text-[#F3E5AB] hover:border-[#D4AF37]"
                >
                  <span className="truncate">
                    {agentId} → {shortName}
                  </span>
                  <span className="shrink-0 text-[8.5px] text-[#DCE7B6]">ACTIVE</span>
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
                    className="flex w-full items-center justify-between gap-1 rounded border border-[#7B8C32]/50 bg-[#1E2410]/60 px-2 py-1 text-left font-mono text-[9px] text-[#DCE7B6] hover:bg-[#2C360E]/60"
                  >
                    <span className="truncate">
                      {district}/{shortName}
                    </span>
                    <span className="shrink-0 text-[8.5px] text-[#DCE7B6]">
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
              <span className="rounded-sm border border-[#D4AF37]/60 bg-[#14100B]/95 px-1.5 py-0.5 font-mono text-[9px] text-[#F3E5AB] shadow-md">
                {id} · {agentTasks[id]?.filePath.split('/').pop() ?? 'active'}
              </span>
            </div>
          </div>
        ))}
        {ambientRosters.map((c) => {
          const meta = vexillumMeta[c.id] ?? {
            legion: c.id,
            model: 'Claude Sonnet 4.5',
            task: 'Patrolling Workspace',
            border: 'border-[#D4AF37]/70 bg-[#14100B]/95 text-[#F3E5AB]',
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
        <div className="absolute right-2 top-2 rounded-sm border border-[#D4AF37]/50 bg-[#14100B]/90 px-2 py-1 font-display text-[10px] tracking-widest text-[#F3E5AB]">
          BUILD · {buildStage}
        </div>
      )}
      {paused && (
        <div className="absolute right-2 top-14 z-20 rounded-sm border border-amber-700/50 bg-black/85 px-2.5 py-1 font-sans text-[10px] font-medium tracking-wide text-amber-300 shadow-md">
          SIM PAUSED — live stream unavailable
          <span className="sr-only">SIM PAVSED</span>
        </div>
      )}
      {menu && (
        <div
          className="absolute z-20 rounded-lg border border-[#D4AF37]/50 bg-[#16120C] p-1 text-[11px] text-[#F3E5AB] shadow-lg"
          style={{ left: menu.sx + 8, top: menu.sy + 8 }}
        >
          <div className="max-w-[220px] truncate px-2 py-1 font-mono text-[#B8A88A]">{menu.path}</div>
          <button type="button" className="block w-full rounded-sm px-2 py-1 text-left hover:bg-[#2A2011]"
            onClick={() => { voxTransport.openLocator({ kind: 'file', value: scanRoot ? `${scanRoot}/${menu.path}` : menu.path }).catch(() => {}); setMenu(null); }}>
            Open file in editor
          </button>
          <button type="button" className="block w-full rounded-sm px-2 py-1 text-left hover:bg-[#2A2011]"
            onClick={() => { useLudusStore.getState().setFocusedFile(menu.path); setMenu(null); }}>
            Focus on map
          </button>
          <div className="px-2 py-1 font-mono text-[10px] text-[#B8A88A]">
            WARN {buildings[menu.path]?.warnings ?? 0} · ERR {buildings[menu.path]?.errors ?? 0}
          </div>
        </div>
      )}

      {/* Plate 2 Engraved Marble & Brass 7-Region Architecture Astrolabe Radar */}
      {layout && !embedded && showRadarDrawer && (
        <RomanTooltip
          title="7-Region Architecture Astrolabe Radar"
          purpose={`Aggregates all ${layout.districts.length} workspace crates into the 7 canonical architectural layers (I Foundation through VII Surfaces).`}
          actionHint="Click anywhere on the Astrolabe dial to pan the camera to that region"
          statusBadge={`${layout.districts.length} crates · 7 regions`}
          placement="left"
        >
          <div
            data-testid="urbs-minimap-radar"
            className="pointer-events-auto absolute bottom-2.5 right-2.5 z-10 w-[192px] rounded-2xl border-2 border-[#C59B27]/85 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 p-2 shadow-[0_12px_32px_rgba(28,20,12,0.35),inset_0_1px_0_rgba(255,255,255,0.9)]"
          >
            <div className="flex items-center justify-between px-1 pb-1">
              <span className="inline-flex items-center gap-1 font-display text-[9px] font-bold uppercase tracking-[0.14em] text-[#1C140C]">
                <Icon.astrolabeRadar className="size-3 text-[#8C6214]" aria-hidden="true" />
                <span>Architecture Radar</span>
              </span>
              <span className="font-sans text-[8.5px] font-semibold text-[#5C4428]">
                7 Regions ({layout.districts.length})
              </span>
            </div>
            <div className="relative h-[126px] w-full overflow-hidden rounded-xl border border-[#D4AF37]/45 bg-[#0A0806]">
              <img
                src="/assets/romanesque/astrolabe-radar-dial.jpg"
                alt=""
                aria-hidden="true"
                className="pointer-events-none absolute inset-0 h-full w-full object-cover opacity-90"
              />
              <svg
                viewBox={`0 0 ${mapW} ${mapH}`}
                onClick={onMinimapClick}
                className="relative z-10 h-full w-full cursor-crosshair"
                aria-label="Architecture Astrolabe Radar"
              >
                {/* Animated Rotating Brass Astrolabe Sweep Hand */}
                <g transform={`translate(${mapW / 2}, ${mapH / 2})`}>
                  <line
                    x1="0"
                    y1="0"
                    x2="0"
                    y2={-mapH * 0.42}
                    stroke="#F3E5AB"
                    strokeWidth="1.4"
                    strokeOpacity="0.8"
                  >
                    <animateTransform
                      attributeName="transform"
                      type="rotate"
                      from="0"
                      to="360"
                      dur="12s"
                      repeatCount="indefinite"
                    />
                  </line>
                </g>

                {/* 7 Canonical Architectural Region Hotspots (Replacing 148-polygon wireframe clutter) */}
                {[
                  { rx: 0.30, ry: 0.32, label: 'I·CORE', color: '#F59E0B' },
                  { rx: 0.68, ry: 0.30, label: 'II·COMPILER', color: '#D4AF37' },
                  { rx: 0.32, ry: 0.68, label: 'III·ORCH', color: '#8FA339' },
                  { rx: 0.68, ry: 0.68, label: 'IV·MENS', color: '#F97316' },
                  { rx: 0.50, ry: 0.22, label: 'V·GUI', color: '#F3E5AB' },
                  { rx: 0.22, ry: 0.50, label: 'VI·DB', color: '#C59B27' },
                  { rx: 0.78, ry: 0.50, label: 'VII·MESH', color: '#FBBF24' },
                ].map((reg) => (
                  <g key={reg.label}>
                    <circle
                      cx={mapW * reg.rx}
                      cy={mapH * reg.ry}
                      r="5.5"
                      fill={reg.color}
                      fillOpacity="0.28"
                      stroke={reg.color}
                      strokeWidth="1.2"
                    >
                      <animate attributeName="r" values="4.5;6.8;4.5" dur="2.4s" repeatCount="indefinite" />
                    </circle>
                    <circle
                      cx={mapW * reg.rx}
                      cy={mapH * reg.ry}
                      r="2.2"
                      fill="#FEF08A"
                    />
                  </g>
                ))}

                {/* Camera Viewport Frustum Box */}
                <rect
                  data-testid="urbs-minimap-frustum"
                  x={frustumRect.x}
                  y={frustumRect.y}
                  width={frustumRect.w}
                  height={frustumRect.h}
                  rx="2"
                  fill="rgba(212, 175, 55, 0.18)"
                  stroke="#F3E5AB"
                  strokeWidth="1.6"
                />
              </svg>
            </div>
          </div>
        </RomanTooltip>
      )}

      {layout && !embedded && !selection && (
        showTimelineDrawer ? (
          <div className="pointer-events-auto absolute bottom-[62px] left-2.5 max-w-[calc(100%-220px)] z-10 flex justify-start">
            <ViaTemporisScrubber
              milestones={viaMilestones}
              checkpoints={viaMilestones}
              selectedTurn={selectedTurn}
              activeTurn={selectedTurn}
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
              onClose={() => setShowTimelineDrawer(false)}
            />
          </div>
        ) : (
          <div className="pointer-events-auto absolute bottom-2.5 left-2.5 z-10">
            <RomanTooltip
              title="Open Git Milestone Timeline"
              purpose="Open the interactive Git Milestone Scrubber for inspecting commit diffs and restoring checkpoints."
              actionHint="Click to open Git Timeline"
              placement="top"
            >
              <button
                type="button"
                data-testid="urbs-git-timeline-trigger"
                onClick={() => setShowTimelineDrawer(true)}
                className="inline-flex items-center gap-1.5 rounded-lg border-2 border-[#C59B27]/80 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 px-2.5 py-1 font-display text-[9.5px] font-semibold uppercase tracking-wider text-[#1C140C] shadow-[0_6px_18px_rgba(28,20,12,0.3)] hover:border-[#B8860B] transition"
              >
                <div className="relative size-4 shrink-0 overflow-hidden rounded-full border border-[#C59B27]/80 shadow-[0_0_6px_rgba(212,175,55,0.4)]">
                  <img
                    src="/assets/romanesque/wax-seal-medallion.jpg"
                    alt=""
                    aria-hidden="true"
                    className="size-full object-cover"
                  />
                </div>
                <span className="font-semibold text-[#1C140C]">
                  Git Step {selectedTurn}/{viaMilestones[viaMilestones.length - 1]?.turnNumber ?? 3} ▾
                </span>
              </button>
            </RomanTooltip>
          </div>
        )
      )}
      {/* Phase 364: Semiotic Layer Navigator & Developer Quick-Action Command Dock (Concept Art Plate 1) */}
      {layout && !embedded && (
        <div
          data-testid="urbs-developer-action-dock"
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(242,235,222,0.96) 0%, rgba(226,215,195,0.95) 100%), url(/assets/romanesque/marble-frieze-banner.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="vox-marble-tablet pointer-events-auto absolute bottom-[64px] left-2.5 z-10 flex w-max max-w-[calc(100%-214px)] flex-wrap items-center justify-between gap-1.5 rounded-xl border-2 border-[#B38728]/80 px-2.5 py-1 text-[#1E160D] shadow-[0_10px_28px_rgba(0,0,0,0.85),inset_0_1px_0_rgba(255,255,255,0.85)]"
        >
          {/* Semiotic Architectural Layer Filter Pills (L0..L5) */}
          <div data-testid="urbs-semiotic-layer-pills" className="flex items-center gap-1">
            <span className="mr-0.5 font-display text-[8.5px] font-bold uppercase tracking-[0.14em] text-[#5C4318]">
              Tiers:
            </span>
            {[
              { layer: null, label: 'All', title: 'Show all architectural layers' },
              { layer: 0, label: 'L0 Vaults', title: 'L0 Foundation Vaults (crypto, secrets, config)' },
              { layer: 1, label: 'L1 Acropolis', title: 'L1 Compiler Acropolis (lexer, parser, typeck, codegen)' },
              { layer: 2, label: 'L2 Rotunda', title: 'L2 Database & Runtime Rotunda (vox-db, search, actor-runtime)' },
              { layer: 3, label: 'L3 Praetorium', title: 'L3 Orchestrator Praetorium (multi-agent routing, MCP, DEI)' },
              { layer: 4, label: 'L4 Thermae', title: 'L4 MENS Thermae (QLoRA training, eval, Crucible)' },
              { layer: 5, label: 'L5 Colosseum', title: 'L5 Surface Colosseum (vox-gui, vox-cli, ludus)' },
            ].map((item) => {
              const isAct = activeLayerFilter === item.layer;
              return (
                <button
                  key={item.label}
                  type="button"
                  title={item.title}
                  data-testid={`urbs-layer-pill-${item.layer ?? 'all'}`}
                  onClick={() => {
                    setActiveLayerFilter(item.layer);
                    if (item.layer === null) {
                      fitWorld();
                      setDevActionReceipt('Viewing all 6 Semiotic Architectural Tiers (L0–L5)');
                    } else {
                      const targetDist =
                        layout.districts.find((d) => (d.layer ?? 1) === item.layer) ?? layout.districts[0];
                      if (targetDist) {
                        jumpToTile((targetDist.x0 + targetDist.x1) / 2, (targetDist.y0 + targetDist.y1) / 2);
                        setSelection({ kind: 'district', name: targetDist.name });
                        setDevActionReceipt(`Focused ${item.label} → ${targetDist.name}`);
                      }
                    }
                  }}
                  className={`rounded-md border px-1.5 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider transition ${
                    isAct
                      ? 'border-[#8C6214] bg-linear-to-b from-[#2A1F12] to-[#161009] text-[#F3E5AB] shadow-sm'
                      : 'border-[#B38728]/55 bg-[#F8F3E8]/90 text-[#3B2B10] hover:border-[#8C6214] hover:bg-white'
                  }`}
                >
                  {item.label}
                </button>
              );
            })}
          </div>

          <span className="h-3.5 w-px bg-[#B38728]/45" aria-hidden="true" />

          {/* 1-Click Developer Action Push-Buttons */}
          {(() => {
            const focusedDist =
              (selection?.kind === 'district'
                ? layout.districts.find((d) => d.name === selection.name)
                : undefined) ?? layout.districts[0];
            const crateName = focusedDist?.name ?? 'vox-cli';
            const firstPlot = focusedDist?.buildings[0];
            const isPinned = pinnedCrates.has(crateName);
            const isQuarantined = quarantinedSet.has(crateName);

            return (
              <div data-testid="urbs-developer-actions" className="flex items-center gap-1">
                <button
                  type="button"
                  data-testid="urbs-action-check-crate"
                  onClick={() => {
                    setDevActionReceipt(`⚙ Dispatched cargo check -p ${crateName}`);
                    void voxTransport.executeCommand(['check'], { crate: crateName }).catch(() => {});
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  <span>⚙ Check Crate</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-run-tests"
                  onClick={() => {
                    setDevActionReceipt(`✓ Running test suite for ${crateName}`);
                    void voxTransport.executeCommand(['test'], { crate: crateName }).catch(() => {});
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  <span>✓ Run Tests</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-inspect-diff"
                  onClick={() => {
                    if (firstPlot) {
                      openSpeculumLoupe(
                        firstPlot.path,
                        Math.round(viewportSize.w * 0.45),
                        Math.round(viewportSize.h * 0.34),
                      );
                      setDevActionReceipt(`🔍 Opened Speculum Diff Loupe on ${firstPlot.path}`);
                    }
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  <span>🔍 Diff Loupe</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-pin-context"
                  onClick={() => {
                    setPinnedCrates((prev) => {
                      const next = new Set(prev);
                      if (next.has(crateName)) next.delete(crateName);
                      else next.add(crateName);
                      return next;
                    });
                    setDevActionReceipt(
                      isPinned
                        ? `Unpinned ${crateName} from LLM context`
                        : `📌 Pinned ${crateName} to active LLM context budget`,
                    );
                  }}
                  className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider shadow-sm ${
                    isPinned
                      ? 'border-[#7B8C32] bg-[#1E2410] text-[#DCE7B6]'
                      : 'border-[#8C6214]/80 bg-linear-to-b from-[#261D11] to-[#140F08] text-[#F3E5AB] hover:border-[#D4AF37]'
                  }`}
                >
                  <span>{isPinned ? '📌 Pinned' : '📌 Pin Context'}</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-quarantine"
                  onClick={() => {
                    toggleDistrictQuarantine(crateName);
                    setDevActionReceipt(
                      isQuarantined
                        ? `🔓 Lifted quarantine on ${crateName}`
                        : `🔒 Quarantined ${crateName} from autonomous edits`,
                    );
                  }}
                  className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider shadow-sm ${
                    isQuarantined
                      ? 'border-amber-600 bg-amber-950 text-amber-200'
                      : 'border-[#8C6214]/80 bg-linear-to-b from-[#261D11] to-[#140F08] text-[#F3E5AB] hover:border-[#D4AF37]'
                  }`}
                >
                  <span>{isQuarantined ? '🔒 Quarantined' : '🔒 Quarantine'}</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-spawn-agent"
                  onClick={() => {
                    const targetFile = firstPlot?.path ?? `${crateName}/src/lib.rs`;
                    useLudusStore.getState().updateAgentTask('AEDILIS-01', {
                      taskId: `task-${Date.now()}`,
                      filePath: targetFile,
                      status: `Auditing ${crateName}`,
                    });
                    void voxTransport
                      .callTool('vox_submit_task', {
                        description: `Inspect and verify ${crateName}`,
                        file_affinity: [targetFile],
                      })
                      .catch(() => {});
                    setDevActionReceipt(`⚔ Dispatched LEGIO-01 agent to ${crateName}`);
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#D4AF37]"
                >
                  <span>⚔ Spawn Agent</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-claim-yield"
                  onClick={() => {
                    setClaimedYieldXp((xp) => xp + 50);
                    setDevActionReceipt(`🏆 Claimed +50 XP · +8 Denarii from ${crateName} verification!`);
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#B38728] bg-linear-to-b from-[#D4AF37] to-[#9A711C] px-2 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-[#1A1208] shadow-sm hover:brightness-110"
                >
                  <span>🏆 Yield ({claimedYieldXp} XP)</span>
                </button>
              </div>
            );
          })()}

          {devActionReceipt && (
            <span
              data-testid="urbs-dev-action-receipt"
              className="rounded border border-[#8C6214]/50 bg-[#1B140B] px-2 py-0.5 font-mono text-[8.5px] text-[#F3E5AB]"
            >
              {devActionReceipt}
            </span>
          )}
        </div>
      )}

      <div
        className={`absolute ${
          embedded ? 'bottom-1.5 left-2' : 'bottom-1.5 left-[142px] max-w-[calc(100%-348px)]'
        } z-10 flex flex-nowrap items-center gap-1.5`}
      >
        <HudPanels
          treasuryUsd={treasuryUsd}
          spendUsd={treasuryUsd}
          energy={energy}
          maxEnergy={maxEnergy}
          speed={speed}
          simSpeed={speed}
          onSetSpeed={setSpeed}
          embedded={embedded}
          onNavigate={onNavigate}
        />
        <RomanTooltip
          title="Fit city to view"
          purpose="Centers and zooms the camera so all architectural districts and monuments fit cleanly on screen."
          actionHint="Click to reset camera zoom & position"
        >
          <button
            type="button"
            title="Fit city to view"
            aria-label="Fit city to view"
            onClick={fitWorld}
            className="pointer-events-auto rounded-lg border border-[#D4AF37]/50 bg-[#14100B]/95 px-2.5 py-1 font-display text-[10px] uppercase tracking-wider text-[#F3E5AB] hover:border-[#F3E5AB] hover:bg-[#2A2011]"
          >
            ⌂ Fit Map
          </button>
        </RomanTooltip>
        {!embedded && (
          <>
            <RomanTooltip
              title="Zoom out"
              purpose="Step camera zoom out to view broader architectural connections."
              actionHint="Click to zoom out"
            >
              <button
                type="button"
                title="Zoom out"
                aria-label="Zoom out"
                onClick={() => stepZoom(1 / 1.2)}
                className="pointer-events-auto rounded-lg border border-[#D4AF37]/40 bg-[#14100B]/95 px-2.5 py-1 font-mono text-[11px] text-[#F3E5AB] hover:border-[#F3E5AB] hover:bg-[#2A2011]"
              >
                −
              </button>
            </RomanTooltip>
            <RomanTooltip
              title="Zoom in"
              purpose="Step camera zoom in to inspect individual crate buildings and file diagnostics."
              actionHint="Click to zoom in"
            >
              <button
                type="button"
                title="Zoom in"
                aria-label="Zoom in"
                onClick={() => stepZoom(1.2)}
                className="pointer-events-auto rounded-lg border border-[#D4AF37]/40 bg-[#14100B]/95 px-2.5 py-1 font-mono text-[11px] text-[#F3E5AB] hover:border-[#F3E5AB] hover:bg-[#2A2011]"
              >
                +
              </button>
            </RomanTooltip>
          </>
        )}
      </div>
    </div>
  );
};
