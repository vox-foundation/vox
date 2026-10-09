import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

/** Remove fenced code blocks and inline code spans so example links are not checked. */
function stripCode(markdown) {
  const out = [];
  let fence = null;
  for (const line of markdown.split('\n')) {
    const m = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (fence) {
      // A closing fence carries no info string (CommonMark), so ```bash inside ```markdown does not close it.
      if (m && m[1][0] === fence[0] && m[1].length >= fence.length && !line.slice(line.indexOf(m[1]) + m[1].length).trim()) fence = null;
      out.push('');
      continue;
    }
    if (m) {
      fence = m[1];
      out.push('');
      continue;
    }
    out.push(line.replace(/(`+)[\s\S]*?\1/g, ''));
  }
  return out.join('\n');
}

/**
 * Relative link targets in `markdown`: inline `](target "title")` and reference
 * definitions `[label]: target` (footnotes `[^label]:` are not links), with code
 * stripped. URLs, anchors, absolute paths, autolinks and mailto are skipped.
 * Returned targets have the `#fragment` and a trailing `:N` / `:N-M` line suffix
 * removed and are URI-decoded.
 */
export function relativeLinkTargets(markdown) {
  const text = stripCode(markdown);
  const raw = [];
  for (const m of text.matchAll(/\]\(\s*([^)\s]+)(?:\s+"[^"]*")?\s*\)/g)) raw.push(m[1]);
  for (const m of text.matchAll(/^\s{0,3}\[(?!\^)[^\]]+\]:\s*(\S+)/gm)) raw.push(m[1]);
  const targets = [];
  for (const t of raw) {
    if (/^[a-z][a-z0-9+.-]*:/i.test(t) || /^[#/<]/.test(t)) continue;
    let path = t.split('#')[0].replace(/:\d+(-\d+)?$/, '');
    if (!path) continue;
    try {
      path = decodeURI(path);
    } catch {
      // keep the raw path; a malformed escape is still reported as missing
    }
    targets.push(path);
  }
  return targets;
}

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
