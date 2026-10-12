import { readFileSync, readdirSync } from 'node:fs';
import { join, relative } from 'node:path';
import matter from 'gray-matter';
import { docSlug } from './doc-slug.mjs';
import { statusPolicy } from './page-status.mjs';

// Mirrors the docs collection in src/content.config.ts: Starlight loads
// `**/[^_]*.{md,mdx}` and the config excludes these paths.
const EXCLUDED_DIRS = new Set(['archive', '.well-known']);
const EXCLUDED_FILES = new Set(['SUMMARY.md']);

/**
 * Every docs page under `docsSrc` with the frontmatter fields the site uses:
 * `{ relPath, id, title, category, sort_order, status }`. `id` is the Starlight
 * route id (see doc-slug.mjs); the page is served at `/<id>/`.
 */
export function listDocPages(docsSrc) {
  const pages = [];
  const walk = (dir) => {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!EXCLUDED_DIRS.has(entry.name) && !entry.name.startsWith('.')) walk(full);
        continue;
      }
      if (!/\.mdx?$/.test(entry.name) || entry.name.startsWith('_')) continue;
      const relPath = relative(docsSrc, full).replace(/\\/g, '/');
      if (EXCLUDED_FILES.has(relPath)) continue;
      let data;
      try {
        ({ data } = matter(readFileSync(full, 'utf8')));
      } catch {
        continue;
      }
      pages.push({
        relPath,
        id: typeof data.slug === 'string' ? data.slug : docSlug(relPath),
        title: data.title || entry.name.replace(/\.mdx?$/, ''),
        category: data.category || null,
        sort_order: data.sort_order ?? 999,
        status: data.status || 'current',
      });
    }
  };
  walk(docsSrc);
  return pages.sort((a, b) => a.relPath.localeCompare(b.relPath));
}

/** Route paths (`/<id>/`) of pages whose status asks crawlers not to index them. */
export function noindexRoutes(pages) {
  return new Set(
    pages.filter((page) => statusPolicy(page.status).noindex).map((page) => (page.id ? `/${page.id}/` : '/')),
  );
}

/** Sorted ids of pages that belong in the Internals section. */
export function internalsDocIds(pages) {
  return pages
    .filter((page) => statusPolicy(page.status).internals)
    .map((page) => page.id)
    .sort();
}
