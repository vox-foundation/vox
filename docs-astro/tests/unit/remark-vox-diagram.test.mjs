import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  remarkVoxDiagram,
  assertNoMisses,
  resetMisses,
  normalizeSource,
  svgToHast,
  reprefix,
  pageSlug,
} from '../../src/plugins/remark-vox-diagram.mjs';

const FIXTURES = fileURLToPath(new URL('../../../crates/vox-diagram/tests/fixtures/', import.meta.url));
const SVG_DIR = join(FIXTURES, 'svg');

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

test('a miss in a mounted repo page names its real source, not docs/src (WR-05)', () => {
  resetMisses();
  const run = remarkVoxDiagram({ manifestPath: NO_MANIFEST, strict: false });
  const mounted = '/r/docs-astro/src/content/docs/repo/agents-md.md';
  run(fenceTree(SOURCE), { path: mounted, data: { astro: { frontmatter: { mounted_from: 'AGENTS.md' } } } });
  run(fenceTree(SOURCE), { path: '/r/docs-astro/src/content/docs/repo/crates-x-readme-md.md', data: {} });
  assert.throws(
    () => assertNoMisses(),
    (err) =>
      err.message.includes('  AGENTS.md fence 1') &&
      err.message.includes('docs-astro/src/content/docs/repo/crates-x-readme-md.md fence 1') &&
      !err.message.includes('docs/src/repo/'),
  );
  resetMisses();
});

// ---- Task 2: parser, ids, slug, near-miss, figure shape, pipeline, key parity ----

const goldens = (dir) =>
  readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) return goldens(full);
    return name.endsWith('.svg') ? [full] : [];
  });
const MINIMAL_VDG = readFileSync(join(SVG_DIR, 'cases/00-minimal.vdg'), 'utf8');
const MINIMAL_SVG = readFileSync(join(SVG_DIR, 'cases/00-minimal.svg'), 'utf8');

/** Write a manifest holding the minimal golden; returns { dir, manifestPath }. */
function minimalManifest() {
  const dir = mkdtempSync(join(tmpdir(), 'vd-manifest-'));
  const manifestPath = join(dir, 'manifest.json');
  const diagrams = [{ source: normalizeSource(MINIMAL_VDG), svg: MINIMAL_SVG }];
  writeFileSync(manifestPath, JSON.stringify({ schema_version: 1, diagrams }));
  return { dir, manifestPath };
}

const hasId = (node, id) =>
  node.type === 'element' && (node.properties.id === id || node.children.some((c) => hasId(c, id)));
const findEl = (node, match) => {
  if (node.type !== 'element') return null;
  const ok = typeof match === 'string' ? node.tagName === match : match(node);
  return ok ? node : node.children.map((c) => findEl(c, match)).find(Boolean) ?? null;
};

test('normalizeSource matches the Rust key rule', () => {
  assert.equal(normalizeSource('a\r\nb\rc\n'), 'a\nb\nc');
  assert.equal(normalizeSource('x \t\n\u3000\u0085'), 'x');
  assert.equal(normalizeSource('x\uFEFF'), 'x\uFEFF');
  assert.equal(normalizeSource('  \tx'), '  \tx');
});

test('svgToHast accepts every committed golden', () => {
  const files = goldens(SVG_DIR);
  assert.ok(files.length >= 3, `expected goldens under ${SVG_DIR}`);
  for (const file of files) {
    const root = svgToHast(readFileSync(file, 'utf8'));
    assert.equal(root.tagName, 'svg', file);
    assert.match(/^0 0 (\S+) \S+$/.exec(root.properties.viewBox)?.[1] ?? '', /^\d+(\.\d+)?$/, file);
  }
  const root = svgToHast('<svg viewBox="0 0 1 1"><text>&lt;a&gt; &amp; &quot;b&quot; &#39;c&#39;</text></svg>');
  assert.equal(root.children[0].children[0].value, `<a> & "b" 'c'`);
});

