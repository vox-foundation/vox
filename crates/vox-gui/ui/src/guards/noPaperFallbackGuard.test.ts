import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const TRANSPORT_DIR = join(import.meta.dirname, '..', 'transport');

function collectTransportTsFiles(dir: string, acc: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (entry === '__tests__') continue;
      collectTransportTsFiles(full, acc);
    } else if (entry.endsWith('.ts') && !entry.endsWith('.test.ts')) {
      acc.push(full);
    }
  }
  return acc.sort();
}

/**
 * Extract `{ tryBody, catchBody, line }` for every `try { ... } catch ... { ... }` block
 * in `source` using brace-balanced scanning.
 */
function extractTryCatchBlocks(
  source: string,
): Array<{ tryBody: string; catchBody: string; line: number }> {
  const results: Array<{ tryBody: string; catchBody: string; line: number }> = [];
  const readBalancedBlock = (startBraceIdx: number): { body: string; endIdx: number } | null => {
    if (source[startBraceIdx] !== '{') return null;
    let depth = 0;
    for (let i = startBraceIdx; i < source.length; i++) {
      if (source[i] === '{') depth++;
      else if (source[i] === '}') {
        depth--;
        if (depth === 0) {
          return { body: source.slice(startBraceIdx + 1, i), endIdx: i + 1 };
        }
      }
    }
    return null;
  };

  const tryRegex = /\btry\s*\{/g;
  let match: RegExpExecArray | null;
  while ((match = tryRegex.exec(source)) !== null) {
    const tryBrace = source.indexOf('{', match.index);
    const tryBlock = readBalancedBlock(tryBrace);
    if (!tryBlock) continue;
    const afterTry = source.slice(tryBlock.endIdx);
    const catchMatch = /^\s*catch(?:\s*\([^)]*\))?\s*\{/.exec(afterTry);
    if (!catchMatch) continue;
    const catchBrace = tryBlock.endIdx + catchMatch[0].lastIndexOf('{');
    const catchBlock = readBalancedBlock(catchBrace);
    if (!catchBlock) continue;
    const line = source.slice(0, match.index).split('\n').length;
    results.push({ tryBody: tryBlock.body, catchBody: catchBlock.body, line });
  }
  return results;
}

describe('Transport IPC honesty & god-object guard (AXIS-IPC-HONESTY-01 / AXIS-IPC-HONESTY-02)', () => {
  const files = collectTransportTsFiles(TRANSPORT_DIR);

  it('discovers split transport modules under src/transport/', () => {
    expect(files.length).toBeGreaterThanOrEqual(4);
  });

  it('contains zero _demo fake token literals across src/transport/*.ts', () => {
    const violations: string[] = [];
    const demoPattern = /vox_share_[^\n]*_demo|['"`]gsub_demo_1['"`]/;
    for (const full of files) {
      const rel = relative(TRANSPORT_DIR, full).replace(/\\/g, '/');
      const text = readFileSync(full, 'utf8');
      if (demoPattern.test(text)) {
        violations.push(rel);
      }
    }
    expect(violations).toEqual([]);
  });

  it('contains zero silent catch blocks swallowing invokeMcpTool errors', () => {
    const violations: string[] = [];
    for (const full of files) {
      const rel = relative(TRANSPORT_DIR, full).replace(/\\/g, '/');
      const text = readFileSync(full, 'utf8');
      if (/\/\/\s*Fall through/i.test(text)) {
        violations.push(`${rel}: contains "// Fall through" comment`);
      }
      for (const blk of extractTryCatchBlocks(text)) {
        const isJsonParseOnly =
          blk.tryBody.includes('JSON.parse(') && !blk.tryBody.includes('invokeMcpTool');
        const rethrows = /\bthrow\b/.test(blk.catchBody);
        if (!isJsonParseOnly && !rethrows) {
          violations.push(`${rel}:${blk.line} catch block does not rethrow`);
        }
      }
    }
    expect(violations).toEqual([]);
  });

  it('keeps every src/transport/*.ts file <= 480 non-blank lines and free of raw Tauri imports', () => {
    const sizeViolations: string[] = [];
    const importViolations: string[] = [];
    for (const full of files) {
      const rel = relative(TRANSPORT_DIR, full).replace(/\\/g, '/');
      const text = readFileSync(full, 'utf8');
      const nonBlankLines = text.split('\n').filter((line) => line.trim().length > 0).length;
      if (nonBlankLines > 480) {
        sizeViolations.push(`${rel} (${nonBlankLines} non-blank lines > 480)`);
      }
      if (/['"]@tauri-apps\/api\/(core|event)['"]/.test(text)) {
        importViolations.push(rel);
      }
    }
    expect(sizeViolations).toEqual([]);
    expect(importViolations).toEqual([]);
  });
});
