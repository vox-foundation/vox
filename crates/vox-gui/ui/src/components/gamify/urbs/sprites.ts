// crates/vox-gui/ui/src/components/gamify/urbs/sprites.ts
/** Procedural Roman sprite atlas. No image assets: every sprite is drawn
 *  parametrically once per (scale, DPR) into an offscreen canvas, then stamped
 *  by the world renderer. Anchor (ax, ay) = the tile's ground-center point. */

export const SPRITE_KEYS = [
  'plate',            // district ground plate tile
  'hut', 'villa', 'insula', 'temple',       // building tiers 0-3
  'landmark',         // god-node crate temple (gold Pantheon pediment)
  'colosseum',        // AMPHITHEATRVM GUI (oval multi-tier Roman Colosseum)
  'praetorium',       // PRAETORIVM ORCHESTRATOR (peristyle basilica + emerald garden)
  'rotunda',          // ATHENAEVM MENS (circular Rotunda library temple)
  'fire-0', 'fire-1', // error flame animation frames
  'weeds',            // warning overlay
  'scaffold',         // active-agent-task overlay
  'castrum', 'tent', 'tent-busy',           // CI fort + runner tents
  'ship',             // orchestrator queue entry
  'arch', 'arch-dry', // aqueduct spans (MCP up/down)
  'gate', 'caravan',  // git gate + PR caravan
] as const;
export type SpriteKey = (typeof SPRITE_KEYS)[number];

export interface SpriteRect { x: number; y: number; w: number; h: number; ax: number; ay: number }
export interface AtlasPlan { width: number; height: number; rects: Record<SpriteKey, SpriteRect> }

const TILE_W = 64;
const TILE_H = 32;

/** Logical size of each sprite (before scale). Height leaves headroom for walls. */
const SIZES: Record<SpriteKey, { w: number; h: number; ay: number }> = {
  plate: { w: TILE_W, h: TILE_H, ay: TILE_H / 2 },
  hut: { w: TILE_W, h: 52, ay: 42 },
  villa: { w: TILE_W, h: 62, ay: 52 },
  insula: { w: TILE_W, h: 78, ay: 68 },
  temple: { w: TILE_W, h: 92, ay: 82 },
  landmark: { w: TILE_W * 2, h: 120, ay: 104 },
  colosseum: { w: TILE_W * 2, h: 116, ay: 100 },
  praetorium: { w: TILE_W * 2, h: 110, ay: 96 },
  rotunda: { w: 96, h: 98, ay: 84 },
  'fire-0': { w: 28, h: 34, ay: 32 },
  'fire-1': { w: 28, h: 34, ay: 32 },
  weeds: { w: 30, h: 18, ay: 9 },
  scaffold: { w: TILE_W, h: 60, ay: 50 },
  castrum: { w: 120, h: 90, ay: 78 },
  tent: { w: 26, h: 22, ay: 20 },
  'tent-busy': { w: 26, h: 22, ay: 20 },
  ship: { w: 44, h: 40, ay: 36 },
  arch: { w: 40, h: 54, ay: 50 },
  'arch-dry': { w: 40, h: 54, ay: 50 },
  gate: { w: 56, h: 64, ay: 56 },
  caravan: { w: 36, h: 26, ay: 22 },
};

/** Shelf-pack the sprites left-to-right into rows (max row width 1024·scale). */
export function planAtlas(scale: number): AtlasPlan {
  const maxW = 1024 * scale;
  const pad = 2 * scale;
  const rects = {} as Record<SpriteKey, SpriteRect>;
  let x = pad; let y = pad; let rowH = 0; let width = 0;
  for (const k of SPRITE_KEYS) {
    const w = SIZES[k].w * scale;
    const h = SIZES[k].h * scale;
    if (x + w + pad > maxW) { x = pad; y += rowH + pad; rowH = 0; }
    rects[k] = { x, y, w, h, ax: w / 2, ay: SIZES[k].ay * scale };
    x += w + pad;
    rowH = Math.max(rowH, h);
    width = Math.max(width, x);
  }
  return { width: Math.ceil(width), height: Math.ceil(y + rowH + pad), rects };
}

