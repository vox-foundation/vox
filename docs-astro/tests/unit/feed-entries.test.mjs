import { test } from 'node:test';
import assert from 'node:assert/strict';
import { feedEntries } from '../../src/utils/feed-entries.mjs';

const doc = (id, status) => ({ id, data: status === undefined ? {} : { status } });
const DATES = {
  current: '2026-10-01T00:00:00Z',
  research: '2026-10-09T00:00:00Z',
  roadmap: '2026-10-08T00:00:00Z',
  deprecated: '2026-10-07T00:00:00Z',
  legacy: '2026-10-06T00:00:00Z',
  shouty: '2026-10-05T00:00:00Z',
  nostatus: '2026-09-30T00:00:00Z',
  experimental: '2026-10-02T00:00:00Z',
};
const dateFor = (d) => DATES[d.id];

test('noindex statuses (research, roadmap, deprecated, legacy) never reach the feed', () => {
  const docs = [
    doc('current', 'current'),
    doc('research', 'research'),
    doc('roadmap', 'roadmap'),
    doc('deprecated', 'deprecated'),
    doc('legacy', 'legacy'),
    doc('shouty', ' Research '),
    doc('nostatus'),
    doc('experimental', 'experimental'),
  ];
  assert.deepEqual(
    feedEntries(docs, dateFor).map((e) => e.doc.id),
    ['experimental', 'current', 'nostatus'],
  );
});

test('entries are newest first, undated docs are skipped, and the limit applies after filtering', () => {
  const docs = [
    doc('research', 'research'),
    doc('nostatus'),
    doc('undated', 'current'),
    doc('current', 'current'),
    doc('experimental', 'experimental'),
  ];
  const entries = feedEntries(docs, dateFor, 2);
  assert.deepEqual(entries.map((e) => e.doc.id), ['experimental', 'current']);
  assert.deepEqual(entries.map((e) => e.date), [DATES.experimental, DATES.current]);
});
