import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const SRC_ROOT = join(import.meta.dirname, '..');

/** Infrastructure files that must route IPC through VoxTransport (Phase 0B). */
const MUST_USE_TRANSPORT = [
  'lib/consoleBridge.ts',
  'hooks/usePersistedDbState.ts',
  'components/layout/Omnibar.tsx',
  'main.tsx',
];

function collectTsFiles(dir: string, acc: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (entry === 'guards') continue;
      collectTsFiles(full, acc);
    } else if (/\.(ts|tsx)$/.test(entry) && !entry.endsWith('.test.ts') && !entry.endsWith('.test.tsx')) {
      acc.push(full);
    }
  }
  return acc;
}

describe('IPC boundaries (Phase 0B)', () => {
  it('infrastructure files do not import @tauri-apps/api/core directly', () => {
    const violations: string[] = [];
    for (const rel of MUST_USE_TRANSPORT) {
      const full = join(SRC_ROOT, rel);
      const text = readFileSync(full, 'utf8');
      if (/['"]@tauri-apps\/api\/core['"]/.test(text)) {
        violations.push(rel);
      }
    }
    expect(violations).toEqual([]);
  });

  it('only transport.ts imports invoke in the IPC hub layer (tracked allowlist shrinks per wave)', () => {
    const ALLOW_DIRECT_INVOKE = new Set(['transport.ts']);

    const unexpected: string[] = [];
    for (const full of collectTsFiles(SRC_ROOT)) {
      const rel = relative(SRC_ROOT, full).replace(/\\/g, '/');
      const text = readFileSync(full, 'utf8');
      if (!/['"]@tauri-apps\/api\/core['"]/.test(text)) continue;
      if (!ALLOW_DIRECT_INVOKE.has(rel)) {
        unexpected.push(rel);
      }
    }
    expect(unexpected).toEqual([]);
  });

  it('only transport.ts imports listen from @tauri-apps/api/event', () => {
    const ALLOW_DIRECT_LISTEN = new Set(['transport.ts']);

    const unexpected: string[] = [];
    for (const full of collectTsFiles(SRC_ROOT)) {
      const rel = relative(SRC_ROOT, full).replace(/\\/g, '/');
      const text = readFileSync(full, 'utf8');
      if (!/['"]@tauri-apps\/api\/event['"]/.test(text)) continue;
      if (!ALLOW_DIRECT_LISTEN.has(rel)) {
        unexpected.push(rel);
      }
    }
    expect(unexpected).toEqual([]);
  });
});
