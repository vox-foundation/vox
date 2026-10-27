import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { contrastRatio, figureRoutes, FONT_STACKS, type StackName } from './lib/diagrams';

/*
 * Browser checks for the inline VoxDiagram figures (DOCS-04). Pages are discovered from
 * `dist/`, so every page that gains a figure is covered without editing this file.
 * Helpers passed to `page.evaluate` are defined inside the evaluated function (Playwright
 * serialises only that function's source).
 */
const ROUTES = figureRoutes();
const REFERENCE = '/reference/vox-diagram/';
const BUNDLE = join(process.cwd(), 'test-results/review-bundle/vox-diagram');
const THEMES = ['light', 'dark'] as const;

async function open(page: Page, route: string, theme?: (typeof THEMES)[number]): Promise<void> {
  await page.goto(route, { waitUntil: 'domcontentloaded' });
  if (theme) await page.evaluate((t) => { document.documentElement.dataset.theme = t; }, theme);
  await page.evaluate(() => document.fonts.ready);
}

async function useStack(page: Page, stack: StackName): Promise<void> {
  const font = FONT_STACKS[stack];
  if (font) await page.addStyleTag({ content: `.vd-figure { --vd-font: ${font} !important; }` });
  await page.evaluate(() => document.fonts.ready);
}

test('figure pages are discovered', async ({ page }) => {
  expect(ROUTES, 'dist/ has no page with a vd-figure; run pnpm build').not.toHaveLength(0);
  expect(ROUTES).toContain(REFERENCE);
  await page.goto(REFERENCE, { waitUntil: 'domcontentloaded' });
  expect(await page.locator('svg.vd-diagram:not(:has(.vd-lifeline))').count(), 'graph figures').toBeGreaterThan(0);
  expect(await page.locator('svg.vd-diagram:has(.vd-lifeline)').count(), 'sequence figures').toBeGreaterThan(0);
});

test('fonts: the wide stack is measurably wider', async ({ page }) => {
  await open(page, REFERENCE);
  const widths = await page.evaluate((stacks) => {
    const probe = 'The quick brown fox jumps over the lazy dog 0123456789 Illinois';
    const measure = (family: string): number => {
      const span = document.createElement('span');
      span.style.cssText = `position:absolute;white-space:nowrap;font-size:14px;font-family:${family}`;
      span.textContent = probe;
      document.body.append(span);
      const w = span.getBoundingClientRect().width;
      span.remove();
      return w;
    };
    return { narrow: measure(stacks.narrow), wide: measure(stacks.wide) };
  }, { narrow: FONT_STACKS.narrow, wide: FONT_STACKS.wide });
  console.log(`font probe: narrow ${widths.narrow.toFixed(1)} px, wide ${widths.wide.toFixed(1)} px, ratio ${(widths.wide / widths.narrow).toFixed(3)}`);
  expect(widths.wide / widths.narrow, 'wide and narrow stacks resolved to the same font').toBeGreaterThanOrEqual(1.05);
});

