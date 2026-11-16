/**
 * Deterministic Canvas 2D `Path2D` vector badge primitives for `worldRenderer.ts`
 * and `hydraulicNetwork.ts` — eliminates font-dependent Unicode `ctx.fillText` glyphs.
 */

export interface CanvasBadgeStyle {
  fill?: string;
  stroke?: string;
  lineWidth?: number;
}

export function drawDiamondBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  style: CanvasBadgeStyle,
): void {
  ctx.save();
  ctx.beginPath();
  ctx.moveTo(cx, cy - r);
  ctx.lineTo(cx + r * 0.86, cy);
  ctx.lineTo(cx, cy + r);
  ctx.lineTo(cx - r * 0.86, cy);
  ctx.closePath();
  if (style.fill) {
    ctx.fillStyle = style.fill;
    ctx.fill();
  }
  if (style.stroke) {
    ctx.strokeStyle = style.stroke;
    ctx.lineWidth = style.lineWidth ?? 1.25;
    ctx.stroke();
  }
  ctx.restore();
}

export function drawTriangleUpBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  style: CanvasBadgeStyle,
): void {
  ctx.save();
  ctx.beginPath();
  ctx.moveTo(cx, cy - r);
  ctx.lineTo(cx + r * 0.92, cy + r * 0.78);
  ctx.lineTo(cx - r * 0.92, cy + r * 0.78);
  ctx.closePath();
  if (style.fill) {
    ctx.fillStyle = style.fill;
    ctx.fill();
  }
  if (style.stroke) {
    ctx.strokeStyle = style.stroke;
    ctx.lineWidth = style.lineWidth ?? 1.25;
    ctx.stroke();
  }
  ctx.restore();
}

export function drawArrowUpBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  strokeColor: string,
  lineWidth = 1.5,
): void {
  ctx.save();
  ctx.strokeStyle = strokeColor;
  ctx.lineWidth = lineWidth;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.beginPath();
  ctx.moveTo(cx, cy + r * 0.85);
  ctx.lineTo(cx, cy - r * 0.85);
  ctx.moveTo(cx - r * 0.65, cy - r * 0.2);
  ctx.lineTo(cx, cy - r * 0.85);
  ctx.lineTo(cx + r * 0.65, cy - r * 0.2);
  ctx.stroke();
  ctx.restore();
}

export function drawArrowRightBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  strokeColor: string,
  lineWidth = 1.5,
): void {
  ctx.save();
  ctx.strokeStyle = strokeColor;
  ctx.lineWidth = lineWidth;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.beginPath();
  ctx.moveTo(cx - r * 0.85, cy);
  ctx.lineTo(cx + r * 0.85, cy);
  ctx.moveTo(cx + r * 0.2, cy - r * 0.65);
  ctx.lineTo(cx + r * 0.85, cy);
  ctx.lineTo(cx + r * 0.2, cy + r * 0.65);
  ctx.stroke();
  ctx.restore();
}

export function drawChevronRightBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  strokeColor: string,
  lineWidth = 1.6,
): void {
  ctx.save();
  ctx.strokeStyle = strokeColor;
  ctx.lineWidth = lineWidth;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.beginPath();
  ctx.moveTo(cx - r * 0.4, cy - r * 0.75);
  ctx.lineTo(cx + r * 0.45, cy);
  ctx.lineTo(cx - r * 0.4, cy + r * 0.75);
  ctx.stroke();
  ctx.restore();
}

export function drawCheckBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  strokeColor: string,
  lineWidth = 1.6,
): void {
  ctx.save();
  ctx.strokeStyle = strokeColor;
  ctx.lineWidth = lineWidth;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.beginPath();
  ctx.moveTo(cx - r * 0.75, cy + r * 0.05);
  ctx.lineTo(cx - r * 0.18, cy + r * 0.65);
  ctx.lineTo(cx + r * 0.8, cy - r * 0.6);
  ctx.stroke();
  ctx.restore();
}

export function drawCrossBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  strokeColor: string,
  lineWidth = 1.6,
): void {
  ctx.save();
  ctx.strokeStyle = strokeColor;
  ctx.lineWidth = lineWidth;
  ctx.lineCap = 'round';
  ctx.beginPath();
  ctx.moveTo(cx - r * 0.65, cy - r * 0.65);
  ctx.lineTo(cx + r * 0.65, cy + r * 0.65);
  ctx.moveTo(cx + r * 0.65, cy - r * 0.65);
  ctx.lineTo(cx - r * 0.65, cy + r * 0.65);
  ctx.stroke();
  ctx.restore();
}

export function drawTargetRingBadge(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  color: string,
  filledCenter = true,
): void {
  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.35;
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.stroke();
  ctx.beginPath();
  ctx.arc(cx, cy, r * 0.42, 0, Math.PI * 2);
  if (filledCenter) {
    ctx.fillStyle = color;
    ctx.fill();
  } else {
    ctx.stroke();
  }
  ctx.restore();
}
