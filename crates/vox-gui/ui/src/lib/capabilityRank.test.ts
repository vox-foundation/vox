import { describe, it, expect } from 'vitest';
import { CAPABILITY_FIXTURE } from '../test-fixtures/capabilityIndexFixture';
import { normalizeQuery, rankCapabilities, toRankEntries } from './capabilityRank';

const entries = toRankEntries(CAPABILITY_FIXTURE);

describe('capabilityRank', () => {
  it('"vox check" ranks cli.check first at tier 0 (leading "vox " stripped, equals the path alias)', () => {
    const hits = rankCapabilities(entries, 'vox check');
    expect(hits[0]?.row.id).toBe('cli.check');
    expect(hits[0]?.tier).toBe(0);
  });

  it('"fabrica f" is a tier-1 prefix hit on the spaced cli path of cli.fabrica.fmt', () => {
    const hit = rankCapabilities(entries, 'fabrica f').find((h) => h.row.id === 'cli.fabrica.fmt');
    expect(hit?.tier).toBe(1);
  });

  it('empty and whitespace queries return nothing; long queries are bounded to 256 chars', () => {
    expect(rankCapabilities(entries, '')).toEqual([]);
    expect(rankCapabilities(entries, '   ')).toEqual([]);
    expect(normalizeQuery('x'.repeat(10_000)).length).toBeLessThanOrEqual(256);
    expect(normalizeQuery('  VOX Check ')).toBe('check');
  });

  it('the kinds option restricts which row kinds are returned', () => {
    const hits = rankCapabilities(entries, 'check', { kinds: ['cli'] });
    expect(hits.length).toBeGreaterThan(0);
    expect(hits.every((h) => h.row.kind === 'cli')).toBe(true);
  });
});
