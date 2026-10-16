/**
 * Explain card computed in TS from index rows (D-07). Mirrors
 * crates/vox-capability-registry/src/card.rs `render_card`; never shells out to `vox explain`.
 */
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';
import { docKey } from './omnibarFacets';

// Must equal SURFACE_CRATES in crates/vox-capability-registry/src/index.rs (parity test reads it).
export const SURFACE_CRATES = ['vox-cli', 'vox-cli-ci', 'vox-orchestrator-mcp', 'vox-gui'] as const;

const DASH = '—';

export interface ExplainCard {
  title: string;
  kind: string;
  id: string;
  summary: string | null;
  equivalents: { cli: string[]; mcp: string[]; gui: string; catalog: string };
  location: { crate: string; source: string; docs: string };
  status: { coverage: string; tier: string; sideEffects: string; domain: string };
  related: { id: string; title: string }[];
  actions: {
    openForm: { path: string[]; unavailable?: string } | null;
    copyMcp: string | null;
    openDocs: string | null;
  };
}

const MAX_MCP_NAMES = 5;
const RELATED_LIMIT = 5;
const UNAVAILABLE = "not in this build's command catalog";

type Row = CapabilityIndexEntry;

function guiText(r: Row): string | null {
  if (r.navLabel && r.viewKey) return `${r.navLabel} (view ${r.viewKey})`;
  if (r.viewKey) return `view ${r.viewKey}`;
  return r.navLabel;
}

/**
 * @param asOf coverage snapshot (CAPABILITY_COVERAGE_AS_OF); passed in so this module never
 * imports the 784 KB generated artifact at runtime.
 */
export function buildExplainCard(
  row: Row,
  rows: readonly Row[],
  domains: readonly { id: string; label: string }[],
  catalogPaths: ReadonlySet<string>,
  asOf: { sha: string; date: string } | null = null,
): ExplainCard {
  const byId = new Map(rows.map((r) => [r.id, r]));
  const linked = [...new Set([...row.equivalents, ...row.refs])]
    .sort()
    .map((id) => byId.get(id))
    .filter((r): r is Row => !!r);
  const gather = (f: (r: Row) => string | null): string[] => {
    const own = f(row);
    if (own) return [own];
    return [...new Set(linked.map(f).filter((v): v is string => !!v))].sort();
  };

  const mcpAll = gather((r) => r.mcp);
  const mcp = mcpAll.length > MAX_MCP_NAMES
    ? [...mcpAll.slice(0, MAX_MCP_NAMES), `+${mcpAll.length - MAX_MCP_NAMES} more`]
    : mcpAll;
  const cli = gather((r) => r.cli);
  const gui = gather(guiText);
  const generic = [row, ...linked].some((r) => r.catalogForm);

  const cov = row.coverage
    ? row.coverageAmbiguous
      ? `${row.coverage} (ambiguous)`
      : row.coverage
    : row.coverageReason
      ? `none (${row.coverageReason})`
      : 'none (not computed)';
  const snap = asOf ? `coverage as of ${asOf.sha.slice(0, 9)} (${asOf.date})` : 'no snapshot';
  const domainLabel = row.domain ? (domains.find((d) => d.id === row.domain)?.label ?? row.domain) : DASH;

  const skip = new Set([...row.equivalents, ...row.refs]);
  const sameDomain = (r: Row) => row.domain !== null && r.domain === row.domain;
  const sameOwner = (r: Row) =>
    row.crate !== null && r.crate === row.crate && !(SURFACE_CRATES as readonly string[]).includes(row.crate);
  const related = rows
    .filter((r) => r.kind === 'cli' || r.kind === 'mcp_tool' || r.kind === 'surface')
    .filter((r) => r.id !== row.id && !skip.has(r.id))
    .filter((r) => sameDomain(r) || sameOwner(r))
    .sort((a, b) => Number(!sameDomain(a)) - Number(!sameDomain(b)) || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .slice(0, RELATED_LIMIT)
    .map((r) => ({ id: r.id, title: r.title }));

  const cliPath = cli[0]?.replace(/^vox\s+/, '').split(/\s+/).filter(Boolean) ?? [];
  const openForm = cliPath.length
    ? catalogPaths.has(cliPath.join(' '))
      ? { path: cliPath }
      : { path: cliPath, unavailable: UNAVAILABLE }
    : null;

  const docsText = row.docs ? (row.docOrigin ? `${row.docs} (${row.docOrigin})` : row.docs) : DASH;

  return {
    title: row.title,
    kind: row.kind,
    id: row.id,
    summary: row.summary,
    equivalents: { cli: cli.length ? cli : [DASH], mcp: mcp.length ? mcp : [DASH], gui: gui[0] ?? DASH, catalog: generic ? 'generic form' : DASH },
    location: {
      crate: row.crate ? (row.layer !== null ? `${row.crate} (L${row.layer})` : row.crate) : 'unresolved',
      source: row.source ?? DASH,
      docs: docsText, // first doc only
    },
    status: {
      coverage: `${cov} \u2014 ${snap}`,
      tier: row.tier ?? DASH,
      sideEffects: row.sideEffect ?? 'unknown',
      domain: `${domainLabel} \u00b7 lane ${row.lane ?? DASH}`,
    },
    related,
    actions: {
      openForm,
      copyMcp: mcpAll[0] ?? null,
      openDocs: row.docs?.startsWith('docs/src/') ? docKey(row.docs) : null,
    },
  };
}
