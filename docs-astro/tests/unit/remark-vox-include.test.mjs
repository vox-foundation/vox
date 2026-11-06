import { test } from 'node:test';
import assert from 'node:assert/strict';
import { extractRegion } from '../../src/plugins/remark-vox-include.mjs';

const SRC = [
  'fn a() {}',
  '// ANCHOR: foo',
  '    let x = 1;',
  '    // tag::inner[]',
  '    let y = 2;',
  '    // end::inner[]',
  '// ANCHOR_END: foo',
  '# tag::foo-2[]',
  'other',
  '# end::foo-2[]',
].join('\n');

test('anchor: exact name, strips nested markers, dedents', () => {
  assert.equal(extractRegion(SRC, 'foo', 'f'), 'let x = 1;\nlet y = 2;');
});
test('tag with # leader; foo does not match foo-2', () => {
  assert.equal(extractRegion(SRC, 'foo-2', 'f'), 'other');
});
test('comma multi-select concatenates in order', () => {
  assert.equal(extractRegion(SRC, 'foo-2,inner', 'f'), 'other\n    let y = 2;');
});
test('line range', () => {
  assert.equal(extractRegion(SRC, '1..1', 'f'), 'fn a() {}');
});
test('no selector returns whole file minus markers', () => {
  assert.ok(!extractRegion(SRC, undefined, 'f').includes('ANCHOR'));
});
test('missing anchor and bad range throw', () => {
  assert.throws(() => extractRegion(SRC, 'nope', 'f'), /not found/);
  assert.throws(() => extractRegion(SRC, '9..2', 'f'), /Invalid line range/);
});
