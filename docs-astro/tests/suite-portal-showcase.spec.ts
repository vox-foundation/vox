import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const THEMES = ['dark', 'light'] as const;

test.describe('Phase 402 — voxlang.org Suite Portal, Vox Axis Showcase & Dual-Theme Contrast Gate', () => {
  for (const theme of THEMES) {
    test(`frontpage (/) & Axis workbench guide render all images and pass WCAG >= 4.5:1 contrast in ${theme} theme`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto('/', { waitUntil: 'networkidle' });

      await page.evaluate((t) => {
        document.documentElement.dataset.theme = t;
      }, theme);

      // 1. Verify hero, Axis animated tour, 4-screen showcase grid, and dual-theme showcase exist
      await expect(page.getByTestId('vox-suite-portal-hero')).toBeVisible();
      await expect(page.getByTestId('progressive-onboarding-tiers')).toBeVisible();
      await expect(page.getByTestId('axis-animated-showcase')).toBeVisible();
      await expect(page.getByTestId('axis-screens-showcase')).toBeVisible();
      await expect(page.getByTestId('axis-dual-theme-showcase')).toBeVisible();

      // 2. Scroll through all images to trigger lazy loading and verify 100% decode (naturalWidth > 0)
      const brokenImages = await page.evaluate(async () => {
        const imgs = Array.from(document.querySelectorAll('main img')) as HTMLImageElement[];
        for (const img of imgs) {
          img.scrollIntoView({ behavior: 'instant', block: 'center' });
          if (!img.complete) {
            await new Promise<void>((res) => {
              img.addEventListener('load', () => res(), { once: true });
              img.addEventListener('error', () => res(), { once: true });
              setTimeout(() => res(), 3000);
            });
          }
        }
        window.scrollTo(0, 0);
        return imgs
          .filter((img) => !img.complete || img.naturalWidth === 0)
          .map((img) => img.getAttribute('src') || img.alt);
      });
      expect(brokenImages, `Broken images on / (${theme}): ${brokenImages.join(', ')}`).toEqual([]);

      // 3. Programmatic WCAG 2.1 AA contrast verification (>= 4.5:1) across portal typography
      const contrastFailures = await page.evaluate(() => {
        function parseRgb(str: string): [number, number, number, number] {
          const m = str.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
          if (!m) return [0, 0, 0, 1];
          return [Number(m[1]), Number(m[2]), Number(m[3]), m[4] !== undefined ? Number(m[4]) : 1];
        }
        function effectiveBg(el: Element | null): [number, number, number] {
          let cur: Element | null = el;
          while (cur) {
            const bg = window.getComputedStyle(cur).backgroundColor;
            const [r, g, b, a] = parseRgb(bg);
            if (a >= 0.85) return [r, g, b];
            cur = cur.parentElement;
          }
          const rootTheme = document.documentElement.dataset.theme;
          return rootTheme === 'light' ? [248, 250, 252] : [11, 15, 23];
        }
        function relLum([r, g, b]: [number, number, number]): number {
          const chan = (c: number) => {
            const s = c / 255;
            return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
          };
          return 0.2126 * chan(r) + 0.7152 * chan(g) + 0.0722 * chan(b);
        }
        function ratio(fg: [number, number, number], bg: [number, number, number]): number {
          const l1 = relLum(fg);
          const l2 = relLum(bg);
          return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
        }

        const selectors = [
          '.vox-home-kicker',
          '.vox-home-title',
          '.vox-home-lead',
          '.vox-home-sublead',
          '.vox-hero-btn-primary',
          '.vox-hero-btn-secondary',
          '.vox-hero-btn-gold',
          '.vox-hero-btn-ghost',
          '.vox-tier-title',
          '.vox-tier-body',
          '.vox-showcase-badge',
          '.vox-showcase-title',
          '.vox-showcase-desc',
          '.vox-media-caption',
          '.vox-details-summary',
        ];
        const failures: string[] = [];
        for (const sel of selectors) {
          for (const el of Array.from(document.querySelectorAll(sel))) {
            const cs = window.getComputedStyle(el);
            const [fr, fg, fb] = parseRgb(cs.color);
            const bg = effectiveBg(el);
            const cr = ratio([fr, fg, fb], bg);
            if (cr < 4.5) {
              failures.push(`${sel} ("${(el.textContent || '').trim().slice(0, 32)}") ratio=${cr.toFixed(2)}`);
            }
          }
        }
        return failures;
      });
      expect(contrastFailures, `WCAG contrast failures in ${theme} theme`).toEqual([]);

      // 4. Capture verification screenshots for both Hero and Vox Axis Showcase section
      const outDir = join(process.cwd(), 'test-results/review-bundle');
      mkdirSync(outDir, { recursive: true });

      await page.screenshot({
        path: join(outDir, `suite-portal-hero-${theme}.png`),
        fullPage: false,
      });

      const axisSection = page.getByTestId('axis-screens-showcase');
      await axisSection.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: join(outDir, `suite-portal-axis-showcase-${theme}.png`),
        fullPage: false,
      });

      const dualThemeSection = page.getByTestId('axis-dual-theme-showcase');
      await dualThemeSection.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: join(outDir, `suite-portal-dual-theme-${theme}.png`),
        fullPage: false,
      });
    });
  }
});
