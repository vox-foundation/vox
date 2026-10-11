import { describe, it, expect } from 'vitest';
import { buildOmnibarFacets, FACET_CAP, queryForFederatedSearch, shouldIncludeDocsInOmnibar, type OmnibarSources } from './omnibarFacets';
import { searchFederatedIndex } from './federatedSearchIndex';
import type { FederatedIndexEntry } from './federatedSearchIndex';
import type { UnifiedHit } from '../components/surfaces/Search/searchHelpers';
import type { ContentManifestEntry } from '../hooks/useContentManifest';
import type { SearchableHit } from './searchableRegistry';
import type { RankHit } from './capabilityRank';
import { CAPABILITY_FIXTURE, row } from '../test-fixtures/capabilityIndexFixture';

const surface = (vk: string, label: string): FederatedIndexEntry => ({
  kind: 'surface',
  id: `surface:${vk}`,
  label,
  detail: 'Runs',
  payload: { type: 'surface', viewKey: vk },
});

const docEntry = (path: string, title: string, score?: number): FederatedIndexEntry => ({
  kind: 'doc',
  id: `doc:${path}`,
  label: title,
  detail: '',
  score,
  payload: { type: 'doc', path },
});

const policyEntry = (name: string, score: number): FederatedIndexEntry => ({
  kind: 'policy',
  id: `policy:${name}`,
  label: name,
  detail: 'active',
  score,
  payload: { type: 'policy', policyId: name },
});

const settingEntry = (label: string, score: number): FederatedIndexEntry => ({
  kind: 'setting',
  id: 'setting:max_agents',
  label,
  detail: 'Agents',
  score,
  payload: { type: 'setting', section: 'agents', settingId: 'max_agents' },
});

const skillEntry = (name: string, score: number): FederatedIndexEntry => ({
  kind: 'skill',
  id: `skill:${name}`,
  label: name,
  detail: '',
  score,
  payload: { type: 'skill', skillId: name },
});

const byId = (id: string) => CAPABILITY_FIXTURE.find((r) => r.id === id)!;
const hit = (id: string, tier: RankHit['tier'] = 0, score = 1): RankHit => ({ row: byId(id), tier, score });
const DOMAINS = [
  { id: 'compiler', label: 'Compiler' },
  { id: 'mesh', label: 'Mesh' },
  { id: 'plugins', label: 'Plugins' },
];
const allRows = (facets: ReturnType<typeof buildOmnibarFacets>) => facets.flatMap((f) => f.rows);
const facetOf = (facets: ReturnType<typeof buildOmnibarFacets>, key: string) => facets.find((f) => f.key === key)!;
const WTL_DOC = '/Users/me/dev/vox/docs/src/architecture/where-things-live.md';

const cmdHit = (cmd: string): UnifiedHit => ({
  source: 'commands',
  kind: 'command',
  path: cmd,
  title: cmd,
  snippet: '',
  score: 0.9,
  provenance: ['commands:catalog'],
  locator: { kind: 'command', value: cmd },
});

const manifestRow = (vk: string): ContentManifestEntry => ({
  viewKey: vk,
  label: 'Activity',
  route: `#view=${vk}`,
  headings: ['3 pending approvals'],
  copy: ['3 pending approvals'],
  commands: [],
  docs: [],
});

const regHit = (label: string): SearchableHit => ({
  surfaceId: 'activity',
  label,
  detail: 'Activity',
  viewKey: 'activity',
});

function sources(partial: Partial<OmnibarSources>): OmnibarSources {
  return {
    query: 'pending',
    federated: [],
    backendHits: [],
    manifest: [],
    runtimeHits: [],
    graph: { rows: [], error: null },
    ...partial,
  };
}

