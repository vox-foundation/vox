// Node half of the diagram-authoring offline re-score: every Node-derived file of every dataset
// (reference-outputs.jsonl, runs/*/oracle-outputs.jsonl) is recomputed from its Rust-derived input
// with the pinned Mermaid parser and compared byte-for-byte. The Rust half
// (crates/vox-cli/tests/it/eval_diagram_rescore.rs) re-derives the inputs and the tallies; neither
// half runs the other runtime. EVAL_DIAGRAM_BLESS=1 writes the outputs instead of comparing.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseError } from '../lib/mermaid-oracle.mjs';
import { oracleJsonl, oracleLine } from '../lib/mermaid-batch.mjs';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const DATASETS = ['crates/vox-cli/tests/fixtures/eval_diagram_synthetic'];
const BLESS = process.env.EVAL_DIAGRAM_BLESS === '1';

/** The (input, output) file pairs of one dataset, in chain order. */
function pairs(dir) {
  const out = [[join(dir, 'reference-inputs.jsonl'), join(dir, 'reference-outputs.jsonl')]];
  const runs = join(dir, 'runs');
  if (existsSync(runs)) {
    for (const run of readdirSync(runs).sort()) {
      out.push([join(runs, run, 'oracle-inputs.jsonl'), join(runs, run, 'oracle-outputs.jsonl')]);
    }
  }
  return out;
}

test('parseError: null for a valid flowchart, the parser message for a broken sequence', async () => {
  assert.equal(await parseError('flowchart TD\n  a --> b'), null);
  assert.match(await parseError('sequenceDiagram\n  A->>B hello'), /Parse error on line 2/);
});

test('oracleLine: an invalid text is not ok, carries the error and has no sets', async () => {
  const line = await oracleLine({ id: 'x', text: 'sequenceDiagram\n  A->>B hello', expected: { kind: 'sequence', actors: [], events: [] } });
  assert.deepEqual(Object.keys(line), ['id', 'ok', 'error', 'sets', 'match']);
  assert.equal(line.ok, false);
  assert.match(line.error, /Parse error on line 2/);
  assert.equal(line.sets, null);
  assert.equal(line.match, false);
});

test('oracleLine: match is null without expected, true on equal sets, false on different sets', async () => {
  const text = 'flowchart LR\n  a["A"] --> b["B"]';
  const plain = await oracleLine({ id: 'p', text });
  assert.equal(plain.ok, true);
  assert.equal(plain.match, null);
  assert.equal((await oracleLine({ id: 'm', text, expected: plain.sets })).match, true);
  const other = { ...plain.sets, direction: 'down' };
  assert.equal((await oracleLine({ id: 'n', text, expected: other })).match, false);
});

test('oracleJsonl: rejects a malformed line with its line number', async () => {
  await assert.rejects(oracleJsonl('{"id":"a","text":"flowchart TD"}\n{oops\n'), /line 2/);
});

for (const rel of DATASETS) {
  test(`re-score: Node-derived files of ${rel} reproduce byte-for-byte`, async () => {
    let checked = 0;
    for (const [input, output] of pairs(join(REPO_ROOT, rel))) {
      if (!existsSync(input)) {
        assert.ok(BLESS, `missing Rust-derived input ${input}`);
        continue;
      }
      const derived = await oracleJsonl(readFileSync(input, 'utf8'));
      if (BLESS) writeFileSync(output, derived);
      else assert.equal(derived, readFileSync(output, 'utf8'), `${output} differs from its re-derivation`);
      checked += 1;
    }
    assert.ok(checked > 0, `${rel}: no input/output pair was checked`);
  });
}
