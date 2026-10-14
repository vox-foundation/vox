import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  STATUS_POLICY,
  statusPolicy,
  isInternalsStatus,
  bannerFor,
} from '../../src/utils/page-status.mjs';

test('statusPolicy: research', () => {
  const p = statusPolicy('research');
  assert.equal(p.internals, true);
  assert.equal(p.noindex, true);
  assert.ok(p.banner?.includes('Internals — research note:'));
  assert.equal(isInternalsStatus('research'), true);
  assert.equal(bannerFor('research'), p.banner);
});

test('statusPolicy: roadmap', () => {
  const p = statusPolicy('roadmap');
  assert.equal(p.internals, true);
  assert.equal(p.noindex, true);
  assert.ok(p.banner?.includes('Internals — roadmap:'));
  assert.equal(isInternalsStatus('roadmap'), true);
  assert.equal(bannerFor('roadmap'), p.banner);
});

test('statusPolicy: deprecated', () => {
  const p = statusPolicy('deprecated');
  assert.equal(p.internals, false);
  assert.equal(p.noindex, true);
  assert.ok(p.banner?.includes('Deprecated:'));
  assert.equal(isInternalsStatus('deprecated'), false);
  assert.equal(bannerFor('deprecated'), p.banner);
});

test('statusPolicy: legacy', () => {
  const p = statusPolicy('legacy');
  assert.equal(p.internals, false);
  assert.equal(p.noindex, true);
  assert.ok(p.banner?.includes('Legacy:'));
  assert.equal(isInternalsStatus('legacy'), false);
  assert.equal(bannerFor('legacy'), p.banner);
});

test('statusPolicy: current', () => {
  const p = statusPolicy('current');
  assert.equal(p.internals, false);
  assert.equal(p.noindex, false);
  assert.equal(p.banner, null);
  assert.equal(isInternalsStatus('current'), false);
  assert.equal(bannerFor('current'), null);
});

test('statusPolicy: undefined and non-string', () => {
  const p1 = statusPolicy(undefined);
  assert.equal(p1.internals, false);
  assert.equal(p1.noindex, false);
  assert.equal(p1.banner, null);

  const p2 = statusPolicy(null);
  assert.equal(p2.internals, false);
  assert.equal(p2.noindex, false);
  assert.equal(p2.banner, null);
});

test('statusPolicy: casing and whitespace ("Research ")', () => {
  const p = statusPolicy('  Research  ');
  assert.equal(p.internals, true);
  assert.equal(p.noindex, true);
  assert.ok(p.banner?.includes('Internals — research note:'));
  assert.equal(isInternalsStatus('  Research  '), true);
  assert.equal(bannerFor('  Research  '), p.banner);
});
