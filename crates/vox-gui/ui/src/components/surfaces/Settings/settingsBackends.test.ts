import { describe, it, expect } from 'vitest';
import { AXIS_LOCAL_SECTIONS, LOCAL_MACHINE_SECTIONS, sectionLockedRemotely } from './settingsBackends';
import { SECTIONS } from './SettingsView';

describe('settingsBackends', () => {
  it('classifies every Settings section exactly once', () => {
    for (const s of SECTIONS) {
      const inLocal = LOCAL_MACHINE_SECTIONS.has(s.id);
      const inAxis = AXIS_LOCAL_SECTIONS.has(s.id);
      expect(inLocal !== inAxis, `section ${s.id}`).toBe(true);
    }
  });

  it('locks local-machine sections only when remote', () => {
    expect(sectionLockedRemotely('secrets', 'remote')).toBe(true);
    expect(sectionLockedRemotely('secrets', 'local')).toBe(false);
    expect(sectionLockedRemotely('server', 'remote')).toBe(false);
    expect(sectionLockedRemotely('theme', 'remote')).toBe(false);
  });
});
