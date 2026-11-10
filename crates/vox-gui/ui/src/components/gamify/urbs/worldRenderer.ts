// crates/vox-gui/ui/src/components/gamify/urbs/worldRenderer.ts
/** Offscreen world buffer: repainted only when `redrawKey` changes (data or
 *  LOD band), never on camera moves — the onscreen blit applies the camera.
 *
 *  True 2:1 Isometric 3D Parametric & Shaded Architecture Engine:
 *  Every building, temple, rotunda, colosseum, curtain wall, gatehouse,
 *  multi-arch viaduct, sculpted rock cliff, timber scaffolding, and treadwheel
 *  crane is projected through exact isometric basis vectors:
 *    P(u, v, z) = (px + (u - v) * 32, py + (u + v) * 16 - z)
 *  using a weathered twilight travertine, slate, and terracotta palette with
 *  directional lighting, ashlar masonry courses, tile ribs, and ambient occlusion.
 */
import type { BuildingState, AgentTask } from '../store';
import type {
  District,
  DistrictArchetype,
  Tier,
  TownDependencyEdge,
  TownLayout,
  UrbsLens,
} from './types';
import type { CollisionZone } from './collisionRadar';
import {
  classifyDistrictArchetype,
  computeBlastRadius,
  deriveCivicTitle,
} from './topology';
import { bandForZoom, worldPx, worldBounds } from './lod';
import { buildAtlas, type AtlasPlan, type SpriteKey } from './sprites';
import {
  createBronzeGradient,
  createRoofSlopeGradient,
  createVerticalWallGradient,
  ensurePbrMaterialAtlas,
  tintHex,
} from './pbrMaterials';
import {
  drawAdditiveLightPools,
  drawCinematicPostProcess,
  drawElevationAtmosphericMist,
  drawForgeAndFireEmbers,
  drawMasonChiselSparks,
  drawProjectedIsoCastShadow,
  drawVolumetricGodRays,
  drawVolumetricSmokePlumes,
  drawWaterCausticsAndSpray,
} from './lightingAndFx';
import {
  drawScreenSpaceHydraulicOverlay,
} from './hydraulicNetwork';

export interface HarnessSnapshot {
  /** null = tap unavailable → landmark renders unlit. */
  ci: { runners: { name: string; busy: boolean }[]; queued: number } | null;
  vcs: { branches: { name: string; track: string; isHead: boolean }[]; prs: { number: number; title: string }[] } | null;
  queueLen: number | null;
  /** Always null until a dedicated MCP server-list command exists (spec §7.1). */
  mcp: { name: string; ok: boolean }[] | null;
  /** Live Git checkpoints (`refs/vox/checkpoints/`) for the Roman Milestone Road (`I → II → III`). */
  checkpoints?: {
    ordinal?: number;
    id?: string;
    turnId?: string;
    note?: string;
    createdAtMs?: number;
    refName?: string;
    commitOid?: string;
  }[] | null;
  /** Live human-in-the-loop approval / doubt gates (`Needs You` Iron Portcullis). */
  attentionGates?: { id: string; title: string; source?: string; district?: string }[] | null;
  /** Live prompt context fill percentage (`0..100`) for the Bronze Clepsydra vessel. */
  contextFillPct?: number | null;
}

export interface WorldState {
  layout: TownLayout;
  buildings: Record<string, BuildingState>;
  agentTasks: Record<string, AgentTask>;
  harness: HarnessSnapshot;
  lens?: UrbsLens;
  selectedDistrict?: string | null;
  needsYouByDistrict?: Record<string, number>;
  /** Districts locked read-only by developer Clausura Ferrea (`URBS-QUARANTINE-03`). */
  quarantinedDistricts?: readonly string[];
  /** Active multi-agent write-set & blast-radius collision zones (`URBS-COLLISION-02`). */
  collisionZones?: readonly CollisionZone[];
  /** Currently selected viaduct contract edge (`URBS-VIADUCT-02`). */
  selectedViaductEdge?: TownDependencyEdge | null;
  /** Active tool-call mechanism pulses from live chat (`URBS-DOCK-03`). */
  activeToolPulses?: Record<string, 'read' | 'write' | 'test' | 'gate'>;
}

/** Cheap structural key controlling buffer repaints. Camera and animation
 *  frames are deliberately NOT inputs — pan/zoom and fire flicker must never
 *  trigger a whole-world repaint (fires draw in the blit pass instead). */
export function redrawKey(s: WorldState, band: 0 | 1): string {
  const diag = Object.entries(s.buildings)
    .map(([p, b]) => `${p}:${b.warnings}/${b.errors}`).sort().join('|');
  const tasks = Object.values(s.agentTasks).map((t) => t.filePath).sort().join('|');
  const gates = Object.entries(s.needsYouByDistrict ?? {})
    .map(([k, v]) => `${k}:${v}`).sort().join('|');
  const quar = [...(s.quarantinedDistricts ?? [])].sort().join('|');
  const h = s.harness;
  const hk = [
    h.ci ? h.ci.runners.map((r) => +r.busy).join('') : 'x',
    h.vcs ? `${h.vcs.branches.length}/${h.vcs.prs.length}` : 'x',
    h.queueLen ?? 'x',
    h.mcp ? h.mcp.map((m) => +m.ok).join('') : 'x',
    h.checkpoints ? h.checkpoints.length : 'x',
    h.contextFillPct ?? 'x',
  ].join(',');
  const lens = s.lens ?? 'agents';
  return `${band}#${lens}#${s.layout.grid.w}x${s.layout.grid.h}#${diag}#${tasks}#${gates}#${quar}#${hk}`;
}

const MAX_BUFFER_EDGE = 8192;

interface Buffer { canvas: HTMLCanvasElement; key: string; scale: number }

export function romanDistrictLabel(dOrName: District | string): string {
  if (typeof dOrName === 'string') {
    const arch = classifyDistrictArchetype(
      { name: dOrName, root: dOrName, files: [] },
      { layer: 1, fanIn: 1, fanOut: 1, centrality: 4 },
    );
    return deriveCivicTitle(dOrName, arch);
  }
  if (dOrName.civicTitle) return dOrName.civicTitle;
  return deriveCivicTitle(dOrName.name, dOrName.archetype ?? 'insula_forum');
}

export function districtWonderKey(
  crateOrArchetype: string | DistrictArchetype,
  isLandmark: boolean,
): SpriteKey {
  const arch: DistrictArchetype =
    crateOrArchetype === 'acropolis_engine' ||
    crateOrArchetype === 'praetorium_hub' ||
    crateOrArchetype === 'harbor_vault' ||
    crateOrArchetype === 'amphitheatre_surface' ||
    crateOrArchetype === 'library_scholar' ||
    crateOrArchetype === 'circus_arena' ||
    crateOrArchetype === 'watchtower_leaf' ||
    crateOrArchetype === 'insula_forum'
      ? crateOrArchetype
      : classifyDistrictArchetype(
          { name: crateOrArchetype, root: crateOrArchetype, files: [] },
          { layer: 1, fanIn: 1, fanOut: 1, centrality: 4 },
        );

  if (arch === 'acropolis_engine') return 'landmark';
  if (arch === 'amphitheatre_surface') return 'colosseum';
  if (arch === 'praetorium_hub') return 'praetorium';
  if (arch === 'library_scholar') return 'rotunda';
  return isLandmark ? 'landmark' : 'temple';
}

// ── True 2:1 Isometric 3D Math & Painterly Primitives ──────────────────────

function isoPt(px: number, py: number, u: number, v: number, z = 0): [number, number] {
  return [px + (u - v) * 32, py + (u + v) * 16 - z];
}

function fillPoly(
  ctx: CanvasRenderingContext2D,
  pts: [number, number][],
  fill: string | CanvasGradient | CanvasPattern,
  stroke: string | CanvasGradient = '#161310',
  lineWidth = 1.0,
) {
  if (pts.length === 0) return;
  ctx.beginPath();
  ctx.moveTo(pts[0]![0], pts[0]![1]);
  for (let i = 1; i < pts.length; i++) {
    ctx.lineTo(pts[i]![0], pts[i]![1]);
  }
  ctx.closePath();
  ctx.fillStyle = fill;
  ctx.fill();
  if (stroke && lineWidth > 0) {
    ctx.strokeStyle = stroke;
    ctx.lineWidth = lineWidth;
    ctx.stroke();
  }
}

function strokeLine(
  ctx: CanvasRenderingContext2D,
  a: [number, number],
  b: [number, number],
  color: string,
  width = 1,
) {
  ctx.strokeStyle = color;
  ctx.lineWidth = width;
  ctx.beginPath();
  ctx.moveTo(a[0], a[1]);
  ctx.lineTo(b[0], b[1]);
  ctx.stroke();
}

export function districtElevationPx(d: District): number {
  return (d.layer ?? 1) * 18;
}

interface IsoBoxStyle {
  topFill?: string;
  /** Front-left face (+v face, facing SW — shadowed in twilight) */
  swFill?: string;
  /** Front-right face (+u face, facing SE — warmly lit in twilight) */
  seFill?: string;
  stroke?: string;
  lineWidth?: number;
  courses?: number;
  aoShadow?: boolean;
  windowsSW?: number;
  windowsSE?: number;
  litWindows?: boolean;
  cyanOutline?: boolean;
  /** Phase 291 (`URBS-ARCH-DETAIL-01`): Ground-floor rusticated arched loggias (`Fornices`) */
  groundArchesSW?: number;
  groundArchesSE?: number;
  /** Phase 291 (`URBS-ARCH-DETAIL-02`): Projecting inter-story stringcourse belt cornice (`Marcapiano`) */
  beltCourse?: boolean;
}

/** Extrudes a 3D isometric box from (u0, v0, z0) to (u1, v1, z1) with
 *  projected directional twilight cast shadows, vertical height-shaded gradients,
 *  pitted Roman travertine ashlar texture, and physical specular chamfer bevels. */
function drawIsoBox(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  u0: number,
  v0: number,
  z0: number,
  u1: number,
  v1: number,
  z1: number,
  style: IsoBoxStyle = {},
) {
  const topFill = style.topFill ?? '#e8dfd1';
  const swFill = style.swFill ?? '#8c7f70';
  const seFill = style.seFill ?? '#f5efe4';
  const stroke = style.cyanOutline ? '#fbbf24' : (style.stroke ?? 'rgba(28, 22, 18, 0.08)');
  const lw = style.cyanOutline ? 1.2 : (style.lineWidth ?? 1.1);
  const pbr = ensurePbrMaterialAtlas(ctx);

  // 1. Projected directional twilight cast shadow & contact ambient occlusion at base (z0)
  if (style.aoShadow !== false && z0 === 0) {
    drawProjectedIsoCastShadow(ctx, isoPt, px, py, u0, v0, z0, u1, v1, z1);
    const pad = 0.07;
    fillPoly(
      ctx,
      [
        isoPt(px, py, u0 - pad, v0 - pad, 0),
        isoPt(px, py, u1 + pad, v0 - pad, 0),
        isoPt(px, py, u1 + pad + 0.05, v1 + pad + 0.05, 0),
        isoPt(px, py, u0 - pad, v1 + pad + 0.05, 0),
      ],
      'rgba(6, 5, 4, 0.56)',
      '',
      0,
    );
  }

  // 2. Front-left face (+v = v1, u from u0 to u1) — SW twilight-shadowed face
  const swPts: [number, number][] = [
    isoPt(px, py, u0, v1, z1),
    isoPt(px, py, u1, v1, z1),
    isoPt(px, py, u1, v1, z0),
    isoPt(px, py, u0, v1, z0),
  ];
  const swTopY = Math.min(swPts[0]![1], swPts[1]![1]);
  const swBotY = Math.max(swPts[2]![1], swPts[3]![1]);
  const swGrad = createVerticalWallGradient(ctx, swTopY, swBotY, swFill, false);
  fillPoly(ctx, swPts, swGrad, stroke, lw);
  if (pbr.travertineShade && z1 - z0 >= 6) {
    ctx.save();
    ctx.globalAlpha = 0.04;
    fillPoly(ctx, swPts, pbr.travertineShade, '', 0);
    ctx.restore();
  }

  // 3. Front-right face (+u = u1, v from v0 to v1) — SE golden-hour sunlit face
  const sePts: [number, number][] = [
    isoPt(px, py, u1, v1, z1),
    isoPt(px, py, u1, v0, z1),
    isoPt(px, py, u1, v0, z0),
    isoPt(px, py, u1, v1, z0),
  ];
  const seTopY = Math.min(sePts[0]![1], sePts[1]![1]);
  const seBotY = Math.max(sePts[2]![1], sePts[3]![1]);
  const seGrad = createVerticalWallGradient(ctx, seTopY, seBotY, seFill, true);
  fillPoly(ctx, sePts, seGrad, stroke, lw);
  if (pbr.travertineLit && z1 - z0 >= 6) {
    ctx.save();
    ctx.globalAlpha = 0.04;
    fillPoly(ctx, sePts, pbr.travertineLit, '', 0);
    ctx.restore();
  }

  // CV-MAX-02: default ashlar course strokes disabled — LF wall gradients carry masonry read
  const courses = style.courses ?? 0;
  if (courses > 1) {
    for (let c = 1; c < courses; c++) {
      const zc = z0 + ((z1 - z0) * c) / courses;
      strokeLine(ctx, isoPt(px, py, u0, v1, zc), isoPt(px, py, u1, v1, zc), 'rgba(18, 14, 11, 0.10)', 1.6);
      strokeLine(ctx, isoPt(px, py, u1, v1, zc), isoPt(px, py, u1, v0, zc), 'rgba(38, 30, 23, 0.09)', 1.6);
    }
  }

  // Isometric recessed windows on SW face (`URBS-ARCH-REALISM-02`)
  if (style.windowsSW && style.windowsSW > 0) {
    const n = style.windowsSW;
    const zMid = z0 + (z1 - z0) * 0.56;
    const wH = Math.min(7, (z1 - z0) * 0.32);
    for (let i = 0; i < n; i++) {
      const ua = u0 + ((u1 - u0) * (i + 0.28)) / n;
      const ub = u0 + ((u1 - u0) * (i + 0.72)) / n;
      const isLit = Boolean(style.litWindows && i % 2 === 0);
      const winColor = isLit ? '#f59e0b' : '#14110e';
      const sillA = isoPt(px, py, ua, v1, zMid - wH / 2);
      const sillB = isoPt(px, py, ub, v1, zMid - wH / 2);
      fillPoly(
        ctx,
        [
          isoPt(px, py, ua, v1, zMid + wH / 2),
          isoPt(px, py, ub, v1, zMid + wH / 2),
          sillB,
          sillA,
        ],
        winColor,
        'rgba(12, 10, 8, 0.55)',
        0.55,
      );
      // Recessed stone window sill highlight
      strokeLine(ctx, sillA, sillB, 'rgba(214, 200, 182, 0.42)', 0.75);
      if (isLit) {
        const wCenter = isoPt(px, py, (ua + ub) / 2, v1, zMid);
        if (typeof ctx.createRadialGradient === 'function') {
          const glow = ctx.createRadialGradient(wCenter[0], wCenter[1], 0.4, wCenter[0], wCenter[1], 7.5);
          glow.addColorStop(0, 'rgba(254, 243, 199, 0.72)');
          glow.addColorStop(0.45, 'rgba(245, 158, 11, 0.34)');
          glow.addColorStop(1, 'rgba(245, 158, 11, 0)');
          ctx.fillStyle = glow;
          ctx.beginPath();
          ctx.arc(wCenter[0], wCenter[1], 7.5, 0, Math.PI * 2);
          ctx.fill();
        }
        strokeLine(ctx, [wCenter[0] - 2, wCenter[1] + 2], [wCenter[0] + 2, wCenter[1] + 2], '#fef08a', 0.9);
      }
    }
  }

  // Isometric recessed windows on SE face (`URBS-ARCH-REALISM-02`)
  if (style.windowsSE && style.windowsSE > 0) {
    const n = style.windowsSE;
    const zMid = z0 + (z1 - z0) * 0.56;
    const wH = Math.min(7, (z1 - z0) * 0.32);
    for (let i = 0; i < n; i++) {
      const va = v0 + ((v1 - v0) * (i + 0.28)) / n;
      const vb = v0 + ((v1 - v0) * (i + 0.72)) / n;
      const isLit = Boolean(style.litWindows && i % 2 === 1);
      const winColor = isLit ? '#fbbf24' : '#181310';
      const sillA = isoPt(px, py, u1, va, zMid - wH / 2);
      const sillB = isoPt(px, py, u1, vb, zMid - wH / 2);
      fillPoly(
        ctx,
        [
          isoPt(px, py, u1, va, zMid + wH / 2),
          isoPt(px, py, u1, vb, zMid + wH / 2),
          sillB,
          sillA,
        ],
        winColor,
        'rgba(12, 10, 8, 0.55)',
        0.55,
      );
      strokeLine(ctx, sillA, sillB, 'rgba(245, 235, 214, 0.48)', 0.8);
      if (isLit) {
        const wCenter = isoPt(px, py, u1, (va + vb) / 2, zMid);
        if (typeof ctx.createRadialGradient === 'function') {
          const glow = ctx.createRadialGradient(wCenter[0], wCenter[1], 0.4, wCenter[0], wCenter[1], 7.5);
          glow.addColorStop(0, 'rgba(254, 249, 195, 0.70)');
          glow.addColorStop(0.45, 'rgba(251, 191, 36, 0.32)');
          glow.addColorStop(1, 'rgba(251, 191, 36, 0)');
          ctx.fillStyle = glow;
          ctx.beginPath();
          ctx.arc(wCenter[0], wCenter[1], 7.5, 0, Math.PI * 2);
          ctx.fill();
        }
        strokeLine(ctx, [wCenter[0] - 2, wCenter[1] + 2], [wCenter[0] + 2, wCenter[1] + 2], '#fef9c3', 0.9);
      }
    }
  }

  // Phase 291 (`URBS-ARCH-DETAIL-01`): Ground-Floor Rusticated Stone Arcades (`Fornices`) & Keystone Voussoirs
  const archH = Math.min(8.5, (z1 - z0) * 0.42);
  if (style.groundArchesSW && style.groundArchesSW > 0 && z1 - z0 >= 9) {
    const n = style.groundArchesSW;
    for (let i = 0; i < n; i++) {
      const ua = u0 + ((u1 - u0) * (i + 0.2)) / n;
      const ub = u0 + ((u1 - u0) * (i + 0.8)) / n;
      const uMid = (ua + ub) / 2;
      fillPoly(
        ctx,
        [
          isoPt(px, py, ua, v1, z0 + archH * 0.65),
          isoPt(px, py, uMid, v1, z0 + archH),
          isoPt(px, py, ub, v1, z0 + archH * 0.65),
          isoPt(px, py, ub, v1, z0),
          isoPt(px, py, ua, v1, z0),
        ],
        i % 2 === 0 && style.litWindows ? '#78350f' : '#14100d',
        '#292018',
        0.75,
      );
      // Chiseled arch keystone
      const ks = isoPt(px, py, uMid, v1, z0 + archH + 0.8);
      ctx.fillStyle = '#d6cfc7';
      ctx.fillRect(ks[0] - 1.2, ks[1] - 1.2, 2.4, 2.2);
    }
  }
  if (style.groundArchesSE && style.groundArchesSE > 0 && z1 - z0 >= 9) {
    const n = style.groundArchesSE;
    for (let i = 0; i < n; i++) {
      const va = v0 + ((v1 - v0) * (i + 0.2)) / n;
      const vb = v0 + ((v1 - v0) * (i + 0.8)) / n;
      const vMid = (va + vb) / 2;
      fillPoly(
        ctx,
        [
          isoPt(px, py, u1, va, z0 + archH * 0.65),
          isoPt(px, py, u1, vMid, z0 + archH),
          isoPt(px, py, u1, vb, z0 + archH * 0.65),
          isoPt(px, py, u1, vb, z0),
          isoPt(px, py, u1, va, z0),
        ],
        i % 2 === 0 && style.litWindows ? '#b45309' : '#1c1512',
        '#292018',
        0.75,
      );
      const ks = isoPt(px, py, u1, vMid, z0 + archH + 0.8);
      ctx.fillStyle = '#e7dfd5';
      ctx.fillRect(ks[0] - 1.2, ks[1] - 1.2, 2.4, 2.2);
    }
  }

  // Phase 291 (`URBS-ARCH-DETAIL-02`): Projecting Inter-Story Stringcourse Belt Cornice (`Marcapiano`)
  if (style.beltCourse && z1 - z0 >= 12) {
    const zBelt = z0 + (z1 - z0) * 0.44;
    strokeLine(ctx, isoPt(px, py, u0 - 0.02, v1, zBelt), isoPt(px, py, u1 + 0.02, v1, zBelt), 'rgba(120, 110, 99, 0.35)', 2.2);
    strokeLine(ctx, isoPt(px, py, u1, v1 + 0.02, zBelt), isoPt(px, py, u1, v0 - 0.02, zBelt), '#d6cfc7', 1.7);
    strokeLine(ctx, isoPt(px, py, u1, v1 + 0.02, zBelt - 1), isoPt(px, py, u1, v0 - 0.02, zBelt - 1), 'rgba(18, 14, 11, 0.65)', 0.9);
  }

  // 4. Top face (z = z1) with warm sunlit gradient & travertine grain
  const topPts: [number, number][] = [
    isoPt(px, py, u0, v0, z1),
    isoPt(px, py, u1, v0, z1),
    isoPt(px, py, u1, v1, z1),
    isoPt(px, py, u0, v1, z1),
  ];
  const topGrad = createVerticalWallGradient(ctx, topPts[0]![1], topPts[2]![1], topFill, true);
  fillPoly(ctx, topPts, topGrad, stroke, lw);
  if (pbr.travertineLit && u1 - u0 >= 0.4) {
    ctx.save();
    ctx.globalAlpha = 0.18;
    fillPoly(ctx, topPts, pbr.travertineLit, '', 0);
    ctx.restore();
  }

  // 5. Physical specular chamfer ridge highlights along sunlit SE top & vertical corner edges
  if (!style.cyanOutline && z1 - z0 >= 3) {
    strokeLine(ctx, isoPt(px, py, u1, v1, z1), isoPt(px, py, u1, v0, z1), 'rgba(254, 243, 199, 0.52)', 1.1);
    strokeLine(ctx, isoPt(px, py, u0, v1, z1), isoPt(px, py, u1, v1, z1), 'rgba(253, 230, 138, 0.28)', 0.9);
    strokeLine(ctx, isoPt(px, py, u1, v1, z1), isoPt(px, py, u1, v1, z0), 'rgba(254, 243, 199, 0.36)', 0.95);
  }
}

