import { describe, it, expect } from 'vitest';
import { CAPABILITY_FIXTURE, row } from '../test-fixtures/capabilityIndexFixture';
import { jaroWinkler, kindsForMode, noteRecentCapability, normalizeQuery, rankCapabilities, toRankEntries } from './capabilityRank';

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

describe('jaroWinkler (parity with strsim)', () => {
  it('matches the strsim reference values', () => {
    expect(jaroWinkler('martha', 'marhta')).toBeCloseTo(0.9611, 4);
    expect(jaroWinkler('dwayne', 'duane')).toBeCloseTo(0.84, 4);
    expect(jaroWinkler('dixon', 'dicksonx')).toBeCloseTo(0.8133, 4);
  });
  it('is 1 for identical strings and 0 when one side is empty', () => {
    expect(jaroWinkler('mesh', 'mesh')).toBe(1);
    expect(jaroWinkler('', 'mesh')).toBe(0);
    expect(jaroWinkler('mesh', '')).toBe(0);
  });
});

describe('rankCapabilities tiers', () => {
  it('"vox fabrica fmt" ranks cli.fabrica.fmt first at tier 0 with no tier-3 noise', () => {
    const hits = rankCapabilities(entries, 'vox fabrica fmt');
    expect(hits[0]?.row.id).toBe('cli.fabrica.fmt');
    expect(hits[0]?.tier).toBe(0);
    expect(hits.some((h) => h.tier === 3)).toBe(false);
  });
  it('"fabrica" is a tier-1 prefix hit', () => {
    expect(rankCapabilities(entries, 'fabrica').find((h) => h.row.id === 'cli.fabrica.fmt')?.tier).toBe(1);
  });
  it('"fab fm" is a tier-2 token-prefix hit', () => {
    expect(rankCapabilities(entries, 'fab fm').find((h) => h.row.id === 'cli.fabrica.fmt')?.tier).toBe(2);
  });
  it('"mesh" ranks surface.mesh first at tier 0 with no tier-3 hits', () => {
    const hits = rankCapabilities(entries, 'mesh');
    expect(hits[0]?.row.id).toBe('surface.mesh');
    expect(hits[0]?.tier).toBe(0);
    expect(hits.some((h) => h.tier === 3)).toBe(false);
  });
  it('"mehs" falls back to a tier-3 typo hit on surface.mesh', () => {
    expect(rankCapabilities(entries, 'mehs').find((h) => h.row.id === 'surface.mesh')?.tier).toBe(3);
  });
  it('Jaro-Winkler never runs below 3 characters', () => {
    expect(rankCapabilities(entries, 'cx')).toEqual([]);
    expect(rankCapabilities(entries, 'mh')).toEqual([]);
  });
  it('surfaces are found through extra (lexicon) aliases', () => {
    const withLa = toRankEntries(CAPABILITY_FIXTURE, { 'surface.mesh': ['Reticulum'] });
    expect(rankCapabilities(withLa, 'reticulum')[0]?.row.id).toBe('surface.mesh');
    const viaFn = toRankEntries(CAPABILITY_FIXTURE, (r) => (r.id === 'surface.mesh' ? ['Reticulum'] : []));
    expect(rankCapabilities(viaFn, 'reticulum')[0]?.row.id).toBe('surface.mesh');
  });
});

