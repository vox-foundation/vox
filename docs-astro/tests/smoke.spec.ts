import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import matter from 'gray-matter';
import { internalsDocIds, listDocPages } from '../src/utils/page-index.mjs';
import { checkUrls, extractLlmsUrls } from './lib/llms-links.mjs';

test.setTimeout(60_000);

const PRIMARY = process.env.BASE_URL ?? 'https://voxlang.org';
const LLMS_FILES = ['/llms.txt', '/.well-known/llms.txt', '/.well-known/llms-full.txt'];
const IS_LOCAL = ['localhost', '127.0.0.1'].includes(new URL(PRIMARY).hostname);

const REPO_ROOT = fileURLToPath(new URL('../../', import.meta.url));
const INTERNALS_ROUTES = new Set(
  internalsDocIds(listDocPages(REPO_ROOT + 'docs/src')).map((id: string) => `/${id}/`),
);

type RetiredSymbol = { id: string; regex: RegExp };

/** `contracts/documentation/retired-symbols.v1.yaml`, plus retired decorator spellings it does not list. */
function retiredSymbols(): RetiredSymbol[] {
  const yaml = readFileSync(REPO_ROOT + 'contracts/documentation/retired-symbols.v1.yaml', 'utf8');
  const { symbols } = matter(`---\n${yaml}\n---\n`).data as { symbols: { id: string; pattern: string }[] };
  return [
    ...symbols.map((symbol) => ({ id: symbol.id, regex: new RegExp(symbol.pattern) })),
    { id: 'endpoint-decorator', regex: /@endpoint\s*\(\s*kind/ },
    { id: 'py-import-decorator', regex: /@py\.import\b/ },
  ];
}

const RETIRED = retiredSymbols();
const CARVE_OUT = /retired|deprecated|removed|replaced by|instead of/i;

/** Lines the CI retired-symbol check also accepts: current names sharing a retired prefix. */
function symbolCarveOut(id: string, line: string): boolean {
  if (id === 'vox-dei-old-crate') return /vox-dei-d|vox-dei-shim|crates[\\/]vox-dei|no[-_]vox[-_]dei[-_]import/.test(line);
  if (id === 'vox-ml-cli-standalone') return /vox-ml-cli-|crates[\\/]vox-ml-cli/.test(line);
  if (id === 'sync-recall-api') return line.includes('lookup_fact_by_key');
  return false;
}

/** Text lines of the page's `<main>` element, one per block element or code line. */
function mainTextLines(html: string): string[] {
  const main = html.match(/<main\b[^>]*>([\s\S]*)<\/main>/i)?.[1] ?? '';
  return main
    .replace(/<(script|style)\b[\s\S]*?<\/\1>/gi, '')
    .replace(/<br\s*\/?>|<\/(?:div|p|li|h[1-6]|tr|pre|code|dt|dd|blockquote|summary|figcaption)>/gi, '\n')
    .replace(/<[^>]+>/g, '')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#(?:39|x27);/gi, "'")
    .replace(/&amp;/g, '&')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
}

const SAMPLED_PAGES = [
  '/',
  '/tutorials/tut-actor-basics/',
  '/tutorials/tut-first-app/',
  '/tutorials/tut-first-vox-app-checkpoints/',
  '/tutorials/tut-getting-started/',
  '/tutorials/tut-ui-integration/',
  '/tutorials/tut-workflow-durability/',
  '/tutorials/use-a-react-component-from-vox/',
  '/reference/installation/',
  '/reference/ref-syntax/',
  '/reference/cli/',
  '/reference/stability/',
];

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

  test('llms.txt is published at site root', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/llms.txt', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    const body = await resp.text();
    expect(body.toLowerCase()).toMatch(/pre-1\.0|0\.6\.0|stability/);
  });

  test('hand-authored /.well-known/llms.txt is published', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/.well-known/llms.txt', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    const body = await resp.text();
    expect(body.toLowerCase()).toMatch(/pre-1\.0|0\.6\.0|stability/);
    expect(body).toContain('table');
    expect(body).not.toMatch(/@endpoint\(kind/);
  });

  test('/voxup installer is served as a shell script', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/voxup', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
    expect((await resp.text()).startsWith('#!')).toBe(true);
  });

  test('/voxup.ps1 installer is served', async ({ request }) => {
    const resp = await request.get(PRIMARY + '/voxup.ps1', { timeout: 15_000 });
    expect(resp.status()).toBe(200);
  });

  for (const file of LLMS_FILES) {
    test(`every voxlang.org URL in ${file} resolves (llms links)`, async ({ request }) => {
      const resp = await request.get(PRIMARY + file, { timeout: 15_000 });
      expect(resp.status()).toBe(200);
      const urls = extractLlmsUrls(await resp.text(), { baseUrl: PRIMARY });
      expect(urls.length).toBeGreaterThan(0);
      const result = await checkUrls(urls, async (url: string) => ({
        status: (await request.get(url, { timeout: 15_000 })).status(),
      }));
      expect(result.failures, `${file} lists URLs that do not resolve`).toEqual([]);
      expect(result.ok).toBe(true);
    });

    test(`no Internals page is listed in ${file}`, async ({ request }) => {
      expect(INTERNALS_ROUTES.size).toBeGreaterThan(0);
      const resp = await request.get(PRIMARY + file, { timeout: 15_000 });
      expect(resp.status()).toBe(200);
      const listed = extractLlmsUrls(await resp.text()).map((url: string) => new URL(url).pathname);
      expect(listed.filter((path: string) => INTERNALS_ROUTES.has(path))).toEqual([]);
    });
  }

  for (const route of SAMPLED_PAGES) {
    test(`no retired syntax on ${route}`, async ({ request }) => {
      const resp = await request.get(PRIMARY + route, { timeout: 15_000 });
      expect(resp.status()).toBe(200);
      const lines = mainTextLines(await resp.text());
      expect(lines.length).toBeGreaterThan(0);
      const hits: string[] = [];
      for (const line of lines) {
        if (CARVE_OUT.test(line)) continue;
        for (const symbol of RETIRED) {
          if (symbol.regex.test(line) && !symbolCarveOut(symbol.id, line)) hits.push(`${route} [${symbol.id}] ${line}`);
        }
      }
      expect(hits, `retired syntax taught on ${route}`).toEqual([]);
    });
  }

  test('archive redirect', async ({ request }) => {
    test.skip(IS_LOCAL, '_redirects is applied only by Cloudflare Pages');
    const resp = await request.get(PRIMARY + '/archive/p20-redirect-probe/', { maxRedirects: 0, timeout: 15_000 });
    expect(resp.status()).toBe(301);
    expect(new URL(resp.headers()['location'], PRIMARY).pathname).toBe('/retired/');
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
    // Starlight renders a search button (also opens via Ctrl-K). Wait for it.
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
