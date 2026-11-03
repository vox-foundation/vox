import test from 'node:test';
import assert from 'node:assert/strict';
import { mountRoute, discoverMounts } from '../../src/utils/repo-mounts.mjs';

test('mountRoute generates clean lower-case slug with hyphens', () => {
  assert.equal(mountRoute('AGENTS.md'), 'agents-md');
  assert.equal(mountRoute('crates/vox-cli/README.md'), 'crates-vox-cli-readme-md');
  assert.equal(mountRoute('LANGUAGE_DESIGN_PRIORITIES.md'), 'language-design-priorities-md');
  assert.equal(mountRoute('CLAUDE.md'), 'claude-md');
  assert.equal(mountRoute('GEMINI.md'), 'gemini-md');
});

test('discoverMounts discovers always_mount and docs links on real repo', () => {
  const mounts = discoverMounts();
  assert.ok(mounts.length >= 3);

  const paths = mounts.map((m) => m.repoPath);
  assert.ok(paths.includes('AGENTS.md'), 'Should include AGENTS.md');
  assert.ok(paths.includes('CLAUDE.md'), 'Should include CLAUDE.md');
  assert.ok(paths.includes('GEMINI.md'), 'Should include GEMINI.md');

  // Verify none under docs/superpowers/
  for (const m of mounts) {
    assert.ok(
      !m.repoPath.startsWith('docs/superpowers/'),
      `Mount should not be under docs/superpowers: ${m.repoPath}`
    );
  }

  // Verify every route is unique
  const routes = mounts.map((m) => m.route);
  const uniqueRoutes = new Set(routes);
  assert.equal(uniqueRoutes.size, routes.length, 'Every mount route must be unique');
});
