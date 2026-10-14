import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

// Same walk as components/surfaces/__guards__/surfaceHonesty.guard.test.ts (vitest runs from crates/vox-gui/ui).
function walk(d: string): string[] {
  return readdirSync(d).flatMap(n => {
    const p = join(d, n);
    return statSync(p).isDirectory() ? walk(p) : /\.tsx?$/.test(p) && !/\.test\.tsx?$/.test(p) ? [p] : [];
  });
}

describe('typed toasts', () => {
  it('no component takes an untyped pushToast', () => {
    const offenders = walk('src')
      .filter(f => /pushToast\??\s*:\s*\(\s*\w+\s*:\s*(any|unknown)\s*\)/.test(readFileSync(f, 'utf8')));
    expect(offenders).toEqual([]);
  });
});
