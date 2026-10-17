// Three-way Mermaid oracle: the PINNED real parser judges fixtures that the Rust crate wrote.
//   C(B) = canon(original block)   R(B) = Rust sets from the imported IR   C(E) = canon(exported text)
// Asserted: accepts(E), C(B) == R(B) (importer), C(E) == R(B), C(E) == C(B) (exporter, lossless).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { accepts, canon, sameSets } from '../lib/mermaid-oracle.mjs';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const FIXTURES = join(REPO_ROOT, 'crates/vox-diagram/tests/fixtures');
const read = (...p) => readFileSync(join(FIXTURES, ...p), 'utf8');
const stems = (dir, suffix) =>
  readdirSync(join(FIXTURES, dir)).filter((f) => f.endsWith(suffix)).map((f) => f.slice(0, -suffix.length)).sort();

test('helper: accepts rejects a backtick-first label and accepts a minimal block', async () => {
  assert.equal(await accepts('flowchart TD\n  A["`x` y"]\n'), false);
  assert.equal(await accepts('flowchart TD\n    a --> b\n'), true);
});

// 84-01 import-cases family: original .mmd -> Rust import -> export -> sets.
for (const stem of stems('import-cases-roundtrip', '.sets.json')) {
  test(`import-cases-roundtrip/${stem}`, async () => {
    const original = read('import-cases', `${stem}.mmd`);
    const exported = read('import-cases-roundtrip', `${stem}.export.mmd`);
    const rust = JSON.parse(read('import-cases-roundtrip', `${stem}.sets.json`));
    assert.equal(await accepts(exported), true, 'exported text must be accepted by Mermaid');
    sameSets(await canon(original), rust, 'C(B) == R(B)');
    sameSets(await canon(exported), rust, 'C(E) == R(B)');
    sameSets(await canon(exported), await canon(original), 'C(E) == C(B)');
  });
}
