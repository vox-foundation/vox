// crates/vox-gui/ui/src/components/gamify/urbs/hydraulicNetwork.ts
/**
 * Phases 292, 293 & 294 (`URBS-HYDRAULIC-01..04`, `URBS-CONDUIT-ANIM-01..04`, `URBS-SEMIOTICS-CV-01..02`):
 * Integrated Roman Hydraulic Plumbing (`Aquaeductus`, `Castellum Divisorium`, `Fistulae` Supply Pipes,
 * `Epistomia` Bronze Sluice Valves) & `Cloaca Maxima` Arched Stone Sewer System, paired with
 * Multi-Channel Color + Geometric Shape Animated Activity Conduits:
 *   1. `AQUA PURA` (`supply`)            — Cyan (`#38bdf8`)    + Chevron Wave-Crests (`❯❯`)
 *   2. `IGNIS OPERIS` (`work_in_progress`) — Gold (`#f59e0b`)    + Diamond / Rhombus Ingots (`◆◆`)
 *   3. `CLOACA FAULT` (`error_fault`)    — Crimson (`#ef4444`) + Warning Triangles (`▲`) & Eddies (`◎`)
 *   4. `VIA VICTORIAE` (`verified_ci`)   — Emerald (`#10b981`) + Circular Laurel Seals (`◉`)
 */

import type { AgentTask, BuildingState } from '../store';
import type { District, TownLayout } from './types';
import type { WorldState } from './worldRenderer';
import { computeBlastRadius } from './topology';

export type WorldPxFn = (layout: TownLayout, x: number, y: number) => { px: number; py: number };
export type ElevFn = (d: District) => number;
export type ConduitChannelKind = 'supply' | 'work_in_progress' | 'error_fault' | 'verified_ci';

export interface HydraulicConduitSegment {
  id: string;
  kind: ConduitChannelKind;
  waypoints: [number, number][];
  label?: string;
}

function strokeSegment(
  ctx: CanvasRenderingContext2D,
  a: [number, number],
  b: [number, number],
  color: string,
  width: number,
): void {
  ctx.strokeStyle = color;
  ctx.lineWidth = width;
  ctx.beginPath();
  ctx.moveTo(a[0], a[1]);
  ctx.lineTo(b[0], b[1]);
  ctx.stroke();
}

/**
 * Computes an isometric-aligned intermediate elbow point between `from` and `to`
 * so plumbing/sewer conduits route along Roman street & terrace axes (`2:1` isometric)
 * rather than cutting diagonally across rooftops at arbitrary screen angles.
 */
export function computeIsoConduitWaypoints(
  from: [number, number],
  to: [number, number],
): [number, number][] {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  if (Math.hypot(dx, dy) < 28) {
    return [from, to];
  }
  // Route via a 2:1 isometric elbow (along u/v street grid)
  const midX = from[0] + dx * 0.55;
  const isoStepY = Math.sign(dy || 1) * Math.min(Math.abs(dy) * 0.72, Math.abs(dx) * 0.28);
  const elbow: [number, number] = [midX, from[1] + isoStepY];
  return [from, elbow, to];
}

/**
 * Interpolates a position `[x, y]` and tangent angle `theta` along a multi-segment polyline for `t ∈ [0, 1]`.
 */
export function samplePolylineAt(
  waypoints: [number, number][],
  t: number,
): { x: number; y: number; angle: number } {
  if (waypoints.length === 0) return { x: 0, y: 0, angle: 0 };
  if (waypoints.length === 1) return { x: waypoints[0]![0], y: waypoints[0]![1], angle: 0 };

  const clamped = Math.max(0, Math.min(1, t));
  const segLengths: number[] = [];
  let total = 0;
  for (let i = 0; i < waypoints.length - 1; i++) {
    const a = waypoints[i]!;
    const b = waypoints[i + 1]!;
    const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
    segLengths.push(len);
    total += len;
  }
  if (total <= 0.001) {
    const a = waypoints[0]!;
    const b = waypoints[waypoints.length - 1]!;
    return { x: a[0], y: a[1], angle: Math.atan2(b[1] - a[1], b[0] - a[0]) };
  }

  let targetDist = clamped * total;
  for (let i = 0; i < segLengths.length; i++) {
    const len = segLengths[i]!;
    const a = waypoints[i]!;
    const b = waypoints[i + 1]!;
    if (targetDist <= len || i === segLengths.length - 1) {
      const localT = len > 0.001 ? targetDist / len : 0;
      return {
        x: a[0] + (b[0] - a[0]) * localT,
        y: a[1] + (b[1] - a[1]) * localT,
        angle: Math.atan2(b[1] - a[1], b[0] - a[0]),
      };
    }
    targetDist -= len;
  }
  const last = waypoints[waypoints.length - 1]!;
  return { x: last[0], y: last[1], angle: 0 };
}

/**
 * Draws a Bronze Hydraulic Junction Manifold (`Calix` / `Epistomium` Valve) at a conduit elbow or header.
 */
