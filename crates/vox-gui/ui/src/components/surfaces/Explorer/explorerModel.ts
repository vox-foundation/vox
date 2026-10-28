import type { CapabilityGroup, CapabilityIndexEntry } from '../../../generated/capabilityIndex.generated';

export type ExplorerStatus = 'dedicated' | 'generic_form' | 'cli_only' | 'mcp_only';

export interface VerbLeaf {
  kind: 'verb'; key: string; id: string; title: string; cli: string; path: string[];
  summary: string | null; docs: string | null; sideEffect: string | null; catalogForm: boolean;
  status: 'dedicated' | 'generic_form' | 'cli_only'; groupName: string; domain: string;
}

export interface McpLeaf {
  kind: 'mcp_tool'; key: string; id: string; title: string; mcp: string;
  summary: string | null; docs: string | null; status: 'mcp_only'; domain: string;
}

export interface GroupNode {
  kind: 'group'; key: string; name: string; domain: string;
  status: 'dedicated' | 'generic_form' | 'cli_only'; viewKey: string | null;
  crate: string | null; crateRowId: string | null; layer: number | null;
  verbCount: number; compiledCount: number; summary: string | null; docs: string | null;
  rowId: string | null; sideEffect: string | null; catalogForm: boolean;
  exemption: { owner: string; reason: string; reviewBy: string } | null; verbs: VerbLeaf[];
}

export interface McpPseudoGroupNode {
  kind: 'mcp_group'; key: string; name: string; domain: string; status: 'mcp_only'; tools: McpLeaf[];
}

export interface DomainScopeNode {
  id: string; label: string; groups: GroupNode[]; mcpGroup: McpPseudoGroupNode | null;
}

export interface ExplorerTreeModel {
  domains: DomainScopeNode[]; totalExemptGroups: number;
}

export interface ExplorerFilters {
  domain?: string | null; query?: string; statuses?: Iterable<ExplorerStatus>; showExempt?: boolean;
}

export interface StatusMixCounts {
  dedicated: number; generic_form: number; cli_only: number; mcp_only: number; total: number;
}

export interface DomainCountEntry {
  id: string; label: string; mix: StatusMixCounts;
}

export interface DomainCountsResult {
  all: StatusMixCounts; domains: DomainCountEntry[];
}

export interface FilteredDomainScope {
  id: string; label: string; groups: GroupNode[]; mcpGroup: McpPseudoGroupNode | null;
  leafCount: number; groupCount: number;
}

export interface FilteredExplorerTree {
  domain: string | null; query: string; statuses: Set<ExplorerStatus>; showExempt: boolean;
  domains: FilteredDomainScope[]; leafCount: number; groupCount: number;
  exemptHidden: number; exemptHiddenMatchingInScope: number; otherDomainsHaveHits: boolean;
}

export interface FlattenedRow {
  key: string; kind: 'domain' | 'group' | 'verb' | 'mcp_group' | 'mcp_tool' | 'exemption_strip';
  level: number; setsize: number; posinset: number; rowIndex: number;
  expandable: boolean; expanded?: boolean; parentKey: string | null; domainId: string;
  domainLabel?: string; domainLeafCount?: number; domainMix?: StatusMixCounts;
  group?: GroupNode; verb?: VerbLeaf; mcpGroup?: McpPseudoGroupNode; mcpTool?: McpLeaf;
  exemption?: { owner: string; reason: string; reviewBy: string };
}

export interface FlattenResult {
  rows: FlattenedRow[]; capped: boolean;
}

export function localIsoDate(date: Date = new Date()): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

export function isOverdue(reviewBy: string, today: string = localIsoDate()): boolean {
  return reviewBy < today;
}

function emptyMix(): StatusMixCounts {
  return { dedicated: 0, generic_form: 0, cli_only: 0, mcp_only: 0, total: 0 };
}

function addMix(target: StatusMixCounts, status: ExplorerStatus, n = 1): void {
  target[status] += n;
  target.total += n;
}

function isMcpOnlyOrOrphan(row: CapabilityIndexEntry): boolean {
  if (row.kind !== 'mcp_tool') return false;
  if (row.coverage === 'orphan_backend' || row.coverage === 'dead_end') return true;
  return row.equivalents.length === 0;
}

