import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { scanSource } from './honestyScan';
import { HIDDEN_ALLOWLIST } from './honestyScan.allowlist';
import { SURFACE_REGISTRY } from '../../../generated/surfaceRegistry.generated';

const ROOT = 'src/components/surfaces';

// Skip tests, __guards__, AND *.unfinished.tsx — the HIDE mechanism moves
// not-yet-wired markup into a sibling `<Name>.unfinished.tsx` that shipped code
// never imports.
function walk(d: string): string[] {
  return readdirSync(d).flatMap(n => {
    if (n === '__guards__' || n === '__tests__') return [];
    const p = join(d, n);
    if (statSync(p).isDirectory()) return walk(p);
    const isShippedSource =
      (p.endsWith('.tsx') || p.endsWith('.ts')) &&
      !p.endsWith('.test.tsx') &&
      !p.endsWith('.test.ts') &&
      !p.endsWith('.unfinished.tsx');
    return isShippedSource ? [p] : [];
  });
}

const allowed = (f: string, l: number) =>
  HIDDEN_ALLOWLIST.some(a => f.endsWith(a.file) && a.line === l);

describe('surface honesty guard (REAL-04)', () => {
  it('covers all 32 registered GUI surfaces and finds zero placeholder text, dead handlers, or unguarded mocks in shipped surfaces', () => {
    const registeredViews = SURFACE_REGISTRY.filter(e => e.viewKey != null).map(e => e.viewKey as string);
    expect(registeredViews.length).toBeGreaterThanOrEqual(32);

    const files = walk(ROOT);
    expect(files.length).toBeGreaterThanOrEqual(32);

    const violations = files
      .flatMap(f => scanSource(f, readFileSync(f, 'utf8')))
      .filter(v => !allowed(v.file, v.line));
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
  });
});