// ── Parametric drawers ──────────────────────────────────────────────────────
// Each draws into rect-local coordinates; s = rect.w / logical width.

type Ctx = CanvasRenderingContext2D;

function diamond(ctx: Ctx, cx: number, cy: number, w: number, h: number, fill: string, stroke?: string) {
  ctx.beginPath();
  ctx.moveTo(cx, cy - h / 2);
  ctx.lineTo(cx + w / 2, cy);
  ctx.lineTo(cx, cy + h / 2);
  ctx.lineTo(cx - w / 2, cy);
  ctx.closePath();
  ctx.fillStyle = fill;
  ctx.fill();
  if (stroke) { ctx.strokeStyle = stroke; ctx.stroke(); }
}

/** An iso block: top diamond + two visible walls, wall height `wh`, with soft ground AO and sunlit SE bevel. */
function block(ctx: Ctx, cx: number, groundY: number, w: number, wh: number,
  top: string, left: string, right: string) {
  const h = (w * TILE_H) / TILE_W;
  // Soft contact shadow ellipse under block
  ctx.fillStyle = 'rgba(4, 6, 10, 0.45)';
  ctx.beginPath();
  ctx.ellipse(cx - 2, groundY - h * 0.35, w * 0.56, h * 0.54, 0, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = right;
  ctx.beginPath();
  ctx.moveTo(cx, groundY); ctx.lineTo(cx + w / 2, groundY - h / 2);
  ctx.lineTo(cx + w / 2, groundY - h / 2 - wh); ctx.lineTo(cx, groundY - wh);
  ctx.closePath(); ctx.fill();
  ctx.fillStyle = left;
  ctx.beginPath();
  ctx.moveTo(cx, groundY); ctx.lineTo(cx - w / 2, groundY - h / 2);
  ctx.lineTo(cx - w / 2, groundY - h / 2 - wh); ctx.lineTo(cx, groundY - wh);
  ctx.closePath(); ctx.fill();
  diamond(ctx, cx, groundY - h / 2 - wh, w, h, top);

  // Specular sunlit SE ridge highlight
  ctx.strokeStyle = 'rgba(254, 243, 199, 0.46)';
  ctx.lineWidth = 0.9;
  ctx.beginPath();
  ctx.moveTo(cx, groundY - wh);
  ctx.lineTo(cx + w / 2, groundY - h / 2 - wh);
  ctx.stroke();
}

export function drawSprite(ctx: Ctx, key: SpriteKey, r: SpriteRect): void {
  ctx.save();
  ctx.translate(r.x, r.y);
  const s = r.w / SIZES[key].w;
  ctx.scale(s, s);
  const g = SIZES[key].ay; // ground line in logical units
  const cx = SIZES[key].w / 2;
  switch (key) {
    case 'plate': {
      // Stepped Roman travertine & obsidian-slate courtyard tile with subtle gold border
      diamond(ctx, cx, TILE_H / 2 + 2, TILE_W - 2, TILE_H - 2, '#141210', '#292524');
      diamond(ctx, cx, TILE_H / 2, TILE_W - 4, TILE_H - 4, '#1f1b18', '#44403c');
      diamond(ctx, cx, TILE_H / 2, TILE_W - 14, TILE_H - 10, '#26211d', '#57534e');
      break;
    }
    case 'hut': {
      // Roman Domus workshop with warm terracotta tiled roof & lit doorway
      block(ctx, cx, g, 36, 15, '#c25e00', '#3f3832', '#574e46');
      diamond(ctx, cx, g - 15 - 9, 38, 19, '#d97706', '#9a3412');
      ctx.fillStyle = '#fbbf24';
      ctx.fillRect(cx + 5, g - 10, 4, 7);
      break;
    }
    case 'villa': {
      // Patrician Peristyle Villa with terracotta wings & emerald courtyard
      block(ctx, cx, g, 46, 22, '#e07a16', '#443c35', '#61564c');
      diamond(ctx, cx, g - 22 - 11, 48, 24, '#b45309', '#f59e0b');
      diamond(ctx, cx, g - 22 - 11, 22, 11, '#065f46', '#34d399');
      ctx.fillStyle = '#fde68a';
      for (let i = 0; i < 3; i++) {
        ctx.fillRect(cx - 14 + i * 5, g - 16, 2, 6);
        ctx.fillRect(cx + 4 + i * 6, g - 16, 2, 6);
      }
      break;
    }
    case 'insula': {
      // Multi-storey Roman Insula / Basilica with arched windows & stepped penthouse
      block(ctx, cx, g, 48, 34, '#d6d3d1', '#3f3a34', '#57534e');
      ctx.fillStyle = '#fbbf24';
      for (let row = 0; row < 2; row++) {
        for (let col = 0; col < 3; col++) {
          ctx.fillRect(cx + 4 + col * 6, g - 28 + row * 10, 3, 5);
          ctx.fillRect(cx - 18 + col * 5, g - 26 + row * 10, 3, 5);
        }
      }
      block(ctx, cx, g - 34, 32, 13, '#b45309', '#292524', '#44403c');
      diamond(ctx, cx, g - 47 - 8, 34, 17, '#d97706', '#f59e0b');
      break;
    }
    case 'temple': {
      // Marble Corinthian Temple with golden pediment & lit portico
      block(ctx, cx, g, 56, 6, '#78716c', '#292524', '#44403c');
      block(ctx, cx, g - 6, 50, 32, '#e7e5e4', '#44403c', '#78716c');
      ctx.fillStyle = '#d6d3d1';
      for (let i = 0; i < 4; i++) {
        ctx.fillRect(cx + 3 + i * 6, g - 34, 3, 25);
        ctx.fillRect(cx - 21 + i * 5, g - 32, 2.5, 24);
      }
      diamond(ctx, cx, g - 44, 56, 26, '#e5a93c', '#fde68a');
      ctx.fillStyle = '#fbbf24';
      ctx.beginPath();
      ctx.arc(cx, g - 44, 4, 0, Math.PI * 2);
      ctx.fill();
      break;
    }
    case 'landmark': {
      // Grand Roman Pantheon / Forum Wonder with stepped marble podium, Corinthian colonnade & golden dome
      block(ctx, cx, g, 102, 8, '#57534e', '#292524', '#44403c');
      block(ctx, cx, g - 8, 92, 38, '#e7e5e4', '#44403c', '#78716c');
      ctx.fillStyle = '#f5f5f4';
      for (let i = 0; i < 6; i++) {
        ctx.fillRect(cx + 4 + i * 7, g - 42, 3.5, 30);
        ctx.fillRect(cx - 38 + i * 6, g - 40, 3, 28);
      }
      // Golden pediment roof
      diamond(ctx, cx, g - 54, 98, 42, '#b45309', '#e5a93c');
      // Stepped Pantheon rotunda & golden dome with glowing oculus
      ctx.fillStyle = '#d97706';
      ctx.beginPath();
      ctx.arc(cx, g - 62, 26, Math.PI, 0);
      ctx.closePath();
      ctx.fill();
      ctx.strokeStyle = '#fde68a';
      ctx.lineWidth = 1.5;
      ctx.stroke();
      ctx.fillStyle = '#e5a93c';
      ctx.beginPath();
      ctx.arc(cx, g - 64, 20, Math.PI, 0);
      ctx.closePath();
      ctx.fill();
      // Glowing oculus at apex
      ctx.fillStyle = '#fef08a';
      ctx.beginPath();
      ctx.arc(cx, g - 76, 4.5, 0, Math.PI * 2);
      ctx.fill();
      // Twin entrance braziers
      ctx.fillStyle = '#f59e0b';
      ctx.beginPath();
      ctx.arc(cx - 20, g - 6, 3.5, 0, Math.PI * 2);
      ctx.arc(cx + 20, g - 6, 3.5, 0, Math.PI * 2);
      ctx.fill();
      break;
    }
    case 'colosseum': {
      // AMPHITHEATRVM GUI — Multi-tiered oval Roman Colosseum with glowing amber arched arcades & warm arena floor
      diamond(ctx, cx, g - 6, 112, 54, '#1c1917', '#57534e');
      // Outer cylindrical travertine wall (lower & upper tiers)
      ctx.fillStyle = '#78716c';
      ctx.beginPath();
      ctx.ellipse(cx, g - 16, 48, 22, 0, 0, Math.PI);
      ctx.lineTo(cx - 48, g - 50);
      ctx.ellipse(cx, g - 50, 48, 22, 0, Math.PI, 0, true);
      ctx.closePath();
      ctx.fill();
      ctx.strokeStyle = '#d6d3d1';
      ctx.lineWidth = 1.4;
      ctx.stroke();
      // Cornice bands separating the 3 stories of arches
      for (const tierY of [g - 26, g - 38, g - 50]) {
        ctx.strokeStyle = '#e7e5e4';
        ctx.lineWidth = 1.2;
        ctx.beginPath();
        ctx.ellipse(cx, tierY, 48, 22, 0, 0.08 * Math.PI, 0.92 * Math.PI);
        ctx.stroke();
      }
      // Glowing arched arcades along the visible front wall
      for (let tier = 0; tier < 3; tier++) {
        const baseY = g - 16 - tier * 11;
        for (let i = -4; i <= 4; i++) {
          const angle = Math.PI * 0.5 + i * 0.095;
          const ax = cx + Math.cos(angle) * 44;
          const ay = baseY + Math.sin(angle) * 19 - 10;
          ctx.fillStyle = tier === 1 ? '#fbbf24' : '#f59e0b';
          ctx.fillRect(ax - 2, ay - 6, 4, 6);
        }
      }
      // Top rim, stepped cavea seating bowl & glowing golden-sand arena floor
      ctx.fillStyle = '#44403c';
      ctx.beginPath();
      ctx.ellipse(cx, g - 50, 48, 22, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = '#e5a93c';
      ctx.lineWidth = 1.5;
      ctx.stroke();
      ctx.fillStyle = '#78716c';
      ctx.beginPath();
      ctx.ellipse(cx, g - 49, 38, 17, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#d97706';
      ctx.beginPath();
      ctx.ellipse(cx, g - 48, 25, 11, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = '#fde68a';
      ctx.lineWidth = 1;
      ctx.stroke();
      break;
    }
    case 'praetorium': {
      // PRAETORIVM ORCHESTRATOR — Patrician Peristyle Basilica with twin terracotta halls & emerald courtyard garden
      block(ctx, cx, g, 104, 8, '#57534e', '#292524', '#44403c');
      block(ctx, cx, g - 8, 92, 28, '#c2410c', '#44403c', '#78716c');
      diamond(ctx, cx, g - 48, 96, 44, '#b45309', '#f59e0b');
      // Lush emerald peristyle garden atrium in center
      diamond(ctx, cx, g - 48, 52, 24, '#065f46', '#34d399');
      diamond(ctx, cx, g - 48, 34, 16, '#047857', '#6ee7b7');
      // Central azure marble fountain in the peristyle garden
      ctx.fillStyle = '#38bdf8';
      ctx.beginPath();
      ctx.arc(cx, g - 48, 4.5, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = '#fde68a';
      ctx.lineWidth = 1.2;
      ctx.stroke();
      // Colonnade pillars along both wings
      ctx.fillStyle = '#f5f5f4';
      for (let i = 0; i < 6; i++) {
        ctx.fillRect(cx + 4 + i * 7, g - 32, 3, 20);
        ctx.fillRect(cx - 38 + i * 6, g - 30, 2.5, 18);
      }
      break;
    }
    case 'rotunda': {
      // ATHENAEVM MENS — Circular Temple of Vesta / Rotunda Library with Corinthian columns & stepped dome
      diamond(ctx, cx, g - 4, 84, 40, '#292524', '#78716c');
      ctx.fillStyle = '#57534e';
      ctx.beginPath();
      ctx.ellipse(cx, g - 12, 34, 15, 0, 0, Math.PI * 2);
      ctx.fill();
      // Circular colonnade drum
      ctx.fillStyle = '#d6d3d1';
      ctx.fillRect(cx - 28, g - 44, 56, 30);
      ctx.fillStyle = '#f5f5f4';
      for (let i = -3; i <= 3; i++) {
        ctx.fillRect(cx + i * 7 - 1.5, g - 44, 3, 28);
      }
      // Stepped bronze & gold dome
      ctx.fillStyle = '#b45309';
      ctx.beginPath();
      ctx.arc(cx, g - 44, 30, Math.PI, 0);
      ctx.closePath();
      ctx.fill();
      ctx.strokeStyle = '#e5a93c';
      ctx.lineWidth = 1.5;
      ctx.stroke();
      ctx.fillStyle = '#d97706';
      ctx.beginPath();
      ctx.arc(cx, g - 48, 21, Math.PI, 0);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = '#fef08a';
      ctx.beginPath();
      ctx.arc(cx, g - 68, 4, 0, Math.PI * 2);
      ctx.fill();
      break;
    }
    case 'fire-0':
    case 'fire-1': {
      const lean = key === 'fire-1' ? 3 : -3;
      ctx.fillStyle = '#ea580c';
      ctx.beginPath();
      ctx.moveTo(6, g); ctx.quadraticCurveTo(10 + lean, g - 26, 14, g - 32);
      ctx.quadraticCurveTo(18 - lean, g - 22, 22, g);
      ctx.closePath(); ctx.fill();
      ctx.fillStyle = '#fbbf24';
      ctx.beginPath();
      ctx.moveTo(10, g); ctx.quadraticCurveTo(14 + lean / 2, g - 14, 14, g - 18);
      ctx.quadraticCurveTo(16, g - 12, 18, g);
      ctx.closePath(); ctx.fill();
      break;
    }
    case 'weeds':
      ctx.strokeStyle = '#4d7c0f';
      ctx.lineWidth = 2;
      for (const dx of [4, 12, 22]) {
        ctx.beginPath();
        ctx.moveTo(dx, g); ctx.quadraticCurveTo(dx - 2, g - 9, dx + 1, g - 12);
        ctx.stroke();
      }
      break;
    case 'scaffold':
      ctx.strokeStyle = '#f59e0b';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(6, g - 44); ctx.lineTo(6, g);
      ctx.moveTo(SIZES.scaffold.w - 6, g - 44); ctx.lineTo(SIZES.scaffold.w - 6, g);
      ctx.moveTo(6, g - 38); ctx.lineTo(SIZES.scaffold.w - 6, g - 8);
      ctx.moveTo(6, g - 8); ctx.lineTo(SIZES.scaffold.w - 6, g - 38);
      ctx.stroke();
      break;
    case 'castrum': {
      // Fortified Roman Legion Castrum with watchtowers, gatehouse & crimson/gold SPQR standard
      diamond(ctx, 60, g - 24, 112, 52, '#292524', '#78716c');
      diamond(ctx, 60, g - 24, 96, 44, '#1c1917', '#b45309');
      ctx.fillStyle = '#57534e';
      for (const [tx, ty] of [[12, g - 32], [96, g - 32], [54, g - 52], [54, g - 8]]) {
        ctx.fillRect(tx, ty, 12, 16);
        ctx.fillStyle = '#b45309';
        ctx.fillRect(tx - 1, ty - 3, 14, 4);
        ctx.fillStyle = '#57534e';
      }
      // SPQR standard banner
      ctx.strokeStyle = '#fde68a'; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.moveTo(60, g - 30); ctx.lineTo(60, g - 74); ctx.stroke();
      ctx.fillStyle = '#dc2626';
      ctx.fillRect(61, g - 74, 16, 10);
      ctx.strokeStyle = '#e5a93c'; ctx.lineWidth = 1;
      ctx.strokeRect(61, g - 74, 16, 10);
      break;
    }
    case 'tent':
    case 'tent-busy':
      ctx.fillStyle = key === 'tent-busy' ? '#dc2626' : '#57534e';
      ctx.strokeStyle = key === 'tent-busy' ? '#fbbf24' : '#a8a29e';
      ctx.lineWidth = 1.2;
      ctx.beginPath();
      ctx.moveTo(2, g); ctx.lineTo(13, g - 16); ctx.lineTo(24, g);
      ctx.closePath(); ctx.fill(); ctx.stroke();
      break;
    case 'ship': {
      // Roman Trireme Galley on azure harbor water ripple with bronze ram & golden sail
      ctx.fillStyle = 'rgba(14, 116, 144, 0.45)';
      ctx.beginPath();
      ctx.ellipse(22, g - 2, 20, 6, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = '#38bdf8';
      ctx.lineWidth = 1;
      ctx.stroke();
      // Hull
      ctx.fillStyle = '#92400e';
      ctx.beginPath();
      ctx.moveTo(3, g - 9); ctx.quadraticCurveTo(22, g + 1, 41, g - 9);
      ctx.lineTo(36, g - 2); ctx.lineTo(8, g - 2); ctx.closePath(); ctx.fill();
      // Mast & sail
      ctx.strokeStyle = '#d6d3d1'; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.moveTo(22, g - 7); ctx.lineTo(22, g - 32); ctx.stroke();
      ctx.fillStyle = '#e5a93c';
      ctx.beginPath();
      ctx.moveTo(12, g - 28); ctx.quadraticCurveTo(22, g - 32, 34, g - 26);
      ctx.lineTo(32, g - 14); ctx.quadraticCurveTo(22, g - 16, 14, g - 14);
      ctx.closePath(); ctx.fill();
      break;
    }
    case 'arch':
    case 'arch-dry': {
      // Roman Stone Aqueduct span with glowing cyan MCP water channel on top
      ctx.fillStyle = key === 'arch' ? '#57534e' : '#292524';
      ctx.fillRect(4, g - 44, 7, 44);
      ctx.fillRect(29, g - 44, 7, 44);
      ctx.fillRect(2, g - 48, 36, 6);
      ctx.strokeStyle = key === 'arch' ? '#a8a29e' : '#44403c';
      ctx.lineWidth = 2.5;
      ctx.beginPath(); ctx.arc(20, g - 28, 10, Math.PI, 0); ctx.stroke();
      if (key === 'arch') {
        // Glowing cyan MCP aqueduct stream
        ctx.fillStyle = '#06b6d4';
        ctx.fillRect(2, g - 51, 36, 4);
        ctx.fillStyle = '#67e8f9';
        ctx.fillRect(4, g - 50, 32, 2);
      } else {
        ctx.strokeStyle = '#ef4444'; ctx.lineWidth = 2;
        ctx.beginPath(); ctx.moveTo(10, g - 44); ctx.lineTo(30, g - 24); ctx.stroke();
      }
      break;
    }
    case 'gate':
      // Porta Appia Twin-Tower Gatehouse
      block(ctx, cx, g, 48, 34, '#78716c', '#292524', '#44403c');
      ctx.fillStyle = '#e5a93c';
      ctx.fillRect(cx - 18, g - 34, 36, 3);
      ctx.fillStyle = '#090b10';
      ctx.beginPath(); ctx.arc(cx, g - 14, 10, Math.PI, 0);
      ctx.lineTo(cx + 10, g); ctx.lineTo(cx - 10, g); ctx.closePath(); ctx.fill();
      break;
    case 'caravan':
      ctx.fillStyle = '#d97706';
      ctx.fillRect(4, g - 15, 24, 11);
      ctx.fillStyle = '#fde68a';
      ctx.fillRect(6, g - 18, 20, 4);
      ctx.fillStyle = '#292524'; ctx.strokeStyle = '#e5a93c';
      for (const wx of [9, 23]) {
        ctx.beginPath(); ctx.arc(wx, g - 3, 3.5, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
      }
      break;
  }
  ctx.restore();
}

/** Build the atlas canvas. Browser-only (needs real 2D canvas). */
export function buildAtlas(scale: number): { canvas: HTMLCanvasElement; plan: AtlasPlan } {
  const plan = planAtlas(scale);
  const canvas = document.createElement('canvas');
  canvas.width = plan.width;
  canvas.height = plan.height;
  const ctx = canvas.getContext('2d')!;
  for (const k of SPRITE_KEYS) drawSprite(ctx, k, plan.rects[k]);
  return { canvas, plan };
}
