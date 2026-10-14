import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import matter from 'gray-matter';
import { docSlug } from './doc-slug.mjs';
import { statusPolicy } from './page-status.mjs';

/**
 * Recursively walks docsSrc to collect all .md/.mdx pages excluding archive/ and .well-known/.
 *
 * @param {string} docsSrc - Absolute or relative path to docs/src
 * @returns {Array<{ relPath: string, id: string, title: string, description: string, category: string, sort_order: number | null, status: string }>}
 */
export function listDocPages(docsSrc) {
  const pages = [];

  function walk(currentDir) {
    const entries = readdirSync(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = join(currentDir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === 'archive' || entry.name === '.well-known') {
          continue;
        }
        walk(fullPath);
      } else if (entry.name.endsWith('.md') || entry.name.endsWith('.mdx')) {
        const relPath = relative(docsSrc, fullPath).replace(/\\/g, '/');
        const raw = readFileSync(fullPath, 'utf8');
        const { data } = matter(raw);

        const id = docSlug(relPath);
        const title = typeof data.title === 'string' ? data.title.trim() : '';
        const description = typeof data.description === 'string' ? data.description.trim() : '';
        const category = typeof data.category === 'string' ? data.category.trim() : '';
        const sort_order = typeof data.sort_order === 'number' ? data.sort_order : null;
        const status = typeof data.status === 'string' ? data.status.trim() : '';

        pages.push({
          relPath,
          id,
          title,
          description,
          category,
          sort_order,
          status,
        });
      }
    }
  }

  walk(docsSrc);
  return pages;
}

/**
 * Returns a Set of URL pathnames (e.g. `/<id>/`, or `/` for root) for pages whose statusPolicy says noindex.
 *
 * @param {Array<{ id: string, status: string }>} pages
 * @returns {Set<string>}
 */
export function noindexRoutes(pages) {
  const routes = new Set();
  for (const page of pages) {
    const policy = statusPolicy(page.status);
    if (policy.noindex) {
      const path = page.id ? `/${page.id}/` : '/';
      routes.add(path);
    }
  }
  return routes;
}

/**
 * Returns a sorted array of document IDs whose status belongs in the Internals section.
 *
 * @param {Array<{ id: string, status: string }>} pages
 * @returns {string[]}
 */
export function internalsDocIds(pages) {
  const ids = [];
  for (const page of pages) {
    const policy = statusPolicy(page.status);
    if (policy.internals) {
      ids.push(page.id);
    }
  }
  return ids.sort();
}
