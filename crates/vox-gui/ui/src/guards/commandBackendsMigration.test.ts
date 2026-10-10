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

describe('plan 07 models group', () => {
  it('models is daemon-class and its host-only commands live in models_host', () => {
    expect(moduleClass('models')).toBe('daemon');
    expect(moduleClass('models_host')).toBe('host');
    expect(moduleClass('routing_explain')).toBe('daemon');
  });
});

describe('plan 07 chat group', () => {
  it('chat and chat_turn reach sessions through the daemon, not a GUI db pool', () => {
    expect(moduleClass('chat')).toBe('daemon');
    expect(moduleClass('chat_turn')).toBe('daemon');
  });
});

describe('plan 07 preferences', () => {
  it('preferences stays shell-class: client-side when remote, loopback daemon ops locally (D1)', () => {
    expect(moduleClass('preferences')).toBe('shell');
  });
});

describe('plan 07 knowledge/memory/search group', () => {
  it('knowledge, memory and search are daemon-class; opening a hit stays on the host', () => {
    for (const m of ['knowledge', 'memory', 'search']) expect(moduleClass(m)).toBe('daemon');
    expect(moduleClass('locator')).toBe('host');
  });
});

describe('plan 07 research group', () => {
  it('research, scientia and harness-issue modules are daemon-class; local doc/sandbox I/O stays on the host', () => {
    for (const m of ['scientia', 'scientia_review', 'research', 'search_probe', 'harness_issues']) {
      expect(moduleClass(m)).toBe('daemon');
    }
    expect(moduleClass('research_host')).toBe('host');
  });
});
