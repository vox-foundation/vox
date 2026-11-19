import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const THEMES = ['dark', 'light'] as const;

async function auditRowAlignments(page: Page, routeLabel: string): Promise<string[]> {
  return page.evaluate((label) => {
    const failures: string[] = [];

    const checkHorizontalButtonGroup = (groupName: string, selector: string) => {
      for (const container of Array.from(document.querySelectorAll(selector))) {
        const items = Array.from(container.querySelectorAll('a, button')).filter((el) => {
          const rect = el.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0;
        });
        for (let i = 0; i < items.length; i++) {
          for (let j = i + 1; j < items.length; j++) {
            const a = items[i].getBoundingClientRect();
            const b = items[j].getBoundingClientRect();
            // Same visual row when tops are within 20px of each other
            if (Math.abs(a.top - b.top) < 20) {
              const topDelta = Math.abs(a.top - b.top);
              const heightDelta = Math.abs(a.height - b.height);
              if (topDelta > 2) {
                failures.push(
                  `${label} [${groupName}] button top mismatch (${topDelta.toFixed(1)}px): "${items[i].textContent?.trim()}" vs "${items[j].textContent?.trim()}"`,
                );
              }
              if (heightDelta > 2) {
                failures.push(
                  `${label} [${groupName}] button height mismatch (${heightDelta.toFixed(1)}px): "${items[i].textContent?.trim()}" vs "${items[j].textContent?.trim()}"`,
                );
              }
            }
          }
        }
      }
    };

    const checkCardGridRows = (gridName: string, selector: string) => {
      for (const grid of Array.from(document.querySelectorAll(selector))) {
        const cards = Array.from(grid.children).filter((el) => {
          const rect = el.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0;
        });
        for (let i = 0; i < cards.length; i++) {
          for (let j = i + 1; j < cards.length; j++) {
            const a = cards[i].getBoundingClientRect();
            const b = cards[j].getBoundingClientRect();
            // Same visual grid row when tops are within 40px
            if (Math.abs(a.top - b.top) < 40) {
              const topDelta = Math.abs(a.top - b.top);
              const heightDelta = Math.abs(a.height - b.height);
              if (topDelta > 2) {
                failures.push(`${label} [${gridName}] row card top mismatch (${topDelta.toFixed(1)}px)`);
              }
              if (heightDelta > 2) {
                failures.push(
                  `${label} [${gridName}] row card height mismatch (${heightDelta.toFixed(1)}px): ${a.height.toFixed(1)}px vs ${b.height.toFixed(1)}px`,
                );
              }
            }
          }
        }
      }
    };

    checkHorizontalButtonGroup('hero-actions', '.vox-home-actions');
    checkHorizontalButtonGroup('playground-controls', '.playground-header-controls');
    checkHorizontalButtonGroup('playground-target-tabs', '.target-tabs-bar');
    checkHorizontalButtonGroup('playground-secondary-tabs', '.secondary-tabs-bar');
    checkHorizontalButtonGroup('carousel-tabs', '.carousel-tabs');
    checkHorizontalButtonGroup('carousel-controls', '.carousel-controls');

    checkCardGridRows('progressive-tiers', '.vox-progressive-tiers');
    checkCardGridRows('showcase-grid', '.vox-showcase-grid');
    checkCardGridRows('starlight-card-grid', '.sl-markdown-content .card-grid');

    return failures;
  }, routeLabel);
}

