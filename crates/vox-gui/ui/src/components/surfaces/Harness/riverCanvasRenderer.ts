import type { RiverSnapshot, RiverStrand } from './riverTypes';

export const TIER_CFG = {
  fast: { color: '#E5C158', rgb: '229, 193, 88' }, // warm burnished gold
  standard: { color: '#D97706', rgb: '217, 119, 6' }, // imperial amber-bronze
  deep: { color: '#8FA339', rgb: '143, 163, 57' }, // olive-laurel bronze
} as const;

export interface AlluvialControlPoints {
  x0: number;
  y0: number;
  cx1: number;
  cy1: number;
  sx: number;
  sy: number;
  cx2: number;
  cy2: number;
  x1: number;
  y1: number;
}

/**
 * Computes upper and lower cubic-Bézier segments converging through the
 * central Interceptor/Verification Singularity chokepoint at (width/2, height/2).
 */
export function computeAlluvialControlPoints(
  strand: RiverStrand,
  width: number,
  height: number,
): AlluvialControlPoints {
  const padX = Math.min(24, width * 0.04);
  const usableW = Math.max(40, width - padX * 2);
  const x0 = padX + strand.sourceSlot * usableW;
  const y0 = 18;
  const x1 = padX + strand.targetSlot * usableW;
  const y1 = Math.max(40, height - 18);

  const pinchSpread = Math.min(36, width * 0.06);
  const offset = (strand.sourceSlot - 0.5) * pinchSpread;
  const sx = width / 2 + offset;
  const sy = height / 2;

  return {
    x0,
    y0,
    cx1: x0,
    cy1: y0 + (sy - y0) * 0.62,
    sx,
    sy,
    cx2: x1,
    cy2: sy + (y1 - sy) * 0.38,
    x1,
    y1,
  };
}

function strandStrokeColor(status: RiverStrand['status'], alpha: number): string {
  switch (status) {
    case 'verified':
      return `rgba(${TIER_CFG.deep.rgb}, ${alpha.toFixed(2)})`; // verdigris emerald
    case 'streaming':
      return `rgba(${TIER_CFG.fast.rgb}, ${alpha.toFixed(2)})`; // warm burnished gold
    case 'doubted':
      return `rgba(${TIER_CFG.standard.rgb}, ${alpha.toFixed(2)})`; // imperial amber-bronze
    case 'cached':
    default:
      return `rgba(243, 229, 171, ${alpha.toFixed(2)})`; // warm ivory stone
  }
}

