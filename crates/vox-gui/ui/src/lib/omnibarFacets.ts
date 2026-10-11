/**
 * Pure faceting for the Omnibar: merge the five sources into capped,
 * provenance-labeled facets. No transport, no DOM. Facets fail independently —
 * a graph-source error is carried on the facet, never propagated to the others.
 *
 * See docs/superpowers/specs/2026-06-27-vox-graph-omnibar-dashboard-design.md §3.3, §6.
 */
import type { FederatedIndexEntry } from './federatedSearchIndex';
import type { UnifiedHit } from '../components/surfaces/Search/searchHelpers';
import type { ContentManifestEntry } from '../hooks/useContentManifest';
import type { SearchableHit } from './searchableRegistry';
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';
import type { RankHit } from './capabilityRank';
import type { PalettePrefixMode } from '../components/layout/paletteSources';
import { LEXICON } from './lexicon';

export const FACET_CAP = 6;

/** Help docs appear in Omnibar only when the query looks help-oriented. */
export function shouldIncludeDocsInOmnibar(
  query: string,
  options?: { prefixMode?: PalettePrefixMode },
): boolean {
  if (options?.prefixMode === 'skills') return true;
  const q = query.trim().toLowerCase();
  if (q.startsWith('/')) return true;
  if (q.startsWith('help ')) return true;
  return /\bhelp\b/.test(q);
}

/** Strip leading `help` tokens so `help cli` matches doc titles like "CLI Reference". */
export function queryForFederatedSearch(query: string): string {
  const trimmed = query.trim();
  const lower = trimmed.toLowerCase();
  if (lower.startsWith('help ')) {
    return trimmed.slice(5).trim();
  }
  if (/\bhelp\b/.test(lower)) {
    return trimmed.replace(/\bhelp\b/gi, '').trim();
  }
  return trimmed;
}

export type FacetKey = 'best' | 'surfaces' | 'commands' | 'tools' | 'onScreen' | 'graph' | 'docs';
export type Provenance = 'manifest' | 'corpus' | 'runtime' | 'graph' | 'docs';
/** Row kind: the capability-index kinds plus the non-index sources (drives the icon). */
export type RowKind =
  | CapabilityIndexEntry['kind']
  | 'setting'
  | 'policy'
  | 'agent'
  | 'action'
  | 'graph'
  | 'onscreen'
  | 'quick';

export interface GraphNeighbor {
  id: string;
  label: string;
  viewKey?: string;
}

export interface OmnibarGraphResult {
  rows: GraphNeighbor[];
  error: string | null;
}

export interface OmnibarSources {
  query: string;
  /** Palette prefix mode (`/` opens docs+skills lane). */
  prefixMode?: PalettePrefixMode;
  federated: FederatedIndexEntry[];
  backendHits: UnifiedHit[];
  manifest: ContentManifestEntry[];
  runtimeHits: SearchableHit[];
  graph: OmnibarGraphResult;
  /** Ranked capability-index hits (every kind; D-05). */
  indexHits?: RankHit[];
  /** Index domain id -> label, for the domain chip. */
  domains?: { id: string; label: string }[];
  /** Installed skill names: an index skill row is a gap-fill only when its name is absent here. */
  installedSkillNames?: ReadonlySet<string>;
}

export type InterfaceChip = 'CLI' | 'GUI' | 'MCP';

/** D-09: every interface a capability row is reachable through. */
export function interfaceChips(row: CapabilityIndexEntry): InterfaceChip[] {
  const chips: InterfaceChip[] = [];
  if (row.cli || row.equivalents.some((e) => e.startsWith('cli.'))) chips.push('CLI');
  if (row.viewKey || row.catalogForm) chips.push('GUI');
  if (row.mcp || row.equivalents.some((e) => e.startsWith('mcp.'))) chips.push('MCP');
  return chips;
}

export type OmnibarActivation =
  | { type: 'navigate'; viewKey: string; anchorId?: string }
  | { type: 'command'; command: string }
  /** Open the Catalog form for an index CLI row (nothing is executed). */
  | { type: 'cli'; rowId: string; path: string[] }
  /** Explain an index row (MCP tool, crate, skill) in the drawer. */
  | { type: 'explain'; rowId: string }
  | { type: 'doc'; path: string }
  | { type: 'graph'; node: GraphNeighbor }
  // finding #5: carry CommandPalette's agents/skills/settings/policies arms so
  // the consolidation does not silently lose them. O4 wires the routing.
  | { type: 'agent'; agentId: string }
  | { type: 'skill'; skillId: string }
  | { type: 'setting'; section: string; settingId: string }
  | { type: 'policy'; policyId: string }
  | { type: 'submit-task' };