export function buildTree(
  domains: readonly { id: string; label: string }[],
  groups: readonly CapabilityGroup[],
  rows: readonly CapabilityIndexEntry[],
): ExplorerTreeModel {
  const crateRowByName = new Map<string, string>();
  const rowById = new Map<string, CapabilityIndexEntry>();
  const verbsByGroup = new Map<string, CapabilityIndexEntry[]>();

  for (const r of rows) {
    rowById.set(r.id, r);
    if (r.kind === 'crate' && r.crate) crateRowByName.set(r.crate, r.id);
    if (r.kind === 'cli' && r.cli) {
      const tokens = r.cli.trim().split(/\s+/);
      if (tokens[0] === 'vox' && tokens.length >= 3) {
        const gName = tokens[1];
        const list = verbsByGroup.get(gName) ?? [];
        list.push(r);
        verbsByGroup.set(gName, list);
      }
    }
  }

  const groupsByDomain = new Map<string, GroupNode[]>();
  let totalExemptGroups = 0;

  const sortedGroups = [...groups].sort((a, b) => a.name.localeCompare(b.name));
  for (const g of sortedGroups) {
    if (g.exemption) totalExemptGroups += 1;
    const groupStatus: 'dedicated' | 'generic_form' | 'cli_only' = g.exemption ? 'cli_only' : g.status;
    const rawVerbs = (verbsByGroup.get(g.name) ?? []).sort((a, b) => (a.cli ?? '').localeCompare(b.cli ?? ''));

    const verbs: VerbLeaf[] = rawVerbs.map((r) => {
      const cli = r.cli ?? `vox ${g.name}`;
      const tokens = cli.trim().split(/\s+/);
      const path = tokens[0] === 'vox' ? tokens.slice(1) : tokens;
      const verbStatus: 'dedicated' | 'generic_form' | 'cli_only' =
        g.exemption || !r.catalogForm ? 'cli_only' : g.status;
      return {
        kind: 'verb',
        key: `verb:${r.id}`,
        id: r.id,
        title: r.title,
        cli,
        path,
        summary: r.summary,
        docs: r.docs,
        sideEffect: r.sideEffect,
        catalogForm: r.catalogForm,
        status: verbStatus,
        groupName: g.name,
        domain: g.domain,
      };
    });

    const bareRow = g.rowId ? rowById.get(g.rowId) : undefined;
    const node: GroupNode = {
      kind: 'group',
      key: `group:${g.name}`,
      name: g.name,
      domain: g.domain,
      status: groupStatus,
      viewKey: g.viewKey,
      crate: g.crate,
      crateRowId: g.crate ? (crateRowByName.get(g.crate) ?? null) : null,
      layer: g.layer,
      verbCount: verbs.length > 0 ? verbs.length : g.verbs,
      compiledCount: g.compiled,
      summary: g.summary ?? bareRow?.summary ?? null,
      docs: g.docs ?? bareRow?.docs ?? null,
      rowId: g.rowId,
      sideEffect: bareRow?.sideEffect ?? null,
      catalogForm: bareRow ? bareRow.catalogForm : g.compiled > 0,
      exemption: g.exemption,
      verbs,
    };

    const domainList = groupsByDomain.get(g.domain) ?? [];
    domainList.push(node);
    groupsByDomain.set(g.domain, domainList);
  }

  const mcpByDomain = new Map<string, McpLeaf[]>();
  for (const r of rows) {
    if (!isMcpOnlyOrOrphan(r) || !r.domain) continue;
    const mcpName = r.mcp ?? r.title;
    const list = mcpByDomain.get(r.domain) ?? [];
    list.push({
      kind: 'mcp_tool',
      key: `mcp:${r.id}`,
      id: r.id,
      title: r.title,
      mcp: mcpName,
      summary: r.summary,
      docs: r.docs,
      status: 'mcp_only',
      domain: r.domain,
    });
    mcpByDomain.set(r.domain, list);
  }

  const domainNodes: DomainScopeNode[] = [];
  for (const d of domains) {
    const domGroups = groupsByDomain.get(d.id) ?? [];
    const domMcp = (mcpByDomain.get(d.id) ?? []).sort((a, b) => a.mcp.localeCompare(b.mcp));
    const leafCount =
      domGroups.reduce((acc, g) => acc + (g.verbs.length > 0 ? g.verbs.length : 1), 0) + domMcp.length;
    if (leafCount === 0) continue;

    domainNodes.push({
      id: d.id,
      label: d.label,
      groups: domGroups,
      mcpGroup:
        domMcp.length > 0
          ? { kind: 'mcp_group', key: `mcp-only:${d.id}`, name: 'MCP-only tools', domain: d.id, status: 'mcp_only', tools: domMcp }
          : null,
    });
  }

  return { domains: domainNodes, totalExemptGroups };
}

