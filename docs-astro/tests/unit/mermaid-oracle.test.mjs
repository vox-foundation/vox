// Three-way Mermaid oracle: the PINNED real parser judges fixtures that the Rust crate wrote.
//   C(B) = canon(original block)   R(B) = Rust sets from the imported IR   C(E) = canon(exported text)
// Asserted: accepts(E), C(B) == R(B) (importer), C(E) == R(B), C(E) == C(B) (exporter, lossless).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { accepts, canon, decodeLabel, sameSets } from '../lib/mermaid-oracle.mjs';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const FIXTURES = join(REPO_ROOT, 'crates/vox-diagram/tests/fixtures');
const read = (...p) => readFileSync(join(FIXTURES, ...p), 'utf8');
const stems = (dir, suffix) =>
  readdirSync(join(FIXTURES, dir)).filter((f) => f.endsWith(suffix)).map((f) => f.slice(0, -suffix.length)).sort();

test('helper: accepts rejects a backtick-first label and accepts a minimal block', async () => {
  assert.equal(await accepts('flowchart TD\n  A["`x` y"]\n'), false);
  assert.equal(await accepts('flowchart TD\n    a --> b\n'), true);
});

test('helper: accepts is true for a gantt block the importer refuses on purpose', async () => {
  assert.equal(await accepts('gantt\n    title Plan\n    section S\n    Task :a1, 2024-01-01, 3d\n'), true);
});

test('helper: decodeLabel decodes br tags, placeholders and entities, and throws on unknown ones', () => {
  assert.equal(decodeLabel('a<br/>b'), 'a\nb');
  assert.equal(decodeLabel('a<br>b'), 'a\nb');
  assert.equal(decodeLabel('ﬂ°quot¶ßhiﬂ°quot¶ß'), '"hi"');
  assert.equal(decodeLabel('ﬂ°°35¶ß'), '#');
  assert.equal(decodeLabel('a &amp; b &lt;'), 'a & b <');
  assert.equal(decodeLabel('&#35;&#x41;'), '#A');
  assert.throws(() => decodeLabel('&bogus;'), /unknown entity/);
});

test('canon: flowchart with a titled subgraph, an implicit node, a diamond and an edge label', async () => {
  const src = 'flowchart LR\n  subgraph s1 ["Group One"]\n    a["A"]\n    d{"D"}\n  end\n  a -->|t| b\n  d --> a\n';
  const g = ['Group One'];
  sameSets(
    await canon(src),
    {
      kind: 'graph',
      direction: 'right',
      containers: [g],
      nodes: [['A', 'rect', g], ['D', 'diamond', g], ['b', 'rect', []]],
      edges: [[['A', 'rect', g], ['b', 'rect', []], 't'], [['D', 'diamond', g], ['A', 'rect', g], '']],
    },
    'flowchart canon',
  );
});

test('canon: stateDiagram-v2 maps [*] to Start/End', async () => {
  const src = 'stateDiagram-v2\n    [*] --> Idle\n    Idle --> Busy: start job\n    Busy --> Busy: retry\n    Busy --> [*]\n';
  const S = ['Start', 'start', []];
  const I = ['Idle', 'rect', []];
  const B = ['Busy', 'rect', []];
  const E = ['End', 'end', []];
  sameSets(
    await canon(src),
    { kind: 'graph', direction: 'down', containers: [], nodes: [S, I, B, E], edges: [[S, I, ''], [I, B, 'start job'], [B, B, 'retry'], [B, E, '']] },
    'state canon',
  );
});

test('canon: state composite and sequence loops throw instead of skipping', async () => {
  await assert.rejects(canon('stateDiagram-v2\n    state Outer {\n        A --> B\n    }\n'), /unsupported state/);
  await assert.rejects(canon('gantt\n    title x\n'), /unsupported diagram header/);
});

test('canon: sequenceDiagram actors, messages and notes in order', async () => {
  const src = 'sequenceDiagram\n    participant A as Alice\n    participant B\n    A->>B: hello\n    B-->>A: hi\n    Note over A,B: both\n    Note over A: one\n';
  assert.deepEqual(await canon(src), {
    kind: 'sequence',
    actors: ['Alice', 'B'],
    events: [['message', 0, 1, false, 'hello'], ['message', 1, 0, true, 'hi'], ['note', [0, 1], 'both'], ['note', [0], 'one']],
  });
});

/** One test per fixture; a directory is skipped only when absent and fails when it holds nothing. */
function family(dir, suffix, body) {
  if (!existsSync(join(FIXTURES, dir))) return;
  const names = stems(dir, suffix);
  if (names.length === 0) test(`${dir}: has fixtures`, () => assert.fail(`${dir} exists but has no ${suffix} fixtures`));
  for (const stem of names) test(`${dir}/${stem}`, () => body(stem));
}

/** Three-way assertions shared by every family that starts from an original block. */
async function threeWay(original, exported, rust) {
  assert.equal(await accepts(exported), true, 'exported text must be accepted by Mermaid');
  sameSets(await canon(original), rust, 'C(B) == R(B)');
  sameSets(await canon(exported), rust, 'C(E) == R(B)');
  sameSets(await canon(exported), await canon(original), 'C(E) == C(B)');
}

family('import-cases-roundtrip', '.sets.json', async (stem) => {
  await threeWay(
    read('import-cases', `${stem}.mmd`),
    read('import-cases-roundtrip', `${stem}.export.mmd`),
    JSON.parse(read('import-cases-roundtrip', `${stem}.sets.json`)),
  );
});

// Frozen corpus (plan 84-05 writes corpus-roundtrip/): every block is either round-tripped or
// listed in unsupported.json, never both.
if (existsSync(join(FIXTURES, 'corpus-roundtrip'))) {
  const listed = new Map(
    JSON.parse(read('corpus-roundtrip', 'unsupported.json')).map((e) => [e.file.replace(/\.mmd$/, ''), e.unsupported]),
  );
  const done = new Set(stems('corpus-roundtrip', '.sets.json'));
  for (const stem of stems('corpus', '.mmd')) {
    test(`corpus-roundtrip/${stem}`, async () => {
      assert.ok(done.has(stem) !== listed.has(stem), `${stem} must have sets.json or an unsupported.json entry, exactly one`);
      const original = read('corpus', `${stem}.mmd`);
      if (listed.has(stem)) {
        assert.equal(existsSync(join(FIXTURES, 'corpus-roundtrip', `${stem}.export.mmd`)), false, 'listed block has no export');
        const markdown = listed.get(stem).some((u) => u.construct === 'markdown-string');
        assert.equal(await accepts(original), !markdown, markdown ? 'markdown-string block must be rejected by Mermaid' : 'refused block must be valid Mermaid');
        return;
      }
      await threeWay(original, read('corpus-roundtrip', `${stem}.export.mmd`), JSON.parse(read('corpus-roundtrip', `${stem}.sets.json`)));
    });
  }
}

// Hostile exporter inputs (plan 84-02 writes export-hostile/): the export alone is judged.
family('export-hostile', '.sets.json', async (stem) => {
  const exported = read('export-hostile', `${stem}.export.mmd`);
  assert.equal(await accepts(exported), true, 'exported text must be accepted by Mermaid');
  sameSets(await canon(exported), JSON.parse(read('export-hostile', `${stem}.sets.json`)), 'C(E) == R(IR)');
});