export function drawBronzeValveJunction(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  kind: ConduitChannelKind,
  radius = 4.2,
): void {
  const ringColor =
    kind === 'error_fault'
      ? '#ef4444'
      : kind === 'work_in_progress'
        ? '#f59e0b'
        : kind === 'verified_ci'
          ? '#10b981'
          : '#d4af37';

  ctx.save();
  ctx.fillStyle = '#29180d';
  ctx.strokeStyle = '#d97706';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = ringColor;
  ctx.beginPath();
  ctx.arc(x, y, radius * 0.52, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

/**
 * Traces a polyline path on `ctx` with smooth rounded isometric elbow fillets (`quadraticCurveTo`)
 * so Roman bronze `Fistulae` pipes and `Cloaca Maxima` trenches bend smoothly without sharp vertices.
 */
export function traceRoundedPolylinePath(
  ctx: CanvasRenderingContext2D,
  waypoints: [number, number][],
  cornerRadius = 14,
): void {
  if (waypoints.length === 0) return;
  ctx.beginPath();
  ctx.moveTo(waypoints[0]![0], waypoints[0]![1]);
  if (waypoints.length === 2 || typeof ctx.quadraticCurveTo !== 'function') {
    for (let i = 1; i < waypoints.length; i++) {
      ctx.lineTo(waypoints[i]![0], waypoints[i]![1]);
    }
    return;
  }

  for (let i = 1; i < waypoints.length - 1; i++) {
    const prev = waypoints[i - 1]!;
    const curr = waypoints[i]!;
    const next = waypoints[i + 1]!;

    const dx1 = curr[0] - prev[0];
    const dy1 = curr[1] - prev[1];
    const len1 = Math.hypot(dx1, dy1);

    const dx2 = next[0] - curr[0];
    const dy2 = next[1] - curr[1];
    const len2 = Math.hypot(dx2, dy2);

    const r = Math.min(cornerRadius, len1 * 0.42, len2 * 0.42);
    if (r > 1.5 && len1 > 0.01 && len2 > 0.01) {
      const inX = curr[0] - (dx1 / len1) * r;
      const inY = curr[1] - (dy1 / len1) * r;
      const outX = curr[0] + (dx2 / len2) * r;
      const outY = curr[1] + (dy2 / len2) * r;
      ctx.lineTo(inX, inY);
      ctx.quadraticCurveTo(curr[0], curr[1], outX, outY);
    } else {
      ctx.lineTo(curr[0], curr[1]);
    }
  }
  const last = waypoints[waypoints.length - 1]!;
  ctx.lineTo(last[0], last[1]);
}

/** Plan alias for `traceRoundedPolylinePath` (`URBS-PLUMBING-3D-02` / Phase 298). */
export function strokeRoundedPolyline(
  ctx: CanvasRenderingContext2D,
  waypoints: [number, number][],
  cornerRadius = 14,
): void {
  traceRoundedPolylinePath(ctx, waypoints, cornerRadius);
}

/**
 * Scales conduit gauge & particle cadence by live activity intensity (`URBS-FLOW-SEMIOTICS-02`).
 * Returns a multiplier in `[0.72, 1.85]`.
 */
export function computeConduitActivityLoadScale(input: {
  mcpOk?: number;
  mcpTotal?: number;
  churnLines?: number;
  errorCount?: number;
  runnerCount?: number;
  activeTaskCount?: number;
}): number {
  const mcpHealth = input.mcpTotal && input.mcpTotal > 0
    ? (input.mcpOk ?? 0) / input.mcpTotal
    : 0.6;
  const churn = Math.min(1, (input.churnLines ?? 0) / 120);
  const errors = Math.min(1, (input.errorCount ?? 0) / 6);
  const runners = Math.min(1, (input.runnerCount ?? 0) / 4);
  const tasks = Math.min(1, (input.activeTaskCount ?? 0) / 5);
  const raw = 0.72 + mcpHealth * 0.22 + churn * 0.28 + errors * 0.35 + runners * 0.18 + tasks * 0.22;
  return Math.max(0.72, Math.min(1.85, Number(raw.toFixed(3))));
}

/**
 * Draws a multi-segment Roman hydraulic pipe or sewer trench along `waypoints`
 * with recessed ground trench shadow, smooth rounded elbow fillets, and 3D cylindrical highlights.
 */
export function drawConduitTrack(
  ctx: CanvasRenderingContext2D,
  waypoints: [number, number][],
  kind: ConduitChannelKind,
  widthScale = 1,
): void {
  if (waypoints.length < 2) return;
  ctx.save();

  const casingColor =
    kind === 'error_fault'
      ? '#3b1212'
      : kind === 'work_in_progress'
        ? '#451a03'
        : kind === 'verified_ci'
          ? '#064e3b'
          : '#3a2e1b';
  const railColor =
    kind === 'error_fault'
      ? 'rgba(239, 68, 68, 0.80)'
      : kind === 'work_in_progress'
        ? 'rgba(245, 158, 11, 0.84)'
        : kind === 'verified_ci'
          ? 'rgba(16, 185, 129, 0.80)'
          : 'rgba(212, 175, 55, 0.82)';
  const coreColor =
    kind === 'error_fault'
      ? 'rgba(254, 202, 202, 0.65)'
      : kind === 'work_in_progress'
        ? 'rgba(254, 240, 138, 0.70)'
        : kind === 'verified_ci'
          ? 'rgba(167, 243, 208, 0.65)'
          : 'rgba(254, 243, 199, 0.72)';

  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';

  // 0. Soft recessed ground trench shadow
  ctx.strokeStyle = 'rgba(8, 6, 5, 0.48)';
  ctx.lineWidth = 7.4 * widthScale;
  traceRoundedPolylinePath(ctx, waypoints, 14);
  ctx.stroke();

  // 1. Stone/bronze trench casing
  ctx.strokeStyle = casingColor;
  ctx.lineWidth = 5.2 * widthScale;
  traceRoundedPolylinePath(ctx, waypoints, 14);
  ctx.stroke();

  // 2. Semantic pressurized fluid / hypocaust channel
  ctx.strokeStyle = railColor;
  ctx.lineWidth = 2.8 * widthScale;
  ctx.stroke();

  // 3. Specular fluid core
  ctx.strokeStyle = coreColor;
  ctx.lineWidth = 1.15 * widthScale;
  ctx.stroke();

  // 4. Collared bronze flange rings & valve junctions at elbows
  for (let i = 0; i < waypoints.length; i++) {
    const pt = waypoints[i]!;
    if (i > 0 && i < waypoints.length - 1) {
      drawBronzeValveJunction(ctx, pt[0], pt[1], kind, 3.8 * widthScale);
    }
  }
  ctx.restore();
}

// ── 4 Geometric Shape Particle Renderers (`URBS-CONDUIT-ANIM-01..04` & `URBS-FLOW-SEMIOTICS-01`) ──────

function rotPt(
  x: number,
  y: number,
  lx: number,
  ly: number,
  cos: number,
  sin: number,
  scale: number,
): [number, number] {
  return [
    x + (lx * cos - ly * sin) * scale,
    y + (lx * sin + ly * cos) * scale,
  ];
}

function drawParticleHaloAndTrail(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  scale: number,
  haloRgb: string,
  trailRgba: string,
): void {
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const tailPt = rotPt(x, y, -11.5, 0, cos, sin, scale);
  const sideA = rotPt(x, y, -2.5, -2.2, cos, sin, scale);
  const sideB = rotPt(x, y, -2.5, 2.2, cos, sin, scale);

  // Tapered wake/comet trail
  ctx.fillStyle = trailRgba;
  ctx.beginPath();
  ctx.moveTo(tailPt[0], tailPt[1]);
  ctx.lineTo(sideA[0], sideA[1]);
  ctx.lineTo(sideB[0], sideB[1]);
  ctx.closePath();
  ctx.fill();

  // Soft radial glow halo
  const r = 7.5 * scale;
  if (typeof ctx.createRadialGradient === 'function') {
    const g = ctx.createRadialGradient(x, y, 0.5, x, y, r);
    g.addColorStop(0, `rgba(${haloRgb}, 0.22)`);
    g.addColorStop(0.55, `rgba(${haloRgb}, 0.08)`);
    g.addColorStop(1, `rgba(${haloRgb}, 0)`);
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();
  }
}

/** Shape 1 (`AQUA PURA` / `supply`): Crystalline Cyan Chevron Wave-Crest (`❯`). */
export function drawChevronWaveParticle(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  scale = 1,
): void {
  ctx.save();
  drawParticleHaloAndTrail(ctx, x, y, angle, scale, '56, 189, 248', 'rgba(56, 189, 248, 0.30)');
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const p0 = rotPt(x, y, 4.8, 0, cos, sin, scale);
  const p1 = rotPt(x, y, -3.2, -3.8, cos, sin, scale);
  const p2 = rotPt(x, y, -1.0, 0, cos, sin, scale);
  const p3 = rotPt(x, y, -3.2, 3.8, cos, sin, scale);

  ctx.fillStyle = '#e0f2fe';
  ctx.strokeStyle = '#0284c7';
  ctx.lineWidth = 0.9;
  ctx.beginPath();
  ctx.moveTo(p0[0], p0[1]);
  ctx.lineTo(p1[0], p1[1]);
  ctx.lineTo(p2[0], p2[1]);
  ctx.lineTo(p3[0], p3[1]);
  ctx.closePath();
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

/** Shape 2 (`IGNIS OPERIS` / `work_in_progress`): Molten Gold Diamond / Rhombus Ingot (`◆`). */
export function drawDiamondIngotParticle(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  scale = 1,
): void {
  ctx.save();
  drawParticleHaloAndTrail(ctx, x, y, angle, scale, '251, 191, 36', 'rgba(251, 191, 36, 0.32)');
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const p0 = rotPt(x, y, 5.0, 0, cos, sin, scale);
  const p1 = rotPt(x, y, 0, -3.6, cos, sin, scale);
  const p2 = rotPt(x, y, -5.0, 0, cos, sin, scale);
  const p3 = rotPt(x, y, 0, 3.6, cos, sin, scale);

  ctx.fillStyle = '#fef08a';
  ctx.strokeStyle = '#d97706';
  ctx.lineWidth = 1.0;
  ctx.beginPath();
  ctx.moveTo(p0[0], p0[1]);
  ctx.lineTo(p1[0], p1[1]);
  ctx.lineTo(p2[0], p2[1]);
  ctx.lineTo(p3[0], p3[1]);
  ctx.closePath();
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

/** Shape 3 (`CLOACA FAULT` / `error_fault`): Crimson Spiked Warning Triangle (`▲`) & Backpressure Eddy Ring (`◎`). */
export function drawWarningTriangleParticle(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  scale = 1,
): void {
  ctx.save();
  drawParticleHaloAndTrail(ctx, x, y, angle, scale, '239, 68, 68', 'rgba(239, 68, 68, 0.34)');
  // Outer turbulent crimson backpressure eddy ring
  ctx.strokeStyle = 'rgba(248, 113, 113, 0.65)';
  ctx.lineWidth = 1.0;
  ctx.beginPath();
  ctx.arc(x, y, 5.6 * scale, 0, Math.PI * 2);
  ctx.stroke();

  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const p0 = rotPt(x, y, 4.8, 0, cos, sin, scale);
  const p1 = rotPt(x, y, -3.6, -4.0, cos, sin, scale);
  const p2 = rotPt(x, y, -3.6, 4.0, cos, sin, scale);

  ctx.fillStyle = '#fca5a5';
  ctx.strokeStyle = '#b91c1c';
  ctx.lineWidth = 1.0;
  ctx.beginPath();
  ctx.moveTo(p0[0], p0[1]);
  ctx.lineTo(p1[0], p1[1]);
  ctx.lineTo(p2[0], p2[1]);
  ctx.closePath();
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

/** Shape 4 (`VIA VICTORIAE` / `verified_ci`): Emerald Circular Laurel Medallion Seal (`◉`). */
export function drawLaurelSealParticle(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  scale = 1,
): void {
  ctx.save();
  drawParticleHaloAndTrail(ctx, x, y, 0, scale, '16, 185, 129', 'rgba(16, 185, 129, 0.28)');
  ctx.fillStyle = '#10b981';
  ctx.strokeStyle = '#ecfdf5';
  ctx.lineWidth = 1.1;
  ctx.beginPath();
  ctx.arc(x, y, 3.8 * scale, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = '#d1fae5';
  ctx.beginPath();
  ctx.arc(x, y, 1.5 * scale, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

/**
 * Phases 292 & 298 (`URBS-HYDRAULIC-01..04`, `URBS-PLUMBING-3D-01..04`):
 * Renders the physical Roman Hydraulic Supply (`Aqua Claudia` 3D multi-arch trunk, `Castellum Divisorium` header tank,
 * `Fistulae` bronze/lead street pipes, `Epistomia` sluice valves, sub-building inter-district conduits, and stepped
 * stone `Cataracta` cascades) and `Cloaca Maxima` Sewer System on `Layer 0.85` (BENEATH 3D buildings so rooftops remain 100% clean).
 */
export function drawRomanHydraulicInfrastructure(
  ctx: CanvasRenderingContext2D,
  layout: TownLayout,
  buildings: Record<string, BuildingState>,
  agentTasks: Record<string, AgentTask>,
  worldPxFn: WorldPxFn,
  elevFn: ElevFn,
): void {
  ctx.save();
  const activePaths = new Set(Object.values(agentTasks).map((t) => t.filePath));
  const byName = new Map(layout.districts.map((d) => [d.name, d]));
  const sortedDistricts = [...layout.districts].sort(
    (a, b) => (a.x0 + a.x1 + a.y0 + a.y1) - (b.x0 + b.x1 + b.y0 + b.y1),
  );
  const totalErrors = Object.values(buildings).reduce((n, b) => n + (b.errors ?? 0), 0);
  const loadScale = computeConduitActivityLoadScale({
    errorCount: totalErrors,
    activeTaskCount: activePaths.size,
    churnLines: totalErrors * 18 + activePaths.size * 12,
  });

  // 1. Connect Peripheral `AQVAE MCP` Aqueduct via a Continuous 3D Multi-Arch Stone Viaduct (`Aqua Claudia`)
  //    into the Upper-Terrace `Castellum Divisorium` (`URBS-PLUMBING-3D-01`)
  const aqvaePt = worldPxFn(layout, layout.landmarks.aqvae.x, layout.landmarks.aqvae.y);
  const upperDistrict =
    sortedDistricts.find((d) => (d.layer ?? 1) >= 2 || d.landmark) ?? sortedDistricts[0];

  if (upperDistrict) {
    const uElev = elevFn(upperDistrict);
    const castellumAnchor = worldPxFn(
      layout,
      upperDistrict.x1 - 0.65,
      (upperDistrict.y0 + upperDistrict.y1) / 2,
    );
    const cx = castellumAnchor.px;
    const cy = castellumAnchor.py - uElev;

    const aqStart: [number, number] = [aqvaePt.px - 24, aqvaePt.py - 30];
    const aqEnd: [number, number] = [cx, cy - 10];
    const spans = 6;
    for (let i = 0; i < spans; i++) {
      const t0 = i / spans;
      const t1 = (i + 1) / spans;
      const x0 = aqStart[0] + (aqEnd[0] - aqStart[0]) * t0;
      const y0 = aqStart[1] + (aqEnd[1] - aqStart[1]) * t0;
      const x1 = aqStart[0] + (aqEnd[0] - aqStart[0]) * t1;
      const y1 = aqStart[1] + (aqEnd[1] - aqStart[1]) * t1;
      const xm = (x0 + x1) * 0.5;
      const ym = (y0 + y1) * 0.5;

      // Upper spandrel masonry deck
      ctx.fillStyle = '#85796c';
      ctx.strokeStyle = 'rgba(28, 25, 23, 0.55)';
      ctx.lineWidth = 0.8;
      ctx.beginPath();
      ctx.moveTo(x0, y0);
      ctx.lineTo(x1, y1);
      ctx.lineTo(x1, y1 + 9);
      ctx.lineTo(x0, y0 + 9);
      ctx.closePath();
      ctx.fill();
      ctx.stroke();

      // 3D Stone Pier with shaded SW and sunlit SE faces
      ctx.fillStyle = '#4e463e';
      ctx.fillRect(x0 - 3.5, y0 + 2, 3.5, 22);
      ctx.fillStyle = '#786d61';
      ctx.fillRect(x0, y0 + 2, 3.5, 22);

      // Vaulted stone arch opening + cyan water bounce reflection
      ctx.fillStyle = 'rgba(34, 211, 238, 0.38)';
      ctx.beginPath();
      ctx.arc(xm, ym + 9, 5.2, Math.PI, 0, false);
      ctx.fill();
    }
    drawConduitTrack(ctx, [aqStart, aqEnd], 'supply', 1.05 * loadScale);
    const aqMid = samplePolylineAt([aqStart, aqEnd], 0.5);
    drawChevronWaveParticle(ctx, aqMid.x, aqMid.y, aqMid.angle, 0.95);

    // 3D Domed Travertine & Bronze `Castellum Divisorium` (Main Water Distribution Header Tank)
    ctx.fillStyle = 'rgba(8, 7, 6, 0.45)';
    ctx.beginPath();
    ctx.ellipse(cx + 2, cy - 3, 15, 8, 0, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = '#63594f';
    ctx.strokeStyle = 'rgba(214, 200, 182, 0.55)';
    ctx.lineWidth = 1.0;
    ctx.beginPath();
    ctx.ellipse(cx, cy - 6, 14, 7.5, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();

    ctx.fillStyle = '#0284c7';
    ctx.strokeStyle = '#fbbf24';
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.ellipse(cx, cy - 8, 10, 5.2, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }

  // 2. Sub-Building Ground-Layer Inter-District Dependency & Blast-Radius Conduits (`URBS-PLUMBING-3D-03..04`)
  for (const d of sortedDistricts) {
    const elev = elevFn(d);
    const cRaw = worldPxFn(layout, (d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
    const cPt: [number, number] = [cRaw.px, cRaw.py - elev + 4];
    const errCount = d.buildings.reduce((n, b) => n + (buildings[b.path]?.errors ?? 0), 0);

    for (const depName of d.deps ?? []) {
      const target = byName.get(depName);
      if (!target) continue;
      const tElev = elevFn(target);
      const tRaw = worldPxFn(layout, (target.x0 + target.x1) / 2, (target.y0 + target.y1) / 2);
      const tPt: [number, number] = [tRaw.px, tRaw.py - tElev + 4];
      const targetErr = target.buildings.reduce((n, b) => n + (buildings[b.path]?.errors ?? 0), 0);

      let kind: ConduitChannelKind = 'supply';
      if (errCount > 0 || targetErr > 0) {
        kind = 'error_fault';
      } else if (
        d.buildings.some((b) => activePaths.has(b.path) || b.isTouched) ||
        target.buildings.some((b) => activePaths.has(b.path) || b.isTouched)
      ) {
        kind = 'work_in_progress';
      }

      const wps = computeIsoConduitWaypoints(tPt, cPt);
      drawConduitTrack(ctx, wps, kind, 0.92);

      // Stepped Stone `Cataracta` Drop-Basin at the intermediate elbow when elevation changes (`URBS-PLUMBING-3D-04`)
      if (wps.length >= 3 && Math.abs(tElev - elev) > 2) {
        const elbow = wps[1]!;
        ctx.fillStyle = '#4e463e';
        ctx.strokeStyle = 'rgba(217, 119, 6, 0.65)';
        ctx.lineWidth = 0.9;
        ctx.beginPath();
        ctx.ellipse(elbow[0], elbow[1], 7.5, 4.2, 0, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
      }

      // Static mid-span shape+color token for static screenshot & overview scannability
      const mid = samplePolylineAt(wps, 0.5);
      if (kind === 'error_fault') {
        drawWarningTriangleParticle(ctx, mid.x, mid.y, mid.angle, 0.9);
      } else if (kind === 'work_in_progress') {
        drawDiamondIngotParticle(ctx, mid.x, mid.y, mid.angle, 0.9);
      } else {
        drawChevronWaveParticle(ctx, mid.x, mid.y, mid.angle, 0.9);
      }
    }
  }

  // 3. Per-District `Fistulae` Supply Conduits, `Cloaca Maxima` Sewer Trenches & Cliff Sea Outfalls (`URBS-HYDRAULIC-02..04`)
  for (const d of sortedDistricts) {
    const elev = elevFn(d);
    const cx = (d.x0 + d.x1) / 2;
    const cy = (d.y0 + d.y1) / 2;
    const centerRaw = worldPxFn(layout, cx, cy);
    const centerPt: [number, number] = [centerRaw.px, centerRaw.py - elev + 4];

    const nRaw = worldPxFn(layout, cx, d.y0 + 0.35);
    const sRaw = worldPxFn(layout, cx, d.y1 - 0.35);
    const wRaw = worldPxFn(layout, d.x0 + 0.35, cy);
    const eRaw = worldPxFn(layout, d.x1 - 0.35, cy);

    const nPt: [number, number] = [nRaw.px, nRaw.py - elev];
    const sPt: [number, number] = [sRaw.px, sRaw.py - elev];
    const wPt: [number, number] = [wRaw.px, wRaw.py - elev];
    const ePt: [number, number] = [eRaw.px, eRaw.py - elev];

    const errCount = d.buildings.reduce((n, b) => n + (buildings[b.path]?.errors ?? 0), 0);
    const hasActiveWork = d.buildings.some((b) => activePaths.has(b.path) || b.isTouched);

    // 3a. Recessed `Cloaca Maxima` Sewer Trench along the Cardo street axis (`nPt -> sPt`)
    strokeSegment(ctx, nPt, sPt, 'rgba(24, 21, 18, 0.82)', 4.4);
    strokeSegment(
      ctx,
      nPt,
      sPt,
      errCount > 0 ? 'rgba(239, 68, 68, 0.65)' : 'rgba(14, 116, 144, 0.55)',
      2.0,
    );

    // 3b. Pressurized `Fistulae` Bronze/Aqua Supply Pipe along the Decumanus street axis (`wPt -> ePt`)
    strokeSegment(ctx, wPt, ePt, 'rgba(59, 33, 11, 0.82)', 4.0);
    strokeSegment(
      ctx,
      wPt,
      ePt,
      hasActiveWork ? 'rgba(245, 158, 11, 0.72)' : 'rgba(56, 189, 248, 0.65)',
      1.9,
    );

    // 3c. Circular Bronze Street Manhole Grates (`Spiracula`) at the Forum crossroads & street ends
    for (const gratePt of [centerPt, nPt, sPt, wPt, ePt]) {
      ctx.fillStyle = '#29180d';
      ctx.strokeStyle = '#d97706';
      ctx.lineWidth = 0.9;
      ctx.beginPath();
      ctx.ellipse(gratePt[0], gratePt[1], 3.8, 2.0, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }

    // 3d. Branch `Fistulae` / `Cloaca` Lateral Service Pipes directly into Active or Failing File Worksites
    for (const b of d.buildings) {
      const bErr = (buildings[b.path]?.errors ?? 0) > 0;
      const bAct = activePaths.has(b.path) || Boolean(b.isTouched);
      if (!bErr && !bAct) continue;

      const bRaw = worldPxFn(layout, b.x, b.y);
      const bPt: [number, number] = [bRaw.px, bRaw.py - elev + 2];
      const lateralWps = computeIsoConduitWaypoints(centerPt, bPt);
      drawConduitTrack(ctx, lateralWps, bErr ? 'error_fault' : 'work_in_progress', 0.72 * loadScale);
    }

    // 3e. Arched Stone `Cloaca Maxima` Sea Outfall Portal at the Base of the District's Front Cliff Wall (`URBS-HYDRAULIC-04`)
    const cliffH = 26 + elev;
    const outfallX = sPt[0] - 18;
    const outfallY = sPt[1] + cliffH - 6;

    // Carved stone arch portal in the lower cliff wall
    ctx.fillStyle = '#4a423a';
    ctx.strokeStyle = 'rgba(214, 200, 182, 0.38)';
    ctx.lineWidth = 0.9;
    ctx.beginPath();
    ctx.arc(outfallX, outfallY, 6.8, Math.PI, 0, false);
    ctx.lineTo(outfallX + 6.8, outfallY + 5);
    ctx.lineTo(outfallX - 6.8, outfallY + 5);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    // Dark recessed sewer vault tunnel mouth
    ctx.fillStyle = '#0c0a09';
    ctx.beginPath();
    ctx.arc(outfallX, outfallY + 1, 4.2, Math.PI, 0, false);
    ctx.lineTo(outfallX + 4.2, outfallY + 4.5);
    ctx.lineTo(outfallX - 4.2, outfallY + 4.5);
    ctx.closePath();
    ctx.fill();

    // Vertical drop-shaft from the street `Cloaca` trench (`sPt`) to the cliff outfall arch + feathered sea discharge plume
    strokeSegment(
      ctx,
      sPt,
      [outfallX, outfallY - 4],
      errCount > 0 ? 'rgba(239, 68, 68, 0.52)' : 'rgba(34, 211, 238, 0.40)',
      2.0,
    );
    if (typeof ctx.createRadialGradient === 'function') {
      const plumeG = ctx.createRadialGradient(outfallX, outfallY + 7, 1, outfallX, outfallY + 7, 14);
      if (errCount > 0) {
        plumeG.addColorStop(0, 'rgba(239, 68, 68, 0.46)');
        plumeG.addColorStop(0.55, 'rgba(185, 28, 28, 0.18)');
        plumeG.addColorStop(1, 'rgba(185, 28, 28, 0)');
      } else {
        plumeG.addColorStop(0, 'rgba(56, 189, 248, 0.42)');
        plumeG.addColorStop(0.55, 'rgba(14, 116, 144, 0.16)');
        plumeG.addColorStop(1, 'rgba(14, 116, 144, 0)');
      }
      ctx.fillStyle = plumeG;
    } else {
      ctx.fillStyle = errCount > 0 ? 'rgba(239, 68, 68, 0.36)' : 'rgba(56, 189, 248, 0.32)';
    }
    ctx.beginPath();
    ctx.ellipse(outfallX, outfallY + 7, 14, 6, 0, 0, Math.PI * 2);
    ctx.fill();
  }

  ctx.restore();
}

/**
 * Builds the active list of hydraulic/sewer conduit segments connecting landmarks, districts,
 * and active/failing worksites for `s: WorldState`.
 */
export function buildActiveHydraulicConduits(
  s: WorldState,
  worldPxFn: WorldPxFn,
  elevFn: ElevFn,
): HydraulicConduitSegment[] {
  const { layout } = s;
  const segments: HydraulicConduitSegment[] = [];
  const activePaths = new Set(Object.values(s.agentTasks).map((t) => t.filePath));
  const byName = new Map(layout.districts.map((d) => [d.name, d]));

  // 1. AQVAE MCP -> Upper Terrace Castellum Divisorium (`supply`)
  const sortedDistricts = [...layout.districts].sort(
    (a, b) => (a.x0 + a.x1 + a.y0 + a.y1) - (b.x0 + b.x1 + b.y0 + b.y1),
  );
  const upperD = sortedDistricts.find((d) => (d.layer ?? 1) >= 2 || d.landmark) ?? sortedDistricts[0];
  if (upperD) {
    const aqPt = worldPxFn(layout, layout.landmarks.aqvae.x, layout.landmarks.aqvae.y);
    const castPt = worldPxFn(layout, upperD.x1 - 0.65, (upperD.y0 + upperD.y1) / 2);
    const uElev = elevFn(upperD);
    segments.push({
      id: 'aqvae-trunk',
      kind: 'supply',
      waypoints: [
        [aqPt.px - 18, aqPt.py - 32],
        [castPt.px, castPt.py - uElev - 10],
      ],
      label: 'AQVAE → CASTELLVM',
    });
  }

  // 2. Intra-District & Inter-District Activity Conduits
  let focusDistrictName = s.selectedDistrict ?? null;
  if (!focusDistrictName) {
    const dirtyOrActive = sortedDistricts.find(
      (d) =>
        (d.dirtyFileCount ?? 0) > 0 ||
        d.buildings.some((b) => b.isTouched || activePaths.has(b.path) || (s.buildings[b.path]?.errors ?? 0) > 0),
    );
    if (dirtyOrActive) focusDistrictName = dirtyOrActive.name;
  }

  const blastReport = focusDistrictName ? computeBlastRadius(focusDistrictName, layout.districts) : null;

  for (const d of sortedDistricts) {
    const elev = elevFn(d);
    const cRaw = worldPxFn(layout, (d.x0 + d.x1) / 2, (d.y0 + d.y1) / 2);
    const cPt: [number, number] = [cRaw.px, cRaw.py - elev + 4];
    const errCount = d.buildings.reduce((n, b) => n + (s.buildings[b.path]?.errors ?? 0), 0);

    // Active / failing worksite lateral conduits inside the district
    for (const b of d.buildings) {
      const bErr = (s.buildings[b.path]?.errors ?? 0) > 0;
      const bAct = activePaths.has(b.path) || Boolean(b.isTouched);
      if (!bErr && !bAct) continue;
      const bRaw = worldPxFn(layout, b.x, b.y);
      const bPt: [number, number] = [bRaw.px, bRaw.py - elev + 2];
      segments.push({
        id: `lateral:${b.path}`,
        kind: bErr ? 'error_fault' : 'work_in_progress',
        waypoints: computeIsoConduitWaypoints(cPt, bPt),
        label: b.path.split('/').pop() ?? b.path,
      });
    }

    // Inter-district dependency viaducts
    for (const depName of d.deps ?? []) {
      const target = byName.get(depName);
      if (!target) continue;
      const tRaw = worldPxFn(layout, (target.x0 + target.x1) / 2, (target.y0 + target.y1) / 2);
      const tPt: [number, number] = [tRaw.px, tRaw.py - elevFn(target) + 4];
      const targetErr = target.buildings.reduce((n, b) => n + (s.buildings[b.path]?.errors ?? 0), 0);

      let kind: ConduitChannelKind = 'supply';
      if (errCount > 0 || targetErr > 0) {
        kind = 'error_fault';
      } else if (
        d.buildings.some((b) => activePaths.has(b.path) || b.isTouched) ||
        target.buildings.some((b) => activePaths.has(b.path) || b.isTouched)
      ) {
        kind = 'work_in_progress';
      } else if ((d.layer ?? 1) === 0) {
        kind = 'verified_ci';
      }

      segments.push({
        id: `dep:${d.name}->${target.name}`,
        kind,
        waypoints: computeIsoConduitWaypoints(tPt, cPt),
      });
    }
  }

  // 3. Explicit Focus Blast-Radius Provider & Impacted Conduits
  if (focusDistrictName && blastReport) {
    const originD = byName.get(focusDistrictName);
    if (originD) {
      const oRaw = worldPxFn(layout, (originD.x0 + originD.x1) / 2, (originD.y0 + originD.y1) / 2);
      const oPt: [number, number] = [oRaw.px, oRaw.py - elevFn(originD) + 4];
      const originHasErr = originD.buildings.some((b) => (s.buildings[b.path]?.errors ?? 0) > 0);

      for (const pName of blastReport.providers) {
        const pd = byName.get(pName);
        if (!pd) continue;
        const pRaw = worldPxFn(layout, (pd.x0 + pd.x1) / 2, (pd.y0 + pd.y1) / 2);
        const pPt: [number, number] = [pRaw.px, pRaw.py - elevFn(pd) + 4];
        segments.push({
          id: `blast-provider:${pName}->${focusDistrictName}`,
          kind: 'supply',
          waypoints: computeIsoConduitWaypoints(pPt, oPt),
        });
      }

      for (const iName of blastReport.impactedDependents) {
        const id = byName.get(iName);
        if (!id) continue;
        const iRaw = worldPxFn(layout, (id.x0 + id.x1) / 2, (id.y0 + id.y1) / 2);
        const iPt: [number, number] = [iRaw.px, iRaw.py - elevFn(id) + 4];
        segments.push({
          id: `blast-impacted:${focusDistrictName}->${iName}`,
          kind: originHasErr ? 'error_fault' : 'work_in_progress',
          waypoints: computeIsoConduitWaypoints(oPt, iPt),
        });
      }
    }
  }

  // 4. Verified CI & Porta Git Triumphal Conduit (`VIA VICTORIAE`)
  const castrumPt = worldPxFn(layout, layout.landmarks.castrum.x, layout.landmarks.castrum.y);
  const portaPt = worldPxFn(layout, layout.landmarks.gate.x, layout.landmarks.gate.y);
  segments.push({
    id: 'victoria:castrum->porta',
    kind: 'verified_ci',
    waypoints: computeIsoConduitWaypoints(
      [castrumPt.px - 16, castrumPt.py],
      [portaPt.px + 18, portaPt.py],
    ),
    label: 'CASTRVM CI → PORTA GIT',
  });

  return segments;
}

/**
 * Phase 293 (`URBS-CONDUIT-ANIM-01..04`):
 * Animates the 4 Color + Geometric Shape particle streams along all active hydraulic & sewer conduits.
 */
export function drawAnimatedHydraulicActivityFlows(
  ctx: CanvasRenderingContext2D,
  s: WorldState,
  timeMs: number,
  worldPxFn: WorldPxFn,
  elevFn: ElevFn,
): void {
  const errN = Object.values(s.buildings).reduce((n, b) => n + (b.errors ?? 0), 0);
  const runners = s.harness?.ci?.runners?.length ?? 0;
  const mcpList = s.harness?.mcp ?? [];
  const flowScale = computeConduitActivityLoadScale({
    errorCount: errN,
    runnerCount: runners,
    mcpOk: mcpList.filter((m) => m.ok).length,
    mcpTotal: Math.max(1, mcpList.length),
    activeTaskCount: Object.keys(s.agentTasks).length,
  });
  const cadence = Math.max(0, Math.round(1 * flowScale) - 1);

  const conduits = buildActiveHydraulicConduits(s, worldPxFn, elevFn);
  ctx.save();

  conduits.forEach((seg, idx) => {
    const pulseCount = (seg.kind === 'error_fault' || seg.kind === 'work_in_progress' ? 2 : 1) + cadence;
    const speed =
      seg.kind === 'work_in_progress'
        ? 0.00055
        : seg.kind === 'error_fault'
          ? 0.00048
          : 0.00036;

    for (let p = 0; p < pulseCount; p++) {
      const t = ((timeMs * speed + idx * 0.17 + p / pulseCount) % 1 + 1) % 1;
      const sample = samplePolylineAt(seg.waypoints, t);
      if (seg.kind === 'supply') {
        drawChevronWaveParticle(ctx, sample.x, sample.y, sample.angle, 0.45);
      } else if (seg.kind === 'work_in_progress') {
        drawDiamondIngotParticle(ctx, sample.x, sample.y, sample.angle, 0.45);
      } else if (seg.kind === 'error_fault') {
        drawWarningTriangleParticle(ctx, sample.x, sample.y, sample.angle, 0.45);
      } else {
        drawLaurelSealParticle(ctx, sample.x, sample.y, 0.45);
      }
    }
  });

  ctx.restore();
}

/**
 * Phases 294 & 299 (`URBS-SEMIOTICS-CV-01..02`, `URBS-FLOW-SEMIOTICS-03..04`):
 * Renders glowing Perimeter Bronze Manifold Port Rings (`Epistomia`) at district street gates
 * (while keeping 3D building rooftops 100% free of un-occluded crossing lines) plus the
 * Vector-Shape Roman Hydraulic & Sewer Semiotic Legend Pill with live channel counts.
 */
export function drawScreenSpaceHydraulicOverlay(
  ctx: CanvasRenderingContext2D,
  s: WorldState,
  toScreen: (wx: number, wy: number) => [number, number],
  worldPxFn: WorldPxFn,
  elevFn: ElevFn,
  focusDistrictName: string | null,
  hideLegendPill = false,
): void {
  const { layout } = s;
  ctx.save();

  if (focusDistrictName) {
    const blastReport = computeBlastRadius(focusDistrictName, layout.districts);
    const originD = layout.districts.find((d) => d.name === focusDistrictName);
    if (originD) {
      const oElev = elevFn(originD);
      const oPort = worldPxFn(layout, originD.x0 + 0.35, (originD.y0 + originD.y1) / 2);
      const oScreen = toScreen(oPort.px, oPort.py - oElev + 2);
      const originHasErr = originD.buildings.some((b) => (s.buildings[b.path]?.errors ?? 0) > 0);
      drawBronzeValveJunction(ctx, oScreen[0], oScreen[1], originHasErr ? 'error_fault' : 'work_in_progress', 5.2);

      for (const pName of blastReport.providers) {
        const pd = layout.districts.find((d) => d.name === pName);
        if (!pd) continue;
        const pPort = worldPxFn(layout, pd.x1 - 0.35, (pd.y0 + pd.y1) / 2);
        const pScreen = toScreen(pPort.px, pPort.py - elevFn(pd) + 2);
        const angle = Math.atan2(oScreen[1] - pScreen[1], oScreen[0] - pScreen[0]);
        drawBronzeValveJunction(ctx, pScreen[0], pScreen[1], 'supply', 4.6);
        drawChevronWaveParticle(ctx, pScreen[0], pScreen[1], angle, 0.85);
      }

      for (const iName of blastReport.impactedDependents) {
        const id = layout.districts.find((d) => d.name === iName);
        if (!id) continue;
        const iPort = worldPxFn(layout, id.x0 + 0.35, (id.y0 + id.y1) / 2);
        const iScreen = toScreen(iPort.px, iPort.py - elevFn(id) + 2);
        const angle = Math.atan2(iScreen[1] - oScreen[1], iScreen[0] - oScreen[0]);
        const kind: ConduitChannelKind = originHasErr ? 'error_fault' : 'work_in_progress';
        drawBronzeValveJunction(ctx, iScreen[0], iScreen[1], kind, 4.6);
        if (kind === 'error_fault') {
          drawWarningTriangleParticle(ctx, iScreen[0], iScreen[1], angle, 0.85);
        } else {
          drawDiamondIngotParticle(ctx, iScreen[0], iScreen[1], angle, 0.85);
        }
      }
    }
  }

  // Count live conduits per channel for the legend pill (`URBS-FLOW-SEMIOTICS-04`)
  const activeConduits = buildActiveHydraulicConduits(s, worldPxFn, elevFn);
  const supplyCount = activeConduits.filter((c) => c.kind === 'supply').length;
  const wipCount = activeConduits.filter((c) => c.kind === 'work_in_progress').length;
  const faultCount = activeConduits.filter((c) => c.kind === 'error_fault').length;
  const ciCount = activeConduits.filter((c) => c.kind === 'verified_ci').length;

  if (hideLegendPill) {
    ctx.restore();
    return;
  }

  // Compact Chiseled Roman Hydraulic & Sewer Semiotic Legend Pill (top-left of map canvas below lens bar)
  const lx = 14;
  const ly = 82;
  const lw = 386;
  const lh = 22;

  ctx.fillStyle = 'rgba(12, 10, 9, 0.88)';
  ctx.strokeStyle = 'rgba(217, 119, 6, 0.72)';
  ctx.lineWidth = 1.1;
  ctx.beginPath();
  ctx.roundRect(lx, ly, lw, lh, 4);
  ctx.fill();
  ctx.stroke();

  const cy = ly + lh / 2;
  drawChevronWaveParticle(ctx, lx + 12, cy, 0, 0.72);
  drawDiamondIngotParticle(ctx, lx + 106, cy, 0, 0.72);
  drawWarningTriangleParticle(ctx, lx + 208, cy, 0, 0.72);
  drawLaurelSealParticle(ctx, lx + 306, cy, 0.72);

  const items: { sym: string; label: string; color: string; xOffset: number }[] = [
    { sym: '❯', label: `AQUA Supply (${supplyCount})`, color: '#38bdf8', xOffset: 20 },
    { sym: '◆', label: `IGNIS WIP (${wipCount})`, color: '#fbbf24', xOffset: 115 },
    { sym: '▲', label: `CLOACA Fault (${faultCount})`, color: '#f87171', xOffset: 217 },
    { sym: '◉', label: `VICTORIA CI (${ciCount})`, color: '#34d399', xOffset: 314 },
  ];

  ctx.font = "600 8px 'Palatino', 'Palatino Linotype', Georgia, serif";
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  for (const item of items) {
    ctx.fillStyle = item.color;
    ctx.fillText(`${item.sym} ${item.label}`, lx + item.xOffset, cy + 0.5);
  }

  ctx.restore();
}
