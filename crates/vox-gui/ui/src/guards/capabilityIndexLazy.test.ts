import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';

/**
 * D-06: the generated capability index (~784 KB) must never reach the startup bundle.
 * lib/capabilityIndexLoader.ts is its only runtime importer; everyone else may only
 * `import type`. This guard flags a static import, a re-export or a dynamic import anywhere else.
 */
const SRC_ROOT = join(import.meta.dirname, '..');
const MODULE = 'capabilityIndex.generated';
export const LOADER = 'lib/capabilityIndexLoader.ts';

export interface SourceFile {
  path: string;
  text: string;
}

/** Returns "path:line" for every runtime reference to the generated module outside the loader. */
export function findRuntimeImporters(files: SourceFile[]): string[] {
  const violations: string[] = [];
  for (const { path, text } of files) {
    if (path === LOADER) continue;
    const lines = text.split('\n');
    lines.forEach((line, i) => {
      if (!line.includes(MODULE)) return;
      if (/^\s*(\/\/|\*|\/\*)/.test(line)) return; // comment mention
      if (!line.includes('import(')) {
        // Walk back to the start of this (possibly multi-line) statement.
        let j = i;
        while (j > 0 && !/^\s*(import|export)\b/.test(lines[j])) j--;
        if (/^\s*import\s+type\b/.test(lines[j])) return;
      }
      violations.push(`${path}:${i + 1}`);
    });
  }
  return violations;
}

function collectSourceFiles(dir: string, acc: SourceFile[] = []): SourceFile[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      if (entry === 'guards' || entry === 'generated') continue;
      collectSourceFiles(full, acc);
    } else if (/\.(ts|tsx)$/.test(entry) && !/\.test\.tsx?$/.test(entry)) {
      acc.push({ path: relative(SRC_ROOT, full).split(sep).join('/'), text: readFileSync(full, 'utf8') });
    }
  }
  return acc;
}

const f = (path: string, text: string): SourceFile => ({ path, text });

describe('capability index lazy-load guard (D-06)', () => {
  it('flags a static named import', () => {
    const src = `import { CAPABILITY_INDEX } from '../generated/capabilityIndex.generated';`;
    expect(findRuntimeImporters([f('lib/other.ts', src)])).toEqual(['lib/other.ts:1']);
  });

  it('flags a re-export', () => {
    const src = `export { CAPABILITY_INDEX } from '../generated/capabilityIndex.generated';`;
    expect(findRuntimeImporters([f('lib/barrel.ts', src)])).toEqual(['lib/barrel.ts:1']);
  });

  it('flags a dynamic import outside the loader, even after an import type line', () => {
    const src = [
      `import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';`,
      `const m = await import('../generated/capabilityIndex.generated');`,
    ].join('\n');
    expect(findRuntimeImporters([f('hooks/sneaky.ts', src)])).toEqual(['hooks/sneaky.ts:2']);
  });

  it('flags a multi-line non-type import', () => {
    const src = `import {\n  CAPABILITY_INDEX,\n} from '../generated/capabilityIndex.generated';`;
    expect(findRuntimeImporters([f('lib/multi.ts', src)])).toEqual(['lib/multi.ts:3']);
  });

  it('accepts single- and multi-line import type', () => {
    const one = `import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';`;
    const multi = `import type {\n  CapabilityIndexEntry,\n} from '../generated/capabilityIndex.generated';`;
    expect(findRuntimeImporters([f('lib/a.ts', one), f('lib/b.ts', multi)])).toEqual([]);
  });

  it('accepts the loader itself', () => {
    const src = `pending ??= import('../generated/capabilityIndex.generated');`;
    expect(findRuntimeImporters([f(LOADER, src)])).toEqual([]);
  });

  it('the real src tree has no runtime importer besides the loader', () => {
    const files = collectSourceFiles(SRC_ROOT);
    expect(files.some((x) => x.path === LOADER)).toBe(true); // the walk actually saw the loader
    expect(findRuntimeImporters(files)).toEqual([]);
  });
});
