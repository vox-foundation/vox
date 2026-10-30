import test from 'node:test';
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { REPO_ROOT, RENDER_COMMAND, mountArgs, renderDiagrams } from '../../scripts/render-diagrams.mjs';
import { discoverMounts } from '../../src/utils/repo-mounts.mjs';

test('pre-render runs the pipeline flag from the repo root', () => {
  assert.deepEqual(RENDER_COMMAND.slice(1), ['run', '-p', 'vox-doc-pipeline', '--', '--render-diagrams']);
  assert.equal(RENDER_COMMAND[0], 'cargo');
  assert.equal(resolve(REPO_ROOT), resolve(fileURLToPath(new URL('../../../', import.meta.url))));
});

test('pre-render covers every repo file the site mounts (WR-05)', () => {
  const args = mountArgs();
  const passed = args.filter((_, i) => i % 2 === 1);
  assert.ok(args.every((a, i) => i % 2 === 1 || a === '--mount'), JSON.stringify(args.slice(0, 6)));
  for (const always of ['AGENTS.md', 'CLAUDE.md', 'GEMINI.md']) assert.ok(passed.includes(always), always);
  assert.deepEqual(passed, discoverMounts({ repoRoot: REPO_ROOT }).map((m) => m.repoPath));
  const calls = [];
  const spawn = (cmd, argv, opts) => (calls.push({ cmd, argv, opts }), { status: 0 });
  assert.equal(renderDiagrams({ spawn }), 0);
  assert.deepEqual(calls[0].argv, [...RENDER_COMMAND.slice(1), ...args]);
  assert.equal(calls[0].opts.cwd, REPO_ROOT);
});
