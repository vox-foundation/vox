import test from 'node:test';
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { REPO_ROOT, RENDER_COMMAND } from '../../scripts/render-diagrams.mjs';

test('pre-render runs the pipeline flag from the repo root', () => {
  assert.deepEqual(RENDER_COMMAND.slice(1), ['run', '-p', 'vox-doc-pipeline', '--', '--render-diagrams']);
  assert.equal(RENDER_COMMAND[0], 'cargo');
  assert.equal(resolve(REPO_ROOT), resolve(fileURLToPath(new URL('../../../', import.meta.url))));
});
