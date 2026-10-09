import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DIST_DIR, readDist } from '../lib/dist';

// P20-D8: docs/src/archive/ is tombstoned and no longer published.

const ARCHIVE_TREE = 'https://github.com/vox-foundation/vox/tree/main/docs/src/archive';
const repoRoot = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();

test('the archive is not built', () => {
  expect(existsSync(join(DIST_DIR, 'archive'))).toBe(false);
});

test('no sitemap URL points into the archive', () => {
  const sitemaps = readdirSync(DIST_DIR).filter((file) => /^sitemap-.*\.xml$/.test(file));
  expect(sitemaps.length).toBeGreaterThan(0);
  for (const file of sitemaps) {
    const locs = readFileSync(join(DIST_DIR, file), 'utf8').match(/<loc>[^<]*<\/loc>/g) ?? [];
    expect(locs.filter((loc) => loc.includes('/archive/')), file).toEqual([]);
  }
});

for (const route of ['tutorials/tut-getting-started', 'reference/cli']) {
  test(`reader page ${route} is still built`, () => {
    expect(existsSync(join(DIST_DIR, route, 'index.html'))).toBe(true);
  });
}

// Sitemap exclusion of /retired/ is asserted by honesty.spec, which owns the sitemap filter.
test('the retired notice page is noindex, out of Pagefind, and links only same-site or to the archive tree', () => {
  const html = readDist('retired');
  expect(html).toMatch(/<meta name="robots" content="noindex"/);
  expect(html).not.toContain('data-pagefind-body');
  expect(html).toContain(`href="${ARCHIVE_TREE}"`);
  const main = html.match(/<main[\s\S]*<\/main>/)?.[0] ?? '';
  const hrefs = [...main.matchAll(/href="([^"]*)"/g)].map((m) => m[1]);
  expect(hrefs.length).toBeGreaterThan(0);
  expect(hrefs.filter((href) => !href.startsWith('/') && !href.startsWith('#') && href !== ARCHIVE_TREE)).toEqual([]);
});

test('dist/_redirects sends /archive/* to /retired/ once and keeps every existing rule', () => {
  const lines = readFileSync(join(DIST_DIR, '_redirects'), 'utf8').split('\n');
  expect(lines.filter((line) => line === '/archive/* /retired/ 301')).toHaveLength(1);
  const committed = execFileSync('git', ['show', 'HEAD:docs-astro/public/_redirects'], { cwd: repoRoot, encoding: 'utf8' })
    .split('\n')
    .filter((line) => line.trim() !== '' && !line.startsWith('#'));
  expect(committed.length).toBeGreaterThan(25);
  expect(committed.filter((line) => !lines.includes(line))).toEqual([]);
});