export function renderRiverFrame(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  snapshot: RiverSnapshot,
  phaseSec: number,
): void {
  if (width <= 0 || height <= 0) return;

  ctx.save();
  if (typeof ctx.clearRect === 'function') {
    ctx.clearRect(0, 0, width, height);
  }
  // Sunlit crystal-clear water sheen so the uncropped aqueduct-sluice-cascade-bg.jpg shines through
  const bgGrad = ctx.createLinearGradient(0, 0, 0, height);
  bgGrad.addColorStop(0, 'rgba(255, 253, 247, 0.04)');
  bgGrad.addColorStop(0.5, 'rgba(56, 189, 248, 0.06)');
  bgGrad.addColorStop(1, 'rgba(16, 185, 129, 0.08)');
  ctx.fillStyle = bgGrad;
  ctx.fillRect(0, 0, width, height);

  // 1. Diegetic 3-Tier Aqueduct Watercourse Channels (Cursus I: Fast MENS, Cursus II: Hybrid, Cursus III: Deep)
  const channelLabels = [
    {
      yFrac: 0.28,
      label: 'CURSUS I · FAST LOCAL MENS',
      crestColor: 'rgba(224, 247, 250, 0.78)',
      goldStream: 'rgba(254, 240, 138, 0.72)',
    },
    {
      yFrac: 0.54,
      label: 'CURSUS II · BALANCED HYBRID',
      crestColor: 'rgba(224, 247, 250, 0.82)',
      goldStream: 'rgba(251, 191, 36, 0.74)',
    },
    {
      yFrac: 0.80,
      label: 'CURSUS III · DEEP FRONTIER',
      crestColor: 'rgba(209, 250, 229, 0.84)',
      goldStream: 'rgba(167, 243, 208, 0.78)',
    },
  ];
  const waterStartX = Math.min(196, Math.round(width * 0.22));
  const waterEndX = Math.max(waterStartX + 80, width - 24);

  for (let c = 0; c < channelLabels.length; c++) {
    const ch = channelLabels[c]!;
    const cy = Math.round(height * ch.yFrac);

    // Short organic water-ripple wavelets inside the painted turquoise water basins (no full-width line/box!)
    const waveletCount = 5;
    const spanW = (waterEndX - waterStartX) / waveletCount;
    for (let w = 0; w < waveletCount; w++) {
      const wx0 = waterStartX + w * spanW + 8;
      const wx1 = wx0 + spanW * 0.65;
      const waveOffset = Math.sin(phaseSec * 3.1 + c * 1.4 + w * 0.9) * 2.8;

      ctx.strokeStyle = w % 2 === 0 ? ch.crestColor : ch.goldStream;
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.moveTo(wx0, cy + waveOffset);
      ctx.bezierCurveTo(
        wx0 + (wx1 - wx0) * 0.33,
        cy - waveOffset * 1.4,
        wx0 + (wx1 - wx0) * 0.66,
        cy + waveOffset * 1.4,
        wx1,
        cy - waveOffset,
      );
      ctx.stroke();
    }

    // Traveling hydro-foam droplets & golden token glints inside the water trough
    const crestCount = 6;
    for (let k = 0; k < crestCount; k++) {
      const u = (phaseSec * (0.16 + c * 0.04) + k / crestCount) % 1;
      const wx = waterStartX + u * (waterEndX - waterStartX);
      const wy = cy + Math.sin(u * Math.PI * 4 + phaseSec * 3) * 2.0;
      ctx.fillStyle = k % 2 === 0 ? 'rgba(224, 247, 250, 0.88)' : 'rgba(254, 240, 138, 0.88)';
      ctx.beginPath();
      ctx.arc(wx, wy, 2.0, 0, Math.PI * 2);
      ctx.fill();
    }

    // Compact Carrara marble & gold tier nameplate on the left edge (never overlapping spoke badges)
    const badgeW = 166;
    const badgeH = 16;
    ctx.fillStyle = 'rgba(250, 246, 238, 0.94)';
    ctx.fillRect(12, cy - badgeH / 2, badgeW, badgeH);
    ctx.strokeStyle = 'rgba(197, 155, 39, 0.88)';
    ctx.lineWidth = 1.1;
    ctx.strokeRect(12, cy - badgeH / 2, badgeW, badgeH);

    ctx.font = '700 8px Cinzel, Palatino, Georgia, serif';
    ctx.fillStyle = '#1C140C';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(ch.label, 18, cy);
  }

  // 1a. Diegetic Waterfall Plunge Mist & Foam Droplets at the stepped cascade drops
  const cascadePools = [
    { u: 0.34, v: 0.42 },
    { u: 0.48, v: 0.48 },
    { u: 0.72, v: 0.56 },
    { u: 0.64, v: 0.78 },
  ];
  for (let s = 0; s < cascadePools.length; s++) {
    const pool = cascadePools[s]!;
    const px = Math.round(width * pool.u);
    const py = Math.round(height * pool.v);
    const pulse = 0.5 + 0.5 * Math.sin(phaseSec * 3.4 + s * 1.2);
    ctx.fillStyle = `rgba(224, 247, 250, ${(0.28 + pulse * 0.22).toFixed(2)})`;
    ctx.beginPath();
    ctx.arc(px, py, 4.5 + pulse * 3.5, 0, Math.PI * 2);
    ctx.fill();
  }

  // 1b. Aquae Caput — 8-Spoke Source Arches along top parapet (starting right of the Cursus badges so zero overlap!)
  const spokeNames = ['LANG', 'COMPILER', 'RUNTIME', 'ORCH', 'DB', 'GUI', 'CLI', 'TEST'];
  const archStartX = Math.min(192, Math.round(width * 0.21));
  const archZoneW = Math.max(200, width - archStartX - 16);
  const archStep = archZoneW / spokeNames.length;
  for (let s = 0; s < spokeNames.length; s++) {
    const ax = Math.round(archStartX + archStep * (s + 0.5));
    const ay = 38;
    ctx.fillStyle = 'rgba(250, 246, 238, 0.94)';
    ctx.strokeStyle = 'rgba(197, 155, 39, 0.85)';
    ctx.lineWidth = 1;
    ctx.fillRect(ax - 23, ay - 7, 46, 14);
    ctx.strokeRect(ax - 23, ay - 7, 46, 14);
    ctx.font = '700 7.5px Cinzel, Palatino, Georgia, serif';
    ctx.fillStyle = '#1C140C';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(spokeNames[s]!, ax, ay);
  }

  // 2. Render Subtle Alluvial Water Currents + Flowing Token Droplets
  for (let i = 0; i < snapshot.strands.length; i++) {
    const strand = snapshot.strands[i];
    const pts = computeAlluvialControlPoints(strand, width, height);
    const alpha = 0.12 + strand.intensity * 0.16;

    ctx.strokeStyle = strandStrokeColor(strand.status, alpha);
    ctx.lineWidth = strand.status === 'streaming' ? 1.4 : 0.9;

    // Upper half: top slot -> Singularity chokepoint
    ctx.beginPath();
    ctx.moveTo(pts.x0, pts.y0);
    ctx.bezierCurveTo(pts.cx1, pts.cy1, pts.sx, pts.y0 + (pts.sy - pts.y0) * 0.82, pts.sx, pts.sy);
    // Lower half: Singularity chokepoint -> bottom target slot
    ctx.bezierCurveTo(pts.sx, pts.sy + (pts.y1 - pts.sy) * 0.18, pts.cx2, pts.cy2, pts.x1, pts.y1);
    ctx.stroke();

    // Animated particle pulse traveling along the water current
    const t = (phaseSec * (0.38 + (i % 5) * 0.09) + i * 0.13) % 1;
    const py = pts.y0 + t * (pts.y1 - pts.y0);
    const blend = Math.sin(t * Math.PI);
    const LinearX = pts.x0 + t * (pts.x1 - pts.x0);
    const px = LinearX * (1 - blend * 0.82) + pts.sx * (blend * 0.82);

    ctx.fillStyle = strandStrokeColor(strand.status, 0.88);
    if (strand.status === 'streaming') {
      ctx.fillRect(px - 2.5, py - 1.5, 5, 3);
    } else {
      ctx.beginPath();
      ctx.arc(px, py, strand.status === 'verified' ? 2.2 : 1.6, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // 3. Central Convergence Gate — Subtle Sunlit Astrolabe Glow over the Painted Bronze Medallion
  const singX = width * 0.53;
  const singY = height * 0.62;
  const outerR = Math.min(64, width * 0.14);
  const sealGrad = ctx.createRadialGradient(singX, singY, 2, singX, singY, outerR);
  sealGrad.addColorStop(0, 'rgba(254, 240, 138, 0.45)');
  sealGrad.addColorStop(0.45, 'rgba(212, 175, 55, 0.18)');
  sealGrad.addColorStop(1, 'rgba(212, 175, 55, 0)');
  ctx.fillStyle = sealGrad;
  ctx.beginPath();
  ctx.arc(singX, singY, outerR, 0, Math.PI * 2);
  ctx.fill();

  // 4. Translucent Sunbeam Verification Sweep (replaces opaque dark-brown vertical pillar)
  const sweepNorm = 0.5 + 0.36 * Math.sin(phaseSec * 0.75);
  const gateX = Math.round(width * sweepNorm);
  const pillarW = 18;
  const pulseGrad = ctx.createLinearGradient(gateX - pillarW, 0, gateX + pillarW, 0);
  pulseGrad.addColorStop(0, 'rgba(212, 175, 55, 0)');
  pulseGrad.addColorStop(0.5, 'rgba(254, 249, 195, 0.24)');
  pulseGrad.addColorStop(1, 'rgba(212, 175, 55, 0)');
  ctx.fillStyle = pulseGrad;
  ctx.fillRect(gateX - pillarW, 0, pillarW * 2, height);

  // 5. Top callout pill: Luminous Carrara Marble & Gold Tablet (replaces dark #18130D box)
  const isCompact = width < 480;
  ctx.font = isCompact
    ? "700 8.5px 'Cinzel', 'Palatino', Georgia, serif"
    : "700 9.5px 'Cinzel', 'Palatino', Georgia, serif";
  const gateTitle = 'AQUEDUCT VERIFICATION GATE — LIVE TASK STREAM';
  const banner = snapshot.activePhaseBanner
    ? `${gateTitle} · ${snapshot.activePhaseBanner}`
    : gateTitle;
  const charW = isCompact ? 5.4 : 6.2;
  const pillW = Math.min(width - (isCompact ? 10 : 16), Math.max(260, banner.length * charW + 24));
  const pillH = isCompact ? 18 : 20;
  const pillX = Math.max(isCompact ? 5 : 8, (width - pillW) / 2);
  const pillY = 6;

  ctx.fillStyle = 'rgba(250, 246, 238, 0.94)';
  ctx.fillRect(pillX, pillY, pillW, pillH);
  ctx.strokeStyle = 'rgba(197, 155, 39, 0.88)';
  ctx.lineWidth = 1.2;
  ctx.strokeRect(pillX, pillY, pillW, pillH);

  ctx.fillStyle = '#1C140C';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(banner, pillX + pillW / 2, pillY + pillH / 2);

  ctx.restore();
}
