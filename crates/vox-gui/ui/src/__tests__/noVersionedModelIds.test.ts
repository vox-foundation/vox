import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { VERSIONED_CLOUD_MODEL_ID } from '../lib/modelFamily';

// src/__tests__ -> src -> ui
const SELF = fileURLToPath(import.meta.url);
const UI_ROOT = resolve(dirname(SELF), '../..');
const SCAN_DIRS = ['src', 'e2e'];
const EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.mjs', '.json']);
// `fixtures` hold recorded run data (research traces name the real model that answered), not mocks of the
// catalog; the two research-trace tests assert on exactly that recorded data, so they are exempt with it.
const SKIP_DIRS = new Set(['node_modules', 'screens', 'fixtures']);
const RECORDED_TRACE_TESTS = new Set([
  'src/components/surfaces/Chat/ResearchTracePanel.test.tsx',
  'e2e/research-trace-panel.spec.ts',
]);
/**
 * Local model names may be literal (AGENTS.md: local MENS revisions are exempt): Ollama, MENS runs,
 * mesh- and locally-served models, and the MENS fine-tuning base.
 */
const LOCAL_PREFIXES = ['ollama/', 'mens/', 'local/', 'mesh/', 'qwen/qwen3-8b'];
const TOKEN_CHAR = /[A-Za-z0-9_./:@\\-]/;

/**
 * The whole id-like token around a regex hit (so a match inside `ollama/…` is judged by its prefix). Test regex
 * literals escape slashes (`/ollama\/llama3/i`), so backslashes and a leading `/` delimiter are dropped first.
 */
function tokenAt(line: string, index: number): string {
  let start = index;
  let end = index;
  while (start > 0 && TOKEN_CHAR.test(line[start - 1])) start -= 1;
  while (end < line.length && TOKEN_CHAR.test(line[end])) end += 1;
  return line.slice(start, end).replace(/\\/g, '').replace(/^\/+/, '');
}

function findVersionedIds(text: string): string[] {
  const re = new RegExp(VERSIONED_CLOUD_MODEL_ID.source, 'gi');
  const hits: string[] = [];
  for (const line of text.split('\n')) {
    for (const m of line.matchAll(re)) {
      const token = tokenAt(line, m.index ?? 0);
      if (LOCAL_PREFIXES.some((p) => token.toLowerCase().startsWith(p))) continue;
      hits.push(token);
    }
  }
  return hits;
}

function walk(dir: string, out: string[]): void {
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name)) continue;
    const full = join(dir, name);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (EXTENSIONS.has(extname(name))) out.push(full);
  }
}

describe('findVersionedIds (guard self-test)', () => {
  it('flags versioned cloud ids, including bare mock shortnames, and exempts local model names', () => {
    const sample = [
      "id: 'anthropic/claude-opus-4.7',",
      "active: 'opus-4-8',",
      "family: 'anthropic/claude-opus',",
      "local: 'ollama/llama3', base: 'Qwen/Qwen3-8B', run: 'mens/runs/qwen3_27b_metal_check/quant_q6_k',",
      "other: 'openai/gpt-5.2-mini', deep: 'deepseek/deepseek-v4-flash', o: 'openai/o3-mini',",
    ].join('\n');
    expect(findVersionedIds(sample)).toEqual([
      'anthropic/claude-opus-4.7',
      'opus-4-8',
      'openai/gpt-5.2-mini',
      'deepseek/deepseek-v4-flash',
      'openai/o3-mini',
    ]);
  });

  it('reads ids inside escaped regex literals: exempts a local one, flags a cloud one', () => {
    const sample = [
      String.raw`expect(screen.queryByRole('option', { name: /ollama\/llama3/i })).toBeNull();`,
      String.raw`expect(screen.queryByRole('option', { name: /openai\/gpt-5\.2-mini/i })).toBeNull();`,
    ].join('\n');
    expect(findVersionedIds(sample)).toEqual(['openai/gpt-5.2-mini/i']);
  });
});

describe('no versioned cloud model ids in GUI source, tests or mocks', () => {
  it('src/ and e2e/ hold only family keys or exempt local names', () => {
    const files: string[] = [];
    for (const d of SCAN_DIRS) walk(join(UI_ROOT, d), files);
    const offenders = files
      .filter((f) => resolve(f) !== resolve(SELF) && !RECORDED_TRACE_TESTS.has(relative(UI_ROOT, f)))
      .flatMap((f) => findVersionedIds(readFileSync(f, 'utf8')).map((id) => `${relative(UI_ROOT, f)}: ${id}`));
    expect(offenders).toEqual([]);
  });
});