function matchesText(haystacks: (string | null | undefined)[], qLower: string): boolean {
  if (!qLower) return true;
  return haystacks.some((h) => Boolean(h && h.toLowerCase().includes(qLower)));
}

function filterGroupLeaves(g: GroupNode, qLower: string, statuses: Set<ExplorerStatus>): GroupNode | null {
  const groupSelfMatches = matchesText([g.rowId, g.name, `vox ${g.name}`, g.summary], qLower);
  if (g.verbs.length === 0) {
    if (!groupSelfMatches) return null;
    if (statuses.size > 0 && !statuses.has(g.status)) return null;
    return g;
  }

  const directMatchingVerbs = g.verbs.filter((v) => matchesText([v.id, v.title, v.cli, v.summary], qLower));
  const candidateVerbs = qLower && directMatchingVerbs.length === 0 && groupSelfMatches ? g.verbs : directMatchingVerbs;
  const statusFiltered = statuses.size > 0 ? candidateVerbs.filter((v) => statuses.has(v.status)) : candidateVerbs;
  if (statusFiltered.length === 0) return null;
  return { ...g, verbs: statusFiltered };
}

function filterMcpGroup(
  mcpGroup: McpPseudoGroupNode | null,
  qLower: string,
  statuses: Set<ExplorerStatus>,
): McpPseudoGroupNode | null {
  if (!mcpGroup) return null;
  if (statuses.size > 0 && !statuses.has('mcp_only')) return null;
  const matching = mcpGroup.tools.filter((t) => matchesText([t.id, t.title, t.mcp, t.summary], qLower));
  if (matching.length === 0) return null;
  return { ...mcpGroup, tools: matching };
}

export function statusCounts(
  tree: ExplorerTreeModel,
  filters: Omit<ExplorerFilters, 'statuses'> = {},
): StatusMixCounts {
  const domain = filters.domain ?? null;
  const qLower = (filters.query ?? '').trim().toLowerCase();
  const showExempt = filters.showExempt ?? false;
  const emptyStatuses = new Set<ExplorerStatus>();
  const mix = emptyMix();

  for (const d of tree.domains) {
    if (domain !== null && d.id !== domain) continue;
    for (const g of d.groups) {
      if (!showExempt && g.exemption) continue;
      const fg = filterGroupLeaves(g, qLower, emptyStatuses);
      if (!fg) continue;
      if (fg.verbs.length === 0) addMix(mix, fg.status, 1);
      else for (const v of fg.verbs) addMix(mix, v.status, 1);
    }
    const fm = filterMcpGroup(d.mcpGroup, qLower, emptyStatuses);
    if (fm) addMix(mix, 'mcp_only', fm.tools.length);
  }
  return mix;
}

export function domainCounts(
  tree: ExplorerTreeModel,
  filters: Pick<ExplorerFilters, 'query' | 'showExempt'> = {},
): DomainCountsResult {
  const qLower = (filters.query ?? '').trim().toLowerCase();
  const showExempt = filters.showExempt ?? false;
  const emptyStatuses = new Set<ExplorerStatus>();
  const all = emptyMix();
  const domains: DomainCountEntry[] = [];

  for (const d of tree.domains) {
    const mix = emptyMix();
    for (const g of d.groups) {
      if (!showExempt && g.exemption) continue;
      const fg = filterGroupLeaves(g, qLower, emptyStatuses);
      if (!fg) continue;
      if (fg.verbs.length === 0) {
        addMix(mix, fg.status, 1);
        addMix(all, fg.status, 1);
      } else {
        for (const v of fg.verbs) {
          addMix(mix, v.status, 1);
          addMix(all, v.status, 1);
        }
      }
    }
    const fm = filterMcpGroup(d.mcpGroup, qLower, emptyStatuses);
    if (fm) {
      addMix(mix, 'mcp_only', fm.tools.length);
      addMix(all, 'mcp_only', fm.tools.length);
    }
    domains.push({ id: d.id, label: d.label, mix });
  }

  return { all, domains };
}

