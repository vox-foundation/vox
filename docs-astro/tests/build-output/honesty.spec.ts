import { test, expect } from '@playwright/test';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { gunzipSync } from 'node:zlib';
import matter from 'gray-matter';
import { readDist, DIST_DIR } from '../lib/dist';
// @ts-ignore
import { listDocPages, noindexRoutes } from '../../src/utils/page-index.mjs';

const REPO_ROOT = resolve(DIST_DIR, '..', '..');
const DOCS_SRC = join(REPO_ROOT, 'docs/src');

test.describe('Build Output: Page Status & Honesty (HONEST-01, HONEST-02)', () => {
  test('research page has banner, noindex, and data-pagefind-body', () => {
    const html = readDist('architecture/front-facing-honesty-audit-2026');
    expect(html).toContain('Internals — research note');
    expect(html).toContain('<meta name="robots" content="noindex"');
    expect(html).toContain('data-pagefind-body');
  });

  test('deprecated page has banner and noindex', () => {
    const html = readDist('adr/012-internal-web-ir-strategy');
    expect(html).toContain('Deprecated:');
    expect(html).toContain('<meta name="robots" content="noindex"');
  });

  test('legacy page has banner and noindex', () => {
    const html = readDist('reference/mcp-tool-reference');
    expect(html).toContain('Legacy:');
    expect(html).toContain('<meta name="robots" content="noindex"');
  });

  test('current reference/cli page has neither banner nor noindex', () => {
    const html = readDist('reference/cli');
    expect(html).not.toContain('sl-banner');
    expect(html).not.toContain('name="robots" content="noindex"');
  });

  test('all research and roadmap pages have noindex meta', () => {
    // Find all research and roadmap docs under docs/src
    function walk(dir: string, fileList: string[] = []): string[] {
      const files = readdirSync(dir, { withFileTypes: true });
      for (const file of files) {
        const fullPath = join(dir, file.name);
        if (file.isDirectory()) {
          if (file.name === 'archive' || file.name === '.well-known') continue;
          walk(fullPath, fileList);
        } else if (file.name.endsWith('.md') || file.name.endsWith('.mdx')) {
          fileList.push(fullPath);
        }
      }
      return fileList;
    }

    const allDocs = walk(DOCS_SRC);
    const researchOrRoadmapPages: string[] = [];

    for (const docPath of allDocs) {
      const raw = readFileSync(docPath, 'utf8');
      const { data } = matter(raw);
      const status = typeof data.status === 'string' ? data.status.trim().toLowerCase() : '';
      if (status === 'research' || status === 'roadmap') {
        const rel = docPath.slice(DOCS_SRC.length + 1).replace(/\.mdx?$/, '');
        researchOrRoadmapPages.push(rel);
      }
    }

    expect(researchOrRoadmapPages.length).toBeGreaterThan(100);

    let missingNoindex = 0;
    const failures: string[] = [];

    for (const rel of researchOrRoadmapPages) {
      try {
        const html = readDist(rel);
        if (!html.includes('name="robots" content="noindex"')) {
          missingNoindex++;
          failures.push(rel);
        }
      } catch {
        // Some pages might map differently (e.g. index)
      }
    }

    expect(missingNoindex, `Pages missing noindex: ${failures.join(', ')}`).toBe(0);
  });

  test('Pagefind search index labels Internals correctly in pf_fragment files', () => {
    const fragDir = join(DIST_DIR, 'pagefind/fragment');
    expect(existsSync(fragDir)).toBe(true);

    const files = readdirSync(fragDir).filter((f) => f.endsWith('.pf_fragment'));
    expect(files.length).toBeGreaterThan(100);

    let auditFrag: any = null;
    let cliFrag: any = null;

    for (const file of files) {
      let buf = readFileSync(join(fragDir, file));
      try {
        buf = gunzipSync(buf);
      } catch {
        // Raw buffer
      }
      let str = buf.toString('utf8');
      if (str.startsWith('pagefind_dcd')) {
        str = str.slice('pagefind_dcd'.length);
      }
      const data = JSON.parse(str);
      if (data.url === '/architecture/front-facing-honesty-audit-2026/') {
        auditFrag = data;
      }
      if (data.url === '/reference/cli/') {
        cliFrag = data;
      }
    }

    expect(auditFrag, 'Pagefind fragment for front-facing-honesty-audit-2026 not found').not.toBeNull();
    expect(auditFrag.filters?.section).toContain('Internals');
    expect(auditFrag.meta?.title).toMatch(/^Internals — /);

    expect(cliFrag, 'Pagefind fragment for reference/cli not found').not.toBeNull();
    const cliSections = cliFrag.filters?.section || [];
    expect(cliSections).not.toContain('Internals');
  });

  test('Internals group is the last top-level sidebar group and contains all internals pages', () => {
    const html = readDist('');
    const sidebarStart = html.indexOf('id="starlight__sidebar"');
    expect(sidebarStart).toBeGreaterThan(-1);
    const sidebarEnd = html.indexOf('</nav>', sidebarStart);
    const sidebar = html.slice(sidebarStart, sidebarEnd);

    const internalsIdx = sidebar.indexOf('>Internals<');
    expect(internalsIdx).toBeGreaterThan(-1);
    const startDetails = sidebar.lastIndexOf('<details', internalsIdx);
    expect(startDetails).toBeGreaterThan(-1);

    let depth = 0;
    let pos = startDetails;
    let endDetails = -1;
    while (pos < sidebar.length) {
      const nextOpen = sidebar.indexOf('<details', pos);
      const nextClose = sidebar.indexOf('</details>', pos);
      if (nextClose === -1) break;
      if (nextOpen !== -1 && nextOpen < nextClose) {
        depth++;
        pos = nextOpen + 8;
      } else {
        depth--;
        pos = nextClose + 10;
        if (depth === 0) {
          endDetails = pos;
          break;
        }
      }
    }
    expect(endDetails).toBeGreaterThan(-1);

    const internalsBlock = sidebar.slice(startDetails, endDetails);
    const sidebarOutside = sidebar.slice(0, startDetails) + sidebar.slice(endDetails);

    // After the internals block in the top-level list, there should be no more <details>
    const afterInternals = sidebar.slice(endDetails);
    expect(afterInternals).not.toContain('<details');

    const hrefRegex = /href="([^"]+)"/g;
    const internalsHrefs = new Set<string>();
    let m;
    while ((m = hrefRegex.exec(internalsBlock)) !== null) {
      internalsHrefs.add(m[1]);
    }

    expect(internalsHrefs.size).toBeGreaterThanOrEqual(100);

    for (const href of internalsHrefs) {
      expect(sidebarOutside).not.toContain(`href="${href}"`);
    }
  });

  test('sitemap-index.xml is generated, sitemap.xml is absent, and no noindex URLs are listed', () => {
    expect(existsSync(join(DIST_DIR, 'sitemap-index.xml'))).toBe(true);
    expect(existsSync(join(DIST_DIR, 'sitemap.xml'))).toBe(false);

    // @ts-ignore
    const allPages = listDocPages(DOCS_SRC);
    // @ts-ignore
    const noindexSet = noindexRoutes(allPages);

    const sitemapFiles = readdirSync(DIST_DIR).filter((f) => /^sitemap-.*\.xml$/.test(f));
    expect(sitemapFiles.length).toBeGreaterThan(0);

    for (const file of sitemapFiles) {
      const content = readFileSync(join(DIST_DIR, file), 'utf8');
      const locRegex = /<loc>([^<]+)<\/loc>/g;
      let m;
      while ((m = locRegex.exec(content)) !== null) {
        const urlStr = m[1];
        const pathname = new URL(urlStr).pathname;
        expect(noindexSet.has(pathname), `Noindex URL ${pathname} found in sitemap ${file}`).toBe(false);
        expect(pathname, `Retired notice URL found in sitemap ${file}`).not.toBe('/retired/');
      }
    }
  });

  test('robots.txt points to sitemap-index.xml and has no Disallow', () => {
    const robotsPath = join(DIST_DIR, 'robots.txt');
    expect(existsSync(robotsPath)).toBe(true);
    const robots = readFileSync(robotsPath, 'utf8');
    expect(robots).toContain('sitemap-index.xml');
    expect(robots).not.toContain('Disallow');
  });
});

