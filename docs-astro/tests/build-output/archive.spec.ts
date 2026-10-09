import { test, expect } from '@playwright/test';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DIST_DIR } from '../lib/dist';

// P20-D8: docs/src/archive/ is tombstoned and no longer published.

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
