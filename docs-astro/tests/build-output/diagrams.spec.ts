import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DIST_DIR, listDistHtml, readDist } from '../lib/dist';

// Built-site guarantees for VoxDiagram figures (Phase 89, DOCS-03). fs-only: run after `pnpm build`.
// A rendered figure ends `</details></figure>`; Expressive Code nests its own <figure> inside the
// `details`, so a lazy match up to the first `</figure>` would cut it short.
const FIGURE = /<figure class="vd-figure"[\s\S]*?<\/details><\/figure>/g;
const REFERENCE = 'reference/vox-diagram';

const pages = (): { file: string; html: string }[] =>
  listDistHtml().map((file) => ({ file, html: readFileSync(join(DIST_DIR, file), 'utf8') }));
const figuresOf = (html: string): string[] => html.match(FIGURE) ?? [];

test.describe('Build Output: VoxDiagram figures (DOCS-03)', () => {
  test('reference page has at least 2 figures, each one svg.vd-diagram plus one source block', () => {
    const figures = figuresOf(readDist(REFERENCE));
    expect(figures.length).toBeGreaterThanOrEqual(2);
    for (const fig of pages().flatMap((p) => figuresOf(p.html))) {
      expect(fig.match(/<svg\b/g)).toHaveLength(1);
      expect(fig).toMatch(/<svg [^>]*class="vd-diagram"/);
      expect(fig.match(/data-language="vox-diagram"/g)).toHaveLength(1);
    }
  });

  test('no placeholder ships and no figure sits inside a pre', () => {
    for (const { file, html } of pages()) {
      expect(html, file).not.toContain('vd-missing');
      expect(html, file).not.toMatch(/<pre\b[^>]*>\s*<figure class="vd-figure"/);
    }
  });

  test('ids are page-prefixed and unique per page', () => {
    for (const { file, html } of pages()) {
      const ids = figuresOf(html).flatMap((fig) => [...fig.matchAll(/\sid="(vd-[^"]*)"/g)].map((m) => m[1]));
      for (const id of ids) expect(id, file).toMatch(/^vd-[a-z0-9-]+-\d+-(title|desc|arrow)$/);
      expect(new Set(ids).size, file).toBe(ids.length);
    }
  });

  test('figures cannot match the Mermaid CDN loader selectors', () => {
    for (const { file, html } of pages()) {
      for (const fig of figuresOf(html)) {
        expect(fig, file).not.toContain('data-language="mermaid"');
        expect(fig, file).not.toContain('language-mermaid');
      }
    }
  });

  test('llms-full.txt keeps the fence source and drops the svg description', () => {
    const llms = readFileSync(join(DIST_DIR, 'llms-full.txt'), 'utf8');
    expect(llms).toMatch(/^```vox-diagram$/m);
    const desc = />([^<]+)<\/desc>/.exec(figuresOf(readDist(REFERENCE))[0])?.[1];
    expect(desc, 'reference figure has a <desc>').toBeTruthy();
    const plain = desc!.replace(/&#x([0-9a-f]+);/gi, (_, h) => String.fromCodePoint(parseInt(h, 16)))
      .replace(/&(amp|lt|gt|quot|#39);/g, (_, n) => ({ amp: '&', lt: '<', gt: '>', quot: '"', '#39': "'" })[n as string]!);
    expect(llms).not.toContain(plain);
  });

  test('the reference page svg is excluded from the search index', () => {
    const svgs = figuresOf(readDist(REFERENCE)).map((fig) => /<svg [^>]*>/.exec(fig)![0]);
    expect(svgs.length).toBeGreaterThanOrEqual(2);
    for (const tag of svgs) expect(tag).toContain('data-pagefind-ignore');
  });
});
