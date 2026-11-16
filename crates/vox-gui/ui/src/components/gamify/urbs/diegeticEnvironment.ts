// crates/vox-gui/ui/src/components/gamify/urbs/diegeticEnvironment.ts
/**
 * Phase 384 (`URBS-DIEGETIC-ENV-01..04`):
 * Diegetic Environmental Animation Layer for the Roman Metropolis Simulator (`LudusSandbox`).
 *
 * Replaces pasted-over straight rooftop vector lines with:
 *   1. Safe-Frame Panorama Fitting (`computeSafePanoramaRect`) so top/bottom monuments (Acropolis,
 *      Pantheon, Colosseum, Portus Harbor) are never decapitated by `object-cover`.
 *   2. Diegetic Animated Watercourses (`drawDiegeticWatercourses`) flowing *inside* the painted
 *      stone Aqueduct (`Aqua Marcia`), Central Nymphaeum Waterfall Cascade, Grand Forum Canal
 *      (`Euripus`), and Portus Harbor Basin with sunlit caustics, foam crests, and plunge mist.
 *   3. Living Roman Citizen Crowds, Legionary Processions & Harbor Galleys (`drawDiegeticCrowdsAndGalleys`)
 *      walking along the actual cobblestone streets, bridges, and harbor quays.
 */

import { samplePolylineAt } from './hydraulicNetwork';
import {
  depthScaleFromUvV,
  renderDiegeticTorchAndBrazierPools,
  renderHypocaustForgeSteam,
  renderPerspectiveIntegratedProcession,
  renderPhotorealWaterCausticsAndMist,
  renderSunlitGodRays,
} from '../../fx/PhotorealisticCanvasFx';

export interface PanoramaRect {
  drawX: number;
  drawY: number;
  drawW: number;
  drawH: number;
}

/**
 * Computes an aspect-preserving safe-frame rectangle for the 16:9 Roman Metropolis panorama
 * so that wide or docked containers never slice off the top Acropolis pediment or bottom Portus Harbor.
 * Allows at most 4% vertical trim before letterbox-blending with warm Carrara sky/harbor gradients.
 */
export function computeSafePanoramaRect(
  cw: number,
  ch: number,
  imgW: number,
  imgH: number,
): PanoramaRect {
  const safeCw = Math.max(1, cw);
  const safeCh = Math.max(1, ch);
  const imgAspect = Math.max(0.5, imgW / Math.max(1, imgH));
  const canvasAspect = safeCw / safeCh;

  if (canvasAspect > imgAspect) {
    // Ultra-wide panoramic strips (e.g. Dashboard embedded 4.5:1 map): cover 100% width with
    // horizon focal framing (0.44) so zero empty sideband letterboxing appears on left/right.
    if (canvasAspect > 2.15) {
      const drawW = safeCw;
      const drawH = safeCw / imgAspect;
      const drawX = 0;
      const drawY = (safeCh - drawH) * 0.44;
      return { drawX, drawY, drawW, drawH };
    }
    // Standard wide desktop viewports (16:10..2:1): preserve vertical monument & harbor bounds.
    const coverH = safeCw / imgAspect;
    const maxAllowedH = safeCh * 1.24;
    const drawH = Math.min(coverH, maxAllowedH);
    const drawW = drawH * imgAspect;
    const drawX = (safeCw - drawW) * 0.5;
    const drawY = (safeCh - drawH) * 0.45;
    return { drawX, drawY, drawW, drawH };
  }

  // Container is taller/narrower than 16:9 (e.g., docked Chat inspector panel):
  // Fill 100% of vertical height (drawH = safeCh, drawY = 0) and center on the Citadel Acropolis
  // so there is never an empty sky band at the top or bottom of a narrow dock panel.
  const drawH = safeCh;
  const drawW = safeCh * imgAspect;
  const drawX = (safeCw - drawW) * 0.5;
  const drawY = 0;
  return { drawX, drawY, drawW, drawH };
}