describe('rankCapabilities tie-breaks (deterministic)', () => {
  const ids = (rows: ReturnType<typeof row>[], opts = {}) =>
    rankCapabilities(toRankEntries(rows), 'dupe', opts).map((h) => h.row.id);

  it('a dedicated surface (viewKey) beats a row with none', () => {
    const rows = [row({ id: 'cli.a', kind: 'cli', title: 'dupe' }), row({ id: 'surface.b', kind: 'surface', title: 'dupe', viewKey: 'zz' })];
    expect(ids(rows)).toEqual(['surface.b', 'cli.a']);
  });
  it('recommended beats advanced when nothing else differs', () => {
    const rows = [row({ id: 'cli.a', kind: 'cli', title: 'dupe', tier: 'advanced' }), row({ id: 'cli.b', kind: 'cli', title: 'dupe', tier: 'recommended' })];
    expect(ids(rows)).toEqual(['cli.b', 'cli.a']);
  });
  it('recent use beats id order; otherwise id ascending, independent of input order', () => {
    const a = row({ id: 'cli.a', kind: 'cli', title: 'dupe' });
    const b = row({ id: 'cli.b', kind: 'cli', title: 'dupe' });
    expect(ids([a, b], { recent: ['cli.b'] })).toEqual(['cli.b', 'cli.a']);
    expect(ids([a, b])).toEqual(['cli.a', 'cli.b']);
    expect(ids([b, a])).toEqual(['cli.a', 'cli.b']);
  });
  it('noteRecentCapability keeps a bounded most-recent-first list', async () => {
    const { recentCapabilities } = await import('./capabilityRank');
    for (let i = 0; i < 25; i++) noteRecentCapability(`cli.r${i}`);
    noteRecentCapability('cli.r3');
    const r = recentCapabilities();
    expect(r.length).toBe(20);
    expect(r[0]).toBe('cli.r3');
    expect(new Set(r).size).toBe(20);
  });
});

describe('kindsForMode and layer queries', () => {
  it('maps prefix modes to row kinds', () => {
    expect([...kindsForMode('default')].sort()).toEqual(['cli', 'crate', 'doc', 'mcp_tool', 'skill', 'surface']);
    expect(kindsForMode('commands')).toEqual(['cli']);
    expect([...kindsForMode('skills')].sort()).toEqual(['doc', 'skill']);
    expect(kindsForMode('crates')).toEqual(['crate']);
    expect(kindsForMode('agents')).toEqual([]);
  });
  it('lists the crates of a layer sorted by id for "L3" and "layer 3"', () => {
    for (const q of ['L3', 'layer 3', 'Layer  3']) {
      const hits = rankCapabilities(entries, q, { kinds: ['crate'] });
      expect(hits.map((h) => h.row.id)).toEqual(['crate.vox-actor-runtime', 'crate.vox-orchestrator']);
    }
    expect(rankCapabilities(entries, 'L9', { kinds: ['crate'] })).toEqual([]);
  });
  it('kinds restricted to crates returns only crates', () => {
    const hits = rankCapabilities(entries, 'vox-orch', { kinds: ['crate'] });
    expect(hits.map((h) => h.row.id)).toEqual(['crate.vox-orchestrator']);
  });
  it('an empty query yields no rows in crates mode', () => {
    expect(rankCapabilities(entries, '', { kinds: ['crate'] })).toEqual([]);
  });
});

describe('rankCapabilities perf guard (D-19)', () => {
  it('ranks 1600 rows well under the per-keystroke budget (median of 7 < 25 ms)', () => {
    const kinds = ['cli', 'mcp_tool', 'crate', 'skill'] as const;
    const rows = Array.from({ length: 1600 }, (_, i) =>
      row({ id: `${kinds[i % 4]}.item-${i}.sub`, kind: kinds[i % 4], title: `Item ${i} Sub Thing`, cli: i % 4 === 0 ? `vox item-${i} sub` : null, mcp: i % 4 === 1 ? `vox_item_${i}` : null }),
    );
    const big = toRankEntries(rows);
    const medians: Record<string, number> = {};
    for (const q of ['vox fabrica fmt', 'mesh', 'fmt']) {
      const times: number[] = [];
      for (let k = 0; k < 7; k++) {
        const t0 = performance.now();
        rankCapabilities(big, q);
        times.push(performance.now() - t0);
      }
      times.sort((a, b) => a - b);
      medians[q] = times[3];
      expect(times[3]).toBeLessThan(25);
    }
    console.log('rank perf medians (ms):', JSON.stringify(medians));
  });
});
