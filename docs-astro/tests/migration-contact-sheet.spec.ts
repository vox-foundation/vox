import { test, expect, type Page } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { NATURAL_CSS } from './lib/diagrams';

/*
 * Phase 90 migration contact sheets (MIG-03). Skipped unless CONTACT_SHEET_STEMS (comma list of
 * corpus MANIFEST stems) and CONTACT_SHEET_OUT (absolute directory) are both set. Per stem it
 * writes `<stem>.png` (pinned Mermaid render of the frozen original, then the built site's figure
 * at natural size in the light and dark theme) and `<stem>-column.png` (the figure as shipped in
 * the page column). Needs a built `dist/` served by the webServer (`pnpm build`).
 */
const STEMS = (process.env.CONTACT_SHEET_STEMS ?? '').split(',').map((s) => s.trim()).filter(Boolean);
const OUT = process.env.CONTACT_SHEET_OUT ?? '';
const CONFIGURED = STEMS.length > 0 && OUT !== '';
const ROOT = resolve(process.cwd(), '..');
const CORPUS = join(ROOT, 'crates/vox-diagram/tests/fixtures/corpus');
const MERMAID_JS = join(process.cwd(), 'node_modules/mermaid/dist/mermaid.min.js');

interface Entry { fixture: string; source: string; open_line: number }