test('svgToHast rejects anything outside the writer grammar', () => {
  const wrap = (inner) => `<svg viewBox="0 0 1 1">${inner}</svg>`;
  const hostile = [
    wrap('<text>&nbsp;</text>'),
    wrap('<text>&#x41;</text>'),
    wrap('<g>'),
    wrap('</g>'),
    wrap('<script>alert(1)</script>'),
    wrap('<foreignObject></foreignObject>'),
    wrap('<style>*{}</style>'),
    wrap('<image/>'),
    wrap("<rect x='1'/>"),
    wrap('<rect hidden/>'),
    wrap('<!-- c -->'),
    wrap('<![CDATA[x]]>'),
    `<?xml version="1.0"?>${wrap('')}`,
    `<!DOCTYPE svg>${wrap('')}`,
    wrap('<rect onload="x"/>'),
    wrap('<rect ONCLICK="x"/>'),
    wrap('<path href="javascript:x"/>'),
    wrap('<path xlink:href="javascript:x"/>'),
    `${wrap('')}trailing`,
    wrap('') + wrap(''),
  ];
  for (const svg of hostile) assert.throws(() => svgToHast(svg), /remark-vox-diagram/, svg);
});

test('reprefix rewrites all three id families and refuses a stray id', () => {
  const root = reprefix(svgToHast(MINIMAL_SVG), 'vd-p-1');
  assert.equal(root.properties['aria-labelledby'], 'vd-p-1-title vd-p-1-desc');
  assert.ok(hasId(root, 'vd-p-1-title') && hasId(root, 'vd-p-1-desc') && hasId(root, 'vd-p-1-arrow'));
  assert.equal(findEl(root, (e) => 'marker-end' in e.properties).properties['marker-end'], 'url(#vd-p-1-arrow)');
  const stray = MINIMAL_SVG.replace('<rect ', '<rect id="other" ');
  assert.throws(() => reprefix(svgToHast(stray), 'vd-p-1'), /other/);
});

test('pageSlug', () => {
  assert.equal(pageSlug('/r/docs-astro/src/content/docs/reference/vox-diagram.md'), 'reference-vox-diagram');
  assert.equal(pageSlug('/r/docs/src/index.mdx'), 'index');
  assert.equal(pageSlug('/r/docs-astro/src/content/docs/repo/crates/x/README.md'), 'repo-crates-x-readme');
  assert.equal(pageSlug('/r/docs/src/architecture/qwen-3.7-Eval.md'), 'architecture-qwen-37-eval');
  for (const f of ['/r/docs/src/index.mdx', '/r/docs/src/a/B.c.md', undefined, '/elsewhere/9.md']) {
    assert.match(`vd-${pageSlug(f)}-1`, /^[a-z][a-z0-9-]*$/);
  }
});

test('near-miss info strings are left alone', () => {
  resetMisses();
  const tree = {
    type: 'root',
    children: [
      { type: 'code', lang: 'vox-diagram', meta: 'title="x"', value: 'a' },
      { type: 'code', lang: 'vox-diagram,x', meta: null, value: 'b' },
    ],
  };
  remarkVoxDiagram({ manifestPath: NO_MANIFEST, strict: true })(tree, { path: '/r/docs/src/a.md' });
  assert.deepEqual(tree.children.map((c) => c.type), ['code', 'code']);
  assert.doesNotThrow(() => assertNoMisses());
});

test('two identical fences get distinct prefixes; figure shape', () => {
  resetMisses();
  const { dir, manifestPath } = minimalManifest();
  try {
    const tree = fenceTree(MINIMAL_VDG, MINIMAL_VDG);
    remarkVoxDiagram({ manifestPath, strict: true })(tree, { path: '/r/docs/src/x.md' });
    const [a, b] = tree.children.map((n) => n.data.hChildren[0]);
    assert.ok(hasId(a, 'vd-x-1-title') && hasId(b, 'vd-x-2-title'));
    assert.ok(!hasId(a, 'vd-x-2-title'));

    const fig = tree.children[0];
    assert.equal(fig.type, 'vdFigure');
    assert.equal(fig.data.hName, 'figure');
    // Not a landmark or a tab stop by default: the page script adds both only to a figure that
    // scrolls (WR-03).
    assert.deepEqual(fig.data.hProperties, {
      className: ['vd-figure'],
      style: '--vd-w:96px',
      ariaLabelledBy: findEl(a, 'title').properties.id,
    });
    assert.equal(a.properties.dataPagefindIgnore, '');
    const [svg, details] = fig.data.hChildren;
    assert.equal(svg.tagName, 'svg');
    assert.equal(details.tagName, 'details');
    assert.deepEqual(details.properties.className, ['vd-source']);
    assert.deepEqual(details.children.map((c) => c.tagName), ['summary', 'pre']);
    const code = details.children[1].children[0];
    assert.deepEqual(code.properties.className, ['language-vox-diagram']);
    assert.equal(code.children[0].value, normalizeSource(MINIMAL_VDG));
  } finally {
    rmSync(dir, { recursive: true, force: true });
    resetMisses();
  }
});

