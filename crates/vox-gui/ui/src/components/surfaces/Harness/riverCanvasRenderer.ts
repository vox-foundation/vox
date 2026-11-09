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
  // Translucent warm travertine glaze so the high-res aqueduct-sluice-cascade-bg.jpg shines underneath
  const bgGrad = ctx.createLinearGradient(0, 0, 0, height);
  bgGrad.addColorStop(0, 'rgba(20, 16, 11, 0.38)');
  bgGrad.addColorStop(0.5, 'rgba(14, 11, 7, 0.30)');
  bgGrad.addColorStop(1, 'rgba(23, 18, 12, 0.42)');
  ctx.fillStyle = bgGrad;
  ctx.fillRect(0, 0, width, height);

  // 1. 3-Tier Stepped Marble Aqueduct Channels (Cursus I: Fast MENS, Cursus II: Hybrid, Cursus III: Deep)
  const channelLabels = [
    { yFrac: 0.28, label: 'CURSUS I · FAST LOCAL MENS', color: 'rgba(229, 193, 88, 0.22)', border: 'rgba(212, 175, 55, 0.42)' },
    { yFrac: 0.52, label: 'CURSUS II · BALANCED HYBRID', color: 'rgba(217, 119, 6, 0.20)', border: 'rgba(245, 158, 11, 0.42)' },
    { yFrac: 0.76, label: 'CURSUS III · DEEP FRONTIER', color: 'rgba(143, 163, 57, 0.20)', border: 'rgba(184, 155, 76, 0.45)' },
  ];
  for (let c = 0; c < channelLabels.length; c++) {
    const ch = channelLabels[c]!;
    const cy = Math.round(height * ch.yFrac);
    const bandH = Math.max(22, Math.round(height * 0.14));
    ctx.fillStyle = ch.color;
    ctx.fillRect(12, cy - bandH / 2, width - 24, bandH);
    ctx.strokeStyle = ch.border;
    ctx.lineWidth = 1.1;
    ctx.strokeRect(12, cy - bandH / 2, width - 24, bandH);

    // Animated laminar hydraulic wave crest inside each aqueduct channel
    ctx.strokeStyle = c === 2 ? 'rgba(220, 231, 182, 0.48)' : 'rgba(253, 230, 138, 0.48)';
    ctx.lineWidth = 1.4;
    const waveOffset = Math.sin(phaseSec * 2.2 + c * 1.3) * 4;
    ctx.beginPath();
    ctx.moveTo(16, cy + waveOffset);
    ctx.bezierCurveTo(
      width * 0.33,
      cy - waveOffset * 1.4,
      width * 0.66,
      cy + waveOffset * 1.4,
      width - 16,
      cy - waveOffset,
    );
    ctx.stroke();

    ctx.font = '600 8.5px Cinzel, Palatino, Georgia, serif';
    ctx.fillStyle = 'rgba(245, 239, 228, 0.88)';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(ch.label, 20, cy - bandH / 2 + 8);
  }

  // 1b. Aquae Caput — 8-Spoke Source Arches along top parapet
  const spokeNames = ['LANG', 'COMPILER', 'RUNTIME', 'ORCH', 'DB', 'GUI', 'CLI', 'TEST'];
  const archZoneW = Math.max(240, width - 48);
  const archStep = archZoneW / spokeNames.length;
  for (let s = 0; s < spokeNames.length; s++) {
    const ax = Math.round(24 + archStep * (s + 0.5));
    const ay = 38;
    ctx.fillStyle = 'rgba(28, 21, 14, 0.86)';
    ctx.strokeStyle = 'rgba(212, 175, 55, 0.68)';
    ctx.lineWidth = 1;
    ctx.fillRect(ax - 22, ay - 8, 44, 15);
    ctx.strokeRect(ax - 22, ay - 8, 44, 15);
    ctx.font = '600 7.5px Cinzel, Palatino, Georgia, serif';
    ctx.fillStyle = '#F3E5AB';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(spokeNames[s]!, ax, ay - 0.5);
  }

  // 2. Render Alluvial Bézier Strands + Flowing Multi-Shape Hydraulic Token Particles
  for (let i = 0; i < snapshot.strands.length; i++) {
    const strand = snapshot.strands[i];
    const pts = computeAlluvialControlPoints(strand, width, height);
    const alpha = 0.28 + strand.intensity * 0.52;

    ctx.strokeStyle = strandStrokeColor(strand.status, alpha);
    ctx.lineWidth = strand.status === 'streaming' ? 2.1 : 1.3;

    // Upper half: top slot -> Singularity chokepoint
    ctx.beginPath();
    ctx.moveTo(pts.x0, pts.y0);
    ctx.bezierCurveTo(pts.cx1, pts.cy1, pts.sx, pts.y0 + (pts.sy - pts.y0) * 0.82, pts.sx, pts.sy);
    // Lower half: Singularity chokepoint -> bottom target slot
    ctx.bezierCurveTo(pts.sx, pts.sy + (pts.y1 - pts.sy) * 0.18, pts.cx2, pts.cy2, pts.x1, pts.y1);
    ctx.stroke();

    // Animated particle pulse traveling along the strand (shape + color semiotic encoding)
    const t = (phaseSec * (0.38 + (i % 5) * 0.09) + i * 0.13) % 1;
    const py = pts.y0 + t * (pts.y1 - pts.y0);
    const blend = Math.sin(t * Math.PI); // peaks at 1.0 in center chokepoint
    const LinearX = pts.x0 + t * (pts.x1 - pts.x0);
    const px = LinearX * (1 - blend * 0.82) + pts.sx * (blend * 0.82);

    ctx.fillStyle = strandStrokeColor(strand.status, 0.96);
    if (strand.status === 'streaming') {
      // Gold Token Ingot block
      ctx.fillRect(px - 3, py - 2, 6, 4);
    } else {
      ctx.beginPath();
      ctx.arc(px, py, strand.status === 'verified' ? 2.6 : 1.9, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // 3. Central Convergence Gate — Burnished Bronze Keystone & Astrolabe Seal
  const singX = width / 2;
  const singY = height / 2;
  const outerR = Math.min(90, width * 0.22);
  const sealGrad = ctx.createRadialGradient(singX, singY, 2, singX, singY, outerR);
  sealGrad.addColorStop(0, '#F3E5AB'); // warm molten gold core
  sealGrad.addColorStop(0.35, 'rgba(212, 175, 55, 0.42)'); // outer burnished brass ring
  sealGrad.addColorStop(1, 'rgba(23, 18, 12, 0)');
  ctx.fillStyle = sealGrad;
  ctx.beginPath();
  ctx.arc(singX, singY, outerR, 0, Math.PI * 2);
  ctx.fill();

  // Engraved Roman compass spokes & keystone astrolabe ring
  ctx.save();
  ctx.strokeStyle = 'rgba(212, 175, 55, 0.42)';
  ctx.lineWidth = 1;
  const ringR = Math.min(32, outerR * 0.5);
  ctx.beginPath();
  ctx.arc(singX, singY, ringR, 0, Math.PI * 2);
  ctx.stroke();
  ctx.beginPath();
  ctx.arc(singX, singY, ringR * 0.4, 0, Math.PI * 2);
  ctx.stroke();

  // 8 engraved Roman compass spokes
  for (let s = 0; s < 8; s++) {
    const angle = (s * Math.PI) / 4 + phaseSec * 0.05;
    const xFrom = singX + Math.cos(angle) * (ringR * 0.4);
    const yFrom = singY + Math.sin(angle) * (ringR * 0.4);
    const xTo = singX + Math.cos(angle) * ringR;
    const yTo = singY + Math.sin(angle) * ringR;
    ctx.beginPath();
    ctx.moveTo(xFrom, yFrom);
    if (typeof ctx.lineTo === 'function') {
      ctx.lineTo(xTo, yTo);
    } else {
      ctx.bezierCurveTo(xFrom, yFrom, xTo, yTo, xTo, yTo);
    }
    ctx.stroke();
  }
  ctx.restore();

  // 4. Carved Travertine & Burnished Brass Aqueduct Sluice Gate
  const sweepNorm = 0.5 + 0.36 * Math.sin(phaseSec * 0.75);
  const gateX = Math.round(width * sweepNorm);
  const pillarW = 12;

  // #1B160F stone pillar
  ctx.fillStyle = '#1B160F';
  ctx.fillRect(gateX - pillarW / 2, 0, pillarW, height);

  // #D4AF37 brass border
  ctx.strokeStyle = '#D4AF37';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(gateX - pillarW / 2, 0);
  if (typeof ctx.lineTo === 'function') {
    ctx.lineTo(gateX - pillarW / 2, height);
    ctx.moveTo(gateX + pillarW / 2, 0);
    ctx.lineTo(gateX + pillarW / 2, height);
  } else {
    ctx.bezierCurveTo(gateX - pillarW / 2, 0, gateX - pillarW / 2, height, gateX - pillarW / 2, height);
    ctx.moveTo(gateX + pillarW / 2, 0);
    ctx.bezierCurveTo(gateX + pillarW / 2, 0, gateX + pillarW / 2, height, gateX + pillarW / 2, height);
  }
  ctx.stroke();

  // Warm golden verification pulse
  const pulseGrad = ctx.createLinearGradient(gateX - pillarW * 1.5, 0, gateX + pillarW * 1.5, 0);
  pulseGrad.addColorStop(0, 'rgba(212, 175, 55, 0)');
  pulseGrad.addColorStop(0.5, 'rgba(243, 229, 171, 0.45)');
  pulseGrad.addColorStop(1, 'rgba(212, 175, 55, 0)');
  ctx.fillStyle = pulseGrad;
  ctx.fillRect(gateX - pillarW * 1.5, 0, pillarW * 3, height);

  // 5. Top callout pill: Carved travertine & brass tablet
  const isCompact = width < 480;
  ctx.font = isCompact
    ? "600 9px 'Cinzel', 'Palatino', Georgia, serif"
    : "600 10px 'Cinzel', 'Palatino', Georgia, serif";
  const gateTitle = 'AQUEDUCT VERIFICATION GATE — LIVE TASK STREAM';
  const banner = snapshot.activePhaseBanner
    ? `${gateTitle} · ${snapshot.activePhaseBanner}`
    : gateTitle;
  const charW = isCompact ? 5.6 : 6.6;
  const pillW = Math.min(width - (isCompact ? 10 : 16), Math.max(280, banner.length * charW + 24));
  const pillH = isCompact ? 20 : 22;
  const pillX = Math.max(isCompact ? 5 : 8, (width - pillW) / 2);
  const pillY = 8;

  // Tablet fill: #18130D, border: rgba(212, 175, 55, 0.55), text: #F5F0E6
  ctx.fillStyle = '#18130D';
  ctx.fillRect(pillX, pillY, pillW, pillH);
  ctx.strokeStyle = 'rgba(212, 175, 55, 0.55)';
  ctx.lineWidth = 1;
  ctx.strokeRect(pillX, pillY, pillW, pillH);

  ctx.fillStyle = '#F5F0E6';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(banner, pillX + pillW / 2, pillY + pillH / 2);

  ctx.restore();
}
