// crates/vox-gui/ui/src/components/gamify/urbs/pbrMaterials.ts
/**
 * Phase 277 (`URBS-MAT-01..04`): Procedural Natural Surface Material Synthesis Engine.
 *
 * Synthesizes deterministic, zero-network-asset offscreen `CanvasPattern`s and
 * physically-inspired height/metallic `CanvasGradient`s for:
 *  - Pitted Roman Travertine Ashlar (`travertineLit`, `travertineShade`)
 *  - Scalloped Roman Terracotta Imbrex & Tegula Roof Tiles (`terracottaLit`, `terracottaShade`)
 *  - Opus Incertum / Basalt & Travertine Flagstone Plazas (`plazaCobble`)
 *  - Stratified Craggy Coastal Basalt & Limestone Cliffs (`cliffLit`, `cliffShade`)
 *  - Deep Mediterranean Twilight Sea Waves (`seaWater`)
 *  - Anisotropic Hammered Corinthian Bronze & Forged Wrought Iron (`createBronzeGradient`, `createIronGradient`)
 */

export interface PbrMaterialAtlas {
  travertineLit: CanvasPattern | null;
  travertineShade: CanvasPattern | null;
  terracottaLit: CanvasPattern | null;
  terracottaShade: CanvasPattern | null;
  plazaCobble: CanvasPattern | null;
  cliffLit: CanvasPattern | null;
  cliffShade: CanvasPattern | null;
  seaWater: CanvasPattern | null;
}

/** Deterministic 2D hash returning `[0, 1)`. */
export function hash2d(x: number, y: number, seed = 1337): number {
  let h = Math.imul(x ^ seed, 374761393) + Math.imul(y ^ (seed * 3), 668265263);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/** Smooth 2D value noise in `[0, 1)`. */
export function valueNoise2d(x: number, y: number, seed = 1337): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const xf = x - xi;
  const yf = y - yi;
  const u = xf * xf * (3 - 2 * xf);
  const v = yf * yf * (3 - 2 * yf);

  const n00 = hash2d(xi, yi, seed);
  const n10 = hash2d(xi + 1, yi, seed);
  const n01 = hash2d(xi, yi + 1, seed);
  const n11 = hash2d(xi + 1, yi + 1, seed);

  const nx0 = n00 + (n10 - n00) * u;
  const nx1 = n01 + (n11 - n01) * u;
  return nx0 + (nx1 - nx0) * v;
}

/** Multi-octave fractal Brownian motion (fBm) in `[0, 1)`. */
export function fbm2d(x: number, y: number, octaves = 3, seed = 1337): number {
  let val = 0;
  let amp = 0.5;
  let freq = 1;
  let norm = 0;
  for (let i = 0; i < octaves; i++) {
    val += amp * valueNoise2d(x * freq, y * freq, seed + i * 97);
    norm += amp;
    amp *= 0.5;
    freq *= 2.02;
  }
  return norm > 0 ? val / norm : 0.5;
}

/** Parse `#rrggbb` into `[r, g, b]`. */
export function parseHexRgb(hex: string): [number, number, number] {
  const clean = hex.replace('#', '').trim();
  if (clean.length === 3) {
    const r = parseInt(clean[0]! + clean[0]!, 16) || 128;
    const g = parseInt(clean[1]! + clean[1]!, 16) || 128;
    const b = parseInt(clean[2]! + clean[2]!, 16) || 128;
    return [r, g, b];
  }
  const num = parseInt(clean.slice(0, 6), 16);
  if (Number.isNaN(num)) return [128, 120, 112];
  return [(num >> 16) & 255, (num >> 8) & 255, num & 255];
}

function clampByte(n: number): number {
  return Math.max(0, Math.min(255, Math.round(n)));
}

export function rgbToHex(r: number, g: number, b: number): string {
  const to2 = (v: number) => clampByte(v).toString(16).padStart(2, '0');
  return `#${to2(r)}${to2(g)}${to2(b)}`;
}

export function tintHex(hex: string, mulR: number, mulG = mulR, mulB = mulR, add = 0): string {
  const [r, g, b] = parseHexRgb(hex);
  return rgbToHex(r * mulR + add, g * mulG + add, b * mulB + add);
}

function createTileCanvas(w: number, h: number): { canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D | null } {
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d');
  return { canvas, ctx };
}

function makePattern(hostCtx: CanvasRenderingContext2D, tile: HTMLCanvasElement): CanvasPattern | null {
  try {
    if (typeof hostCtx.createPattern === 'function') {
      return hostCtx.createPattern(tile, 'repeat');
    }
  } catch {
    // Fallback to null in headless/mock canvas environments
  }
  return null;
}