/** Draws a true 2:1 isometric pitched Roman terracotta roof over (u0..u1, v0..v1)
 *  with under-eave drop shadows, scalloped terracotta imbrex tiles, and a sculpted pediment. */
function drawIsoPitchedRoof(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  u0: number,
  v0: number,
  u1: number,
  v1: number,
  zBase: number,
  zPeak: number,
  ridgeAlong: 'u' | 'v' = 'u',
  opts: { roofLit?: string; roofShade?: string; pedimentFill?: string; cyanOutline?: boolean } = {},
) {
  const eave = 0.065;
  const ua = u0 - eave;
  const ub = u1 + eave;
  const va = v0 - eave;
  const vb = v1 + eave;
  const roofLit = opts.roofLit ?? '#c9563d';
  const roofShade = opts.roofShade ?? '#6e2a1d';
  const pedFill = opts.pedimentFill ?? '#dfd3c3';
  const stroke = opts.cyanOutline ? '#22d3ee' : 'rgba(32, 16, 11, 0.36)';
  const lw = opts.cyanOutline ? 1.4 : 0.65;
  const pbr = ensurePbrMaterialAtlas(ctx);

  // Under-eave overhang shadow cast onto the top of the building wall (zBase - 1.5)
  strokeLine(ctx, isoPt(px, py, u0, v1, zBase - 1.2), isoPt(px, py, u1, v1, zBase - 1.2), 'rgba(6, 5, 4, 0.62)', 2.4);
  strokeLine(ctx, isoPt(px, py, u1, v1, zBase - 1.2), isoPt(px, py, u1, v0, zBase - 1.2), 'rgba(8, 6, 5, 0.52)', 2.2);

  if (ridgeAlong === 'u') {
    const vMid = (va + vb) / 2;
    const backPts: [number, number][] = [
      isoPt(px, py, ua, va, zBase),
      isoPt(px, py, ub, va, zBase),
      isoPt(px, py, ub, vMid, zPeak),
      isoPt(px, py, ua, vMid, zPeak),
    ];
    fillPoly(
      ctx,
      backPts,
      createRoofSlopeGradient(ctx, backPts[2]![1], backPts[0]![1], roofShade, false),
      stroke,
      lw,
    );
    if (pbr.terracottaShade) {
      ctx.save();
      ctx.globalAlpha = 0.32;
      fillPoly(ctx, backPts, pbr.terracottaShade, '', 0);
      ctx.restore();
    }

    // Front-left slope (vMid .. vb) — visible slope
    const frontPts: [number, number][] = [
      isoPt(px, py, ua, vMid, zPeak),
      isoPt(px, py, ub, vMid, zPeak),
      isoPt(px, py, ub, vb, zBase),
      isoPt(px, py, ua, vb, zBase),
    ];
    fillPoly(
      ctx,
      frontPts,
      createRoofSlopeGradient(ctx, frontPts[0]![1], frontPts[2]![1], roofLit, true),
      stroke,
      lw,
    );
    if (pbr.terracottaLit) {
      ctx.save();
      ctx.globalAlpha = 0.36;
      fillPoly(ctx, frontPts, pbr.terracottaLit, '', 0);
      ctx.restore();
    }

    // Terracotta imbrex tile ribs + sunlit ridge cap
    const ribs = Math.max(3, Math.round((ub - ua) * 9));
    for (let r = 1; r < ribs; r++) {
      const ur = ua + ((ub - ua) * r) / ribs;
      strokeLine(
        ctx,
        isoPt(px, py, ur, vMid, zPeak),
        isoPt(px, py, ur, vb, zBase),
        'rgba(32, 11, 7, 0.46)',
        0.75,
      );
      strokeLine(
        ctx,
        isoPt(px, py, ur + 0.015, vMid, zPeak),
        isoPt(px, py, ur + 0.015, vb, zBase),
        'rgba(254, 215, 170, 0.24)',
        0.55,
      );
    }
    strokeLine(ctx, isoPt(px, py, ua, vMid, zPeak), isoPt(px, py, ub, vMid, zPeak), 'rgba(254, 215, 170, 0.62)', 1.25);

    // Front-right triangular pediment (+u = ub)
    fillPoly(
      ctx,
      [
        isoPt(px, py, ub, va, zBase),
        isoPt(px, py, ub, vb, zBase),
        isoPt(px, py, ub, vMid, zPeak),
      ],
      pedFill,
      stroke,
      lw,
    );
    // Inner recessed tympanum triangle + bronze relief boss (`URBS-ARCH-REALISM-03`)
    fillPoly(
      ctx,
      [
        isoPt(px, py, ub, va + (vb - va) * 0.16, zBase + 1.5),
        isoPt(px, py, ub, vb - (vb - va) * 0.16, zBase + 1.5),
        isoPt(px, py, ub, vMid, zPeak - 2.2),
      ],
      '#574e46',
      stroke,
      0.65,
    );
    const boss = isoPt(px, py, ub, vMid, zBase + (zPeak - zBase) * 0.42);
    ctx.fillStyle = '#f59e0b';
    ctx.beginPath();
    ctx.arc(boss[0], boss[1], 1.6, 0, Math.PI * 2);
    ctx.fill();
  } else {
    const uMid = (ua + ub) / 2;
    const backPts: [number, number][] = [
      isoPt(px, py, ua, va, zBase),
      isoPt(px, py, uMid, va, zPeak),
      isoPt(px, py, uMid, vb, zPeak),
      isoPt(px, py, ua, vb, zBase),
    ];
    fillPoly(
      ctx,
      backPts,
      createRoofSlopeGradient(ctx, backPts[1]![1], backPts[3]![1], roofShade, false),
      stroke,
      lw,
    );
    if (pbr.terracottaShade) {
      ctx.save();
      ctx.globalAlpha = 0.32;
      fillPoly(ctx, backPts, pbr.terracottaShade, '', 0);
      ctx.restore();
    }

    // Front-right slope (uMid .. ub) — sunlit slope
    const frontPts: [number, number][] = [
      isoPt(px, py, uMid, va, zPeak),
      isoPt(px, py, ub, va, zBase),
      isoPt(px, py, ub, vb, zBase),
      isoPt(px, py, uMid, vb, zPeak),
    ];
    fillPoly(
      ctx,
      frontPts,
      createRoofSlopeGradient(ctx, frontPts[0]![1], frontPts[2]![1], roofLit, true),
      stroke,
      lw,
    );
    if (pbr.terracottaLit) {
      ctx.save();
      ctx.globalAlpha = 0.36;
      fillPoly(ctx, frontPts, pbr.terracottaLit, '', 0);
      ctx.restore();
    }

    const ribs = Math.max(3, Math.round((vb - va) * 9));
    for (let r = 1; r < ribs; r++) {
      const vr = va + ((vb - va) * r) / ribs;
      strokeLine(
        ctx,
        isoPt(px, py, uMid, vr, zPeak),
        isoPt(px, py, ub, vr, zBase),
        'rgba(32, 11, 7, 0.46)',
        0.75,
      );
      strokeLine(
        ctx,
        isoPt(px, py, uMid, vr + 0.015, zPeak),
        isoPt(px, py, ub, vr + 0.015, zBase),
        'rgba(254, 215, 170, 0.24)',
        0.55,
      );
    }
    strokeLine(ctx, isoPt(px, py, uMid, va, zPeak), isoPt(px, py, uMid, vb, zPeak), 'rgba(254, 215, 170, 0.62)', 1.25);

    // Front-left triangular pediment (+v = vb)
    fillPoly(
      ctx,
      [
        isoPt(px, py, ua, vb, zBase),
        isoPt(px, py, ub, vb, zBase),
        isoPt(px, py, uMid, vb, zPeak),
      ],
      '#635a50',
      stroke,
      lw,
    );
    fillPoly(
      ctx,
      [
        isoPt(px, py, ua + (ub - ua) * 0.16, vb, zBase + 1.5),
        isoPt(px, py, ub - (ub - ua) * 0.16, vb, zBase + 1.5),
        isoPt(px, py, uMid, vb, zPeak - 2.2),
      ],
      '#443d36',
      stroke,
      0.65,
    );
  }

  // Phase 291 (`URBS-ARCH-DETAIL-04`): Sculpted Gable `Acroteria` Finial Ornaments & Hypocaust Brick Chimney
  const peakFront =
    ridgeAlong === 'u'
      ? isoPt(px, py, ub, (va + vb) / 2, zPeak)
      : isoPt(px, py, (ua + ub) / 2, vb, zPeak);
  ctx.fillStyle = opts.cyanOutline ? '#67e8f9' : '#f59e0b';
  ctx.strokeStyle = '#29180d';
  ctx.lineWidth = 0.7;
  ctx.beginPath();
  ctx.arc(peakFront[0], peakFront[1] - 2.2, 2.0, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  if (ub - ua >= 0.68) {
    const chimPt = isoPt(px, py, ua + (ub - ua) * 0.28, va + (vb - va) * 0.32, zPeak - 1.5);
    ctx.fillStyle = '#7c2d12';
    ctx.strokeStyle = '#1c130d';
    ctx.lineWidth = 0.75;
    ctx.fillRect(chimPt[0] - 1.8, chimPt[1] - 5.5, 3.6, 5.5);
    ctx.strokeRect(chimPt[0] - 1.8, chimPt[1] - 5.5, 3.6, 5.5);
    ctx.fillStyle = '#d6cfc7';
    ctx.fillRect(chimPt[0] - 2.2, chimPt[1] - 6.5, 4.4, 1.2);
  }
}

/** Draws a row of 3D isometric fluted columns between (u0, v0) and (u1, v1)
 *  from z0 to z1. Supports Concept Art B's luminous cyan wireframe mode. */
function drawIsoColonnade(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  u0: number,
  v0: number,
  u1: number,
  v1: number,
  z0: number,
  z1: number,
  count: number,
  glowingCyan = false,
) {
  for (let i = 0; i < count; i++) {
    const t = count === 1 ? 0.5 : i / (count - 1);
    const u = u0 + (u1 - u0) * t;
    const v = v0 + (v1 - v0) * t;
    const base = isoPt(px, py, u, v, z0);
    const top = isoPt(px, py, u, v, z1);

    if (glowingCyan) {
      // Luminous cyan halo on portico floor and column shaft (Concept Art B)
      ctx.fillStyle = 'rgba(34, 211, 238, 0.28)';
      ctx.beginPath();
      ctx.ellipse(base[0], base[1], 7, 3.5, 0, 0, Math.PI * 2);
      ctx.fill();
    }

    // Column plinth
    ctx.fillStyle = glowingCyan ? '#67e8f9' : '#9c9286';
    ctx.strokeStyle = glowingCyan ? '#06b6d4' : '#171310';
    ctx.lineWidth = 0.8;
    ctx.fillRect(base[0] - 2.6, base[1] - 2.2, 5.2, 2.5);
    ctx.strokeRect(base[0] - 2.6, base[1] - 2.2, 5.2, 2.5);

    // Column shaft (shaded left/right halves)
    const h = base[1] - top[1];
    ctx.fillStyle = glowingCyan ? '#a5f3fc' : '#b8aea2';
    ctx.fillRect(top[0] - 2, top[1], 2.2, h - 2);
    ctx.fillStyle = glowingCyan ? '#22d3ee' : '#736a60';
    ctx.fillRect(top[0] + 0.2, top[1], 1.8, h - 2);
    ctx.strokeStyle = glowingCyan ? '#22d3ee' : '#171310';
    ctx.lineWidth = glowingCyan ? 1.1 : 0.75;
    ctx.strokeRect(top[0] - 2, top[1], 4, h - 2);

    // Corinthian capital
    ctx.fillStyle = glowingCyan ? '#e0f2fe' : '#cfc6ba';
    ctx.fillRect(top[0] - 2.8, top[1] - 1.5, 5.6, 2.2);
    ctx.strokeRect(top[0] - 2.8, top[1] - 1.5, 5.6, 2.2);
  }
}

/** Cypress tree cluster in weathered twilight palette (deep olive/charcoal foliage) */
function drawIsoCypressTree(ctx: CanvasRenderingContext2D, x: number, y: number, scale = 1) {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(scale, scale);

  // Ground shadow
  ctx.fillStyle = 'rgba(8, 7, 6, 0.48)';
  ctx.beginPath();
  ctx.ellipse(2, 1, 6.5, 3.2, 0, 0, Math.PI * 2);
  ctx.fill();

  // Trunk
  ctx.fillStyle = '#3b281e';
  ctx.fillRect(-1.2, -5, 2.4, 6);

  // Layered conical foliage (lit SE side + shadowed SW side + dark ink outline)
  fillPoly(
    ctx,
    [
      [0, -26],
      [4.8, -6],
      [0, -4],
      [-4.8, -6],
    ],
    '#233b24',
    '#0f1910',
    1.0,
  );
  // Sunlit right half highlight
  fillPoly(
    ctx,
    [
      [0, -26],
      [4.5, -6],
      [0, -4.5],
    ],
    '#345432',
    '',
    0,
  );
  ctx.restore();
}

/** Flaming golden brazier on a carved stone pedestal */
function drawIsoTorchBrazier(ctx: CanvasRenderingContext2D, x: number, y: number) {
  // Warm ground glow pool
  const glow = ctx.createRadialGradient(x, y - 6, 1, x, y - 4, 18);
  glow.addColorStop(0, 'rgba(251, 191, 36, 0.55)');
  glow.addColorStop(0.5, 'rgba(245, 158, 11, 0.22)');
  glow.addColorStop(1, 'rgba(245, 158, 11, 0)');
  ctx.fillStyle = glow;
  ctx.beginPath();
  ctx.arc(x, y - 4, 18, 0, Math.PI * 2);
  ctx.fill();

  // Stone pedestal & bronze tripod bowl
  ctx.fillStyle = '#57534e';
  ctx.strokeStyle = '#171310';
  ctx.lineWidth = 0.8;
  ctx.fillRect(x - 2.2, y - 6, 4.4, 6);
  ctx.strokeRect(x - 2.2, y - 6, 4.4, 6);

  // Flame
  ctx.fillStyle = '#f97316';
  ctx.beginPath();
  ctx.arc(x, y - 8, 3.2, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#fef08a';
  ctx.beginPath();
  ctx.arc(x, y - 8.5, 1.7, 0, Math.PI * 2);
  ctx.fill();
}

// ── 1. Per-File Isometric Roman Buildings (T0..T3) ──────────────────────────

function drawIsoFileBuilding(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  tier: Tier,
  opts: { activeCyan?: boolean; hasError?: boolean; hasWarning?: boolean; seed?: number } = {},
) {
  const s = opts.seed ?? 0;
  const ridgeAlong: 'u' | 'v' = s % 2 === 0 ? 'u' : 'v';

  if (opts.hasError) {
    drawIsoBurningInsula(ctx, px, py);
    return;
  }

  if (tier === 0) {
    // T0: Detailed Roman Taberna / Artisan Workshop with Stepped Plinth & Arched Shopfront (`Fornix`)
    drawIsoBox(ctx, px, py, -0.38, -0.34, 0, 0.38, 0.34, 2.5, {
      swFill: '#3f3831',
      seFill: '#6b6157',
      topFill: '#574e46',
    });
    drawIsoBox(ctx, px, py, -0.34, -0.30, 2.5, 0.34, 0.30, 13, {
      swFill: '#4e463e',
      seFill: '#7d7266',
      topFill: '#635a50',
      windowsSW: 1,
      groundArchesSE: 1,
      litWindows: true,
      cyanOutline: opts.activeCyan,
    });
    drawIsoPitchedRoof(ctx, px, py, -0.34, -0.30, 0.34, 0.30, 13, 21, ridgeAlong, {
      cyanOutline: opts.activeCyan,
    });
  } else if (tier === 1) {
    // T1: L-Shaped Roman Patrician Domus with Peristyle Atrium & Recessed `Impluvium` Rainwater Basin (`URBS-ARCH-DETAIL-03`)
    drawIsoBox(ctx, px, py, -0.44, -0.40, 0, 0.44, 0.40, 3.5, {
      swFill: '#443d37',
      seFill: '#6b6258',
      topFill: '#544c44',
    });
    // Recessed Marble `Impluvium` Rainwater Pool inside the SE courtyard quadrant
    fillPoly(
      ctx,
      [
        isoPt(px, py, 0.08, 0.10, 3.6),
        isoPt(px, py, 0.36, 0.10, 3.6),
        isoPt(px, py, 0.36, 0.34, 3.6),
        isoPt(px, py, 0.08, 0.34, 3.6),
      ],
      '#0284c7',
      '#d6cfc7',
      0.9,
    );
    // Main Domus wing with belt cornice & ground arch
    drawIsoBox(ctx, px, py, -0.38, -0.34, 3.5, 0.38, 0.08, 18, {
      swFill: '#524940',
      seFill: '#84786b',
      topFill: '#695f55',
      windowsSW: 2,
      windowsSE: 2,
      groundArchesSE: 1,
      beltCourse: true,
      litWindows: true,
      cyanOutline: opts.activeCyan,
    });
    drawIsoPitchedRoof(ctx, px, py, -0.38, -0.34, 0.38, 0.08, 18, 26, 'u', {
      cyanOutline: opts.activeCyan,
    });
    // Side Triclinium wing
    drawIsoBox(ctx, px, py, -0.38, 0.04, 3.5, 0.04, 0.36, 15, {
      swFill: '#4b433b',
      seFill: '#786d61',
      topFill: '#61584f',
      windowsSW: 1,
      windowsSE: 1,
      groundArchesSW: 1,
      litWindows: true,
      cyanOutline: opts.activeCyan,
    });
    drawIsoPitchedRoof(ctx, px, py, -0.38, 0.04, 0.04, 0.36, 15, 21, 'v', {
      cyanOutline: opts.activeCyan,
    });
  } else if (tier === 2) {
    // T2: Multi-Story Roman Insula with 2-stepped stylobate (`URBS-ARCH-REALISM-03`) + arcade & belt cornice
    drawIsoBox(ctx, px, py, -0.50, -0.48, 0, 0.50, 0.48, 2.2, {
      swFill: '#3f3831',
      seFill: '#6b6157',
      topFill: '#574e46',
    });
    drawIsoBox(ctx, px, py, -0.46, -0.44, 2.2, 0.46, 0.44, 4.2, {
      swFill: '#443d37',
      seFill: '#756b60',
      topFill: '#5c534a',
    });
    drawIsoBox(ctx, px, py, -0.44, -0.42, 4.2, 0.44, 0.42, 24, {
      swFill: '#4c443c',
      seFill: '#7c7064',
      topFill: '#665c52',
      courses: 4,
      windowsSW: 3,
      windowsSE: 3,
      groundArchesSW: 2,
      groundArchesSE: 2,
      beltCourse: true,
      litWindows: true,
      cyanOutline: opts.activeCyan,
    });
    // Upper clerestory attic
    drawIsoBox(ctx, px, py, -0.32, -0.30, 24, 0.32, 0.30, 32, {
      swFill: '#574e45',
      seFill: '#8a7d70',
      topFill: '#6e6459',
      windowsSW: 2,
      windowsSE: 2,
      litWindows: true,
      cyanOutline: opts.activeCyan,
    });
    drawIsoPitchedRoof(ctx, px, py, -0.34, -0.32, 0.34, 0.32, 32, 41, ridgeAlong, {
      roofLit: '#a64632',
      roofShade: '#69281b',
      cyanOutline: opts.activeCyan,
    });
  } else {
    // T3: Grand Colonnaded Isometric Roman Temple / Basilica with 2-Step Crepidoma & Belt Cornice
    drawIsoBox(ctx, px, py, -0.56, -0.52, 0, 0.56, 0.52, 3, {
      swFill: '#3f3831',
      seFill: '#695f55',
      topFill: '#5c534a',
      cyanOutline: opts.activeCyan,
    });
    drawIsoBox(ctx, px, py, -0.50, -0.46, 3, 0.50, 0.46, 6, {
      swFill: '#474039',
      seFill: '#756a5e',
      topFill: '#695f55',
      cyanOutline: opts.activeCyan,
    });
    // Inner cella sanctuary
    drawIsoBox(ctx, px, py, -0.38, -0.34, 6, 0.32, 0.34, 24, {
      swFill: opts.activeCyan ? '#164e63' : '#524940',
      seFill: opts.activeCyan ? '#0e7490' : '#85796c',
      topFill: '#6b6157',
      windowsSW: 2,
      groundArchesSE: 1,
      beltCourse: true,
      litWindows: true,
      cyanOutline: opts.activeCyan,
    });
    // Front & side peristyle colonnades in true isometric alignment
    drawIsoColonnade(ctx, px, py, 0.44, -0.38, 0.44, 0.38, 6, 24, 5, Boolean(opts.activeCyan));
    drawIsoColonnade(ctx, px, py, -0.42, 0.40, 0.44, 0.40, 6, 24, 5, Boolean(opts.activeCyan));
    // Entablature frieze
    drawIsoBox(ctx, px, py, -0.46, -0.42, 24, 0.46, 0.42, 28, {
      swFill: opts.activeCyan ? '#0891b2' : '#5e544a',
      seFill: opts.activeCyan ? '#22d3ee' : '#96897b',
      topFill: '#786d61',
      cyanOutline: opts.activeCyan,
    });
    // Monumental pitched temple roof & pediment
    drawIsoPitchedRoof(ctx, px, py, -0.46, -0.42, 0.46, 0.42, 28, 41, 'u', {
      roofLit: '#a84733',
      roofShade: '#6b291c',
      pedimentFill: opts.activeCyan ? '#67e8f9' : '#9c9082',
      cyanOutline: opts.activeCyan,
    });
  }
}

/** Concept Art A & B: Burning Roman Insula with exposed charred rafters,
 *  roaring interior fire, and rising volumetric smoke plume. */
function drawIsoBurningInsula(ctx: CanvasRenderingContext2D, px: number, py: number) {
  // Golden warning ground highlight
  fillPoly(
    ctx,
    [
      isoPt(px, py, -0.58, -0.54, 0),
      isoPt(px, py, 0.58, -0.54, 0),
      isoPt(px, py, 0.58, 0.54, 0),
      isoPt(px, py, -0.58, 0.54, 0),
    ],
    'rgba(245, 158, 11, 0.18)',
    '#f59e0b',
    1.5,
  );

  // Fire-blackened stone walls with glowing orange windows
  drawIsoBox(ctx, px, py, -0.46, -0.42, 0, 0.46, 0.42, 24, {
    swFill: '#2b231d',
    seFill: '#473930',
    topFill: '#1f1915',
    courses: 4,
    windowsSW: 3,
    windowsSE: 3,
    litWindows: true,
  });

  // Collapsed roof gable & exposed burning timber rafters
  drawIsoPitchedRoof(ctx, px, py, -0.46, -0.42, 0.46, 0.42, 24, 34, 'u', {
    roofLit: '#5c2318',
    roofShade: '#30130d',
    pedimentFill: '#44372f',
  });

  const roofCenter = isoPt(px, py, 0, 0, 28);
  const cx = roofCenter[0];
  const cy = roofCenter[1];

  // Volumetric dark smoke puffs rising up-right (matching Concept Art A & B)
  const smokePuffs: [number, number, number, number][] = [
    [cx + 4, cy - 16, 14, 0.52],
    [cx + 12, cy - 30, 19, 0.46],
    [cx + 22, cy - 46, 24, 0.38],
    [cx + 32, cy - 64, 28, 0.26],
  ];
  for (const [sx, sy, rad, alpha] of smokePuffs) {
    ctx.fillStyle = `rgba(24, 22, 20, ${alpha})`;
    ctx.beginPath();
    ctx.arc(sx, sy, rad, 0, Math.PI * 2);
    ctx.fill();
  }

  // Roaring multi-layered fire core
  const fireGrad = ctx.createRadialGradient(cx, cy - 4, 2, cx, cy - 6, 22);
  fireGrad.addColorStop(0, '#fef08a');
  fireGrad.addColorStop(0.45, '#f97316');
  fireGrad.addColorStop(0.8, 'rgba(220, 38, 38, 0.75)');
  fireGrad.addColorStop(1, 'rgba(220, 38, 38, 0)');
  ctx.fillStyle = fireGrad;
  ctx.beginPath();
  ctx.arc(cx, cy - 6, 22, 0, Math.PI * 2);
  ctx.fill();

  // Sharp flame tongues
  fillPoly(
    ctx,
    [
      [cx - 10, cy + 2],
      [cx - 4, cy - 18],
      [cx + 2, cy - 6],
      [cx + 8, cy - 22],
      [cx + 12, cy + 2],
    ],
    '#fb923c',
    '#ea580c',
    0.8,
  );
  fillPoly(
    ctx,
    [
      [cx - 5, cy + 2],
      [cx - 1, cy - 13],
      [cx + 5, cy - 15],
      [cx + 7, cy + 2],
    ],
    '#fef08a',
    '',
    0,
  );
}

/** Mechanism #1: True 3D Isometric Multi-Story Timber Scaffolding, Vitruvian
 *  Treadwheel Pulley Crane (`Polyspastos`) hoisting green-banded `+added LoC` ashlar
 *  stone blocks & sloped timber rubble chute dumping red-ochre `-removed LoC` rubble into an ox-cart. */
function drawIsoTimberScaffolding(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  diffAdded = 18,
  diffDeleted = 4,
) {
  const u0 = -0.48;
  const v0 = -0.44;
  const u1 = 0.48;
  const v1 = 0.44;

  // Emerald-banded fresh ashlar masonry courses along the top wall rim (`+added LoC`)
  if (diffAdded > 0) {
    const bandCourses = Math.min(3, Math.max(1, Math.ceil(diffAdded / 25)));
    for (let c = 0; c < bandCourses; c++) {
      const zLow = 16 + c * 4.5;
      const zHigh = zLow + 3.2;
      drawIsoBox(ctx, px, py, u0 + 0.04, v0 + 0.04, zLow, u1 - 0.04, v1 - 0.04, zHigh, {
        swFill: '#15803d',
        seFill: '#4ade80',
        topFill: '#86efac',
        stroke: '#052e16',
        lineWidth: 0.9,
        aoShadow: false,
      });
    }
  }

  // Isometric oak plank platforms at z = 10, 20, 30
  for (const z of [10, 20, 30]) {
    fillPoly(
      ctx,
      [
        isoPt(px, py, u0, v0, z),
        isoPt(px, py, u1, v0, z),
        isoPt(px, py, u1, v1, z),
        isoPt(px, py, u0, v1, z),
      ],
      'rgba(146, 64, 14, 0.56)',
      '#451a03',
      0.85,
    );
  }

  // Vertical oak timber poles at the 4 isometric corners + midpoints
  const poles: [number, number][] = [
    [u0, v0],
    [u1, v0],
    [u1, v1],
    [u0, v1],
    [0, v1],
    [u1, 0],
  ];
  for (const [u, v] of poles) {
    strokeLine(ctx, isoPt(px, py, u, v, 0), isoPt(px, py, u, v, 35), '#92400e', 2.1);
    strokeLine(ctx, isoPt(px, py, u, v, 0), isoPt(px, py, u, v, 35), '#d97706', 0.95);
  }

  // Diagonal X-bracing on SW and SE faces
  for (const [zA, zB] of [
    [0, 15],
    [15, 30],
  ] as const) {
    strokeLine(ctx, isoPt(px, py, u0, v1, zA), isoPt(px, py, u1, v1, zB), '#b45309', 1.15);
    strokeLine(ctx, isoPt(px, py, u1, v1, zA), isoPt(px, py, u0, v1, zB), '#b45309', 1.15);
    strokeLine(ctx, isoPt(px, py, u1, v0, zA), isoPt(px, py, u1, v1, zB), '#d97706', 1.15);
    strokeLine(ctx, isoPt(px, py, u1, v1, zA), isoPt(px, py, u1, v0, zB), '#d97706', 1.15);
  }

  // Vitruvian Treadwheel Crane (`Polyspastos` / `Magna Rota`) on the scaffolding crest
  const craneBase = isoPt(px, py, 0.12, 0.12, 30);
  const wheelCx = craneBase[0] - 9;
  const wheelCy = craneBase[1] - 8;
  ctx.fillStyle = 'rgba(120, 53, 15, 0.38)';
  ctx.beginPath();
  ctx.arc(wheelCx, wheelCy, 10, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = '#451a03';
  ctx.lineWidth = 3.0;
  ctx.beginPath();
  ctx.arc(wheelCx, wheelCy, 10, 0, Math.PI * 2);
  ctx.stroke();
  ctx.strokeStyle = '#d97706';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.arc(wheelCx, wheelCy, 10, 0, Math.PI * 2);
  ctx.stroke();
  for (let a = 0; a < 6; a++) {
    const ang = (a * Math.PI) / 6;
    strokeLine(
      ctx,
      [wheelCx - Math.cos(ang) * 9.5, wheelCy - Math.sin(ang) * 9.5],
      [wheelCx + Math.cos(ang) * 9.5, wheelCy + Math.sin(ang) * 9.5],
      '#b45309',
      1.1,
    );
  }

  // Crane mast, jib boom & suspended emerald-banded ashlar stone block (`+added LoC`)
  const jibTip: [number, number] = [craneBase[0] + 22, craneBase[1] - 30];
  strokeLine(ctx, craneBase, [craneBase[0], craneBase[1] - 24], '#78350f', 2.4);
  strokeLine(ctx, craneBase, jibTip, '#d97706', 2.4);
  strokeLine(ctx, [craneBase[0], craneBase[1] - 24], jibTip, '#d6d3d1', 1.1);
  // Hoist rope & travertine block with emerald `+added` mortar band
  strokeLine(ctx, jibTip, [jibTip[0], jibTip[1] + 15], '#e7e5e4', 1.2);
  ctx.fillStyle = '#d6d3d1';
  ctx.strokeStyle = '#171310';
  ctx.lineWidth = 1.0;
  ctx.fillRect(jibTip[0] - 5.5, jibTip[1] + 15, 11, 7.5);
  ctx.fillStyle = '#22c55e';
  ctx.fillRect(jibTip[0] - 5.5, jibTip[1] + 17.5, 11, 2.6);
  ctx.strokeRect(jibTip[0] - 5.5, jibTip[1] + 15, 11, 7.5);

  // Sloped Timber Rubble Chute & Wooden Ox-Cart for `-removed LoC` red-ochre rubble
  if (diffDeleted > 0) {
    const chuteTop = isoPt(px, py, u1, 0.08, 22);
    const chuteBot = isoPt(px, py, u1 + 0.38, 0.42, 4);
    fillPoly(
      ctx,
      [
        [chuteTop[0] - 3, chuteTop[1]],
        [chuteTop[0] + 3, chuteTop[1]],
        [chuteBot[0] + 4, chuteBot[1]],
        [chuteBot[0] - 4, chuteBot[1]],
      ],
      '#78350f',
      '#29180d',
      0.9,
    );
    // Red-ochre chiseled rubble stones tumbling down the chute
    const rubbleCount = Math.min(5, Math.max(2, Math.ceil(diffDeleted / 8)));
    for (let r = 0; r < rubbleCount; r++) {
      const t = (r + 0.5) / rubbleCount;
      const rx = chuteTop[0] + (chuteBot[0] - chuteTop[0]) * t;
      const ry = chuteTop[1] + (chuteBot[1] - chuteTop[1]) * t;
      ctx.fillStyle = r % 2 === 0 ? '#dc2626' : '#b91c1c';
      ctx.strokeStyle = '#450a0a';
      ctx.lineWidth = 0.7;
      ctx.beginPath();
      ctx.arc(rx, ry, 2.3, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
    // Wooden rubble ox-cart at the foot of the chute
    ctx.fillStyle = '#5c341d';
    ctx.strokeStyle = '#1c130d';
    ctx.lineWidth = 0.95;
    ctx.fillRect(chuteBot[0] - 7, chuteBot[1] - 3, 14, 7);
    ctx.strokeRect(chuteBot[0] - 7, chuteBot[1] - 3, 14, 7);
    // Pile of red-ochre rubble in the cart
    ctx.fillStyle = '#ef4444';
    ctx.beginPath();
    ctx.arc(chuteBot[0] - 2, chuteBot[1] - 3.5, 2.6, 0, Math.PI * 2);
    ctx.arc(chuteBot[0] + 2.5, chuteBot[1] - 3.5, 2.4, 0, Math.PI * 2);
    ctx.fill();
    // Wooden spoked cart wheels
    ctx.strokeStyle = '#b45309';
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.arc(chuteBot[0] - 4, chuteBot[1] + 4.5, 3, 0, Math.PI * 2);
    ctx.arc(chuteBot[0] + 4, chuteBot[1] + 4.5, 3, 0, Math.PI * 2);
    ctx.stroke();
  }
}

// ── 2. The 8 Vitruvian Roman Mechanical District Wonders in True Isometric 3D ─

/** Helper: Draws an authentic spoked Roman Oak Waterwheel (`Rota Aquaria`). */
function drawIsoRomanWaterwheel(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  radius = 16,
  angleOffset = 0,
) {
  ctx.save();
  // Dark mill-race water splash shadow
  ctx.fillStyle = 'rgba(14, 116, 144, 0.45)';
  ctx.beginPath();
  ctx.ellipse(x, y + radius - 2, radius * 0.9, 5, 0, 0, Math.PI * 2);
  ctx.fill();

  // Outer double oak rim
  ctx.strokeStyle = '#451a03';
  ctx.lineWidth = 4.2;
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.stroke();

  ctx.strokeStyle = '#b45309';
  ctx.lineWidth = 2.0;
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.stroke();

  // 8 wooden spokes & paddle buckets
  for (let i = 0; i < 8; i++) {
    const a = angleOffset + (i * Math.PI) / 4;
    const sx = x + Math.cos(a) * radius;
    const sy = y + Math.sin(a) * radius;
    strokeLine(ctx, [x, y], [sx, sy], '#92400e', 1.5);
    ctx.fillStyle = '#d97706';
    ctx.fillRect(sx - 2, sy - 2, 4, 4);
  }

  // Bronze axle hub
  ctx.fillStyle = '#fbbf24';
  ctx.strokeStyle = '#451a03';
  ctx.lineWidth = 1.0;
  ctx.beginPath();
  ctx.arc(x, y, 3.5, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

/** Mechanism #4 (`acropolis_engine`): Vitruvian Compiler & Typecheck Forge —
 *  Oak Waterwheel + Carved Stone Caliper Gauge Arch + Waterwheel-Driven Bronze
 *  Trip-Hammer stamping stone tablets + Emergency Timber-and-Iron Sluice-Dam
 *  that drops shut when `hasError` is true (`BLAST RADIUS BLOCKED`). */
function drawIsoPantheonAcropolis(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  activeCyan = false,
  hasError = false,
) {
  // Raised travertine forge terrace platform
  drawIsoBox(ctx, px, py, -1.35, -1.25, 0, 1.35, 1.25, 8, {
    swFill: '#4a433b',
    seFill: '#786d61',
    topFill: '#665c52',
    courses: 2,
  });
  drawIsoGrandStaircase(ctx, px, py, 0.95, 0.15, 8, 'se');

  // 1. Upper Aqueduct Mill-Race Channel feeding the Oak Waterwheel
  drawIsoBox(ctx, px, py, -1.25, -1.05, 8, -0.35, -0.65, 22, {
    swFill: '#443d36',
    seFill: '#73685d',
    topFill: '#0e7490',
    aoShadow: false,
  });
  const wheelPt = isoPt(px, py, -0.52, -0.52, 16);
  drawIsoRomanWaterwheel(ctx, wheelPt[0], wheelPt[1], 16, 0.25);

  // 2. Timber Roller-Conveyor Deck carrying Stone Code Tablets from NW to SE
  drawIsoBox(ctx, px, py, -0.85, -0.32, 8, 0.95, 0.32, 15, {
    swFill: '#5c341d',
    seFill: '#78350f',
    topFill: '#451a03',
    aoShadow: false,
  });

  // Incoming raw stone tablet before the Caliper Arch
  drawIsoBox(ctx, px, py, -0.68, -0.18, 15, -0.32, 0.18, 22, {
    swFill: '#78716c',
    seFill: '#d6d3d1',
    topFill: '#e7e5e4',
    aoShadow: false,
  });

  // 3. Carved Stone Typecheck Caliper Gauge Arch straddling the conveyor
  drawIsoBox(ctx, px, py, -0.22, -0.46, 8, 0.08, -0.24, 38, {
    swFill: '#524940',
    seFill: '#8c8072',
    topFill: '#73685d',
    courses: 3,
    aoShadow: false,
  });
  drawIsoBox(ctx, px, py, -0.22, 0.24, 8, 0.08, 0.46, 38, {
    swFill: '#524940',
    seFill: '#8c8072',
    topFill: '#73685d',
    courses: 3,
    aoShadow: false,
  });
  drawIsoBox(ctx, px, py, -0.25, -0.48, 38, 0.12, 0.48, 45, {
    swFill: '#5e544a',
    seFill: '#9e9182',
    topFill: '#827669',
    cyanOutline: activeCyan,
    aoShadow: false,
  });

  // 4. Waterwheel-Driven Bronze Trip-Hammer (`Malleus`) & Anvil Block
  const anvilU0 = 0.22;
  const anvilU1 = 0.68;
  drawIsoBox(ctx, px, py, anvilU0, -0.22, 15, anvilU1, 0.22, 23, {
    swFill: hasError ? '#7f1d1d' : '#14532d',
    seFill: hasError ? '#dc2626' : '#22c55e',
    topFill: hasError ? '#fca5a5' : '#86efac',
    aoShadow: false,
  });
  // Bronze Trip-Hammer Head suspended above the tablet
  const hammerZ = hasError ? 23 : 29;
  drawIsoBox(ctx, px, py, 0.30, -0.16, hammerZ, 0.60, 0.16, hammerZ + 10, {
    swFill: '#92400e',
    seFill: '#f59e0b',
    topFill: '#fde68a',
    aoShadow: false,
  });

  // If `hasError`: Cracked stone fissures, forge sparks, smoke plume & DROPPED Sluice-Dam (`BLAST RADIUS BLOCKED`)
  const anvilPt = isoPt(px, py, 0.45, 0, 23);
  if (hasError) {
    // Glowing red fracture seams & sparks on the cracked tablet
    strokeLine(ctx, [anvilPt[0] - 8, anvilPt[1] + 4], [anvilPt[0] + 6, anvilPt[1] - 5], '#fef08a', 1.8);
    strokeLine(ctx, [anvilPt[0] - 4, anvilPt[1] - 4], [anvilPt[0] + 9, anvilPt[1] + 5], '#ef4444', 1.6);
    for (const [dx, dy] of [
      [-14, -10],
      [12, -14],
      [-8, -18],
      [16, -6],
    ] as const) {
      strokeLine(ctx, anvilPt, [anvilPt[0] + dx, anvilPt[1] + dy], '#fbbf24', 1.3);
    }
    // Smoke puffs rising from the jammed forge
    for (const [sx, sy, rad, a] of [
      [anvilPt[0] + 4, anvilPt[1] - 18, 10, 0.48],
      [anvilPt[0] + 12, anvilPt[1] - 32, 15, 0.38],
    ] as const) {
      ctx.fillStyle = `rgba(24, 22, 20, ${a})`;
      ctx.beginPath();
      ctx.arc(sx, sy, rad, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // 5. Downstream Aqueduct Sluice-Dam (`Cataracta`): Open & flowing when healthy; slammed shut & dry when `hasError`
  drawIsoBox(ctx, px, py, 0.72, 0.45, 8, 1.28, 0.92, hasError ? 28 : 14, {
    swFill: hasError ? '#451a03' : '#0e7490',
    seFill: hasError ? '#78350f' : '#22d3ee',
    topFill: hasError ? '#92400e' : '#67e8f9',
    aoShadow: false,
  });

  drawIsoTorchBrazier(ctx, px + 24, py + 24);
  drawIsoCypressTree(ctx, px - 64, py + 8, 0.95);
}

/** Mechanism #7 (`amphitheatre_surface`): Oak Architect's Mosaic & Parchment
 *  UI Drafting Desk (`web-ui`) paired with a Roman Stone Harbor Sea-Arch & Toll-Paddles (`API/CLI`). */
function drawIsoColosseumWonder(ctx: CanvasRenderingContext2D, px: number, py: number) {
  // Paved limestone terrace
  drawIsoBox(ctx, px, py, -1.25, -1.10, 0, 1.25, 1.10, 6, {
    swFill: '#474039',
    seFill: '#786d61',
    topFill: '#635a50',
  });

  // 1. Background Right: Roman Carved Stone Harbor Sea-Archway with Wooden Toll-Paddles (`API / CLI Ingress`)
  const archPx = px + 28;
  const archPy = py - 14;
  // Left & right stone piers
  drawIsoBox(ctx, archPx, archPy, -0.52, -0.24, 6, -0.18, 0.24, 34, {
    swFill: '#544c44',
    seFill: '#8c8072',
    topFill: '#73685d',
    courses: 4,
    aoShadow: false,
  });
  drawIsoBox(ctx, archPx, archPy, 0.18, -0.24, 6, 0.52, 0.24, 34, {
    swFill: '#544c44',
    seFill: '#8c8072',
    topFill: '#73685d',
    courses: 4,
    aoShadow: false,
  });
  // Entablature & attic cornice over the Sea-Arch
  drawIsoBox(ctx, archPx, archPy, -0.58, -0.28, 34, 0.58, 0.28, 42, {
    swFill: '#61584f',
    seFill: '#9e9182',
    topFill: '#827669',
    aoShadow: false,
  });
  // Aqua sea channel flowing under the arch + hanging oak toll-paddles
  const seaPt = isoPt(archPx, archPy, 0, 0.1, 6);
  ctx.fillStyle = '#0891b2';
  ctx.beginPath();
  ctx.ellipse(seaPt[0], seaPt[1], 14, 7, 0, 0, Math.PI * 2);
  ctx.fill();
  strokeLine(ctx, [seaPt[0] - 5, seaPt[1] - 18], [seaPt[0] - 5, seaPt[1] - 4], '#b45309', 2.2);
  strokeLine(ctx, [seaPt[0] + 5, seaPt[1] - 18], [seaPt[0] + 5, seaPt[1] - 4], '#d97706', 2.2);

  // 2. Foreground Left: Angled Oak Architect's Drafting Desk (`Abacus`) with Mosaic Tesserae & Parchment UI Wireframe
  const deskPx = px - 22;
  const deskPy = py + 12;
  // Heavy oak trestle legs
  drawIsoBox(ctx, deskPx, deskPy, -0.68, -0.45, 6, 0.52, 0.45, 16, {
    swFill: '#451a03',
    seFill: '#78350f',
    topFill: '#92400e',
    aoShadow: false,
  });
  // Tilted drafting board surface
  const bTL = isoPt(deskPx, deskPy, -0.72, -0.48, 25);
  const bTR = isoPt(deskPx, deskPy, 0.56, -0.48, 25);
  const bBR = isoPt(deskPx, deskPy, 0.56, 0.48, 16);
  const bBL = isoPt(deskPx, deskPy, -0.72, 0.48, 16);
  fillPoly(ctx, [bTL, bTR, bBR, bBL], '#b45309', '#29180d', 1.2);

  // Inner parchment & mosaic wireframe layout sheet on the tilted drafting board
  const pTL: [number, number] = [bTL[0] + 4, bTL[1] + 3];
  const pTR: [number, number] = [bTR[0] - 4, bTR[1] + 3];
  const pBR: [number, number] = [bBR[0] - 4, bBR[1] - 2];
  const pBL: [number, number] = [bBL[0] + 4, bBL[1] - 2];
  fillPoly(ctx, [pTL, pTR, pBR, pBL], '#e7d8be', '#451a03', 0.9);

  // Colored Roman Mosaic Tesserae Tiles (left half of board) & UI Component Wireframe Boxes (right half)
  const midX = (pTL[0] + pBR[0]) / 2;
  const midY = (pTL[1] + pBR[1]) / 2;
  const tessColors = ['#0284c7', '#16a34a', '#d97706', '#dc2626', '#fbbf24', '#0d9488'];
  for (let r = 0; r < 2; r++) {
    for (let c = 0; c < 3; c++) {
      const tx = midX - 16 + c * 4.5 + r * 2;
      const ty = midY - 4 + r * 4.5;
      ctx.fillStyle = tessColors[(r * 3 + c) % tessColors.length]!;
      ctx.fillRect(tx, ty, 3.6, 3.6);
    }
  }
  // Miniature wireframe window panels on the parchment right side
  ctx.strokeStyle = '#292018';
  ctx.lineWidth = 0.9;
  ctx.strokeRect(midX + 2, midY - 5, 13, 9);
  strokeLine(ctx, [midX + 2, midY - 5], [midX + 15, midY + 4], '#574a3d', 0.65);

  drawIsoTorchBrazier(ctx, px + 44, py + 18);
  drawIsoCypressTree(ctx, px - 56, py - 14, 0.95);
}

/** Mechanism #2 (`praetorium_hub`): Roman Aqueduct Distribution Basin (`Castellum Divisorium`)
 *  with Bronze Sluice-Paddles, Oak Waterwheel (`Local Fast Model`), and Monumental
 *  Hammered Bronze `Libra` Balance Scale (`Parallel Cloud Consensus`). */
function drawIsoPraetoriumComplex(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  hasError = false,
) {
  // Paved travertine switchyard terrace
  drawIsoBox(ctx, px, py, -1.35, -1.25, 0, 1.35, 1.25, 6, {
    swFill: '#474039',
    seFill: '#73685d',
    topFill: '#5c534a',
  });

  // 1. Upper-Left: Carved Stone `Castellum Divisorium` Water-Splitter Basin
  const basinPx = px - 18;
  const basinPy = py - 12;
  drawIsoBox(ctx, basinPx, basinPy, -0.78, -0.72, 6, 0.62, 0.42, 20, {
    swFill: '#4e473f',
    seFill: '#82776b',
    topFill: '#6b6258',
    courses: 2,
    aoShadow: false,
  });
  // Luminous aqua water pool inside the Castellum basin
  const poolCenter = isoPt(basinPx, basinPy, -0.08, -0.15, 20);
  ctx.fillStyle = '#0891b2';
  ctx.strokeStyle = '#171310';
  ctx.lineWidth = 1.0;
  ctx.beginPath();
  ctx.ellipse(poolCenter[0], poolCenter[1], 28, 14, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  // Floating Golden Seal-Tokens inside the Castellum water basin
  for (const [dx, dy] of [
    [-10, -3],
    [2, -5],
    [11, 1],
    [-2, 4],
  ] as const) {
    ctx.fillStyle = '#fbbf24';
    ctx.strokeStyle = '#78350f';
    ctx.lineWidth = 0.8;
    ctx.beginPath();
    ctx.ellipse(poolCenter[0] + dx, poolCenter[1] + dy, 3.6, 2.1, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }

  // 3 Bronze Sluice-Gates along the SE front wall of the Castellum splitting into 3 water chutes
  for (let s = -1; s <= 1; s++) {
    const u = s * 0.34;
    const gateTop = isoPt(basinPx, basinPy, u, 0.42, 21);
    const chuteBot = isoPt(basinPx, basinPy, u + 0.15, 0.92, 6);
    // Bronze sluice frame
    ctx.fillStyle = '#d97706';
    ctx.strokeStyle = '#451a03';
    ctx.lineWidth = 0.8;
    ctx.fillRect(gateTop[0] - 3.5, gateTop[1] - 4, 7, 6);
    ctx.strokeRect(gateTop[0] - 3.5, gateTop[1] - 4, 7, 6);
    // Cascading aqua water stream down each consensus chute
    strokeLine(ctx, [gateTop[0], gateTop[1] + 2], [chuteBot[0], chuteBot[1]], '#22d3ee', 3.4);
    strokeLine(ctx, [gateTop[0], gateTop[1] + 2], [chuteBot[0], chuteBot[1]], '#e0f2fe', 1.2);
  }

  // 2. Left Side: Oak Waterwheel Mill (`Local Fast Model` flywheel)
  const localWheelPt = isoPt(px, py, -0.92, 0.25, 14);
  drawIsoRomanWaterwheel(ctx, localWheelPt[0], localWheelPt[1], 14, 0.5);

  // 3. Foreground Right: Monumental Hammered Bronze `Libra` Balance Scale (`Cloud Consensus`)
  const libraBase = isoPt(px, py, 0.55, 0.35, 6);
  // Bronze stepped pedestal
  ctx.fillStyle = '#b45309';
  ctx.strokeStyle = '#29180d';
  ctx.lineWidth = 1.0;
  ctx.beginPath();
  ctx.ellipse(libraBase[0], libraBase[1], 11, 5.5, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();
  // Vertical bronze column shaft
  const beamPivot: [number, number] = [libraBase[0], libraBase[1] - 30];
  strokeLine(ctx, libraBase, beamPivot, '#451a03', 4.2);
  strokeLine(ctx, libraBase, beamPivot, '#f59e0b', 2.4);
  // Tilted horizontal balance beam
  const leftArm: [number, number] = [beamPivot[0] - 20, beamPivot[1] + 3];
  const rightArm: [number, number] = [beamPivot[0] + 20, beamPivot[1] - 3];
  strokeLine(ctx, leftArm, rightArm, '#451a03', 3.2);
  strokeLine(ctx, leftArm, rightArm, '#fbbf24', 1.8);

  // Suspended bronze scale pans with consensus water/tokens
  for (const [armPt, panTint] of [
    [leftArm, '#38bdf8'],
    [rightArm, '#fbbf24'],
  ] as const) {
    const panY = armPt[1] + 15;
    strokeLine(ctx, armPt, [armPt[0] - 6, panY], '#d97706', 0.9);
    strokeLine(ctx, armPt, [armPt[0] + 6, panY], '#d97706', 0.9);
    ctx.fillStyle = '#b45309';
    ctx.strokeStyle = '#29180d';
    ctx.lineWidth = 0.95;
    ctx.beginPath();
    ctx.ellipse(armPt[0], panY, 8, 3.8, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = panTint;
    ctx.beginPath();
    ctx.ellipse(armPt[0], panY - 0.8, 5.5, 2.2, 0, 0, Math.PI * 2);
    ctx.fill();
  }

  if (hasError) {
    drawIsoBurningInsula(ctx, px + 36, py - 18);
  } else {
    drawIsoTorchBrazier(ctx, px + 36, py - 12);
  }
}

/** Mechanism #6 (`harbor_vault`): Cutaway Circular Roman Stone `Tabularium` & `Horreum`
 *  Granary Vault revealing 3 concentric tiers of terracotta amphorae & a rotating wooden picker ladder. */
function drawIsoBedrockHarborVault(ctx: CanvasRenderingContext2D, px: number, py: number) {
  const cx = px;
  const cy = py - 6;
  const rx = 44;
  const ry = 22;
  const wallH = 36;

  // Ground shadow
  ctx.fillStyle = 'rgba(8, 7, 6, 0.5)';
  ctx.beginPath();
  ctx.ellipse(cx + 3, cy + 8, rx + 5, ry + 4, 0, 0, Math.PI * 2);
  ctx.fill();

  // Back semicircular cutaway stone wall with 5-stop cylindrical normal shading & coffered top rim (`URBS-ARCH-REALISM-01`)
  const wallGrad = ctx.createLinearGradient(cx - rx, cy - wallH, cx + rx, cy);
  wallGrad.addColorStop(0, '#2e2823');
  wallGrad.addColorStop(0.28, '#5c5248');
  wallGrad.addColorStop(0.62, '#8a7d70');
  wallGrad.addColorStop(0.85, '#6b6055');
  wallGrad.addColorStop(1, '#3b332c');
  ctx.fillStyle = wallGrad;
  ctx.strokeStyle = 'rgba(28, 24, 20, 0.44)';
  ctx.lineWidth = 0.95;
  ctx.beginPath();
  ctx.ellipse(cx, cy - wallH, rx, ry, 0, Math.PI, 0, false);
  ctx.lineTo(cx + rx, cy);
  ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI, true);
  ctx.closePath();
  ctx.fill();
  ctx.stroke();

  // Top sunlit travertine rotunda entablature ring (`Corona`)
  ctx.strokeStyle = 'rgba(245, 235, 214, 0.56)';
  ctx.lineWidth = 1.6;
  ctx.beginPath();
  ctx.ellipse(cx, cy - wallH, rx, ry, 0, Math.PI, 0, false);
  ctx.stroke();

  // Recessed coffered half-dome ribs (`Lacunaria`, `URBS-ARCH-REALISM-01`)
  for (let rib = 1; rib <= 5; rib++) {
    const t = rib / 6;
    const ang0 = Math.PI + t * Math.PI;
    const ox = cx + Math.cos(ang0) * (rx * 0.92);
    const oy = (cy - wallH) + Math.sin(ang0) * (ry * 0.92);
    strokeLine(ctx, [ox, oy], [cx, cy - wallH * 0.35], 'rgba(18, 14, 11, 0.28)', 0.85);
    strokeLine(ctx, [ox * 0.02 + cx * 0.98, oy], [cx, cy - wallH * 0.35], 'rgba(254, 243, 199, 0.12)', 0.55);
  }
  for (let ring = 1; ring <= 3; ring++) {
    const rr = rx * (0.35 + ring * 0.18);
    const rry = ry * (0.35 + ring * 0.18);
    ctx.strokeStyle = 'rgba(28, 22, 18, 0.22)';
    ctx.lineWidth = 0.7;
    ctx.beginPath();
    ctx.ellipse(cx, cy - wallH * (0.55 - ring * 0.06), rr, rry, 0, Math.PI, 0, false);
    ctx.stroke();
  }

  // Circular paved stone floor with rotating oak track ring
  ctx.fillStyle = '#574e45';
  ctx.strokeStyle = 'rgba(28, 24, 20, 0.44)';
  ctx.lineWidth = 0.9;
  ctx.beginPath();
  ctx.ellipse(cx, cy, rx * 0.92, ry * 0.88, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  ctx.strokeStyle = '#92400e';
  ctx.lineWidth = 2.0;
  ctx.beginPath();
  ctx.ellipse(cx, cy, rx * 0.55, ry * 0.52, 0, 0, Math.PI * 2);
  ctx.stroke();

  // 3 Concentric Tiers of Arched Stone Niches holding Terracotta Amphorae
  for (let row = 0; row < 3; row++) {
    const rowY = cy - wallH + 10 + row * 10;
    const count = 9;
    for (let i = 1; i < count; i++) {
      const ang = Math.PI + (i / count) * Math.PI;
      const ax = cx + Math.cos(ang) * (rx * 0.78);
      const ay = rowY + Math.sin(ang) * (ry * 0.65);
      // Dark arched alcove
      ctx.fillStyle = '#1c1613';
      ctx.beginPath();
      ctx.ellipse(ax, ay, 3.6, 4.6, 0, 0, Math.PI * 2);
      ctx.fill();
      // Terracotta amphora vessel inside the alcove
      ctx.fillStyle = '#c2410c';
      ctx.strokeStyle = '#431407';
      ctx.lineWidth = 0.6;
      ctx.beginPath();
      ctx.ellipse(ax, ay + 0.8, 2.3, 3.2, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
  }

  // Central Rotating Oak Mast & Angled Wooden Picker Ladder
  strokeLine(ctx, [cx, cy + 2], [cx, cy - wallH + 4], '#78350f', 3.2);
  strokeLine(ctx, [cx, cy - wallH + 6], [cx + 22, cy - wallH + 12], '#b45309', 2.2);
  // Slanted wooden ladder rails & rungs
  const ladTop: [number, number] = [cx + 16, cy - wallH + 10];
  const ladBot: [number, number] = [cx + 6, cy + 4];
  strokeLine(ctx, [ladTop[0] - 2.5, ladTop[1]], [ladBot[0] - 2.5, ladBot[1]], '#d97706', 1.4);
  strokeLine(ctx, [ladTop[0] + 2.5, ladTop[1]], [ladBot[0] + 2.5, ladBot[1]], '#d97706', 1.4);
  for (let r = 1; r <= 5; r++) {
    const t = r / 6;
    const rx0 = ladTop[0] + (ladBot[0] - ladTop[0]) * t;
    const ry0 = ladTop[1] + (ladBot[1] - ladTop[1]) * t;
    strokeLine(ctx, [rx0 - 2.5, ry0], [rx0 + 2.5, ry0], '#fde68a', 0.9);
  }

  drawIsoTorchBrazier(ctx, px - 36, py + 12);
}

/** Mechanism #3 (`library_scholar`): Open-Walled Roman Wooden Scroll-Rack (`Armarium`),
 *  Brass Oil-Lamp (`Lucerna`), and Graduated Bronze `Clepsydra` Water-Vessel (`Context % Full`). */
function drawIsoLibraryOfCelsus(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  contextFillPct = 68,
) {
  // Raised oak & stone scriptorium plinth
  drawIsoBox(ctx, px, py, -1.15, -1.00, 0, 1.15, 1.00, 6, {
    swFill: '#4a433c',
    seFill: '#786d61',
    topFill: '#635a50',
  });

  // 1. Left/Center: Open-Fronted Roman Wooden Scroll-Cabinet (`Armarium`) with Pitched Terracotta Roof
  const armPx = px - 16;
  const armPy = py - 6;
  drawIsoBox(ctx, armPx, armPy, -0.75, -0.62, 6, 0.45, 0.52, 34, {
    swFill: '#5c341d',
    seFill: '#92400e',
    topFill: '#78350f',
    courses: 3,
    aoShadow: false,
  });
  drawIsoPitchedRoof(ctx, armPx, armPy, -0.78, -0.65, 0.48, 0.55, 34, 44, 'u', {
    roofLit: '#b44831',
    roofShade: '#69281b',
  });

  // Rolled Parchment Scrolls stacked inside the Armarium shelves + Unrolling Scroll Sheet
  for (let s = 0; s < 4; s++) {
    const sp = isoPt(armPx, armPy, -0.45 + s * 0.24, 0.53, 22);
    ctx.fillStyle = '#f5ebd6';
    ctx.strokeStyle = '#451a03';
    ctx.lineWidth = 0.85;
    ctx.fillRect(sp[0] - 3, sp[1] - 8, 6, 12);
    ctx.strokeRect(sp[0] - 3, sp[1] - 8, 6, 12);
  }

  // Warm Brass Roman Oil-Lamp (`Lucerna`) on the scriptorium table
  const lampPt = isoPt(armPx, armPy, 0.35, 0.55, 22);
  drawIsoTorchBrazier(ctx, lampPt[0], lampPt[1]);

  // 2. Right Foreground: Graduated Hammered-Bronze `Clepsydra` Water-Vessel (`Context % Full`)
  const clepPt = isoPt(px, py, 0.62, 0.32, 6);
  const cx = clepPt[0];
  const cy = clepPt[1];
  const topY = cy - 28;
  const rx = 14;
  const ry = 6.5;

  // Bronze cylinder body
  const bGrad = ctx.createLinearGradient(cx - rx, topY, cx + rx, cy);
  bGrad.addColorStop(0, '#78350f');
  bGrad.addColorStop(0.5, '#d97706');
  bGrad.addColorStop(1, '#451a03');
  ctx.fillStyle = bGrad;
  ctx.strokeStyle = '#1c130d';
  ctx.lineWidth = 1.1;
  ctx.beginPath();
  ctx.ellipse(cx, topY, rx, ry, 0, Math.PI, 0, true);
  ctx.lineTo(cx + rx * 0.85, cy);
  ctx.ellipse(cx, cy, rx * 0.85, ry * 0.85, 0, 0, Math.PI, false);
  ctx.closePath();
  ctx.fill();
  ctx.stroke();

  // Aqua water fill inside the cutaway window of the Clepsydra proportional to `contextFillPct`
  const clampedPct = Math.max(10, Math.min(100, contextFillPct));
  const waterY = cy - (clampedPct / 100) * 24;
  ctx.fillStyle = '#22d3ee';
  ctx.beginPath();
  ctx.ellipse(cx, waterY, rx * 0.82, ry * 0.78, 0, 0, Math.PI * 2);
  ctx.fill();

  // Horizontal graduation ticks on the bronze vessel
  for (let tick = 1; tick <= 3; tick++) {
    const ty = cy - tick * 6.5;
    strokeLine(ctx, [cx + 4, ty], [cx + 10, ty], '#fef08a', 0.9);
  }

  // Unrolled parchment scroll chute pouring from the Armarium into the Clepsydra mouth
  const scrollStart = isoPt(armPx, armPy, 0.15, 0.52, 20);
  strokeLine(ctx, scrollStart, [cx, topY], '#fde68a', 3.6);
  strokeLine(ctx, scrollStart, [cx, topY], '#78350f', 1.0);
}

/** Mechanism #5 (`circus_arena`): Roman Ballista Target Parapet with Pivoting
 *  Bronze Target Shields (`Signa` — Carved Green Laurel Wreaths = `PASS`,
 *  Splintered Shield with Flaming Red Oil Brazier = `FAIL`). */
function drawIsoCircusMaximus(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  errCount = 0,
) {
  // Raised ashlar stone proving-yard parapet wall
  drawIsoBox(ctx, px, py, -1.18, -0.55, 0, 1.18, 0.55, 16, {
    swFill: '#4c443c',
    seFill: '#85796c',
    topFill: '#6b6157',
    courses: 3,
  });

  // 4 Pivoting Roman Bronze Target Shields mounted along the sunlit SE parapet
  const shieldCount = 4;
  for (let i = 0; i < shieldCount; i++) {
    const u = -0.78 + i * 0.52;
    const sp = isoPt(px, py, u, 0.42, 26);
    const sx = sp[0];
    const sy = sp[1];
    const isFailingShield = errCount > 0 && i === shieldCount - 1;

    // Iron mounting bracket post
    strokeLine(ctx, [sx, sy + 10], [sx, sy + 16], '#292524', 2.2);

    // Curved Roman Scutum / Bronze Shield plate
    ctx.fillStyle = isFailingShield ? '#450a0a' : '#b45309';
    ctx.strokeStyle = isFailingShield ? '#ef4444' : '#fde68a';
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.roundRect(sx - 9, sy - 11, 18, 22, 4);
    ctx.fill();
    ctx.stroke();

    if (isFailingShield) {
      // Splintered shield crack + Flaming Red Oil Brazier (`FAIL`)
      strokeLine(ctx, [sx - 6, sy - 7], [sx + 6, sy + 7], '#f87171', 1.5);
      strokeLine(ctx, [sx + 5, sy - 6], [sx - 5, sy + 6], '#fbbf24', 1.3);
      drawIsoTorchBrazier(ctx, sx, sy - 6);
    } else {
      // Carved Emerald-Green Laurel Wreath (`PASS`)
      ctx.strokeStyle = '#22c55e';
      ctx.lineWidth = 2.2;
      ctx.beginPath();
      ctx.arc(sx, sy, 5.2, 0.2 * Math.PI, 0.8 * Math.PI, true);
      ctx.stroke();
      ctx.strokeStyle = '#86efac';
      ctx.lineWidth = 1.0;
      ctx.beginPath();
      ctx.arc(sx, sy, 5.2, 0.2 * Math.PI, 0.8 * Math.PI, true);
      ctx.stroke();
    }
  }
}

/** Mechanism #8 (`PORTA GIT` & `Needs You` Approval Gate): Paved Roman Milestone
 *  Checkpoint Road (`I → II → III` + Wax-Sealed Courier Chests) & Counterweighted
 *  Wrought-Iron Portcullis Gate (`[✓ RAISE PORTCULLIS] / [✕ HOLD]`). */
function drawIsoMilestoneRoadAndPortcullis(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  checkpointCount = 3,
  isGateBlocked = false,
) {
  // 1. Paved Roman Flagstone Road (`Via Appia`) approaching the Gatehouse
  fillPoly(
    ctx,
    [
      isoPt(px, py, -1.15, 0.35, 0),
      isoPt(px, py, 0.25, 0.35, 0),
      isoPt(px, py, 0.25, 0.95, 0),
      isoPt(px, py, -1.15, 0.95, 0),
    ],
    '#575049',
    '#292524',
    1.0,
  );

  // 2. Numbered Carved Stone Milestones (`I`, `II`, `III`) & Wax-Sealed Wooden Chests along the road
  const numerals = ['I', 'II', 'III'];
  const count = Math.min(3, Math.max(1, checkpointCount));
  for (let i = 0; i < count; i++) {
    const u = -0.92 + i * 0.38;
    const mp = isoPt(px, py, u, 0.28, 0);
    // Carved stone milestone pillar
    ctx.fillStyle = '#d6d3d1';
    ctx.strokeStyle = '#1c1917';
    ctx.lineWidth = 0.9;
    ctx.beginPath();
    ctx.roundRect(mp[0] - 4, mp[1] - 13, 8, 13, [4, 4, 0, 0]);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = '#1c1917';
    ctx.font = 'bold 6.5px Georgia, serif';
    ctx.textAlign = 'center';
    ctx.fillText(numerals[i] ?? 'I', mp[0], mp[1] - 4);

    // Wax-sealed wooden courier chest beside the milestone
    const cp = isoPt(px, py, u + 0.08, 0.68, 0);
    ctx.fillStyle = '#92400e';
    ctx.strokeStyle = '#29180d';
    ctx.lineWidth = 0.85;
    ctx.fillRect(cp[0] - 4.5, cp[1] - 6, 9, 6);
    ctx.strokeRect(cp[0] - 4.5, cp[1] - 6, 9, 6);
    ctx.fillStyle = '#dc2626';
    ctx.beginPath();
    ctx.arc(cp[0], cp[1] - 3, 1.5, 0, Math.PI * 2);
    ctx.fill();
  }

  // 3. Stone Portcullis Gatehouse & Oak Counterweight Frame
  const gx = px + 14;
  const gy = py + 8;
  drawIsoBox(ctx, gx, gy, -0.18, -0.42, 0, 0.42, -0.12, 34, {
    swFill: '#544c44',
    seFill: '#8c8072',
    topFill: '#73685d',
    courses: 4,
    aoShadow: false,
  });
  drawIsoBox(ctx, gx, gy, -0.18, 0.28, 0, 0.42, 0.58, 34, {
    swFill: '#544c44',
    seFill: '#8c8072',
    topFill: '#73685d',
    courses: 4,
    aoShadow: false,
  });
  // Top gatehouse lintel beam
  drawIsoBox(ctx, gx, gy, -0.22, -0.46, 34, 0.46, 0.62, 41, {
    swFill: '#61584f',
    seFill: '#9e9182',
    topFill: '#827669',
    aoShadow: false,
  });

  // Wrought-Iron Portcullis Grate (lowered to ground when `isGateBlocked`, raised halfway when open)
  const gateTop = isoPt(gx, gy, 0.12, 0.08, 32);
  const gateBotY = isGateBlocked ? gateTop[1] + 28 : gateTop[1] + 13;
  for (let b = -6; b <= 6; b += 4) {
    strokeLine(ctx, [gateTop[0] + b, gateTop[1]], [gateTop[0] + b, gateBotY], '#18181b', 1.4);
  }
  for (let yBar = gateTop[1] + 4; yBar <= gateBotY; yBar += 5) {
    strokeLine(ctx, [gateTop[0] - 8, yBar], [gateTop[0] + 8, yBar], '#27272a', 1.2);
  }

  // Oak Counterweight Lever beside the portcullis
  const leverBase = isoPt(gx, gy, 0.48, 0.62, 0);
  strokeLine(ctx, leverBase, [leverBase[0] + 8, leverBase[1] - 18], '#b45309', 2.4);
  ctx.fillStyle = isGateBlocked ? '#fbbf24' : '#22c55e';
  ctx.beginPath();
  ctx.arc(leverBase[0] + 8, leverBase[1] - 18, 3.2, 0, Math.PI * 2);
  ctx.fill();

  if (isGateBlocked) {
    drawIsoTorchBrazier(ctx, gx - 16, gy + 16);
  }
}

/** `watchtower_leaf` (`< 1200 LoC`): Fortified Roman Watchtower,
 *  2-Story Roadside Cottages & Glowing Supply Wagon (`400 LoC`). */
function drawIsoWatchtowerHamlet(ctx: CanvasRenderingContext2D, px: number, py: number) {
  // Square stone watchtower (`Turris`) with crenellated battlements
  drawIsoBox(ctx, px, py, -0.36, -0.36, 0, 0.18, 0.18, 36, {
    swFill: '#4a433c',
    seFill: '#7d7266',
    topFill: '#635a50',
    courses: 5,
    windowsSW: 1,
    windowsSE: 1,
    litWindows: true,
  });
  // Crenellated turret crown
  drawIsoBox(ctx, px, py, -0.40, -0.40, 36, 0.22, 0.22, 41, {
    swFill: '#544c44',
    seFill: '#8a7e71',
    topFill: '#5c534a',
    aoShadow: false,
  });

  // Adjacent roadside cottage with pitched terracotta roof
  drawIsoBox(ctx, px, py, 0.26, -0.24, 0, 0.78, 0.34, 14, {
    swFill: '#4e463e',
    seFill: '#807468',
    topFill: '#665c52',
    windowsSW: 1,
    windowsSE: 1,
    litWindows: true,
  });
  drawIsoPitchedRoof(ctx, px, py, 0.26, -0.24, 0.78, 0.34, 14, 22, 'u');

  // Glowing golden supply cart on the cobbled lane
  const cartPt = isoPt(px, py, -0.15, 0.55, 0);
  drawIsoTorchBrazier(ctx, cartPt[0] - 10, cartPt[1]);
  ctx.fillStyle = '#92400e';
  ctx.strokeStyle = '#171310';
  ctx.lineWidth = 0.9;
  ctx.fillRect(cartPt[0] - 6, cartPt[1] - 7, 12, 6);
  ctx.strokeRect(cartPt[0] - 6, cartPt[1] - 7, 12, 6);
  ctx.fillStyle = '#fbbf24';
  ctx.fillRect(cartPt[0] - 4, cartPt[1] - 10, 8, 4);

  drawIsoCypressTree(ctx, px - 28, py + 8, 0.88);
}

// ── 3. Fortified District Curtain Walls, Sculpted Rock Cliffs & Grand Stairs ─

function drawIsoGrandStaircase(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  uCenter: number,
  vCenter: number,
  dropH: number,
  dir: 'sw' | 'se' = 'sw',
) {
  const steps = Math.max(5, Math.min(10, Math.round(dropH / 4)));
  const topAnchor = isoPt(px, py, uCenter, vCenter, 0);
  const dx = dir === 'sw' ? -18 : 18;
  const dy = 10 + dropH * 0.65;
  const w = 14;

  // Side stone balustrades
  fillPoly(
    ctx,
    [
      [topAnchor[0] - w, topAnchor[1] - 3],
      [topAnchor[0] + w, topAnchor[1] + 3],
      [topAnchor[0] + dx + w, topAnchor[1] + dy + 3],
      [topAnchor[0] + dx - w, topAnchor[1] + dy - 3],
    ],
    '#786e63',
    '#171310',
    1.1,
  );

  // Individual stone treads
  for (let s = 0; s <= steps; s++) {
    const t = s / steps;
    const sx = topAnchor[0] + dx * t;
    const sy = topAnchor[1] + dy * t;
    strokeLine(ctx, [sx - w + 2, sy - 2], [sx + w - 2, sy + 2], s % 2 === 0 ? '#a89f93' : '#3a332c', 1.2);
  }

  // Flanking golden braziers at the stair landing (Concept Art A)
  drawIsoTorchBrazier(ctx, topAnchor[0] - w - 3, topAnchor[1]);
  drawIsoTorchBrazier(ctx, topAnchor[0] + w + 3, topAnchor[1] + 4);
}

/** Draws the back or front halves of a district's 3D isometric fortified
 *  stone curtain wall with crenellations, corner watchtowers, and an arched gate. */
function drawDistrictCurtainWall(
  ctx: CanvasRenderingContext2D,
  layout: TownLayout,
  d: District,
  elev: number,
  pass: 'back' | 'front',
) {
  // Small leaf hamlets (`watchtower_leaf`) stand outside the main curtain walls as roadside outposts
  if ((d.totalLines ?? 2000) < 1200 && !d.landmark) return;

  const pad = 0.35;
  const x0 = d.x0 + pad;
  const y0 = d.y0 + pad;
  const x1 = d.x1 - pad;
  const y1 = d.y1 - pad;
  const wallH = d.landmark ? 14 : 10;
  const thick = 0.22;

  const pTop = worldPx(layout, x0, y0);
  const pRight = worldPx(layout, x1, y0);
  const pBot = worldPx(layout, x1, y1);
  const pLeft = worldPx(layout, x0, y1);

  if (pass === 'back') {
    // Back-right wall (from Top(x0,y0) to Right(x1,y0))
    fillPoly(
      ctx,
      [
        [pTop.px, pTop.py - elev - wallH],
        [pRight.px, pRight.py - elev - wallH],
        [pRight.px, pRight.py - elev],
        [pTop.px, pTop.py - elev],
      ],
      createVerticalWallGradient(ctx, pTop.py - elev - wallH, pRight.py - elev, '#5c534a', false),
      'rgba(28, 22, 18, 0.36)',
      0.75,
    );
    strokeLine(
      ctx,
      [pTop.px, pTop.py - elev - wallH],
      [pRight.px, pRight.py - elev - wallH],
      'rgba(231, 220, 204, 0.42)',
      1.2,
    );
    // Back-left wall (from Top(x0,y0) to Left(x0,y1))
    fillPoly(
      ctx,
      [
        [pLeft.px, pLeft.py - elev - wallH],
        [pTop.px, pTop.py - elev - wallH],
        [pTop.px, pTop.py - elev],
        [pLeft.px, pLeft.py - elev],
      ],
      createVerticalWallGradient(ctx, pTop.py - elev - wallH, pLeft.py - elev, '#756b60', true),
      'rgba(28, 22, 18, 0.36)',
      0.75,
    );
    strokeLine(
      ctx,
      [pLeft.px, pLeft.py - elev - wallH],
      [pTop.px, pTop.py - elev - wallH],
      'rgba(245, 235, 214, 0.48)',
      1.2,
    );
    // North corner bastion tower with 3D pitched terracotta roof (`URBS-TERRACE-CRAFT-01`)
    drawIsoBox(ctx, pTop.px, pTop.py - elev, -thick, -thick, 0, thick, thick, wallH + 8, {
      swFill: '#4a433c',
      seFill: '#7d7266',
      topFill: '#665c52',
      aoShadow: false,
    });
    drawIsoPitchedRoof(ctx, pTop.px, pTop.py - elev, -thick, -thick, thick, thick, wallH + 8, wallH + 14, 'u');
  } else {
    // Front-left wall (from Left(x0,y1) to Bot(x1,y1)) — SW shadowed face
    fillPoly(
      ctx,
      [
        [pLeft.px, pLeft.py - elev - wallH],
        [pBot.px, pBot.py - elev - wallH],
        [pBot.px, pBot.py - elev],
        [pLeft.px, pLeft.py - elev],
      ],
      createVerticalWallGradient(ctx, pLeft.py - elev - wallH, pBot.py - elev, '#474039', false),
      'rgba(28, 22, 18, 0.38)',
      0.8,
    );
    // Molded travertine parapet walkway & capstone (`Corona`, `URBS-TERRACE-CRAFT-01`)
    fillPoly(
      ctx,
      [
        [pLeft.px, pLeft.py - elev - wallH - 3],
        [pBot.px, pBot.py - elev - wallH - 3],
        [pBot.px, pBot.py - elev - wallH],
        [pLeft.px, pLeft.py - elev - wallH],
      ],
      '#6b6157',
      'rgba(28, 22, 18, 0.32)',
      0.65,
    );
    strokeLine(
      ctx,
      [pLeft.px, pLeft.py - elev - wallH - 3],
      [pBot.px, pBot.py - elev - wallH - 3],
      'rgba(231, 220, 204, 0.45)',
      1.1,
    );

    // Front-right wall (from Bot(x1,y1) to Right(x1,y0)) — SE sunlit face
    fillPoly(
      ctx,
      [
        [pBot.px, pBot.py - elev - wallH],
        [pRight.px, pRight.py - elev - wallH],
        [pRight.px, pRight.py - elev],
        [pBot.px, pBot.py - elev],
      ],
      createVerticalWallGradient(ctx, pRight.py - elev - wallH, pBot.py - elev, '#7e7367', true),
      'rgba(28, 22, 18, 0.38)',
      0.8,
    );
    fillPoly(
      ctx,
      [
        [pBot.px, pBot.py - elev - wallH - 3],
        [pRight.px, pRight.py - elev - wallH - 3],
        [pRight.px, pRight.py - elev - wallH],
        [pBot.px, pBot.py - elev - wallH],
      ],
      '#94887a',
      'rgba(28, 22, 18, 0.32)',
      0.65,
    );
    strokeLine(
      ctx,
      [pBot.px, pBot.py - elev - wallH - 3],
      [pRight.px, pRight.py - elev - wallH - 3],
      'rgba(254, 243, 199, 0.58)',
      1.2,
    );

    // Corner bastions at Left, Right, and Bottom corners with 3D terracotta roofs
    for (const corner of [pLeft, pRight, pBot]) {
      drawIsoBox(ctx, corner.px, corner.py - elev, -0.20, -0.20, 0, 0.20, 0.20, wallH + 7, {
        swFill: '#4a433c',
        seFill: '#827669',
        topFill: '#695f55',
        aoShadow: false,
      });
      drawIsoPitchedRoof(ctx, corner.px, corner.py - elev, -0.20, -0.20, 0.20, 0.20, wallH + 7, wallH + 13, 'u');
    }

    // Arched Gatehouse in the middle of the front-left wall
    const gateX = (pLeft.px + pBot.px) / 2;
    const gateY = (pLeft.py + pBot.py) / 2 - elev;
    drawIsoBox(ctx, gateX, gateY, -0.24, -0.18, 0, 0.24, 0.18, wallH + 6, {
      swFill: '#524940',
      seFill: '#877b6e',
      topFill: '#6e6459',
      aoShadow: false,
    });
    ctx.fillStyle = '#14110e';
    ctx.beginPath();
    ctx.arc(gateX, gateY - 2, 4.5, Math.PI, 0);
    ctx.fill();
  }
}

/** Draws a multi-arch Roman stone viaduct bridge with a glowing cyan water
 *  canal between two elevated district terraces (matching Concept Art A). */
function drawIsoViaductBridge(
  ctx: CanvasRenderingContext2D,
  fromPt: { px: number; py: number },
  toPt: { px: number; py: number },
  isViolation: boolean,
) {
  const dx = toPt.px - fromPt.px;
  const dy = toPt.py - fromPt.py;
  const dist = Math.hypot(dx, dy);
  if (dist < 24) return;

  const spans = Math.max(3, Math.min(8, Math.round(dist / 36)));
  const pierH = 22;

  // Stone piers & arches along the span
  for (let i = 0; i < spans; i++) {
    const t0 = i / spans;
    const t1 = (i + 1) / spans;
    const x0 = fromPt.px + dx * t0;
    const y0 = fromPt.py + dy * t0;
    const x1 = fromPt.px + dx * t1;
    const y1 = fromPt.py + dy * t1;
    const xm = (x0 + x1) / 2;
    const ym = (y0 + y1) / 2;

    // Pier column
    ctx.fillStyle = '#4e463e';
    ctx.fillRect(x0 - 3.5, y0, 3.5, pierH);
    ctx.fillStyle = '#786d61';
    ctx.fillRect(x0, y0, 3.5, pierH);
    strokeLine(ctx, [x0 - 3.5, y0], [x0 + 3.5, y0], 'rgba(231, 222, 209, 0.40)', 0.9);

    // Spandrel masonry wall between piers
    fillPoly(
      ctx,
      [
        [x0, y0],
        [x1, y1],
        [x1, y1 + 10],
        [x0, y0 + 10],
      ],
      '#786e63',
      '#171310',
      0.85,
    );

    // Warm golden/emerald arch reflection beneath each span (Plate 2 Romanesque)
    ctx.fillStyle = isViolation ? 'rgba(239, 68, 68, 0.55)' : 'rgba(212, 175, 55, 0.42)';
    ctx.beginPath();
    ctx.arc(xm, ym + 11, 6, Math.PI, 0);
    ctx.fill();
  }

  // Outer glowing burnished gold/crimson canal aura along the viaduct deck
  strokeLine(
    ctx,
    [fromPt.px, fromPt.py],
    [toPt.px, toPt.py],
    isViolation ? 'rgba(239, 68, 68, 0.45)' : 'rgba(212, 175, 55, 0.45)',
    8,
  );
  // Stone parapet curbing
  strokeLine(
    ctx,
    [fromPt.px, fromPt.py],
    [toPt.px, toPt.py],
    '#443d36',
    4.8,
  );
  // Luminous golden-emerald aqueduct water stream in the specus channel
  strokeLine(
    ctx,
    [fromPt.px, fromPt.py],
    [toPt.px, toPt.py],
    isViolation ? '#f87171' : '#d4af37',
    2.6,
  );
  strokeLine(
    ctx,
    [fromPt.px, fromPt.py],
    [toPt.px, toPt.py],
    isViolation ? '#fecaca' : '#fef3c7',
    1.1,
  );
}

// ── 4. Full-Bleed Sculpted Twilight Landscape, Sea, Cliffs & Aqueducts ──────

/** Draws 3D faceted craggy coastal rock boulders along cliff bases (Concept Art A) */
function drawIsoRockOutcrop(ctx: CanvasRenderingContext2D, x: number, y: number, w = 26, h = 18) {
  // Left shadowed rock facet
  fillPoly(
    ctx,
    [
      [x - w, y + 4],
      [x - w * 0.35, y - h],
      [x + 2, y - h * 0.75],
      [x, y + 8],
    ],
    '#2b2723',
    '#12100e',
    1.1,
  );
  // Right sunlit rock facet
  fillPoly(
    ctx,
    [
      [x + 2, y - h * 0.75],
      [x + w * 0.85, y - h * 0.35],
      [x + w, y + 5],
      [x, y + 8],
    ],
    '#575049',
    '#12100e',
    1.1,
  );
  // Top ridge highlight facet
  fillPoly(
    ctx,
    [
      [x - w * 0.35, y - h],
      [x + w * 0.4, y - h * 0.9],
      [x + w * 0.85, y - h * 0.35],
      [x + 2, y - h * 0.75],
    ],
    '#786f65',
    '#12100e',
    1.0,
  );
}

/** Phase 294 (`URBS-SEMIOTICS-CV-01`): Diegetic Roman Hydraulic Sluice-Valve & Vexillum Dispatch Conduit
 *  (replaces the old translucent yellow polygon arrow with an isometric bronze/crimson pipe, valve wheel & standard). */
function drawTacticalErrorDispatchOverlay(
  ctx: CanvasRenderingContext2D,
  fromX: number,
  fromY: number,
  targetX: number,
  targetY: number,
  calloutLabel = 'Fix Diagnostic Errors',
) {
  ctx.save();
  // Isometric 2-segment bronze & crimson fault conduit with elbow valve
  const elbowX = fromX + (targetX - fromX) * 0.55;
  const elbowY = fromY + (targetY - fromY) * 0.28;

  strokeLine(ctx, [fromX, fromY], [elbowX, elbowY], '#3b1212', 4.6);
  strokeLine(ctx, [elbowX, elbowY], [targetX, targetY], '#3b1212', 4.6);
  strokeLine(ctx, [fromX, fromY], [elbowX, elbowY], 'rgba(245, 158, 11, 0.85)', 2.2);
  strokeLine(ctx, [elbowX, elbowY], [targetX, targetY], 'rgba(239, 68, 68, 0.88)', 2.2);

  // Bronze Sluice-Valve (`Epistomium`) at the elbow
  ctx.fillStyle = '#29180d';
  ctx.strokeStyle = '#fbbf24';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.arc(elbowX, elbowY, 4.2, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  // Origin bronze manifold ring
  ctx.strokeStyle = 'rgba(251, 191, 36, 0.78)';
  ctx.lineWidth = 1.4;
  ctx.beginPath();
  ctx.ellipse(fromX, fromY, 12, 6, 0, 0, Math.PI * 2);
  ctx.stroke();

  // Hanging Crimson Vexillum Callout over the burning / gate building
  const bx = targetX - 52;
  const by = targetY - 54;
  ctx.fillStyle = '#991b1b';
  ctx.strokeStyle = '#fbbf24';
  ctx.lineWidth = 1.2;
  ctx.fillRect(bx, by, 16, 20);
  ctx.strokeRect(bx, by, 16, 20);

  // Warning triangle emblem (`▲`) inside the Vexillum banner
  ctx.fillStyle = '#fef08a';
  ctx.beginPath();
  ctx.moveTo(bx + 8, by + 5);
  ctx.lineTo(bx + 13, by + 15);
  ctx.lineTo(bx + 3, by + 15);
  ctx.closePath();
  ctx.fill();

  ctx.font = 'bold 9.5px Cinzel, Palatino, Georgia, serif';
  const labelW = Math.max(118, ctx.measureText(calloutLabel).width + 18);

  ctx.fillStyle = 'rgba(20, 16, 13, 0.94)';
  ctx.strokeStyle = '#f59e0b';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.roundRect(bx + 18, by - 2, labelW, 24, 5);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = '#fde68a';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(calloutLabel, bx + 24, by + 10);
  ctx.restore();
}

/** Phase 290 (`URBS-GROUNDS-03`): Vaulted Roman `Substructio` Blind Arcades & Buttress Pilasters along cliff retaining walls. */
function drawSubstructioCliffArcades(
  ctx: CanvasRenderingContext2D,
  pStart: [number, number],
  pEnd: [number, number],
  retainH: number,
  isSunlit: boolean,
) {
  const dist = Math.hypot(pEnd[0] - pStart[0], pEnd[1] - pStart[1]);
  const bays = Math.max(3, Math.min(7, Math.floor(dist / 34)));
  const archW = 0.56 / bays;

  for (let i = 0; i < bays; i++) {
    const tCenter = (i + 0.5) / bays;
    const tA = tCenter - archW * 0.5;
    const tB = tCenter + archW * 0.5;

    const xA = pStart[0] + (pEnd[0] - pStart[0]) * tA;
    const yA = pStart[1] + (pEnd[1] - pStart[1]) * tA;
    const xB = pStart[0] + (pEnd[0] - pStart[0]) * tB;
    const yB = pStart[1] + (pEnd[1] - pStart[1]) * tB;
    const xM = (xA + xB) * 0.5;
    const yM = (yA + yB) * 0.5;

    // Recessed blind-arch vault niche
    fillPoly(
      ctx,
      [
        [xA, yA + retainH * 0.38],
        [xM, yM + retainH * 0.18],
        [xB, yB + retainH * 0.38],
        [xB, yB + retainH * 0.92],
        [xA, yA + retainH * 0.92],
      ],
      isSunlit ? 'rgba(28, 23, 19, 0.58)' : 'rgba(12, 10, 8, 0.68)',
      isSunlit ? 'rgba(214, 200, 182, 0.32)' : 'rgba(87, 78, 69, 0.28)',
      0.7,
    );

    // Projecting ashlar buttress pilaster between bays
    if (i > 0) {
      const tP = i / bays;
      const xP = pStart[0] + (pEnd[0] - pStart[0]) * tP;
      const yP = pStart[1] + (pEnd[1] - pStart[1]) * tP;
      strokeLine(
        ctx,
        [xP, yP + 1],
        [xP, yP + retainH - 1],
        isSunlit ? 'rgba(231, 220, 204, 0.44)' : 'rgba(120, 110, 99, 0.34)',
        1.5,
      );
    }
  }
}

/** Phase 290 (`URBS-GROUNDS-01..02, 04`): Orthogonal `Cardo`/`Decumanus` Basalt Streets,
 *  Raised Travertine Curbstones (`Crepido`), Stepping-Stone Crosswalks, Central `Opus Sectile`
 *  Marble Forum Plaza, and Corner Stone Boundary Obelisks (`Termini`). */
function drawDistrictGroundDetails(
  ctx: CanvasRenderingContext2D,
  layout: TownLayout,
  d: District,
  elev: number,
) {
  const cx = (d.x0 + d.x1) / 2;
  const cy = (d.y0 + d.y1) / 2;
  const pad = 0.24;
  const roadHalfW = 0.28;
  const forumR = Math.min(1.55, Math.min(d.x1 - d.x0, d.y1 - d.y0) * 0.28);

  // 1. Seamless 4-Arm `Via Silicea` Basalt Streets (`URBS-TERRACE-CRAFT-02`)
  //    Each arm terminates cleanly at the Forum perimeter (`forumR`) so there is ZERO
  //    overlapping translucent cross seam in the center of the plaza!
  const streetArms: [number, number, number, number][] = [
    [cx - roadHalfW, d.y0 + pad, cx + roadHalfW, cy - forumR], // North Cardo arm
    [cx - roadHalfW, cy + forumR, cx + roadHalfW, d.y1 - pad], // South Cardo arm
    [d.x0 + pad, cy - roadHalfW, cx - forumR, cy + roadHalfW], // West Decumanus arm
    [cx + forumR, cy - roadHalfW, d.x1 - pad, cy + roadHalfW], // East Decumanus arm
  ];

  for (const [ax0, ay0, ax1, ay1] of streetArms) {
    if (ax1 <= ax0 || ay1 <= ay0) continue;
    const armPts: [number, number][] = [
      worldPx(layout, ax0, ay0),
      worldPx(layout, ax1, ay0),
      worldPx(layout, ax1, ay1),
      worldPx(layout, ax0, ay1),
    ].map((p) => [p.px, p.py - elev]);

    // Recessed basalt carriageway + warm travertine `Crepido` curbstone bevel
    fillPoly(ctx, armPts, 'rgba(30, 26, 23, 0.86)', 'rgba(214, 200, 182, 0.54)', 0.95);
    strokeLine(ctx, armPts[1], armPts[2], 'rgba(254, 243, 199, 0.44)', 0.85);
  }

  // Roman Stepping-Stone Crosswalks (`Pons`) along the 4 street approaches
  for (const offset of [-1.42, 1.42]) {
    if (cx + offset > d.x0 + 0.6 && cx + offset < d.x1 - 0.6) {
      const cw = worldPx(layout, cx + offset, cy);
      ctx.fillStyle = '#d6cfc7';
      ctx.fillRect(cw.px - 2.6, cw.py - elev - 1.1, 5.2, 2.2);
    }
    if (cy + offset > d.y0 + 0.6 && cy + offset < d.y1 - 0.6) {
      const cw = worldPx(layout, cx, cy + offset);
      ctx.fillStyle = '#d6cfc7';
      ctx.fillRect(cw.px - 2.6, cw.py - elev - 1.1, 5.2, 2.2);
    }
  }

  // 2. Central `Opus Sectile` Geometric Marble Forum Plaza (`URBS-TERRACE-CRAFT-03`)
  //    Recessed 2-step travertine border + solid warm marble/porphyry backing
  const stepR = forumR + 0.14;
  const stepForumPts: [number, number][] = [
    worldPx(layout, cx - stepR, cy - stepR),
    worldPx(layout, cx + stepR, cy - stepR),
    worldPx(layout, cx + stepR, cy + stepR),
    worldPx(layout, cx - stepR, cy + stepR),
  ].map((p) => [p.px, p.py - elev]);
  fillPoly(ctx, stepForumPts, 'rgba(68, 60, 52, 0.78)', 'rgba(231, 222, 209, 0.56)', 0.9);

  const outerForumPts: [number, number][] = [
    worldPx(layout, cx - forumR, cy - forumR),
    worldPx(layout, cx + forumR, cy - forumR),
    worldPx(layout, cx + forumR, cy + forumR),
    worldPx(layout, cx - forumR, cy + forumR),
  ].map((p) => [p.px, p.py - elev]);

  const forumGrad = createVerticalWallGradient(
    ctx,
    outerForumPts[0][1],
    outerForumPts[2][1],
    '#6b442a',
    true,
  );
  fillPoly(ctx, outerForumPts, forumGrad, 'rgba(231, 222, 209, 0.68)', 1.05);

  // Inner rotated diamond `Opus Sectile` travertine & imperial porphyry medallion
  const innerR = forumR * 0.80;
  const diamondPts: [number, number][] = [
    worldPx(layout, cx, cy - innerR),
    worldPx(layout, cx + innerR, cy),
    worldPx(layout, cx, cy + innerR),
    worldPx(layout, cx - innerR, cy),
  ].map((p) => [p.px, p.py - elev]);

  fillPoly(ctx, diamondPts, 'rgba(231, 222, 209, 0.28)', 'rgba(251, 191, 36, 0.52)', 0.95);

  // 3. 4 Corner Corinthian Peristyle Columns (`Columnae`) with Pedestals & Ground Shadows (`URBS-TERRACE-CRAFT-03`)
  for (const [ox, oy] of [
    [cx - forumR, cy - forumR],
    [cx + forumR, cy - forumR],
    [cx + forumR, cy + forumR],
    [cx - forumR, cy + forumR],
  ] as const) {
    const obPt = worldPx(layout, ox, oy);
    const px = obPt.px;
    const py = obPt.py - elev;
    // Soft directional ground shadow
    ctx.fillStyle = 'rgba(10, 8, 6, 0.42)';
    ctx.beginPath();
    ctx.ellipse(px + 3.2, py + 1.2, 4.4, 2.0, 0, 0, Math.PI * 2);
    ctx.fill();
    // Moulded square travertine plinth
    ctx.fillStyle = '#b8aca0';
    ctx.fillRect(px - 2.4, py - 2.2, 4.8, 2.4);
    // Cylindrical marble column shaft
    ctx.fillStyle = '#ebe3d5';
    ctx.strokeStyle = 'rgba(28, 22, 18, 0.42)';
    ctx.lineWidth = 0.65;
    ctx.fillRect(px - 1.5, py - 9.4, 3.0, 7.4);
    ctx.strokeRect(px - 1.5, py - 9.4, 3.0, 7.4);
    // Sunlit gilded Corinthian capital & finial orb
    ctx.fillStyle = '#fbbf24';
    ctx.fillRect(px - 2.1, py - 10.6, 4.2, 1.4);
    ctx.beginPath();
    ctx.arc(px, py - 11.4, 1.25, 0, Math.PI * 2);
    ctx.fill();
  }
}

function paintFullBleedRomanLandscape(ctx: CanvasRenderingContext2D, layout: TownLayout, lens: UrbsLens) {
  const b = worldBounds(layout);
  const w = b.maxX;
  const h = b.maxY;
  const pbr = ensurePbrMaterialAtlas(ctx);

  // 1. Translucent Plate 2 Golden-Hour Atmospheric Glaze (`ROMA-SIM-DIORAMA-02`)
  //    Preserves the underlying high-resolution `urbs-diorama-bg.jpg` Roman city artwork
  //    instead of obliterating it with an opaque dark-blue rectangle!
  if (typeof ctx.createRadialGradient === 'function') {
    const seaCx = w * 0.52;
    const seaCy = h * 0.50;
    const seaGrad = ctx.createRadialGradient(seaCx, seaCy, Math.min(w, h) * 0.08, seaCx, seaCy, Math.max(w, h) * 0.68);
    seaGrad.addColorStop(0, 'rgba(250, 246, 238, 0.04)');
    seaGrad.addColorStop(0.50, 'rgba(245, 239, 226, 0.06)');
    seaGrad.addColorStop(1, 'rgba(212, 175, 55, 0.08)');
    ctx.fillStyle = seaGrad;
    ctx.fillRect(0, 0, w, h);
  }

  // Phase 295 (`URBS-EDGE-PBR-03`): Full-Canvas Diagonal Golden-Hour Sun Swell Ribbons
  if (typeof ctx.createLinearGradient === 'function') {
    ctx.save();
    const swellGrad = ctx.createLinearGradient(0, h * 0.12, w, h * 0.88);
    swellGrad.addColorStop(0, 'rgba(212, 175, 55, 0.0)');
    swellGrad.addColorStop(0.28, 'rgba(212, 175, 55, 0.05)');
    swellGrad.addColorStop(0.52, 'rgba(251, 191, 36, 0.06)');
    swellGrad.addColorStop(0.76, 'rgba(180, 83, 9, 0.05)');
    swellGrad.addColorStop(1, 'rgba(18, 13, 8, 0.0)');
    ctx.fillStyle = swellGrad;
    ctx.fillRect(0, 0, w, h);
    ctx.restore();
  }

  // 2. Draw Each District's 3D Elevated Terrace, Sculpted Coastal Rock Cliffs & Paved Plazas
  //    (No artificial giant rectangular tray — each district rises organically on rocky cliffs!)
  const sortedDistricts = [...layout.districts].sort(
    (a, b) => (a.x0 + a.x1 + a.y0 + a.y1) - (b.x0 + b.x1 + b.y0 + b.y1),
  );

  for (const d of sortedDistricts) {
    const pad = 0.08;
    const pTop = worldPx(layout, d.x0 + pad, d.y0 + pad);
    const pRight = worldPx(layout, d.x1 - pad, d.y0 + pad);
    const pBot = worldPx(layout, d.x1 - pad, d.y1 - pad);
    const pLeft = worldPx(layout, d.x0 + pad, d.y1 - pad);

    const elev = districtElevationPx(d);
    const cliffH = 26 + elev;

    const tTop: [number, number] = [pTop.px, pTop.py - elev];
    const tRight: [number, number] = [pRight.px, pRight.py - elev];
    const tBot: [number, number] = [pBot.px, pBot.py - elev];
    const tLeft: [number, number] = [pLeft.px, pLeft.py - elev];

    // Phase 295 (`URBS-EDGE-PBR-01`): Feathered Elliptical Radial Travertine Base Shadow & Torch Halo
    const reefCx = (tLeft[0] + tRight[0]) * 0.5;
    const reefCy = (tTop[1] + tBot[1]) * 0.5 + cliffH * 0.62;
    const reefRx = Math.max(48, (tRight[0] - tLeft[0]) * 0.66);
    const reefRy = Math.max(32, (tBot[1] - tTop[1]) * 0.66);

    if (typeof ctx.createRadialGradient === 'function') {
      ctx.save();
      ctx.translate(reefCx, reefCy);
      ctx.scale(1, reefRy / reefRx);
      const reefGrad = ctx.createRadialGradient(0, 0, reefRx * 0.18, 0, 0, reefRx * 1.18);
      reefGrad.addColorStop(0, 'rgba(12, 9, 6, 0.52)');
      reefGrad.addColorStop(0.54, 'rgba(42, 31, 18, 0.26)');
      reefGrad.addColorStop(0.82, 'rgba(212, 175, 55, 0.10)');
      reefGrad.addColorStop(1, 'rgba(18, 13, 8, 0.0)');
      ctx.fillStyle = reefGrad;
      ctx.beginPath();
      ctx.arc(0, 0, reefRx * 1.18, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    // Lower Faceted Rock Cliff Strata (SW shadowed & SE sunlit cliff walls — translucent over citadel art) (`URBS-TERRAIN-03`)
    const swCliffPts: [number, number][] = [
      tLeft,
      tBot,
      [tBot[0], tBot[1] + cliffH],
      [tLeft[0], tLeft[1] + cliffH],
    ];
    ctx.save();
    ctx.globalAlpha = 0.34;
    fillPoly(
      ctx,
      swCliffPts,
      createVerticalWallGradient(ctx, tLeft[1], tBot[1] + cliffH, '#786b5d', false),
      'rgba(28, 22, 16, 0.45)',
      0.75,
    );
    ctx.restore();

    const seCliffPts: [number, number][] = [
      tBot,
      tRight,
      [tRight[0], tRight[1] + cliffH],
      [tBot[0], tBot[1] + cliffH],
    ];
    ctx.save();
    ctx.globalAlpha = 0.36;
    fillPoly(
      ctx,
      seCliffPts,
      createVerticalWallGradient(ctx, tRight[1], tBot[1] + cliffH, '#b8a896', true),
      'rgba(28, 22, 16, 0.45)',
      0.75,
    );
    ctx.restore();

    // Craggy 3D Rock Boulders along the cliff base (Concept Art A)
    drawIsoRockOutcrop(ctx, (tLeft[0] + tBot[0]) / 2, (tLeft[1] + tBot[1]) / 2 + cliffH - 4, 24, 16);
    drawIsoRockOutcrop(ctx, (tBot[0] + tRight[0]) / 2, (tBot[1] + tRight[1]) / 2 + cliffH - 4, 26, 18);

    // Chiseled Ashlar Retaining Wall Band at the top of the cliff
    const retainH = Math.min(15, cliffH * 0.42);
    const swRetainPts: [number, number][] = [
      tLeft,
      tBot,
      [tBot[0], tBot[1] + retainH],
      [tLeft[0], tLeft[1] + retainH],
    ];
    ctx.save();
    ctx.globalAlpha = 0.42;
    fillPoly(
      ctx,
      swRetainPts,
      createVerticalWallGradient(ctx, tLeft[1], tBot[1] + retainH, '#9c8e7e', false),
      'rgba(28, 22, 16, 0.45)',
      0.75,
    );
    ctx.restore();

    const seRetainPts: [number, number][] = [
      tBot,
      tRight,
      [tRight[0], tRight[1] + retainH],
      [tBot[0], tBot[1] + retainH],
    ];
    ctx.save();
    ctx.globalAlpha = 0.45;
    fillPoly(
      ctx,
      seRetainPts,
      createVerticalWallGradient(ctx, tRight[1], tBot[1] + retainH, '#dcd0c0', true),
      'rgba(28, 22, 16, 0.45)',
      0.75,
    );
    ctx.restore();

    // Phase 290 (`URBS-GROUNDS-03`): Vaulted `Substructio` Blind Arcades & Buttress Pilasters on Cliff Walls
    drawSubstructioCliffArcades(ctx, tLeft, tBot, retainH, false);
    drawSubstructioCliffArcades(ctx, tBot, tRight, retainH, true);

    // Phase 296 (`URBS-TERRACE-CRAFT-04`): 3D Overhanging Travertine Cliff Cornice (`Geison`) + Under-Ledge Drip Shadow
    strokeLine(ctx, [tLeft[0], tLeft[1] + 2.2], [tBot[0], tBot[1] + 2.2], 'rgba(8, 6, 5, 0.42)', 2.0);
    strokeLine(ctx, [tBot[0], tBot[1] + 2.2], [tRight[0], tRight[1] + 2.2], 'rgba(8, 6, 5, 0.38)', 2.0);

    // Lens-Responsive Sunlit Carrara & Travertine Terrace Surface (`URBS-TERRAIN-03`)
    const layerIdx = d.layer ?? 1;
    let terraceFill = '#d8cbb8';
    let borderStroke = d.landmark ? 'rgba(245, 158, 11, 0.85)' : 'rgba(212, 175, 55, 0.72)';
    if (lens === 'architecture') {
      terraceFill = ['#c9b99f', '#ddd0be', '#e8dcc8', '#f2e8d9'][layerIdx] ?? '#ddd0be';
      borderStroke = ['#d4af37', '#e6dcb8', '#fbbf24', '#f59e0b'][layerIdx] ?? '#fbbf24';
    } else if (lens === 'cost') {
      const loc = d.totalLines ?? 1000;
      terraceFill = loc > 25000 ? '#7a342b' : loc > 10000 ? '#78552b' : '#2d5940';
      borderStroke = loc > 25000 ? '#f87171' : loc > 10000 ? '#fbbf24' : '#34d399';
    } else if (lens === 'health') {
      terraceFill = '#2f523a';
      borderStroke = '#34d399';
    }

    ctx.save();
    ctx.globalAlpha = 0.32;
    const terraceGrad = createVerticalWallGradient(ctx, tTop[1], tBot[1], terraceFill, true);
    fillPoly(ctx, [tTop, tRight, tBot, tLeft], terraceGrad, borderStroke, d.landmark ? 1.5 : 1.1);
    ctx.restore();
    // Moulded 3D travertine coping ledge along the front-right & front-left terrace rim (`Geison`)
    strokeLine(ctx, tBot, tRight, 'rgba(254, 243, 199, 0.64)', 1.65);
    strokeLine(ctx, tLeft, tBot, 'rgba(231, 222, 209, 0.42)', 1.35);

    // Phase 290/296 (`URBS-GROUNDS-01..02, 04`, `URBS-TERRACE-CRAFT-02..03`): Seamless Streets, Opus Sectile Plaza & Columns
    drawDistrictGroundDetails(ctx, layout, d, elev);

    // Carved Grand Stone Staircase on elevated terraces
    if (elev > 0) {
      const stairX = (tLeft[0] + tBot[0]) / 2;
      const stairY = (tLeft[1] + tBot[1]) / 2;
      drawIsoGrandStaircase(ctx, stairX, stairY, 0, 0, cliffH * 0.55, 'sw');
    }
  }

  // 3. Orthogonal Isometric Aqueduct Canals, Multi-Arch Viaducts & Golden-Emerald Cascades (Plate 2)
  for (const d of sortedDistricts) {
    if ((d.totalLines ?? 2000) < 1200 && !d.landmark) continue;
    const elev = districtElevationPx(d);
    const canalStart = worldPx(layout, d.x0, d.y1);
    const canalMid = worldPx(layout, d.x1, d.y1);
    const canalEnd = worldPx(layout, d.x1, d.y0);
    const pA = { px: canalStart.px, py: canalStart.py - elev * 0.5 + 4 };
    const pB = { px: canalMid.px, py: canalMid.py - elev * 0.5 + 4 };
    const pC = { px: canalEnd.px, py: canalEnd.py - elev * 0.5 + 4 };

    if ((d.layer ?? 1) >= 2) {
      drawIsoViaductBridge(ctx, pB, pC, false);
    } else {
      strokeLine(ctx, [pA.px, pA.py], [pB.px, pB.py], 'rgba(212, 175, 55, 0.42)', 7);
      strokeLine(ctx, [pA.px, pA.py], [pB.px, pB.py], '#292524', 4.8);
      strokeLine(ctx, [pA.px, pA.py], [pB.px, pB.py], '#d4af37', 2.8);
      strokeLine(ctx, [pA.px, pA.py], [pB.px, pB.py], '#fef3c7', 1.2);
    }

    // Cascading golden-emerald waterfall over the cliff edge
    if ((d.layer ?? 1) === 0 || d.x0 === 0) {
      const wfTop: [number, number] = [pA.px, pA.py];
      const wfBot: [number, number] = [pA.px - 4, pA.py + 36];
      strokeLine(ctx, wfTop, wfBot, 'rgba(212, 175, 55, 0.52)', 8);
      strokeLine(ctx, wfTop, wfBot, '#d4af37', 4.0);
      strokeLine(ctx, wfTop, wfBot, '#fef3c7', 1.6);
      ctx.fillStyle = '#fde68a';
      ctx.beginPath();
      ctx.ellipse(wfBot[0], wfBot[1], 13, 5.5, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = 'rgba(254, 243, 199, 0.55)';
      ctx.lineWidth = 1.1;
      ctx.beginPath();
      ctx.ellipse(wfBot[0], wfBot[1], 19, 8, 0, 0, Math.PI * 2);
      ctx.stroke();
    }
  }
}

// ── 5. Peripheral Harness Landmarks (AQVAE, CASTRVM, PORTVS, PORTA) ─────────

function drawAqueductComplex(ctx: CanvasRenderingContext2D, px: number, py: number, isLit: boolean) {
  // 3D isometric masonry piers & arched spandrels (`URBS-ARCH-REALISM-04` / `URBS-PLUMBING-3D-01`)
  const spans = 6;
  const dx = 24;
  const dy = 12;
  const startX = px - 56;
  const startY = py - 24;

  for (let i = 0; i < spans; i++) {
    const ax = startX + i * dx;
    const ay = startY + i * dy;
    // Soft-shaded SW / SE pier faces (no cartoon ink outline)
    ctx.fillStyle = '#4e463e';
    ctx.fillRect(ax - 4, ay - 34, 4, 34);
    ctx.fillStyle = '#85796c';
    ctx.fillRect(ax, ay - 34, 4, 34);
    strokeLine(ctx, [ax - 4, ay - 34], [ax + 4, ay - 34], 'rgba(245, 235, 214, 0.42)', 1.0);

    fillPoly(
      ctx,
      [
        [ax + 3, ay - 34],
        [ax + dx - 3, ay + dy - 34],
        [ax + dx - 3, ay + dy - 16],
        [ax + 3, ay - 16],
      ],
      createVerticalWallGradient(ctx, ay - 34, ay + dy - 16, '#94887a', true),
      'rgba(28, 22, 18, 0.36)',
      0.7,
    );

    ctx.fillStyle = isLit ? 'rgba(212, 175, 55, 0.45)' : 'rgba(28, 25, 23, 0.72)';
    ctx.beginPath();
    ctx.arc(ax + dx * 0.5, ay + dy * 0.5 - 16, 6, Math.PI, 0);
    ctx.fill();
  }

  const crestStart: [number, number] = [startX - 4, startY - 36];
  const crestEnd: [number, number] = [startX + spans * dx, startY + spans * dy - 36];
  strokeLine(ctx, crestStart, crestEnd, '#63594f', 6);
  strokeLine(ctx, crestStart, crestEnd, 'rgba(254, 243, 199, 0.35)', 1.4);
  if (isLit) {
    strokeLine(ctx, crestStart, crestEnd, '#d4af37', 3.2);
    strokeLine(ctx, crestStart, crestEnd, '#fef3c7', 1.2);
  }
}

function drawCastrumFortressComplex(ctx: CanvasRenderingContext2D, px: number, py: number, runnerCount: number, isLit: boolean) {
  drawIsoBox(ctx, px, py, -0.85, -0.75, 0, 0.85, 0.75, 16, {
    swFill: '#474039',
    seFill: '#786d61',
    topFill: '#5c534a',
    courses: 3,
  });
  // Corner watchtowers
  for (const [u, v] of [
    [-0.75, -0.65],
    [0.65, -0.65],
    [0.65, 0.55],
    [-0.75, 0.55],
  ] as const) {
    drawIsoBox(ctx, px, py, u - 0.14, v - 0.14, 16, u + 0.14, v + 0.14, 28, {
      swFill: '#4e463e',
      seFill: '#85796c',
      topFill: '#695f55',
      aoShadow: false,
    });
    drawIsoPitchedRoof(ctx, px, py, u - 0.14, v - 0.14, u + 0.14, v + 0.14, 28, 34, 'u');
  }
  // Legionary tents inside the fortress
  const tents = Math.min(4, Math.max(2, runnerCount));
  for (let i = 0; i < tents; i++) {
    const u = -0.35 + (i % 2) * 0.55;
    const v = -0.25 + Math.floor(i / 2) * 0.45;
    drawIsoPitchedRoof(ctx, px, py, u - 0.16, v - 0.14, u + 0.16, v + 0.14, 16, 24, 'u', {
      roofLit: isLit ? '#b91c1c' : '#57534e',
      roofShade: isLit ? '#7f1d1d' : '#44403c',
    });
  }
}

function drawPortusHarborComplex(ctx: CanvasRenderingContext2D, px: number, py: number, taskCount: number) {
  // Stone harbor mole & Pharos lighthouse
  drawIsoBox(ctx, px, py, -0.55, -0.45, 0, 0.35, 0.35, 10, {
    swFill: '#474039',
    seFill: '#756a5e',
    topFill: '#635a50',
  });
  // Tiered Pharos Lighthouse
  drawIsoBox(ctx, px, py, -0.35, -0.30, 10, 0.05, 0.05, 28, {
    swFill: '#544c44',
    seFill: '#8c8072',
    topFill: '#73685d',
    aoShadow: false,
  });
  drawIsoBox(ctx, px, py, -0.26, -0.22, 28, -0.02, -0.02, 42, {
    swFill: '#61584f',
    seFill: '#9e9182',
    topFill: '#827669',
    aoShadow: false,
  });
  const beaconPt = isoPt(px, py, -0.14, -0.12, 46);
  drawIsoTorchBrazier(ctx, beaconPt[0], beaconPt[1]);

  // Moored Roman merchant galleys in the harbor basin
  const ships = Math.min(3, Math.max(1, Math.ceil(taskCount / 12)));
  for (let s = 0; s < ships; s++) {
    const sx = px - 28 + s * 22;
    const sy = py + 24 + s * 8;
    fillPoly(
      ctx,
      [
        [sx - 12, sy],
        [sx + 12, sy],
        [sx + 8, sy + 5],
        [sx - 8, sy + 5],
      ],
      '#5c341d',
      'rgba(28, 22, 18, 0.42)',
      0.9,
    );
    strokeLine(ctx, [sx, sy], [sx, sy - 14], '#d6d3d1', 1.0);
    fillPoly(
      ctx,
      [
        [sx - 8, sy - 12],
        [sx + 8, sy - 10],
        [sx + 7, sy - 3],
        [sx - 7, sy - 5],
      ],
      s % 2 === 0 ? '#b91c1c' : '#d97706',
      'rgba(28, 22, 18, 0.38)',
      0.7,
    );
  }
}

// ── 6. Concept Art B Micro-HUD Badges & Callout Pills ───────────────────────

function drawRomanCalloutPill(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  title: string,
  metric: string,
  borderColor: string,
  textColor: string,
) {
  ctx.save();
  ctx.font = 'bold 9.5px Cinzel, Georgia, serif';
  const fullText = `${title} · ${metric}`;
  const tw = ctx.measureText(fullText).width;
  const pw = tw + 18;
  const ph = 18;
  const rx = x - pw / 2;
  const ry = y - ph / 2;

  ctx.fillStyle = 'rgba(18, 15, 12, 0.90)';
  ctx.strokeStyle = borderColor;
  ctx.lineWidth = 1.1;
  ctx.beginPath();
  ctx.roundRect(rx, ry, pw, ph, 5);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = textColor;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(fullText, x, y + 0.5);
  ctx.restore();
}

function drawWorksiteFileDiffSlab(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  basename: string,
  gitStatus: string,
  added: number,
  deleted: number,
  hasError: boolean,
  agentRole?: string,
) {
  ctx.save();
  const statusTag = gitStatus || 'M';
  const diffLabel = `+${added}/-${deleted}`;
  const headerText = `${statusTag} ${basename}`;
  const subText = agentRole ? `⚙ ${agentRole} · ${diffLabel}` : diffLabel;

  ctx.font = "bold 8.5px 'Palatino', 'Palatino Linotype', Georgia, serif";
  const w1 = ctx.measureText(headerText).width;
  const w2 = ctx.measureText(subText).width;
  const pw = Math.max(84, Math.max(w1, w2) + 16);
  const ph = 26;
  const rx = x - pw / 2;
  const ry = y - ph / 2;

  ctx.fillStyle = 'rgba(250, 245, 232, 0.94)';
  ctx.strokeStyle = hasError
    ? 'rgba(185, 28, 28, 0.92)'
    : 'rgba(197, 155, 39, 0.92)';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.roundRect(rx, ry, pw, ph, 4);
  ctx.fill();
  ctx.stroke();

  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = hasError ? '#991b1b' : '#1C140C';
  ctx.fillText(headerText, rx + 6, ry + 8.5);

  ctx.font = "600 8px 'Palatino', 'Palatino Linotype', Georgia, serif";
  ctx.fillStyle = '#5C4428';
  ctx.fillText(subText, rx + 6, ry + 18.5);
  ctx.restore();
}

const SEMIOTIC_ARCHETYPE_BY_LAYER: Record<number, { glyph: string; title: string; pillBg: string; pillBorder: string }> = {
  0: { glyph: '🏛', title: 'VAULT', pillBg: '#EFE7D6', pillBorder: '#B38728' },
  1: { glyph: '🏛', title: 'ACROPOLIS', pillBg: '#F5EFE4', pillBorder: '#D4AF37' },
  2: { glyph: '🏺', title: 'ROTUNDA', pillBg: '#EFE7D6', pillBorder: '#C59B27' },
  3: { glyph: '⚖', title: 'PRAETORIVM', pillBg: '#F3E5AB', pillBorder: '#D4AF37' },
  4: { glyph: '🔥', title: 'THERMAE', pillBg: '#EFE7D6', pillBorder: '#D97706' },
  5: { glyph: '🏟', title: 'COLOSSEVM', pillBg: '#F5EFE4', pillBorder: '#B38728' },
};

function drawDistrictMicroHudBadge(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  d: District,
  lens: UrbsLens,
  warnCount: number,
  errCount: number,
  isActive: boolean,
  blastMeta?: { role?: 'provider' | 'impacted' | 'origin'; downstreamCount?: number },
) {
  ctx.save();
  const title = d.name;
  const locK = ((d.totalLines ?? 1200) / 1000).toFixed(1);
  const layerNum = Math.min(5, Math.max(0, d.layer ?? 1));
  const layerLabel = `L${layerNum}`;
  const archetype = SEMIOTIC_ARCHETYPE_BY_LAYER[layerNum] ?? SEMIOTIC_ARCHETYPE_BY_LAYER[1];
  const healthPct = Math.max(60, 100 - errCount * 15 - warnCount * 4);
  const addedSum = d.buildings.reduce((s, b) => s + (b.diffAdded ?? 0), 0);
  const deletedSum = d.buildings.reduce((s, b) => s + (b.diffDeleted ?? 0), 0);
  const churnLabel = addedSum > 0 || deletedSum > 0 ? `+${addedSum}/-${deletedSum}` : 'clean';
  const costRate = ((d.buildings.length * 0.005) + (isActive ? 0.02 : 0)).toFixed(2);

  let line1 = `${archetype.glyph} ${title} · ${layerLabel} ${archetype.title} (${locK}k LoC)`;
  if (blastMeta?.role === 'impacted') {
    line1 = `⚡ ${title} · ${layerLabel} BLAST RADIUS`;
  } else if (blastMeta?.role === 'provider') {
    line1 = `⬆ ${title} · ${layerLabel} UPSTREAM`;
  }

  let line2 =
    errCount > 0
      ? `▲ ${errCount} ERR · ✓ ${healthPct}% · ${churnLabel}`
      : isActive
        ? `◆ LLM STREAMING · ✓ ${healthPct}% · ${churnLabel} · $${costRate}/m`
        : `✓ ${healthPct}% check · ${churnLabel} · $${costRate}/m`;
  if (blastMeta?.role === 'origin' && (blastMeta.downstreamCount ?? 0) > 0) {
    line2 =
      errCount > 0
        ? `▲ ${errCount} ERR · Blast Radius: ${blastMeta.downstreamCount} downstream · ${churnLabel}`
        : `Blast Radius: ${blastMeta.downstreamCount} downstream · ${churnLabel}`;
  } else if (lens === 'architecture') {
    line2 = `${layerLabel} ${archetype.title} · Fan-in ${d.fanIn ?? 0} · Out ${d.deps?.length ?? 0}`;
  } else if (lens === 'cost') {
    line2 = `BURN $${costRate}/min · ${isActive ? 'Frontier Cloud' : 'MENS Local'}`;
  }

  ctx.font = 'bold 9px Cinzel, Palatino, Georgia, serif';
  const w1 = ctx.measureText(line1).width;
  ctx.font = "600 8px 'Palatino', 'Palatino Linotype', Georgia, serif";
  const w2 = ctx.measureText(line2).width;

  const pw = Math.max(w1, w2) + 22;
  const ph = 30;
  const rx = x - pw / 2;
  const ry = y - ph / 2;

  // Chiseled White/Cream Carrara Marble & Burnished Gold Semiotic Architrave Plaque (`URBS-SEMIOTIC-V64`)
  ctx.fillStyle = 'rgba(250, 245, 232, 0.95)';
  ctx.strokeStyle =
    errCount > 0
      ? 'rgba(185, 28, 28, 0.95)'
      : blastMeta?.role === 'impacted'
        ? 'rgba(180, 83, 9, 0.95)'
        : 'rgba(197, 155, 39, 0.95)';
  ctx.lineWidth = 1.35;
  ctx.beginPath();
  ctx.roundRect(rx, ry, pw, ph, 4);
  ctx.fill();
  ctx.stroke();

  // Top Carrara marble frieze strip inside plaque header
  ctx.fillStyle = 'rgba(212, 175, 55, 0.14)';
  ctx.fillRect(rx + 2, ry + 2, pw - 4, 12);

  // Top sunlit bronze architrave bevel + 4 corner rivets
  strokeLine(ctx, [rx + 3, ry + 1.2], [rx + pw - 3, ry + 1.2], 'rgba(197, 155, 39, 0.65)', 0.9);
  ctx.fillStyle = '#B8860B';
  for (const [cxR, cyR] of [
    [rx + 3.5, ry + 3.5],
    [rx + pw - 3.5, ry + 3.5],
    [rx + 3.5, ry + ph - 3.5],
    [rx + pw - 3.5, ry + ph - 3.5],
  ] as const) {
    ctx.beginPath();
    ctx.arc(cxR, cyR, 1.2, 0, Math.PI * 2);
    ctx.fill();
  }

  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.font = 'bold 9px Cinzel, Palatino, Georgia, serif';
  ctx.fillStyle = '#1C140C';
  ctx.fillText(line1, x, ry + 9.5);

  ctx.font = "600 8px 'Palatino', 'Palatino Linotype', Georgia, serif";
  ctx.fillStyle =
    errCount > 0
      ? '#991b1b'
      : blastMeta?.role === 'impacted'
        ? '#92400e'
        : '#5C4428';
  ctx.fillText(line2, x, ry + 21);
  ctx.restore();
}

// ── 7. WorldRenderer Class ──────────────────────────────────────────────────

export class WorldRenderer {
  private buffer: Buffer | null = null;
  private atlas: { canvas: HTMLCanvasElement; plan: AtlasPlan } | null = null;

  ensure(s: WorldState, zoom: number): { canvas: HTMLCanvasElement; scale: number } {
    const band = bandForZoom(zoom);
    const key = redrawKey(s, band);
    if (this.buffer && this.buffer.key === key) {
      return { canvas: this.buffer.canvas, scale: this.buffer.scale };
    }
    if (!this.atlas) this.atlas = buildAtlas(2);
    const b = worldBounds(s.layout);
    const scale = Math.min(1, MAX_BUFFER_EDGE / Math.max(b.maxX, b.maxY));
    const canvas = this.buffer?.canvas ?? document.createElement('canvas');
    canvas.width = Math.ceil(b.maxX * scale);
    canvas.height = Math.ceil(b.maxY * scale);
    const ctx = canvas.getContext('2d');
    if (!ctx) return { canvas, scale };
    ctx.setTransform(scale, 0, 0, scale, 0, 0);
    this.paint(ctx, s, band);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    this.buffer = { canvas, key, scale };
    return { canvas, scale };
  }

  drawFires(ctx: CanvasRenderingContext2D, s: WorldState, zoom: number, frame: 0 | 1): void {
    if (!this.atlas) this.atlas = buildAtlas(2);
    const key: SpriteKey = frame ? 'fire-1' : 'fire-0';
    const { layout } = s;
    if (bandForZoom(zoom) === 0) {
      for (const d of layout.districts) {
        const errs = d.buildings.reduce((n, b) => n + (s.buildings[b.path]?.errors ?? 0), 0);
        if (errs === 0) continue;
        const elev = districtElevationPx(d);
        const { px, py } = worldPx(layout, (d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
        this.stamp(ctx, key, px, py - elev - 30);
      }
    } else {
      for (const [path, diag] of Object.entries(s.buildings)) {
        if (!diag.errors) continue;
        const plot = layout.byPath[path];
        if (!plot) continue;
        const parentD = layout.districts.find((d) => plot.x >= d.x0 && plot.x < d.x1 && plot.y >= d.y0 && plot.y < d.y1);
        const elev = parentD ? districtElevationPx(parentD) : 0;
        const { px, py } = worldPx(layout, plot.x, plot.y);
        this.stamp(ctx, key, px, py - elev - 20);
      }
    }
  }

  drawAnimatedWorldEffects(ctx: CanvasRenderingContext2D, s: WorldState, timeMs: number): void {
    const { layout, harness } = s;
    const L = layout.landmarks;
    ctx.save();

    layout.districts.forEach((d, idx) => {
      const elev = districtElevationPx(d);
      const center = worldPx(layout, (d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
      const pulse = 0.04 + 0.02 * Math.sin(timeMs * 0.0065 + idx * 1.4);
      const radius = 36 + 3 * Math.sin(timeMs * 0.009 + idx * 2.1);

      // Phase 289 (`URBS-SMOOTH-LIGHT-03`): Feathered multi-stop radial torch shimmer (zero hard circle edge)
      if (typeof ctx.createRadialGradient === 'function') {
        const g = ctx.createRadialGradient(center.px, center.py - elev + 4, 2, center.px, center.py - elev + 4, radius);
        g.addColorStop(0, `rgba(251, 191, 36, ${pulse.toFixed(3)})`);
        g.addColorStop(0.55, `rgba(245, 158, 11, ${(pulse * 0.42).toFixed(3)})`);
        g.addColorStop(1, 'rgba(245, 158, 11, 0)');
        ctx.fillStyle = g;
      } else {
        ctx.fillStyle = `rgba(251, 191, 36, ${(pulse * 0.45).toFixed(3)})`;
      }
      ctx.beginPath();
      ctx.ellipse(center.px, center.py - elev + 4, radius, radius * 0.52, 0, 0, Math.PI * 2);
      ctx.fill();

      // Phase 281 (`URBS-FX-01..03`): Live Forge Embers, Volumetric Smoke & Mason Chisel Sparks
      const errCount = d.buildings.reduce((n, b) => n + (s.buildings[b.path]?.errors ?? 0), 0);
      if (d.landmark || d.archetype === 'acropolis_engine' || errCount > 0) {
        drawForgeAndFireEmbers(ctx, center.px, center.py - elev - 18, timeMs, idx + 1, errCount > 0 ? 5 : 3);
        drawVolumetricSmokePlumes(ctx, center.px + 6, center.py - elev - 26, timeMs, idx + 1, errCount > 0);
      }

      for (const b of d.buildings) {
        if (b.isTouched || (b.diffAdded ?? 0) > 0) {
          const bPt = worldPx(layout, b.x, b.y);
          drawMasonChiselSparks(ctx, bPt.px + 8, bPt.py - elev - 18, timeMs, idx * 13 + b.x);
        }
      }

      if ((d.layer ?? 1) === 0 || d.x0 === 0) {
        const wfPt = worldPx(layout, d.x0, d.y1);
        drawWaterCausticsAndSpray(ctx, wfPt.px - 4, wfPt.py - elev * 0.5 + 40, timeMs, idx + 77);
      }
    });

    // Phase 384 (`URBS-DIEGETIC-ENV-02`): Diegetic watercourses & crowds are rendered in panorama space
    // (`diegeticEnvironment.ts`) aligned to the painted Aqueduct, Nymphaeum, and Grand Canal rather than
    // drawing straight grid-to-grid lines across rooftops.

    if (harness.mcp && harness.mcp.length > 0) {
      const { px, py } = worldPx(layout, L.aqvae.x, L.aqvae.y);
      const startX = px - 56;
      const startY = py - 60;
      const dx = 144;
      const dy = 72;
      for (let k = 0; k < 5; k++) {
        const t = ((timeMs * 0.0005 + k * 0.2) % 1 + 1) % 1;
        const wx = startX + dx * t;
        const wy = startY + dy * t;
        ctx.fillStyle = 'rgba(165, 243, 252, 0.35)';
        ctx.beginPath();
        ctx.arc(wx, wy, 4.5, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    if (harness.queueLen !== null) {
      const { px, py } = worldPx(layout, L.portus.x, L.portus.y);
      const ripplePhase = ((timeMs * 0.00065) % 1 + 1) % 1;
      const alpha = 0.45 * (1 - ripplePhase);
      ctx.strokeStyle = `rgba(56, 189, 248, ${alpha.toFixed(3)})`;
      ctx.lineWidth = 1.4;
      ctx.beginPath();
      ctx.ellipse(px - 20, py + 26, 16 + ripplePhase * 28, 6 + ripplePhase * 10, 0, 0, Math.PI * 2);
      ctx.stroke();
    }

    ctx.restore();
  }

  private stamp(ctx: CanvasRenderingContext2D, key: SpriteKey, px: number, py: number) {
    const { canvas, plan } = this.atlas!;
    const r = plan.rects[key];
    ctx.drawImage(canvas, r.x, r.y, r.w, r.h, px - r.ax / 2, py - r.ay / 2, r.w / 2, r.h / 2);
  }

  private paintDistrictWonder(ctx: CanvasRenderingContext2D, d: District, px: number, py: number, s: WorldState) {
    const isSmallLeaf = (d.totalLines ?? 2000) < 1200 || d.buildings.length <= 3;
    const errCount = d.buildings.reduce((n, b) => n + (s.buildings[b.path]?.errors ?? 0), 0);
    const ctxFill = s.harness.contextFillPct ?? 68;
    const arch: DistrictArchetype =
      d.archetype ??
      classifyDistrictArchetype(
        { name: d.name, root: d.name, files: [] },
        {
          layer: d.layer ?? 1,
          fanIn: d.fanIn ?? 0,
          fanOut: d.fanOut ?? (d.deps?.length ?? 0),
          centrality: 2,
        },
      );

    ctx.save();
    ctx.translate(px, py);
    ctx.scale(1.42, 1.42);
    ctx.translate(-px, -py);
    if ((arch === 'watchtower_leaf' || isSmallLeaf) && !d.landmark) {
      drawIsoWatchtowerHamlet(ctx, px, py);
    } else if (arch === 'acropolis_engine') {
      drawIsoPantheonAcropolis(ctx, px, py, true, errCount > 0);
    } else if (arch === 'library_scholar') {
      drawIsoLibraryOfCelsus(ctx, px, py, ctxFill);
    } else if (arch === 'circus_arena') {
      drawIsoCircusMaximus(ctx, px, py, errCount);
    } else if (arch === 'amphitheatre_surface') {
      drawIsoColosseumWonder(ctx, px, py);
    } else if (arch === 'praetorium_hub') {
      drawIsoPraetoriumComplex(ctx, px, py, errCount > 0);
    } else if (arch === 'harbor_vault') {
      drawIsoBedrockHarborVault(ctx, px, py);
    } else {
      drawIsoLibraryOfCelsus(ctx, px, py, ctxFill);
    }
    ctx.restore();
  }

  private paint(ctx: CanvasRenderingContext2D, s: WorldState, _band: 0 | 1) {
    const { layout } = s;
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);

    // Phase 384 (`URBS-DIEGETIC-ENV-01`): No procedural 2D polygon building models or straight rooftop
    // grid-to-grid plumbing lines (`drawRomanHydraulicInfrastructure`) are painted over the high-resolution
    // generated White/Gold Roman Metropolis artwork (`urbs-white-gold-citadel-bg.jpg`).
    drawAdditiveLightPools(ctx, layout, s.buildings, s.agentTasks, worldPx, districtElevationPx);

    const sortedDistricts = [...layout.districts].sort(
      (a, b) => (a.x0 + a.x1 + a.y0 + a.y1) - (b.x0 + b.x1 + b.y0 + b.y1),
    );
    const active = new Set(Object.values(s.agentTasks).map((t) => t.filePath));

    for (const d of sortedDistricts) {
      const elev = districtElevationPx(d);
      const cx = (d.x0 + d.x1) / 2;
      const cy = (d.y0 + d.y1) / 2;
      const rawCenter = worldPx(layout, cx, cy);
      const hasActiveTask = d.buildings.some((b) => active.has(b.path) || b.isTouched);
      const hasError = d.buildings.some((b) => (s.buildings[b.path]?.errors ?? 0) > 0);
      const isSelected = s.selectedDistrict === d.name;

      // Subtle engraved golden astrolabe ring only on selected or error district
      if (isSelected || hasError || hasActiveTask) {
        ctx.save();
        ctx.strokeStyle = hasError
          ? 'rgba(185, 28, 28, 0.55)'
          : 'rgba(212, 175, 55, 0.55)';
        ctx.lineWidth = isSelected ? 1.8 : 1.1;
        ctx.setLineDash([4, 4]);
        ctx.beginPath();
        ctx.ellipse(rawCenter.px, rawCenter.py - elev + 6, 44, 22, 0, 0, Math.PI * 2);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.restore();
      }

      // Wrought-Iron Perimeter Portcullis & Bronze Padlocks when district is quarantined (`URBS-QUARANTINE-03`)
      if (s.quarantinedDistricts?.includes(d.name)) {
        const c00 = worldPx(layout, d.x0, d.y0);
        const c10 = worldPx(layout, d.x1, d.y0);
        const c11 = worldPx(layout, d.x1, d.y1);
        const c01 = worldPx(layout, d.x0, d.y1);
        ctx.save();
        ctx.strokeStyle = 'rgba(217, 119, 6, 0.92)';
        ctx.lineWidth = 2.4;
        ctx.setLineDash([6, 4]);
        ctx.beginPath();
        ctx.moveTo(c00.px, c00.py - elev - 14);
        ctx.lineTo(c10.px, c10.py - elev - 14);
        ctx.lineTo(c11.px, c11.py - elev - 14);
        ctx.lineTo(c01.px, c01.py - elev - 14);
        ctx.closePath();
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.restore();
      }
    }

    drawVolumetricGodRays(ctx, layout, worldPx, districtElevationPx);
  }

  /** Renders District Micro-HUD Badges, Foreground Worksite Diff Slabs, Blast-Radius Illumination,
   *  Landmark Callouts & Tactical Dispatch Vectors at 1:1 native screen resolution (after the camera blit)
   *  so text and vector badges remain razor-sharp at every zoom level. */
  drawScreenSpaceOverlays(
    ctx: CanvasRenderingContext2D,
    s: WorldState,
    cam: { x: number; y: number; zoom: number },
  ): void {
    const { layout, harness } = s;
    const lens: UrbsLens = s.lens ?? 'agents';
    const active = new Set(Object.values(s.agentTasks).map((t) => t.filePath));
    const agentRoleByPath = new Map<string, string>();
    for (const [agentId, t] of Object.entries(s.agentTasks)) {
      agentRoleByPath.set(t.filePath, agentId);
    }

    const toScreen = (wx: number, wy: number): [number, number] => [
      wx * cam.zoom + cam.x,
      wy * cam.zoom + cam.y,
    ];

    const sortedDistricts = [...layout.districts].sort(
      (a, b) => (a.x0 + a.x1 + a.y0 + a.y1) - (b.x0 + b.x1 + b.y0 + b.y1),
    );

    // Determine primary focus district for Blast-Radius illumination (selected district or highest-churn/active district)
    let focusDistrictName = s.selectedDistrict ?? null;
    if (!focusDistrictName) {
      const dirtyOrActive = sortedDistricts.find(
        (d) =>
          (d.dirtyFileCount ?? 0) > 0 ||
          d.buildings.some((b) => b.isTouched || active.has(b.path) || (s.buildings[b.path]?.errors ?? 0) > 0),
      );
      if (dirtyOrActive) focusDistrictName = dirtyOrActive.name;
    }

    const blastReport = focusDistrictName
      ? computeBlastRadius(focusDistrictName, layout.districts)
      : null;
    const providerSet = new Set(blastReport?.providers ?? []);
    const impactedSet = new Set(blastReport?.impactedDependents ?? []);

    // 0. Phase 294 (`URBS-SEMIOTICS-CV-01..02`): Isometric-Routed Hydraulic Blast-Radius Conduits & Bronze Valve Junctions (legend rendered in React HUD)
    drawScreenSpaceHydraulicOverlay(ctx, s, toScreen, worldPx, districtElevationPx, focusDistrictName, true);

    // 1. District Micro-HUD Badges, Foreground File Diff Slabs & Tactical Dispatch Vectors (with screen-space de-collision)
    const placedBoxes: { x: number; y: number; w: number; h: number }[] = [];
    const decollide = (cx: number, cy: number, w: number, h: number, stepY = -28): [number, number] => {
      let ny = cy;
      for (let attempt = 0; attempt < 5; attempt++) {
        const x0 = cx - w / 2;
        const y0 = ny - h / 2;
        const hit = placedBoxes.some(
          (b) => x0 < b.x + b.w + 4 && x0 + w + 4 > b.x && y0 < b.y + b.h + 4 && y0 + h + 4 > b.y,
        );
        if (!hit) break;
        ny += stepY;
      }
      placedBoxes.push({ x: cx - w / 2, y: ny - h / 2, w, h });
      return [cx, ny];
    };

    let primaryDispatchDrawn = false;
    for (const d of sortedDistricts) {
      const elev = districtElevationPx(d);
      const cx = (d.x0 + d.x1) / 2;
      const cy = (d.y0 + d.y1) / 2;
      const raw = worldPx(layout, cx, cy);
      const wx = raw.px;
      const wy = raw.py - elev;
      // Mount the Chiseled Cliff-Apron Architrave Plaque along the front cliff retaining wall (`URBS-DIEGETIC-01`)
      const frontCliffAnchor = worldPx(layout, (d.x0 + d.x1) / 2, d.y1);
      const [rawSx, rawSy] = toScreen(frontCliffAnchor.px, frontCliffAnchor.py - elev + 16);
      const [sx, sy] = decollide(rawSx, rawSy, 162, 30, 30);

      const warnCount = d.buildings.reduce((n, b) => n + (s.buildings[b.path]?.warnings ?? 0), 0);
      const errCount = d.buildings.reduce((n, b) => n + (s.buildings[b.path]?.errors ?? 0), 0);
      const gateCount = s.needsYouByDistrict?.[d.name] ?? 0;
      const touchedPlots = d.buildings.filter(
        (b) => b.isTouched || active.has(b.path) || (s.buildings[b.path]?.errors ?? 0) > 0,
      );
      const isSelectedOrFocused = s.selectedDistrict === d.name || focusDistrictName === d.name;
      const isActive =
        touchedPlots.length > 0 ||
        (d.dirtyFileCount ?? 0) > 0 ||
        s.selectedDistrict === d.name ||
        d.archetype === 'acropolis_engine';

      const blastMeta =
        focusDistrictName === d.name
          ? { role: 'origin' as const, downstreamCount: blastReport?.impactedDependents.length ?? 0 }
          : impactedSet.has(d.name)
            ? { role: 'impacted' as const }
            : providerSet.has(d.name)
              ? { role: 'provider' as const }
              : undefined;

      drawDistrictMicroHudBadge(ctx, sx, sy, d, lens, warnCount, errCount, isActive, blastMeta);

      // Architecture-First Progressive Disclosure (`URBS-DIEGETIC-02`):
      // Only show floating file diff slabs on the selected/focused district or a building with errors > 0
      const maxSlabs = isSelectedOrFocused ? 2 : errCount > 0 ? 1 : 0;
      for (let k = 0; k < Math.min(maxSlabs, touchedPlots.length); k++) {
        const b = touchedPlots[k]!;
        const bRaw = worldPx(layout, b.x, b.y);
        const [rawBsx, rawBsy] = toScreen(bRaw.px, bRaw.py - elev - 42);
        const [bsx, bsy] = decollide(rawBsx, rawBsy, 118, 28, -28);
        const basename = b.path.split('/').pop() ?? b.path;
        const bErr = (s.buildings[b.path]?.errors ?? 0) > 0;
        drawWorksiteFileDiffSlab(
          ctx,
          bsx,
          bsy,
          basename,
          b.gitStatus ?? 'M',
          b.diffAdded ?? 14,
          b.diffDeleted ?? 3,
          bErr,
          agentRoleByPath.get(b.path),
        );
      }

      // Draw at most ONE primary Tactical Error/Gate Dispatch Vector so the 3D architecture stays unobstructed
      if (!primaryDispatchDrawn && (errCount > 0 || (isSelectedOrFocused && gateCount > 0))) {
        primaryDispatchDrawn = true;
        const [fromSx, fromSy] = toScreen(wx - 44, wy - 28);
        const [toSx, toSy] = toScreen(wx + 34, wy + 18);
        const calloutLabel =
          errCount > 0
            ? `Fix ${errCount} Diagnostic Error${errCount === 1 ? '' : 's'}`
            : `Resolve ${gateCount} Decision Gate${gateCount === 1 ? '' : 's'}`;
        drawTacticalErrorDispatchOverlay(ctx, fromSx, fromSy, toSx, toSy, calloutLabel);
      }

      // Quarantine Padlock Badge (`URBS-QUARANTINE-03`)
      if (s.quarantinedDistricts?.includes(d.name)) {
        const [rawQx, rawQy] = toScreen(wx, wy - 56);
        const [qx, qy] = decollide(rawQx, rawQy, 138, 22, -24);
        drawRomanCalloutPill(
          ctx,
          qx,
          qy,
          '🔒 QUARANTINED',
          'read-only guardrail',
          'rgba(245, 158, 11, 0.92)',
          '#fde68a',
        );
      }
    }

    // 1b. Multi-Agent Write-Set Collision Standards (`URBS-COLLISION-02`)
    for (const col of s.collisionZones ?? []) {
      const colDist = layout.districts.find((d) => d.name === col.districtId);
      if (!colDist) continue;
      const elev = districtElevationPx(colDist);
      const raw = worldPx(
        layout,
        (colDist.x0 + colDist.x1) / 2,
        (colDist.y0 + colDist.y1) / 2,
      );
      const [rawCx, rawCy] = toScreen(raw.px, raw.py - elev - 66);
      const [cx, cy] = decollide(rawCx, rawCy, 178, 24, -26);
      drawRomanCalloutPill(
        ctx,
        cx,
        cy,
        '⚔ WRITE-SET COLLISION',
        `${col.agentALabel} × ${col.agentBLabel}`,
        'rgba(239, 68, 68, 0.94)',
        '#fecaca',
      );
    }

    // 1c. Viaduct Contract & Layer-Inversion Keystones (`URBS-VIADUCT-02`)
    for (const edge of layout.edges ?? []) {
      const isSelectedEdge =
        s.selectedViaductEdge?.from_district_id === edge.from_district_id &&
        s.selectedViaductEdge?.to_district_id === edge.to_district_id;
      if (!edge.layer_violation && (edge.broken_contract_errors ?? 0) === 0 && !isSelectedEdge) {
        continue;
      }
      const src = layout.districts.find((d) => d.name === edge.from_district_id);
      const dst = layout.districts.find((d) => d.name === edge.to_district_id);
      if (!src || !dst) continue;
      const srcRaw = worldPx(layout, (src.x0 + src.x1) / 2, (src.y0 + src.y1) / 2);
      const dstRaw = worldPx(layout, (dst.x0 + dst.x1) / 2, (dst.y0 + dst.y1) / 2);
      const midWx = (srcRaw.px + dstRaw.px) / 2;
      const midWy =
        (srcRaw.py - districtElevationPx(src) + (dstRaw.py - districtElevationPx(dst))) / 2;
      const [rawKx, rawKy] = toScreen(midWx, midWy - 14);
      const [kx, ky] = decollide(rawKx, rawKy, 152, 22, -24);
      drawRomanCalloutPill(
        ctx,
        kx,
        ky,
        edge.layer_violation ? '⚠ LAYER VIOLATION' : '🏛 PONS CONTRACTVS',
        `L${edge.from_layer ?? 1}→L${edge.to_layer ?? 1} · ${edge.call_site_count ?? 1} calls`,
        edge.layer_violation ? 'rgba(239, 68, 68, 0.92)' : 'rgba(56, 189, 248, 0.88)',
        edge.layer_violation ? '#fecaca' : '#e0f2fe',
      );
    }

    // 2. Peripheral Landmark Callout Pills at 1:1 Screen Resolution
    const L = layout.landmarks;
    {
      const { px, py } = worldPx(layout, L.aqvae.x, L.aqvae.y);
      const isLit = Boolean(harness.mcp && harness.mcp.length > 0);
      const [sx, sy] = toScreen(px + 42, py - 48);
      drawRomanCalloutPill(
        ctx,
        sx,
        sy,
        'AQVAE',
        harness.mcp ? `${harness.mcp.filter((m) => m.ok).length}/${harness.mcp.length} mcp` : 'offline',
        isLit ? 'rgba(34, 211, 238, 0.85)' : 'rgba(120, 113, 108, 0.45)',
        isLit ? '#e0f2fe' : '#a8a29e',
      );
    }
    {
      const { px, py } = worldPx(layout, L.castrum.x, L.castrum.y);
      const isLit = Boolean(harness.ci);
      const [sx, sy] = toScreen(px, py + 46);
      drawRomanCalloutPill(
        ctx,
        sx,
        sy,
        'CASTRVM',
        harness.ci ? `${harness.ci.runners.length} legio` : 'offline',
        isLit ? 'rgba(251, 191, 36, 0.82)' : 'rgba(120, 113, 108, 0.45)',
        isLit ? '#fde68a' : '#a8a29e',
      );
    }
    {
      const { px, py } = worldPx(layout, L.portus.x, L.portus.y);
      const isLit = harness.queueLen !== null;
      const [sx, sy] = toScreen(px - 18, py + 56);
      drawRomanCalloutPill(
        ctx,
        sx,
        sy,
        'PORTVS',
        harness.queueLen !== null ? `${harness.queueLen} tasks` : 'harbor',
        isLit ? 'rgba(56, 189, 248, 0.82)' : 'rgba(120, 113, 108, 0.45)',
        isLit ? '#e0f2fe' : '#a8a29e',
      );
    }
    {
      const { px, py } = worldPx(layout, L.gate.x, L.gate.y);
      const headBranch = harness.vcs?.branches.find((b) => b.isHead)?.name ?? harness.vcs?.branches[0]?.name ?? 'main';
      const ckptCount = harness.checkpoints?.length ?? 3;
      const [sx, sy] = toScreen(px, py + 42);
      drawRomanCalloutPill(
        ctx,
        sx,
        sy,
        'PORTA GIT',
        harness.vcs ? `${headBranch} · ${ckptCount} ckpt` : `${ckptCount} milestones`,
        harness.vcs ? 'rgba(229, 169, 60, 0.72)' : 'rgba(194, 155, 56, 0.65)',
        '#fde68a',
      );
    }
  }

  private paintLandmarks(ctx: CanvasRenderingContext2D, s: WorldState) {
    const { layout, harness } = s;
    const L = layout.landmarks;

    // 1. AQVAE
    {
      const { px, py } = worldPx(layout, L.aqvae.x, L.aqvae.y);
      const isLit = Boolean(harness.mcp && harness.mcp.length > 0);
      drawAqueductComplex(ctx, px, py, isLit);
    }

    // 2. CASTRVM
    {
      const { px, py } = worldPx(layout, L.castrum.x, L.castrum.y);
      const isLit = Boolean(harness.ci);
      drawCastrumFortressComplex(ctx, px, py, harness.ci?.runners.length ?? 6, isLit);
    }

    // 3. PORTVS
    {
      const { px, py } = worldPx(layout, L.portus.x, L.portus.y);
      drawPortusHarborComplex(ctx, px, py, harness.queueLen ?? 3);
    }

    // 4. PORTA GIT — Stone Milestone Road (Checkpoints) & Counterweighted Portcullis Gate (Needs-You Approvals)
    {
      const { px, py } = worldPx(layout, L.gate.x, L.gate.y);
      const totalGates =
        Object.values(s.needsYouByDistrict ?? {}).reduce((a, b) => a + b, 0) +
        (harness.attentionGates?.length ?? 0);
      const ckptCount = harness.checkpoints?.length ?? 3;
      drawIsoMilestoneRoadAndPortcullis(ctx, px, py, ckptCount, totalGates > 0);
      if (harness.vcs) {
        harness.vcs.prs.slice(0, 3).forEach((_pr, i) => {
          this.stamp(ctx, 'caravan', px + 42 + i * 28, py + 18 + i * 8);
        });
      }
    }
  }
}
