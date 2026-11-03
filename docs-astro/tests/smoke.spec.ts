import { test, expect } from '@playwright/test';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import matter from 'gray-matter';
import { extractLlmsUrls, checkUrls } from './lib/llms-links.mjs';
// @ts-ignore
import { listDocPages, internalsDocIds } from '../src/utils/page-index.mjs';

test.setTimeout(60_000);

const PRIMARY = process.env.BASE_URL ?? 'https://voxlang.org';
const REPO_ROOT = resolve(process.cwd(), '..');
const DOCS_SRC = join(REPO_ROOT, 'docs/src');

// Load retired symbols contract
const RETIRED_SYMBOLS_PATH = join(REPO_ROOT, 'contracts/documentation/retired-symbols.v1.yaml');
const contractRaw = existsSync(RETIRED_SYMBOLS_PATH) ? readFileSync(RETIRED_SYMBOLS_PATH, 'utf8') : '';
const parsedContract = matter(`---\n${contractRaw}\n---`).data;
const symbols = Array.isArray(parsedContract?.symbols) ? parsedContract.symbols : [];
const compiledSymbols = symbols.map((s: any) => ({
  id: s.id,
  regex: new RegExp(s.pattern),
}));

// Load internals IDs from docs/src
const allPages = existsSync(DOCS_SRC) ? listDocPages(DOCS_SRC) : [];
const internalsIds = new Set(internalsDocIds(allPages));