export function applyFilters(tree: ExplorerTreeModel, filters: ExplorerFilters = {}): FilteredExplorerTree {
  const domain = filters.domain ?? null;
  const query = filters.query ?? '';
  const qLower = query.trim().toLowerCase();
  const statuses = new Set<ExplorerStatus>(filters.statuses ?? []);
  const showExempt = filters.showExempt ?? false;

  let exemptHidden = 0;
  let exemptHiddenMatchingInScope = 0;
  let otherDomainsHaveHits = false;
  const filteredDomains: FilteredDomainScope[] = [];
  let totalLeafCount = 0;
  let totalGroupCount = 0;

  for (const d of tree.domains) {
    const inScope = domain === null || d.id === domain;
    const keptGroups: GroupNode[] = [];
    let domLeaves = 0;

    for (const g of d.groups) {
      if (!showExempt && g.exemption) {
        exemptHidden += 1;
        if (filterGroupLeaves(g, qLower, statuses) && inScope) exemptHiddenMatchingInScope += 1;
        continue;
      }
      const fg = filterGroupLeaves(g, qLower, statuses);
      if (!fg) continue;
      const leaves = fg.verbs.length > 0 ? fg.verbs.length : 1;
      if (inScope) {
        keptGroups.push(fg);
        domLeaves += leaves;
      } else {
        otherDomainsHaveHits = true;
      }
    }

    const fm = filterMcpGroup(d.mcpGroup, qLower, statuses);
    if (fm) {
      if (inScope) domLeaves += fm.tools.length;
      else otherDomainsHaveHits = true;
    }

    if (inScope && domLeaves > 0) {
      const groupCount = keptGroups.length + (fm ? 1 : 0);
      filteredDomains.push({ id: d.id, label: d.label, groups: keptGroups, mcpGroup: fm, leafCount: domLeaves, groupCount });
      totalLeafCount += domLeaves;
      totalGroupCount += groupCount;
    }
  }

  return {
    domain,
    query,
    statuses,
    showExempt,
    domains: filteredDomains,
    leafCount: totalLeafCount,
    groupCount: totalGroupCount,
    exemptHidden,
    exemptHiddenMatchingInScope,
    otherDomainsHaveHits,
  };
}

function isGroupExpandable(g: GroupNode): boolean {
  return g.verbs.length > 0 || g.exemption !== null;
}

function groupChildCount(g: GroupNode): number {
  return g.verbs.length + (g.exemption ? 1 : 0);
}

