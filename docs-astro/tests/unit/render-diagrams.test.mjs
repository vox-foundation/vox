import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { REPO_ROOT, RENDER_COMMAND, clearStaleStore } from '../../scripts/render-diagrams.mjs';

test('pre-render runs the pipeline flag from the repo root', () => {
  assert.deepEqual(RENDER_COMMAND.slice(1), ['run', '-p', 'vox-doc-pipeline', '--', '--render-diagrams']);
  assert.equal(RENDER_COMMAND[0], 'cargo');
  assert.equal(resolve(REPO_ROOT), resolve(fileURLToPath(new URL('../../../', import.meta.url))));
});

test('clearStaleStore deletes the store only when the manifest changed', () => {
  const dir = mkdtempSync(join(tmpdir(), 'vd-store-'));
  try {
    const storePath = join(dir, 'data-store.json');
    writeFileSync(storePath, '{}');
    assert.equal(clearStaleStore({ before: 'a', after: 'a', storePath }), false);
    assert.ok(existsSync(storePath));
    assert.equal(clearStaleStore({ before: 'a', after: 'b', storePath }), true);
    assert.ok(!existsSync(storePath));
    assert.equal(clearStaleStore({ before: 'a', after: 'b', storePath }), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