for (const route of ROUTES) {
  test(`ids: unique and every reference resolves (${route})`, async ({ page }) => {
    await open(page, route);
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

  for (const stack of Object.keys(FONT_STACKS) as StackName[]) {
    test(`fits: ${stack} stack (${route})`, async ({ page }) => {
      await open(page, route);
      await useStack(page, stack);
      const { total, checked, bad, fill } = await page.evaluate(() => {
        const EPS = 0.5;
        const texts = [...document.querySelectorAll<SVGTextElement>('svg.vd-diagram text')];
        const bad: string[] = [];
        let checked = 0;
        let fill = 0;
        for (const t of texts) {
          const name = (t.textContent ?? '').trim().slice(0, 40);
          let box: SVGGraphicsElement | null = null;
          if (t.classList.contains('vd-label')) box = t.parentElement?.querySelector<SVGGraphicsElement>('.vd-node-shape') ?? null;
          else {
            const prev = t.previousElementSibling;
            if (prev?.matches('.vd-edge-label-bg, .vd-container, .vd-note')) box = prev as SVGGraphicsElement;
          }
          if (!box) { bad.push(`unmapped text "${name}"`); continue; }
          checked += 1;
          const tb = t.getBBox();
          const bb = box.getBBox();
          if (box.tagName.toLowerCase() === 'polygon') {
            const cx = bb.x + bb.width / 2;
            const cy = bb.y + bb.height / 2;
            const corners = [[tb.x, tb.y], [tb.x + tb.width, tb.y], [tb.x, tb.y + tb.height], [tb.x + tb.width, tb.y + tb.height]];
            const worst = Math.max(...corners.map(([x, y]) => Math.abs(x - cx) / (bb.width / 2) + Math.abs(y - cy) / (bb.height / 2)));
            if (worst > 1 + 1e-9) bad.push(`diamond "${name}" corner reaches ${worst.toFixed(3)} of the half-extent`);
          } else {
            fill = Math.max(fill, tb.width / bb.width);
            const over = Math.max(bb.x - tb.x, tb.x + tb.width - (bb.x + bb.width), bb.y - tb.y, tb.y + tb.height - (bb.y + bb.height));
            if (over > EPS) bad.push(`"${name}" overflows its box by ${over.toFixed(1)} units (text ${tb.width.toFixed(1)}x${tb.height.toFixed(1)}, box ${bb.width.toFixed(1)}x${bb.height.toFixed(1)})`);
          }
        }
        return { total: texts.length, checked, bad, fill };
      });
      console.log(`fits ${stack} ${route}: ${checked}/${total} texts, widest text fills ${(fill * 100).toFixed(1)}% of its box`);
      expect(total, 'no svg text found').toBeGreaterThan(0);
      expect(bad).toEqual([]);
      expect(checked, 'every text must be mapped to a box').toBe(total);
    });
  }

  for (const theme of THEMES) {
    test(`contrast: ${theme} (${route})`, async ({ page }) => {
      await open(page, route, theme);
      const r = await measure(page);
      console.log(`contrast ${theme} ${route}: min text ${r.minText.toFixed(2)}, min stroke ${r.minStroke.toFixed(2)}`);
      expect(r.violations).toEqual([]);
    });
  }

  test(`forced colors stay legible (${route})`, async ({ page }) => {
    await page.emulateMedia({ forcedColors: 'active' });
    await open(page, route);
    const r = await measure(page);
    console.log(`forced colors ${route}: min text ${r.minText.toFixed(2)}, min stroke ${r.minStroke.toFixed(2)}`);
    expect(r.violations).toEqual([]);
  });

  for (const viewport of [{ width: 1280, height: 800 }, { width: 375, height: 667 }]) {
    test(`wide diagrams: ${viewport.width}px (${route})`, async ({ page }) => {
      await page.setViewportSize(viewport);
      await open(page, route);
      const figures = await page.evaluate(() => [...document.querySelectorAll<HTMLElement>('figure.vd-figure')].map((fig) => {
        const svg = fig.querySelector('svg')!;
        const label = document.getElementById((fig.getAttribute('aria-labelledby') ?? '').split(/\s+/)[0]);
        return {
          id: label?.id ?? '(none)',
          natural: parseFloat(fig.style.getPropertyValue('--vd-w')),
          rendered: svg.getBoundingClientRect().width,
          scrolls: fig.scrollWidth > fig.clientWidth + 1,
          clientWidth: fig.clientWidth,
          focusable: fig.getAttribute('tabindex') === '0',
          region: fig.getAttribute('role') === 'region',
          name: (label?.textContent ?? '').trim(),
        };
      }));
      const pageOverflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
      expect(figures.length, 'figures on page').toBeGreaterThan(0);
      for (const f of figures) {
        expect(f.natural, `${f.id} has a numeric --vd-w`).toBeGreaterThan(0);
        expect(f.rendered, `${f.id} wider than its viewBox`).toBeLessThanOrEqual(f.natural + 1);
        expect(f.rendered, `${f.id} below the 0.6 scale floor`).toBeGreaterThanOrEqual(f.natural * 0.6 - 1);
        expect(f.scrolls, `${f.id} wider than its figure must scroll inside it`).toBe(f.rendered > f.clientWidth + 1);
        // WR-03: only a figure that scrolls is a tab stop and a (named) region landmark.
        expect(f.focusable, `${f.id} is a tab stop exactly when it scrolls`).toBe(f.scrolls);
        expect(f.region, `${f.id} is a region landmark exactly when it scrolls`).toBe(f.scrolls);
        expect(f.name, `${f.id} has an accessible name`).not.toBe('');
        expect(f.name, `${f.id} keeps the writer's generic title`).not.toBe('Diagram');
      }
      const names = figures.map((f) => f.name);
      expect(new Set(names).size, `figure names repeat: ${names.join(' | ')}`).toBe(names.length);
      expect(pageOverflow, 'the page itself overflows horizontally').toBe(false);
      if (route === REFERENCE && viewport.width === 1280) {
        expect(figures.some((f) => f.scrolls), 'at least one reference figure (the wide sequence) scrolls at 1280px').toBe(true);
      }
    });
  }
}

test('contrast check rejects a pale palette', async ({ page }) => {
  await open(page, REFERENCE, 'light');
  expect((await measure(page)).violations, 'the real palette is clean').toEqual([]);
  await page.addStyleTag({ content: '.vd-figure { --vd-text: #ccc !important; --vd-edge: #eee !important; --vd-node-stroke: #eee !important; }' });
  const pale = await measure(page);
  expect(pale.violations.filter((v) => v.startsWith('text')), 'pale text must fail').not.toHaveLength(0);
  expect(pale.violations.filter((v) => v.startsWith('stroke')), 'pale strokes must fail').not.toHaveLength(0);
});

test('contrast check rejects strokes lost on the container fill', async ({ page }) => {
  await open(page, REFERENCE, 'dark');
  // Only the container fill moves, to the stroke colour: strokes over the page stay clean.
  await page.addStyleTag({ content: '.vd-figure { --vd-container-fill: var(--sl-color-gray-3) !important; }' });
  const lost = (await measure(page)).violations.filter((v) => v.startsWith('stroke'));
  expect(lost, 'strokes on the container fill must fail').not.toHaveLength(0);
});

test('review bundle: every reference figure in light, dark and forced colors', async ({ page }) => {
  mkdirSync(BUNDLE, { recursive: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  const modes = [...THEMES, 'forced'] as const;
  for (const mode of modes) {
    await page.emulateMedia({ forcedColors: mode === 'forced' ? 'active' : 'none' });
    await open(page, REFERENCE, mode === 'forced' ? 'light' : mode);
    const figures = page.locator('figure.vd-figure');
    const count = await figures.count();
    expect(count).toBeGreaterThan(0);
    for (let i = 0; i < count; i += 1) {
      const n = String(i + 1).padStart(2, '0');
      await figures.nth(i).screenshot({ path: join(BUNDLE, `${n}-${mode}.png`) });
    }
    // Natural size (no 0.6 floor, no scroll clipping): lifted over the page so the whole diagram is captured.
    await page.addStyleTag({ content: '.vd-figure.vd-natural { position: fixed; left: 0; top: 0; z-index: 9999; width: max-content; max-width: none; overflow: visible; margin: 0; padding: 8px; background: var(--sl-color-bg); } body :is(header, nav, aside, .sidebar-pane, .right-sidebar-container) { visibility: hidden !important; } .vd-figure.vd-natural > svg { width: var(--vd-w); max-width: none; min-width: 0; }' });
    for (let i = 0; i < count; i += 1) {
      const n = String(i + 1).padStart(2, '0');
      await figures.nth(i).evaluate((el) => el.classList.add('vd-natural'));
      await figures.nth(i).screenshot({ path: join(BUNDLE, `${n}-${mode}-natural.png`) });
      await figures.nth(i).evaluate((el) => el.classList.remove('vd-natural'));
    }
  }
});

/** Computed text and stroke colours of every figure, checked against WCAG thresholds (D-10). */
async function measure(page: Page): Promise<{ minText: number; minStroke: number; violations: string[] }> {
  const samples = await page.evaluate(() => {
    const rgb = (c: string): boolean => !/^rgba\(.*,\s*0\)$/.test(c) && c !== 'transparent';
    const backdrop = (el: Element): string => {
      for (let n: Element | null = el; n; n = n.parentElement) {
        const bg = getComputedStyle(n).backgroundColor;
        if (rgb(bg)) return bg;
      }
      return 'rgb(255, 255, 255)';
    };
    const cs = (el: Element, prop: 'fill' | 'stroke'): string => getComputedStyle(el)[prop];
    const out: { kind: 'text' | 'stroke'; what: string; fg: string; bg: string }[] = [];
    for (const t of document.querySelectorAll('svg.vd-diagram text')) {
      let box: Element | null = null;
      if (t.classList.contains('vd-label')) box = t.parentElement?.querySelector('.vd-node-shape') ?? null;
      else if (t.previousElementSibling?.matches('.vd-edge-label-bg, .vd-container, .vd-note')) box = t.previousElementSibling;
      out.push({ kind: 'text', what: `"${(t.textContent ?? '').trim().slice(0, 30)}"`, fg: cs(t, 'fill'), bg: box ? cs(box, 'fill') : 'unmapped' });
    }
    // A stroke is checked against every fill it can sit on: the page behind the figure, the fill
    // of each container its box overlaps, and for a shape its own fill (WR-06).
    const overlaps = (a: DOMRect, b: DOMRect): boolean =>
      a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
    for (const el of document.querySelectorAll<SVGGraphicsElement>('svg.vd-diagram :is(.vd-node-shape, .vd-edge, .vd-container, .vd-lifeline)')) {
      const what = el.getAttribute('class') ?? el.tagName;
      const box = el.getBBox();
      const fills = [backdrop(el.closest('figure') ?? el)];
      for (const c of el.ownerSVGElement?.querySelectorAll<SVGGraphicsElement>('.vd-container') ?? []) {
        if (overlaps(box, c.getBBox())) fills.push(cs(c, 'fill'));
      }
      if (el.matches('.vd-node-shape, .vd-container')) fills.push(cs(el, 'fill'));
      for (const bg of new Set(fills)) out.push({ kind: 'stroke', what, fg: cs(el, 'stroke'), bg });
    }
    return out;
  });
  expect(samples.length, 'nothing to measure').toBeGreaterThan(0);
  let minText = Infinity;
  let minStroke = Infinity;
  const violations: string[] = [];
  for (const s of samples) {
    if (s.bg === 'unmapped') { violations.push(`${s.kind} ${s.what} has no box`); continue; }
    const ratio = contrastRatio(s.fg, s.bg);
    if (s.kind === 'text') {
      minText = Math.min(minText, ratio);
      if (ratio < 4.5) violations.push(`text ${s.what} ${s.fg} on ${s.bg} is ${ratio.toFixed(2)} (< 4.5)`);
    } else {
      minStroke = Math.min(minStroke, ratio);
      if (ratio < 3) violations.push(`stroke ${s.what} ${s.fg} on ${s.bg} is ${ratio.toFixed(2)} (< 3)`);
    }
  }
  return { minText, minStroke, violations };
}
