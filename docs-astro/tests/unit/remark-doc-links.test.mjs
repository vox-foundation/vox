import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { docLinksGate, remarkDocLinks, resolveDocLink } from '../../src/plugins/remark-doc-links.mjs';

const REPO_URL = 'https://github.com/vox-foundation/vox';

/** A throwaway repo tree: docs/src pages plus a few out-of-tree files. */
function fixtureRepo() {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'remark-doc-links-')));
  const files = [
    'docs/src/index.mdx',
    'docs/src/tutorials/index.md',
    'docs/src/tutorials/tut-a.md',
    'docs/src/tutorials/tut-b.md',
    'docs/src/reference/cli.md',
    'docs/src/adr/README.md',
  ];
  for (const file of files) {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), '# page\n');
  }
  return root;
}

const repo = fixtureRepo();
process.on('exit', () => rmSync(repo, { recursive: true, force: true }));
const docsSrc = join(repo, 'docs/src');
const ctx = { repoRoot: repo, docsSrc, repoUrl: REPO_URL, strict: true };
const from = join(docsSrc, 'tutorials/tut-a.md');
const href = (url, file = from) => resolveDocLink(url, file, ctx)?.href ?? null;

test('sibling page link becomes its route', () => {
  assert.equal(href('tut-b.md'), '/tutorials/tut-b/');
});

test('parent-relative link keeps its anchor', () => {
  assert.equal(href('../reference/cli.md#vox-init'), '/reference/cli/#vox-init');
});

test('index.md collapses to its directory route', () => {
  assert.equal(href('./index.md'), '/tutorials/');
});

test('README.md in a subdir maps to <dir>/readme/', () => {
  assert.equal(href('../adr/README.md'), '/adr/readme/');
});

test('root index.mdx maps to /', () => {
  assert.equal(href('../index.mdx'), '/');
});

test('external, anchor-only, site-absolute and mailto links are untouched', () => {
  for (const url of ['https://rustup.rs/', '//cdn.example.com/x.md', '#top', '/reference/cli/', 'mailto:a@b.c']) {
    assert.equal(resolveDocLink(url, from, ctx), null, url);
  }
});

test('missing target throws naming the source file and URL', () => {
  assert.throws(() => href('../reference/nope.md'), (err) => {
    assert.match(err.message, /'\.\.\/reference\/nope\.md'/);
    assert.ok(err.message.includes(from));
    return true;
  });
});

test('missing target only warns when not strict', () => {
  const warn = console.warn;
  console.warn = () => {};
  try {
    assert.equal(resolveDocLink('nope.md', from, { ...ctx, strict: false }), null);
  } finally {
    console.warn = warn;
  }
});

test('plugin rewrites link and definition nodes in a docs/src file', () => {
  const tree = {
    type: 'root',
    children: [
      { type: 'paragraph', children: [{ type: 'link', url: 'tut-b.md#x', children: [] }] },
      { type: 'definition', identifier: 'ref', url: '../reference/cli.md' },
    ],
  };
  remarkDocLinks({ repoRoot: repo, repoUrl: REPO_URL })(tree, { path: from });
  assert.equal(tree.children[0].children[0].url, '/tutorials/tut-b/#x');
  assert.equal(tree.children[1].url, '/reference/cli/');
});

test('plugin is strict for docs/src files', () => {
  const tree = { type: 'root', children: [{ type: 'link', url: 'gone.md', children: [] }] };
  assert.throws(() => remarkDocLinks({ repoRoot: repo, repoUrl: REPO_URL })(tree, { path: from }), /dead link 'gone\.md'/);
});

test('build gate fails once any page had a dead link', () => {
  const tree = { type: 'root', children: [{ type: 'link', url: 'also-gone.md', children: [] }] };
  assert.throws(() => remarkDocLinks({ repoRoot: repo, repoUrl: REPO_URL })(tree, { path: from }));
  assert.throws(() => docLinksGate().hooks['astro:build:done'](), /dead link\(s\):[\s\S]*also-gone\.md/);
});
