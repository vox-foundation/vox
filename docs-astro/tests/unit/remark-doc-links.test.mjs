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
    'docs/src/page.md',
    'docs/src/assets/logo.png',
    'docs/src/archive/x.md',
    'docs/superpowers/plans/p.md',
    'crates/vox-cli/src/main.rs',
    'AGENTS.md',
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

const page = join(docsSrc, 'page.md');
const BLOB = `${REPO_URL}/blob/main`;

test('source file with a line suffix becomes a blob #L link', () => {
  assert.equal(href('../../crates/vox-cli/src/main.rs:42', page), `${BLOB}/crates/vox-cli/src/main.rs#L42`);
});

test('line range becomes #LN-LM', () => {
  assert.equal(href('../../crates/vox-cli/src/main.rs:10-20', page), `${BLOB}/crates/vox-cli/src/main.rs#L10-L20`);
});

test('explicit fragment on a blob link is kept', () => {
  assert.equal(href('../../crates/vox-cli/src/main.rs#main', page), `${BLOB}/crates/vox-cli/src/main.rs#main`);
});

test('out-of-tree directory becomes a tree link', () => {
  assert.equal(href('../../crates/vox-cli/', page), `${REPO_URL}/tree/main/crates/vox-cli`);
});

test('repo Markdown outside docs/src goes to blob', () => {
  assert.equal(href('../../AGENTS.md', page), `${BLOB}/AGENTS.md`);
});

test('archive pages go to blob, not a site route', () => {
  assert.equal(href('../archive/x.md'), `${BLOB}/docs/src/archive/x.md`);
});

test('superpowers plans go to blob', () => {
  assert.equal(href('../superpowers/plans/p.md', page), `${BLOB}/docs/superpowers/plans/p.md`);
});

test('non-Markdown file inside docs/src goes to blob', () => {
  assert.equal(href('assets/logo.png', page), `${BLOB}/docs/src/assets/logo.png`);
});

test('docs directory with an index page maps to its route, without one to tree', () => {
  assert.equal(href('tutorials/', page), '/tutorials/');
  assert.equal(href('assets', page), `${REPO_URL}/tree/main/docs/src/assets`);
});

test('a link that escapes the repository throws', () => {
  assert.throws(() => href('../../../../etc/passwd', page), /leaves the repository/);
});

test('missing out-of-tree target throws', () => {
  assert.throws(() => href('../../crates/vox-gone/src/lib.rs:3', page), /dead link/);
});

test('definitions used by an image are left alone', () => {
  const tree = {
    type: 'root',
    children: [
      { type: 'paragraph', children: [{ type: 'imageReference', identifier: 'logo', children: [] }] },
      { type: 'definition', identifier: 'logo', url: 'assets/logo.png' },
      { type: 'definition', identifier: 'src', url: '../../AGENTS.md' },
    ],
  };
  remarkDocLinks({ repoRoot: repo, repoUrl: REPO_URL })(tree, { path: page });
  assert.equal(tree.children[1].url, 'assets/logo.png');
  assert.equal(tree.children[2].url, `${BLOB}/AGENTS.md`);
});