export interface OmnibarRow {
  id: string;
  facet: FacetKey;
  label: string;
  detail: string;
  provenance: Provenance;
  activate: OmnibarActivation;
  kind?: RowKind;
  chips?: { interfaces: InterfaceChip[]; domain?: string; chord?: string };
  /** Present on ranked rows: what Best match is picked from. */
  rank?: { tier: number; score: number };
}

export interface OmnibarFacet {
  key: FacetKey;
  label: string;
  provenanceHint: string;
  rows: OmnibarRow[];
  /** Uncapped row count (rows.length when nothing was cut). */
  total: number;
  error: string | null;
}

const FACET_LABELS: Record<FacetKey, string> = {
  best: 'Best match',
  surfaces: 'Surfaces',
  commands: 'Commands',
  tools: 'Tools & crates',
  onScreen: 'On Screen',
  graph: 'Graph',
  docs: 'Docs',
};

const FACET_PROVENANCE_HINT: Record<FacetKey, string> = {
  best: 'ranked',
  surfaces: 'capability index',
  commands: 'index + corpus',
  tools: 'capability index',
  onScreen: 'runtime + manifest',
  graph: 'vox-graph',
  docs: 'docs',
};

/** Facets Best match is chosen from, in tie-break order. */
const RANKED_ORDER: FacetKey[] = ['surfaces', 'commands', 'tools', 'docs'];

/** Doc key and the path handed to onOpenDoc: the text after the last `docs/src/` (read_doc_markdown resolves under it). */
export function docKey(path: string): string {
  const p = path.replace(/\\/g, '/');
  const i = p.lastIndexOf('docs/src/');
  return i < 0 ? p : p.slice(i + 'docs/src/'.length);
}

/** Federated scorer value (100 exact / 80 prefix / 50 substring) -> palette rank. */
function fedRank(score: number | undefined): OmnibarRow['rank'] {
  if (!score) return undefined;
  return { tier: score >= 100 ? 0 : score >= 80 ? 1 : 2, score: score / 100 };
}

function matches(query: string, ...fields: string[]): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return false;
  return fields.some((f) => f.toLowerCase().includes(q));
}

