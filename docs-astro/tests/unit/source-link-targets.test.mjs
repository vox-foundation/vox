import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { relativeLinkTargets } from '../../src/utils/repo-mounts.mjs';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

// The guard's link extraction is shared with the /repo/ mount discovery.
export { relativeLinkTargets };

function livePages() {
  const out = execFileSync('git', ['ls-files', '-z', '--', 'docs/src'], { cwd: REPO_ROOT, encoding: 'utf8' });
  return out
    .split('\0')
    .filter((p) => /\.mdx?$/.test(p))
    .filter((p) => !p.startsWith('docs/src/archive/') && !p.startsWith('docs/src/.well-known/'));
}

test('relativeLinkTargets: inline, reference, code-stripped, skips non-relative', () => {
  const md = [
    'See [a](../a.md#sec) and [b](b.md "Title") and [c](https://x.dev) and [d](#top).',
    'Line ref [e](../../crates/x/src/lib.rs:12-20) and [f](/abs.md) and [g](<x.md>).',
    '`[not](code.md)` and [h](my%20file.md)',
    '[ref]: ./ref.md',
    '[^note]: *A Paper Title* (2025). https://arxiv.org/x',
    '```md',
    '[fenced](fenced.md)',
    '```',
    '```markdown',
    '```bash',
    '[nested](still-code.md)',
    '```',
    '[after](after.md)',
  ].join('\n');
  assert.deepEqual(relativeLinkTargets(md), [
    '../a.md',
    'b.md',
    '../../crates/x/src/lib.rs',
    'my file.md',
    'after.md',
    './ref.md',
  ]);
});

for (const page of livePages()) {
  test(page, () => {
    const abs = join(REPO_ROOT, page);
    const missing = relativeLinkTargets(readFileSync(abs, 'utf8')).filter(
      (t) => !existsSync(resolve(dirname(abs), t)),
    );
    assert.deepEqual(missing, [], `dead relative link targets in ${page}`);
  });
}
