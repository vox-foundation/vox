import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { docSlug } from '../../src/utils/doc-slug.mjs';
import { internalsDocIds, listDocPages, noindexRoutes } from '../../src/utils/page-index.mjs';

const DOCS_SRC = fileURLToPath(new URL('../../../docs/src', import.meta.url));
const DIST_DIR = fileURLToPath(new URL('../../dist', import.meta.url));

test('docSlug matches the ids Starlight assigns', () => {
  assert.match(docSlug('architecture/qwen-3.7-profile-and-mens-4b-feasibility-2026-06-07.md'), /qwen-37/);
  assert.equal(docSlug('reference/cli.md'), 'reference/cli');
  assert.equal(docSlug('index.md'), '');
  assert.equal(docSlug('index.mdx'), '');
  assert.equal(docSlug('tutorials/index.md'), 'tutorials');
  assert.equal(docSlug('contributors/AGENTS.md'), 'contributors/agents');
  assert.equal(docSlug('reference\\cli.md'), 'reference/cli');
});

test('page index derives noindex routes and Internals ids from status', () => {
  const pages = [
    { relPath: 'a.md', id: 'a', status: 'research' },
    { relPath: 'b/c.md', id: 'b/c', status: 'roadmap' },
    { relPath: 'd.md', id: 'd', status: 'legacy' },
    { relPath: 'e.md', id: 'e', status: 'current' },
  ];
  assert.deepEqual([...noindexRoutes(pages)].sort(), ['/a/', '/b/c/', '/d/']);
  assert.deepEqual(internalsDocIds(pages), ['a', 'b/c']);
});

test('listDocPages skips the archive and returns frontmatter fields', () => {
  const pages = listDocPages(DOCS_SRC);
  assert.ok(pages.length > 300, `only ${pages.length} pages`);
  assert.ok(!pages.some((page) => page.relPath.startsWith('archive/')));
  const audit = pages.find((page) => page.relPath === 'architecture/front-facing-honesty-audit-2026.md');
  assert.equal(audit?.status, 'research');
  assert.equal(audit?.id, 'architecture/front-facing-honesty-audit-2026');
});

test('every listed page id has a built page', (t) => {
  if (!existsSync(DIST_DIR)) {
    t.skip('dist/ does not exist; run pnpm build to cross-check ids');
    return;
  }
  const missing = listDocPages(DOCS_SRC)
    .filter((page) => !existsSync(join(DIST_DIR, page.id, 'index.html')))
    .map((page) => `${page.relPath} -> ${page.id}`);
  assert.deepEqual(missing, []);
});
