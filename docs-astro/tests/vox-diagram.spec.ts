import { test, expect } from '@playwright/test';
import { figureRoutes } from './lib/diagrams';

/*
 * Browser checks for the inline VoxDiagram figures (DOCS-04). Pages are discovered from
 * `dist/`, so every page that gains a figure is covered without editing this file.
 */
const ROUTES = figureRoutes();

test('figure pages are discovered', async ({ page }) => {
  expect(ROUTES, 'dist/ has no page with a vd-figure; run pnpm build').not.toHaveLength(0);
  expect(ROUTES).toContain('/reference/vox-diagram/');
  await page.goto('/reference/vox-diagram/', { waitUntil: 'domcontentloaded' });
  expect(await page.locator('svg.vd-diagram:not(:has(.vd-lifeline))').count(), 'graph figures').toBeGreaterThan(0);
  expect(await page.locator('svg.vd-diagram:has(.vd-lifeline)').count(), 'sequence figures').toBeGreaterThan(0);
});

for (const route of ROUTES) {
  test(`ids: unique and every reference resolves (${route})`, async ({ page }) => {
    await page.goto(route, { waitUntil: 'domcontentloaded' });
    const problems = await page.evaluate(() => {
      const out: string[] = [];
      const seen = new Set<string>();
      for (const el of document.querySelectorAll('[id]')) {
        if (seen.has(el.id)) out.push(`duplicate id ${el.id}`);
        seen.add(el.id);
      }
      for (const el of document.querySelectorAll('[aria-labelledby]')) {
        for (const token of (el.getAttribute('aria-labelledby') ?? '').split(/\s+/).filter(Boolean)) {
          if (!document.getElementById(token)) out.push(`aria-labelledby ${token} does not resolve`);
        }
      }
      for (const el of document.querySelectorAll('[marker-end]')) {
        const ref = /url\(#([^)]+)\)/.exec(el.getAttribute('marker-end') ?? '');
        if (ref && !document.getElementById(ref[1])) out.push(`marker-end ${ref[1]} does not resolve`);
      }
      return out;
    });
    expect(problems).toEqual([]);
  });
}