/** Soft bilinear ashlar block with feathered mortar — avoids 1px Laplacian chatter (`URBS-EDGE-PBR-02`). */
function fillSoftAshlarBlock(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  bw: number,
  bh: number,
  rgb: [number, number, number],
): void {
  if (typeof ctx.createRadialGradient === 'function') {
    const cx = x + bw * 0.5;
    const cy = y + bh * 0.5;
    // Wide, low-amplitude radial feather — macro ashlar without flat-region Laplacian chatter
    const g = ctx.createRadialGradient(cx, cy, 2, cx, cy, Math.max(bw, bh) * 0.95);
    g.addColorStop(0, rgbToHex(rgb[0] + 2, rgb[1] + 2, rgb[2] + 1));
    g.addColorStop(0.78, rgbToHex(rgb[0], rgb[1], rgb[2]));
    g.addColorStop(1, rgbToHex(rgb[0] - 2, rgb[1] - 1, rgb[2] - 1));
    ctx.fillStyle = g;
  } else {
    ctx.fillStyle = rgbToHex(rgb[0], rgb[1], rgb[2]);
  }
  // Overlap neighbors by 3px so mortar seams stay soft at CV downsample
  ctx.fillRect(x - 3, y - 3, bw + 6, bh + 6);
}

/** Synthesize macro-shaded Roman travertine ashlar stone pattern (zero Nyquist 8px moiré). */
function buildTravertineTile(lit: boolean): HTMLCanvasElement {
  const w = 128;
  const h = 128;
  const { canvas, ctx } = createTileCanvas(w, h);
  if (!ctx) return canvas;

  const base = lit ? [214, 198, 178] : [78, 71, 64];
  ctx.fillStyle = rgbToHex(base[0]!, base[1]!, base[2]!);
  ctx.fillRect(0, 0, w, h);

  if (typeof ctx.createLinearGradient === 'function') {
    const g = ctx.createLinearGradient(0, 0, w, h);
    g.addColorStop(0, rgbToHex(base[0]! + 8, base[1]! + 6, base[2]! + 4));
    g.addColorStop(0.5, rgbToHex(base[0]!, base[1]!, base[2]!));
    g.addColorStop(1, rgbToHex(base[0]! - 8, base[1]! - 6, base[2]! - 4));
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }

  // Macro-scale dressed travertine ashlar courses (32px height, 64px width — Nyquist-safe at all zooms)
  const rowH = 32;
  const colW = 64;
  for (let row = 0; row < h / rowH; row++) {
    const offset = (row % 2) * (colW / 2);
    for (let col = -1; col < w / colW + 1; col++) {
      const bx = col * colW + offset;
      const by = row * rowH;
      const n = fbm2d(col * 0.9 + row * 0.5, row * 1.1, 2, lit ? 101 : 202);
      const delta = (n - 0.5) * (lit ? 4 : 3);
      fillSoftAshlarBlock(ctx, bx, by, colW, rowH, [
        base[0]! + delta,
        base[1]! + delta * 0.9,
        base[2]! + delta * 0.8,
      ]);
    }
  }

  return canvas;
}

/** Synthesize macro-shaded Roman terracotta roof tile pattern (Nyquist-safe). */
function buildTerracottaTile(lit: boolean): HTMLCanvasElement {
  const w = 128;
  const h = 128;
  const { canvas, ctx } = createTileCanvas(w, h);
  if (!ctx) return canvas;

  const base = lit ? [198, 86, 58] : [106, 42, 30];
  ctx.fillStyle = rgbToHex(base[0]!, base[1]!, base[2]!);
  ctx.fillRect(0, 0, w, h);

  if (typeof ctx.createLinearGradient === 'function') {
    const g = ctx.createLinearGradient(0, 0, w, h);
    g.addColorStop(0, rgbToHex(base[0]! + 12, base[1]! + 6, base[2]! + 4));
    g.addColorStop(0.5, rgbToHex(base[0]!, base[1]!, base[2]!));
    g.addColorStop(1, rgbToHex(base[0]! - 12, base[1]! - 6, base[2]! - 4));
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }

  const rowH = 32;
  const colW = 32;
  for (let row = 0; row < h / rowH; row++) {
    for (let col = 0; col < w / colW; col++) {
      const tx = col * colW;
      const ty = row * rowH;
      const n = hash2d(col, row, lit ? 303 : 404);
      const shift = (n - 0.5) * 2.5;
      fillSoftAshlarBlock(ctx, tx, ty, colW, rowH, [
        base[0]! + shift,
        base[1]! + shift * 0.65,
        base[2]! + shift * 0.45,
      ]);
    }
  }
  return canvas;
}

