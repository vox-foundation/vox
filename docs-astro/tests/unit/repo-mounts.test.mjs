import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import matter from 'gray-matter';
import { discoverMounts, mountRoute, wrapperSource } from '../../src/utils/repo-mounts.mjs';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

test('mountRoute lower-cases and dashes path separators, dots and underscores', () => {
  assert.equal(mountRoute('AGENTS.md'), 'agents-md');
  assert.equal(mountRoute('crates/vox-cli/README.md'), 'crates-vox-cli-readme-md');
  assert.equal(mountRoute('LANGUAGE_DESIGN_PRIORITIES.md'), 'language-design-priorities-md');
  assert.equal(mountRoute('.github/copilot-instructions.md'), 'github-copilot-instructions-md');
});

test('discoverMounts on the real repo: agent files mounted, no superpowers, unique routes', () => {
  const mounts = discoverMounts({ repoRoot: REPO_ROOT });
  const paths = mounts.map((mount) => mount.repoPath);
  for (const file of ['AGENTS.md', 'CLAUDE.md', 'GEMINI.md']) assert.ok(paths.includes(file), file);
  assert.deepEqual(paths.filter((path) => path.startsWith('docs/superpowers/')), []);
  assert.deepEqual(paths.filter((path) => path.startsWith('docs/src/')), []);
  const routes = mounts.map((mount) => mount.route);
  assert.equal(new Set(routes).size, routes.length);
  assert.ok(mounts.find((mount) => mount.repoPath === 'AGENTS.md').hasTitle);
});

test('discoverMounts follows docs links, skips code, prefixes, missing and docs/src targets', () => {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'repo-mounts-')));
  try {
    const files = {
      'docs/src/page.md': [
        '[a](../../NOTES.md) [b](../../docs/superpowers/plan.md) [c](../../gone.md) [d](other.md)',
        '`[e](../../IN_CODE.md)` [f](../../crates/x/README.md#usage) [g](../../src/lib.rs)',
      ].join('\n'),
      'docs/src/other.md': '# other\n',
      'NOTES.md': '---\ntitle: "Notes"\n---\n',
      'IN_CODE.md': '# in code\n',
      'ALWAYS.md': '# always\n',
      'docs/superpowers/plan.md': '# plan\n',
      'crates/x/README.md': '# x\n',
      'src/lib.rs': '',
    };
    for (const [file, body] of Object.entries(files)) {
      mkdirSync(dirname(join(root, file)), { recursive: true });
      writeFileSync(join(root, file), body);
    }
    const contract = { always_mount: ['ALWAYS.md', 'MISSING.md'], never_mount_prefixes: ['docs/superpowers/'] };
    assert.deepEqual(discoverMounts({ repoRoot: root, contract }), [
      { repoPath: 'ALWAYS.md', route: 'always-md', hasTitle: false },
      { repoPath: 'NOTES.md', route: 'notes-md', hasTitle: true },
      { repoPath: 'crates/x/README.md', route: 'crates-x-readme-md', hasTitle: false },
    ]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('wrapperSource: title from the first heading, which is removed; YAML-safe strings', () => {
  const page = wrapperSource('crates/x/README.md', '```md\n# not this\n```\n\n# X: "quoted" crate\n\nBody [l](../y.md)\n');
  const { data, content } = matter(page);
  assert.deepEqual(data, { title: 'X: "quoted" crate', mounted_from: 'crates/x/README.md', status: 'current' });
  assert.ok(content.includes('# not this'));
  assert.ok(!content.includes('# X:'));
  assert.ok(content.includes('Body [l](../y.md)'));
});

test('wrapperSource: contract title wins; repo path when there is no heading', () => {
  assert.equal(matter(wrapperSource('README.md', '# Heading\n', { 'README.md': 'Readme' })).data.title, 'Readme');
  assert.equal(matter(wrapperSource('README.md', '<p>html only</p>\n')).data.title, 'README.md');
});
