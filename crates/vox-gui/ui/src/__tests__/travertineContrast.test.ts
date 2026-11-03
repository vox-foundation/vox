import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { contrastRatio } from '../lib/contrast';
import { SRC_ROOT } from './sourceScan';

/** `--color-*` hex values of the light (travertine) scope, read from the generated stylesheet. */
function travertine(): Record<string, string> {
  const css = readFileSync(join(SRC_ROOT, 'styles/tokens.travertine.generated.css'), 'utf8');
  return Object.fromEntries([...css.matchAll(/--color-([a-z-]+):\s*(#[0-9a-fA-F]{6})\s*;/g)].map((m) => [m[1], m[2]]));
}

const AA_TEXT = 4.5;
const SURFACES = ['bg-base', 'bg-surface', 'bg-elevated', 'overlay-solid'];
const TEXT = ['text-primary', 'text-secondary', 'text-muted', 'accent-default', 'status-pass', 'status-fail', 'status-warn', 'status-info'];

describe('travertine (light) scope: text tokens meet WCAG AA on every surface', () => {
  const t = travertine();

  it('reads a real token set', () => {
    for (const k of [...SURFACES, ...TEXT]) expect(t[k], `missing --color-${k}`).toBeTruthy();
  });

  for (const fg of TEXT) {
    for (const bg of SURFACES) {
      it(`${fg} on ${bg} is at least 4.5:1`, () => {
        expect(contrastRatio(t[fg], t[bg])).toBeGreaterThanOrEqual(AA_TEXT);
      });
    }
  }

  it('discriminates: the dark-scope status text would fail on the light surface', () => {
    expect(contrastRatio('#79c8ba', t['bg-base'])).toBeLessThan(AA_TEXT);
  });
});
