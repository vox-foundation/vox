// crates/vox-gui/ui/src/components/gamify/urbs/lightingAndFx.ts
/**
 * Phases 280 & 281 (`URBS-LIGHT-01..04`, `URBS-FX-01..04`):
 * Golden-Hour & Twilight Dual-Key Atmospheric Lighting, Volumetric Crepuscular God-Rays,
 * Inter-Terrace Elevation Mist, Additive Point-Light GI Pools & Live Particle FX Engine.
 */

import type { AgentTask, BuildingState } from '../store';
import type { District, TownLayout } from './types';
import { hash2d } from './pbrMaterials';

type WorldPxFn = (layout: TownLayout, x: number, y: number) => { px: number; py: number };
type ElevFn = (d: District) => number;

/**
 * Projects a soft multi-layer directional twilight shadow polygon onto the ground/terrace plane (`z = z0`)
 * from a South-East (`+u, -v`, 36° altitude) golden-hour sun toward North-West (`-u, +v`).
 */
export function drawProjectedIsoCastShadow(
  ctx: CanvasRenderingContext2D,
  isoPtFn: (px: number, py: number, u: number, v: number, z?: number) => [number, number],
  px: number,
  py: number,
  u0: number,
  v0: number,
  z0: number,
  u1: number,
  v1: number,
  z1: number,
): void {
  const height = z1 - z0;
  if (height <= 1) return;

  // Sun in SE (+u, -v) casts shadow toward NW (-u, +v*0.35)
  const shadowLenU = Math.min(0.85, height * 0.019);
  const shadowLenV = Math.min(0.42, height * 0.008);

  // Outer soft penumbra + inner umbra layers for realistic soft shadow falloff
  const layers: { scale: number; alpha: number }[] = [
    { scale: 1.22, alpha: 0.16 },
    { scale: 0.92, alpha: 0.24 },
    { scale: 0.55, alpha: 0.32 },
  ];

  for (const layer of layers) {
    const du = shadowLenU * layer.scale;
    const dv = shadowLenV * layer.scale;
    const pts: [number, number][] = [
      isoPtFn(px, py, u0 - du, v0 - dv * 0.4, z0),
      isoPtFn(px, py, u1 - du * 0.4, v0 - dv * 0.4, z0),
      isoPtFn(px, py, u1 + 0.05, v1 + 0.05, z0),
      isoPtFn(px, py, u0 - 0.05, v1 + dv, z0),
      isoPtFn(px, py, u0 - du, v1 + dv * 0.6, z0),
    ];

    ctx.beginPath();
    ctx.moveTo(pts[0]![0], pts[0]![1]);
    for (let i = 1; i < pts.length; i++) {
      ctx.lineTo(pts[i]![0], pts[i]![1]);
    }
    ctx.closePath();
    ctx.fillStyle = `rgba(5, 8, 15, ${layer.alpha})`;
    ctx.fill();
  }
}

/**
 * Phase 280 (`URBS-LIGHT-01`): Inter-Terrace Atmospheric Depth Mist.
 * Renders soft aerial-perspective haze cushions in the valleys between L0..L3 terraces
 * so elevated districts pop forward with photographic depth.
 */
