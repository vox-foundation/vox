import { test, expect } from '@playwright/test';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';
import matter from 'gray-matter';
import { statusPolicy } from '../../src/utils/page-status.mjs';
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

/** Research/roadmap source pages whose naive lower-cased route was built. */
function internalsRoutes(): string[] {
  const routes: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== 'archive' && entry.name !== '.well-known') walk(full);
      } else if (/\.mdx?$/.test(entry.name)) {
        if (!statusPolicy(matter(readFileSync(full, 'utf8')).data.status).internals) continue;
        const route = relative(DOCS_SRC, full).split('\\').join('/').replace(/\.mdx?$/, '').toLowerCase();
        if (existsSync(join(DIST_DIR, route, 'index.html'))) routes.push(route);
      }
    }
  };
  walk(DOCS_SRC);
  return routes;
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
    const routes = internalsRoutes();
    expect(routes.length).toBeGreaterThan(100);
    const missing = routes.filter((route) => !NOINDEX.test(readDist(route)));
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
