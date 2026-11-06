import test from 'node:test';
import assert from 'node:assert/strict';
import { remarkStripTitleH1 } from '../../src/plugins/remark-strip-title-h1.mjs';

const h1 = (text) => ({ type: 'heading', depth: 1, children: [{ type: 'text', value: text }] });
const para = (text) => ({ type: 'paragraph', children: [{ type: 'text', value: text }] });
const run = (children) => {
  const tree = { type: 'root', children };
  remarkStripTitleH1()(tree);
  return tree.children;
};

test('drops a leading H1, which Starlight already renders from the title', () => {
  assert.deepEqual(run([h1('Title'), para('Body')]), [para('Body')]);
});

test('skips HTML comments before the leading H1', () => {
  const comment = { type: 'html', value: '<!-- note -->' };
  assert.deepEqual(run([comment, h1('Title'), para('Body')]), [comment, para('Body')]);
});

test('keeps an H1 that does not open the page, and pages without one', () => {
  assert.deepEqual(run([para('Intro'), h1('Section')]), [para('Intro'), h1('Section')]);
  const h2 = { type: 'heading', depth: 2, children: [] };
  assert.deepEqual(run([h2, para('Body')]), [h2, para('Body')]);
});
