// crates/vox-gui/ui/src/components/gamify/urbs/layout.ts
import {
  classifyDistrictArchetype,
  computeTopologicalLayers,
  deriveCivicTitle,
} from './topology';
import type {
  ArchLayer,
  BuildingPlot,
  District,
  DistrictAttentionSignal,
  Pt,
  RealEstateFocusMode,
  Tier,
  TownCrate,
  TownDependencyEdge,
  TownFile,
  TownLayout,
  TownScan,
} from './types';

/** Deterministic 32-bit string hash (FNV-1a) for stable tie-breaking. */
export function hash32(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

interface Rect { x0: number; y0: number; x1: number; y1: number }

export interface LayoutTownOptions {
  focusMode?: RealEstateFocusMode;
  attentionByDistrict?: Record<string, DistrictAttentionSignal>;
  activeFiles?: Set<string>;
}

/**
 * Computes the real-estate attention multiplier for a module district.
 * - `'proportional'`: always `1.0`
 * - `'attention'`: active/dirty/error/gate districts inflate `2.0x..3.2x`, blast-radius `1.45x`, dormant `0.55x`
 * - `'worksite_only'`: active/dirty/error/gate districts inflate `2.4x..3.2x`, dormant `0.45x`
 */
export function computeAttentionMultiplier(
  signal: DistrictAttentionSignal | undefined,
  focusMode: RealEstateFocusMode = 'attention',
): number {
  if (focusMode === 'proportional') return 1.0;
  if (!signal) {
    return focusMode === 'worksite_only' ? 0.45 : 0.55;
  }
  const agents = signal.activeAgentCount ?? 0;
  const dirty = signal.dirtyFileCount ?? 0;
  const churn = signal.totalChurnLines ?? 0;
  const errors = signal.errorCount ?? 0;
  const warnings = signal.warningCount ?? 0;
  const gates = signal.needsYouGates ?? 0;
  const selected = Boolean(signal.isSelected);

  const hasDirectActivity =
    agents > 0 || dirty > 0 || churn > 0 || errors > 0 || warnings > 0 || gates > 0 || selected;

  if (hasDirectActivity) {
    const base = focusMode === 'worksite_only' ? 2.3 : 2.0;
    const boost =
      Math.min(0.6, agents * 0.35) +
      Math.min(0.45, errors * 0.25 + warnings * 0.05) +
      Math.min(0.4, gates * 0.4) +
      (dirty > 0 || churn > 0 ? 0.3 + Math.min(0.25, churn / 400) : 0) +
      (selected ? 0.25 : 0);
    return Number(Math.min(3.2, base + boost).toFixed(2));
  }

  if (signal.inBlastRadius) {
    return 1.45;
  }

  return focusMode === 'worksite_only' ? 0.45 : 0.55;
}

/** Side of the square interior a module needs at ~50% building density,
 *  scaled by file count, total LoC, and attention multiplier.
 *  interiorSide² ≥ 2·files guarantees the checkerboard pass alone can hold
 *  every file — capacity is a construction invariant, not a hope. */
function interiorSide(
  fileCount: number,
  totalLines = 0,
  attentionMultiplier = 1.0,
  extraTouchedSlots = 0,
): number {
  const locBonusSlots = Math.min(8, Math.floor(totalLines / 6000));
  const effectiveMult =
    attentionMultiplier < 1.0 ? attentionMultiplier : Math.min(1.75, Math.pow(attentionMultiplier, 0.78));
  const rawSlots = (fileCount * 1.85 + locBonusSlots) * effectiveMult + extraTouchedSlots;
  const minCapacitySide = Math.max(2, Math.ceil(Math.sqrt(fileCount * 2)));
  return Math.max(minCapacitySide, Math.ceil(Math.sqrt(Math.max(4, rawSlots))));
}

/** Deterministic shelf-packing: each district is sized from its file count + LoC +
 *  attention multiplier (+2 for the road border), packed into rows of ~square width. */
function packDistricts(
  crates: {
    name: string;
    files: number;
    totalLines: number;
    attentionMultiplier: number;
    extraTouchedSlots: number;
  }[],
): { rects: Map<string, Rect>; side: number } {
  const sized = crates.map((c) => ({
    name: c.name,
    s: interiorSide(c.files, c.totalLines, c.attentionMultiplier, c.extraTouchedSlots) + 2,
  }));
  const totalArea = sized.reduce((a, d) => a + d.s * d.s, 0);
  const targetW = Math.ceil(Math.sqrt(totalArea * 1.1));
  // Height-desc order (name-hash tiebreak) keeps rows tight; deterministic.
  const order = [...sized].sort((a, b) => b.s - a.s || hash32(a.name) - hash32(b.name));
  const rects = new Map<string, Rect>();
  let x = 0; let y = 0; let rowH = 0; let maxX = 0;
  for (const d of order) {
    if (x > 0 && x + d.s > Math.max(targetW, d.s)) { x = 0; y += rowH; rowH = 0; }
    rects.set(d.name, { x0: x, y0: y, x1: x + d.s, y1: y + d.s });
    x += d.s;
    rowH = Math.max(rowH, d.s);
    maxX = Math.max(maxX, x);
  }
  return { rects, side: Math.max(maxX, y + rowH) };
}

function tierFor(lines: number, sorted: number[]): Tier {
  const idx = sorted.findIndex((v) => v >= lines);
  const q = (idx < 0 ? sorted.length - 1 : idx) / Math.max(1, sorted.length - 1);
  if (q < 0.25) return 0;
  if (q < 0.5) return 1;
  if (q < 0.75) return 2;
  return 3;
}

function isFileTouched(file: TownFile, activeFiles?: Set<string>): boolean {
  if (activeFiles?.has(file.path)) return true;
  if (file.git_status && file.git_status.trim().length > 0) return true;
  if ((file.diff_added_lines ?? 0) > 0 || (file.diff_deleted_lines ?? 0) > 0) return true;
  return false;
}

export function layoutTown(
  scan: TownScan,
  godNodes: Set<string>,
  options?: LayoutTownOptions,
): TownLayout {
  const focusMode: RealEstateFocusMode = options?.focusMode ?? 'attention';
  const totalLinesByCrate = new Map<string, number>();
  const resolvedDepsByCrate = new Map<string, string[]>();
  const crateNames = new Set(scan.crates.map((c) => c.name));

  // Pure topological layer + fan-in/fan-out calculation (zero vox-* hardcoding)
  const topoByCrate = computeTopologicalLayers(scan.crates);
  const layerByCrate = new Map<string, ArchLayer>();
  const fanInByCrate = new Map<string, number>();
  const fanOutByCrate = new Map<string, number>();

  for (const c of scan.crates) {
    let lines = 0;
    for (let i = 0; i < c.files.length; i++) lines += c.files[i]!.lines;
    totalLinesByCrate.set(c.name, lines);
    const topo = topoByCrate.get(c.name) ?? {
      layer: 1 as ArchLayer,
      fanIn: 0,
      fanOut: 0,
      centrality: 1,
      rawRank: 0,
    };
    layerByCrate.set(c.name, topo.layer);
    fanInByCrate.set(c.name, topo.fanIn);
    fanOutByCrate.set(c.name, topo.fanOut);
  }

  for (const c of scan.crates) {
    let deps = c.deps ? c.deps.filter((d) => crateNames.has(d) && d !== c.name) : [];
    if (deps.length === 0 && scan.crates.length > 1) {
      const myLayer = layerByCrate.get(c.name) ?? 1;
      const lowerCrates = scan.crates
        .filter((other) => other.name !== c.name && (layerByCrate.get(other.name) ?? 1) < myLayer)
        .map((other) => other.name);
      deps = lowerCrates.slice(0, 2);
      for (const dep of deps) {
        fanInByCrate.set(dep, (fanInByCrate.get(dep) ?? 0) + 1);
      }
    }
    resolvedDepsByCrate.set(c.name, deps);
  }

  // Determine whether ANY district in the scan has live attention signals
  const dirtyByCrate = new Map<string, { dirtyFiles: number; churnLines: number }>();
  let anyWorkspaceActivity = Boolean(
    options?.attentionByDistrict && Object.keys(options.attentionByDistrict).length > 0,
  );
  if (options?.activeFiles && options.activeFiles.size > 0) {
    anyWorkspaceActivity = true;
  }

  for (const c of scan.crates) {
    let dirtyFiles = c.dirty_file_count ?? 0;
    let churnLines = c.total_churn_lines ?? 0;
    if (dirtyFiles === 0 && churnLines === 0) {
      for (const f of c.files) {
        if (isFileTouched(f, options?.activeFiles)) {
          dirtyFiles++;
          churnLines += (f.diff_added_lines ?? 0) + (f.diff_deleted_lines ?? 0);
        }
      }
    }
    if (dirtyFiles > 0 || churnLines > 0) anyWorkspaceActivity = true;
    dirtyByCrate.set(c.name, { dirtyFiles, churnLines });
  }

  const multiplierByCrate = new Map<string, number>();
  for (const c of scan.crates) {
    const dirtyInfo = dirtyByCrate.get(c.name) ?? { dirtyFiles: 0, churnLines: 0 };
    const explicitSignal = options?.attentionByDistrict?.[c.name];
    const mergedSignal: DistrictAttentionSignal | undefined =
      explicitSignal || anyWorkspaceActivity
        ? {
            ...explicitSignal,
            dirtyFileCount: Math.max(explicitSignal?.dirtyFileCount ?? 0, dirtyInfo.dirtyFiles),
            totalChurnLines: Math.max(explicitSignal?.totalChurnLines ?? 0, dirtyInfo.churnLines),
          }
        : undefined;

    const mult =
      focusMode === 'proportional' || !anyWorkspaceActivity
        ? 1.0
        : computeAttentionMultiplier(mergedSignal, focusMode);
    multiplierByCrate.set(c.name, mult);
  }

  // Deterministic order for placement: attention-weighted size desc, then file count, then name-hash.
  const crates = [...scan.crates].sort((a, b) => {
    const multA = multiplierByCrate.get(a.name) ?? 1.0;
    const multB = multiplierByCrate.get(b.name) ?? 1.0;
    if (multA !== multB) return multB - multA;
    return (
      b.files.length - a.files.length ||
      (totalLinesByCrate.get(b.name) ?? 0) - (totalLinesByCrate.get(a.name) ?? 0) ||
      hash32(a.name) - hash32(b.name)
    );
  });

  const { rects, side } = packDistricts(
    crates.map((c) => {
      const dirtyInfo = dirtyByCrate.get(c.name) ?? { dirtyFiles: 0, churnLines: 0 };
      const mult = multiplierByCrate.get(c.name) ?? 1.0;
      const extraTouchedSlots =
        focusMode !== 'proportional' && dirtyInfo.dirtyFiles > 0
          ? Math.min(12, Math.max(4, dirtyInfo.dirtyFiles * 3))
          : 0;
      return {
        name: c.name,
        files: c.files.length,
        totalLines: totalLinesByCrate.get(c.name) ?? 0,
        attentionMultiplier: mult,
        extraTouchedSlots,
      };
    }),
  );

  const districts: District[] = [];
  const byPath: Record<string, BuildingPlot> = {};
  const roads = new Array<boolean>(side * side).fill(true);

  for (const crate of crates) {
    const r = rects.get(crate.name)!;
    for (let y = r.y0 + 1; y < r.y1 - 1; y++) {
      for (let x = r.x0 + 1; x < r.x1 - 1; x++) {
        roads[y * side + x] = false; // interior = buildable, not walkable
      }
    }

    // Place touched/active files first in foreground slots when focusMode !== 'proportional'
    const files = [...crate.files].sort((a, b) => {
      if (focusMode !== 'proportional') {
        const ta = isFileTouched(a, options?.activeFiles) ? 1 : 0;
        const tb = isFileTouched(b, options?.activeFiles) ? 1 : 0;
        if (ta !== tb) return tb - ta;
      }
      return hash32(a.path) - hash32(b.path);
    });

    const touchedCount = files.filter((f) => isFileTouched(f, options?.activeFiles)).length;
    const untouchedCount = files.length - touchedCount;
    // Guarantee touched files receive >= 65% of total building plot footprint area in active districts
    let touchedSpanSide = 1;
    if (focusMode !== 'proportional' && touchedCount > 0) {
      const minTotalTouchedArea = Math.ceil(( Math.max(1, untouchedCount) * 0.66) / 0.34);
      const areaPerTouched = Math.max(4, Math.ceil(minTotalTouchedArea / touchedCount));
      touchedSpanSide = Math.max(2, Math.ceil(Math.sqrt(areaPerTouched)));
    }

    const sortedLines = crate.files.map((f) => f.lines).sort((a, b) => a - b);
    const buildings: BuildingPlot[] = [];
    const dcx = (r.x0 + r.x1) / 2;
    const dcy = (r.y0 + r.y1) / 2;
    const candidateCells: { x: number; y: number; parity: number; outsideWonder: number; depth: number }[] = [];
    for (const parity of [0, 1]) {
      for (let y = r.y0 + 1; y < r.y1 - 1; y++) {
        for (let x = r.x0 + 1; x < r.x1 - 1; x++) {
          if ((x + y) % 2 !== parity) continue;
          const dist = Math.hypot(x - dcx, y - dcy);
          candidateCells.push({
            x,
            y,
            parity,
            outsideWonder: dist >= 2.25 ? 1 : 0,
            depth: x + y,
          });
        }
      }
    }
    candidateCells.sort(
      (a, b) =>
        a.parity - b.parity ||
        b.outsideWonder - a.outsideWonder ||
        b.depth - a.depth ||
        a.y - b.y ||
        a.x - b.x,
    );

    let i = 0;
    for (const cell of candidateCells) {
      if (i >= files.length) break;
      const f = files[i++];
      const touched = isFileTouched(f, options?.activeFiles);
      const plot: BuildingPlot = {
        path: f.path,
        x: cell.x,
        y: cell.y,
        tier: touched && focusMode !== 'proportional' ? 3 : tierFor(f.lines, sortedLines),
        lines: f.lines,
        ...(touched
          ? {
              isTouched: true,
              colSpan: touchedSpanSide,
              rowSpan: touchedSpanSide,
              gitStatus: f.git_status ?? 'M',
              diffAdded: f.diff_added_lines ?? 0,
              diffDeleted: f.diff_deleted_lines ?? 0,
            }
          : {}),
      };
      buildings.push(plot);
      byPath[f.path] = plot;
    }
    if (i < files.length) {
      throw new Error(`layoutTown: district ${crate.name} under-fit (${i}/${files.length})`);
    }
    const totalLines = totalLinesByCrate.get(crate.name) ?? 0;
    const layer = layerByCrate.get(crate.name) ?? 1;
    const fanIn = fanInByCrate.get(crate.name) ?? 0;
    const fanOut = fanOutByCrate.get(crate.name) ?? 0;
    const resolvedDeps = resolvedDepsByCrate.get(crate.name) ?? crate.deps ?? [];
    const archetype = classifyDistrictArchetype(
      crate as TownCrate,
      { layer, fanIn, fanOut, centrality: (fanIn + 1) * (fanOut + 1) },
      scan.crates.length,
    );
    const civicTitle = deriveCivicTitle(crate.name, archetype);
    const attentionMultiplier = multiplierByCrate.get(crate.name) ?? 1.0;
    const dirtyInfo = dirtyByCrate.get(crate.name) ?? { dirtyFiles: 0, churnLines: 0 };

    districts.push({
      name: crate.name,
      x0: r.x0,
      y0: r.y0,
      x1: r.x1,
      y1: r.y1,
      landmark: godNodes.has(crate.name),
      buildings,
      totalLines,
      layer,
      fanIn,
      fanOut,
      archetype,
      civicTitle,
      attentionMultiplier,
      dirtyFileCount: dirtyInfo.dirtyFiles,
      totalChurnLines: dirtyInfo.churnLines,
      isCompactedDormant: attentionMultiplier <= 0.6,
      ...(resolvedDeps.length > 0 ? { deps: resolvedDeps } : {}),
    });
  }

  const landmarks: { castrum: Pt; portus: Pt; aqvae: Pt; gate: Pt } = {
    castrum: { x: Math.max(2, Math.floor(side * 0.82)), y: 1 },
    portus: { x: -1, y: Math.max(2, Math.floor(side * 0.42)) },
    aqvae: { x: Math.max(2, Math.floor(side * 0.34)), y: -1 },
    gate: { x: Math.max(2, Math.floor(side * 0.48)), y: Math.max(2, side - 2) },
  };
  const synthesizedEdges: TownDependencyEdge[] =
    scan.edges && scan.edges.length > 0
      ? scan.edges
      : districts.flatMap((d) =>
          (d.deps ?? []).map((dep) => {
            const target = districts.find((x) => x.name === dep);
            const fromL = d.layer ?? 1;
            const toL = target?.layer ?? 1;
            return {
              from_district_id: d.name,
              to_district_id: dep,
              imported_symbols: [`use ${dep.replace(/-/g, '_')}::*`],
              call_site_count: Math.max(1, (d.fanOut ?? 1) * 2),
              from_layer: fromL,
              to_layer: toL,
              layer_violation: fromL < toL,
              broken_contract_errors: 0,
            };
          }),
        );
  return { districts, byPath, edges: synthesizedEdges, grid: { w: side, h: side }, roads, landmarks };
}

/**
 * Hit-test whether tile coordinates `(tx, ty)` lie along a stone viaduct connecting two districts (`URBS-VIADUCT-02`).
 */
export function hitTestViaductEdge(
  tx: number,
  ty: number,
  districts: readonly District[],
  edges: readonly TownDependencyEdge[] = [],
  tolerance = 0.85,
): TownDependencyEdge | null {
  const byName = new Map(districts.map((d) => [d.name, d]));
  let bestEdge: TownDependencyEdge | null = null;
  let bestDist = tolerance;

  for (const edge of edges) {
    const src = byName.get(edge.from_district_id);
    const dst = byName.get(edge.to_district_id);
    if (!src || !dst) continue;
    const ax = (src.x0 + src.x1) / 2;
    const ay = (src.y0 + src.y1) / 2;
    const bx = (dst.x0 + dst.x1) / 2;
    const by = (dst.y0 + dst.y1) / 2;
    const dx = bx - ax;
    const dy = by - ay;
    const lenSq = dx * dx + dy * dy;
    if (lenSq < 0.01) continue;
    const t = Math.max(0.15, Math.min(0.85, ((tx - ax) * dx + (ty - ay) * dy) / lenSq));
    const px = ax + t * dx;
    const py = ay + t * dy;
    const dist = Math.hypot(tx - px, ty - py);
    if (dist <= bestDist) {
      bestDist = dist;
      bestEdge = edge;
    }
  }
  return bestEdge;
}

/** Mid-session file creation: nearest free interior tile in its module district across polyglot layouts. */
export function assignNewFile(layout: TownLayout, path: string): BuildingPlot | null {
  const m = path.match(
    /^(?:crates|packages|apps|services|libs|modules|plugins|cmd|internal|pkg|src)\/([^/]+)\//,
  );
  const name = m ? m[1] : '(workspace)';
  const d = layout.districts.find((x) => x.name === name);
  if (!d) return null;
  const occupied = new Set(d.buildings.map((b) => `${b.x},${b.y}`));
  for (let y = d.y0 + 1; y < d.y1 - 1; y++) {
    for (let x = d.x0 + 1; x < d.x1 - 1; x++) {
      if (!occupied.has(`${x},${y}`)) {
        const plot: BuildingPlot = { path, x, y, tier: 0 };
        d.buildings.push(plot);
        layout.byPath[path] = plot;
        return plot;
      }
    }
  }
  return null;
}
