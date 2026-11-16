/**
 * Modular, Reusable Photorealistic Diegetic FX Engine (`AXIS-FX-01..04`).
 *
 * Provides composable 2D Canvas atmospheric, lighting, hydraulic, and perspective-integrated
 * procession passes shared across `LudusSandbox` (`Urbs`), `Dashboard`, `HarnessRiverPanel`,
 * `AgentFlow`, and `Oratio`.
 *
 * Design invariants:
 * - Perspective foreshortening (`depthScaleFromUvV`) + aerial-perspective haze integration so
 *   marching legionaries, citizens, and telemetry conduits feel physically part of painted stone
 *   scenes rather than pasted-on flat vector overlays.
 * - Soft oblique contact shadows (`+28deg`) aligned with the sunlit Carrara directional light.
 * - Graceful fallback when `createLinearGradient` / `createRadialGradient` are absent in unit-test
 *   canvas mocks (`diegeticEnvironment.test.ts`).
 */

export interface FxBoundsRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface FxWaterChannelSpec {
  uvPoints: readonly (readonly [number, number])[];
  widthPx: number;
  flowSpeed: number;
  tint?: 'turquoise' | 'gold' | 'emerald';
}

export interface FxMistPoolSpec {
  uv: readonly [number, number];
  radiusPx: number;
  plumeHeightPx?: number;
}

export interface FxProcessionRouteSpec {
  uvPoints: readonly (readonly [number, number])[];
  count: number;
  caste: 'legionary' | 'senator' | 'scholar' | 'artisan';
  speed?: number;
}

function clamp01(v: number): number {
  return Math.max(0, Math.min(1, v));
}

export function depthScaleFromUvV(v: number): number {
  const clamped = clamp01(v);
  return 0.42 + 0.84 * Math.pow(clamped, 1.32);
}

