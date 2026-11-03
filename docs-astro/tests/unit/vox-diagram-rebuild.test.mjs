// WR-01 regression: a build that fails on a diagram miss must fail again when rerun unchanged.
// Astro caches the failed md render (no body) in the build's content store and skips the unchanged
// file next time, so without the integration's pending marker the second build passes and ships an
// empty page. Two real `astro build` runs on a one-page project with no manifest.
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { build } from 'astro';
import { voxDiagram } from '../../src/plugins/vox-diagram-integration.mjs';

const DOCS_ASTRO = fileURLToPath(new URL('../../', import.meta.url));

const FILES = {
  'src/content.config.mjs': `import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
export const collections = { docs: defineCollection({ loader: glob({ pattern: '*.md', base: './src/docs' }) }) };
`,
  'src/pages/[id].astro': `---
import { getCollection, render } from 'astro:content';
export async function getStaticPaths() {
  return (await getCollection('docs')).map((entry) => ({ params: { id: entry.id }, props: { entry } }));
}
const { Content } = await render(Astro.props.entry);
---
<Content />
`,
  'src/docs/page.md': '# Page\n\n```vox-diagram\ngraph {\n    node a: "A"\n}\n```\n',
};

test('a diagram miss fails the build, and an unchanged rebuild fails again', async () => {
  // Inside docs-astro so the project resolves astro from docs-astro/node_modules.
  const root = mkdtempSync(join(DOCS_ASTRO, 'node_modules', '.vd-rebuild-'));
  try {
    for (const [rel, body] of Object.entries(FILES)) {
      mkdirSync(join(root, rel, '..'), { recursive: true });
      writeFileSync(join(root, rel), body);
    }
    const once = () =>
      build({
        root,
        logLevel: 'silent',
        outDir: join(root, 'dist'),
        cacheDir: join(root, 'cache'),
        vite: { cacheDir: join(root, 'vite') },
        integrations: [voxDiagram({ manifestFile: pathToFileURL(join(root, 'no-manifest.json')) })],
      });
    await assert.rejects(once(), /docs\/page\.md fence 1|page\.md fence 1/, 'first build');
    await assert.rejects(once(), /page\.md fence 1/, 'rebuild with the same (missing) manifest');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
