import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { docSlug } from '../../src/utils/doc-slug.mjs';
import { listDocPages } from '../../src/utils/page-index.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = resolve(__filename, '..');
const REPO_ROOT = resolve(__dirname, '../../..');
const DOCS_SRC = join(REPO_ROOT, 'docs/src');
const DIST_DIR = resolve(__dirname, '../../dist');

test('docSlug handles basic cases', () => {
  assert.equal(docSlug('index.md'), '');
  assert.equal(docSlug('tutorials/index.md'), 'tutorials');
  assert.equal(docSlug('reference/cli.md'), 'reference/cli');
  assert.equal(docSlug('AGENTS.md'), 'agents');
  assert.ok(docSlug('architecture/qwen-3.7-eval.md').includes('qwen-37'));
});

test('docSlug cross-checks against built dist if available', (t) => {
  if (!existsSync(DIST_DIR)) {
    t.skip('dist/ directory does not exist, skipping built page cross-check');
    return;
  }

  const pages = listDocPages(DOCS_SRC);
  assert.ok(pages.length > 500, `Expected at least 500 pages, found ${pages.length}`);

  const missing = [];
  for (const page of pages) {
    const relativeTarget = page.id ? join(page.id, 'index.html') : 'index.html';
    const builtPath = join(DIST_DIR, relativeTarget);
    if (!existsSync(builtPath)) {
      missing.push({ relPath: page.relPath, id: page.id, builtPath });
    }
  }

  assert.equal(
    missing.length,
    0,
    `Built HTML missing for ${missing.length} docs pages:\n` +
      missing.slice(0, 10).map((m) => `  - ${m.relPath} -> expected ${m.builtPath}`).join('\n')
  );
});
