import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { SRC_ROOT, chatScopeFiles, readSrc, stripComments } from './sourceScan';

/** Tailwind palette classes that carry status meaning; status goes through --color-status-* instead. */
const PALETTE_STATUS = /\b(?:text|bg|border|ring|from|to|fill|stroke)-(?:amber|rose|red|emerald|cyan)-\d{2,3}\b/g;
/** Raw hex colours such as `bg-[#0b0b0e]`; components use tokens. */
const RAW_HEX = /#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b/g;
const STATUS_TOKEN_REF = /--color-status-[a-z]+/g;
/** A status class whose hover variant repeats it exactly (the recolour dropped the shade that told them apart). */
const HOVER_COLLAPSE = /(?<![\w:-])((?:text|bg|border)-\(--color-status-[a-z]+\)(?:\/\d+)?)(?=[\s"'`]).*hover:\1(?=[\s"'`])/;

function offendersIn(rel: string): string[] {
  const code = stripComments(readSrc(rel));
  return [...code.matchAll(PALETTE_STATUS), ...code.matchAll(RAW_HEX)].map((m) => `${rel}: ${m[0]}`);
}

describe('status colour guard patterns (self-test)', () => {
  it('flags palette status classes and raw hex, including variants and opacity', () => {
    const src = '<a className="hover:text-rose-300 bg-amber-400/8 border-emerald-400/20 from-cyan-400/40 text-red-400 bg-[#0b0b0e]" />';
    expect([...src.matchAll(PALETTE_STATUS)].map((m) => m[0])).toEqual([
      'text-rose-300',
      'bg-amber-400',
      'border-emerald-400',
      'from-cyan-400',
      'text-red-400',
    ]);
    expect([...src.matchAll(RAW_HEX)].map((m) => m[0])).toEqual(['#0b0b0e']);
  });

  it('passes token classes, brass and unrelated palette utilities', () => {
    const src = '<a className="text-(--color-status-warn) bg-(--color-status-fail)/6 ring-brass text-brass ring-offset-zinc-950 text-violet-300" />';
    expect([...src.matchAll(PALETTE_STATUS)]).toEqual([]);
    expect([...src.matchAll(RAW_HEX)]).toEqual([]);
  });

  it('flags a hover that repeats its base status class, passes distinct hovers', () => {
    expect(HOVER_COLLAPSE.test('className="text-(--color-status-pass) hover:text-(--color-status-pass) disabled:opacity-50"')).toBe(true);
    expect(HOVER_COLLAPSE.test('className="bg-(--color-status-fail)/12 hover:bg-(--color-status-fail)/12"')).toBe(true);
    expect(HOVER_COLLAPSE.test('className="text-(--color-status-pass) hover:brightness-125"')).toBe(false);
    expect(HOVER_COLLAPSE.test('className="bg-(--color-status-fail)/12 hover:bg-(--color-status-fail)/18"')).toBe(false);
    expect(HOVER_COLLAPSE.test('className="text-text-muted hover:text-(--color-status-fail)"')).toBe(false);
  });
});

describe('chat, composer and status bar route status colour through --color-status-* tokens', () => {
  it('scans a real, non-empty file set', () => {
    expect(chatScopeFiles()).toContain('components/layout/BottomStatusBar.tsx');
    expect(chatScopeFiles()).toContain('components/common/StatusBarCluster.tsx');
    expect(chatScopeFiles().length).toBeGreaterThanOrEqual(12);
  });

  it('uses no Tailwind palette status classes and no raw hex', () => {
    expect(chatScopeFiles().flatMap(offendersIn)).toEqual([]);
  });

  it('actually uses the status tokens', () => {
    expect(chatScopeFiles().filter((rel) => readSrc(rel).includes('--color-status-')).length).toBeGreaterThan(0);
  });

  it('never gives a status class a hover that repeats it', () => {
    const offenders = chatScopeFiles().flatMap((rel) =>
      stripComments(readSrc(rel))
        .split('\n')
        .filter((line) => HOVER_COLLAPSE.test(line))
        .map((line) => `${rel}: ${line.trim()}`),
    );
    expect(offenders).toEqual([]);
  });

  it('references only status tokens the generated token sheet defines', () => {
    // Seam with the real producer: Style Dictionary's output, not a copied list.
    const css = readFileSync(join(SRC_ROOT, 'styles/tokens.generated.css'), 'utf8');
    const defined = new Set([...css.matchAll(/(--color-status-[a-z]+):/g)].map((m) => m[1]));
    expect([...defined].sort()).toEqual([
      '--color-status-fail',
      '--color-status-info',
      '--color-status-pass',
      '--color-status-warn',
    ]);
    const missing = chatScopeFiles().flatMap((rel) =>
      [...readSrc(rel).matchAll(STATUS_TOKEN_REF)]
        .map((m) => m[0])
        .filter((t) => !defined.has(t))
        .map((t) => `${rel}: ${t}`),
    );
    expect(missing).toEqual([]);
  });
});