export interface DiegeticWaterChannel {
  id: string;
  /** Normalized `[u, v]` coordinates (`0..1`) relative to the painted panorama rectangle. */
  uvPoints: [number, number][];
  widthPx: number;
  flowSpeed: number;
  hasCascadeDrop?: boolean;
}

export const DIEGETIC_WATER_CHANNELS: DiegeticWaterChannel[] = [
  {
    id: 'aqua-marcia-aqueduct',
    uvPoints: [
      [0.185, 0.205],
      [0.102, 0.242],
      [0.095, 0.268],
      [0.185, 0.322],
      [0.305, 0.380],
      [0.398, 0.422],
    ],
    widthPx: 4.8,
    flowSpeed: 0.28,
    hasCascadeDrop: true,
  },
  {
    id: 'acropolis-nymphaeum-cascade',
    uvPoints: [
      [0.435, 0.485],
      [0.452, 0.535],
      [0.465, 0.575],
    ],
    widthPx: 8.5,
    flowSpeed: 0.36,
    hasCascadeDrop: true,
  },
  {
    id: 'euripus-grand-canal',
    uvPoints: [
      [0.752, 0.395],
      [0.695, 0.452],
      [0.642, 0.515],
      [0.585, 0.595],
      [0.548, 0.685],
    ],
    widthPx: 9.2,
    flowSpeed: 0.3,
    hasCascadeDrop: false,
  },
  {
    id: 'pons-harbor-waterfall',
    uvPoints: [
      [0.552, 0.775],
      [0.552, 0.848],
    ],
    widthPx: 14.0,
    flowSpeed: 0.42,
    hasCascadeDrop: true,
  },
];

export interface DiegeticStreetRoute {
  id: string;
  uvPoints: [number, number][];
  crowdCount: number;
  caste: 'senator' | 'legionary' | 'scholar' | 'artisan';
}

export const DIEGETIC_STREET_ROUTES: DiegeticStreetRoute[] = [
  {
    id: 'via-sacra-lower-forum',
    uvPoints: [
      [0.06, 0.88],
      [0.18, 0.78],
      [0.27, 0.69],
    ],
    crowdCount: 5,
    caste: 'senator',
  },
  {
    id: 'forum-canal-promenade',
    uvPoints: [
      [0.49, 0.68],
      [0.53, 0.57],
      [0.63, 0.47],
      [0.71, 0.37],
    ],
    crowdCount: 5,
    caste: 'scholar',
  },
  {
    id: 'via-triumphalis-colosseum',
    uvPoints: [
      [0.72, 0.34],
      [0.80, 0.37],
      [0.90, 0.36],
    ],
    crowdCount: 6,
    caste: 'legionary',
  },
  {
    id: 'via-portuensis-quay',
    uvPoints: [
      [0.25, 0.82],
      [0.39, 0.85],
      [0.46, 0.76],
    ],
    crowdCount: 4,
    caste: 'artisan',
  },
];

function toPanoramaPx(rect: PanoramaRect, uv: [number, number]): [number, number] {
  return [rect.drawX + uv[0] * rect.drawW, rect.drawY + uv[1] * rect.drawH];
}

function traceSmoothSpline(ctx: CanvasRenderingContext2D, pts: [number, number][]): void {
  if (pts.length === 0) return;
  ctx.beginPath();
  ctx.moveTo(pts[0]![0], pts[0]![1]);
  if (pts.length === 2 || typeof ctx.quadraticCurveTo !== 'function') {
    for (let i = 1; i < pts.length; i++) {
      ctx.lineTo(pts[i]![0], pts[i]![1]);
    }
    return;
  }
  for (let i = 1; i < pts.length - 1; i++) {
    const curr = pts[i]!;
    const next = pts[i + 1]!;
    const midX = (curr[0] + next[0]) * 0.5;
    const midY = (curr[1] + next[1]) * 0.5;
    ctx.quadraticCurveTo(curr[0], curr[1], midX, midY);
  }
  const last = pts[pts.length - 1]!;
  ctx.lineTo(last[0], last[1]);
}

