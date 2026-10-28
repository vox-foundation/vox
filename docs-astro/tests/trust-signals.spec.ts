import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const VIEWPORTS = {
  desktop: { width: 1280, height: 800 },
  tablet: { width: 800, height: 1024 },
  phone: { width: 375, height: 667 },
};

const THEMES = ['dark', 'light'] as const;

const TEST_PAGES = [
  {
    name: 'verified',
    path: '/agents/',
    expectedState: 'verified',
    expectedText: 'Verified: Accurate against commit',
    hasCommitLink: true,
  },
  {
    name: 'adr-009-verified',
    path: '/adr/009-populi-hosted-baas/',
    expectedState: 'verified',
    expectedText: 'Verified: Accurate against commit',
    hasCommitLink: true,
  },
  {
    name: 'adr-001-verified',
    path: '/adr/001-burn-backend-selection/',
    expectedState: 'verified',
    expectedText: 'Verified: Accurate against commit',
    hasCommitLink: true,
  },
  {
    name: 'landing',
    path: '/',
    expectedState: null,
    expectedText: null,
    hasCommitLink: false,
  },
];

test.describe('TRUST-01 & TRUST-04 Trust Signals and Sidebar Badges', () => {
  for (const [vpName, vpSize] of Object.entries(VIEWPORTS)) {
    for (const theme of THEMES) {
      for (const p of TEST_PAGES) {
        test(`${p.name} - ${vpName} - ${theme} banner & layout`, async ({ page }) => {
          await page.setViewportSize(vpSize);
          await page.goto(p.path, { waitUntil: 'domcontentloaded' });

          // Apply theme
          await page.evaluate((t) => {
            document.documentElement.dataset.theme = t;
          }, theme);

          const bundleDir = join(process.cwd(), 'test-results/review-bundle');
          mkdirSync(bundleDir, { recursive: true });

          // 1. Viewport Overflow Check
          const hasHorizontalOverflow = await page.evaluate(() => {
            return document.documentElement.scrollWidth > document.documentElement.clientWidth;
          });
          expect(hasHorizontalOverflow).toBeFalsy();

          // 2. Banner Assertions
          const banner = page.locator('.vox-trust-banner');
          if (p.expectedState) {
            await expect(banner).toBeVisible();
            await expect(banner).toHaveClass(new RegExp(`vox-trust-${p.expectedState}`));
            await expect(banner).toContainText(p.expectedText!);

            if (p.hasCommitLink) {
              const link = banner.locator('.vox-trust-link');
              await expect(link).toBeVisible();
              await expect(link).toHaveAttribute('href', /github\.com\/vox-foundation\/vox\/commit\//);
            }
          } else {
            // Landing page must NOT render trust banner
            await expect(banner).toHaveCount(0);
          }

          // 3. Desktop Sidebar Badges Assertion
          if (vpName === 'desktop' && p.name !== 'landing') {
            const badges = page.locator('nav.sidebar .sl-badge');
            const count = await badges.count();
            expect(count).toBeGreaterThan(0);
          }

          // 4. Visual Review Screenshot Capture
          await page.screenshot({
            path: join(bundleDir, `trust-${p.name}-${vpName}-${theme}.png`),
            fullPage: false,
          });
        });
      }
    }
  }
});
