import test from 'node:test';
import assert from 'node:assert/strict';
import {
  remarkVoxDiagram,
  assertNoMisses,
  resetMisses,
} from '../../src/plugins/remark-vox-diagram.mjs';

const NO_MANIFEST = '/nonexistent/vox-diagram-manifest.json';
const SOURCE = 'graph LR\n  a --> b';

const fenceTree = (...values) => ({
  type: 'root',
  children: values.map((value) => ({ type: 'code', lang: 'vox-diagram', meta: null, value })),
});

const flatText = (nodes) =>
  nodes.map((n) => (n.type === 'text' ? n.value : flatText(n.children ?? []))).join('');

test('miss in strict mode records and throws', () => {
  resetMisses();
  const run = remarkVoxDiagram({ manifestPath: NO_MANIFEST, strict: true });
  assert.throws(
    () => run(fenceTree(SOURCE), { path: '/repo/docs/src/reference/x.md' }),
    (err) => err.message.includes('docs/src/reference/x.md') && err.message.includes('pnpm diagrams'),
  );
  assert.throws(() => assertNoMisses(), /docs\/src\/reference\/x\.md fence 1/);
  resetMisses();
});

test('miss in dev mode renders a placeholder', () => {
  resetMisses();
  const tree = fenceTree(SOURCE);
  remarkVoxDiagram({ manifestPath: NO_MANIFEST, strict: false })(tree, {
    path: '/repo/docs/src/reference/x.md',
  });
  const node = tree.children[0];
  assert.equal(node.type, 'vdFigure');
  assert.equal(node.data.hName, 'figure');
  assert.deepEqual(node.data.hProperties.className, ['vd-figure', 'vd-missing']);
  assert.equal(node.data.hProperties.role, 'note');
  const [notice, details] = node.data.hChildren;
  assert.match(flatText([notice]), /pnpm diagrams/);
  assert.deepEqual(details.properties.className, ['vd-source']);
  assert.match(flatText([details]), /graph LR/);
  assert.throws(() => assertNoMisses(), /fence 1/);
  resetMisses();
});

test('assertNoMisses lists every miss', () => {
  resetMisses();
  const run = remarkVoxDiagram({ manifestPath: NO_MANIFEST, strict: false });
  run(fenceTree(SOURCE), { path: '/repo/docs/src/a.md' });
  run(fenceTree('x', 'y'), { path: '/repo/docs/src/b/c.mdx' });
  assert.throws(
    () => assertNoMisses(),
    (err) =>
      ['docs/src/a.md fence 1', 'docs/src/b/c.mdx fence 1', 'docs/src/b/c.mdx fence 2', 'pnpm diagrams'].every(
        (s) => err.message.includes(s),
      ),
  );
  resetMisses();
  assert.doesNotThrow(() => assertNoMisses());
});
