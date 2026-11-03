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
            // Desktop and Tablet (>= 50rem): Left sidebar present with pin toggle
            const pinBtn = page.locator('#vox-sidebar-pin-toggle');
            if (await pinBtn.isVisible()) {
              // 2. Collapse Left Sidebar
              await pinBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'collapsed');

              // Move mouse away and blur button so hover/focus overlay does not expand it
              await page.mouse.move(600, 600);
              await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());

              // Wait for 200ms width transition to settle to 56px (3.5rem)
              await page.waitForFunction(() => {
                const el = document.querySelector('.sidebar-pane');
                return el && window.getComputedStyle(el).width === '56px';
              }, { timeout: 3000 });

              await page.screenshot({
                path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-02-left-collapsed.png`),
                fullPage: false,
              });

              // 3. Hover Rail Expansion Overlay (on desktop with hover)
              if (vpName === 'desktop') {
                await page.locator('.sidebar-pane').hover();
                // Wait for hover transition to expand to 300px (18.75rem)
                await page.waitForFunction(() => {
                  const el = document.querySelector('.sidebar-pane');
                  return el && window.getComputedStyle(el).width === '300px';
                }, { timeout: 3000 });

                await page.screenshot({
                  path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-03-left-hover.png`),
                  fullPage: false,
                });
                await page.mouse.move(600, 600); // unhover

                // Wait for collapse back to 56px
                await page.waitForFunction(() => {
                  const el = document.querySelector('.sidebar-pane');
                  return el && window.getComputedStyle(el).width === '56px';
                }, { timeout: 3000 });
              }

              // 4. Hide Right Sidebar (if TOC present at >= 72rem)
              if (vpName === 'desktop') {
                const hideBtn = page.locator('#vox-toc-hide-btn');
                if (await hideBtn.isVisible()) {
                  await hideBtn.click();
                  await expect(page.locator('html')).toHaveAttribute('data-toc-state', 'hidden');

                  const restoreBtn = page.locator('#vox-toc-restore-btn');
                  await expect(restoreBtn).toBeVisible();

                  await page.screenshot({
                    path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-04-both-collapsed.png`),
                    fullPage: false,
                  });

                  // 5. Restore Right Sidebar
                  await restoreBtn.click();
                  await expect(page.locator('html')).toHaveAttribute('data-toc-state', 'visible');
                }
              }

              // 6. Completely Hide Left Sidebar and Restore via themed pill button
              // First pin it back so #vox-sidebar-hide-btn is directly visible
              await pinBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'pinned');
              const leftHideBtn = page.locator('#vox-sidebar-hide-btn');
              await expect(leftHideBtn).toBeVisible();
              await leftHideBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'hidden');

              const leftRestoreBtn = page.locator('#vox-sidebar-restore-btn');
              await expect(leftRestoreBtn).toBeVisible();
              await leftRestoreBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'pinned');

              // Collapse again for persistence check
              await pinBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'collapsed');

              // 7. Persistence across page reload
              await page.reload({ waitUntil: 'domcontentloaded' });
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'collapsed');
            }
          }
        });
      }
    }
  }
});
