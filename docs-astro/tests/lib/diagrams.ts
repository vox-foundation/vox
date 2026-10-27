import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DIST_DIR, listDistHtml } from './dist';

/**
 * Routes (`/reference/vox-diagram/`) of every built page that contains a VoxDiagram figure.
 * Discovered from `dist/`, so pages added by later phases are checked without editing the spec.
 */
export function figureRoutes(): string[] {
  return listDistHtml()
    .filter((file) => file.endsWith('index.html'))
    .filter((file) => readFileSync(join(DIST_DIR, file), 'utf8').includes('class="vd-figure"'))
    .map((file) => `/${file.replace(/index\.html$/, '')}`)
    .sort();
}

/** Font stacks the fit check runs under. `site` leaves the site's own `--vd-font` (self-hosted Arimo) alone. */
export const FONT_STACKS = {
  site: null,
  narrow: '"Liberation Sans", Arimo, Arial, Helvetica, sans-serif',
  wide: 'Verdana, "DejaVu Sans", sans-serif',
} as const;
export type StackName = keyof typeof FONT_STACKS;

/** WCAG 2 contrast ratio between two computed `rgb(...)`/`rgba(...)` colours. */
export function contrastRatio(a: string, b: string): number {
  const luminance = (color: string): number => {
    const m = color.match(/rgba?\(\s*([\d.]+)[ ,]+([\d.]+)[ ,]+([\d.]+)/);
    if (!m) throw new Error(`not an rgb colour: ${color}`);
    const [r, g, bl] = [m[1], m[2], m[3]].map((v) => {
      const c = Number(v) / 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
  };
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/**
 * Lifts a `figure.vd-figure.vd-natural` over the page at its natural width (no 0.6 floor, no scroll
 * clipping) so a screenshot captures the whole diagram. Shared by the review bundle and the
 * migration contact-sheet generator.
 */
export const NATURAL_CSS =
  '.vd-figure.vd-natural { position: fixed; left: 0; top: 0; z-index: 9999; width: max-content; max-width: none; overflow: visible; margin: 0; padding: 8px; background: var(--sl-color-bg); } body :is(header, nav, aside, .sidebar-pane, .right-sidebar-container) { visibility: hidden !important; } .vd-figure.vd-natural > svg { width: var(--vd-w); max-width: none; min-width: 0; }';
