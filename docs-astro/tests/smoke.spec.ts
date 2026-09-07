import { test, expect } from '@playwright/test';

test.setTimeout(60_000);

const PRIMARY = process.env.BASE_URL ?? 'https://voxlang.org';

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
