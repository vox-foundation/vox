import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { DIST_DIR, readDist, listDistHtml } from '../lib/dist';
// @ts-ignore
import { listDocPages } from '../../src/utils/page-index.mjs';

const REPO_ROOT = resolve(DIST_DIR, '..', '..');
const DOCS_SRC = join(REPO_ROOT, 'docs/src');

// starlight-github-alerts@0.5.0 default mapping.
const ALERT_TO_ASIDE: Record<string, string> = {
  NOTE: 'note',
  TIP: 'tip',
  IMPORTANT: 'caution',
  WARNING: 'caution',
  CAUTION: 'danger',
};
const TYPES = Object.keys(ALERT_TO_ASIDE).join('|');

interface Alert {
  type: string;
  line: number;
  indented: boolean;
}

/** Alert marker lines (`> [!TYPE]`) outside fenced code, with 1-based line numbers. */
function scanAlerts(src: string): Alert[] {
  const marker = new RegExp(`^(\\s*)>\\s*\\[!(${TYPES})\\]\\s*$`);
  const out: Alert[] = [];
  let fence: string | null = null;
  src.split(/\r?\n/).forEach((text, i) => {
    const f = /^\s*(```|~~~)/.exec(text);
    if (f) {
      if (fence === null) fence = f[1];
      else if (fence === f[1]) fence = null;
      return;
    }
    if (fence !== null) return;
    const m = marker.exec(text);
    if (m) out.push({ type: m[2], line: i + 1, indented: m[1].length > 0 });
  });
  return out;
}

const pages = listDocPages(DOCS_SRC).map((p: { relPath: string; id: string }) => ({
  relPath: p.relPath,
  id: p.id,
  alerts: scanAlerts(readFileSync(join(DOCS_SRC, p.relPath), 'utf8')),
}));
const alertPages = pages.filter((p: { alerts: Alert[] }) => p.alerts.length > 0);

test.describe('Build Output: GitHub alerts render as Starlight asides (DOCS-PARITY-03)', () => {
  test('alert pages render mapped asides', () => {
    expect(alertPages.length, 'no docs page has an alert (vacuous pass)').toBeGreaterThan(0);
    for (const page of alertPages) {
      const html = readDist(page.id);
      for (const type of new Set(page.alerts.map((a: Alert) => a.type))) {
        const cls = `starlight-aside--${ALERT_TO_ASIDE[type as string]}`;
        expect(html.includes(cls), `${page.relPath}: [!${type}] did not render ${cls}`).toBe(true);
      }
    }
  });

  test('no raw alert blockquote survives', () => {
    const raw = new RegExp(`<blockquote[^>]*>\\s*<p[^>]*>\\s*\\[!(${TYPES})\\]`, 'i');
    const bad = listDistHtml().filter((f) => raw.test(readFileSync(join(DIST_DIR, f), 'utf8')));
    expect(bad, `raw [!TYPE] blockquotes in: ${bad.join(', ')}`).toEqual([]);
  });

  // GitHub renders alerts only as top-level blockquotes; nested ones show as raw text there.
  test('no docs/src page nests an alert', () => {
    const nested = pages.flatMap((p: { relPath: string; alerts: Alert[] }) =>
      p.alerts.filter((a) => a.indented).map((a) => `${p.relPath}:${a.line}`),
    );
    expect(nested, `indented alert markers: ${nested.join(', ')}`).toEqual([]);
  });
});
