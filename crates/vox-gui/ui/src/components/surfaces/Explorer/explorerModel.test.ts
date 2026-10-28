import { describe, it, expect } from 'vitest';
import {
  CAPABILITY_FIXTURE,
  DOMAIN_FIXTURE,
  GROUP_FIXTURE,
  group,
  row,
} from '../../../test-fixtures/capabilityIndexFixture';
import {
  applyFilters,
  buildTree,
  domainCounts,
  flatten,
  isOverdue,
  localIsoDate,
  statusCounts,
} from './explorerModel';

describe('explorerModel', () => {
  it('leaf_model: counts verbs on multi-verb groups, counts childless groups once, and assigns uncompiled verbs cli_only', () => {
    const tree = buildTree(DOMAIN_FIXTURE, GROUP_FIXTURE, CAPABILITY_FIXTURE);
    const meshDomain = tree.domains.find((d) => d.id === 'mesh');
    const compilerDomain = tree.domains.find((d) => d.id === 'compiler');
    const orchDomain = tree.domains.find((d) => d.id === 'orchestrator');

    expect(meshDomain).toBeDefined();
    expect(compilerDomain).toBeDefined();
    expect(orchDomain).toBeDefined();

    const meshGroup = meshDomain!.groups.find((g) => g.name === 'mesh')!;
    expect(meshGroup.verbs).toHaveLength(2);
    const gpuVerb = meshGroup.verbs.find((v) => v.cli === 'vox mesh gpu')!;
    const statusVerb = meshGroup.verbs.find((v) => v.cli === 'vox mesh status')!;
    expect(gpuVerb.status).toBe('cli_only');
    expect(statusVerb.status).toBe('dedicated');

    const fmtGroup = compilerDomain!.groups.find((g) => g.name === 'fmt')!;
    expect(fmtGroup.verbs).toHaveLength(0);
    expect(fmtGroup.status).toBe('generic_form');

    const attentionGroup = orchDomain!.groups.find((g) => g.name === 'attention')!;
    expect(attentionGroup.verbs).toHaveLength(0);
    expect(attentionGroup.status).toBe('cli_only');

    // Total leaves with showExempt: true = 1 (fmt) + 1 (attention) + 2 (mesh verbs) + 2 (mesh mcp-only) = 6
    const allWithExempt = applyFilters(tree, { showExempt: true });
    expect(allWithExempt.leafCount).toBe(6);
  });

  it('mcp_only_pseudo_group: includes MCP tools with empty equivalents or orphan coverage and excludes MCP tools with equivalents', () => {
    const tree = buildTree(DOMAIN_FIXTURE, GROUP_FIXTURE, CAPABILITY_FIXTURE);
    const meshDomain = tree.domains.find((d) => d.id === 'mesh')!;
    const compilerDomain = tree.domains.find((d) => d.id === 'compiler')!;

    expect(meshDomain.mcpGroup).not.toBeNull();
    expect(meshDomain.mcpGroup!.name).toBe('MCP-only tools');
    expect(meshDomain.mcpGroup!.tools.map((t) => t.mcp)).toEqual([
      'vox_mesh_nodes',
      'vox_orphan_probe',
    ]);
    expect(meshDomain.mcpGroup!.tools.every((t) => t.status === 'mcp_only')).toBe(true);

    // Compiler MCP tools all have CLI equivalents and non-orphan coverage
    expect(compilerDomain.mcpGroup).toBeNull();
  });

  it('ordering: preserves CAPABILITY_DOMAINS order, sorts groups by name and verbs by path, places MCP-only last, and omits empty domains', () => {
    const extraGroups = [
      ...GROUP_FIXTURE,
      group({ name: 'alpha', status: 'generic_form', domain: 'mesh' }),
    ];
    const tree = buildTree(DOMAIN_FIXTURE, extraGroups, CAPABILITY_FIXTURE);

    expect(tree.domains.map((d) => d.id)).toEqual(['compiler', 'orchestrator', 'mesh']);
    expect(tree.domains.some((d) => d.id === 'empty_domain')).toBe(false);

    const meshDomain = tree.domains.find((d) => d.id === 'mesh')!;
    expect(meshDomain.groups.map((g) => g.name)).toEqual(['alpha', 'mesh']);
    expect(meshDomain.groups[1].verbs.map((v) => v.cli)).toEqual([
      'vox mesh gpu',
      'vox mesh status',
    ]);

    const filtered = applyFilters(tree, { domain: 'mesh' });
    const flat = flatten(filtered);
    expect(flat.rows.map((r) => r.key)).toEqual([
      'group:alpha',
      'group:mesh',
      'mcp-only:mesh',
    ]);
  });

  it('filters: case-insensitive query keeps ancestor group, statuses OR together, and showExempt false hides exempt groups', () => {
    const tree = buildTree(DOMAIN_FIXTURE, GROUP_FIXTURE, CAPABILITY_FIXTURE);

    const byQuery = applyFilters(tree, { query: 'STATUS', showExempt: false });
    expect(byQuery.exemptHidden).toBe(1);
    expect(byQuery.domains.map((d) => d.id)).toEqual(['mesh']);
    expect(byQuery.domains[0].groups).toHaveLength(1);
    expect(byQuery.domains[0].groups[0].name).toBe('mesh');
    expect(byQuery.domains[0].groups[0].verbs.map((v) => v.cli)).toEqual(['vox mesh status']);

    const byStatuses = applyFilters(tree, {
      statuses: ['generic_form', 'cli_only'],
      showExempt: false,
    });
    // fmt (generic_form) + mesh gpu (cli_only); attention hidden because showExempt is false
    expect(byStatuses.leafCount).toBe(2);
    expect(byStatuses.exemptHidden).toBe(1);
  });

  it('counts: statusCounts ignores status filter and sums to leafCount; domainCounts ignores status filter and selected domain', () => {
    const tree = buildTree(DOMAIN_FIXTURE, GROUP_FIXTURE, CAPABILITY_FIXTURE);
    const filtered = applyFilters(tree, { domain: 'mesh', showExempt: false });
    const chips = statusCounts(tree, { domain: 'mesh', showExempt: false });

    expect(chips.dedicated).toBe(1);
    expect(chips.cli_only).toBe(1);
    expect(chips.mcp_only).toBe(2);
    expect(chips.generic_form).toBe(0);
    expect(chips.dedicated + chips.generic_form + chips.cli_only + chips.mcp_only).toBe(
      filtered.leafCount,
    );

    const rail = domainCounts(tree, { query: '', showExempt: false });
    expect(rail.all.total).toBe(5);
    const orch = rail.domains.find((d) => d.id === 'orchestrator')!;
    expect(orch.mix.total).toBe(0); // attention is hidden when showExempt is false, but domain stays in rail
  });

  it('flatten_all_domains and flatten_one_domain: computes contiguous rowIndex starting at 2, level/setsize/posinset, and expandable flags', () => {
    const tree = buildTree(DOMAIN_FIXTURE, GROUP_FIXTURE, CAPABILITY_FIXTURE);
    const allFiltered = applyFilters(tree, { showExempt: false });
    const expanded = new Set<string>(['group:mesh']);
    const flatAll = flatten(allFiltered, expanded);

    expect(flatAll.rows[0]).toMatchObject({
      key: 'domain:compiler',
      level: 1,
      setsize: 2,
      posinset: 1,
      rowIndex: 2,
      expandable: true,
      expanded: true,
    });
    // fmt is a childless non-exempt group -> expandable: false
    expect(flatAll.rows[1]).toMatchObject({
      key: 'group:fmt',
      level: 2,
      setsize: 1,
      posinset: 1,
      rowIndex: 3,
      expandable: false,
    });
    expect(flatAll.rows[1].expanded).toBeUndefined();

    // Verify contiguous rowIndex starting at 2
    flatAll.rows.forEach((r, idx) => {
      expect(r.rowIndex).toBe(idx + 2);
    });

    // Single domain view: groups at level 1, verbs at level 2
    const meshFiltered = applyFilters(tree, { domain: 'mesh', showExempt: false });
    const flatMesh = flatten(meshFiltered, expanded);
    expect(flatMesh.rows[0]).toMatchObject({
      key: 'group:mesh',
      level: 1,
      setsize: 2,
      posinset: 1,
      rowIndex: 2,
      expandable: true,
      expanded: true,
    });
    expect(flatMesh.rows[1]).toMatchObject({
      key: 'verb:cli.mesh.gpu',
      level: 2,
      setsize: 2,
      posinset: 1,
      rowIndex: 3,
      expandable: false,
    });
  });

  it('auto_expand_cap: caps auto-expanded groups at 20 when a query matches > 200 rows', () => {
    const bigGroups = Array.from({ length: 25 }, (_, gIdx) =>
      group({
        name: `grp${String(gIdx).padStart(2, '0')}`,
        status: 'generic_form',
        domain: 'compiler',
      }),
    );
    const bigRows = bigGroups.flatMap((g) =>
      Array.from({ length: 10 }, (_, vIdx) =>
        row({
          id: `cli.${g.name}.v${vIdx}`,
          kind: 'cli',
          title: `${g.name} v${vIdx}`,
          cli: `vox ${g.name} v${vIdx}`,
          domain: 'compiler',
          catalogForm: true,
        }),
      ),
    );
    const tree = buildTree(DOMAIN_FIXTURE, bigGroups, bigRows);
    const filtered = applyFilters(tree, { query: 'vox' });
    const flat = flatten(filtered);

    expect(flat.capped).toBe(true);
    const expandedGroups = flat.rows.filter((r) => r.kind === 'group' && r.expanded);
    const collapsedGroups = flat.rows.filter((r) => r.kind === 'group' && !r.expanded);
    expect(expandedGroups).toHaveLength(20);
    expect(collapsedGroups).toHaveLength(5);
  });

  it('exemption_strip: expanding an exempt group with showExempt on yields an exemption_strip row as its first child', () => {
    const tree = buildTree(DOMAIN_FIXTURE, GROUP_FIXTURE, CAPABILITY_FIXTURE);
    const filtered = applyFilters(tree, { domain: 'orchestrator', showExempt: true });
    const flat = flatten(filtered, new Set(['group:attention']));

    expect(flat.rows).toHaveLength(2);
    expect(flat.rows[0]).toMatchObject({
      key: 'group:attention',
      kind: 'group',
      expandable: true,
      expanded: true,
    });
    expect(flat.rows[1]).toMatchObject({
      key: 'strip:attention',
      kind: 'exemption_strip',
      level: 2,
      setsize: 1,
      posinset: 1,
      exemption: {
        owner: 'vox-orchestrator',
        reason: 'Feature-gated behind dei; scheduled for Inbox consolidation.',
        reviewBy: '2026-12-31',
      },
    });
  });

  it('isOverdue and localIsoDate: compares ISO dates against local date without UTC Date(string) shift', () => {
    expect(isOverdue('2026-10-12', '2026-10-12')).toBe(false);
    expect(isOverdue('2026-10-11', '2026-10-12')).toBe(true);
    expect(isOverdue('2026-10-13', '2026-10-12')).toBe(false);

    const local = new Date(2026, 9, 5, 23, 30, 0); // Oct 5, 2026 local
    expect(localIsoDate(local)).toBe('2026-10-05');
  });
});
