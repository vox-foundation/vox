import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = resolve(__filename, '..');
const REPO_ROOT = resolve(__dirname, '../../..');
const RECORD_PATH = join(REPO_ROOT, 'contracts/documentation/tutorial-verification.v1.json');
const TUTORIALS_DIR = join(REPO_ROOT, 'docs/src/tutorials');

export function skipsWithoutReason(markdown) {
  const lines = markdown.split(/\r?\n/);
  const hits = [];
  let inVoxFence = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (trimmed.startsWith('```')) {
      const rest = trimmed.replace(/`/g, '').trim();
      if (!inVoxFence && rest === 'vox') {
        inVoxFence = true;
      } else if (inVoxFence) {
        inVoxFence = false;
      }
    } else if (inVoxFence) {
      if (trimmed.startsWith('// vox:skip')) {
        const afterMarker = trimmed.slice('// vox:skip'.length).trim();
        const stripped = afterMarker.replace(/^[:—\-]\s*/, '').trim();
        if (!/[a-zA-Z]/.test(stripped)) {
          hits.push({ line: i + 1, text: line });
        }
      }
    }
  }
  return hits;
}

test('skipsWithoutReason flags bare skip and permits reasoned skip', () => {
  const bareMarkdown = '```vox\n// vox:skip\nfn main() {}\n```';
  assert.equal(skipsWithoutReason(bareMarkdown).length, 1);

  const bareWithPunctuation = '```vox\n// vox:skip : \nfn main() {}\n```';
  assert.equal(skipsWithoutReason(bareWithPunctuation).length, 1);

  const reasonedMarkdown = '```vox\n// vox:skip — excerpt depends on out-of-file types\nfn main() {}\n```';
  assert.equal(skipsWithoutReason(reasonedMarkdown).length, 0);
});

test('tutorial files have no unreasoned vox:skip lines', () => {
  const files = readdirSync(TUTORIALS_DIR).filter((f) => f.endsWith('.md')).sort();
  for (const file of files) {
    const fullPath = join(TUTORIALS_DIR, file);
    const content = readFileSync(fullPath, 'utf8');
    const hits = skipsWithoutReason(content);
    assert.equal(hits.length, 0, `Unreasoned vox:skip found in ${file} at line ${hits[0]?.line}: ${hits[0]?.text}`);
  }
});

test('tutorial verification record parses and matches git blob hashes', () => {
  assert.ok(existsSync(RECORD_PATH), `Record file does not exist: ${RECORD_PATH}`);
  const record = JSON.parse(readFileSync(RECORD_PATH, 'utf8'));

  assert.equal(record['x-vox-version'], 1);
  assert.ok(record.verified_at_commit && /^[0-9a-f]{40}$/.test(record.verified_at_commit));

  const tutorialFiles = readdirSync(TUTORIALS_DIR)
    .filter((f) => f.endsWith('.md'))
    .sort()
    .map((f) => `docs/src/tutorials/${f}`);

  assert.equal(record.tutorials.length, tutorialFiles.length);

  const checkKeys = ['commands_in_registry', 'snippets_compile', 'install_versions_match', 'links_resolve'];

  for (const entry of record.tutorials) {
    assert.ok(tutorialFiles.includes(entry.path), `Unexpected tutorial path in record: ${entry.path}`);

    for (const key of checkKeys) {
      assert.ok(key in entry.checks, `Missing check key ${key} in ${entry.path}`);
      assert.equal(entry.checks[key], 'pass', `Check ${key} failed in tutorial ${entry.path}`);
    }

    const actualBlobSha = execFileSync('git', ['hash-object', entry.path], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
    }).trim();

    assert.equal(
      entry.blob_sha,
      actualBlobSha,
      `tutorial changed since verification — run vox run --mode interp scripts/docs/tutorial-verify.vox (${entry.path})`
    );
  }
});