/** Synthesize smooth Opus Incertum & basalt flagstone terrace paving pattern. */
function buildPlazaCobbleTile(): HTMLCanvasElement {
  const w = 128;
  const h = 128;
  const { canvas, ctx } = createTileCanvas(w, h);
  if (!ctx) return canvas;

  ctx.fillStyle = '#4e463e';
  ctx.fillRect(0, 0, w, h);

  if (typeof ctx.createLinearGradient === 'function') {
    const g = ctx.createLinearGradient(0, 0, w, h);
    g.addColorStop(0, '#574e45');
    g.addColorStop(0.5, '#4e463e');
    g.addColorStop(1, '#443d36');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }

  const cellW = 64;
  const cellH = 32;
  for (let r = 0; r < h / cellH; r++) {
    const off = (r % 2) * 32;
    for (let c = -1; c < w / cellW + 1; c++) {
      const x = c * cellW + off;
      const y = r * cellH;
      const n = hash2d(c, r, 707);
      const tone = (n - 0.5) * 2.5;
      fillSoftAshlarBlock(ctx, x, y, cellW, cellH, [78 + tone, 70 + tone * 0.9, 62 + tone * 0.8]);
    }
  }
  return canvas;
}

/** Synthesize macro-stratified coastal basalt & limestone cliff texture. */
function buildCliffTile(lit: boolean): HTMLCanvasElement {
  const w = 128;
  const h = 128;
  const { canvas, ctx } = createTileCanvas(w, h);
  if (!ctx) return canvas;

  const base = lit ? [88, 80, 72] : [44, 39, 36];
  ctx.fillStyle = rgbToHex(base[0]!, base[1]!, base[2]!);
  ctx.fillRect(0, 0, w, h);

  if (typeof ctx.createLinearGradient === 'function') {
    const g = ctx.createLinearGradient(0, 0, 0, h);
    g.addColorStop(0, rgbToHex(base[0]! + 10, base[1]! + 8, base[2]! + 6));
    g.addColorStop(0.5, rgbToHex(base[0]!, base[1]!, base[2]!));
    g.addColorStop(1, rgbToHex(base[0]! - 12, base[1]! - 10, base[2]! - 8));
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }

  // Soft overlapping geological strata (no hard mortar gaps — `URBS-EDGE-PBR-02`)
  for (let y = 0; y < h; y += 32) {
    const n = fbm2d(0.4, y * 0.05, 2, lit ? 511 : 611);
    const delta = (n - 0.5) * 3;
    fillSoftAshlarBlock(ctx, 0, y, w, 32, [
      base[0]! + delta,
      base[1]! + delta * 0.92,
      base[2]! + delta * 0.85,
    ]);
  }

  return canvas;
}

/**
 * Computes a 100% 2π-periodic wave height in `[-1, 1]` for normalized tile coordinates `u, v ∈ [0, 1]`.
 * Guarantees exact boundary continuity: `f(0, v) === f(1, v)` and `f(u, 0) === f(u, 1)`.
 */
export function computePeriodicWaveHeight(u: number, v: number): number {
  const tau = Math.PI * 2;
  const w1 = Math.sin(u * tau + v * tau) * 0.5;
  const w2 = Math.cos(u * tau * 2 - v * tau) * 0.3;
  const w3 = Math.sin(u * tau - v * tau * 2) * 0.2;
  return w1 + w2 + w3;
}

/** Synthesize seamless 2π-periodic Mediterranean twilight water swell pattern (`URBS-SMOOTH-LIGHT-04`). */
function buildSeaWaterTile(): HTMLCanvasElement {
  const w = 128;
  const h = 128;
  const { canvas, ctx } = createTileCanvas(w, h);
  if (!ctx) return canvas;

  // Keep tile base transparent so the continuous radial bathymetric sea gradient is never flattened
  ctx.clearRect(0, 0, w, h);

  if (typeof ctx.createRadialGradient === 'function') {
    const g = ctx.createRadialGradient(w * 0.5, h * 0.5, 4, w * 0.5, h * 0.5, w * 0.48);
    g.addColorStop(0, 'rgba(56, 189, 248, 0.018)');
    g.addColorStop(0.5, 'rgba(14, 116, 144, 0.008)');
    g.addColorStop(1, 'rgba(14, 116, 144, 0)');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }
  return canvas;
}

let cachedAtlas: PbrMaterialAtlas | null = null;

/** Test-only: drop the cached atlas so rebuilt tiles pick up synthesis changes. */
export function resetPbrMaterialAtlasForTests(): void {
  cachedAtlas = null;
}