export function drawElevationAtmosphericMist(
  ctx: CanvasRenderingContext2D,
  layout: TownLayout,
  worldPxFn: WorldPxFn,
  elevFn: ElevFn,
): void {
  if (typeof ctx.createRadialGradient !== 'function') return;
  ctx.save();
  for (const d of layout.districts) {
    const elev = elevFn(d);
    if (elev <= 0) continue;
    const center = worldPxFn(layout, (d.x0 + d.x1) / 2, d.y1);
    const rx = center.px;
    const ry = center.py - elev * 0.35 + 14;
    const radius = Math.max(90, (d.x1 - d.x0) * 28);

    const mist = ctx.createRadialGradient(rx, ry, radius * 0.1, rx, ry, radius);
    mist.addColorStop(0, 'rgba(148, 163, 184, 0.13)');
    mist.addColorStop(0.55, 'rgba(56, 189, 248, 0.06)');
    mist.addColorStop(1, 'rgba(15, 23, 42, 0)');

    ctx.fillStyle = mist;
    ctx.beginPath();
    ctx.ellipse(rx, ry, radius, radius * 0.42, 0, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}

/**
 * Phase 280 & Phase 289 (`URBS-LIGHT-02`, `URBS-SMOOTH-LIGHT-02`):
 * 2D-Feathered Elliptical Sun-Shafts (Crepuscular God-Rays).
 * Uses rotated elliptical radial gradients (`stop 1.0 = rgba(..., 0)` around the full 360° perimeter)
 * so light shafts feather smoothly in both longitudinal and lateral axes with zero hard polygon edges.
 */
export function drawVolumetricGodRays(
  ctx: CanvasRenderingContext2D,
  layout: TownLayout,
  worldPxFn: WorldPxFn,
  elevFn: ElevFn,
): void {
  if (typeof ctx.createRadialGradient !== 'function') return;
  ctx.save();
  for (const d of layout.districts) {
    if (!d.landmark && (d.layer ?? 1) < 2) continue;
    const elev = elevFn(d);
    const center = worldPxFn(layout, (d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
    const tx = center.px + 24;
    const ty = center.py - elev - 38;
    const majorR = 135;

    ctx.save();
    if (typeof ctx.translate === 'function' && typeof ctx.rotate === 'function' && typeof ctx.scale === 'function') {
      ctx.translate(tx, ty);
      ctx.rotate(-0.68); // SE-to-NW golden-hour sun incidence angle
      ctx.scale(1, 0.28);
      const rayGrad = ctx.createRadialGradient(0, 0, 4, 0, 0, majorR);
      rayGrad.addColorStop(0, 'rgba(254, 243, 199, 0.085)');
      rayGrad.addColorStop(0.35, 'rgba(251, 191, 36, 0.048)');
      rayGrad.addColorStop(0.7, 'rgba(245, 158, 11, 0.018)');
      rayGrad.addColorStop(1, 'rgba(245, 158, 11, 0)');
      ctx.fillStyle = rayGrad;
      ctx.beginPath();
      ctx.arc(0, 0, majorR, 0, Math.PI * 2);
      ctx.fill();
    } else {
      const rayGrad = ctx.createRadialGradient(tx, ty, 4, tx, ty, majorR * 0.6);
      rayGrad.addColorStop(0, 'rgba(254, 243, 199, 0.075)');
      rayGrad.addColorStop(0.5, 'rgba(251, 191, 36, 0.035)');
      rayGrad.addColorStop(1, 'rgba(245, 158, 11, 0)');
      ctx.fillStyle = rayGrad;
      ctx.beginPath();
      ctx.ellipse(tx, ty, majorR * 0.6, majorR * 0.22, -0.68, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }
  ctx.restore();
}

/**
 * Phase 280 & Phase 289 (`URBS-LIGHT-03`, `URBS-SMOOTH-LIGHT-01`):
 * Ground-Plane-Recessed Global Illumination (GI) Bounce Pools.
 * Rendered on the terrace pavement plane BEFORE 3D buildings and wonders are painted,
 * so structures sit physically inside the illuminated plaza and occlude ground light naturally
 * instead of having milky alpha circles painted over their rooftops.
 */
export function drawAdditiveLightPools(
  ctx: CanvasRenderingContext2D,
  layout: TownLayout,
  buildings: Record<string, BuildingState>,
  agentTasks: Record<string, AgentTask>,
  worldPxFn: WorldPxFn,
  elevFn: ElevFn,
): void {
  if (typeof ctx.createRadialGradient !== 'function') return;
  ctx.save();
  const activePaths = new Set(Object.values(agentTasks).map((t) => t.filePath));

  for (const d of layout.districts) {
    const elev = elevFn(d);
    const center = worldPxFn(layout, (d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
    const wx = center.px;
    const wy = center.py - elev;

    // Smooth 5-stop quadratic-falloff ground bounce pool on the terrace stones
    const wonderGlow = ctx.createRadialGradient(wx, wy + 2, 2, wx, wy + 2, 68);
    wonderGlow.addColorStop(0, 'rgba(251, 191, 36, 0.16)');
    wonderGlow.addColorStop(0.32, 'rgba(245, 158, 11, 0.09)');
    wonderGlow.addColorStop(0.65, 'rgba(217, 119, 6, 0.035)');
    wonderGlow.addColorStop(1, 'rgba(217, 119, 6, 0)');
    ctx.fillStyle = wonderGlow;
    ctx.beginPath();
    ctx.ellipse(wx, wy + 2, 68, 34, 0, 0, Math.PI * 2);
    ctx.fill();

    // Active worksite & error fire ground-plane GI pools
    for (const b of d.buildings) {
      const diag = buildings[b.path];
      const hasErr = (diag?.errors ?? 0) > 0;
      const isAct = activePaths.has(b.path) || Boolean(b.isTouched);
      if (!hasErr && !isAct) continue;

      const bPt = worldPxFn(layout, b.x, b.y);
      const bx = bPt.px;
      const by = bPt.py - elev;
      const rad = hasErr ? 42 : 32;
      const g = ctx.createRadialGradient(bx, by, 2, bx, by, rad);
      if (hasErr) {
        g.addColorStop(0, 'rgba(254, 202, 202, 0.22)');
        g.addColorStop(0.4, 'rgba(239, 68, 68, 0.11)');
        g.addColorStop(0.75, 'rgba(185, 28, 28, 0.04)');
        g.addColorStop(1, 'rgba(185, 28, 28, 0)');
      } else {
        g.addColorStop(0, 'rgba(186, 230, 253, 0.18)');
        g.addColorStop(0.45, 'rgba(34, 211, 238, 0.08)');
        g.addColorStop(0.78, 'rgba(6, 182, 212, 0.025)');
        g.addColorStop(1, 'rgba(6, 182, 212, 0)');
      }
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.ellipse(bx, by, rad, rad * 0.5, 0, 0, Math.PI * 2);
      ctx.fill();
    }
  }
  ctx.restore();
}

/**
 * Phase 280 (`URBS-LIGHT-03`): Cinematic Twilight Vignette & Golden-Hour Horizon Grading.
 */
export function drawCinematicPostProcess(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
): void {
  if (typeof ctx.createRadialGradient !== 'function' || width <= 0 || height <= 0) return;
  ctx.save();
  const cx = width * 0.52;
  const cy = height * 0.48;
  const rInner = Math.min(width, height) * 0.32;
  const rOuter = Math.max(width, height) * 0.72;

  const vig = ctx.createRadialGradient(cx, cy, rInner, cx, cy, rOuter);
  vig.addColorStop(0, 'rgba(0, 0, 0, 0)');
  vig.addColorStop(0.72, 'rgba(4, 8, 16, 0.18)');
  vig.addColorStop(1, 'rgba(2, 5, 11, 0.46)');

  ctx.fillStyle = vig;
  ctx.fillRect(0, 0, width, height);
  ctx.restore();
}

/**
 * Phase 281 (`URBS-FX-01`): Deterministic Rising Forge & Error Fire Ember Sparks.
 */
export function drawForgeAndFireEmbers(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  timeMs: number,
  seed = 1,
  count = 9,
): void {
  ctx.save();
  for (let i = 0; i < count; i++) {
    const phaseOffset = hash2d(i, seed, 313);
    const t = ((timeMs * 0.00055 + phaseOffset) % 1 + 1) % 1;
    const driftX = Math.sin(t * Math.PI * 2.5 + i * 1.7) * (6 + i * 1.4) + t * 14;
    const riseY = t * 44;
    const ex = cx + (hash2d(i, seed, 419) - 0.5) * 16 + driftX;
    const ey = cy - riseY;
    const alpha = Math.sin(t * Math.PI) * 0.92;
    const radius = (1 - t * 0.55) * (i % 3 === 0 ? 2.1 : 1.35);

    ctx.fillStyle = `rgba(249, 115, 22, ${(alpha * 0.28).toFixed(3)})`;
    ctx.beginPath();
    ctx.arc(ex, ey, radius * 2.6, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = i % 2 === 0
      ? `rgba(254, 240, 138, ${(alpha * 0.62).toFixed(3)})`
      : `rgba(251, 146, 60, ${(alpha * 0.62).toFixed(3)})`;
    ctx.beginPath();
    ctx.arc(ex, ey, radius * 1.15, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}

/**
 * Phase 281 (`URBS-FX-02`): Soft Volumetric Drifting Smoke Plumes.
 */
export function drawVolumetricSmokePlumes(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  timeMs: number,
  seed = 1,
  isDarkErrorSmoke = false,
): void {
  ctx.save();
  for (let i = 0; i < 5; i++) {
    const phase = ((timeMs * 0.00028 + i * 0.2 + hash2d(i, seed, 521) * 0.1) % 1 + 1) % 1;
    const sx = cx + phase * 28 + Math.sin(phase * 5 + i) * 4;
    const sy = cy - phase * 52;
    const rad = 8 + phase * 22;
    const alpha = Math.sin(phase * Math.PI) * (isDarkErrorSmoke ? 0.34 : 0.18);

    if (typeof ctx.createRadialGradient === 'function') {
      const g = ctx.createRadialGradient(sx, sy, 1, sx, sy, rad);
      if (isDarkErrorSmoke) {
        g.addColorStop(0, `rgba(41, 37, 36, ${alpha.toFixed(3)})`);
        g.addColorStop(0.6, `rgba(28, 25, 23, ${(alpha * 0.55).toFixed(3)})`);
        g.addColorStop(1, 'rgba(28, 25, 23, 0)');
      } else {
        g.addColorStop(0, `rgba(214, 211, 209, ${alpha.toFixed(3)})`);
        g.addColorStop(0.6, `rgba(168, 162, 158, ${(alpha * 0.5).toFixed(3)})`);
        g.addColorStop(1, 'rgba(168, 162, 158, 0)');
      }
      ctx.fillStyle = g;
    } else {
      ctx.fillStyle = `rgba(120, 113, 108, ${(alpha * 0.5).toFixed(3)})`;
    }
    ctx.beginPath();
    ctx.arc(sx, sy, rad, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}

/**
 * Phase 281 (`URBS-FX-02`): Waterfall Mist Spray & Shimmering Harbor Water Caustics.
 */
export function drawWaterCausticsAndSpray(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  timeMs: number,
  seed = 1,
): void {
  ctx.save();
  for (let i = 0; i < 6; i++) {
    const t = ((timeMs * 0.0007 + i * 0.17 + hash2d(i, seed, 613)) % 1 + 1) % 1;
    const sx = x + (hash2d(i, seed, 617) - 0.5) * 22;
    const sy = y - Math.sin(t * Math.PI) * 10;
    const alpha = Math.sin(t * Math.PI) * 0.65;

    ctx.fillStyle = `rgba(186, 230, 253, ${alpha.toFixed(3)})`;
    ctx.beginPath();
    ctx.arc(sx, sy, 1.5 + t * 2.2, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}

/**
 * Phase 281 (`URBS-FX-03`): Active Mason Chisel Sparks & Golden Stone Dust Motes under Treadwheel Cranes.
 */
export function drawMasonChiselSparks(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  timeMs: number,
  seed = 1,
): void {
  ctx.save();
  for (let i = 0; i < 5; i++) {
    const t = ((timeMs * 0.0011 + i * 0.21 + hash2d(i, seed, 719)) % 1 + 1) % 1;
    const angle = -Math.PI * 0.8 + hash2d(i, seed, 727) * Math.PI * 0.6;
    const dist = t * 14;
    const sx = x + Math.cos(angle) * dist;
    const sy = y + Math.sin(angle) * dist + t * t * 9;
    const alpha = (1 - t) * 0.88;

    ctx.fillStyle = i % 2 === 0
      ? `rgba(253, 224, 71, ${alpha.toFixed(3)})`
      : `rgba(134, 239, 172, ${alpha.toFixed(3)})`;
    ctx.beginPath();
    ctx.arc(sx, sy, 1.3, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}
