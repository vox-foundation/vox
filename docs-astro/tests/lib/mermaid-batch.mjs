// Batch Mermaid oracle for the diagram-authoring eval: JSONL `{id, text, expected?}` in, JSONL
// `{id, ok, error, sets, match}` out, in input order. Texts arrive only on stdin (never argv or a
// shell). Test-only, like mermaid-oracle.mjs: the pinned real parser, no new dependency.
import { pathToFileURL } from 'node:url';
import { canon, parseError, sameSets } from './mermaid-oracle.mjs';

/** Judge one text: parse error (feedback), canonical sets when it parses, match against `expected`. */
export async function oracleLine({ id, text, expected }) {
  const error = await parseError(text);
  let sets = null;
  if (error === null) {
    try {
      sets = await canon(text);
    } catch (e) {
      sets = { unsupported: String(e?.message ?? e) };
    }
  }
  let match = null;
  if (expected != null) {
    match = false;
    if (sets !== null && !sets.unsupported) {
      try {
        sameSets(sets, expected, id);
        match = true;
      } catch {
        match = false;
      }
    }
  }
  return { id, ok: error === null, error, sets, match };
}

/** Run every non-blank JSONL line through `oracleLine`, sequentially; one JSON line out per line in. */
export async function oracleJsonl(text) {
  let out = '';
  const lines = text.split('\n');
  for (let i = 0; i < lines.length; i++) {
    if (!lines[i].trim()) continue;
    let input;
    try {
      input = JSON.parse(lines[i]);
    } catch (e) {
      throw new Error(`line ${i + 1}: ${e.message}`);
    }
    if (typeof input?.id !== 'string' || typeof input?.text !== 'string') {
      throw new Error(`line ${i + 1}: expected {"id": string, "text": string}`);
    }
    out += JSON.stringify(await oracleLine(input)) + '\n';
  }
  return out;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  let stdin = '';
  process.stdin.setEncoding('utf8');
  for await (const chunk of process.stdin) stdin += chunk;
  try {
    const out = await oracleJsonl(stdin);
    // Exit once flushed: the jsdom window would otherwise keep the process alive.
    process.stdout.write(out, () => process.exit(0));
  } catch (e) {
    process.stderr.write(`mermaid-batch: ${e.message}\n`);
    process.exit(2);
  }
}