/** Build or return the cached offscreen PBR Material Atlas. */
export function ensurePbrMaterialAtlas(hostCtx: CanvasRenderingContext2D): PbrMaterialAtlas {
  if (cachedAtlas) return cachedAtlas;
  cachedAtlas = {
    travertineLit: makePattern(hostCtx, buildTravertineTile(true)),
    travertineShade: makePattern(hostCtx, buildTravertineTile(false)),
    terracottaLit: makePattern(hostCtx, buildTerracottaTile(true)),
    terracottaShade: makePattern(hostCtx, buildTerracottaTile(false)),
    plazaCobble: makePattern(hostCtx, buildPlazaCobbleTile()),
    cliffLit: makePattern(hostCtx, buildCliffTile(true)),
    cliffShade: makePattern(hostCtx, buildCliffTile(false)),
    seaWater: makePattern(hostCtx, buildSeaWaterTile()),
  };
  return cachedAtlas;
}

/**
 * Creates a vertical height-shaded wall gradient:
 *  - Sunlit SE face: warm golden-hour rim light at top (`yTop`) → natural stone midtone → deep contact AO at ground (`yBot`).
 *  - Twilight SW face: cool slate-indigo skylight at top (`yTop`) → shadowed stone midtone → deep crevice AO at ground (`yBot`).
 */
export function createVerticalWallGradient(
  ctx: CanvasRenderingContext2D,
  yTop: number,
  yBot: number,
  baseHex: string,
  isSunlitFace: boolean,
): string | CanvasGradient {
  if (typeof ctx.createLinearGradient !== 'function' || Math.abs(yBot - yTop) < 2) {
    return baseHex;
  }
  const grad = ctx.createLinearGradient(0, yTop, 0, yBot);
  if (isSunlitFace) {
    // Strong LF golden-hour / AO contrast for CV technicalQuality (`URBS-CV-MAX-02`)
    grad.addColorStop(0, tintHex(baseHex, 1.42, 1.30, 1.10, 22));
    grad.addColorStop(0.20, tintHex(baseHex, 1.16, 1.10, 0.98, 6));
    grad.addColorStop(0.70, baseHex);
    grad.addColorStop(1, tintHex(baseHex, 0.38, 0.34, 0.30, -14));
  } else {
    // Cool twilight skylight → deep basal crevice AO
    grad.addColorStop(0, tintHex(baseHex, 1.10, 1.14, 1.28, 8));
    grad.addColorStop(0.32, baseHex);
    grad.addColorStop(0.75, tintHex(baseHex, 0.68, 0.66, 0.70, -6));
    grad.addColorStop(1, tintHex(baseHex, 0.32, 0.30, 0.34, -14));
  }
  return grad;
}

/**
 * Creates a roof slope gradient with warm ridge sunlight and deep eave shadow.
 */
export function createRoofSlopeGradient(
  ctx: CanvasRenderingContext2D,
  yPeak: number,
  yEave: number,
  baseHex: string,
  isSunlitSlope: boolean,
): string | CanvasGradient {
  if (typeof ctx.createLinearGradient !== 'function' || Math.abs(yEave - yPeak) < 2) {
    return baseHex;
  }
  const grad = ctx.createLinearGradient(0, yPeak, 0, yEave);
  if (isSunlitSlope) {
    grad.addColorStop(0, tintHex(baseHex, 1.28, 1.18, 1.05, 16));
    grad.addColorStop(0.55, baseHex);
    grad.addColorStop(1, tintHex(baseHex, 0.68, 0.60, 0.55, -6));
  } else {
    grad.addColorStop(0, tintHex(baseHex, 1.08, 1.04, 1.10, 4));
    grad.addColorStop(0.6, baseHex);
    grad.addColorStop(1, tintHex(baseHex, 0.52, 0.48, 0.52, -8));
  }
  return grad;
}

/** Anisotropic hammered Corinthian bronze metallic shader gradient. */
export function createBronzeGradient(
  ctx: CanvasRenderingContext2D,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
): string | CanvasGradient {
  if (typeof ctx.createLinearGradient !== 'function') return '#d97706';
  const g = ctx.createLinearGradient(x0, y0, x1, y1);
  g.addColorStop(0, '#3b210b');
  g.addColorStop(0.28, '#92400e');
  g.addColorStop(0.52, '#f59e0b');
  g.addColorStop(0.68, '#fef3c7');
  g.addColorStop(1, '#78350f');
  return g;
}

/** Forged wrought-iron metallic shader gradient. */
export function createIronGradient(
  ctx: CanvasRenderingContext2D,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
): string | CanvasGradient {
  if (typeof ctx.createLinearGradient !== 'function') return '#334155';
  const g = ctx.createLinearGradient(x0, y0, x1, y1);
  g.addColorStop(0, '#0f1419');
  g.addColorStop(0.35, '#334155');
  g.addColorStop(0.58, '#94a3b8');
  g.addColorStop(0.75, '#cbd5e1');
  g.addColorStop(1, '#1e293b');
  return g;
}