/**
 * Renders diegetic animated water inside the Aqueduct arches, Nymphaeum waterfall cascade,
 * Forum canal, and Portus Harbor basin.
 */
export function drawDiegeticWatercourses(
  ctx: CanvasRenderingContext2D,
  rect: PanoramaRect,
  timeMs: number,
  activeTaskCount = 0,
  errorCount = 0,
): void {
  ctx.save();
  const phaseSec = timeMs * 0.001;
  const speedBoost = 1 + Math.min(0.8, activeTaskCount * 0.2);
  const scaleFactor = Math.max(0.65, Math.min(1.35, rect.drawW / 1200));

  for (let cIdx = 0; cIdx < DIEGETIC_WATER_CHANNELS.length; cIdx++) {
    const ch = DIEGETIC_WATER_CHANNELS[cIdx]!;
    const pts = ch.uvPoints.map((uv) => toPanoramaPx(rect, uv));
    const w = ch.widthPx * scaleFactor;

    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    // 1. Animated sunlit water ripples & white foam wavelets flowing inside the painted channel (no continuous vector line!)
    const rippleCount = 9;
    for (let r = 0; r < rippleCount; r++) {
      const t = ((phaseSec * ch.flowSpeed * speedBoost + r / rippleCount + cIdx * 0.13) % 1 + 1) % 1;
      const s0 = samplePolylineAt(pts, Math.max(0, t - 0.018));
      const s1 = samplePolylineAt(pts, Math.min(1, t + 0.018));
      const perpX = -Math.sin(s0.angle) * (w * 0.16) * Math.sin(phaseSec * 3.5 + r);
      const perpY = Math.cos(s0.angle) * (w * 0.16) * Math.sin(phaseSec * 3.5 + r);

      ctx.strokeStyle =
        errorCount > 0 && cIdx === 3
          ? 'rgba(251, 191, 36, 0.55)'
          : r % 2 === 0
            ? 'rgba(254, 249, 195, 0.58)' // sunlit gold caustic shimmer
            : 'rgba(224, 242, 254, 0.62)'; // white water crest
      ctx.lineWidth = Math.max(1.1, w * 0.18);
      ctx.beginPath();
      ctx.moveTo(s0.x + perpX, s0.y + perpY);
      if (typeof ctx.quadraticCurveTo === 'function') {
        ctx.quadraticCurveTo(
          (s0.x + s1.x) * 0.5 + perpX * 0.5,
          (s0.y + s1.y) * 0.5 + perpY * 0.5,
          s1.x - perpX * 0.4,
          s1.y - perpY * 0.4,
        );
      } else {
        ctx.lineTo(s1.x - perpX * 0.4, s1.y - perpY * 0.4);
      }
      ctx.stroke();

      // Sparkling water droplet
      ctx.fillStyle = r % 3 === 0 ? 'rgba(254, 240, 138, 0.72)' : 'rgba(224, 247, 250, 0.75)';
      ctx.beginPath();
      ctx.arc(s0.x + perpX * 0.5, s0.y + perpY * 0.5, 1.3 * scaleFactor, 0, Math.PI * 2);
      ctx.fill();
    }

    // 4. Waterfall cascade mist & plunge-pool foam rings at Nymphaeum & Harbor outfall
    if (ch.hasCascadeDrop && pts.length >= 2) {
      const dropPt = pts[pts.length - 1]!;
      const pulse = (phaseSec * 1.4 + cIdx * 0.4) % 1;
      const rOuter = (8 + pulse * 14) * scaleFactor;
      const alpha = (1 - pulse) * 0.48;
      ctx.strokeStyle = `rgba(224, 242, 254, ${alpha.toFixed(3)})`;
      ctx.lineWidth = 1.4 * scaleFactor;
      ctx.beginPath();
      ctx.ellipse(dropPt[0], dropPt[1], rOuter, rOuter * 0.48, 0, 0, Math.PI * 2);
      ctx.stroke();

      if (typeof ctx.createRadialGradient === 'function') {
        const mistGrad = ctx.createRadialGradient(
          dropPt[0],
          dropPt[1],
          1,
          dropPt[0],
          dropPt[1],
          16 * scaleFactor,
        );
        mistGrad.addColorStop(0, 'rgba(240, 249, 255, 0.42)');
        mistGrad.addColorStop(0.6, 'rgba(125, 211, 252, 0.16)');
        mistGrad.addColorStop(1, 'rgba(125, 211, 252, 0)');
        ctx.fillStyle = mistGrad;
        ctx.beginPath();
        ctx.ellipse(dropPt[0], dropPt[1], 16 * scaleFactor, 9 * scaleFactor, 0, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  // 5. Portus Harbor Basin Shimmering Waves (strictly inside open Mediterranean harbor water v >= 0.89)
  const harborWaves: [number, number][] = [
    [0.22, 0.93],
    [0.36, 0.95],
    [0.54, 0.92],
    [0.64, 0.94],
    [0.76, 0.93],
    [0.86, 0.95],
  ];
  for (let wIdx = 0; wIdx < harborWaves.length; wIdx++) {
    const [hx, hy] = toPanoramaPx(rect, harborWaves[wIdx]!);
    const driftX = Math.sin(phaseSec * 1.6 + wIdx * 1.1) * 6 * scaleFactor;
    const waveAlpha = 0.28 + 0.22 * Math.sin(phaseSec * 2.1 + wIdx * 0.9);
    ctx.strokeStyle = `rgba(254, 249, 195, ${waveAlpha.toFixed(3)})`;
    ctx.lineWidth = 1.3 * scaleFactor;
    ctx.beginPath();
    ctx.moveTo(hx + driftX - 10 * scaleFactor, hy);
    ctx.quadraticCurveTo(hx + driftX, hy - 2.5 * scaleFactor, hx + driftX + 10 * scaleFactor, hy);
    ctx.stroke();
  }

  const bounds = { x: rect.drawX, y: rect.drawY, width: rect.drawW, height: rect.drawH };
  renderSunlitGodRays(ctx, bounds, timeMs, 0.88);
  renderPhotorealWaterCausticsAndMist(
    ctx,
    bounds,
    DIEGETIC_WATER_CHANNELS.map((ch, idx) => ({
      uvPoints: ch.uvPoints,
      widthPx: ch.widthPx * 0.82,
      flowSpeed: ch.flowSpeed * speedBoost,
      tint: errorCount > 0 && idx === 3 ? 'gold' : idx === 1 || idx === 3 ? 'turquoise' : 'turquoise',
    })),
    [
      { uv: [0.458, 0.562], radiusPx: 20, plumeHeightPx: 26 },
      { uv: [0.552, 0.855], radiusPx: 26, plumeHeightPx: 34 },
    ],
    timeMs,
    0.95,
  );
  // Seamless rising steam plumes anchored 1:1 on the twin MENS Foundry chimneys in the painted artwork
  renderHypocaustForgeSteam(
    ctx,
    bounds,
    [
      [0.835, 0.545],
      [0.885, 0.560],
    ],
    timeMs,
    1 + Math.min(0.35, activeTaskCount * 0.1),
  );
  renderDiegeticTorchAndBrazierPools(
    ctx,
    bounds,
    [
      [0.292, 0.205],
      [0.575, 0.268],
      [0.842, 0.338],
      [0.135, 0.722],
      [0.928, 0.692],
    ],
    timeMs,
    1 + Math.min(0.45, activeTaskCount * 0.12),
  );

  ctx.restore();
}

/**
 * Renders perspective-integrated Roman legionary, senatorial, and scholarly processions along
 * cobblestone streets & bridges (`PhotorealisticCanvasFx`), plus gently bobbing merchant galleys.
 */
export function drawDiegeticCrowdsAndGalleys(
  ctx: CanvasRenderingContext2D,
  rect: PanoramaRect,
  timeMs: number,
  activeTaskCount = 0,
): void {
  ctx.save();
  const phaseSec = timeMs * 0.001;
  const scaleFactor = Math.max(0.65, Math.min(1.25, rect.drawW / 1200));
  const walkSpeedMult = 1 + Math.min(0.75, activeTaskCount * 0.2);
  const bounds = { x: rect.drawX, y: rect.drawY, width: rect.drawW, height: rect.drawH };

  // 1. Perspective-integrated Roman processions with oblique contact shadows & aerial haze
  renderPerspectiveIntegratedProcession(
    ctx,
    bounds,
    DIEGETIC_STREET_ROUTES.map((r) => ({
      uvPoints: r.uvPoints,
      count: r.crowdCount,
      caste: r.caste,
      speed: 0.052,
    })),
    timeMs,
    walkSpeedMult,
  );

  // 2. Portus Harbor Bobbing Roman Merchant Galleys inside open harbor water (v >= 0.92)
  const galleys: { uv: [number, number]; dir: number }[] = [
    { uv: [0.23, 0.935], dir: 1 },
    { uv: [0.60, 0.935], dir: -1 },
  ];
  for (let gIdx = 0; gIdx < galleys.length; gIdx++) {
    const g = galleys[gIdx]!;
    const [baseX, baseY] = toPanoramaPx(rect, g.uv);
    const dScale = scaleFactor * depthScaleFromUvV(g.uv[1]) * 0.92;
    const glideX = baseX + Math.sin(phaseSec * 0.45 + gIdx * 2.1) * 14 * dScale * g.dir;
    const bobY = baseY + Math.sin(phaseSec * 1.8 + gIdx) * 1.6 * dScale;

    // Soft water hull shadow + sunlit ripple wake
    ctx.fillStyle = 'rgba(18, 52, 68, 0.34)';
    ctx.beginPath();
    ctx.ellipse(glideX + 1.5 * dScale, bobY + 2.2 * dScale, 11 * dScale, 3.2 * dScale, 0, 0, Math.PI * 2);
    ctx.fill();

    ctx.strokeStyle = 'rgba(240, 252, 255, 0.52)';
    ctx.lineWidth = 1.0 * dScale;
    ctx.beginPath();
    ctx.ellipse(glideX, bobY + 2 * dScale, 13 * dScale, 3.8 * dScale, 0, 0, Math.PI * 2);
    ctx.stroke();

    // Sun-warmed cedar merchant galley hull
    ctx.fillStyle = '#6B4423';
    ctx.strokeStyle = '#C89D54';
    ctx.lineWidth = 0.85 * dScale;
    ctx.beginPath();
    ctx.moveTo(glideX - 8.5 * dScale, bobY);
    ctx.quadraticCurveTo(glideX, bobY + 3.8 * dScale, glideX + 8.5 * dScale, bobY);
    ctx.lineTo(glideX + 5.8 * dScale, bobY - 2.4 * dScale);
    ctx.lineTo(glideX - 5.8 * dScale, bobY - 2.4 * dScale);
    ctx.closePath?.();
    ctx.fill();
    ctx.stroke();

    // Sunlit linen Roman square sail
    ctx.fillStyle = '#F6EFE0';
    ctx.strokeStyle = '#B88928';
    ctx.beginPath();
    ctx.moveTo(glideX - 4.2 * dScale, bobY - 2.8 * dScale);
    ctx.quadraticCurveTo(
      glideX + 1.4 * dScale * g.dir,
      bobY - 7.2 * dScale,
      glideX + 4.2 * dScale,
      bobY - 2.8 * dScale,
    );
    ctx.lineTo(glideX, bobY - 11 * dScale);
    ctx.closePath?.();
    ctx.fill();
    ctx.stroke();
  }

  ctx.restore();
}