test('each figure is named after its nearest heading and its page index (WR-03)', () => {
  resetMisses();
  const { dir, manifestPath } = minimalManifest();
  try {
    const heading = (depth, ...children) => ({ type: 'heading', depth, children });
    const fence = () => ({ type: 'code', lang: 'vox-diagram', meta: null, value: MINIMAL_VDG });
    const tree = {
      type: 'root',
      children: [
        fence(),
        heading(2, { type: 'text', value: 'Layout ' }, { type: 'inlineCode', value: 'stress' }),
        fence(),
        fence(),
        heading(3, { type: 'emphasis', children: [{ type: 'text', value: 'Other' }] }),
        fence(),
      ],
    };
    remarkVoxDiagram({ manifestPath, strict: true })(tree, { path: '/r/docs/src/x.md' });
    const titles = tree.children
      .filter((n) => n.type === 'vdFigure')
      .map((n) => flatText([findEl(n.data.hChildren[0], 'title')]));
    assert.deepEqual(titles, ['Diagram 1', 'Layout stress, diagram 2', 'Layout stress, diagram 3', 'Other, diagram 4']);
  } finally {
    rmSync(dir, { recursive: true, force: true });
    resetMisses();
  }
});

/** Astro's own markdown processor, resolved through astro (no direct dependency). */
async function markdownProcessor(remarkPlugins) {
  const fromHere = createRequire(import.meta.url);
  const viaAstro = createRequire(fromHere.resolve('astro/package.json'));
  const mod = await import(pathToFileURL(viaAstro.resolve('@astrojs/markdown-remark')).href);
  return mod.createMarkdownProcessor({ remarkPlugins, syntaxHighlight: false });
}

test('whole pipeline (md): figure emitted, never inside a pre', async () => {
  resetMisses();
  const { dir, manifestPath } = minimalManifest();
  try {
    const proc = await markdownProcessor([[remarkVoxDiagram, { manifestPath, strict: true }]]);
    const { code } = await proc.render(`# T\n\n\`\`\`vox-diagram\n${MINIMAL_VDG}\n\`\`\`\n`, {
      fileURL: new URL('file:///repo/docs/src/x.md'),
    });
    assert.match(code, /<figure class="vd-figure"/);
    assert.match(code, /vd-x-1-title/);
    assert.doesNotMatch(code, /<pre[^>]*>\s*<figure/);
    assert.doesNotMatch(code, /<script|onload=/i);
  } finally {
    rmSync(dir, { recursive: true, force: true });
    resetMisses();
  }
});

test('key parity with Rust: fence-keys.json', async () => {
  const cases = JSON.parse(readFileSync(join(FIXTURES, 'fence-keys.json'), 'utf8'));
  assert.ok(cases.length >= 20);
  for (const { name, md, keys } of cases) {
    const found = [];
    const collect = () => (tree) => {
      const walk = (n) => {
        if (n.type === 'code' && n.lang === 'vox-diagram' && !n.meta) found.push(normalizeSource(n.value));
        (n.children ?? []).forEach(walk);
      };
      walk(tree);
    };
    const proc = await markdownProcessor([collect]);
    await proc.render(md, { fileURL: new URL('file:///repo/docs/src/k.md') });
    assert.deepEqual(found, keys, name);
  }
});