function hexToRgb(hex: string): [number, number, number] {
  const clean = hex.replace('#', '');
  const full =
    clean.length === 3
      ? clean
          .split('')
          .map((c) => c + c)
          .join('')
      : clean;
  const n = parseInt(full, 16);
  if (Number.isNaN(n)) return [212, 175, 55];
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function mixRgbWithAerialHaze(
  rgb: readonly [number, number, number],
  hazeAmount: number,
): string {
  // Sunlit warm Carrara limestone atmospheric scatter `[238, 229, 212]`
  const h = clamp01(hazeAmount);
  const r = Math.round(rgb[0] * (1 - h) + 238 * h);
  const g = Math.round(rgb[1] * (1 - h) + 229 * h);
  const b = Math.round(rgb[2] * (1 - h) + 212 * h);
  return `rgb(${r}, ${g}, ${b})`;
}

function sampleUvPath(
  bounds: FxBoundsRect,
  uvPoints: readonly (readonly [number, number])[],
  tRaw: number,
): { x: number; y: number; u: number; v: number; angle: number } {
  const t = clamp01(tRaw);
  if (uvPoints.length === 0) {
    return { x: bounds.x, y: bounds.y, u: 0, v: 0, angle: 0 };
  }
  if (uvPoints.length === 1) {
    const p = uvPoints[0]!;
    return {
      x: bounds.x + p[0] * bounds.width,
      y: bounds.y + p[1] * bounds.height,
      u: p[0],
      v: p[1],
      angle: 0,
    };
  }
  const pts = uvPoints.map(([u, v]) => ({
    x: bounds.x + u * bounds.width,
    y: bounds.y + v * bounds.height,
    u,
    v,
  }));
  let totalLen = 0;
  const segLens: number[] = [];
  for (let i = 0; i < pts.length - 1; i++) {
    const dx = pts[i + 1]!.x - pts[i]!.x;
    const dy = pts[i + 1]!.y - pts[i]!.y;
    const len = Math.hypot(dx, dy);
    segLens.push(len);
    totalLen += len;
  }
  if (totalLen <= 0.001) {
    return { x: pts[0]!.x, y: pts[0]!.y, u: pts[0]!.u, v: pts[0]!.v, angle: 0 };
  }
  let target = t * totalLen;
  for (let i = 0; i < segLens.length; i++) {
    const len = segLens[i]!;
    if (target <= len || i === segLens.length - 1) {
      const localT = len > 0.0001 ? clamp01(target / len) : 0;
      const a = pts[i]!;
      const b = pts[i + 1]!;
      return {
        x: a.x + (b.x - a.x) * localT,
        y: a.y + (b.y - a.y) * localT,
        u: a.u + (b.u - a.u) * localT,
        v: a.v + (b.v - a.v) * localT,
        angle: Math.atan2(b.y - a.y, b.x - a.x),
      };
    }
    target -= len;
  }
  const last = pts[pts.length - 1]!;
  return { x: last.x, y: last.y, u: last.u, v: last.v, angle: 0 };
}

/**
 * 1. Sunlit Golden Crepuscular God-Rays + Drifting Sunlit Dust Motes (`screen` blend).
 */
export function renderSunlitGodRays(
  ctx: CanvasRenderingContext2D,
  bounds: FxBoundsRect,
  timeMs: number,
  intensity = 1,
): void {
  if (bounds.width < 16 || bounds.height < 16 || intensity <= 0.01) return;
  ctx.save();
  if ('globalCompositeOperation' in ctx) {
    ctx.globalCompositeOperation = 'screen';
  }
  const t = timeMs * 0.001;
  const hasLinearGrad = typeof ctx.createLinearGradient === 'function';

  const shafts = [
    { u0: 0.08, spread: 0.13, speed: 0.38, phase: 0.0, alpha: 0.16 },
    { u0: 0.29, spread: 0.16, speed: 0.31, phase: 1.7, alpha: 0.14 },
    { u0: 0.54, spread: 0.14, speed: 0.42, phase: 3.1, alpha: 0.13 },
    { u0: 0.77, spread: 0.11, speed: 0.27, phase: 4.4, alpha: 0.11 },
  ];

  for (const s of shafts) {
    const sway = Math.sin(t * s.speed + s.phase) * 0.035;
    const pulse = 0.78 + 0.22 * Math.sin(t * (s.speed * 1.7) + s.phase);
    const topX = bounds.x + (s.u0 + sway) * bounds.width;
    const topW = s.spread * bounds.width;
    const botShift = bounds.height * 0.46;
    const botW = topW * 1.65;
    const a = clamp01(s.alpha * pulse * intensity);

    if (hasLinearGrad && typeof ctx.closePath === 'function') {
      const grad = ctx.createLinearGradient(topX, bounds.y, topX + botShift, bounds.y + bounds.height);
      grad.addColorStop(0, `rgba(255, 245, 206, ${a.toFixed(3)})`);
      grad.addColorStop(0.55, `rgba(244, 212, 124, ${(a * 0.58).toFixed(3)})`);
      grad.addColorStop(1, 'rgba(244, 212, 124, 0)');
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.moveTo(topX - topW * 0.5, bounds.y);
      ctx.lineTo(topX + topW * 0.5, bounds.y);
      ctx.lineTo(topX + botShift + botW * 0.5, bounds.y + bounds.height);
      ctx.lineTo(topX + botShift - botW * 0.5, bounds.y + bounds.height);
      ctx.closePath();
      ctx.fill();
    }
  }

  // Sunlit airborne dust motes drifting slowly through the golden shafts
  const moteCount = Math.min(28, Math.max(10, Math.round(bounds.width / 52)));
  for (let i = 0; i < moteCount; i++) {
    const seed = i * 1.6180339;
    const u = ((seed * 0.37 + t * 0.014 * (1 + (i % 3) * 0.25)) % 1 + 1) % 1;
    const v = ((seed * 0.71 - t * 0.011 * (0.8 + (i % 4) * 0.2)) % 1 + 1) % 1;
    const twinkle = 0.35 + 0.65 * Math.pow(0.5 + 0.5 * Math.sin(t * 2.4 + i * 1.9), 2);
    const r = (1.1 + (i % 3) * 0.65) * Math.max(0.75, Math.min(1.3, bounds.width / 960));
    ctx.fillStyle = `rgba(255, 244, 196, ${(0.42 * twinkle * intensity).toFixed(3)})`;
    ctx.beginPath();
    ctx.arc(bounds.x + u * bounds.width, bounds.y + v * bounds.height, r, 0, Math.PI * 2);
    ctx.fill();
  }

  ctx.restore();
}

/**
 * 2. Multi-Octave Water Caustics + Rising Waterfall Mist Plumes.
 */
export function renderPhotorealWaterCausticsAndMist(
  ctx: CanvasRenderingContext2D,
  bounds: FxBoundsRect,
  channels: readonly FxWaterChannelSpec[],
  pools: readonly FxMistPoolSpec[],
  timeMs: number,
  intensity = 1,
): void {
  if (bounds.width < 16 || bounds.height < 16 || intensity <= 0.01) return;
  ctx.save();
  const t = timeMs * 0.001;
  const scale = Math.max(0.65, Math.min(1.35, bounds.width / 1100));

  // Multi-octave specular caustics travelling along perspective water channels
  for (let cIdx = 0; cIdx < channels.length; cIdx++) {
    const ch = channels[cIdx]!;
    if (ch.uvPoints.length < 2) continue;
    const pts = ch.uvPoints.map(([u, v]) => ({
      x: bounds.x + u * bounds.width,
      y: bounds.y + v * bounds.height,
      v,
    }));
    const avgV = pts.reduce((acc, p) => acc + p.v, 0) / pts.length;
    const depthW = ch.widthPx * scale * depthScaleFromUvV(avgV);

    // Base refracted turquoise/gold glaze integrated into waterbed
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.strokeStyle =
      ch.tint === 'gold'
        ? `rgba(212, 175, 55, ${(0.24 * intensity).toFixed(3)})`
        : ch.tint === 'emerald'
          ? `rgba(27, 107, 58, ${(0.24 * intensity).toFixed(3)})`
          : `rgba(56, 189, 216, ${(0.22 * intensity).toFixed(3)})`;
    ctx.lineWidth = Math.max(1.4, depthW);
    ctx.beginPath();
    ctx.moveTo(pts[0]!.x, pts[0]!.y);
    for (let i = 1; i < pts.length; i++) {
      const prev = pts[i - 1]!;
      const cur = pts[i]!;
      ctx.quadraticCurveTo(prev.x, prev.y, (prev.x + cur.x) * 0.5, (prev.y + cur.y) * 0.5);
    }
    ctx.lineTo(pts[pts.length - 1]!.x, pts[pts.length - 1]!.y);
    ctx.stroke();

    // Octave 1 & 2 organic sunlit water shimmer wavelets (continuous specular glints — zero dashed lines)
    const waveletCount = Math.max(5, Math.min(12, Math.round(pts.length * 2.4)));
    for (let w = 0; w < waveletCount; w++) {
      const progress = ((t * 0.22 * ch.flowSpeed + w / waveletCount + cIdx * 0.17) % 1 + 1) % 1;
      const sample = sampleUvPath(bounds, ch.uvPoints, progress);
      const localScale = scale * depthScaleFromUvV(sample.v);
      const env = Math.sin(progress * Math.PI);
      const rx = Math.max(2.6, depthW * 0.62 * (0.75 + 0.35 * env));
      const ry = Math.max(0.75, depthW * 0.14);
      const sway = Math.sin(t * 2.8 + w * 1.7 + cIdx) * depthW * 0.16;
      const wx = sample.x - Math.sin(sample.angle) * sway;
      const wy = sample.y + Math.cos(sample.angle) * sway;
      const glintAlpha = clamp01((0.28 + 0.32 * env) * intensity);

      ctx.fillStyle =
        w % 2 === 0
          ? `rgba(255, 251, 232, ${glintAlpha.toFixed(3)})`
          : `rgba(253, 226, 132, ${(glintAlpha * 0.82).toFixed(3)})`;
      ctx.beginPath();
      ctx.ellipse(wx, wy, rx, ry, sample.angle, 0, Math.PI * 2);
      ctx.fill();

      if (w % 3 === 0) {
        ctx.fillStyle = `rgba(255, 255, 245, ${(glintAlpha * 0.9).toFixed(3)})`;
        ctx.beginPath();
        ctx.arc(wx, wy, Math.max(0.7, 1.1 * localScale), 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  // Rising buoyant waterfall mist plumes + concentric surface ripples
  const hasRadial = typeof ctx.createRadialGradient === 'function';
  for (let pIdx = 0; pIdx < pools.length; pIdx++) {
    const pool = pools[pIdx]!;
    const cx = bounds.x + pool.uv[0] * bounds.width;
    const cy = bounds.y + pool.uv[1] * bounds.height;
    const baseR = pool.radiusPx * scale;
    const plumeH = (pool.plumeHeightPx ?? pool.radiusPx * 1.15) * scale;

    // 2 expanding concentric water-impact rings
    for (let ring = 0; ring < 2; ring++) {
      const rp = (t * 0.68 + pIdx * 0.37 + ring * 0.5) % 1;
      const rx = baseR * (0.45 + rp * 1.25);
      const ry = rx * 0.42;
      ctx.strokeStyle = `rgba(240, 253, 255, ${((1 - rp) * 0.52 * intensity).toFixed(3)})`;
      ctx.lineWidth = Math.max(0.9, 1.3 * scale);
      ctx.beginPath();
      ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
      ctx.stroke();
    }

    // 4 buoyant rising mist puffs per cascade pool
    for (let puff = 0; puff < 4; puff++) {
      const life = (t * 0.44 + puff * 0.25 + pIdx * 0.19) % 1;
      const driftX = Math.sin(t * 1.3 + puff * 2.1 + pIdx) * baseR * 0.28;
      const py = cy - life * plumeH;
      const pr = baseR * (0.48 + life * 0.65);
      const alpha = Math.sin(life * Math.PI) * 0.34 * intensity;
      if (hasRadial) {
        const g = ctx.createRadialGradient(cx + driftX, py, 0, cx + driftX, py, pr);
        g.addColorStop(0, `rgba(255, 253, 246, ${alpha.toFixed(3)})`);
        g.addColorStop(0.55, `rgba(224, 244, 250, ${(alpha * 0.52).toFixed(3)})`);
        g.addColorStop(1, 'rgba(224, 244, 250, 0)');
        ctx.fillStyle = g;
      } else {
        ctx.fillStyle = `rgba(255, 253, 246, ${(alpha * 0.5).toFixed(3)})`;
      }
      ctx.beginPath();
      ctx.arc(cx + driftX, py, pr, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  ctx.restore();
}

/**
 * 3. Warm Flickering Torch & Temple Brazier Light Pools (`screen` blend).
 */
export function renderDiegeticTorchAndBrazierPools(
  ctx: CanvasRenderingContext2D,
  bounds: FxBoundsRect,
  anchorsUv: readonly (readonly [number, number])[],
  timeMs: number,
  activeBoost = 1,
): void {
  if (bounds.width < 16 || bounds.height < 16 || anchorsUv.length === 0) return;
  ctx.save();
  if ('globalCompositeOperation' in ctx) {
    ctx.globalCompositeOperation = 'screen';
  }
  const t = timeMs * 0.001;
  const scale = Math.max(0.65, Math.min(1.35, bounds.width / 1100));
  const hasRadial = typeof ctx.createRadialGradient === 'function';

  for (let i = 0; i < anchorsUv.length; i++) {
    const [u, v] = anchorsUv[i]!;
    const dScale = depthScaleFromUvV(v);
    const x = bounds.x + u * bounds.width;
    const y = bounds.y + v * bounds.height;
    const flicker =
      0.82 +
      0.11 * Math.sin(t * 7.3 + i * 2.1) +
      0.07 * Math.cos(t * 13.7 + i * 3.4);
    const radius = 26 * scale * dScale * flicker * Math.min(1.4, activeBoost);
    const coreAlpha = clamp01(0.32 * flicker * Math.min(1.35, activeBoost));

    if (hasRadial) {
      const grad = ctx.createRadialGradient(x, y, 0, x, y, radius);
      grad.addColorStop(0, `rgba(255, 232, 158, ${coreAlpha.toFixed(3)})`);
      grad.addColorStop(0.45, `rgba(245, 158, 11, ${(coreAlpha * 0.48).toFixed(3)})`);
      grad.addColorStop(1, 'rgba(245, 158, 11, 0)');
      ctx.fillStyle = grad;
    } else {
      ctx.fillStyle = `rgba(255, 214, 120, ${(coreAlpha * 0.4).toFixed(3)})`;
    }
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, Math.PI * 2);
    ctx.fill();

    // Rising ember spark
    const emberLife = (t * 0.85 + i * 0.31) % 1;
    const ex = x + Math.sin(t * 3.2 + i) * 3.5 * dScale;
    const ey = y - emberLife * 14 * scale * dScale;
    ctx.fillStyle = `rgba(254, 240, 138, ${((1 - emberLife) * 0.65).toFixed(3)})`;
    ctx.beginPath();
    ctx.arc(ex, ey, Math.max(0.8, 1.3 * scale * dScale), 0, Math.PI * 2);
    ctx.fill();
  }

  ctx.restore();
}

/**
 * 4. Perspective-Integrated Roman Legionary & Citizen Procession.
 *
 * Scales silhouettes smoothly by `depthScaleFromUvV(v)`, projects soft oblique ground shadows
 * (`+28deg`) onto cobblestones/bridges, blends distant figures with warm limestone aerial haze,
 * and adds sunlit gold rim highlights so figures belong physically to the painted scene.
 */
export function renderPerspectiveIntegratedProcession(
  ctx: CanvasRenderingContext2D,
  bounds: FxBoundsRect,
  routes: readonly FxProcessionRouteSpec[],
  timeMs: number,
  activeBoost = 1,
): void {
  if (bounds.width < 16 || bounds.height < 16) return;
  ctx.save();
  const t = timeMs * 0.001;
  const resScale = Math.max(0.62, Math.min(1.28, bounds.width / 1180));
  const speedBoost = Math.max(0.85, Math.min(1.85, activeBoost));

  // Collect all figures across routes and sort back-to-front by Y (painter's depth order)
  const drawQueue: {
    x: number;
    y: number;
    v: number;
    dirX: number;
    caste: FxProcessionRouteSpec['caste'];
    stridePhase: number;
    isLeader: boolean;
  }[] = [];

  for (let rIdx = 0; rIdx < routes.length; rIdx++) {
    const route = routes[rIdx]!;
    const baseSpeed = route.speed ?? 0.052;
    for (let i = 0; i < route.count; i++) {
      const forward = i % 2 === 0 || route.caste === 'legionary';
      const rawT = (t * baseSpeed * speedBoost + i / Math.max(1, route.count) + rIdx * 0.17) % 1;
      const sample = sampleUvPath(bounds, route.uvPoints, forward ? rawT : 1 - rawT);
      const dScale = depthScaleFromUvV(sample.v) * resScale;
      const laneOffset = ((i % 2) - 0.5) * 3.4 * dScale;
      const px = sample.x - Math.sin(sample.angle) * laneOffset;
      const py = sample.y + Math.cos(sample.angle) * laneOffset;
      const dirX = (forward ? 1 : -1) * (Math.cos(sample.angle) >= 0 ? 1 : -1);

      drawQueue.push({
        x: px,
        y: py,
        v: sample.v,
        dirX,
        caste: route.caste,
        stridePhase: t * 9.2 * speedBoost + i * 2.1 + rIdx,
        isLeader: i === 0,
      });
    }
  }

  drawQueue.sort((a, b) => a.y - b.y);

  for (const f of drawQueue) {
    const s = depthScaleFromUvV(f.v) * resScale;
    // Aerial perspective haze increases toward the top Acropolis horizon (`v -> 0.25`)
    const haze = clamp01((0.76 - f.v) * 0.48);
    const bobY = Math.abs(Math.sin(f.stridePhase)) * 1.15 * s;
    const stride = Math.sin(f.stridePhase) * 1.45 * s;

    // 4a. Soft oblique directional ground contact shadow (+28deg matching sunlit colonnades)
    ctx.fillStyle = `rgba(38, 27, 16, ${(0.34 * (1 - haze * 0.45)).toFixed(3)})`;
    ctx.beginPath();
    ctx.ellipse(f.x + 2.2 * s, f.y + 0.8 * s, 3.8 * s, 1.35 * s, 0.32, 0, Math.PI * 2);
    ctx.fill();

    // 4b. Grounded walking legs in warm shadow tone
    ctx.strokeStyle = mixRgbWithAerialHaze([78, 54, 34], haze);
    ctx.lineWidth = Math.max(0.8, 1.05 * s);
    ctx.lineCap = 'round';
    ctx.beginPath();
    ctx.moveTo(f.x - 0.7 * s, f.y - 2.4 * s - bobY * 0.4);
    ctx.lineTo(f.x - stride, f.y + 0.6 * s);
    ctx.moveTo(f.x + 0.7 * s, f.y - 2.4 * s - bobY * 0.4);
    ctx.lineTo(f.x + stride, f.y + 0.6 * s);
    ctx.stroke();

    // 4c. Physically-tinted Roman drapery / lorica / cloak
    const baseRgb: [number, number, number] =
      f.caste === 'legionary'
        ? [136, 36, 32] // Sun-weathered Imperial madder crimson cloak (not neon #b91c1c!)
        : f.caste === 'senator'
          ? [242, 235, 220] // Warm travertine toga praetexta
          : f.caste === 'scholar'
            ? [178, 134, 42] // Ochre/gold collegium mantle
            : [138, 96, 54]; // Earth sienna tunic
    ctx.fillStyle = mixRgbWithAerialHaze(baseRgb, haze);
    ctx.beginPath();
    ctx.ellipse(f.x, f.y - 4.4 * s - bobY, 2.15 * s, 3.15 * s, 0, 0, Math.PI * 2);
    ctx.fill();

    // Sunlit top-left rim highlight along shoulder & cloak fold
    ctx.strokeStyle = `rgba(255, 243, 201, ${(0.62 * (1 - haze * 0.35)).toFixed(3)})`;
    ctx.lineWidth = Math.max(0.6, 0.75 * s);
    ctx.beginPath();
    ctx.arc(f.x, f.y - 4.5 * s - bobY, 2.05 * s, Math.PI * 0.95, Math.PI * 1.78);
    ctx.stroke();

    // Legionary curved bronze scutum shield catching sunlight
    if (f.caste === 'legionary') {
      const sx = f.x + f.dirX * 1.75 * s;
      const sy = f.y - 4.1 * s - bobY;
      ctx.fillStyle = mixRgbWithAerialHaze([184, 138, 44], haze * 0.8);
      ctx.beginPath();
      ctx.ellipse(sx, sy, 1.25 * s, 2.35 * s, f.dirX * 0.14, 0, Math.PI * 2);
      ctx.fill();
    }

    // 4d. Head / Galea bronze helmet with sunlit specular crest
    const headRgb: [number, number, number] =
      f.caste === 'legionary' ? [206, 166, 68] : [224, 186, 148];
    ctx.fillStyle = mixRgbWithAerialHaze(headRgb, haze);
    ctx.beginPath();
    ctx.arc(f.x, f.y - 8.2 * s - bobY, 1.45 * s, 0, Math.PI * 2);
    ctx.fill();

    // 4e. Cohort Signifer / Aquilifer standard with fluttering pennant on route leader
    if (f.isLeader && (f.caste === 'legionary' || f.caste === 'scholar')) {
      const poleX = f.x + f.dirX * 2.1 * s;
      const poleTopY = f.y - 14.2 * s - bobY;
      ctx.strokeStyle = mixRgbWithAerialHaze([212, 175, 55], haze * 0.6);
      ctx.lineWidth = Math.max(0.75, 0.95 * s);
      ctx.beginPath();
      ctx.moveTo(poleX, f.y - 1.2 * s - bobY);
      ctx.lineTo(poleX, poleTopY);
      ctx.stroke();

      // Fluttering cloth vexillum pennant
      const flutter = Math.sin(t * 5.6 + f.x * 0.05) * 1.4 * s;
      ctx.fillStyle = mixRgbWithAerialHaze(
        f.caste === 'legionary' ? [27, 107, 58] : [197, 155, 39],
        haze * 0.6,
      );
      ctx.beginPath();
      ctx.moveTo(poleX, poleTopY + 1.0 * s);
      ctx.quadraticCurveTo(
        poleX - f.dirX * 2.6 * s,
        poleTopY + 1.8 * s + flutter * 0.5,
        poleX - f.dirX * 4.8 * s,
        poleTopY + 2.5 * s + flutter,
      );
      ctx.lineTo(poleX, poleTopY + 3.8 * s);
      ctx.fill();
    }
  }

  ctx.restore();
}

/**
 * 5. Laminar Telemetry Pulse Filaments (for Aqueducts, `AgentFlow` DAG conduits, and `Oratio`).
 */
export function renderAtmosphericTelemetryPulse(
  ctx: CanvasRenderingContext2D,
  bounds: FxBoundsRect,
  conduitsUv: readonly (readonly (readonly [number, number])[])[],
  timeMs: number,
  accentHex = '#D4AF37',
): void {
  if (bounds.width < 16 || bounds.height < 16 || conduitsUv.length === 0) return;
  ctx.save();
  if ('globalCompositeOperation' in ctx) {
    ctx.globalCompositeOperation = 'screen';
  }
  const t = timeMs * 0.001;
  const [r, g, b] = hexToRgb(accentHex);
  const scale = Math.max(0.7, Math.min(1.35, bounds.width / 1000));
  const hasRadial = typeof ctx.createRadialGradient === 'function';

  for (let i = 0; i < conduitsUv.length; i++) {
    const ptsUv = conduitsUv[i]!;
    if (ptsUv.length < 2) continue;
    for (let bead = 0; bead < 2; bead++) {
      const progress = (t * 0.36 + i * 0.21 + bead * 0.5) % 1;
      const sample = sampleUvPath(bounds, ptsUv, progress);
      const glowR = 11 * scale;
      if (hasRadial) {
        const grad = ctx.createRadialGradient(sample.x, sample.y, 0, sample.x, sample.y, glowR);
        grad.addColorStop(0, `rgba(255, 250, 224, 0.85)`);
        grad.addColorStop(0.4, `rgba(${r}, ${g}, ${b}, 0.48)`);
        grad.addColorStop(1, `rgba(${r}, ${g}, ${b}, 0)`);
        ctx.fillStyle = grad;
      } else {
        ctx.fillStyle = `rgba(${r}, ${g}, ${b}, 0.55)`;
      }
      ctx.beginPath();
      ctx.arc(sample.x, sample.y, glowR, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  ctx.restore();
}

export interface FxMonumentFootprintSpec {
  id: string;
  /** Normalized `[u, v]` center of the painted building mass. */
  footprintUv: readonly [number, number];
  /** Normalized `[u, v]` plinth/courtyard anchor where player guidance beacon sits. */
  plinthUv: readonly [number, number];
  /** Normalized `[rx, ry]` architectural halo radii relative to panorama `[width, height]`. */
  radiusUv: readonly [number, number];
  /** Live operational state for player guidance cues. */
  activityState: 'ready' | 'active' | 'attention' | 'quarantined';
  isHovered?: boolean;
  isSelected?: boolean;
  /** Optional lens intensity boost (`0..1.5`) when viewing Health / Agents / Cost lenses. */
  lensHighlight?: number;
}

/**
 * 6. Diegetic Interactive Monument Affordance Beacons & Mouseover Architectural Haloing.
 *
 * - Idle interactive monuments emit a gentle, tasteful sunlit sanctuary plinth shimmer & lit brazier
 *   so players intuitively recognize which monumental buildings can be hovered/inspected vs static
 *   background scenery (zero synthetic dotted/dashed circles).
 * - Hovered or selected monuments ignite a warm volumetric Carrara-gold / emerald architectural crown
 *   halo and continuous double-fillet podium ring contoured to the building's true panorama footprint.
 */
export function renderInteractiveMonumentBeaconsAndHalo(
  ctx: CanvasRenderingContext2D,
  bounds: FxBoundsRect,
  monuments: readonly FxMonumentFootprintSpec[],
  timeMs: number,
): void {
  if (bounds.width < 16 || bounds.height < 16 || monuments.length === 0) return;
  ctx.save();
  const t = timeMs * 0.001;
  const scale = Math.max(0.65, Math.min(1.35, bounds.width / 1180));
  const hasRadial = typeof ctx.createRadialGradient === 'function';

  for (let i = 0; i < monuments.length; i++) {
    const m = monuments[i]!;
    const cx = bounds.x + m.footprintUv[0] * bounds.width;
    const cy = bounds.y + m.footprintUv[1] * bounds.height;
    const px = bounds.x + m.plinthUv[0] * bounds.width;
    const py = bounds.y + m.plinthUv[1] * bounds.height;
    const rx = Math.max(18, m.radiusUv[0] * bounds.width);
    const ry = Math.max(12, m.radiusUv[1] * bounds.height);
    const dScale = depthScaleFromUvV(m.plinthUv[1]) * scale;
    const breathe = 0.5 + 0.5 * Math.sin(t * 2.4 + i * 1.15);
    const lensBoost = Math.max(0, Math.min(1.5, m.lensHighlight ?? 0));

    const toneRgb: [number, number, number] =
      m.activityState === 'quarantined' || m.activityState === 'attention'
        ? [217, 119, 6]
        : m.activityState === 'active'
          ? [16, 185, 129]
          : [212, 175, 55];

    // 6a. Subtle ambient interactive affordance aura on plinth (guides player to hoverable monuments)
    const idleAuraAlpha =
      (m.activityState === 'active' ? 0.22 : 0.13) +
      breathe * 0.07 +
      lensBoost * 0.12;
    const auraR = Math.max(rx * 0.82, 22 * dScale);
    if ('globalCompositeOperation' in ctx) {
      ctx.globalCompositeOperation = 'screen';
    }
    if (hasRadial) {
      const g = ctx.createRadialGradient(cx, cy + ry * 0.18, 2, cx, cy + ry * 0.18, auraR);
      g.addColorStop(0, `rgba(255, 248, 214, ${(idleAuraAlpha * 1.15).toFixed(3)})`);
      g.addColorStop(
        0.52,
        `rgba(${toneRgb[0]}, ${toneRgb[1]}, ${toneRgb[2]}, ${(idleAuraAlpha * 0.65).toFixed(3)})`,
      );
      g.addColorStop(1, `rgba(${toneRgb[0]}, ${toneRgb[1]}, ${toneRgb[2]}, 0)`);
      ctx.fillStyle = g;
    } else {
      ctx.fillStyle = `rgba(${toneRgb[0]}, ${toneRgb[1]}, ${toneRgb[2]}, ${(idleAuraAlpha * 0.45).toFixed(3)})`;
    }
    ctx.beginPath();
    ctx.ellipse(cx, cy + ry * 0.18, auraR, auraR * 0.64, 0, 0, Math.PI * 2);
    ctx.fill();

    // Active monument gentle expanding ground ripple at plinth (signals live cohort work)
    if (m.activityState === 'active' || m.activityState === 'attention' || lensBoost > 0.25) {
      if ('globalCompositeOperation' in ctx) {
        ctx.globalCompositeOperation = 'source-over';
      }
      const pulsePhase = (t * 0.55 + i * 0.29) % 1;
      const ringRx = (12 + pulsePhase * 18) * dScale;
      const ringRy = ringRx * 0.44;
      const ringAlpha = (1 - pulsePhase) * (m.activityState === 'active' ? 0.52 : 0.42);
      ctx.strokeStyle = `rgba(${toneRgb[0]}, ${toneRgb[1]}, ${toneRgb[2]}, ${ringAlpha.toFixed(3)})`;
      ctx.lineWidth = Math.max(1.0, 1.35 * scale);
      ctx.beginPath();
      ctx.ellipse(px, py - 4 * dScale, ringRx, ringRy, 0, 0, Math.PI * 2);
      ctx.stroke();
    }

    // 6b. Full Architectural Rim & Podium Halo on Mouseover or Selection (continuous — never dashed!)
    if (m.isHovered || m.isSelected) {
      const prominence = m.isHovered ? 1.0 : 0.82;
      const haloPulse = 1 + 0.035 * Math.sin(t * 4.2);
      const hRx = rx * haloPulse;
      const hRy = ry * haloPulse;

      if ('globalCompositeOperation' in ctx) {
        ctx.globalCompositeOperation = 'screen';
      }
      const outerR = Math.max(hRx, hRy) * 1.42;
      if (hasRadial) {
        const crownGrad = ctx.createRadialGradient(cx, cy, outerR * 0.12, cx, cy, outerR);
        crownGrad.addColorStop(0, `rgba(255, 251, 226, ${(0.52 * prominence).toFixed(3)})`);
        crownGrad.addColorStop(0.48, `rgba(245, 198, 72, ${(0.34 * prominence).toFixed(3)})`);
        crownGrad.addColorStop(
          0.78,
          `rgba(${toneRgb[0]}, ${toneRgb[1]}, ${toneRgb[2]}, ${(0.18 * prominence).toFixed(3)})`,
        );
        crownGrad.addColorStop(1, 'rgba(212, 175, 55, 0)');
        ctx.fillStyle = crownGrad;
      } else {
        ctx.fillStyle = `rgba(255, 244, 196, ${(0.32 * prominence).toFixed(3)})`;
      }
      ctx.beginPath();
      ctx.ellipse(cx, cy, hRx * 1.36, hRy * 1.32, 0, 0, Math.PI * 2);
      ctx.fill();

      if ('globalCompositeOperation' in ctx) {
        ctx.globalCompositeOperation = 'source-over';
      }
      // Smooth outer sunlit gold fillet
      ctx.strokeStyle = m.isHovered
        ? 'rgba(255, 248, 210, 0.94)'
        : 'rgba(234, 196, 82, 0.82)';
      ctx.lineWidth = Math.max(1.6, 2.2 * scale);
      ctx.beginPath();
      ctx.ellipse(cx, cy + hRy * 0.12, hRx * 0.96, hRy * 0.78, 0, 0, Math.PI * 2);
      ctx.stroke();

      // Inner burnished bronze/emerald architectural podium ring
      ctx.strokeStyle = `rgba(${toneRgb[0]}, ${toneRgb[1]}, ${toneRgb[2]}, ${(0.72 * prominence).toFixed(3)})`;
      ctx.lineWidth = Math.max(1.0, 1.3 * scale);
      ctx.beginPath();
      ctx.ellipse(cx, cy + hRy * 0.12, hRx * 1.05, hRy * 0.86, 0, 0, Math.PI * 2);
      ctx.stroke();

      // Orbiting sunlit astrolabe specular bead along the architectural halo
      const orbitAngle = t * 1.35 + i;
      const bx = cx + Math.cos(orbitAngle) * hRx * 0.96;
      const by = cy + hRy * 0.12 + Math.sin(orbitAngle) * hRy * 0.78;
      ctx.fillStyle = 'rgba(255, 253, 235, 0.96)';
      ctx.beginPath();
      ctx.arc(bx, by, Math.max(1.8, 2.6 * scale), 0, Math.PI * 2);
      ctx.fill();
    }
  }

  ctx.restore();
}

/**
 * 7. Seamless Hypocaust Forge Steam Plumes (stitched 1:1 onto MENS Foundry chimneys).
 */
export function renderHypocaustForgeSteam(
  ctx: CanvasRenderingContext2D,
  bounds: FxBoundsRect,
  chimneysUv: readonly (readonly [number, number])[],
  timeMs: number,
  intensity = 1,
): void {
  if (bounds.width < 16 || bounds.height < 16 || chimneysUv.length === 0 || intensity <= 0.01) {
    return;
  }
  ctx.save();
  const t = timeMs * 0.001;
  const scale = Math.max(0.65, Math.min(1.3, bounds.width / 1180));
  const hasRadial = typeof ctx.createRadialGradient === 'function';

  for (let c = 0; c < chimneysUv.length; c++) {
    const [u, v] = chimneysUv[c]!;
    const baseX = bounds.x + u * bounds.width;
    const baseY = bounds.y + v * bounds.height;
    for (let p = 0; p < 5; p++) {
      const life = (t * 0.28 + p * 0.2 + c * 0.37) % 1;
      const driftX = (life * 10 + Math.sin(t * 1.4 + p * 1.9 + c) * 3.5) * scale;
      const py = baseY - life * 34 * scale;
      const pr = (5 + life * 13) * scale;
      const alpha = Math.sin(life * Math.PI) * 0.28 * intensity;
      if (hasRadial) {
        const g = ctx.createRadialGradient(baseX + driftX, py, 0, baseX + driftX, py, pr);
        g.addColorStop(0, `rgba(255, 252, 245, ${alpha.toFixed(3)})`);
        g.addColorStop(0.6, `rgba(242, 236, 224, ${(alpha * 0.5).toFixed(3)})`);
        g.addColorStop(1, 'rgba(242, 236, 224, 0)');
        ctx.fillStyle = g;
      } else {
        ctx.fillStyle = `rgba(255, 252, 245, ${(alpha * 0.45).toFixed(3)})`;
      }
      ctx.beginPath();
      ctx.arc(baseX + driftX, py, pr, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  ctx.restore();
}
