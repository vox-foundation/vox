// crates/vox-gui/ui/src/components/gamify/urbs/lod.ts
import { projectIso, unprojectIso } from '../../../lib/projection';
import type { WorldBounds } from './camera';
import type { Pt, TownLayout } from './types';

export const TILE_W = 64;
export const TILE_H = 32;
/** 8-tile margin keeps out-of-grid landmarks (castrum/portus/gate) in-world. */
export const MARGIN_TILES = 8;
/** Below this zoom districts render as aggregate landmarks. */
export const LOD_THRESHOLD = 0.55;

export function bandForZoom(zoom: number): 0 | 1 {
  return zoom < LOD_THRESHOLD ? 0 : 1;
}

/** Iso-project a tile coordinate into world pixel space (origin ≥ 0). */
export function worldPx(layout: TownLayout, x: number, y: number): { px: number; py: number } {
  const offsetX = (layout.grid.h + MARGIN_TILES * 2) * (TILE_W / 2);
  const offsetY = MARGIN_TILES * TILE_H;
  return projectIso(x + MARGIN_TILES, y + MARGIN_TILES, 0, TILE_W, TILE_H, offsetX, offsetY);
}

/** Inverse of worldPx: world pixel → integer tile coordinate. */
export function tileFromWorld(layout: TownLayout, wx: number, wy: number): Pt {
  const offsetX = (layout.grid.h + MARGIN_TILES * 2) * (TILE_W / 2);
  const offsetY = MARGIN_TILES * TILE_H;
  const t = unprojectIso(wx, wy, TILE_W, TILE_H, offsetX, offsetY);
  return { x: Math.round(t.x) - MARGIN_TILES, y: Math.round(t.y) - MARGIN_TILES };
}

export function worldBounds(layout: TownLayout): WorldBounds {
  const n = layout.grid.w + MARGIN_TILES * 2;
  const m = layout.grid.h + MARGIN_TILES * 2;
  return { minX: 0, minY: 0, maxX: ((n + m) * TILE_W) / 2, maxY: ((n + m) * TILE_H) / 2 + 140 };
}

/**
 * Tight bounding box around the actually occupied districts and peripheral
 * Roman landmarks (`castrum`, `portus`, `aqvae`, `gate`) so `fitBounds`
 * centers the rendered Roman city in the viewport rather than centering the
 * asymmetric empty grid margin.
 */
export function occupiedWorldBounds(layout: TownLayout): WorldBounds {
  const pts: { px: number; py: number }[] = [];
  for (const d of layout.districts) {
    pts.push(worldPx(layout, d.x0, d.y0));
    pts.push(worldPx(layout, d.x1, d.y0));
    pts.push(worldPx(layout, d.x0, d.y1));
    pts.push(worldPx(layout, d.x1, d.y1));
  }
  const L = layout.landmarks;
  pts.push(worldPx(layout, L.castrum.x, L.castrum.y));
  pts.push(worldPx(layout, L.portus.x, L.portus.y));
  pts.push(worldPx(layout, L.aqvae.x, L.aqvae.y));
  pts.push(worldPx(layout, L.gate.x, L.gate.y));

  if (pts.length === 0) return worldBounds(layout);

  let minPx = Infinity;
  let maxPx = -Infinity;
  let minPy = Infinity;
  let maxPy = -Infinity;
  for (const p of pts) {
    if (p.px < minPx) minPx = p.px;
    if (p.px > maxPx) maxPx = p.px;
    if (p.py < minPy) minPy = p.py;
    if (p.py > maxPy) maxPy = p.py;
  }

  return {
    minX: Math.max(0, Math.round(minPx - 28)),
    minY: Math.max(0, Math.round(minPy - 48)),
    maxX: Math.round(maxPx + 28),
    maxY: Math.round(maxPy + 24),
  };
}

/** Map normalized [0..1] widescreen cityscape coordinates into world pixel space. */
export function uvToWorld(layout: TownLayout, u: number, v: number): { px: number; py: number } {
  const b = occupiedWorldBounds(layout);
  return {
    px: b.minX + u * (b.maxX - b.minX),
    py: b.minY + v * (b.maxY - b.minY),
  };
}

// NOTE (deliberate, spec §4): there is no per-frame viewport culling. The
// full world is painted once into an offscreen buffer and pan/zoom is a
// camera-transformed blit; the buffer's size is bounded by a render-scale
// clamp in worldRenderer. Culling would only help buffer *repaints*, which
// happen on data/LOD changes, not per frame.
