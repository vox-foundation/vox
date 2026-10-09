import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const RECORD_PATH = join(REPO_ROOT, 'contracts/documentation/tutorial-verification.v1.json');
const TUTORIALS_DIR = 'docs/src/tutorials';
const CHECK_KEYS = ['commands_in_registry', 'snippets_compile', 'install_versions_match', 'links_resolve'];
const REGEN = 'tutorial changed since verification — run vox run --mode interp scripts/docs/tutorial-verify.vox';

/** 1-based line numbers of `// vox:skip` markers that carry no reason text. */
export function skipsWithoutReason(markdown) {
  const hits = [];
  markdown.split('\n').forEach((line, i) => {
    const at = line.indexOf('// vox:skip');
    if (at === -1) return;
    const rest = line.slice(at + '// vox:skip'.length);
    if (!/[A-Za-z]/.test(rest)) hits.push(i + 1);
  });
  return hits;
}

const record = JSON.parse(readFileSync(RECORD_PATH, 'utf8'));
const tutorialFiles = readdirSync(join(REPO_ROOT, TUTORIALS_DIR))
  .filter((f) => f.endsWith('.md'))
  .map((f) => `${TUTORIALS_DIR}/${f}`)
  .sort();

test('record has the v1 header and a full verified-at commit', () => {
  assert.equal(record['x-vox-version'], 1);
  assert.equal(record.generated_by, 'scripts/docs/tutorial-verify.vox');
  assert.match(record.verified_at_commit, /^[0-9a-f]{40}$/);
});

test('record covers exactly the seven tutorials on disk', () => {
  const paths = record.tutorials.map((t) => t.path).sort();
  assert.equal(paths.length, 7);
  assert.deepEqual(paths, tutorialFiles);
});

test('every tutorial blob_sha matches git hash-object (record is fresh)', () => {
  for (const t of record.tutorials) {
    const sha = execFileSync('git', ['hash-object', t.path], { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
    assert.equal(t.blob_sha, sha, `${t.path}: ${REGEN}`);
  }
});

test('every tutorial carries the four checks', () => {
  for (const t of record.tutorials) {
    assert.deepEqual(Object.keys(t.checks).sort(), [...CHECK_KEYS].sort(), t.path);
    for (const k of CHECK_KEYS) assert.ok(['pass', 'fail'].includes(t.checks[k]), `${t.path} ${k}`);
    assert.ok(Array.isArray(t.notes), t.path);
  }
});

test('skipsWithoutReason flags a bare marker and accepts a reasoned one', () => {
  assert.deepEqual(skipsWithoutReason('```vox\n// vox:skip\nfn a() {}\n```'), [2]);
  assert.deepEqual(skipsWithoutReason('// vox:skip — excerpt depends on out-of-file types'), []);
});

test('no tutorial has a vox:skip without a reason', () => {
  for (const path of tutorialFiles) {
    const md = readFileSync(join(REPO_ROOT, path), 'utf8');
    assert.deepEqual(skipsWithoutReason(md), [], path);
  }
});
