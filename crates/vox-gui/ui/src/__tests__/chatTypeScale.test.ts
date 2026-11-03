import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { SRC_ROOT, chatScopeFiles, readSrc, stripComments } from './sourceScan';

const MIN_DATA_PX = 11;
const PX_TEXT = /\btext-\[(\d+(?:\.\d+)?)px\]/g;
const TRACKING = /\btracking-(\[[^\]]+\]|[a-z]+)/g;
/** Non-caps tightening, allowed anywhere. */
const NEUTRAL_TRACKING = new Set(['tight', 'tighter', 'normal']);
/** One string literal: a whole template (even multi-line), or a single-line quoted string. */
const STRING_LITERAL = /`[^`]*`|"[^"\n]*"|'[^'\n]*'/g;
const BRASS_GLOW = /shadow-\[[^\]]*--brass[^\]]*\]/g;

/** Tracking offenders in one class string: display caps (font-display) use 0.13em, everything else 0.08em. */
function trackingOffenders(classes: string): string[] {
  const want = classes.includes('font-display') ? '[0.13em]' : '[0.08em]';
  return [...classes.matchAll(TRACKING)]
    .filter((m) => !NEUTRAL_TRACKING.has(m[1]) && m[1] !== want)
    .map((m) => `${m[0]} (want tracking-${want})`);
}

function typeOffenders(rel: string): string[] {
  const code = stripComments(readSrc(rel));
  const out: string[] = [];
  for (const m of code.matchAll(PX_TEXT)) if (parseFloat(m[1]) < MIN_DATA_PX) out.push(`${rel}: ${m[0]}`);
  for (const s of code.matchAll(STRING_LITERAL)) for (const t of trackingOffenders(s[0])) out.push(`${rel}: ${t}`);
  for (const m of code.matchAll(BRASS_GLOW)) out.push(`${rel}: ${m[0]}`);
  return out;
}

function cssBlock(css: string, start: string): string {
  const i = css.indexOf(start);
  expect(i, `${start} missing`).toBeGreaterThan(-1);
  return css.slice(i, css.indexOf('}', i));
}
const appCss = readFileSync(join(SRC_ROOT, 'index.css'), 'utf8');
const dsCss = readFileSync(join(SRC_ROOT, '..', '..', 'ds', 'components.css'), 'utf8');

describe('type-scale guard patterns (self-test)', () => {
  it('flags small text and brass glows; passes 11px and neutral shadows', () => {
    const bad = 'text-[10px] text-[9.5px] shadow-[0_0_24px_-8px_rgb(var(--brass)/0.6)]';
    expect([...bad.matchAll(PX_TEXT)].filter((m) => parseFloat(m[1]) < MIN_DATA_PX)).toHaveLength(2);
    expect([...bad.matchAll(BRASS_GLOW)]).toHaveLength(1);
    const good = 'text-[11px] text-[13px] shadow-lg shadow-[0_1px_0_rgba(255,255,255,0.04)_inset]';
    expect([...good.matchAll(PX_TEXT)].filter((m) => parseFloat(m[1]) < MIN_DATA_PX)).toHaveLength(0);
    expect([...good.matchAll(BRASS_GLOW)]).toHaveLength(0);
  });

  it('pairs tracking with the class string, not the line', () => {
    expect(trackingOffenders('font-display uppercase tracking-[0.13em]')).toEqual([]);
    expect(trackingOffenders('font-mono uppercase tracking-[0.08em]')).toEqual([]);
    expect(trackingOffenders('font-display uppercase tracking-[0.08em]')).toHaveLength(1);
    expect(trackingOffenders('uppercase tracking-[0.13em]')).toHaveLength(1);
    expect(trackingOffenders('uppercase tracking-widest')).toHaveLength(1);
    expect(trackingOffenders('text-[13px] tracking-tight')).toEqual([]);
    // A multi-line template literal is one class string: font-display on the first line governs line two.
    const multi = '<a className={`font-display text-[11px]\n  uppercase tracking-[0.08em] ${x ? "a" : "b"}`} />';
    expect([...multi.matchAll(STRING_LITERAL)].flatMap((s) => trackingOffenders(s[0]))).toHaveLength(1);
  });
});

describe('chat, composer and status bar type scale', () => {
  it('scans a real, non-empty file set', () => {
    expect(chatScopeFiles()).toContain('components/surfaces/Loquela/Loquela.tsx');
    expect(chatScopeFiles()).toContain('components/common/StatusBarCluster.tsx');
    expect(chatScopeFiles().length).toBeGreaterThanOrEqual(12);
  });

  it('has no data text under 11px, tracking paired to its caps style, and no brass glow', () => {
    expect(chatScopeFiles().flatMap(typeOffenders)).toEqual([]);
  });
});

describe('Limes stylesheets (no glows; section heads are display caps at data size)', () => {
  it('the range-slider thumb has no glow', () => {
    expect(cssBlock(appCss, '@utility vox-range')).not.toMatch(/box-shadow/);
  });

  it('the app section head is 11px display caps at 0.13em', () => {
    const block = cssBlock(appCss, '@utility ds-section-head');
    expect(block).toContain('font-size: 11px;');
    expect(block).toContain('letter-spacing: 0.13em;');
  });

  it('the design-system copy of the section head matches (ds/components.css says keep in sync)', () => {
    const block = cssBlock(dsCss, '.ds-section-head {');
    expect(block).toContain('font-size: 11px;');
    expect(block).toContain('letter-spacing: 0.13em;');
  });
});
