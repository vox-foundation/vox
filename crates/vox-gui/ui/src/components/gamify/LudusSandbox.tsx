// crates/vox-gui/ui/src/components/gamify/LudusSandbox.tsx  (full replacement)
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useStore } from 'zustand';
import { useLudusStore } from './store';
import { HudPanels } from './HudPanels';
import { useLlmSpend } from '../../hooks/useLlmSpend';
import {
  getSessionContextPins,
  listenAgentEvents,
  setSessionContextPins,
  voxTransport,
} from '../../transport';
import { BackendUnavailableError, sanitizeErrorForToast } from '../../lib/backendGuard';
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
import {
  computeSafePanoramaRect,
  drawDiegeticCrowdsAndGalleys,
  drawDiegeticWatercourses,
  type PanoramaRect,
} from './urbs/diegeticEnvironment';
import {
  renderInteractiveMonumentBeaconsAndHalo,
  type FxMonumentFootprintSpec,
} from '../fx/PhotorealisticCanvasFx';
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
import { Icon, RoleRankEmblem } from '../ui/Icons';
import { RomanCrestPlate } from '../ui/RomanCrestPlate';
import { RomanTooltip, RomanMedallion } from '../ui/RomanTooltip';
import { VoxSelect } from '../ui/Select';

const MIN_ZOOM = 0.15;
const MAX_ZOOM = 4;
const FIRE_FRAME_MS = 400;

function RomanCitizenSvg() {
  return (
    <svg
      data-testid="urbs-citizen-sprite"
      viewBox="0 0 16 18"
      className="h-4 w-3.5 drop-shadow-[0_2px_4px_rgba(28,20,12,0.65)]"
      fill="none"
      aria-hidden="true"
    >
      <ellipse cx="8" cy="15.8" rx="4.2" ry="1.3" fill="rgba(28,20,12,0.42)" />
      <path
        d="M8 1.4C4.9 1.4 2.4 3.9 2.4 7C2.4 10.6 8 15.2 8 15.2C8 15.2 13.6 10.6 13.6 7C13.6 3.9 11.1 1.4 8 1.4Z"
        fill="#1B6B3A"
        stroke="#D4AF37"
        strokeWidth="1.25"
      />
      <circle cx="8" cy="6.8" r="2.5" fill="#F3E5AB" stroke="#8C6214" strokeWidth="0.7" />
    </svg>
  );
}

export interface MonumentCallout {
  id: string;
  romanTier: string;
  label: string;
  shortLabel: string;
  crateHint: string;
  surface: string;
  targetView: string;
  purpose: string;
  /** Legacy CSS fallback % */
  left: string;
  top: string;
  /** Exact normalized `[u, v]` center of the painted building in `urbs-white-gold-citadel-bg.jpg`. */
  buildingUv: [number, number];
  /** Exact normalized `[u, v]` stone plinth/steps anchor where the diegetic inscription sits. */
  plinthUv: [number, number];
  /** Normalized `[rx, ry]` architectural halo footprint radii relative to panorama dimensions. */
  haloRadiusUv: [number, number];
  monumentArt: string;
  defaultFilePath: string;
}

export const MONUMENT_CALLOUTS: MonumentCallout[] = [
  {
    id: 'monument-compiler',
    romanTier: 'I',
    label: 'Compiler & Typecheck',
    shortLabel: 'Compiler',
    crateHint: 'vox-compiler',
    surface: 'explorer',
    targetView: 'explorer',
    purpose: 'Vox parser, HIR typechecker, effect gate, and Rust/TSX code generator.',
    left: '29%',
    top: '22%',
    buildingUv: [0.288, 0.138],
    plinthUv: [0.292, 0.218],
    haloRadiusUv: [0.095, 0.075],
    monumentArt: '/assets/romanesque/monument-temple-compiler.jpg',
    defaultFilePath: 'crates/vox-compiler/src/typeck/mod.rs',
  },
  {
    id: 'monument-orchestrator',
    romanTier: 'IV',
    label: 'Agent Orchestrator',
    shortLabel: 'Orchestrator',
    crateHint: 'vox-orchestrator',
    surface: 'flow',
    targetView: 'flow',
    purpose: 'Multi-agent turn coordinator, spoke router, worktree leases, and tool verification.',
    left: '14%',
    top: '74%',
    buildingUv: [0.108, 0.655],
    plinthUv: [0.138, 0.742],
    haloRadiusUv: [0.088, 0.078],
    monumentArt: '/assets/romanesque/monument-pantheon-orchestrator.jpg',
    defaultFilePath: 'crates/vox-orchestrator/src/router.rs',
  },
  {
    id: 'monument-gui',
    romanTier: 'V',
    label: 'GUI & Surfaces',
    shortLabel: 'GUI Surfaces',
    crateHint: 'vox-gui',
    surface: 'dashboard',
    targetView: 'dashboard',
    purpose: 'Vox Axis Tauri 2 desktop workbench and all 32 interactive operator surfaces.',
    left: '57%',
    top: '28%',
    buildingUv: [0.562, 0.202],
    plinthUv: [0.575, 0.278],
    haloRadiusUv: [0.078, 0.072],
    monumentArt: '/assets/romanesque/monument-rotunda-gui.jpg',
    defaultFilePath: 'crates/vox-gui/ui/src/App.tsx',
  },
  {
    id: 'monument-db',
    romanTier: 'III',
    label: 'Database & Knowledge',
    shortLabel: 'Scientia DB',
    crateHint: 'vox-db',
    surface: 'memory',
    targetView: 'memory',
    purpose: 'Turso/SQLite state store, Tantivy hybrid search index, and Scientia knowledge graph.',
    left: '47%',
    top: '37%',
    buildingUv: [0.478, 0.298],
    plinthUv: [0.468, 0.368],
    haloRadiusUv: [0.078, 0.058],
    monumentArt: '/assets/romanesque/monument-library-knowledge.jpg',
    defaultFilePath: 'crates/vox-db/src/turso.rs',
  },
  {
    id: 'monument-mens',
    romanTier: 'VII',
    label: 'MENS Local Models',
    shortLabel: 'MENS Forge',
    crateHint: 'vox-mens',
    surface: 'mens',
    targetView: 'mens',
    purpose: 'Native QLoRA fine-tuning ladder (0.6B–32B), 8-spoke adapters, and Hugging Face sync.',
    left: '86%',
    top: '69%',
    buildingUv: [0.865, 0.605],
    plinthUv: [0.862, 0.692],
    haloRadiusUv: [0.088, 0.072],
    monumentArt: '/assets/romanesque/monument-foundry-mens.jpg',
    defaultFilePath: 'crates/vox-mens/src/qlora.rs',
  },
  {
    id: 'monument-crypto',
    romanTier: 'II',
    label: 'Security & Secrets',
    shortLabel: 'Clavis Vault',
    crateHint: 'vox-crypto',
    surface: 'settings',
    targetView: 'settings',
    purpose: 'Clavis vault governance, BLAKE3 and Ed25519 cryptographic envelopes, and policy gates.',
    left: '84%',
    top: '35%',
    buildingUv: [0.84, 0.248],
    plinthUv: [0.842, 0.348],
    haloRadiusUv: [0.112, 0.088],
    monumentArt: '/assets/romanesque/monument-vault-crypto.jpg',
    defaultFilePath: 'crates/vox-crypto/src/envelope.rs',
  },
  {
    id: 'monument-mesh',
    romanTier: 'VI',
    label: 'Compute Mesh & Queue',
    shortLabel: 'Populi Mesh',
    crateHint: 'vox-populi',
    surface: 'mesh',
    targetView: 'mesh',
    purpose: 'Distributed iroh QUIC peer mesh, live VRAM selector, and task dispatch queue.',
    left: '72%',
    top: '62%',
    buildingUv: [0.73, 0.552],
    plinthUv: [0.722, 0.622],
    haloRadiusUv: [0.075, 0.065],
    monumentArt: '/assets/romanesque/monument-portus-mesh.jpg',
    defaultFilePath: 'crates/vox-populi/src/mesh.rs',
  },
];

