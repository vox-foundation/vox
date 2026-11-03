import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const EXPLORER_DIR = 'src/components/surfaces/Explorer';

describe('explorerReadOnly guard (D-10a, DISC-08)', () => {
  it('no non-test file in Explorer imports transport or references executeCommand / runCli', () => {
    const files = readdirSync(EXPLORER_DIR).filter(
      (f) => (f.endsWith('.ts') || f.endsWith('.tsx')) && !f.includes('.test.'),
    );
    expect(files.length).toBeGreaterThan(0);

    const violations: string[] = [];
    for (const name of files) {
      const fullPath = join(EXPLORER_DIR, name);
      const src = readFileSync(fullPath, 'utf8');
      if (/from\s+['"][^'"]*transport['"]/.test(src)) {
        violations.push(`${name}: imports from transport module`);
      }
      if (/\b(executeCommand|runCli|runCliVerb)\b/.test(src)) {
        violations.push(`${name}: references command-execution identifier`);
      }
    }

    expect(violations).toEqual([]);
  });
});