describe('buildOmnibarFacets', () => {
  it('buckets sources into the five facets with provenance labels', () => {
    const facets = buildOmnibarFacets(
      sources({
        query: 'help pending',
        federated: [surface('approvals', 'Approvals'), docEntry('a.md', 'Approvals workflow')],
        backendHits: [cmdHit('vox_resolve_approval')],
        manifest: [manifestRow('activity')],
        runtimeHits: [regHit('3 pending approvals (live)')],
        graph: {
          rows: [{ id: 'n1', label: 'Chat rail', viewKey: 'chat' }],
          error: null,
        },
      }),
    );
    const byKey = Object.fromEntries(facets.map((f) => [f.key, f]));
    expect(byKey.surfaces.rows[0].provenance).toBe('manifest'); // surface-kind → manifest-class label
    expect(byKey.commands.rows[0].provenance).toBe('corpus');
    expect(byKey.onScreen.rows.some((r) => r.provenance === 'runtime')).toBe(true);
    expect(byKey.graph.rows[0].provenance).toBe('graph');
    expect(byKey.docs.rows[0].provenance).toBe('docs');
    expect(facets.map((f) => f.key)).toEqual([
      'best',
      'surfaces',
      'commands',
      'tools',
      'onScreen',
      'graph',
      'docs',
    ]);
  });

  it('caps each facet at FACET_CAP', () => {
    const many = Array.from({ length: FACET_CAP + 5 }, (_, i) => cmdHit(`cmd_${i}`));
    const facets = buildOmnibarFacets(sources({ backendHits: many }));
    const commands = facets.find((f) => f.key === 'commands')!;
    expect(commands.rows).toHaveLength(FACET_CAP);
  });

  it('graph facet failure is isolated — other facets still populate', () => {
    const facets = buildOmnibarFacets(
      sources({
        federated: [surface('approvals', 'Approvals')],
        graph: { rows: [], error: 'graph facet pending VG-1' },
      }),
    );
    const graph = facets.find((f) => f.key === 'graph')!;
    const surfaces = facets.find((f) => f.key === 'surfaces')!;
    expect(graph.error).toBe('graph facet pending VG-1');
    expect(graph.rows).toHaveLength(0);
    expect(surfaces.rows.length).toBeGreaterThan(0); // not blanked
  });

  it('topHit returns the highest-priority row across non-empty facets', () => {
    const facets = buildOmnibarFacets(
      sources({ federated: [surface('approvals', 'Approvals')] }),
    );
    const order = facets.flatMap((f) => f.rows);
    expect(order[0].facet).toBe('surfaces');
  });

  it('hides doc facet unless the query is help-oriented', () => {
    const hidden = buildOmnibarFacets(
      sources({ query: 'pending', federated: [docEntry('a.md', 'Guide')] }),
    );
    expect(hidden.find((f) => f.key === 'docs')!.rows).toHaveLength(0);

    const shown = buildOmnibarFacets(
      sources({ query: 'help cli', federated: [docEntry('a.md', 'CLI guide')] }),
    );
    expect(shown.find((f) => f.key === 'docs')!.rows).toHaveLength(1);
  });

  it('queryForFederatedSearch strips help prefix for doc matching', () => {
    expect(queryForFederatedSearch('help cli')).toBe('cli');
    const entries = [docEntry('docs/src/reference/cli.md', 'CLI Reference')];
    const hits = searchFederatedIndex(entries, queryForFederatedSearch('help cli'), { kinds: ['doc'] });
    expect(hits).toHaveLength(1);
  });

  it('shows docs facet for skills prefix mode', () => {
    const shown = buildOmnibarFacets(
      sources({
        query: 'cli',
        prefixMode: 'skills',
        federated: [docEntry('a.md', 'CLI guide')],
      }),
    );
    expect(shown.find((f) => f.key === 'docs')!.rows).toHaveLength(1);
    expect(shouldIncludeDocsInOmnibar('cli', { prefixMode: 'skills' })).toBe(true);
  });

  it('every_kind_reachable: each kind yields exactly one row in its home facet', () => {
    const facets = buildOmnibarFacets(
      sources({
        query: 'x',
        indexHits: [
          hit('cli.fmt'),
          hit('mcp.vox_capability_explain', 1, 0.5),
          hit('crate.vox-actor-runtime', 1, 0.4),
          hit('surface.mesh', 1, 0.3),
          hit('skill.algorithmic-art', 2, 0.5),
          hit('doc.docs/src/architecture/where-things-live.md', 1, 0.2),
        ],
        federated: [settingEntry('Max concurrent agents', 50), policyEntry('fmt.rust', 50)],
        domains: DOMAINS,
        installedSkillNames: new Set<string>(),
      }),
    );
    const rows = allRows(facets);
    expect(rows).toHaveLength(8);
    expect(new Set(rows.map((r) => r.id)).size).toBe(8);
    expect(facetOf(facets, 'best').rows.map((r) => r.id)).toEqual(['index:cli.fmt']);
    expect(facetOf(facets, 'tools').rows.map((r) => r.label).sort()).toEqual(['vox-actor-runtime', 'vox_capability_explain']);
    expect(facetOf(facets, 'surfaces').rows.map((r) => r.activate)).toEqual([{ type: 'navigate', viewKey: 'mesh' }]);
    const commands = facetOf(facets, 'commands').rows;
    expect(commands.map((r) => r.activate.type).sort()).toEqual(['explain', 'policy', 'setting']);
    expect(commands.find((r) => r.activate.type === 'explain')!.activate).toEqual({ type: 'explain', rowId: 'skill.algorithmic-art' });
    expect(facetOf(facets, 'docs').rows.map((r) => r.activate)).toEqual([{ type: 'doc', path: 'architecture/where-things-live.md' }]);
  });

  it('no_duplicates: a row shared by two sources renders once', () => {
    const facets = buildOmnibarFacets(
      sources({
        query: 'pending',
        indexHits: [
          hit('surface.mesh'),
          hit('cli.check'),
          hit('skill.algorithmic-art', 2, 0.5),
          hit('doc.docs/src/architecture/where-things-live.md', 0),
        ],
        federated: [surface('mesh', 'Mesh'), skillEntry('algorithmic-art', 100), docEntry(WTL_DOC, 'Where Things Live', 100)],
        backendHits: [cmdHit('check')],
        manifest: [manifestRow('mesh')],
        domains: DOMAINS,
        installedSkillNames: new Set(['algorithmic-art']),
      }),
    );
    const top = facets.filter((f) => f.key !== 'onScreen');
    const rows = allRows(top);
    expect(rows.filter((r) => r.activate.type === 'navigate' && r.activate.viewKey === 'mesh')).toHaveLength(1);
    expect(rows.filter((r) => r.label === 'algorithmic-art')).toHaveLength(1);
    expect(rows.filter((r) => r.label === 'algorithmic-art')[0].activate.type).toBe('skill');
    expect(rows.filter((r) => r.label === 'Where Things Live')).toHaveLength(1);
    expect(rows.filter((r) => r.id === 'cmd:check' || r.label === 'vox check')).toHaveLength(1);
  });

  it('best_match_moved: the top hit leaves its facet and heads Best match', () => {
    const facets = buildOmnibarFacets(
      sources({ query: 'fmt', indexHits: [hit('cli.fmt')], federated: [policyEntry('fmt.rust', 80)], domains: DOMAINS }),
    );
    expect(facets[0].key).toBe('best');
    expect(facets[0].label).toBe('Best match');
    expect(facets[0].rows.map((r) => r.label)).toEqual(['vox fmt']);
    const commands = facetOf(facets, 'commands').rows;
    expect(commands.map((r) => r.id)).toEqual(['policy:fmt.rust']);
  });

  it('best_match_cross_source: a better federated hit beats a typo-tier index hit', () => {
    const facets = buildOmnibarFacets(
      sources({ query: 'fmt.rust', indexHits: [hit('cli.fmt', 3, 0.9)], federated: [policyEntry('fmt.rust', 100)], domains: DOMAINS }),
    );
    expect(facets[0].rows.map((r) => r.id)).toEqual(['policy:fmt.rust']);
    expect(facetOf(facets, 'commands').rows.map((r) => r.id)).toEqual(['index:cli.fmt']);
  });

  it('best_match_empty: no query, no Best match', () => {
    const facets = buildOmnibarFacets(sources({ query: '', domains: DOMAINS }));
    expect(facets[0].key).toBe('best');
    expect(facets[0].rows).toHaveLength(0);
  });

  it('totals_and_caps: Tools & crates caps at FACET_CAP and reports the uncapped total', () => {
    const crates = Array.from({ length: 9 }, (_, i) => row({ id: `crate.vox-c${i}`, kind: 'crate', title: `vox-c${i}`, crate: `vox-c${i}`, layer: 2 }));
    const facets = buildOmnibarFacets(
      sources({
        query: 'vox',
        indexHits: [hit('cli.check'), ...crates.map((r) => ({ row: r, tier: 2 as const, score: 0.5 }))],
        domains: DOMAINS,
      }),
    );
    const tools = facetOf(facets, 'tools');
    expect(tools.label).toBe('Tools & crates');
    expect(tools.rows).toHaveLength(FACET_CAP);
    expect(tools.total).toBe(9);
    expect(facetOf(facets, 'best').rows[0].id).toBe('index:cli.check');
  });

  it('docs_default_mode: only title-exact/prefix docs show without a help-like query', () => {
    const docHit = (tier: RankHit['tier']) => hit('doc.docs/src/architecture/where-things-live.md', tier, 0.5);
    // a lone doc becomes Best match, so count doc rows across every facet
    const bestOrDocs = (partial: Partial<OmnibarSources>) => allRows(buildOmnibarFacets(sources({ query: 'where', domains: DOMAINS, ...partial }))).filter((r) => r.kind === 'doc');
    expect(bestOrDocs({ indexHits: [docHit(1)] })).toHaveLength(1);
    expect(bestOrDocs({ indexHits: [docHit(2)] })).toHaveLength(0);
    expect(bestOrDocs({ federated: [docEntry('a.md', 'Alpha', 80)] })).toHaveLength(1);
    expect(bestOrDocs({ federated: [docEntry('a.md', 'Alpha', 50)] })).toHaveLength(0);
    // help-like and skills mode keep substring matches
    expect(bestOrDocs({ query: 'help where', federated: [docEntry('a.md', 'Alpha', 50)] })).toHaveLength(1);
    expect(bestOrDocs({ prefixMode: 'skills', indexHits: [docHit(2)] })).toHaveLength(1);
  });

  it('chips: index rows carry interface chips and a domain label', () => {
    const facets = buildOmnibarFacets(
      sources({ query: 'x', indexHits: [hit('cli.check'), hit('surface.mesh', 1, 0.5)], domains: DOMAINS }),
    );
    const rows = allRows(facets);
    const check = rows.find((r) => r.id === 'index:cli.check')!;
    expect(check.chips).toEqual({ interfaces: ['CLI', 'GUI', 'MCP'], domain: 'Compiler' });
    expect(check.kind).toBe('cli');
    expect(rows.find((r) => r.activate.type === 'navigate')!.chips!.interfaces).toEqual(['GUI']);
  });

  it('tie: equal tier and score resolve by facet order, whatever the input order', () => {
    const a = buildOmnibarFacets(sources({ query: 'x', indexHits: [hit('cli.fmt', 1, 0.5), hit('surface.mesh', 1, 0.5)], domains: DOMAINS }));
    const b = buildOmnibarFacets(sources({ query: 'x', indexHits: [hit('surface.mesh', 1, 0.5), hit('cli.fmt', 1, 0.5)], domains: DOMAINS }));
    expect(a[0].rows[0].activate).toEqual({ type: 'navigate', viewKey: 'mesh' });
    expect(b[0].rows[0].activate).toEqual({ type: 'navigate', viewKey: 'mesh' });
  });

  it('crates mode: only Best match and Tools & crates carry rows', () => {
    const facets = buildOmnibarFacets(
      sources({
        query: 'vox',
        prefixMode: 'crates',
        indexHits: [hit('crate.vox-actor-runtime'), hit('crate.vox-orchestrator', 1, 0.5)],
        federated: [surface('mesh', 'Mesh')],
        manifest: [manifestRow('mesh')],
        domains: DOMAINS,
      }),
    );
    expect(facetOf(facets, 'best').rows.map((r) => r.id)).toEqual(['index:crate.vox-actor-runtime']);
    expect(facetOf(facets, 'tools').rows.map((r) => r.id)).toEqual(['index:crate.vox-orchestrator']);
    expect(allRows(facets)).toHaveLength(2);
  });
});