function hitTestPanoramaMonument(
  sx: number,
  sy: number,
  panoRect: PanoramaRect,
): MonumentCallout | null {
  if (panoRect.drawW < 32 || panoRect.drawH < 32) return null;
  let best: MonumentCallout | null = null;
  let bestNormDist = 1.08;
  for (const m of MONUMENT_CALLOUTS) {
    const cx = panoRect.drawX + m.buildingUv[0] * panoRect.drawW;
    const cy = panoRect.drawY + m.buildingUv[1] * panoRect.drawH;
    const rx = Math.max(24, m.haloRadiusUv[0] * panoRect.drawW * 1.08);
    const ry = Math.max(18, m.haloRadiusUv[1] * panoRect.drawH * 1.12);
    const norm = Math.hypot((sx - cx) / rx, (sy - cy) / ry);
    if (norm < bestNormDist) {
      bestNormDist = norm;
      best = m;
    }
  }
  return best;
}

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
  const rendererRef = useRef<WorldRenderer | null>(null);
  const cameraRef = useRef<Camera>({ x: 0, y: 0, zoom: 0.3 });
  const dragRef = useRef<{ sx: number; sy: number; camX: number; camY: number } | null>(null);
  const citizensRef = useRef<Record<string, Citizen>>({});
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
  const [lens, setLens] = useState<UrbsLens>('architecture');
  const [hoveredMonumentId, setHoveredMonumentId] = useState<string | null>(null);
  const [canvasHoveredMonumentId, setCanvasHoveredMonumentId] = useState<string | null>(null);
  const hoveredMonumentRef = useRef<string | null>(null);
  const lastPanoRectRef = useRef<PanoramaRect>({ drawX: 0, drawY: 0, drawW: 800, drawH: 520 });
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

  // Hydrate quarantined districts, session context pins, and claimed yield XP from backend on mount
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
    if (typeof getSessionContextPins === 'function') {
      getSessionContextPins('default')
        .then((pins) => {
          if (!active || !Array.isArray(pins) || pins.length === 0) return;
          const hydrated = pins
            .map((pin) =>
              pin.id.startsWith('crate:')
                ? pin.id.slice('crate:'.length)
                : pin.label.replace(/^@crate:/, ''),
            )
            .filter(Boolean);
          if (hydrated.length > 0) {
            setPinnedCrates(new Set(hydrated));
          }
        })
        .catch(() => {});
    }
    const t = voxTransport as typeof voxTransport & {
      getGuiPreference?: (key: string) => Promise<string | null>;
    };
    if (typeof t.getGuiPreference === 'function') {
      t.getGuiPreference('vox.urbs.claimed_yield_xp.v1')
        .then((saved) => {
          if (!active || saved == null) return;
          const parsed = Number(saved);
          if (Number.isFinite(parsed) && parsed >= 0) {
            setClaimedYieldXp(parsed);
          }
        })
        .catch(() => {});
    }
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

  const { layout, scanRoot, layoutFailed } = React.useMemo(() => {
    const scan = presetId === 'live' ? liveWorkspaceScan : activePreset.scan;
    if (!scan || scan.crates.length === 0) {
      return { layout: null as TownLayout | null, scanRoot: '', layoutFailed: false };
    }
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
      const errorCount = c.files.reduce((s, f) => s + (buildings[f.path]?.errors ?? 0), 0);
      const warningCount = c.files.reduce((s, f) => s + (buildings[f.path]?.warnings ?? 0), 0);
      const needsYouGates = activeGates.filter((g) => g.district === c.name).length;
      attentionByDistrict[c.name] = {
        activeAgentCount,
        dirtyFileCount: c.dirty_file_count ?? 0,
        totalChurnLines: c.total_churn_lines ?? 0,
        errorCount,
        warningCount,
        needsYouGates,
        isSelected: false,
      };
    }
    try {
      return {
        layout: layoutTown(scan, gods, {
          focusMode,
          attentionByDistrict,
          activeFiles,
        }),
        scanRoot: scan.root,
        layoutFailed: false,
      };
    } catch (err) {
      console.error('[urbs] layoutTown failed:', err);
      return { layout: null as TownLayout | null, scanRoot: scan.root, layoutFailed: true };
    }
  }, [presetId, focusMode, liveWorkspaceScan, activePreset, activeGates, agentTasks, buildings]);

  const layoutRef = useRef<TownLayout | null>(null);
  layoutRef.current = layout;

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
    const fallbackTimer = setTimeout(() => {
      if (live) {
        setLiveWorkspaceScan((prev) => prev ?? DEFAULT_LIVE_FALLBACK_SCAN);
      }
    }, 3_500);
    voxTransport.workspaceTownScan()
      .then((rawScan) => {
        clearTimeout(fallbackTimer);
        if (!live) return;
        const scan =
          rawScan && Array.isArray(rawScan.crates) && rawScan.crates.length > 0
            ? rawScan
            : DEFAULT_LIVE_FALLBACK_SCAN;
        setScanFailed(false);
        setLiveWorkspaceScan(scan);
      })
      .catch((err: unknown) => {
        clearTimeout(fallbackTimer);
        if (!live) return;
        if (err instanceof BackendUnavailableError) {
          setScanFailed(false);
          setLiveWorkspaceScan(DEFAULT_LIVE_FALLBACK_SCAN);
        } else {
          setScanFailed(true);
        }
      });
    return () => {
      live = false;
      clearTimeout(fallbackTimer);
    };
  }, []);

  // ── Blit: offscreen buffer → screen with camera transform ───────────────
  const dioramaImgRef = useRef<HTMLImageElement | null>(null);
  const canvas2dUnsupportedRef = useRef(
    typeof navigator !== 'undefined' && /jsdom/i.test(navigator.userAgent),
  );
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
    if (canvas2dUnsupportedRef.current) return;
    let ctx: CanvasRenderingContext2D | null = null;
    try {
      ctx = canvas.getContext('2d');
    } catch {
      ctx = null;
    }
    if (!ctx) {
      canvas2dUnsupportedRef.current = true;
      return;
    }
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
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const cw = canvas.width / dpr;
    const ch = canvas.height / dpr;
    ctx.clearRect(0, 0, cw, ch);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    const nowMs = performance.now();
    const activeTaskCount = Object.keys(agentTasks).length;
    const totalErrCount = Object.values(buildings).reduce((s, b) => s + (b.errors ?? 0), 0);
    const bgImg = dioramaImgRef.current;
    const baseCam = fitBounds(
      occupiedWorldBounds(layout),
      cw,
      ch,
      embedded ? 12 : 20,
    );
    const relZoom = Math.max(
      0.88,
      Math.min(1.45, cam.zoom / Math.max(0.05, baseCam.zoom)),
    );
    const relDx = (cam.x - baseCam.x) * 0.42;
    const relDy = (cam.y - baseCam.y) * 0.42;
    const basePanoRect =
      bgImg && bgImg.complete && bgImg.naturalWidth > 0
        ? computeSafePanoramaRect(cw, ch, bgImg.naturalWidth, bgImg.naturalHeight)
        : { drawX: 0, drawY: 0, drawW: cw, drawH: ch };
    const panoRect = {
      drawX: basePanoRect.drawX * relZoom + cw * (1 - relZoom) * 0.5 + relDx,
      drawY: basePanoRect.drawY * relZoom + ch * (1 - relZoom) * 0.5 + relDy,
      drawW: basePanoRect.drawW * relZoom,
      drawH: basePanoRect.drawH * relZoom,
    };
    lastPanoRectRef.current = panoRect;
    if (bgImg && bgImg.complete && bgImg.naturalWidth > 0) {
      ctx.save();
      const skyGrad = ctx.createLinearGradient(0, 0, 0, ch);
      skyGrad.addColorStop(0, '#DFEDF5');
      skyGrad.addColorStop(0.5, '#F3EBD8');
      skyGrad.addColorStop(1, '#1C6E85');
      ctx.fillStyle = skyGrad;
      ctx.fillRect(0, 0, cw, ch);
      ctx.globalAlpha = 0.99;
      ctx.drawImage(bgImg, panoRect.drawX, panoRect.drawY, panoRect.drawW, panoRect.drawH);
      ctx.restore();
    }
    drawDiegeticWatercourses(ctx, panoRect, nowMs, activeTaskCount, totalErrCount);
    drawDiegeticCrowdsAndGalleys(ctx, panoRect, nowMs, activeTaskCount);
    const hasPaintedPanorama = Boolean(bgImg && bgImg.complete && bgImg.naturalWidth > 0);
    if (!hasPaintedPanorama) {
      const renderer = (rendererRef.current ??= new WorldRenderer());
      const { canvas: buffer, scale } = renderer.ensure(state, cam.zoom);
      ctx.save();
      ctx.translate(cam.x, cam.y);
      ctx.scale(cam.zoom, cam.zoom);
      ctx.drawImage(buffer, 0, 0, buffer.width / scale, buffer.height / scale);
      renderer.drawFires(ctx, state, cam.zoom, fireFrame);
      renderer.drawAnimatedWorldEffects(ctx, state, nowMs);
      ctx.restore();
    } else {
      // Render diegetic interactive building affordance beacons + mouseover/selection architectural halos
      // in 1:1 painted panorama UV space (zero misaligned grid wireframes or dashed lines).
      const activeHoverId = hoveredMonumentRef.current;
      const monumentSpecs: FxMonumentFootprintSpec[] = MONUMENT_CALLOUTS.map((m) => {
        const shortKey = m.crateHint.replace('vox-', '');
        const matchedDist = layout.districts.find((d) => d.name.includes(shortKey));
        const hasLiveAgent = Object.values(agentTasks).some(
          (t) =>
            t.filePath.includes(m.crateHint) ||
            (matchedDist ? t.filePath.includes(matchedDist.name) : false),
        );
        const hasGateOrErr =
          activeGates.some((g) => g.district.includes(shortKey)) ||
          (matchedDist
            ? matchedDist.buildings.some((b) => (buildings[b.path]?.errors ?? 0) > 0)
            : false);
        const isQuarantined = matchedDist
          ? quarantinedDistrictsList.includes(matchedDist.name)
          : false;
        const isDistrictSelected =
          (selection?.kind === 'district' && selection.name.includes(shortKey)) ||
          (selection?.kind === 'landmark' &&
            ((selection.id === 'PRAETORIVM' && m.id === 'monument-orchestrator') ||
              (selection.id === 'CASTRVM' && m.id === 'monument-crypto') ||
              (selection.id === 'PORTVS' && m.id === 'monument-mesh') ||
              (selection.id === 'FORVM' && m.id === 'monument-compiler')));

        const lensHighlight =
          lens === 'agents' && (hasLiveAgent || m.id === 'monument-orchestrator' || m.id === 'monument-compiler')
            ? 1.0
            : lens === 'health' && hasGateOrErr
              ? 1.2
              : lens === 'cost' && (m.id === 'monument-mens' || m.id === 'monument-orchestrator')
                ? 0.95
                : lens === 'pipeline' && (m.id === 'monument-mesh' || m.id === 'monument-crypto')
                  ? 0.95
                  : 0.15;

        return {
          id: m.id,
          footprintUv: m.buildingUv,
          plinthUv: m.plinthUv,
          radiusUv: m.haloRadiusUv,
          activityState: isQuarantined
            ? 'quarantined'
            : hasGateOrErr
              ? 'attention'
              : hasLiveAgent || m.id === 'monument-compiler' || m.id === 'monument-orchestrator'
                ? 'active'
                : 'ready',
          isHovered: activeHoverId === m.id,
          isSelected: isDistrictSelected,
          lensHighlight,
        };
      });
      renderInteractiveMonumentBeaconsAndHalo(
        ctx,
        {
          x: panoRect.drawX,
          y: panoRect.drawY,
          width: panoRect.drawW,
          height: panoRect.drawH,
        },
        monumentSpecs,
        nowMs,
      );
    }
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
    embedded,
  ]);

  const blitRef = useRef(blit);
  blitRef.current = blit;

  useEffect(() => {
    if (typeof Image === 'undefined') return;
    if (dioramaImgRef.current && dioramaImgRef.current.complete && dioramaImgRef.current.naturalWidth > 0) {
      return;
    }
    const img = new Image();
    img.src = '/assets/romanesque/urbs-white-gold-citadel-bg.jpg';
    img.onload = () => {
      dioramaImgRef.current = img;
      blitRef.current();
    };
    dioramaImgRef.current = img;
  }, []);

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
      if (first && wrap.clientWidth > 0 && wrap.clientHeight > 0) {
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
    if (canvas2dUnsupportedRef.current) return;
    const anyErrors = Object.values(buildings).some((b) => b.errors > 0);
    if (!anyErrors || speed === 0) return;
    const id = setInterval(() => setFireFrame((f) => (f ? 0 : 1)), FIRE_FRAME_MS / Math.max(speed, 1));
    return () => clearInterval(id);
  }, [buildings, speed]);

  // ── Pan / zoom + interactive monument hover detection ────────────────────
  const syncMonumentHover = (nextId: string | null, fromCanvas: boolean) => {
    if (hoveredMonumentRef.current !== nextId) {
      hoveredMonumentRef.current = nextId;
      setHoveredMonumentId(nextId);
      blitRef.current();
    }
    setCanvasHoveredMonumentId((prev) => {
      const target = fromCanvas ? nextId : null;
      return prev === target ? prev : target;
    });
  };

  const onPointerDown = (e: React.PointerEvent) => {
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    const cam = cameraRef.current;
    dragRef.current = { sx: e.clientX, sy: e.clientY, camX: cam.x, camY: cam.y };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    const d = dragRef.current;
    const wrap = wrapRef.current;
    if (!wrap || !layout) return;
    if (!d) {
      const rect = wrap.getBoundingClientRect();
      const hitMon = hitTestPanoramaMonument(
        e.clientX - rect.left,
        e.clientY - rect.top,
        lastPanoRectRef.current,
      );
      syncMonumentHover(hitMon ? hitMon.id : null, true);
      return;
    }
    if (Math.hypot(e.clientX - d.sx, e.clientY - d.sy) < 3) return;
    syncMonumentHover(null, false);
    cameraRef.current = clampCamera(
      { ...cameraRef.current, x: d.camX + (e.clientX - d.sx), y: d.camY + (e.clientY - d.sy) },
      worldBounds(layout), wrap.clientWidth, wrap.clientHeight,
    );
    blit();
  };
  const onPointerUp = () => { dragRef.current = null; };
  const onPointerLeave = () => {
    if (!dragRef.current) {
      syncMonumentHover(null, false);
    }
  };
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
    const baseCam = fitBounds(
      occupiedWorldBounds(layout),
      wrap.clientWidth,
      wrap.clientHeight,
      embedded ? 12 : 20,
    );
    const zoom = Math.min(
      MAX_ZOOM,
      Math.max(cameraRef.current.zoom, baseCam.zoom * 1.12),
    );
    const targetX = wrap.clientWidth / 2 - px * zoom;
    const targetY = wrap.clientHeight / 2 - py * zoom;
    cameraRef.current = clampCamera(
      {
        zoom,
        x: baseCam.x * 0.62 + targetX * 0.38,
        y: baseCam.y * 0.62 + targetY * 0.38,
      },
      worldBounds(layout),
      wrap.clientWidth,
      wrap.clientHeight,
    );
    blit();
  };

  // ── Click → painted monument / landmark / viaduct / building / district hit-test ──
  const onClick = (e: React.MouseEvent) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout || dragRef.current) return;
    const rect = wrap.getBoundingClientRect();
    const clickSx = e.clientX - rect.left;
    const clickSy = e.clientY - rect.top;

    // 0. Check painted interactive monument footprint first (1:1 visual building accuracy)
    const hitMon = hitTestPanoramaMonument(clickSx, clickSy, lastPanoRectRef.current);
    if (hitMon) {
      const shortKey = hitMon.crateHint.replace('vox-', '');
      const matchedDistrict =
        layout.districts.find((d) => d.name.includes(shortKey)) ?? layout.districts[0];
      if (matchedDistrict) {
        if (e.altKey) {
          toggleDistrictQuarantine(matchedDistrict.name);
        }
        jumpToTile(
          (matchedDistrict.x0 + matchedDistrict.x1) / 2,
          (matchedDistrict.y0 + matchedDistrict.y1) / 2,
        );
        const alreadySelected =
          selection?.kind === 'district' && selection.name === matchedDistrict.name;
        setSelection({ kind: 'district', name: matchedDistrict.name });
        if (onNavigate && (alreadySelected || e.metaKey || e.shiftKey || embedded)) {
          onNavigate(hitMon.targetView);
        }
      }
      setMenu(null);
      return;
    }

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
      const curLayout = layoutRef.current;
      if (k.type === 'file_edited' && typeof k.path === 'string' && curLayout) {
        if (!curLayout.byPath[k.path]) assignNewFile(curLayout, k.path);
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
  }, []);

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
    if (!layout || canvas2dUnsupportedRef.current) return;
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
  // Compute live safe-frame panorama rectangle in React render space so diegetic monument
  // plinth plaques and garrison vexillum standards stay 100% locked to their painted buildings.
  const livePanoRect: PanoramaRect = React.useMemo(() => {
    const cw = Math.max(1, viewportSize.w);
    const ch = Math.max(1, viewportSize.h);
    const baseRect = computeSafePanoramaRect(cw, ch, 1920, 1080);
    if (!layout) return baseRect;
    const baseCam = fitBounds(occupiedWorldBounds(layout), cw, ch, embedded ? 12 : 20);
    const relZoom = Math.max(
      0.88,
      Math.min(1.45, camView.zoom / Math.max(0.05, baseCam.zoom)),
    );
    const relDx = (camView.x - baseCam.x) * 0.42;
    const relDy = (camView.y - baseCam.y) * 0.42;
    return {
      drawX: baseRect.drawX * relZoom + cw * (1 - relZoom) * 0.5 + relDx,
      drawY: baseRect.drawY * relZoom + ch * (1 - relZoom) * 0.5 + relDy,
      drawW: baseRect.drawW * relZoom,
      drawH: baseRect.drawH * relZoom,
    };
  }, [viewportSize.w, viewportSize.h, layout, camView.x, camView.y, camView.zoom, embedded]);

  const vexillumMeta: Record<
    string,
    { legion: string; rank: string; shortStandard: string; model: string; task: string; uv: [number, number]; dotColor: string }
  > = {
    'AEDILIS-01': {
      legion: 'LEGIO-01',
      rank: 'praetor',
      shortStandard: 'I',
      model: 'Claude Sonnet 4.5',
      task: `Editing ${primaryEditFile}`,
      uv: [0.382, 0.238], // Stationed on paved Acropolis eastern terrace beside vox-compiler
      dotColor: 'bg-(--color-success,rgb(16_185_129))',
    },
    'AVGVR-02': {
      legion: 'LEGIO-02',
      rank: 'legatus',
      shortStandard: 'II',
      model: 'Gemini 2.5 Pro',
      task: `Running ${polyglotTestCmd}`,
      uv: [0.518, 0.592], // Stationed on paved Forum canal promenade beside Nymphaeum
      dotColor: 'bg-(--color-success,rgb(16_185_129))',
    },
    'FABER-03': {
      legion: 'LEGIO-03',
      rank: 'faber',
      shortStandard: 'III',
      model: 'MENS local',
      task: `Indexing ${layout?.districts[0]?.name ?? 'workspace'}`,
      uv: [0.648, 0.448], // Stationed on sunlit upper Grand Canal plaza clear of monument plaques
      dotColor: 'bg-[#B8860B]',
    },
  };

  const ambientRosters =
    layout && layout.districts.length > 0
      ? (isCompactViewport ? ['AEDILIS-01', 'AVGVR-02'] : ['AEDILIS-01', 'AVGVR-02', 'FABER-03'])
          .filter((id) => !storeAgentTasks[id])
          .map((id, idx) => {
            const meta = vexillumMeta[id];
            const uv: [number, number] = meta?.uv ?? [0.36 + idx * 0.16, 0.58];
            if (viewportSize.w > 120 && viewportSize.h > 100) {
              const rawX = livePanoRect.drawX + uv[0] * livePanoRect.drawW;
              const rawY = livePanoRect.drawY + uv[1] * livePanoRect.drawH;
              const minY = embedded ? 38 : 64;
              const maxY = viewportSize.h - (embedded ? 34 : 76);
              const maxX =
                viewportSize.w -
                (embedded ? 56 : showRadarDrawer || showPriorityDrawer ? 176 : 56);
              const clampedX = Math.max(48, Math.min(maxX, rawX));
              const clampedY = Math.max(minY, Math.min(maxY, rawY));
              return { id, left: `${Math.round(clampedX)}px`, top: `${Math.round(clampedY)}px` };
            }
            return {
              id,
              left: `${Math.round(uv[0] * 100)}%`,
              top: `${Math.round(uv[1] * 100)}%`,
            };
          })
      : [];

  const handleBannerDrop = (agentId: string, clientX: number, clientY: number) => {
    const wrap = wrapRef.current;
    if (!wrap || !layout) return;
    const rect = wrap.getBoundingClientRect();
    const dropSx = clientX - rect.left;
    const dropSy = clientY - rect.top;

    // First check painted interactive monuments in panorama UV space
    const hitMon = hitTestPanoramaMonument(dropSx, dropSy, lastPanoRectRef.current);
    if (hitMon) {
      const shortKey = hitMon.crateHint.replace('vox-', '');
      const hitDist =
        layout.districts.find((d) => d.name.includes(shortKey)) ?? layout.districts[0];
      const targetPath = hitDist?.buildings[0]?.path ?? hitMon.defaultFilePath;
      useLudusStore.getState().updateAgentTask(agentId, {
        taskId: agentId,
        filePath: targetPath,
        status: `Assigned via Vexillum drop to ${targetPath}`,
      });
      if (hitDist) setSelection({ kind: 'district', name: hitDist.name });
      void voxTransport
        .callTool('vox_submit_task', {
          description: `Fix & inspect ${targetPath}`,
          file_affinity: [targetPath],
        })
        .catch(() => {});
      return;
    }

    const { wx, wy } = screenToWorld(cameraRef.current, dropSx, dropSy);
    const { x: tx, y: ty } = tileFromWorld(layout, wx, wy);
    const hitPlot = Object.values(layout.byPath).find((p) => p.x === tx && p.y === ty);
    const hitDist = layout.districts.find((d) => tx >= d.x0 && tx < d.x1 && ty >= d.y0 && ty < d.y1);
    const targetPath = hitPlot?.path ?? hitDist?.buildings[0]?.path;
    if (targetPath) {
      useLudusStore.getState().updateAgentTask(agentId, {
        taskId: agentId,
        filePath: targetPath,
        status: `Assigned via Vexillum drop to ${targetPath}`,
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
  if ((presetId === 'live' && scanFailed) || layoutFailed) {
    return (
      <div className="flex h-full w-full items-center justify-center rounded-2xl border border-border-subtle bg-bg-surface text-sm text-text-muted">
        Workspace scan unavailable — the town cannot render.
      </div>
    );
  }
  return (
    <div
      ref={wrapRef}
      className="relative h-full w-full overflow-hidden rounded-xl border border-[#C59B27]/65 bg-[#F5EFE2]"
    >
      {/* Phase 384 Uncropped Safe-Frame Sunlit White/Gold Roman Metropolis Panorama */}
      <img
        src="/assets/romanesque/urbs-white-gold-citadel-bg.jpg"
        alt=""
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 h-full w-full object-cover object-center opacity-98 select-none"
      />
      <canvas
        ref={canvasRef}
        data-testid="urbs-canvas"
        data-diorama-composited="true"
        data-hovered-monument={hoveredMonumentId ?? undefined}
        className={`absolute inset-0 ${
          hoveredMonumentId ? 'cursor-pointer' : 'cursor-grab active:cursor-grabbing'
        } opacity-100 ${layout ? '' : 'invisible'}`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerLeave}
        onWheel={onWheel}
        onClick={onClick}
        onDoubleClick={(e) => {
          if (!onNavigate || !wrapRef.current) return;
          const rect = wrapRef.current.getBoundingClientRect();
          const hitMon = hitTestPanoramaMonument(
            e.clientX - rect.left,
            e.clientY - rect.top,
            lastPanoRectRef.current,
          );
          if (hitMon) {
            e.stopPropagation();
            onNavigate(hitMon.targetView);
          }
        }}
      />

      {/* Diegetic Sunlit Vignette + Architectural Plinth Entablature Inscriptions */}
      {layout && (
        <div className="pointer-events-none absolute inset-0 z-[5]">
          <svg
            data-testid="urbs-diegetic-water-svg"
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 h-full w-full opacity-90"
          >
            <defs>
              <linearGradient id="urbs-sunlit-vignette" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stopColor="#FFF7D6" stopOpacity="0.12" />
                <stop offset="60%" stopColor="#FFF7D6" stopOpacity="0" />
                <stop offset="100%" stopColor="var(--color-text-primary)" stopOpacity="0.09" />
              </linearGradient>
            </defs>
            <rect x="0" y="0" width="100" height="100" fill="url(#urbs-sunlit-vignette)" />
          </svg>

          {/* Subtle Diegetic Affordance Legend (Top-Left beneath Architrave Bar: clarifies interactive monuments vs scenery) */}
          {!embedded && viewportSize.w >= 720 && viewportSize.h >= 420 && (
            <div
              data-testid="urbs-interactivity-guide"
              className="pointer-events-none absolute left-3 top-[58px] z-10 flex items-center gap-1.5 rounded-[5px] border border-border-strong bg-bg-surface/94 px-2 py-0.5 font-sans text-[8.5px] font-semibold text-text-primary shadow-[0_3px_10px_rgba(28,20,12,0.22)]"
            >
              <span className="inline-block size-1.5 rounded-full bg-(--color-status-pass) animate-pulse" />
              <span>Lit plinths = Interactive crates (hover for halo &amp; vignette · click to inspect)</span>
            </div>
          )}

          {MONUMENT_CALLOUTS.filter(
            (m) =>
              !embedded ||
              (viewportSize.w >= 760
                ? ['monument-compiler', 'monument-gui', 'monument-crypto', 'monument-orchestrator', 'monument-mens'].includes(m.id)
                : ['monument-compiler', 'monument-orchestrator', 'monument-mens'].includes(m.id)),
          ).map((m) => {
            const shortKey = m.crateHint.replace('vox-', '');
            const matchedDistrict = layout.districts.find((d) => d.name.includes(shortKey));
            const liveTaskEntry = Object.entries(agentTasks).find(
              ([, t]) =>
                t.filePath.includes(m.crateHint) ||
                (matchedDistrict && t.filePath.includes(matchedDistrict.name)),
            );
            const liveTaskForMonument = liveTaskEntry?.[1];
            const assignedCohort =
              liveTaskEntry?.[0] ??
              (m.id === 'monument-compiler'
                ? 'LEGIO-01'
                : m.id === 'monument-db'
                  ? 'LEGIO-02'
                  : m.id === 'monument-mens'
                    ? 'LEGIO-03'
                    : 'Standby');
            const errCount = matchedDistrict
              ? matchedDistrict.buildings.reduce(
                  (acc, b) => acc + (buildings[b.path]?.errors ?? 0),
                  0,
                )
              : 0;
            const gateCount = activeGates.filter((g) => g.district.includes(shortKey)).length;
            const activeFilePath =
              liveTaskForMonument?.filePath ??
              matchedDistrict?.buildings.find((b) => b.isTouched || (b.diffAdded ?? 0) > 0)?.path ??
              m.defaultFilePath;
            const shortFileBadge = activeFilePath.split('/').slice(-2).join('/');
            const isAlreadySelected =
              selection?.kind === 'district' && selection.name.includes(shortKey);
            const isHovered = hoveredMonumentId === m.id;
            const isActiveBeacon =
              Boolean(liveTaskForMonument) ||
              m.id === 'monument-compiler' ||
              m.id === 'monument-orchestrator';
            const hasAlert = errCount > 0 || gateCount > 0;

            // Project monument plinth UV directly through `livePanoRect` so inscription sits
            // physically on the carved stone base of its building at every resolution & camera pan.
            let leftStyle = m.left;
            let topStyle = m.top;
            if (viewportSize.w > 120 && viewportSize.h > 80) {
              const rawX = livePanoRect.drawX + m.plinthUv[0] * livePanoRect.drawW;
              const rawY = livePanoRect.drawY + m.plinthUv[1] * livePanoRect.drawH;
              const padX = embedded ? 58 : 78;
              const minTop = embedded ? 36 : 54;
              const maxTop = viewportSize.h - (embedded ? 20 : 54);
              const clampedX = Math.max(padX, Math.min(viewportSize.w - padX, rawX));
              const clampedY = Math.max(minTop, Math.min(maxTop, rawY));
              leftStyle = `${Math.round(clampedX)}px`;
              topStyle = `${Math.round(clampedY)}px`;
            }

            const rawModuleLeaf = shortFileBadge.split('/').pop() ?? shortFileBadge;
            const conciseModuleLeaf =
              rawModuleLeaf.length > 14
                ? rawModuleLeaf.replace(/\.(tsx|ts|rs)$/, '')
                : rawModuleLeaf;

            // Compact architectural plinth inscription fits cleanly within each building's carved base
            const plaqueLabel = m.shortLabel;
            const activityTag = hasAlert
              ? `${errCount + gateCount}!`
              : liveTaskForMonument
                ? 'LIVE'
                : assignedCohort !== 'Standby'
                  ? assignedCohort.replace('LEGIO-0', 'L')
                  : null;

            return (
              <div
                key={m.id}
                style={{ left: leftStyle, top: topStyle }}
                onMouseEnter={() => syncMonumentHover(m.id, false)}
                onMouseLeave={() => syncMonumentHover(null, false)}
                className={`pointer-events-auto absolute -translate-x-1/2 -translate-y-1/2 transition-transform duration-150 ${
                  isHovered || isAlreadySelected ? 'z-20 scale-[1.06]' : 'z-10'
                }`}
              >
                <RomanTooltip
                  title={`${m.romanTier} · ${m.label}`}
                  purpose={m.purpose}
                  previewImageSrc={m.monumentArt}
                  crateBadge={m.crateHint}
                  statusBadge={hasAlert ? `${errCount + gateCount} Gate/Diag` : isActiveBeacon ? 'Active Sanctuary' : 'Interactive'}
                  metrics={[
                    {
                      label: 'Garrison',
                      value: assignedCohort,
                      tone: isActiveBeacon ? 'emerald' : 'gold',
                    },
                    {
                      label: 'Health',
                      value: errCount > 0 ? `${errCount} Err` : gateCount > 0 ? `${gateCount} Gate` : 'Verified',
                      tone: hasAlert ? 'crimson' : 'emerald',
                    },
                    {
                      label: 'Module',
                      value: conciseModuleLeaf,
                      tone: 'gold',
                    },
                  ]}
                  actionHint={
                    onNavigate
                      ? `Click to inspect ${m.crateHint} · Click again, double-click, or press open icon to open ${m.targetView}`
                      : `Click to focus & inspect ${m.crateHint} on the map`
                  }
                  forceOpen={canvasHoveredMonumentId === m.id}
                  placement={m.plinthUv[1] < 0.42 ? 'bottom' : 'top'}
                >
                  <button
                    type="button"
                    data-testid={`urbs-${m.id}`}
                    data-diegetic-plaque="true"
                    data-activity-state={hasAlert ? 'attention' : isActiveBeacon ? 'active' : 'ready'}
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
                    className={`group relative inline-flex items-center gap-1 rounded-[4px] border-y-[1.5px] border-x border-border-strong bg-bg-surface text-text-primary ${
                      isHovered || isAlreadySelected
                        ? 'shadow-[0_6px_16px_rgba(28,20,12,0.45),0_0_18px_rgba(245,198,72,0.72),inset_0_1px_0_rgba(255,255,255,0.95)]'
                        : 'shadow-[0_3px_9px_rgba(28,20,12,0.36),inset_0_1px_0_rgba(255,255,255,0.88)]'
                    } ${
                      embedded ? 'px-1.5 py-[1px] text-[7.5px]' : 'px-1.5 py-[2px] text-[8px]'
                    } font-display font-bold tracking-[0.06em] transition-[box-shadow,border-color] cursor-pointer`}
                  >
                    {/* Diegetic Upward Architectural Pediment Finial anchored to building plinth */}
                    <span
                      aria-hidden="true"
                      className="pointer-events-none absolute -top-[5px] left-1/2 -translate-x-1/2 h-0 w-0 border-x-[4px] border-b-[5px] border-x-transparent border-b-[#B88E28]"
                    />
                    {/* Activity Guidance Brazier Seal + Roman Tier */}
                    <span className="inline-flex items-center gap-1 font-mono text-[7.5px] font-bold text-text-primary">
                      <span
                        className={`size-1.5 shrink-0 rounded-full ${
                          hasAlert
                            ? 'animate-pulse bg-[#D97706] shadow-[0_0_5px_#F59E0B]'
                            : isActiveBeacon
                              ? 'animate-pulse bg-(--color-success,rgb(16_185_129)) shadow-[0_0_5px_var(--color-status-pass)]'
                              : 'bg-[#B8860B] shadow-[0_0_4px_#D4AF37]'
                        }`}
                        aria-hidden="true"
                      />
                      <span>{m.romanTier}·</span>
                    </span>
                    <span className="whitespace-nowrap uppercase">{plaqueLabel}</span>
                    {!embedded && activityTag && (
                      <span className="rounded-[2px] border border-border-subtle bg-bg-elevated px-1 py-[0.5px] font-mono text-[6.5px] font-bold tracking-tight text-text-primary">
                        {activityTag}
                      </span>
                    )}
                    {onNavigate && (
                      <span
                        data-testid={`open-${m.id}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          onNavigate(m.targetView);
                        }}
                        title={`Jump directly to ${m.label} (${m.targetView})`}
                        className="ml-0.5 inline-flex items-center whitespace-nowrap rounded-[2px] border border-border-subtle bg-bg-elevated p-0.5 font-display text-[7px] font-bold text-text-primary hover:border-border-strong transition-colors"
                      >
                        <Icon.externalLink className="size-2.5 shrink-0" aria-hidden="true" />
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
          className="pointer-events-auto absolute left-2 top-2 max-w-[calc(100%-108px)] px-1.5 py-0.5 text-[8.5px] z-10 flex flex-nowrap items-center justify-start gap-1 overflow-hidden rounded-xl border border-[#C59B27]/85 font-display text-(--color-text-primary) shadow-[0_6px_18px_rgba(28,20,12,0.28),inset_0_1px_0_rgba(255,255,255,0.9)]"
        >
          {[
            {
              id: 'FORVM',
              label: 'Workspace Map',
              shortLabel: 'Map',
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
              actionHint: 'Click to focus Orchestrator (click again or double-click to open Operations Flow)',
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
              actionHint: 'Click to focus CI Fortress (click again or double-click to open Pipeline Cascade)',
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
              actionHint: 'Click to focus Task Harbor (click again or double-click to open Task Queue)',
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
          ].filter((lm, idx) => !(viewportSize.w > 0 && viewportSize.w < 360 && idx >= 2)).map((lm) => {
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
                  className={`inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-lg border px-1.5 py-0.5 transition ${
                    lm.lit
                      ? 'border-[#C59B27]/80 bg-[#FAF5E8] text-(--color-text-primary) hover:border-[#B8860B] hover:bg-white'
                      : 'border-[#C59B27]/45 bg-[#EFE6D5] text-[#5C4428] hover:text-(--color-text-primary)'
                  }`}
                >
                  <span
                    className={`size-1.5 rounded-full ${
                      lm.lit ? 'bg-[#B8860B] shadow-[0_0_6px_#D4AF37]' : 'bg-[#8C7D64]'
                    }`}
                    aria-hidden="true"
                  />
                  <span className="font-semibold tracking-wider uppercase text-[8px]">
                    {lm.shortLabel}
                  </span>
                </button>
              </RomanTooltip>
            );
          })}
        </div>
      )}

      {/* Embedded Right-Hand Compact Full-Screen Map Pill */}
      {layout && embedded && (
        <div className="pointer-events-auto absolute right-2 top-2 z-10 flex items-center gap-1">
          <RomanTooltip
            title="Full-Screen Workspace Map"
            purpose={`Open the full-screen Urbs Simulator (${activeGates.length} priority alerts, Git Timeline, and 7-Region Astrolabe Radar).`}
            actionHint="Click to expand Workspace Map"
            placement="left"
          >
            <button
              type="button"
              onClick={() => onNavigate?.('gamify')}
              className="inline-flex items-center gap-1 rounded-lg border border-[#C59B27]/85 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 px-2 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-(--color-text-primary) shadow-md hover:border-[#B8860B]"
            >
              <Icon.astrolabeRadar className="size-2.5 text-[#8C6214]" aria-hidden="true" />
              <span>Full Map</span>
              <Icon.externalLink className="size-2.5 shrink-0" aria-hidden="true" />
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
          className="pointer-events-auto absolute left-2.5 right-2.5 top-2.5 z-10 flex flex-nowrap items-center justify-between gap-2 rounded-xl border-2 border-[#C59B27]/80 px-2.5 py-1 text-[10px] font-display text-(--color-text-primary) shadow-[0_8px_24px_rgba(28,20,12,0.28),inset_0_1px_0_rgba(255,255,255,0.9)]"
        >
          {/* Left: Sovereign Crest Plate + 4 compact Landmark Medallion buttons */}
          <div data-testid="urbs-landmark-bar" className="flex min-w-0 items-center gap-1.5 overflow-x-auto no-scrollbar">
            <RomanCrestPlate variant="imperium" size="sm" />
            {[
              {
                id: 'FORVM',
                label: 'Workspace Map',
                shortLabel: 'Workspace Map',
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
                actionHint: 'Click to focus Orchestrator (click again or open icon to open Operations Flow)',
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
                actionHint: 'Click to focus CI Fortress (click again or open icon to open Pipeline Cascade)',
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
                actionHint: 'Click to focus Task Harbor (click again or open icon to open Task Queue)',
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
            ].filter((lm, idx) => !(viewportSize.w > 0 && viewportSize.w < 360 && idx >= 2)).map((lm) => {
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
                    className={`inline-flex items-center gap-1.5 whitespace-nowrap rounded-lg border px-2 py-0.5 transition-colors ${
                      lm.lit
                        ? 'border-[#C59B27]/80 bg-[#FAF5E8] text-(--color-text-primary) hover:border-[#B8860B] hover:bg-white'
                        : 'border-[#C59B27]/45 bg-[#EFE6D5] text-[#5C4428] hover:text-(--color-text-primary)'
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
                        className="inline-flex items-center justify-center rounded border border-[#C59B27]/55 bg-[#F3E5AB]/70 p-0.5 font-display text-[8.5px] font-bold text-(--color-text-primary) hover:bg-[#D4AF37]/50"
                      >
                        <Icon.externalLink className="size-2.5 shrink-0" aria-hidden="true" />
                      </span>
                    )}
                  </button>
                </RomanTooltip>
              );
            })}
          </div>

          {/* Center/Right: Primary Lens + [ Lens & Layout ] + [ Alerts & History ] */}
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
                    ? 'border-[#8C6214] bg-linear-to-b from-[#FAF0D7] to-[#E6C762]/65 font-semibold text-(--color-text-primary) shadow-sm'
                    : 'border-[#C59B27]/75 bg-[#FAF5E8] text-(--color-text-primary) hover:border-[#B8860B] hover:bg-white'
                }`}
              >
                <Icon.romanColumn className={`size-3 ${lens === 'architecture' ? 'text-[#8C6214]' : 'text-[#8C6214]'}`} aria-hidden="true" />
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
                      ? 'border-[#8C6214] bg-linear-to-b from-[#FAF0D7] to-[#E6C762]/65 text-(--color-text-primary)'
                      : 'border-[#C59B27]/75 bg-[#FAF5E8] text-(--color-text-primary) hover:border-[#B8860B] hover:bg-white'
                  }`}
                >
                  <Icon.astrolabeRadar className="size-3 text-[#8C6214]" aria-hidden="true" />
                  <span>Lens &amp; Layout</span>
                  <Icon.chevronDown className="size-3 shrink-0" aria-hidden="true" />
                </button>
              </RomanTooltip>

              {/* Popover Dropdown (Always in DOM for tests/a11y, visible when showLensMenu is true) */}
              <div
                className={
                  showLensMenu
                    ? "absolute right-0 top-full mt-2 z-30 flex flex-col gap-2 rounded-xl border-2 border-[#D4AF37]/65 bg-bg-elevated/98 p-3 shadow-[0_12px_32px_rgba(0,0,0,0.3)] backdrop-blur-md min-w-[280px]"
                    : "pointer-events-none absolute -left-[9999px] top-0 z-[-1] flex flex-col gap-2 opacity-0 min-w-[280px]"
                }
              >
                <div className="flex flex-col gap-1 border-b border-[#D4AF37]/25 pb-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-display text-[9px] font-semibold uppercase tracking-widest text-[#8C6214]">Repo:</span>
                    <VoxSelect
                      size="xs"
                      data-testid="urbs-repo-preset-select"
                      aria-label="Repository simulation preset"
                      value={presetId}
                      onValueChange={(next) => {
                        setPresetId(next as UrbsRepoPresetId);
                        setSelection(null);
                      }}
                      options={Object.values(URBS_REPO_PRESETS).map((preset) => ({
                        value: preset.id,
                        label: preset.label,
                      }))}
                      className="w-auto min-w-[160px] border-[#D4AF37]/45 bg-bg-surface font-mono text-[9.5px] text-text-primary"
                    />
                  </div>
                  <div className="flex items-center justify-between gap-2 mt-1">
                    <span className="font-display text-[9px] font-semibold uppercase tracking-widest text-[#8C6214]">Layout:</span>
                    <VoxSelect
                      size="xs"
                      data-testid="urbs-focus-mode-select"
                      aria-label="Real-estate attention focus mode"
                      value={focusMode}
                      onValueChange={(next) => setFocusMode(next as RealEstateFocusMode)}
                      options={[
                        { value: 'attention', label: 'Attention Weighted' },
                        { value: 'worksite_only', label: 'Active Worksite Only' },
                        { value: 'proportional', label: 'Proportional LoC' },
                      ]}
                      className="w-auto min-w-[160px] border-[#D4AF37]/45 bg-bg-surface font-mono text-[9.5px] text-text-primary"
                    />
                  </div>
                </div>

                <div className="flex flex-col gap-1">
                  <span className="font-display text-[9px] font-semibold uppercase tracking-widest text-[#8C6214]">Map Lenses:</span>
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
                              ? 'border-[#8C6214] bg-linear-to-b from-[#FAF0D7] to-[#E6C762]/65 font-semibold text-text-primary'
                              : 'border-border-subtle bg-bg-surface text-text-secondary hover:border-[#D4AF37]/60 hover:text-text-primary'
                          }`}
                        >
                          <LensIcon className={`size-3 ${active ? 'text-[#8C6214]' : 'text-[#D4AF37]'}`} aria-hidden="true" />
                          <span className="truncate">{item.label}</span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
            </div>

            {/* Compact [ Alerts & History ] Button */}
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
                className={`inline-flex items-center gap-1.5 rounded-lg border px-2 py-0.5 font-display text-[9.5px] font-semibold uppercase tracking-wider transition-colors ${
                  showPriorityDrawer || showTimelineDrawer
                    ? 'border-[#8C6214] bg-linear-to-b from-[#FAF0D7] to-[#E6C762]/65 text-(--color-text-primary)'
                    : 'border-[#C59B27]/75 bg-[#FAF5E8] text-(--color-text-primary) hover:border-[#B8860B] hover:bg-white'
                }`}
              >
                <Icon.gladiusShield className="size-3 text-[#8C6214]" aria-hidden="true" />
                <span>Alerts &amp; History</span>
                <Icon.chevronDown className="size-3 shrink-0" aria-hidden="true" />
                {activeGates.length > 0 && (
                  <span className="rounded-full border border-[#C59B27]/50 bg-[#F3E5AB] px-1 font-sans text-[8.5px] font-bold text-(--color-text-primary)">
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
              ? "pointer-events-auto absolute right-2.5 top-16 z-10 w-[236px] rounded-xl border-2 border-[#C59B27]/65 bg-bg-elevated/95 p-2.5 text-text-primary shadow-lg"
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
                aria-label="Close Priority Alerts"
                className="inline-flex items-center justify-center rounded p-0.5 text-[#8C7D64] hover:text-[#F3E5AB] transition-colors"
              >
                <Icon.close className="size-3 shrink-0" aria-hidden="true" />
              </button>
            </div>
          </div>
          <div className="max-h-36 space-y-1 overflow-y-auto pr-0.5 custom-scrollbar">
            {activeGates.map((gate) => {
              const dist = layout.districts.find((d) => d.name === gate.district) ?? layout.districts[0];
              return (
                <div
                  key={`gate-${gate.id}`}
                  className="flex w-full items-center justify-between gap-1 rounded border border-[#D4AF37]/50 bg-bg-elevated/90 px-2 py-1 font-mono text-[9px] text-[#F3E5AB]"
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
                    className="shrink-0 rounded border border-emerald-600/55 bg-emerald-500/15 px-1.5 py-0.5 text-[8px] font-semibold text-emerald-700 hover:bg-emerald-500/25"
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
                      className="shrink-0 rounded border border-[#D4AF37]/60 bg-bg-surface px-1.5 py-0.5 text-[8px] font-semibold text-text-primary hover:border-[#D4AF37]"
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
                    className="flex w-full items-center justify-between gap-1 rounded border border-[#D4AF37]/40 bg-bg-surface px-2 py-1 text-left font-mono text-[9px] text-text-primary hover:border-[#D4AF37]"
                  >
                    <span className="inline-flex min-w-0 items-center gap-1 truncate">
                      <RoleRankEmblem rank={agentId} className="size-2.5 shrink-0 text-[#8C6214]" />
                      <span>{agentId}</span>
                      <Icon.arrowRight className="size-2.5 shrink-0 text-text-secondary" aria-hidden="true" />
                      <span className="truncate">{shortName}</span>
                    </span>
                    <span className="shrink-0 text-[8.5px] text-emerald-700 font-semibold">ACTIVE</span>
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
                      className="flex w-full items-center justify-between gap-1 rounded border border-[#7B8C32]/50 bg-bg-surface/90 px-2 py-1 text-left font-mono text-[9px] text-text-primary hover:bg-bg-elevated"
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
              <span className="rounded-sm border border-[#D4AF37]/60 bg-bg-elevated/95 px-1.5 py-0.5 font-mono text-[9px] text-[#F3E5AB] shadow-md">
                {id} · {agentTasks[id]?.filePath.split('/').pop() ?? 'active'}
              </span>
            </div>
          </div>
        ))}
        {ambientRosters.map((c) => {
          const meta = vexillumMeta[c.id] ?? {
            legion: c.id,
            rank: 'centurion',
            shortStandard: 'I',
            model: 'Claude Sonnet 4.5',
            task: 'Patrolling Workspace',
            uv: [0.382, 0.238] as [number, number],
            dotColor: 'bg-(--color-success,rgb(16_185_129))',
          };
          return (
            <div
              key={c.id}
              data-testid="urbs-vexillum-banner"
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
              <RomanTooltip
                title={`${meta.legion} · ${meta.model}`}
                purpose={`Stationed cohort standard (${meta.task}).`}
                actionHint="Drag standard onto any lit monument to dispatch this agent cohort"
                statusBadge="Cohort Ready"
                disabled={isCompactViewport || Boolean(dragBanner)}
              >
                <div className="group flex flex-col items-center">
                  {!isCompactViewport && (
                    <div className="mb-0.5 inline-flex items-center gap-1 rounded-[4px] border border-border-strong bg-bg-surface px-1.5 py-[1px] font-display text-[7.5px] font-bold tracking-wider text-text-primary shadow-[0_3px_8px_rgba(28,20,12,0.38)] transition-transform group-hover:scale-105">
                      <span className={`size-1.5 rounded-full ${meta.dotColor} animate-pulse`} aria-hidden="true" />
                      <RoleRankEmblem rank={meta.rank} className="size-2.5 shrink-0 text-[#8C6214]" />
                      <span>{meta.legion}</span>
                    </div>
                  )}
                  <RomanCitizenSvg />
                </div>
              </RomanTooltip>
            </div>
          );
        })}
        {dragBanner && (
          <svg className="pointer-events-none absolute inset-0 h-full w-full">
            <path
              d={`M ${dragBanner.sx} ${dragBanner.sy} Q ${(dragBanner.sx + dragBanner.cx) * 0.5} ${Math.min(dragBanner.sy, dragBanner.cy) - 36} ${dragBanner.cx} ${dragBanner.cy}`}
              fill="none"
              stroke="rgba(255, 248, 210, 0.92)"
              strokeWidth="2.4"
              strokeLinecap="round"
            />
            <circle
              cx={dragBanner.cx}
              cy={dragBanner.cy}
              r="18"
              fill="rgba(251, 191, 36, 0.22)"
              stroke="#D4AF37"
              strokeWidth="2"
            />
          </svg>
        )}
      </div>
      {buildStage && (
        <div className="absolute right-2 top-2 rounded-sm border border-[#D4AF37]/50 bg-bg-elevated/90 px-2 py-1 font-display text-[10px] tracking-widest text-[#F3E5AB]">
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
          className="absolute z-20 rounded-lg border border-[#D4AF37]/50 bg-bg-surface p-1 text-[11px] text-[#F3E5AB] shadow-lg"
          style={{ left: menu.sx + 8, top: menu.sy + 8 }}
        >
          <div className="max-w-[220px] truncate px-2 py-1 font-mono text-[#B8A88A]">{menu.path}</div>
          <button type="button" className="block w-full rounded-sm px-2 py-1 text-left hover:bg-bg-elevated"
            onClick={() => { voxTransport.openLocator({ kind: 'file', value: scanRoot ? `${scanRoot}/${menu.path}` : menu.path }).catch(() => {}); setMenu(null); }}>
            Open file in editor
          </button>
          <button type="button" className="block w-full rounded-sm px-2 py-1 text-left hover:bg-bg-elevated"
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
            className="pointer-events-auto absolute bottom-1.5 right-1.5 z-10 w-[148px] rounded-xl border border-[#C59B27]/85 bg-linear-to-b from-[#FAF6EE]/95 to-[#EFE5D2]/95 p-1.5 shadow-[0_8px_22px_rgba(28,20,12,0.32),inset_0_1px_0_rgba(255,255,255,0.9)]"
          >
            <div className="flex items-center justify-between px-0.5 pb-1">
              <span className="inline-flex items-center gap-1 font-display text-[8px] font-bold uppercase tracking-[0.12em] text-(--color-text-primary)">
                <Icon.astrolabeRadar className="size-2.5 text-[#8C6214]" aria-hidden="true" />
                <span>Radar</span>
              </span>
              <span className="font-sans text-[8px] font-semibold text-[#5C4428]">
                7 Tiers ({layout.districts.length})
              </span>
            </div>
            <div className="relative h-[86px] w-full overflow-hidden rounded-lg border border-[#D4AF37]/45 bg-bg-surface">
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
                className="inline-flex items-center gap-1.5 rounded-lg border-2 border-[#C59B27]/80 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE5D2]/96 px-2.5 py-1 font-display text-[9.5px] font-semibold uppercase tracking-wider text-(--color-text-primary) shadow-[0_6px_18px_rgba(28,20,12,0.3)] hover:border-[#B8860B] transition"
              >
                <div className="relative size-4 shrink-0 overflow-hidden rounded-full border border-[#C59B27]/80 shadow-[0_0_6px_rgba(212,175,55,0.4)]">
                  <img
                    src="/assets/romanesque/wax-seal-medallion.jpg"
                    alt=""
                    aria-hidden="true"
                    className="size-full object-cover"
                  />
                </div>
                <span className="inline-flex items-center gap-1 font-semibold text-(--color-text-primary)">
                  <span>
                    Git Step {selectedTurn}/{viaMilestones[viaMilestones.length - 1]?.turnNumber ?? 3}
                  </span>
                  <Icon.chevronDown className="size-3 shrink-0" aria-hidden="true" />
                </span>
              </button>
            </RomanTooltip>
          </div>
        )
      )}
      {/* Phase 364 / 384: Compact Single-Row Semiotic Layer Navigator & Developer Command Dock */}
      {layout && !embedded && (
        <div
          data-testid="urbs-developer-action-dock"
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(250,246,238,0.95) 0%, rgba(238,229,212,0.95) 100%), url(/assets/romanesque/carrara-seamless-stone.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="vox-marble-tablet pointer-events-auto absolute bottom-[48px] left-2 z-20 flex w-max max-w-[calc(100%-168px)] flex-nowrap items-center justify-between gap-1 overflow-x-auto no-scrollbar rounded-xl border border-[#B38728]/80 px-2 py-0.5 text-[#1E160D] shadow-[0_6px_18px_rgba(28,20,12,0.28),inset_0_1px_0_rgba(255,255,255,0.85)]"
        >
          {/* Semiotic Architectural Layer Filter Pills (L0..L5) */}
          <div data-testid="urbs-semiotic-layer-pills" className="flex shrink-0 items-center gap-0.5">
            <span className="mr-0.5 font-display text-[8px] font-bold uppercase tracking-[0.12em] text-[#5C4318]">
              Tiers:
            </span>
            {[
              { layer: null, label: 'All', title: 'Show all architectural layers' },
              { layer: 0, label: 'L0', title: 'L0 Foundation Vaults (crypto, secrets, config)' },
              { layer: 1, label: 'L1', title: 'L1 Compiler Acropolis (lexer, parser, typeck, codegen)' },
              { layer: 2, label: 'L2', title: 'L2 Database & Runtime Rotunda (vox-db, search, actor-runtime)' },
              { layer: 3, label: 'L3', title: 'L3 Orchestrator Praetorium (multi-agent routing, MCP, DEI)' },
              { layer: 4, label: 'L4', title: 'L4 MENS Thermae (QLoRA training, eval, Crucible)' },
              { layer: 5, label: 'L5', title: 'L5 Surface Colosseum (vox-gui, vox-cli, ludus)' },
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
                        setDevActionReceipt(`Focused ${item.label}: ${targetDist.name}`);
                      }
                    }
                  }}
                  className={`rounded border px-1.5 py-0.5 font-display text-[8px] font-bold uppercase tracking-wider transition ${
                    isAct
                      ? 'border-[#8C6214] bg-linear-to-b from-[#FAF0D7] to-[#E6C762]/65 text-(--color-text-primary) shadow-sm'
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
                    setDevActionReceipt(`Dispatched cargo check -p ${crateName}`);
                    void voxTransport.executeCommand(['check'], { crate: crateName }).catch(() => {});
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  <span>Check Crate</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-run-tests"
                  onClick={() => {
                    setDevActionReceipt(`Running test suite for ${crateName}`);
                    void voxTransport.executeCommand(['test'], { crate: crateName }).catch(() => {});
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  <Icon.check className="size-3 shrink-0" aria-hidden="true" />
                  <span>Run Tests</span>
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
                      setDevActionReceipt(`Opened Speculum Diff Loupe on ${firstPlot.path}`);
                    }
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  <Icon.spark className="size-3 shrink-0" aria-hidden="true" />
                  <span>Diff Loupe</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-pin-context"
                  onClick={() => {
                    const prevSet = new Set(pinnedCrates);
                    const nextSet = new Set(pinnedCrates);
                    if (nextSet.has(crateName)) nextSet.delete(crateName);
                    else nextSet.add(crateName);
                    setPinnedCrates(nextSet);
                    const nextPins = Array.from(nextSet).map((c) => ({
                      id: `crate:${c}`,
                      label: `@crate:${c}`,
                      scope: 'urbs',
                    }));
                    void setSessionContextPins('default', nextPins)
                      .then(() => {
                        setDevActionReceipt(
                          isPinned
                            ? `Unpinned ${crateName} from LLM context`
                            : `Pinned ${crateName} to active LLM context budget`,
                        );
                      })
                      .catch((err: unknown) => {
                        setPinnedCrates(prevSet);
                        setDevActionReceipt(
                          `Pin update failed: ${sanitizeErrorForToast(err)}`,
                        );
                      });
                  }}
                  className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider shadow-sm ${
                    isPinned
                      ? 'border-[#7B8C32] bg-emerald-500/15 text-text-primary'
                      : 'border-[#8C6214]/80 bg-bg-elevated text-text-primary hover:border-[#D4AF37]'
                  }`}
                >
                  <Icon.crystalGem className="size-3 shrink-0" aria-hidden="true" />
                  <span>{isPinned ? 'Pinned' : 'Pin Context'}</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-quarantine"
                  onClick={() => {
                    toggleDistrictQuarantine(crateName);
                    setDevActionReceipt(
                      isQuarantined
                        ? `Lifted quarantine on ${crateName}`
                        : `Quarantined ${crateName} from autonomous edits`,
                    );
                  }}
                  className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider shadow-sm ${
                    isQuarantined
                      ? 'border-amber-600 bg-amber-500/20 text-text-primary'
                      : 'border-[#8C6214]/80 bg-bg-elevated text-text-primary hover:border-[#D4AF37]'
                  }`}
                >
                  {isQuarantined ? (
                    <Icon.lockKeyhole className="size-3 shrink-0" aria-hidden="true" />
                  ) : (
                    <Icon.square className="size-3 shrink-0" aria-hidden="true" />
                  )}
                  <span>{isQuarantined ? 'Quarantined' : 'Quarantine'}</span>
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
                    setDevActionReceipt(`Dispatched LEGIO-01 agent to ${crateName}`);
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#8C6214]/80 bg-bg-elevated px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-text-primary shadow-sm hover:border-[#D4AF37]"
                >
                  <Icon.chevR className="size-3 shrink-0" aria-hidden="true" />
                  <span>Spawn Agent</span>
                </button>
                <button
                  type="button"
                  data-testid="urbs-action-claim-yield"
                  onClick={() => {
                    const prevXp = claimedYieldXp;
                    const nextXp = prevXp + 50;
                    setClaimedYieldXp(nextXp);
                    const t = voxTransport as typeof voxTransport & {
                      recordGuiEvent?: (
                        eventType: string,
                        metadata?: Record<string, unknown>,
                      ) => Promise<{ xpGranted: number; lumensGranted: number }>;
                      setGuiPreference?: (key: string, value: string) => Promise<void>;
                    };
                    const recordPromise =
                      typeof t.recordGuiEvent === 'function'
                        ? t.recordGuiEvent('urbs_yield_claimed', {
                            crate: crateName,
                            xpDelta: 50,
                          })
                        : Promise.resolve({ xpGranted: 50, lumensGranted: 8 });
                    const prefPromise =
                      typeof t.setGuiPreference === 'function'
                        ? t.setGuiPreference('vox.urbs.claimed_yield_xp.v1', String(nextXp))
                        : Promise.resolve();
                    void Promise.all([recordPromise, prefPromise])
                      .then(([res]) => {
                        const xpAwarded = res?.xpGranted ?? 50;
                        const lumensAwarded = res?.lumensGranted ?? 8;
                        setDevActionReceipt(
                          `Claimed +${xpAwarded} XP · +${lumensAwarded} Denarii from ${crateName} verification!`,
                        );
                      })
                      .catch((err: unknown) => {
                        setClaimedYieldXp(prevXp);
                        setDevActionReceipt(
                          `Yield claim failed: ${sanitizeErrorForToast(err)}`,
                        );
                      });
                  }}
                  className="inline-flex items-center gap-1 rounded-md border border-[#B38728] bg-linear-to-b from-[#D4AF37] to-[#9A711C] px-2 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-[#1A1208] shadow-sm hover:brightness-110"
                >
                  <Icon.spark className="size-3 shrink-0" aria-hidden="true" />
                  <span>Yield ({claimedYieldXp} XP)</span>
                </button>
              </div>
            );
          })()}

          {devActionReceipt && (
            <span
              data-testid="urbs-dev-action-receipt"
              className="rounded border border-[#8C6214]/50 bg-bg-elevated px-2 py-0.5 font-mono text-[8.5px] text-text-primary"
            >
              {devActionReceipt}
            </span>
          )}
        </div>
      )}

      <div
        className={`absolute ${
          embedded
            ? 'bottom-1.5 left-1.5 right-1.5 max-w-[calc(100%-12px)] overflow-x-auto no-scrollbar'
            : 'bottom-1.5 left-[142px] max-w-[calc(100%-348px)]'
        } z-10 flex flex-nowrap items-center gap-1`}
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
            className={`pointer-events-auto inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-lg border border-[#C59B27]/85 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE6D5]/96 ${
              embedded ? 'px-1.5 py-0.5 text-[8.5px]' : 'px-2.5 py-1 text-[10px]'
            } font-display font-bold uppercase tracking-wider text-(--color-text-primary) hover:border-[#9A7014] hover:bg-[#FAF6EE]`}
          >
            <Icon.homeVilla className="size-3 shrink-0" aria-hidden="true" />
            <span>{embedded ? 'Fit' : 'Fit Map'}</span>
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
                className="pointer-events-auto rounded-lg border border-[#C59B27]/85 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE6D5]/96 px-2.5 py-1 font-display text-[11px] font-bold text-(--color-text-primary) hover:border-[#9A7014] hover:bg-[#FAF6EE]"
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
                className="pointer-events-auto rounded-lg border border-[#C59B27]/85 bg-linear-to-b from-[#FAF6EE]/96 to-[#EFE6D5]/96 px-2.5 py-1 font-display text-[11px] font-bold text-(--color-text-primary) hover:border-[#9A7014] hover:bg-[#FAF6EE]"
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