test.describe('Phase 533.1 — voxlang.org Carousel, Row Alignments & Dual-Theme Contrast Gate', () => {
  for (const theme of THEMES) {
    test(`frontpage (/), interactive GUI carousel, and key tutorials pass CV, WCAG >= 4.5:1 contrast & <= 2px alignment in ${theme} theme`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto('/', { waitUntil: 'networkidle' });

      await page.evaluate((t) => {
        document.documentElement.dataset.theme = t;
      }, theme);

      // 1. Verify Hero, Onboarding Tiers, Playground, and Early Interactive GUI Carousel
      const hero = page.getByTestId('vox-suite-portal-hero');
      const tiers = page.getByTestId('progressive-onboarding-tiers');
      const playground = page.getByTestId('vox-compiler-playground');
      const carousel = page.getByTestId('axis-tour-carousel');
      const whyVox = page.locator('#why-vox');

      await expect(hero).toBeVisible();
      await expect(tiers).toBeVisible();
      await expect(playground).toBeVisible();
      await expect(carousel).toBeVisible();
      await expect(whyVox).toBeVisible();

      // GUI Carousel must appear earlier than #why-vox
      const carouselBox = await carousel.boundingBox();
      const whyBox = await whyVox.boundingBox();
      expect(carouselBox, 'Carousel bounding box missing').not.toBeNull();
      expect(whyBox, '#why-vox bounding box missing').not.toBeNull();
      expect(carouselBox!.y).toBeLessThan(whyBox!.y);

      // 2. Test Interactive Carousel Navigation (Next, Tab Jump, Theme Toggle, Prev)
      const counter = carousel.locator('[data-carousel-counter]');
      const carouselImg = carousel.locator('[data-carousel-img]');
      await expect(counter).toHaveText('1 / 9');
      await expect(carouselImg).toHaveAttribute('src', /urbs-civilization\.webp$/);

      await carousel.locator('[data-carousel-next]').click();
      await expect(counter).toHaveText('2 / 9');
      await expect(carouselImg).toHaveAttribute('src', /urbs-imperium-inspector\.webp$/);

      await carousel.locator('[data-slide-idx="4"]').click();
      await expect(counter).toHaveText('5 / 9');
      await expect(carouselImg).toHaveAttribute('src', /research-arcane\.webp$/);

      await carousel.locator('[data-carousel-theme-toggle]').click();
      await expect(carouselImg).toHaveAttribute('src', /research-slate\.webp$/);
      await carouselImg.scrollIntoViewIfNeeded();
      await expect.poll(() => carouselImg.evaluate((el: HTMLImageElement) => el.naturalWidth)).toBeGreaterThan(0);

      await carousel.locator('[data-carousel-theme-toggle]').click();
      await expect(carouselImg).toHaveAttribute('src', /research-arcane\.webp$/);

      await carousel.locator('[data-carousel-prev]').click();
      await expect(counter).toHaveText('4 / 9');
      await expect(carouselImg).toHaveAttribute('src', /harness\.webp$/);

      await carousel.locator('[data-slide-idx="0"]').click();
      await expect(counter).toHaveText('1 / 9');

      // 3. Scroll through all images to trigger lazy loading and verify 100% decode
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

      // 4. Programmatic WCAG 2.1 AA contrast verification (>= 4.5:1)
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
          '.vox-tier-btn-cyan-bottom',
          '.vox-tier-btn-amber',
          '.vox-tier-btn-emerald',
          '.carousel-tab',
          '.carousel-theme-btn',
          '.carousel-nav-btn',
          '.carousel-counter',
          '.carousel-brand-label',
          '.carousel-badge',
          '.carousel-title',
          '.carousel-desc',
          '.carousel-link',
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

      // 5. Button alignment, zero label clipping & equal row card height check on / (1440x900)
      const homeAlignFailures = await auditRowAlignments(page, `/ (${theme}, 1440x900)`);
      expect(homeAlignFailures, `Alignment failures on / (${theme}):\n${homeAlignFailures.join('\n')}`).toEqual([]);

      const assertNoClippedControls = async (vpLabel: string) => {
        const clipped = await page.evaluate((ctx) => {
          const selectors = [
            '.vox-home-actions a',
            '.playground-tab',
            '.target-tab',
            '.secondary-tab',
            '.carousel-tab',
            '.carousel-theme-btn',
            '.carousel-nav-btn',
          ];
          const hits: string[] = [];
          for (const sel of selectors) {
            for (const el of Array.from(document.querySelectorAll(sel))) {
              const rect = el.getBoundingClientRect();
              if (rect.width === 0 || rect.height === 0) continue;
              if (el.scrollWidth > el.clientWidth + 2) {
                hits.push(
                  `${ctx} [${sel}] "${(el.textContent || '').trim()}" scrollWidth=${el.scrollWidth} > clientWidth=${el.clientWidth}`,
                );
              }
            }
          }
          return hits;
        }, vpLabel);
        expect(clipped, `Clipped button/tab labels at ${vpLabel}`).toEqual([]);
      };

      const assertHeroAndTiersSingleRow = async (vpLabel: string) => {
        const rowFailures = await page.evaluate((ctx) => {
          const errs: string[] = [];
          const heroBtns = Array.from(document.querySelectorAll('.vox-home-actions a')).map((el) =>
            el.getBoundingClientRect(),
          );
          if (heroBtns.length >= 2) {
            const firstTop = heroBtns[0].top;
            for (const r of heroBtns) {
              if (Math.abs(r.top - firstTop) > 2) {
                errs.push(`${ctx} hero CTAs wrapped across rows (top ${firstTop.toFixed(1)} vs ${r.top.toFixed(1)})`);
                break;
              }
            }
          }
          const tiers = Array.from(document.querySelectorAll('.vox-progressive-tiers .vox-tier-card')).map((el) =>
            el.getBoundingClientRect(),
          );
          if (tiers.length === 3) {
            const top0 = tiers[0].top;
            const h0 = tiers[0].height;
            for (const r of tiers) {
              if (Math.abs(r.top - top0) > 2) {
                errs.push(`${ctx} tier cards wrapped into multiple rows (${top0.toFixed(1)} vs ${r.top.toFixed(1)})`);
              }
              if (Math.abs(r.height - h0) > 2) {
                errs.push(`${ctx} tier card height mismatch (${h0.toFixed(1)} vs ${r.height.toFixed(1)})`);
              }
            }
          }
          return errs;
        }, vpLabel);
        expect(rowFailures, `Hero/tier single-row check failed at ${vpLabel}`).toEqual([]);
      };

      await assertNoClippedControls(`${theme} 1440x900`);
      await assertHeroAndTiersSingleRow(`${theme} 1440x900`);

      // Anti-redundancy gate: prevent "one .vox file" slogan repetition across /
      const sloganRepeatCount = await page.evaluate(() => {
        const text = document.querySelector('main')?.innerText ?? '';
        const matches = text.match(/one\s+\.vox\s+(?:file|module)|single-file\s+full-stack|in\s+one\s+\.vox/gi);
        return matches ? matches.length : 0;
      });
      expect(
        sloganRepeatCount,
        `Too many repetitions of single-file slogan on / (${sloganRepeatCount})`,
      ).toBeLessThanOrEqual(2);

      const outDir = join(process.cwd(), 'test-results/review-bundle');
      mkdirSync(outDir, { recursive: true });

      await page.screenshot({
        path: join(outDir, `suite-portal-hero-${theme}.png`),
        fullPage: false,
      });

      await playground.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: join(outDir, `suite-portal-playground-${theme}.png`),
        fullPage: false,
      });

      await carousel.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: join(outDir, `suite-portal-carousel-${theme}.png`),
        fullPage: false,
      });

      // 5b. Check 1280x800 laptop viewport (single-row CTAs, 3-column tiers, zero clipping) & capture AFTER shots
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.evaluate(() => window.scrollTo(0, 0));
      const laptopAlignFailures = await auditRowAlignments(page, `/ (${theme}, 1280x800)`);
      expect(laptopAlignFailures, `Alignment failures on / 1280x800 (${theme}):\n${laptopAlignFailures.join('\n')}`).toEqual([]);
      await assertNoClippedControls(`${theme} 1280x800`);
      await assertHeroAndTiersSingleRow(`${theme} 1280x800`);
      await page.screenshot({
        path: join(outDir, `after-hero-1280-${theme}.png`),
        fullPage: false,
      });
      await tiers.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: join(outDir, `after-tiers-1280-${theme}.png`),
        fullPage: false,
      });
      await page.setViewportSize({ width: 1440, height: 900 });

      // 6. CV & Alignment Flywheel on key workbench & tutorial pages
      for (const route of [
        '/how-to/how-to-axis-workbench/',
        '/tutorials/tut-deep-research-and-scientia/',
        '/tutorials/tut-knowledgebase-and-openwiki/',
      ]) {
        await page.goto(route, { waitUntil: 'networkidle' });
        await page.evaluate((t) => {
          document.documentElement.dataset.theme = t;
        }, theme);
        const routeBroken = await page.evaluate(async () => {
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
        expect(routeBroken, `Broken images on ${route} (${theme})`).toEqual([]);

        const routeAlignFailures = await auditRowAlignments(page, `${route} (${theme})`);
        expect(
          routeAlignFailures,
          `Alignment failures on ${route} (${theme}):\n${routeAlignFailures.join('\n')}`,
        ).toEqual([]);

        if (route === '/tutorials/tut-knowledgebase-and-openwiki/') {
          const kbShowcase = page.getByTestId('tut-kb-visual-showcase');
          await kbShowcase.scrollIntoViewIfNeeded();
          await page.screenshot({
            path: join(outDir, `tut-kb-${theme}.png`),
            fullPage: false,
          });
        }
      }
    });
  }
});