test.describe('voxlang.org live site', () => {
  test('home page loads with Vox title', async ({ page }) => {
    const resp = await page.goto(PRIMARY + '/', { waitUntil: 'domcontentloaded' });
    expect(resp?.status()).toBe(200);
    await expect(page).toHaveTitle(/Vox/);
  });

  test('homepage HTML does not teach retired @endpoint syntax', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    const html = await resp.text();
    expect(html).not.toContain('@endpoint');
    expect(html).not.toContain('@table type');
    expect(html).not.toContain('@mcp.tool');
  });

  test('stability matrix is published', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/reference/stability/', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
  });

  test('/voxup is published and starts with #!', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/voxup', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    const text = await resp.text();
    expect(text.startsWith('#!')).toBe(true);
  });

  test('/voxup.ps1 is published', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/voxup.ps1', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
  });

  test('every URL in /llms.txt resolves and contains no Internals pages', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/llms.txt', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    const body = await resp.text();
    expect(body.toLowerCase()).toMatch(/pre-1\.0|0\.6\.0|stability/);

    const urls = extractLlmsUrls(body, { baseUrl: PRIMARY });
    expect(urls.length).toBeGreaterThan(0);

    for (const u of urls) {
      const cleanPath = u.replace(PRIMARY, '').replace(/^\/+|\/+$/g, '');
      expect(internalsIds.has(cleanPath), `Internals page found in /llms.txt: ${cleanPath}`).toBe(false);
    }

    const result = await checkUrls(urls, async (url) => request.get(url, { timeout: 15_000 }));
    expect(result.ok, `Broken URLs in /llms.txt: ${JSON.stringify(result.failures)}`).toBe(true);
  });

  test('every URL in /.well-known/llms.txt resolves and contains no Internals pages', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/.well-known/llms.txt', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    const body = await resp.text();
    expect(body.toLowerCase()).toMatch(/pre-1\.0|0\.6\.0|stability/);
    expect(body).toContain('table');
    expect(body).not.toMatch(/@endpoint\(kind/);

    const urls = extractLlmsUrls(body, { baseUrl: PRIMARY });
    expect(urls.length).toBeGreaterThan(0);

    for (const u of urls) {
      const cleanPath = u.replace(PRIMARY, '').replace(/^\/+|\/+$/g, '');
      expect(internalsIds.has(cleanPath), `Internals page found in /.well-known/llms.txt: ${cleanPath}`).toBe(false);
    }

    const result = await checkUrls(urls, async (url) => request.get(url, { timeout: 15_000 }));
    expect(result.ok, `Broken URLs in /.well-known/llms.txt: ${JSON.stringify(result.failures)}`).toBe(true);
  });

  test('every URL in /llms-full.txt resolves and contains no Internals pages', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/llms-full.txt', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    const body = await resp.text();

    const urls = extractLlmsUrls(body, { baseUrl: PRIMARY });
    expect(urls.length).toBeGreaterThan(0);

    for (const u of urls) {
      const cleanPath = u.replace(PRIMARY, '').replace(/^\/+|\/+$/g, '');
      expect(internalsIds.has(cleanPath), `Internals page found in /llms-full.txt: ${cleanPath}`).toBe(false);
    }

    const result = await checkUrls(urls, async (url) => request.get(url, { timeout: 15_000 }));
    expect(result.ok, `Broken URLs in /llms-full.txt: ${JSON.stringify(result.failures)}`).toBe(true);
  });

  test('sampled reader pages contain no retired syntax', async ({ request }) => {
    const samplePages = [
      '/',
      '/tutorials/tut-getting-started/',
      '/tutorials/tut-first-app/',
      '/tutorials/tut-actor-basics/',
      '/tutorials/tut-workflow-durability/',
      '/tutorials/tut-ui-integration/',
      '/tutorials/use-a-react-component-from-vox/',
      '/tutorials/tut-first-vox-app-checkpoints/',
      '/reference/installation/',
      '/reference/ref-syntax/',
      '/reference/cli/',
      '/reference/stability/',
    ];

    const violations: Array<{ page: string; id: string; line: string }> = [];
    for (const pagePath of samplePages) {
      const resp = await request.get(PRIMARY + pagePath, { timeout: 15_000 });
      expect(resp.status(), `Failed to fetch sample page ${pagePath}`).toBe(200);
      const html = await resp.text();
      const mainMatch = html.match(/<main[^>]*>([\s\S]*?)<\/main>/i);
      const mainHtml = mainMatch ? mainMatch[1] : html;
      const text = mainHtml
        .replace(/<[^>]+>/g, ' ')
        .replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>')
        .replace(/&quot;/g, '"')
        .replace(/&amp;/g, '&');

      const lines = text.split('\n').map((l) => l.trim()).filter(Boolean);
      for (const line of lines) {
        if (/retired|deprecated|removed|replaced by|instead of/i.test(line)) continue;
        for (const sym of compiledSymbols) {
          if (sym.regex.test(line)) {
            violations.push({ page: pagePath, id: sym.id, line });
          }
        }
      }
    }

    expect(
      violations.length,
      `Retired syntax violations found:\n${violations.map((v) => `  - [${v.page}] ${v.id}: ${v.line}`).join('\n')}`
    ).toBe(0);
  });

  test('archive redirect', async ({ request }) => {
    const isLocal = PRIMARY.includes('localhost') || PRIMARY.includes('127.0.0.1');
    test.skip(isLocal, '_redirects is applied only by Cloudflare Pages');

    const resp = await request.get(PRIMARY + '/archive/p20-redirect-probe/', { maxRedirects: 0 });
    expect(resp.status()).toBe(301);
    const location = resp.headers()['location'];
    expect(location).toBeDefined();
    const locUrl = new URL(location!, PRIMARY);
    expect(locUrl.pathname).toBe('/retired/');
  });

  test('sidebar renders new section labels on a docs page', async ({ page }) => {
    // Sidebar appears on docs pages, not the splash. Pick a known sidebar page.
    await page.goto(PRIMARY + '/tutorials/tut-first-app/', { waitUntil: 'domcontentloaded' });
    const body = await page.locator('body').textContent({ timeout: 10_000 });
    expect(body).toContain('Getting Started');
    expect(body).toContain('How-To Guides');
    expect(body).toContain('Tutorials');
    expect(body).toContain('Language Reference');
  });

  test('pagefind search is available', async ({ page }) => {
    await page.goto(PRIMARY + '/', { waitUntil: 'domcontentloaded' });
    const searchTrigger = page.locator('button[aria-label*="search" i], [data-pagefind-search], input[type="search"]').first();
    await expect(searchTrigger).toBeVisible({ timeout: 10_000 });
  });

  test('www.voxlang.org also serves the site', async ({ request }) => {
    const resp = await request.get('https://www.voxlang.org/', { maxRedirects: 5, timeout: 15_000 });
    expect(resp.status()).toBe(200);
  });

  test('vox-lang.org redirects to voxlang.org with path preserved', async ({ request }) => {
    const resp = await request.get('https://vox-lang.org/getting-started', { maxRedirects: 0, timeout: 15_000 });
    expect([301, 302, 307, 308]).toContain(resp.status());
    const location = resp.headers()['location'];
    expect(location).toMatch(/^https:\/\/voxlang\.org\/getting-started/);
  });

  test('www.vox-lang.org also redirects', async ({ request }) => {
    const resp = await request.get('https://www.vox-lang.org/', { maxRedirects: 0, timeout: 15_000 });
    expect([301, 302, 307, 308]).toContain(resp.status());
    expect(resp.headers()['location']).toMatch(/^https:\/\/voxlang\.org\//);
  });
});
