import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bannerFor, isInternalsStatus, statusPolicy } from '../../src/utils/page-status.mjs';

test('research and roadmap are Internals, noindex, with an Internals banner', () => {
  for (const status of ['research', 'roadmap']) {
    const policy = statusPolicy(status);
    assert.equal(policy.internals, true, status);
    assert.equal(policy.noindex, true, status);
    assert.match(policy.banner, /^<strong>Internals — /, status);
    assert.equal(isInternalsStatus(status), true, status);
  }
  assert.match(bannerFor('research'), /Internals — research note/);
  assert.match(bannerFor('roadmap'), /Internals — roadmap/);
});

test('deprecated and legacy keep their category but get a banner and noindex', () => {
  for (const [status, label] of [['deprecated', 'Deprecated'], ['legacy', 'Legacy']]) {
    const policy = statusPolicy(status);
    assert.equal(policy.internals, false, status);
    assert.equal(policy.noindex, true, status);
    assert.ok(policy.banner.startsWith(`<strong>${label}:</strong> `), status);
    assert.equal(isInternalsStatus(status), false, status);
  }
});

test('current, experimental, unknown and missing statuses get nothing', () => {
  for (const status of ['current', 'experimental', 'stable', '', undefined, null, 42, 'constructor']) {
    const policy = statusPolicy(status);
    assert.deepEqual(
      { internals: policy.internals, noindex: policy.noindex, banner: policy.banner },
      { internals: false, noindex: false, banner: null },
      String(status),
    );
    assert.equal(bannerFor(status), null);
  }
});

test('status matching ignores case and surrounding whitespace', () => {
  assert.equal(statusPolicy('Research ').internals, true);
  assert.equal(statusPolicy('  ROADMAP').noindex, true);
  assert.equal(bannerFor(' Legacy'), bannerFor('legacy'));
});

test('banners are constants: the status value never appears in the HTML', () => {
  assert.equal(bannerFor('<script>alert(1)</script>'), null);
  for (const status of ['research', 'roadmap', 'deprecated', 'legacy']) {
    assert.doesNotMatch(bannerFor(status), /<(?!\/?strong>)/, status);
  }
});
