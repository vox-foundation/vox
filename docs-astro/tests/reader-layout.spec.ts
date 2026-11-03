import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const VIEWPORTS = {
  desktop: { width: 1280, height: 800 },
  tablet: { width: 800, height: 1024 },
  phone: { width: 375, height: 667 },
};

const THEMES = ['dark', 'light'] as const;

const PAGES = [
  { name: 'docs', path: '/tutorials/tut-getting-started/' },
  { name: 'landing', path: '/' },
];

test.describe('SITE-01..05 Reader Layout Matrix', () => {
  for (const [vpName, vpSize] of Object.entries(VIEWPORTS)) {
    for (const theme of THEMES) {
      for (const pageInfo of PAGES) {
        test(`${pageInfo.name} - ${vpName} - ${theme} layout and transitions`, async ({ page }) => {
          await page.setViewportSize(vpSize);
          await page.goto(pageInfo.path, { waitUntil: 'domcontentloaded' });

          // Apply theme
          await page.evaluate((t) => {
            document.documentElement.dataset.theme = t;
          }, theme);

          const bundleDir = join(process.cwd(), 'test-results/review-bundle');
          mkdirSync(bundleDir, { recursive: true });

          // 1. Initial State: Verify no horizontal overflow
          const hasHorizontalOverflow = await page.evaluate(() => {
            return document.documentElement.scrollWidth > document.documentElement.clientWidth;
          });
          expect(hasHorizontalOverflow).toBeFalsy();

          await page.screenshot({
            path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-01-default.png`),
            fullPage: false,
          });

          if (vpName === 'phone') {
            // Mobile checks: hamburger menu is used; desktop rail pin toggle hidden
            const pinBtn = page.locator('#vox-sidebar-pin-toggle');
            await expect(pinBtn).not.toBeVisible();

            await page.screenshot({
              path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-mobile-view.png`),
              fullPage: false,
            });
          } else {
            // Desktop and tablet: one left chevron closes and reopens the navigation.
            const pinBtn = page.locator('#vox-sidebar-pin-toggle');
            if (await pinBtn.isVisible()) {
              await pinBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'hidden');
              await expect(pinBtn).toBeVisible();

              await page.waitForFunction(() => {
                const el = document.querySelector('.sidebar-pane');
                return el && window.getComputedStyle(el).width === '0px';
              }, { timeout: 3000 });

              await page.screenshot({
                path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-02-left-hidden.png`),
                fullPage: false,
              });

              if (vpName === 'desktop') {
                const tocEdge = page.locator('#vox-toc-edge');
                if (await tocEdge.isVisible()) {
                  await tocEdge.click();
                  await expect(page.locator('html')).toHaveAttribute('data-toc-state', 'hidden');
                  await expect(tocEdge).toBeVisible();

                  await page.screenshot({
                    path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-03-both-hidden.png`),
                    fullPage: false,
                  });

                  await tocEdge.click();
                  await expect(page.locator('html')).toHaveAttribute('data-toc-state', 'visible');
                }
              }

              await pinBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'pinned');

              await pinBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'hidden');
              await page.reload({ waitUntil: 'domcontentloaded' });
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'hidden');
            }
          }
        });
      }
    }
  }
});
