import test from 'node:test';
import assert from 'node:assert/strict';
import { parseGitLog, getGitDates } from '../../src/utils/git-dates.mjs';

test('git-dates parser unit tests', async (t) => {
  await t.test('(a) newest substantive commit wins over an older one', () => {
    const log = [
      'C|sha1|2026-10-05T12:00:00Z|feat: updated doc',
      'M\tdocs/src/page.md',
      'C|sha2|2026-10-01T12:00:00Z|feat: created doc',
      'A\tdocs/src/page.md',
    ].join('\n');

    const res = parseGitLog(log);
    assert.equal(res.get('docs/src/page.md'), '2026-10-05T12:00:00Z');
  });

  await t.test('(b) commit whose SHA is in ignoreShas is skipped', () => {
    const log = [
      'C|sha-ignored|2026-10-05T12:00:00Z|mechanical update',
      'M\tdocs/src/page.md',
      'C|sha-substantive|2026-10-01T12:00:00Z|feat: real work',
      'M\tdocs/src/page.md',
    ].join('\n');

    const res = parseGitLog(log, { ignoreShas: new Set(['sha-ignored']) });
    assert.equal(res.get('docs/src/page.md'), '2026-10-01T12:00:00Z');
  });

  await t.test('(c) chore(ssot): auto-regenerate ... is skipped', () => {
    const log = [
      'C|sha1|2026-10-05T12:00:00Z|chore(ssot): auto-regenerate docs',
      'M\tdocs/src/page.md',
      'C|sha2|2026-10-01T12:00:00Z|fix: actual doc fix',
      'M\tdocs/src/page.md',
    ].join('\n');

    const res = parseGitLog(log);
    assert.equal(res.get('docs/src/page.md'), '2026-10-01T12:00:00Z');
  });

  await t.test('(d) commit with bulkThreshold+1 files is skipped', () => {
    const files = Array.from({ length: 5 }, (_, idx) => `M\tdocs/src/f${idx}.md`).join('\n');
    const log = [
      'C|sha-bulk|2026-10-05T12:00:00Z|bulk commit across repo',
      files,
      'C|sha-substantive|2026-10-01T12:00:00Z|single edit',
      'M\tdocs/src/f0.md',
    ].join('\n');

    const res = parseGitLog(log, { bulkThreshold: 4 });
    assert.equal(res.get('docs/src/f0.md'), '2026-10-01T12:00:00Z');
  });

  await t.test('(e) rename credits older edit to new path', () => {
    const log = [
      'C|sha1|2026-10-05T12:00:00Z|rename file',
      'R100\tdocs/src/old.md\tdocs/src/new.md',
      'C|sha2|2026-10-01T12:00:00Z|edit old file',
      'M\tdocs/src/old.md',
    ].join('\n');

    const res = parseGitLog(log);
    assert.equal(res.get('docs/src/new.md'), '2026-10-05T12:00:00Z');
  });

  await t.test('(f) file touched only by ignored commits gets newest ignored date', () => {
    const log = [
      'C|sha-ignored|2026-10-05T12:00:00Z|chore(ssot): auto-regenerate',
      'M\tdocs/src/only-ignored.md',
    ].join('\n');

    const res = parseGitLog(log);
    assert.equal(res.get('docs/src/only-ignored.md'), '2026-10-05T12:00:00Z');
  });

  await t.test('(g) deleted file is absent', () => {
    const log = [
      'C|sha1|2026-10-05T12:00:00Z|delete file',
      'D\tdocs/src/deleted.md',
      'C|sha2|2026-10-01T12:00:00Z|old edit',
      'M\tdocs/src/deleted.md',
    ].join('\n');

    const res = parseGitLog(log);
    assert.equal(res.has('docs/src/deleted.md'), false);
  });

  await t.test('smoke test against real repo', () => {
    const dates = getGitDates();
    assert.ok(dates.size > 300, `expected >300 dates, got ${dates.size}`);
    assert.ok(dates.has('docs/src/tutorials/tut-getting-started.md'), 'expected tut-getting-started.md in map');
  });
});
