import { test, expect } from '@playwright/test';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';
import { internalsDocIds, listDocPages, noindexRoutes } from '../../src/utils/page-index.mjs';
import { DIST_DIR, readDist } from '../lib/dist';

const NOINDEX = /<meta name="robots" content="noindex"\/?>/;
const BANNER = /<div class="sl-banner[^"]*"[^>]*>(.*?)<\/div>/;
const DOCS_SRC = fileURLToPath(new URL('../../../docs/src', import.meta.url));

type Fragment = { url: string; filters?: Record<string, string[]>; meta?: Record<string, string> };

function readFragments(): Fragment[] {
  const dir = join(DIST_DIR, 'pagefind', 'fragment');
  return readdirSync(dir)
    .filter((file) => file.endsWith('.pf_fragment'))
    .map((file) => {
      const text = gunzipSync(readFileSync(join(dir, file))).toString('utf8');
      return JSON.parse(text.replace(/^pagefind_dcd/, '')) as Fragment;
    });
}

const PAGES = listDocPages(DOCS_SRC);
const INTERNALS = internalsDocIds(PAGES);

/** The top-level `<details>` sidebar group labelled `label`, and the whole top-level list. */
function sidebarGroup(html: string, label: string): { group: string; before: string; after: string } {
  const listStart = html.indexOf('<ul class="top-level');
  expect(listStart, 'no top-level sidebar list').toBeGreaterThanOrEqual(0);
  const tags = /<(\/?)(ul|details)\b[^>]*>/g;

  // End of the top-level list: its matching </ul>.
  tags.lastIndex = listStart;
  let depth = 0;
  let listEnd = -1;
  for (let m; (m = tags.exec(html)); ) {
    if (m[2] !== 'ul') continue;
    depth += m[1] ? -1 : 1;
    if (depth === 0) {
      listEnd = m.index + m[0].length;
      break;
    }
  }
  const list = html.slice(listStart, listEnd);

  const labelAt = list.search(new RegExp(`<span class="large[^"]*">${label}</span>`));
  expect(labelAt, `no sidebar group labelled ${label}`).toBeGreaterThan(0);
  const start = list.lastIndexOf('<details', labelAt);

  // The group must be top-level: no <details> is open around it.
  const opened = (list.slice(0, start).match(/<details\b/g) ?? []).length;
  const closed = (list.slice(0, start).match(/<\/details>/g) ?? []).length;
  expect(opened - closed, `${label} is nested inside another group`).toBe(0);

  const inner = /<(\/?)details\b[^>]*>/g;
  inner.lastIndex = start;
  depth = 0;
  let end = -1;
  for (let m; (m = inner.exec(list)); ) {
    depth += m[1] ? -1 : 1;
    if (depth === 0) {
      end = m.index + m[0].length;
      break;
    }
  }
  return { group: list.slice(start, end), before: list.slice(0, start), after: list.slice(end) };
}

const hrefs = (html: string) =>
  new Set([...html.matchAll(/href="([^"]+)"/g)].map((m) => m[1].replace(/\/$/, '')));

function sitemapUrls(): string[] {
  return readdirSync(DIST_DIR)
    .filter((file) => /^sitemap-\d+\.xml$/.test(file))
    .flatMap((file) => [...readFileSync(join(DIST_DIR, file), 'utf8').matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => m[1]));
}

test.describe('status banners and noindex', () => {
  test('a research page has the Internals banner and noindex but stays searchable', () => {
    const html = readDist('architecture/front-facing-honesty-audit-2026');
    expect(html.match(BANNER)?.[1]).toContain('Internals — research note');
    expect(html).toMatch(NOINDEX);
    expect(html).toContain('data-pagefind-body');
  });

  for (const [route, label] of [
    ['adr/028-deprecate-stub-durability-grammar', 'Deprecated:'],
    ['reference/mcp-tool-reference', 'Legacy:'],
  ]) {
    test(`${route} has a "${label}" banner and noindex`, () => {
      const html = readDist(route);
      expect(html.match(BANNER)?.[1]).toMatch(new RegExp(`^<strong>${label}</strong> `));
      expect(html).toMatch(NOINDEX);
    });
  }

  test('a current page has no banner and no robots meta', () => {
    const html = readDist('reference/cli');
    expect(html).not.toContain('sl-banner');
    expect(html).not.toContain('name="robots"');
  });

  test('every built research/roadmap page carries noindex', () => {
    expect(INTERNALS.length).toBeGreaterThan(100);
    const missing = INTERNALS.filter((id) => !NOINDEX.test(readDist(id)));
    expect(missing).toEqual([]);
  });
});

test.describe('Pagefind labelling', () => {
  test('the research page is indexed and labelled Internals; reference/cli is not', () => {
    const fragments = readFragments();
    const research = fragments.find((f) => f.url === '/architecture/front-facing-honesty-audit-2026/');
    expect(research, 'no Pagefind fragment for the research page').toBeTruthy();
    expect(research!.filters?.section ?? []).toContain('Internals');
    expect(research!.meta?.title ?? '').toMatch(/^Internals — /);

    const cli = fragments.find((f) => f.url === '/reference/cli/');
    expect(cli, 'no Pagefind fragment for reference/cli').toBeTruthy();
    expect(cli!.filters?.section ?? []).not.toContain('Internals');
    expect(cli!.meta?.title ?? '').not.toMatch(/^Internals/);
  });
});

test.describe('Internals sidebar group', () => {
  test('research/roadmap pages appear only in the last, collapsed Internals group', () => {
    const { group, before, after } = sidebarGroup(readDist(''), 'Internals');
    expect(group).toMatch(/^<details(?![^>]*\bopen\b)[^>]*>/);
    expect(after, 'a sidebar group follows Internals').not.toMatch(/<details\b/);

    const inside = hrefs(group);
    const outside = hrefs(before + after);
    const routes = INTERNALS.map((id) => `/${id}`);
    expect(routes.filter((route) => !inside.has(route))).toEqual([]);
    expect(routes.filter((route) => outside.has(route))).toEqual([]);
    expect(inside.size).toBeGreaterThanOrEqual(100);
    test.info().annotations.push({ type: 'internals-pages', description: String(routes.length) });
  });
});

test.describe('sitemap and robots.txt', () => {
  test('sitemap-index.xml is the only sitemap index', () => {
    expect(existsSync(join(DIST_DIR, 'sitemap-index.xml'))).toBe(true);
    expect(existsSync(join(DIST_DIR, 'sitemap.xml'))).toBe(false);
  });

  test('the sitemap lists no noindex page and not /retired/', () => {
    const urls = sitemapUrls();
    expect(urls.length).toBeGreaterThan(300);
    const noindex = noindexRoutes(PAGES);
    const listed = urls.map((url) => new URL(url).pathname).filter((path) => noindex.has(path) || path === '/retired/');
    expect(listed).toEqual([]);
  });

  test('robots.txt points at sitemap-index.xml and disallows nothing', () => {
    const robots = readFileSync(join(DIST_DIR, 'robots.txt'), 'utf8');
    expect(robots).toContain('Sitemap: https://voxlang.org/sitemap-index.xml');
    expect(robots).not.toMatch(/^\s*Disallow/im);
  });
});
