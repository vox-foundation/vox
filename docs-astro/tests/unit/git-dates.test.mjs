import { test } from 'node:test';
import assert from 'node:assert/strict';
import { getGitDates, parseGitLog } from '../../src/utils/git-dates.mjs';

const sha = (n) => String(n).repeat(40).slice(0, 40);

/** Build `git log --format=C|%H|%cI|%s --name-status -M` text, newest first. */
function log(...commits) {
  return commits
    .map(({ sha, date, subject, files }) => [`C|${sha}|${date}|${subject}`, '', ...files].join('\n'))
    .join('\n\n');
}

const D1 = '2026-01-01T00:00:00+00:00';
const D2 = '2026-02-01T00:00:00+00:00';
const D3 = '2026-03-01T00:00:00+00:00';

test('(a) newest substantive commit wins over an older one', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(2), date: D2, subject: 'docs: newer', files: ['M\tdocs/src/a.md'] },
      { sha: sha(1), date: D1, subject: 'docs: older', files: ['A\tdocs/src/a.md'] },
    ),
  );
  assert.equal(dates.get('docs/src/a.md'), D2);
});

test('(b) a commit whose SHA is in ignoreShas is skipped', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(2), date: D2, subject: 'chore(docs): bulk strip', files: ['M\tdocs/src/a.md'] },
      { sha: sha(1), date: D1, subject: 'docs: real edit', files: ['A\tdocs/src/a.md'] },
    ),
    { ignoreShas: new Set([sha(2)]) },
  );
  assert.equal(dates.get('docs/src/a.md'), D1);
});

test('(c) chore(ssot): auto-regenerate commits are skipped', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(2), date: D2, subject: 'chore(ssot): auto-regenerate drifted artifacts', files: ['M\tdocs/src/a.md'] },
      { sha: sha(1), date: D1, subject: 'docs: real edit', files: ['A\tdocs/src/a.md'] },
    ),
  );
  assert.equal(dates.get('docs/src/a.md'), D1);
});

test('style and fmt subjects are skipped', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(3), date: D3, subject: 'style(docs): wrap lines', files: ['M\tdocs/src/a.md'] },
      { sha: sha(2), date: D2, subject: 'chore: cargo fmt', files: ['M\tdocs/src/a.md'] },
      { sha: sha(1), date: D1, subject: 'docs: real edit', files: ['A\tdocs/src/a.md'] },
    ),
  );
  assert.equal(dates.get('docs/src/a.md'), D1);
});

test('(d) a commit listing more than bulkThreshold files is skipped', () => {
  const bulk = Array.from({ length: 4 }, (_, i) => `M\tdocs/src/f${i}.md`);
  const dates = parseGitLog(
    log(
      { sha: sha(2), date: D2, subject: 'docs: sweep', files: ['M\tdocs/src/a.md', ...bulk] },
      { sha: sha(1), date: D1, subject: 'docs: real edit', files: ['A\tdocs/src/a.md'] },
    ),
    { bulkThreshold: 4 },
  );
  assert.equal(dates.get('docs/src/a.md'), D1);
});

test('(e) a rename credits the older edit of the old path to the new path', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(2), date: D2, subject: 'docs: rename', files: ['R100\tdocs/src/a.md\tdocs/src/b.md'] },
      { sha: sha(1), date: D1, subject: 'docs: real edit', files: ['A\tdocs/src/a.md'] },
    ),
    { ignoreShas: new Set([sha(2)]) },
  );
  assert.equal(dates.get('docs/src/b.md'), D1);
  assert.equal(dates.has('docs/src/a.md'), false);
});

test('(f) a file touched only by ignored commits gets the newest ignored date', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(2), date: D2, subject: 'chore(ssot): auto-regenerate x', files: ['M\tdocs/src/a.md'] },
      { sha: sha(1), date: D1, subject: 'docs: bulk add', files: ['A\tdocs/src/a.md'] },
    ),
    { ignoreShas: new Set([sha(1)]) },
  );
  assert.equal(dates.get('docs/src/a.md'), D2);
});

test('(g) a deleted file is absent', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(2), date: D2, subject: 'docs: remove', files: ['D\tdocs/src/a.md'] },
      { sha: sha(1), date: D1, subject: 'docs: add', files: ['A\tdocs/src/a.md'] },
    ),
  );
  assert.equal(dates.has('docs/src/a.md'), false);
});

test('a re-added path does not inherit the previous file history', () => {
  const dates = parseGitLog(
    log(
      { sha: sha(3), date: D3, subject: 'docs: bulk re-add', files: ['A\tdocs/src/a.md'] },
      { sha: sha(2), date: D2, subject: 'docs: remove', files: ['D\tdocs/src/a.md'] },
      { sha: sha(1), date: D1, subject: 'docs: add', files: ['A\tdocs/src/a.md'] },
    ),
    { ignoreShas: new Set([sha(3)]) },
  );
  assert.equal(dates.get('docs/src/a.md'), D3);
});

test('getGitDates reads the real repo, keyed by repo path', () => {
  const dates = getGitDates();
  assert.ok(dates.size > 300, `expected >300 dated docs, got ${dates.size}`);
  assert.ok(dates.has('docs/src/tutorials/tut-getting-started.md'));
  for (const key of dates.keys()) assert.ok(key.startsWith('docs/src/'), key);
});
