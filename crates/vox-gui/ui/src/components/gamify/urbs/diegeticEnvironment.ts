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
    // Container is wider than 16:9: fill full width up to 1.24x height (safe sky & harbor margins)
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
      [0.30, 0.36],
      [0.19, 0.40],
      [0.24, 0.45],
      [0.35, 0.50],
      [0.45, 0.54],
    ],
    widthPx: 6.0,
    flowSpeed: 0.26,
    hasCascadeDrop: true,
  },
  {
    id: 'acropolis-nymphaeum-cascade',
    uvPoints: [
      [0.49, 0.58],
      [0.51, 0.62],
      [0.54, 0.66],
    ],
    widthPx: 10.0,
    flowSpeed: 0.34,
    hasCascadeDrop: true,
  },
  {
    id: 'upper-colosseum-conduit',
    uvPoints: [
      [0.75, 0.51],
      [0.70, 0.59],
      [0.65, 0.67],
      [0.61, 0.77],
    ],
    widthPx: 11.0,
    flowSpeed: 0.28,
    hasCascadeDrop: true,
  },
  {
    id: 'praetorium-harbor-sluice',
    uvPoints: [
      [0.60, 0.83],
      [0.61, 0.89],
    ],
    widthPx: 14.0,
    flowSpeed: 0.38,
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
    id: 'via-sacra-forum',
    uvPoints: [
      [0.25, 0.78],
      [0.34, 0.71],
      [0.43, 0.68],
      [0.48, 0.72],
    ],
    crowdCount: 6,
    caste: 'senator',
  },
  {
    id: 'clivus-capitolinus',
    uvPoints: [
      [0.46, 0.37],
      [0.54, 0.35],
      [0.64, 0.38],
    ],
    crowdCount: 5,
    caste: 'scholar',
  },
  {
    id: 'via-triumphalis-colosseum',
    uvPoints: [
      [0.57, 0.51],
      [0.66, 0.46],
      [0.76, 0.42],
      [0.83, 0.44],
    ],
    crowdCount: 6,
    caste: 'legionary',
  },
  {
    id: 'via-portuensis-quay',
    uvPoints: [
      [0.37, 0.84],
      [0.48, 0.80],
      [0.59, 0.76],
      [0.69, 0.78],
    ],
    crowdCount: 5,
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

  // 5. Portus Harbor Basin Shimmering Waves (lower-right Mediterranean basin)
  const harborWaves: [number, number][] = [
    [0.74, 0.78],
    [0.81, 0.81],
    [0.88, 0.79],
    [0.78, 0.85],
    [0.86, 0.85],
    [0.92, 0.83],
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

  ctx.restore();
}

/**
 * Renders living crowds of Roman citizens, senators, scholars, legionaries walking along
 * the cobblestone streets & bridges, plus gently bobbing Roman merchant galleys in Portus Harbor.
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

  // 1. Living Citizen & Legionary Processions along the 4 Roman Street Arteries
  for (let rIdx = 0; rIdx < DIEGETIC_STREET_ROUTES.length; rIdx++) {
    const route = DIEGETIC_STREET_ROUTES[rIdx]!;
    const pts = route.uvPoints.map((uv) => toPanoramaPx(rect, uv));

    for (let i = 0; i < route.crowdCount; i++) {
      // Alternate forward and reverse walkers along the street
      const forward = i % 2 === 0;
      const rawT = (phaseSec * 0.055 * walkSpeedMult + i / route.crowdCount + rIdx * 0.19) % 1;
      const t = forward ? rawT : 1 - rawT;
      const sample = samplePolylineAt(pts, t);

      // Slight lateral street offset so pairs walk side-by-side without overlapping
      const laneOffset = ((i % 3) - 1) * 3.2 * scaleFactor;
      const px = sample.x - Math.sin(sample.angle) * laneOffset;
      const py = sample.y + Math.cos(sample.angle) * laneOffset;

      // Animated walking stride bob
      const stridePhase = phaseSec * 9.5 + i * 2.3 + rIdx;
      const bobY = Math.abs(Math.sin(stridePhase)) * 1.6 * scaleFactor;
      const strideX = Math.sin(stridePhase) * 1.3 * scaleFactor;

      // Soft cobblestone ground shadow
      ctx.fillStyle = 'rgba(28, 20, 12, 0.38)';
      ctx.beginPath();
      ctx.ellipse(px, py + 1.5 * scaleFactor, 3.2 * scaleFactor, 1.5 * scaleFactor, 0, 0, Math.PI * 2);
      ctx.fill();

      // Legs in walking stride
      ctx.strokeStyle = '#5C4428';
      ctx.lineWidth = 1.1 * scaleFactor;
      ctx.beginPath();
      ctx.moveTo(px - 0.8 * scaleFactor, py - 2 * scaleFactor);
      ctx.lineTo(px - strideX, py + 1 * scaleFactor);
      ctx.moveTo(px + 0.8 * scaleFactor, py - 2 * scaleFactor);
      ctx.lineTo(px + strideX, py + 1 * scaleFactor);
      ctx.stroke();

      // Tunic / Toga / Cloak by caste
      const tunicFill =
        route.caste === 'senator'
          ? '#FAF6EE' // White Carrara Senatorial Toga
          : route.caste === 'legionary'
            ? '#1B6B3A' // Imperial Emerald Legionary Cloak
            : route.caste === 'scholar'
              ? '#D4AF37' // Burnished Gold Collegium Robe
              : '#9A6B32'; // Warm Terracotta Artisan Tunic
      const sashStroke =
        route.caste === 'senator'
          ? '#D4AF37'
          : route.caste === 'legionary'
            ? '#FDE047'
            : '#FFF8E7';

      ctx.fillStyle = tunicFill;
      ctx.strokeStyle = sashStroke;
      ctx.lineWidth = 0.8 * scaleFactor;
      ctx.beginPath();
      ctx.ellipse(
        px,
        py - 4.2 * scaleFactor - bobY,
        2.4 * scaleFactor,
        3.4 * scaleFactor,
        0,
        0,
        Math.PI * 2,
      );
      ctx.fill();
      ctx.stroke();

      // Head / Galea Helmet
      ctx.fillStyle = route.caste === 'legionary' ? '#D4AF37' : '#F5D0A9';
      ctx.beginPath();
      ctx.arc(px, py - 8.4 * scaleFactor - bobY, 1.65 * scaleFactor, 0, Math.PI * 2);
      ctx.fill();

      // Legionary golden standard staff or Scholar scroll glow on leader
      if (i === 0 && (route.caste === 'legionary' || route.caste === 'scholar')) {
        ctx.strokeStyle = '#FDE047';
        ctx.lineWidth = 1.0 * scaleFactor;
        ctx.beginPath();
        ctx.moveTo(px + 2.2 * scaleFactor, py - 1 * scaleFactor - bobY);
        ctx.lineTo(px + 2.2 * scaleFactor, py - 12.5 * scaleFactor - bobY);
        ctx.stroke();

        ctx.fillStyle = route.caste === 'legionary' ? '#10B981' : '#FEF08A';
        ctx.beginPath();
        ctx.arc(px + 2.2 * scaleFactor, py - 13 * scaleFactor - bobY, 1.6 * scaleFactor, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  // 2. Portus Harbor Bobbing Roman Merchant Galleys & Animated Oar Ripples
  const galleys: { uv: [number, number]; dir: number }[] = [
    { uv: [0.78, 0.80], dir: 1 },
    { uv: [0.87, 0.83], dir: -1 },
  ];
  for (let gIdx = 0; gIdx < galleys.length; gIdx++) {
    const g = galleys[gIdx]!;
    const [baseX, baseY] = toPanoramaPx(rect, g.uv);
    const glideX = baseX + Math.sin(phaseSec * 0.45 + gIdx * 2.1) * 14 * scaleFactor * g.dir;
    const bobY = baseY + Math.sin(phaseSec * 1.8 + gIdx) * 1.8 * scaleFactor;

    // Animated white V-wake ripple behind hull
    ctx.strokeStyle = 'rgba(224, 242, 254, 0.55)';
    ctx.lineWidth = 1.1 * scaleFactor;
    ctx.beginPath();
    ctx.ellipse(glideX, bobY + 2 * scaleFactor, 13 * scaleFactor, 4.2 * scaleFactor, 0, 0, Math.PI * 2);
    ctx.stroke();

    // Wooden merchant galley hull
    ctx.fillStyle = '#5C3A1E';
    ctx.strokeStyle = '#D4AF37';
    ctx.lineWidth = 0.9 * scaleFactor;
    ctx.beginPath();
    ctx.moveTo(glideX - 9 * scaleFactor, bobY);
    ctx.quadraticCurveTo(glideX, bobY + 4 * scaleFactor, glideX + 9 * scaleFactor, bobY);
    ctx.lineTo(glideX + 6 * scaleFactor, bobY - 2.5 * scaleFactor);
    ctx.lineTo(glideX - 6 * scaleFactor, bobY - 2.5 * scaleFactor);
    ctx.closePath?.();
    ctx.fill();
    ctx.stroke();

    // Cream & Gold Roman square sail
    ctx.fillStyle = '#FAF5E8';
    ctx.strokeStyle = '#C59B27';
    ctx.beginPath();
    ctx.moveTo(glideX - 4.5 * scaleFactor, bobY - 3 * scaleFactor);
    ctx.quadraticCurveTo(
      glideX + 1.5 * scaleFactor * g.dir,
      bobY - 7.5 * scaleFactor,
      glideX + 4.5 * scaleFactor,
      bobY - 3 * scaleFactor,
    );
    ctx.lineTo(glideX, bobY - 11.5 * scaleFactor);
    ctx.closePath?.();
    ctx.fill();
    ctx.stroke();
  }

  ctx.restore();
}
