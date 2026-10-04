import { describe, it, expect } from 'vitest';
import { contrastRatio } from '../lib/contrast';
import { tokens } from '../styles/tokens.generated';
import { chatScopeFiles, readSrc, stripComments } from './sourceScan';

/**
 * Text classes that fall below 4.5:1 on the chat surfaces: zinc 500-700 (axe, 2026-09-28) and any
 * opacity-reduced text (translucent brass, muted or white text) — tokens are used at full strength.
 */
const LOW_CONTRAST_TEXT = /\btext-(?:zinc-(?:500|600|700)|brass\/\d+|text-muted\/\d+|white\/\d+)(?![\w/])/g;
const AA_TEXT = 4.5;

const SURFACES: Record<string, string> = {
  'bg.base': tokens.color.bg.base,
  'bg.surface': tokens.color.bg.surface,
  'bg.elevated': tokens.color.bg.elevated,
  'overlay.solid': tokens.color.overlay.solid,
};

/** The token colours this plan puts on chat text in place of zinc, translucent brass and palette status classes. */
const TEXT_TOKENS: Record<string, string> = {
  'text.muted': tokens.color.text.muted,
  'text.secondary': tokens.color.text.secondary,
  'accent.default (brass)': tokens.color.accent.default,
  'status.pass': tokens.color.status.pass,
  'status.fail': tokens.color.status.fail,
  'status.warn': tokens.color.status.warn,
  'status.info': tokens.color.status.info,
};

describe('replacement text tokens meet WCAG AA on every basalt surface', () => {
  for (const [fgName, fg] of Object.entries(TEXT_TOKENS)) {
    for (const [bgName, bg] of Object.entries(SURFACES)) {
      it(`${fgName} on ${bgName} is at least 4.5:1`, () => {
        expect(contrastRatio(fg, bg)).toBeGreaterThanOrEqual(AA_TEXT);
      });
    }
  }

  it('the replaced pairs really fail, so this suite discriminates', () => {
    // text-zinc-500 on the composer (overlay.solid): axe measured 3.79:1.
    expect(contrastRatio('#71717b', tokens.color.overlay.solid)).toBeLessThan(AA_TEXT);
    // text-brass/70 on the skill chip: axe measured #957a3c on #1c1d1d, 4.12:1.
    expect(contrastRatio('#957a3c', '#1c1d1d')).toBeLessThan(AA_TEXT);
  });
});

describe('low-contrast text guard', () => {
  it('flags zinc 500-700 and opacity-reduced text, passes the token classes (self-test)', () => {
    const bad = 'text-zinc-500 hover:text-zinc-600 text-zinc-700 text-brass/70 hover:text-brass/80 text-text-muted/70 text-white/40';
    expect([...bad.matchAll(LOW_CONTRAST_TEXT)]).toHaveLength(7);
    const good = 'text-text-muted text-text-secondary text-brass text-zinc-300 text-white bg-brass/6 border-brass/25 ring-brass/30 bg-white/5';
    expect([...good.matchAll(LOW_CONTRAST_TEXT)]).toHaveLength(0);
  });

  it('chat, composer and status bar use no low-contrast text classes', () => {
    const offenders = chatScopeFiles().flatMap((rel) =>
      [...stripComments(readSrc(rel)).matchAll(LOW_CONTRAST_TEXT)].map((m) => `${rel}: ${m[0]}`),
    );
    expect(offenders).toEqual([]);
  });
});
