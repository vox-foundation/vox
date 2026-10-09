import { test, expect } from '@playwright/test';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { internalsDocIds, listDocPages } from '../../src/utils/page-index.mjs';
import { statusPolicy } from '../../src/utils/page-status.mjs';
import { DIST_DIR } from '../lib/dist';

const DOCS_SRC = fileURLToPath(new URL('../../../docs/src', import.meta.url));
const SITE_HOST = 'voxlang.org';

const PAGES = listDocPages(DOCS_SRC);
const INTERNALS = new Set(internalsDocIds(PAGES));

/** Titles of Internals pages that no other page also uses, so a heading match means the page itself. */
function internalsTitles(): Set<string> {
  const others = new Set(PAGES.filter((page) => !INTERNALS.has(page.id)).map((page) => page.title.trim()));
  const titles = new Set<string>();
  const shared: string[] = [];
  for (const page of PAGES) {
    if (!INTERNALS.has(page.id)) continue;
    const title = page.title.trim();
    if (others.has(title)) shared.push(title);
    else titles.add(title);
  }
  if (shared.length) console.log(`llms.spec: skipping titles shared with non-Internals pages: ${shared.join(' | ')}`);
  return titles;
}

/** Generated variants: the plugin's index, small and full files plus any custom sets. */
function generatedFiles(): string[] {
  const files = ['llms.txt', 'llms-small.txt', 'llms-full.txt'];
  const custom = join(DIST_DIR, '_llms-txt');
  if (existsSync(custom)) {
    for (const file of readdirSync(custom)) if (file.endsWith('.txt')) files.push(`_llms-txt/${file}`);
  }
  return files.filter((file) => existsSync(join(DIST_DIR, file)));
}

const headings = (text: string) =>
  text
    .split('\n')
    .filter((line) => line.startsWith('#'))
    .map((line) => line.replace(/^#+/, '').trim());

/** Path (`/<id>/`) of every same-site markdown link in a hand-written link list. */
function siteLinks(text: string): string[] {
  return [...text.matchAll(/\]\(([^)\s]+)\)/g)]
    .map((m) => m[1])
    .flatMap((href) => {
      if (href.startsWith('/')) return [href];
      try {
        const url = new URL(href);
        return url.hostname === SITE_HOST ? [url.pathname] : [];
      } catch {
        return [];
      }
    });
}

/** True when `route` maps to `dist/<route>/index.html` or a file in dist/. */
function builtTarget(route: string): boolean {
  const path = join(DIST_DIR, decodeURI(route.split(/[?#]/)[0]));
  if (route.endsWith('/')) return existsSync(join(path, 'index.html'));
  return existsSync(path) && (statSync(path).isFile() || existsSync(join(path, 'index.html')));
}

test.describe('llms.txt variants', () => {
  test('llms-full.txt and llms-small.txt are generated', () => {
    expect(existsSync(join(DIST_DIR, 'llms-full.txt'))).toBe(true);
    expect(existsSync(join(DIST_DIR, 'llms-small.txt'))).toBe(true);
  });

  const titles = internalsTitles();

  for (const file of generatedFiles()) {
    test(`${file} includes no Internals page`, () => {
      expect(INTERNALS.size).toBeGreaterThan(100);
      const text = readFileSync(join(DIST_DIR, file), 'utf8');
      const found = [...new Set(headings(text).filter((heading) => titles.has(heading)))];
      expect(found).toEqual([]);
    });
  }

  for (const file of ['llms.txt', 'llms-full.txt']) {
    test(`.well-known/${file} links only current pages`, () => {
      const path = join(DIST_DIR, '.well-known', file);
      test.skip(!existsSync(path), `dist/.well-known/${file} is not published`);
      const statusByRoute = new Map(PAGES.map((page) => [page.id ? `/${page.id}/` : '/', page.status]));
      const links = siteLinks(readFileSync(path, 'utf8'));
      expect(links.length).toBeGreaterThan(5);
      const notBuilt = links.filter((route) => !builtTarget(route));
      expect(notBuilt, 'links with no file in dist/').toEqual([]);
      // Mounted repo files (/repo/<route>/) are built pages but not docs/src pages.
      const notPages = links.filter((route) => !route.startsWith('/repo/') && !statusByRoute.has(route));
      expect(notPages, 'links to routes that are not docs pages').toEqual([]);
      const nonCurrent = links.filter((route) => statusPolicy(statusByRoute.get(route)).noindex);
      expect(nonCurrent, 'links to Internals, deprecated or legacy pages').toEqual([]);
    });
  }
});
