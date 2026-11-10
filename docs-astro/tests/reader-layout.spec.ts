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
              const measureContent = () =>
                page.evaluate(() => {
                  const container = document.querySelector('main .content-panel:last-of-type > .sl-container');
                  const firstP = document.querySelector('.sl-markdown-content > p');
                  const rightContainer = document.querySelector('aside.right-sidebar-container');
                  const cr = container?.getBoundingClientRect();
                  const pr = firstP?.getBoundingClientRect();
                  const rr = rightContainer?.getBoundingClientRect();
                  return {
                    x: cr ? Math.round(cr.x) : 0,
                    right: cr ? Math.round(cr.right) : 0,
                    width: cr ? Math.round(cr.width) : 0,
                    pWidth: pr ? Math.round(pr.width) : 0,
                    rightSidebarWidth: rr ? Math.round(rr.width) : 0,
                  };
                });

              const bothOpen = await measureContent();

              await pinBtn.click();
              await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'hidden');
              await expect(pinBtn).toBeVisible();

              await page.waitForFunction(() => {
                const el = document.querySelector('.sidebar-pane');
                return el && window.getComputedStyle(el).width === '0px';
              }, { timeout: 3000 });

              const leftHidden = await measureContent();
              expect(leftHidden.width).toBeGreaterThan(bothOpen.width);
              expect(leftHidden.pWidth).toBeGreaterThan(bothOpen.pWidth);
              expect(leftHidden.x).toBeLessThan(bothOpen.x);
              if (vpName === 'desktop') {
                expect(leftHidden.right).toBe(bothOpen.right);
              } else {
                expect(Math.abs(leftHidden.right - bothOpen.right)).toBeLessThanOrEqual(16);
              }

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

                  const bothHidden = await measureContent();
                  expect(bothHidden.rightSidebarWidth).toBe(0);
                  expect(bothHidden.width).toBeGreaterThan(leftHidden.width);
                  expect(bothHidden.pWidth).toBeGreaterThan(leftHidden.pWidth);
                  expect(bothHidden.x).toBe(leftHidden.x);
                  expect(bothHidden.right).toBeGreaterThan(leftHidden.right);

                  await page.screenshot({
                    path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-03-both-hidden.png`),
                    fullPage: false,
                  });

                  // Reopen left sidebar while right TOC remains hidden -> verify right-only collapse recovery
                  await pinBtn.click();
                  await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'pinned');
                  const rightHidden = await measureContent();
                  expect(rightHidden.rightSidebarWidth).toBe(0);
                  expect(rightHidden.width).toBeGreaterThan(bothOpen.width);
                  expect(rightHidden.pWidth).toBeGreaterThan(bothOpen.pWidth);
                  expect(rightHidden.x).toBe(bothOpen.x);
                  expect(rightHidden.right).toBeGreaterThan(bothOpen.right);
                  expect(bothHidden.width).toBeGreaterThan(rightHidden.width);

                  await page.screenshot({
                    path: join(bundleDir, `${pageInfo.name}-${vpName}-${theme}-04-right-hidden.png`),
                    fullPage: false,
                  });

                  await tocEdge.click();
                  await expect(page.locator('html')).toHaveAttribute('data-toc-state', 'visible');
                  await pinBtn.click();
                  await expect(page.locator('html')).toHaveAttribute('data-sidebar-state', 'hidden');
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

  test('desktop header theme selector and GitHub link are clickable and auto tracks system color scheme', async ({ page }) => {
    await page.setViewportSize(VIEWPORTS.desktop);
    await page.emulateMedia({ colorScheme: 'dark' });
    await page.goto('/', { waitUntil: 'domcontentloaded' });

    // Default auto should resolve to dark when OS prefers-color-scheme is dark
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

    // Emulate system switching to light while in auto mode
    await page.emulateMedia({ colorScheme: 'light' });
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

    // Switch back to dark via OS media query
    await page.emulateMedia({ colorScheme: 'dark' });
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

    // Verify theme select and GitHub link in top-right header are not occluded by right-sidebar
    const themeSelect = page.locator('header.header starlight-theme-select select');
    const githubLink = page.locator('header.header .social-icons a[href*="github.com"]');
    await expect(themeSelect).toBeVisible();
    await expect(githubLink).toBeVisible();

    await themeSelect.selectOption('light');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

    await themeSelect.selectOption('dark');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

    await themeSelect.selectOption('auto');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  });
});
