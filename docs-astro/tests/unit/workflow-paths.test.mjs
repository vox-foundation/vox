import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { discoverMounts } from '../../src/utils/repo-mounts.mjs';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

// Inputs the site build reads outside docs/ and docs-astro/; a change to any of them must redeploy.
const BUILD_INPUTS = [
  'contracts/documentation/docs-sidebar-section-order.v1.json',
  'contracts/documentation/site-mounted-repo-docs.v1.json',
];

/** `paths:` lists per trigger (`push`, `pull_request`) from a workflow's `on:` block. */
function triggerPaths(yaml) {
  const lists = {};
  let trigger = null;
  let inPaths = false;
  for (const line of yaml.split('\n')) {
    if (/^\S/.test(line) && !line.startsWith('on:')) {
      if (Object.keys(lists).length) break;
      continue;
    }
    const triggerMatch = line.match(/^ {2}([a-z_]+):/);
    if (triggerMatch) {
      trigger = triggerMatch[1];
      inPaths = false;
      continue;
    }
    if (/^ {4}paths:\s*$/.test(line)) {
      inPaths = true;
      lists[trigger] = [];
      continue;
    }
    if (inPaths) {
      const item = line.match(/^ {6}- ['"]?([^'"]+?)['"]?\s*$/);
      if (item) lists[trigger].push(item[1]);
      else if (/^ {4}\S/.test(line)) inPaths = false;
    }
  }
  return lists;
}

/** GitHub Actions path-filter glob → RegExp (`**` crosses `/`, `*` does not). */
function globToRegExp(glob) {
  let out = '';
  for (let i = 0; i < glob.length; i += 1) {
    const ch = glob[i];
    if (ch === '*' && glob[i + 1] === '*') {
      out += glob[i + 2] === '/' ? '(?:.*/)?' : '.*';
      i += glob[i + 2] === '/' ? 2 : 1;
    } else if (ch === '*') out += '[^/]*';
    else if (ch === '?') out += '[^/]';
    else out += ch.replace(/[.+^${}()|[\]\\]/g, '\\$&');
  }
  return new RegExp(`^${out}$`);
}

const covered = (globs, path) => globs.some((glob) => globToRegExp(glob).test(path));

function workflowPaths(name) {
  return triggerPaths(readFileSync(join(REPO_ROOT, '.github/workflows', name), 'utf8'));
}

test('globToRegExp follows GitHub path-filter semantics', () => {
  assert.ok(globToRegExp('docs/**').test('docs/src/a/b.md'));
  assert.ok(globToRegExp('crates/**/README.md').test('crates/vox-cli/README.md'));
  assert.ok(globToRegExp('crates/**/README.md').test('crates/a/b/README.md'));
  assert.ok(!globToRegExp('crates/**/README.md').test('crates/vox-cli/src/lib.rs'));
  assert.ok(!globToRegExp('README.md').test('apps/README.md'));
  assert.ok(!globToRegExp('contracts/*.json').test('contracts/documentation/x.json'));
});

test('triggerPaths reads push and pull_request lists and stops at jobs', () => {
  const yaml = [
    'name: x',
    'on:',
    '  pull_request:',
    '    paths:',
    "      - 'docs/**'",
    '  push:',
    '    branches: [main]',
    '    paths:',
    '      # comment',
    "      - 'README.md'",
    '      - "a/b.md"',
    '  workflow_dispatch:',
    'jobs:',
    '  build:',
    '    paths:',
    "      - 'ignored'",
  ].join('\n');
  assert.deepEqual(triggerPaths(yaml), { pull_request: ['docs/**'], push: ['README.md', 'a/b.md'] });
});

test('docs-deploy push paths cover every mounted repo file and build input', () => {
  const { push } = workflowPaths('docs-deploy.yml');
  assert.ok(push?.length, 'docs-deploy.yml has push paths');
  const missing = [...discoverMounts({ repoRoot: REPO_ROOT }).map((mount) => mount.repoPath), ...BUILD_INPUTS]
    .filter((path) => !covered(push, path));
  assert.deepEqual(missing, [], 'add these to docs-deploy.yml on.push.paths');
});

test('docs-quality push and pull_request paths cover every mounted repo file', () => {
  const lists = workflowPaths('docs-quality.yml');
  const mounts = discoverMounts({ repoRoot: REPO_ROOT }).map((mount) => mount.repoPath);
  for (const trigger of ['push', 'pull_request']) {
    assert.ok(lists[trigger]?.length, `docs-quality.yml has ${trigger} paths`);
    const missing = mounts.filter((path) => !covered(lists[trigger], path));
    assert.deepEqual(missing, [], `add these to docs-quality.yml on.${trigger}.paths`);
  }
});