export function flatten(
  filtered: FilteredExplorerTree,
  expandedKeys: ReadonlySet<string> = new Set(),
  collapsedDomainKeys: ReadonlySet<string> = new Set(),
): FlattenResult {
  const autoExpandActive = filtered.query.trim().length > 0 || filtered.statuses.size > 0;
  let capped = false;
  const autoExpandedGroupKeys = new Set<string>();

  if (autoExpandActive) {
    let projectedRows = filtered.domain === null ? filtered.domains.length : 0;
    const expandableGroupOrder: { key: string; childRows: number }[] = [];
    for (const d of filtered.domains) {
      projectedRows += d.groups.length + (d.mcpGroup ? 1 : 0);
      for (const g of d.groups) {
        if (isGroupExpandable(g)) {
          const c = groupChildCount(g);
          projectedRows += c;
          expandableGroupOrder.push({ key: g.key, childRows: c });
        }
      }
      if (d.mcpGroup) {
        const c = d.mcpGroup.tools.length;
        projectedRows += c;
        expandableGroupOrder.push({ key: d.mcpGroup.key, childRows: c });
      }
    }
    const chosen = projectedRows > 200 && expandableGroupOrder.length > 20
      ? ((capped = true), expandableGroupOrder.slice(0, 20))
      : expandableGroupOrder;
    for (const item of chosen) autoExpandedGroupKeys.add(item.key);
  }

  const rows: FlattenedRow[] = [];
  let nextRowIndex = 2;
  const allDomainsView = filtered.domain === null;
  const domainSetsize = filtered.domains.length;

  filtered.domains.forEach((d, dIdx) => {
    const domainKey = `domain:${d.id}`;
    const domainExpanded = autoExpandActive ? true : !collapsedDomainKeys.has(domainKey);
    let parentKeyForGroups: string | null = null;

    if (allDomainsView) {
      const mix = emptyMix();
      for (const g of d.groups) {
        if (g.verbs.length === 0) addMix(mix, g.status, 1);
        else for (const v of g.verbs) addMix(mix, v.status, 1);
      }
      if (d.mcpGroup) addMix(mix, 'mcp_only', d.mcpGroup.tools.length);

      rows.push({
        key: domainKey,
        kind: 'domain',
        level: 1,
        setsize: domainSetsize,
        posinset: dIdx + 1,
        rowIndex: nextRowIndex++,
        expandable: true,
        expanded: domainExpanded,
        parentKey: null,
        domainId: d.id,
        domainLabel: d.label,
        domainLeafCount: d.leafCount,
        domainMix: mix,
      });
      parentKeyForGroups = domainKey;
      if (!domainExpanded) return;
    }

    const groupLevel = allDomainsView ? 2 : 1;
    const childLevel = groupLevel + 1;
    const groupSiblingsTotal = d.groups.length + (d.mcpGroup ? 1 : 0);

    d.groups.forEach((g, gIdx) => {
      const expandable = isGroupExpandable(g);
      const isExpanded = expandable && (expandedKeys.has(g.key) || autoExpandedGroupKeys.has(g.key));

      rows.push({
        key: g.key,
        kind: 'group',
        level: groupLevel,
        setsize: groupSiblingsTotal,
        posinset: gIdx + 1,
        rowIndex: nextRowIndex++,
        expandable,
        ...(expandable ? { expanded: isExpanded } : {}),
        parentKey: parentKeyForGroups,
        domainId: d.id,
        group: g,
      });

      if (isExpanded) {
        const childSetsize = groupChildCount(g);
        let childPos = 1;
        if (g.exemption) {
          rows.push({
            key: `strip:${g.name}`,
            kind: 'exemption_strip',
            level: childLevel,
            setsize: childSetsize,
            posinset: childPos++,
            rowIndex: nextRowIndex++,
            expandable: false,
            parentKey: g.key,
            domainId: d.id,
            group: g,
            exemption: g.exemption,
          });
        }
        for (const v of g.verbs) {
          rows.push({
            key: v.key,
            kind: 'verb',
            level: childLevel,
            setsize: childSetsize,
            posinset: childPos++,
            rowIndex: nextRowIndex++,
            expandable: false,
            parentKey: g.key,
            domainId: d.id,
            group: g,
            verb: v,
          });
        }
      }
    });

    if (d.mcpGroup) {
      const mg = d.mcpGroup;
      const isExpanded = expandedKeys.has(mg.key) || autoExpandedGroupKeys.has(mg.key);
      rows.push({
        key: mg.key,
        kind: 'mcp_group',
        level: groupLevel,
        setsize: groupSiblingsTotal,
        posinset: groupSiblingsTotal,
        rowIndex: nextRowIndex++,
        expandable: true,
        expanded: isExpanded,
        parentKey: parentKeyForGroups,
        domainId: d.id,
        mcpGroup: mg,
      });

      if (isExpanded) {
        const toolSetsize = mg.tools.length;
        mg.tools.forEach((t, tIdx) => {
          rows.push({
            key: t.key,
            kind: 'mcp_tool',
            level: childLevel,
            setsize: toolSetsize,
            posinset: tIdx + 1,
            rowIndex: nextRowIndex++,
            expandable: false,
            parentKey: mg.key,
            domainId: d.id,
            mcpGroup: mg,
            mcpTool: t,
          });
        });
      }
    }
  });

  return { rows, capped };
}