/** Info strings (`mermaid` / `vox-diagram`) of the unindented opening fences of a doc, in order. */
function diagramFences(markdown: string): string[] {
  const found: string[] = [];
  let open = false;
  for (const line of markdown.split(/\r?\n/)) {
    const m = line.match(/^```(.*)$/);
    if (!m) continue;
    if (open) { open = false; continue; }
    open = true;
    const info = m[1].trim();
    if (info === 'mermaid' || info === 'vox-diagram') found.push(info);
  }
  return found;
}

const pngWidth = (png: Buffer): number => png.readUInt32BE(16);
const dataUrl = (png: Buffer): string => `data:image/png;base64,${png.toString('base64')}`;
const escapeHtml = (s: string): string => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');

/** Renders the frozen block with the pinned Mermaid at natural size; the error text when it throws. */
async function mermaidBefore(page: Page, source: string): Promise<{ png?: Buffer; error?: string }> {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('about:blank');
  await page.setContent('<body style="margin:0;background:#fff"><div id="host" style="display:inline-block;padding:8px"></div></body>');
  await page.addScriptTag({ path: MERMAID_JS });
  const error = await page.evaluate(async (src) => {
    const m = (window as unknown as { mermaid: { initialize(c: object): void; render(id: string, s: string): Promise<{ svg: string }> } }).mermaid;
    m.initialize({ startOnLoad: false, securityLevel: 'strict' });
    try {
      const { svg } = await m.render('mm-before', src);
      const host = document.getElementById('host');
      if (!host) return 'no host';
      host.innerHTML = svg;
      const el = host.querySelector('svg');
      const vb = el?.getAttribute('viewBox')?.split(/[ ,]+/).map(Number) ?? [];
      if (el && vb.length === 4) {
        el.style.maxWidth = 'none';
        el.setAttribute('width', String(vb[2]));
        el.setAttribute('height', String(vb[3]));
      }
      return '';
    } catch (e) {
      return e instanceof Error ? e.message : String(e);
    }
  }, source);
  if (error) return { error };
  const host = page.locator('#host');
  const box = await host.boundingBox();
  if (box) await page.setViewportSize({ width: Math.ceil(box.width) + 16, height: Math.ceil(box.height) + 16 });
  return { png: await host.screenshot() };
}

for (const stem of CONFIGURED ? STEMS : ['(unconfigured)']) {
  test(`contact sheet ${stem}`, async ({ page }) => {
    test.skip(!CONFIGURED, 'set CONTACT_SHEET_STEMS and CONTACT_SHEET_OUT');
    test.setTimeout(300_000);
    mkdirSync(OUT, { recursive: true });

    const manifest: Entry[] = JSON.parse(readFileSync(join(CORPUS, 'MANIFEST.json'), 'utf8'));
    const entry = manifest.find((e) => e.fixture === `${stem}.mmd`);
    if (!entry) throw new Error(`${stem}: not in the corpus MANIFEST`);
    const siblings = manifest.filter((e) => e.source === entry.source).sort((a, b) => a.open_line - b.open_line);
    const fences = diagramFences(readFileSync(join(ROOT, entry.source), 'utf8'));
    expect(fences, `${stem}: fence count of ${entry.source} differs from the MANIFEST`).toHaveLength(siblings.length);
    const k = siblings.findIndex((e) => e.fixture === entry.fixture);
    if (fences[k] !== 'vox-diagram') throw new Error(`${stem}: fence not migrated`);
    const index = fences.slice(0, k).filter((f) => f === 'vox-diagram').length;
    const figures = fences.filter((f) => f === 'vox-diagram').length;
    const route = `/${entry.source.replace(/^docs\/src\//, '').replace(/\.mdx?$/, '')}/`;

    // After: as shipped in the column, then lifted to natural size in both themes.
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(route, { waitUntil: 'domcontentloaded' });
    await page.evaluate(() => { document.documentElement.dataset.theme = 'light'; });
    await page.evaluate(() => document.fonts.ready);
    const all = page.locator('figure.vd-figure');
    expect(await all.count(), `${stem}: figures on ${route} vs vox-diagram fences in the doc`).toBe(figures);
    const fig = all.nth(index);
    await fig.screenshot({ path: join(OUT, `${stem}-column.png`) });
    await page.addStyleTag({ content: NATURAL_CSS });
    await fig.evaluate((el) => el.classList.add('vd-natural'));
    for (let pass = 0; pass < 2; pass += 1) {
      const box = await fig.boundingBox();
      if (box) await page.setViewportSize({ width: Math.max(320, Math.ceil(box.width) + 16), height: Math.max(200, Math.ceil(box.height) + 16) });
    }
    const light = await fig.screenshot();
    await page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
    const dark = await fig.screenshot();

    // Before: the frozen original through the pinned Mermaid.
    const source = readFileSync(join(CORPUS, entry.fixture), 'utf8');
    const before = await mermaidBefore(page, source);

    const panel = (title: string, body: string): string =>
      `<section style="margin:8px"><h3 style="font:600 14px sans-serif">${title}</h3>${body}</section>`;
    const img = (png: Buffer): string => `<img src="${dataUrl(png)}">`;
    const beforeBody = before.png
      ? img(before.png)
      : `<pre style="font:12px monospace;max-width:900px;white-space:pre-wrap">Mermaid 11.17.2 threw: ${escapeHtml(before.error ?? '')}\n\n${escapeHtml(source)}</pre>`;
    const widths = [before.png ? pngWidth(before.png) : 900, pngWidth(light), pngWidth(dark)];
    const width = Math.min(3200, Math.max(1200, widths.reduce((a, b) => a + b + 40, 40)));
    await page.setViewportSize({ width, height: 800 });
    await page.setContent(
      `<body style="margin:0;padding:12px;background:#fff;color:#111;font-family:sans-serif">` +
        `<h2 style="font:700 16px sans-serif">${stem} (${entry.source}:${entry.open_line})</h2>` +
        `<div style="display:flex;flex-wrap:wrap;align-items:flex-start">` +
        panel('Mermaid 11.17.2 (original)', beforeBody) +
        panel('VoxDiagram light', img(light)) +
        panel('VoxDiagram dark', img(dark)) +
        `</div></body>`,
    );
    await page.waitForFunction(() => Array.from(document.images).every((i) => i.complete));
    await page.screenshot({ path: join(OUT, `${stem}.png`), fullPage: true });
    console.log(`${stem}: ${route} figure ${index + 1} of ${figures}; mermaid ${before.png ? 'rendered' : 'threw'}`);
  });
}