describe('keybind action rows (D-13)', () => {
  const ACTIONS = [
    { id: 'toggle-sidebar', label: 'Toggle sidebar width', chord: '⌘B' },
    { id: 'toggle-inspector', label: 'Toggle pipeline inspector drawer', chord: '⌘⇧D' },
    { id: 'show-shortcuts', label: 'Show keyboard shortcuts', chord: '⌘/' },
  ];

  it('a matching action is one Commands row with its chord chip and the action activation', () => {
    const facets = buildOmnibarFacets(sources({ query: 'sidebar', actions: ACTIONS }));
    const rows = allRows(facets).filter((r) => r.id.startsWith('action:key.'));
    expect(rows.map((r) => r.id)).toEqual(['action:key.toggle-sidebar']);
    const r = rows[0];
    expect(r.kind).toBe('action');
    expect(r.detail).toBe('Keyboard shortcut');
    expect(r.chips?.chord).toBe('⌘B');
    expect(r.activate).toEqual({ type: 'action', actionId: 'toggle-sidebar' });
    expect(facetOf(facets, 'commands').rows.concat(facetOf(facets, 'best').rows)).toContain(r);
  });

  it('the chord text finds the action too', () => {
    const facets = buildOmnibarFacets(sources({ query: '⌘B', actions: ACTIONS }));
    expect(allRows(facets).map((r) => r.id)).toContain('action:key.toggle-sidebar');
  });

  it('an empty query yields no action rows', () => {
    const facets = buildOmnibarFacets(sources({ query: '', actions: ACTIONS }));
    expect(allRows(facets).filter((r) => r.id.startsWith('action:key.'))).toEqual([]);
  });

  it('two actions sharing a chord are two rows, in registry order, each with that chord', () => {
    const shared = [
      { id: 'open-palette', label: 'Open command palette', chord: '⌘K' },
      { id: 'toggle-sidebar', label: 'Toggle sidebar width', chord: '⌘K' },
    ];
    const rows = allRows(buildOmnibarFacets(sources({ query: '⌘K', actions: shared })));
    expect(rows.map((r) => r.id)).toEqual(['action:key.open-palette', 'action:key.toggle-sidebar']);
    expect(rows.map((r) => r.chips?.chord)).toEqual(['⌘K', '⌘K']);
  });

  it('an exact label match is the Best match', () => {
    const facets = buildOmnibarFacets(sources({ query: 'Toggle sidebar width', actions: ACTIONS }));
    expect(facetOf(facets, 'best').rows.map((r) => r.id)).toEqual(['action:key.toggle-sidebar']);
  });

  it('ranks by tier then registry order: starts-with before contains', () => {
    const facets = buildOmnibarFacets(sources({ query: 'toggle', actions: ACTIONS }));
    const ids = allRows(facets).map((r) => r.id);
    expect(ids).toEqual(['action:key.toggle-sidebar', 'action:key.toggle-inspector']);
    const tiered = [
      { id: 'a', label: 'My sidebar toggle', chord: '⌘1' },
      { id: 'b', label: 'Sidebar toggle', chord: '⌘2' },
    ];
    const t = allRows(buildOmnibarFacets(sources({ query: 'sidebar', actions: tiered }))).map((r) => r.id);
    expect(t).toEqual(['action:key.b', 'action:key.a']);
  });

  it('is included in > (commands) mode and excluded from # (crates) mode', () => {
    const has = (prefixMode: OmnibarSources['prefixMode']) =>
      allRows(buildOmnibarFacets(sources({ query: 'sidebar', actions: ACTIONS, prefixMode }))).some((r) =>
        r.id.startsWith('action:key.'),
      );
    expect(has('commands')).toBe(true);
    expect(has('crates')).toBe(false);
  });
});
