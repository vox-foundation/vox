export type AtlasLodLevel = 'L0_Crates' | 'L1_Files' | 'L2_Symbols' | 'L3_Docs';
export type AtlasDiffStatus = 'added' | 'modified' | 'unchanged';

export interface AtlasGraphNode {
  id: string;
  label: string;
  kind:
    | 'crate'
    | 'file'
    | 'fn'
    | 'doc'
    | 'adr'
    | 'openwiki'
    | '@table'
    | '@query'
    | '@mutation'
    | '@workflow'
    | '@actor'
    | '@tool'
    | '@island';
  layer: string;
  filePath: string;
  diffStatus: AtlasDiffStatus;
  activeAgentId?: string;
  fanIn: number;
  fanOut: number;
  blastRadiusCount?: number;
}

export interface AtlasGraphEdge {
  id: string;
  source: string;
  target: string;
  kind:
    | 'CrateDep'
    | 'ModDecl'
    | 'UsePath'
    | 'CallCandidate'
    | 'DocumentedIn'
    | 'CitesSymbol'
    | 'GovernedByAdr';
}

export const ATLAS_MAX_NODES_BUDGET = 400;
export const ATLAS_MAX_EDGES_BUDGET = 800;

export function lodToBackendLevel(lod: AtlasLodLevel): 'crate' | 'file' | 'symbol' {
  switch (lod) {
    case 'L0_Crates':
      return 'crate';
    case 'L1_Files':
    case 'L3_Docs':
      return 'file';
    case 'L2_Symbols':
      return 'symbol';
  }
}

function normalizeNodeKind(raw: unknown): AtlasGraphNode['kind'] {
  const s = typeof raw === 'string' ? raw : 'file';
  if (
    s === 'crate' ||
    s === 'file' ||
    s === 'fn' ||
    s === 'doc' ||
    s === 'adr' ||
    s === 'openwiki' ||
    s === '@table' ||
    s === '@query' ||
    s === '@mutation' ||
    s === '@workflow' ||
    s === '@actor' ||
    s === '@tool' ||
    s === '@island'
  ) {
    return s;
  }
  return 'file';
}

function normalizeEdgeKind(raw: unknown): AtlasGraphEdge['kind'] {
  const low = (typeof raw === 'string' ? raw : 'UsePath').toLowerCase();
  if (low === 'crate_dep' || low === 'cratedep') return 'CrateDep';
  if (low === 'mod_decl' || low === 'moddecl') return 'ModDecl';
  if (low === 'call_candidate' || low === 'calls' || low === 'callcandidate') return 'CallCandidate';
  if (low === 'documented_in' || low === 'documentedin') return 'DocumentedIn';
  if (low === 'cites_symbol' || low === 'citessymbol') return 'CitesSymbol';
  if (low === 'governed_by_adr' || low === 'governedbyadr') return 'GovernedByAdr';
  return 'UsePath';
}

export function mapBackendSubgraph(rawPayload: unknown): {
  nodes: AtlasGraphNode[];
  edges: AtlasGraphEdge[];
} {
  if (!rawPayload || typeof rawPayload !== 'object') return { nodes: [], edges: [] };
  const obj = rawPayload as Record<string, unknown>;
  const rawLinks = Array.isArray(obj.links) ? obj.links : Array.isArray(obj.edges) ? obj.edges : [];
  const edges: AtlasGraphEdge[] = rawLinks
    .filter((l): l is Record<string, unknown> => Boolean(l) && typeof l === 'object')
    .map((l, idx) => ({
      id: typeof l.id === 'string' ? l.id : `edge-${idx}`,
      source: String(l.source ?? ''),
      target: String(l.target ?? ''),
      kind: normalizeEdgeKind(l.kind),
    }))
    .filter((e) => e.source.length > 0 && e.target.length > 0);

  const inCounts = new Map<string, number>();
  const outCounts = new Map<string, number>();
  for (const e of edges) {
    outCounts.set(e.source, (outCounts.get(e.source) ?? 0) + 1);
    inCounts.set(e.target, (inCounts.get(e.target) ?? 0) + 1);
  }

  const rawNodes = Array.isArray(obj.nodes) ? obj.nodes : [];
  const nodes: AtlasGraphNode[] = rawNodes
    .filter((n): n is Record<string, unknown> => Boolean(n) && typeof n === 'object')
    .map((n) => {
      const id = String(n.id ?? '');
      const diffRaw = n.diffStatus ?? n.diff_status;
      const diffStatus: AtlasDiffStatus =
        diffRaw === 'added' || diffRaw === 'modified' ? diffRaw : 'unchanged';
      const filePath =
        typeof n.filePath === 'string'
          ? n.filePath
          : typeof n.file_path === 'string'
            ? n.file_path
            : typeof n.path === 'string'
              ? n.path
              : id.split('::')[0] ?? id;
      const blast =
        typeof n.blastRadiusCount === 'number'
          ? n.blastRadiusCount
          : typeof n.blast_s === 'number'
            ? n.blast_s
            : undefined;
      return {
        id,
        label: typeof n.label === 'string' && n.label.length > 0 ? n.label : id,
        kind: normalizeNodeKind(n.kind),
        layer:
          typeof n.layer === 'string' && n.layer.length > 0
            ? n.layer
            : typeof n.community === 'string'
              ? n.community
              : 'workspace',
        filePath,
        diffStatus,
        activeAgentId:
          typeof n.activeAgentId === 'string'
            ? n.activeAgentId
            : typeof n.active_agent_id === 'string'
              ? n.active_agent_id
              : undefined,
        fanIn: typeof n.fanIn === 'number' ? n.fanIn : typeof n.fan_in === 'number' ? n.fan_in : (inCounts.get(id) ?? 0),
        fanOut: typeof n.fanOut === 'number' ? n.fanOut : typeof n.fan_out === 'number' ? n.fan_out : (outCounts.get(id) ?? 0),
        blastRadiusCount: blast,
      };
    })
    .filter((n) => n.id.length > 0);

  return { nodes, edges };
}

export function emitAtlasMentionChip(node: AtlasGraphNode): string {
  const token =
    node.kind === 'doc' || node.kind === 'adr' || node.kind === 'openwiki'
      ? `@doc:${node.filePath}`
      : node.kind === 'crate' || node.kind === 'file'
        ? `@file:${node.filePath}`
        : `@symbol:${node.label.replace(/\s+/g, '_')}`;
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent('vox:atlas-mention', {
        detail: { token, nodeId: node.id, label: node.label, filePath: node.filePath },
      }),
    );
  }
  return token;
}