export function buildOmnibarFacets(input: OmnibarSources): OmnibarFacet[] {
  // D-10: `#` shows crates only, so every other source is silenced.
  const src: OmnibarSources =
    input.prefixMode === 'crates'
      ? { ...input, federated: [], backendHits: [], manifest: [], runtimeHits: [], graph: { rows: [], error: null } }
      : input;
  const domainLabel = (id: string | null) => (id ? src.domains?.find((d) => d.id === id)?.label : undefined);
  const indexChips = (row: CapabilityIndexEntry): NonNullable<OmnibarRow['chips']> => {
    const domain = domainLabel(row.domain);
    return domain ? { interfaces: interfaceChips(row), domain } : { interfaces: interfaceChips(row) };
  };
  const indexHits = src.indexHits ?? [];
  const rankOf = (h: RankHit) => ({ tier: h.tier, score: h.score });
  const includeAllDocs = shouldIncludeDocsInOmnibar(src.query, { prefixMode: src.prefixMode });

  // SURFACES — index surface rows replace the federated/manifest ones (D-05); the others only
  // add viewKeys the index lacks.
  const surfaceRows: OmnibarRow[] = [];
  const surfaceKeys = new Set<string>();
  for (const h of indexHits) {
    const r = h.row;
    if (r.kind !== 'surface' || !r.viewKey || surfaceKeys.has(r.viewKey)) continue;
    surfaceKeys.add(r.viewKey);
    surfaceRows.push({
      id: `index:${r.id}`,
      facet: 'surfaces',
      label: LEXICON[r.viewKey]?.en ?? r.navLabel ?? r.title,
      detail: r.summary ?? '',
      provenance: 'manifest',
      activate: { type: 'navigate', viewKey: r.viewKey },
      kind: 'surface',
      chips: indexChips(r),
      rank: rankOf(h),
    });
  }
  for (const e of src.federated) {
    if (e.kind !== 'surface' || e.payload.type !== 'surface' || surfaceKeys.has(e.payload.viewKey)) continue;
    surfaceKeys.add(e.payload.viewKey);
    surfaceRows.push({
      id: e.id,
      facet: 'surfaces',
      label: e.label,
      detail: e.detail,
      provenance: 'manifest',
      activate: { type: 'navigate', viewKey: e.payload.viewKey },
      kind: 'surface',
      rank: fedRank(e.score),
    });
  }
  for (const m of src.manifest) {
    if (!matches(src.query, m.label, ...m.headings) || surfaceKeys.has(m.viewKey)) continue;
    surfaceKeys.add(m.viewKey);
    surfaceRows.push({
      id: `manifest-surface:${m.viewKey}`,
      facet: 'surfaces',
      label: m.label,
      detail: m.route,
      provenance: 'manifest',
      activate: { type: 'navigate', viewKey: m.viewKey },
      kind: 'surface',
    });
  }

  // COMMANDS — index cli rows, backend command hits the index lacks, and the settings /
  // policies / installed skills / index-only skills (finding #5: never silently lose them).
  const commandRows: OmnibarRow[] = [];
  const cliPaths = new Set<string>();
  for (const h of indexHits) {
    const r = h.row;
    if (r.kind !== 'cli' || !r.cli) continue;
    const path = r.cli.split(' ').slice(1);
    cliPaths.add(path.join(' '));
    commandRows.push({
      id: `index:${r.id}`,
      facet: 'commands',
      label: r.cli,
      detail: r.summary ?? r.title,
      provenance: 'corpus',
      activate: { type: 'cli', rowId: r.id, path },
      kind: 'cli',
      chips: indexChips(r),
      rank: rankOf(h),
    });
  }
  for (const h of src.backendHits) {
    if (h.kind !== 'command') continue;
    const p = (h.path ?? '').replace(/^vox\s+/, '');
    if (cliPaths.has(p)) continue;
    commandRows.push({
      id: `cmd:${h.path}`,
      facet: 'commands',
      label: h.title ?? h.path ?? '',
      detail: h.snippet,
      provenance: 'corpus',
      activate: { type: 'command', command: String(h.locator.value ?? h.path) },
    });
  }
  for (const e of src.federated) {
    const base = { id: e.id, facet: 'commands' as const, label: e.label, detail: e.detail, provenance: 'corpus' as const, rank: fedRank(e.score) };
    if (e.kind === 'command' && e.payload.type === 'command') {
      commandRows.push({ ...base, activate: { type: 'command', command: e.payload.command } });
    } else if (e.kind === 'action' && e.payload.type === 'action') {
      commandRows.push({ ...base, kind: 'action', activate: { type: 'command', command: e.payload.actionId } });
    } else if (e.kind === 'setting' && e.payload.type === 'setting') {
      commandRows.push({ ...base, kind: 'setting', activate: { type: 'setting', section: e.payload.section, settingId: e.payload.settingId } });
    } else if (e.kind === 'policy' && e.payload.type === 'policy') {
      commandRows.push({ ...base, kind: 'policy', activate: { type: 'policy', policyId: e.payload.policyId } });
    } else if (e.kind === 'skill' && e.payload.type === 'skill') {
      commandRows.push({ ...base, kind: 'skill', activate: { type: 'skill', skillId: e.payload.skillId } });
    }
  }
  for (const h of indexHits) {
    const r = h.row;
    const name = r.id.replace(/^skill\./, '');
    if (r.kind !== 'skill' || src.installedSkillNames?.has(name)) continue;
    commandRows.push({
      id: `index:${r.id}`,
      facet: 'commands',
      label: r.title,
      detail: r.summary ?? '',
      provenance: 'corpus',
      activate: { type: 'explain', rowId: r.id },
      kind: 'skill',
      chips: indexChips(r),
      rank: rankOf(h),
    });
  }

  // TOOLS & CRATES (D-08a) — MCP tools and crates, always from the index.
  const toolRows: OmnibarRow[] = [];
  for (const h of indexHits) {
    const r = h.row;
    if (r.kind !== 'mcp_tool' && r.kind !== 'crate') continue;
    const label = (r.kind === 'mcp_tool' ? r.mcp : r.crate) ?? r.title;
    toolRows.push({
      id: `index:${r.id}`,
      facet: 'tools',
      label,
      detail: r.summary ?? (r.title === label ? '' : r.title),
      provenance: 'corpus',
      activate: { type: 'explain', rowId: r.id },
      kind: r.kind,
      chips: indexChips(r),
      rank: rankOf(h),
    });
  }

  // ON SCREEN — runtime registry hits + manifest copy/headings.
  const onScreenRows: OmnibarRow[] = [];
  for (const r of src.runtimeHits) {
    onScreenRows.push({
      id: `runtime:${r.surfaceId}:${r.label}`,
      facet: 'onScreen',
      label: r.label,
      detail: r.detail ?? '',
      provenance: 'runtime',
      activate: { type: 'navigate', viewKey: r.viewKey, anchorId: r.anchorId },
      kind: 'onscreen',
    });
  }
  for (const m of src.manifest) {
    const text = [...m.copy, ...m.headings].find((t) => matches(src.query, t));
    if (!text) continue;
    onScreenRows.push({
      id: `manifest-copy:${m.viewKey}:${text}`,
      facet: 'onScreen',
      label: text,
      detail: m.label,
      provenance: 'manifest',
      activate: { type: 'navigate', viewKey: m.viewKey },
      kind: 'onscreen',
    });
  }

  // GRAPH — graph-discover neighbors; error carried, never propagated.
  const graphRows: OmnibarRow[] = src.graph.error
    ? []
    : src.graph.rows.map((n) => ({
        id: `graph:${n.id}`,
        facet: 'graph' as const,
        label: n.label,
        detail: 'relates to',
        provenance: 'graph' as const,
        activate: { type: 'graph' as const, node: n },
        kind: 'graph' as const,
      }));

  // DOCS — federated doc entries first; index docs only fill the gaps (D-05). Without a
  // help-like query only title-exact/prefix matches show.
  const docRows: OmnibarRow[] = [];
  const docKeys = new Set<string>();
  for (const e of src.federated) {
    if (e.kind !== 'doc' || e.payload.type !== 'doc') continue;
    if (!includeAllDocs && (e.score ?? 0) < 80) continue;
    docKeys.add(docKey(e.payload.path));
    docRows.push({
      id: e.id,
      facet: 'docs',
      label: e.label,
      detail: e.detail,
      provenance: 'docs',
      activate: { type: 'doc', path: e.payload.path },
      kind: 'doc',
      rank: fedRank(e.score),
    });
  }
  for (const h of indexHits) {
    const r = h.row;
    if (r.kind !== 'doc') continue;
    if (!includeAllDocs && h.tier > 1) continue;
    const key = docKey(r.docs ?? r.id.replace(/^doc\./, ''));
    if (docKeys.has(key)) continue;
    docKeys.add(key);
    docRows.push({
      id: `index:${r.id}`,
      facet: 'docs',
      label: r.title,
      detail: r.summary ?? '',
      provenance: 'docs',
      activate: { type: 'doc', path: key },
      kind: 'doc',
      chips: indexChips(r),
      rank: rankOf(h),
    });
  }

  // BEST MATCH (D-08) — the single best ranked row across facets, moved out of its facet.
  const lists: Record<string, OmnibarRow[]> = { surfaces: surfaceRows, commands: commandRows, tools: toolRows, docs: docRows };
  type Pick = { row: OmnibarRow; list: OmnibarRow[]; pos: number };
  const bestRows: OmnibarRow[] = [];
  let pick: Pick | undefined;
  // Ties on (tier, score to 2 dp) resolve by facet order, then rank order: strict < keeps the first.
  const beats = (x: OmnibarRow, y: OmnibarRow) =>
    (x.rank!.tier - y.rank!.tier || Math.round(y.rank!.score * 100) - Math.round(x.rank!.score * 100)) < 0;
  for (const key of RANKED_ORDER) {
    lists[key].forEach((row, pos) => {
      if (row.rank && (!pick || beats(row, pick.row))) pick = { row, list: lists[key], pos };
    });
  }
  const chosen = pick as Pick | undefined;
  if (chosen) {
    chosen.list.splice(chosen.pos, 1);
    bestRows.push({ ...chosen.row, facet: 'best' });
  }

  const make = (key: FacetKey, rows: OmnibarRow[], error: string | null = null): OmnibarFacet => ({
    key,
    label: FACET_LABELS[key],
    provenanceHint: FACET_PROVENANCE_HINT[key],
    rows: rows.slice(0, FACET_CAP),
    total: rows.length,
    error,
  });
  return [
    make('best', bestRows),
    make('surfaces', surfaceRows),
    make('commands', commandRows),
    make('tools', toolRows),
    make('onScreen', onScreenRows),
    make('graph', graphRows, src.graph.error),
    make('docs', docRows),
  ];
}

/** Flattened, facet-ordered rows (Best match → Surfaces → Commands → Tools & crates → On-Screen → Graph → Docs). */
export function omnibarRowsInOrder(facets: OmnibarFacet[]): OmnibarRow[] {
  return facets.flatMap((f) => f.rows);
}
