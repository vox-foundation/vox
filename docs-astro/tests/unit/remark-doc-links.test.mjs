import test from 'node:test';
import assert from 'node:assert/strict';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { resolveDocLink, remarkDocLinks } from '../../src/plugins/remark-doc-links.mjs';

const thisDir = fileURLToPath(new URL('.', import.meta.url));
const repoRoot = resolve(thisDir, '../../..');
const docsSrc = join(repoRoot, 'docs/src');
const repoUrl = 'https://github.com/vox-foundation/vox';

const ctx = {
  repoRoot,
  docsSrc,
  repoUrl,
  strict: true,
};

test('untouched URLs: external, protocol-relative, anchors, site-absolute, mailto', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  assert.equal(resolveDocLink('https://example.com/foo', fromFile, ctx), null);
  assert.equal(resolveDocLink('http://example.com/foo', fromFile, ctx), null);
  assert.equal(resolveDocLink('//cdn.example.com/lib.js', fromFile, ctx), null);
  assert.equal(resolveDocLink('#section-anchor', fromFile, ctx), null);
  assert.equal(resolveDocLink('/site/absolute/path', fromFile, ctx), null);
  assert.equal(resolveDocLink('mailto:hello@voxlang.org', fromFile, ctx), null);
});

test('in-docs relative links: cli.md with anchor', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const res = resolveDocLink('../reference/cli.md#vox-init', fromFile, ctx);
  assert.deepEqual(res, { href: '/reference/cli/#vox-init' });
});

test('in-docs relative links: installation.md', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const res = resolveDocLink('../reference/installation.md', fromFile, ctx);
  assert.deepEqual(res, { href: '/reference/installation/' });
});

test('in-docs relative links: index.mdx at docs root', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const res = resolveDocLink('../index.mdx', fromFile, ctx);
  assert.deepEqual(res, { href: '/' });
});

test('in-docs relative links: README.md in adr directory', () => {
  const fromFile = join(docsSrc, 'adr/001-architecture-records.md');
  const res = resolveDocLink('./README.md', fromFile, ctx);
  assert.deepEqual(res, { href: '/adr/readme/' });
});

test('in-docs relative links: missing target throws in strict mode', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  assert.throws(
    () => resolveDocLink('../reference/nonexistent-file-404.md', fromFile, ctx),
    /remark-doc-links: Target not found/
  );
});

test('out-of-tree file with line number and line range', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const resSingle = resolveDocLink('../../../crates/vox-cli/src/main.rs:42', fromFile, ctx);
  assert.deepEqual(resSingle, {
    href: 'https://github.com/vox-foundation/vox/blob/main/crates/vox-cli/src/main.rs#L42',
  });

  const resRange = resolveDocLink('../../../crates/vox-cli/src/main.rs:10-20', fromFile, ctx);
  assert.deepEqual(resRange, {
    href: 'https://github.com/vox-foundation/vox/blob/main/crates/vox-cli/src/main.rs#L10-L20',
  });
});

test('out-of-tree directory links to /tree/main/', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const res = resolveDocLink('../../../crates/vox-cli', fromFile, ctx);
  assert.deepEqual(res, {
    href: 'https://github.com/vox-foundation/vox/tree/main/crates/vox-cli',
  });
});

test('out-of-tree AGENTS.md links to /repo/agents-md/ when mountsMap is provided', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const ctxWithMounts = {
    ...ctx,
    mountsMap: new Map([['AGENTS.md', 'agents-md']]),
  };
  const res = resolveDocLink('../../../AGENTS.md', fromFile, ctxWithMounts);
  assert.deepEqual(res, {
    href: '/repo/agents-md/',
  });
});

test('out-of-tree unmounted .md links to GitHub blob', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  // With empty mountsMap, AGENTS.md is unmounted
  const ctxNoMounts = {
    ...ctx,
    mountsMap: new Map(),
  };
  const res = resolveDocLink('../../../AGENTS.md', fromFile, ctxNoMounts);
  assert.deepEqual(res, {
    href: 'https://github.com/vox-foundation/vox/blob/main/AGENTS.md',
  });
});

test('docs/src/archive/ links to GitHub blob', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const res = resolveDocLink('../archive/how-to-islands-and-pages.md', fromFile, ctx);
  assert.deepEqual(res, {
    href: 'https://github.com/vox-foundation/vox/blob/main/docs/src/archive/how-to-islands-and-pages.md',
  });
});

test('docs/superpowers/ links to GitHub blob', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  const ctxWithMounts = {
    ...ctx,
    mountsMap: new Map([['AGENTS.md', 'agents-md']]),
  };
  const res = resolveDocLink('../../../docs/superpowers/antigravity-handoff-ledger.md', fromFile, ctxWithMounts);
  assert.deepEqual(res, {
    href: 'https://github.com/vox-foundation/vox/blob/main/docs/superpowers/antigravity-handoff-ledger.md',
  });
});

test('wrapper base path: mounted_from frontmatter resolves links relative to repo source', () => {
  const plugin = remarkDocLinks({
    repoRoot,
    docsSrc,
    repoUrl,
    strict: true,
    mountsMap: new Map([['AGENTS.md', 'agents-md']]),
  });

  const fakeTree = {
    type: 'root',
    children: [
      {
        type: 'paragraph',
        children: [
          {
            type: 'link',
            url: 'Cargo.toml',
            children: [{ type: 'text', value: 'Cargo.toml' }],
          },
        ],
      },
    ],
  };

  const fakeFile = {
    path: join(repoRoot, 'docs-astro/src/content/docs/repo/crates-vox-cli-readme-md.md'),
    data: {
      astro: {
        frontmatter: {
          mounted_from: 'crates/vox-cli/README.md',
        },
      },
    },
  };

  plugin(fakeTree, fakeFile);

  const linkNode = fakeTree.children[0].children[0];
  assert.equal(
    linkNode.url,
    'https://github.com/vox-foundation/vox/blob/main/crates/vox-cli/Cargo.toml'
  );
});

test('repo escape path throws in strict mode', () => {
  const fromFile = join(docsSrc, 'tutorials/tut-getting-started.md');
  assert.throws(
    () => resolveDocLink('../../../../../../../etc/passwd', fromFile, ctx),
    /resolves outside repository root/
  );
});
