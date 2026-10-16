import type { RiverSnapshot, RiverStrand } from './riverTypes';

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
      return `rgba(52, 211, 153, ${alpha.toFixed(2)})`; // emerald
    case 'streaming':
      return `rgba(34, 211, 238, ${alpha.toFixed(2)})`; // cyan
    case 'doubted':
      return `rgba(251, 191, 36, ${alpha.toFixed(2)})`; // amber
    case 'cached':
    default:
      return `rgba(228, 228, 231, ${alpha.toFixed(2)})`; // monochrome zinc/white
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
  ctx.fillStyle = '#070709';
  ctx.fillRect(0, 0, width, height);

  // 1. Subtle horizontal telemetry grid lines
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
  ctx.lineWidth = 1;
  const gridRows = 6;
  for (let r = 1; r < gridRows; r++) {
    const gy = Math.round((height / gridRows) * r);
    ctx.beginPath();
    ctx.moveTo(0, gy);
    ctx.bezierCurveTo(width * 0.33, gy, width * 0.66, gy, width, gy);
    ctx.stroke();
  }

  // 2. Render Alluvial Bézier Strands + Flowing Particles
  for (let i = 0; i < snapshot.strands.length; i++) {
    const strand = snapshot.strands[i];
    const pts = computeAlluvialControlPoints(strand, width, height);
    const alpha = 0.16 + strand.intensity * 0.42;

    ctx.strokeStyle = strandStrokeColor(strand.status, alpha);
    ctx.lineWidth = strand.status === 'streaming' ? 1.6 : 1.0;

    // Upper half: top slot -> Singularity chokepoint
    ctx.beginPath();
    ctx.moveTo(pts.x0, pts.y0);
    ctx.bezierCurveTo(pts.cx1, pts.cy1, pts.sx, pts.y0 + (pts.sy - pts.y0) * 0.82, pts.sx, pts.sy);
    // Lower half: Singularity chokepoint -> bottom target slot
    ctx.bezierCurveTo(pts.sx, pts.sy + (pts.y1 - pts.sy) * 0.18, pts.cx2, pts.cy2, pts.x1, pts.y1);
    ctx.stroke();

    // Animated particle pulse traveling along the strand
    const t = (phaseSec * (0.35 + (i % 5) * 0.08) + i * 0.13) % 1;
    const py = pts.y0 + t * (pts.y1 - pts.y0);
    const blend = Math.sin(t * Math.PI); // peaks at 1.0 in center chokepoint
    const LinearX = pts.x0 + t * (pts.x1 - pts.x0);
    const px = LinearX * (1 - blend * 0.82) + pts.sx * (blend * 0.82);

    ctx.fillStyle = strandStrokeColor(strand.status, 0.9);
    ctx.beginPath();
    ctx.arc(px, py, strand.status === 'streaming' ? 2.2 : 1.5, 0, Math.PI * 2);
    ctx.fill();
  }

  // 3. Central Singularity (BLAKE3 Interceptor / Causal Chokepoint Glow)
  const cx = width / 2;
  const cy = height / 2;
  const grad = ctx.createRadialGradient(cx, cy, 2, cx, cy, Math.min(90, width * 0.22));
  grad.addColorStop(0, 'rgba(255, 255, 255, 0.92)');
  grad.addColorStop(0.25, 'rgba(34, 211, 238, 0.38)');
  grad.addColorStop(1, 'rgba(7, 7, 9, 0)');
  ctx.fillStyle = grad;
  ctx.beginPath();
  ctx.arc(cx, cy, Math.min(90, width * 0.22), 0, Math.PI * 2);
  ctx.fill();

  // 4. Sweeping Horizontal Laser Scanner Bar + Inverted Callout Pill
  const sweepNorm = 0.5 + 0.36 * Math.sin(phaseSec * 0.95);
  const laserY = Math.round(height * sweepNorm);

  ctx.strokeStyle = 'rgba(255, 255, 255, 0.78)';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.moveTo(0, laserY);
  ctx.bezierCurveTo(width * 0.3, laserY, width * 0.7, laserY, width, laserY);
  ctx.stroke();

  // Inverted monochrome callout pill on the laser line
  const isCompact = width < 400;
  ctx.font = isCompact
    ? '600 9px ui-monospace, SFMono-Regular, Menlo, monospace'
    : '600 10px ui-monospace, SFMono-Regular, Menlo, monospace';
  const banner = snapshot.activePhaseBanner;
  const charW = isCompact ? 5.5 : 6.4;
  const pillW = Math.min(width - (isCompact ? 10 : 16), Math.max(180, banner.length * charW + 18));
  const pillH = isCompact ? 18 : 20;
  const pillX = Math.max(isCompact ? 5 : 8, (width - pillW) / 2);
  const pillY = Math.max(6, Math.min(height - pillH - 6, laserY - pillH / 2));

  ctx.fillStyle = '#f4f4f5';
  ctx.fillRect(pillX, pillY, pillW, pillH);
  ctx.strokeStyle = '#22d3ee';
  ctx.lineWidth = 1;
  ctx.strokeRect(pillX, pillY, pillW, pillH);

  ctx.fillStyle = '#09090b';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(banner, pillX + pillW / 2, pillY + pillH / 2);

  ctx.restore();
}
