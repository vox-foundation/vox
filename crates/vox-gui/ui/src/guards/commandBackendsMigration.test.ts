import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// Plan 07: each migrated module must be classified `daemon` in the command-backend manifest.
const MANIFEST = readFileSync(
  join(import.meta.dirname, '../../../../../contracts/gui/command-backends.v1.yaml'),
  'utf8',
);

function moduleClass(module: string): string | null {
  const m =
    MANIFEST.match(new RegExp(`- \\{ module: ${module}, class: (\\w+)`)) ??
    MANIFEST.match(new RegExp(`- module: ${module}\\n\\s+class: (\\w+)`));
  return m ? m[1] : null;
}

describe('plan 07 db-command migration', () => {
  it('pattern: model_active is daemon-class', () => {
    expect(moduleClass('model_active')).toBe('daemon');
  });
});
