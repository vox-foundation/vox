import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { BackendUnavailableError, sanitizeErrorForToast } from '../../../lib/backendGuard';
import { unwrapMcpEnvelope, type McpInvokeResult } from '../../../lib/mcpToolResult';
import { voxTransport } from '../../../transport';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';
import {
  ATLAS_MAX_EDGES_BUDGET,
  ATLAS_MAX_NODES_BUDGET,
  emitAtlasMentionChip,
  lodToBackendLevel,
  mapBackendSubgraph,
  type AtlasGraphEdge,
  type AtlasGraphNode,
  type AtlasLodLevel,
} from './atlasHelpers';

export {
  ATLAS_MAX_EDGES_BUDGET,
  ATLAS_MAX_NODES_BUDGET,
  emitAtlasMentionChip,
  type AtlasDiffStatus,
  type AtlasGraphEdge,
  type AtlasGraphNode,
  type AtlasLodLevel,
} from './atlasHelpers';

export interface CodeAtlasCanvasProps {
  compact?: boolean;
  onMentionNode?: (token: string, node: AtlasGraphNode) => void;
}

export function CodeAtlasCanvas({ compact = false, onMentionNode }: CodeAtlasCanvasProps) {
  const [lod, setLod] = useState<AtlasLodLevel>('L0_Crates');
  const [branchOverlay, setBranchOverlay] = useState<string>('origin/main');
  const [onlyChanged, setOnlyChanged] = useState<boolean>(false);
  const [searchFilter, setSearchFilter] = useState<string>('');
  const [nodes, setNodes] = useState<AtlasGraphNode[]>([]);
  const [edges, setEdges] = useState<AtlasGraphEdge[]>([]);
  const [watcherEvents, setWatcherEvents] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [lastMentionedToken, setLastMentionedToken] = useState<string | null>(null);

  const fetchAtlas = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const res = await voxTransport.invokeMcpTool('vox_search_atlas', {
        level: lodToBackendLevel(lod),
        lod,
        branch_tip: branchOverlay,
        compare_ref: branchOverlay,
        kinds:
          lod === 'L3_Docs'
            ? ['documented_in', 'cites_symbol', 'governed_by_adr', 'DocumentedIn', 'CitesSymbol', 'GovernedByAdr']
            : [],
        limit: ATLAS_MAX_NODES_BUDGET,
        max_nodes: ATLAS_MAX_NODES_BUDGET,
        max_edges: ATLAS_MAX_EDGES_BUDGET,
      });
      if (res && typeof res === 'object' && res.is_error) {
        const inner = res.result as Record<string, unknown> | undefined;
        setErrorMsg(
          sanitizeErrorForToast(
            inner && typeof inner.error === 'string' ? inner.error : 'Atlas query failed',
          ),
        );
        return;
      }
      const data = unwrapMcpEnvelope((res as McpInvokeResult | undefined)?.result);
      const mapped = mapBackendSubgraph(data);
      setNodes(mapped.nodes);
      setEdges(mapped.edges);
      setWatcherEvents((c) => c + 1);
      if (mapped.nodes.length > 0) {
        setSelectedNodeId((prev) =>
          prev && mapped.nodes.some((n) => n.id === prev) ? prev : mapped.nodes[0]!.id,
        );
      }
    } catch (err) {
      if (!(err instanceof BackendUnavailableError) && !(err instanceof Error && err.name === 'BackendUnavailableError')) {
        setErrorMsg(sanitizeErrorForToast(err));
      }
    } finally {
      setLoading(false);
    }
  }, [lod, branchOverlay]);

  useEffect(() => {
    void fetchAtlas();
  }, [fetchAtlas]);

  const visibleNodes = useMemo(() => {
    const filtered = nodes.filter((n) => {
      if (onlyChanged && n.diffStatus === 'unchanged') return false;
      if (
        searchFilter.trim().length > 0 &&
        !n.label.toLowerCase().includes(searchFilter.toLowerCase()) &&
        !n.filePath.toLowerCase().includes(searchFilter.toLowerCase())
      ) {
        return false;
      }
      return true;
    });
    return filtered.slice(0, ATLAS_MAX_NODES_BUDGET);
  }, [nodes, onlyChanged, searchFilter]);

  const visibleEdges = useMemo(() => {
    const nodeIds = new Set(visibleNodes.map((n) => n.id));
    return edges
      .filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
      .slice(0, ATLAS_MAX_EDGES_BUDGET);
  }, [edges, visibleNodes]);

  const selectedNode = useMemo(
    () => visibleNodes.find((n) => n.id === selectedNodeId) ?? visibleNodes[0] ?? null,
    [visibleNodes, selectedNodeId],
  );

  const handleMention = (node: AtlasGraphNode) => {
    const token = emitAtlasMentionChip(node);
    setLastMentionedToken(token);
    onMentionNode?.(token, node);
  };

  return (
    <section
      data-testid="code-atlas-canvas"
      className={`vox-marble-tablet rounded-xl border border-(--color-gold-500)/45 bg-bg-elevated ${compact ? 'p-2.5' : 'p-4'}`}
    >
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2 border-b border-border-strong pb-2.5">
        <div className="flex items-center gap-2">
          <h3 className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
            Unified Code & Architecture Atlas (v5)
          </h3>
          <span
            data-testid="atlas-watcher-badge"
            className="inline-flex items-center gap-1 rounded-full border border-(--color-status-pass)/40 bg-bg-elevated px-2.5 py-0.5 font-mono text-[11px] text-text-primary"
          >
            Live Watcher · {watcherEvents} syncs
          </span>
        </div>

        <div className="flex flex-wrap items-center gap-2 text-[11px]">
          <div title="Level of Detail: Crates (L0), Files (L1), AST Symbols (L2), or Docs + OpenWiki Cross-Edges (L3)">
            <VoxSelect
              size="xs"
              aria-label="Atlas LOD level"
              data-testid="atlas-lod-select"
              value={lod}
              onValueChange={(v) => setLod(v as AtlasLodLevel)}
              options={[
                { value: 'L0_Crates', label: 'LOD: L0 Crates' },
                { value: 'L1_Files', label: 'LOD: L1 Files' },
                { value: 'L2_Symbols', label: 'LOD: L2 Symbols + AST' },
                { value: 'L3_Docs', label: 'LOD: L3 Docs & OpenWiki Cross-Edges' },
              ]}
              className="h-8 w-56 border-border-subtle bg-bg-elevated font-mono text-[11px] text-text-primary"
            />
          </div>

          <div title="Compare graph topology against origin/main, working tree HEAD, or Jujutsu @- parent">
            <VoxSelect
              size="xs"
              aria-label="Branch or worktree diff overlay"
              data-testid="atlas-diff-branch-select"
              value={branchOverlay}
              onValueChange={setBranchOverlay}
              options={[
                { value: 'origin/main', label: 'Diff vs origin/main' },
                { value: 'HEAD', label: 'Working Tree vs HEAD' },
                { value: 'jj:@-', label: 'jj change @-' },
              ]}
              className="h-8 w-48 border-border-subtle bg-bg-elevated font-mono text-[11px] text-text-primary"
            />
          </div>

          <label className="inline-flex items-center gap-1.5 text-xs text-text-secondary">
            <input
              type="checkbox"
              data-testid="atlas-diff-only-toggle"
              checked={onlyChanged}
              onChange={(e) => setOnlyChanged(e.target.checked)}
            />
            Changed only
          </label>

          <button
            type="button"
            data-slot="button"
            data-testid="atlas-sync-watcher-btn"
            title="Materialize and synchronize AST & OpenWiki cross-edges from mounted shards"
            disabled={loading}
            onClick={() => void fetchAtlas()}
            className="inline-flex h-8 items-center rounded border border-border-strong bg-bg-elevated px-3 font-sans text-xs font-medium text-text-primary hover:border-brass/50 disabled:opacity-50"
          >
            {loading ? 'Syncing…' : 'Sync Graph'}
          </button>
        </div>
      </div>

      {errorMsg && (
        <div
          role="alert"
          data-testid="atlas-error-banner"
          className="mb-3 rounded border border-(--color-status-error)/45 bg-(--color-status-error)/10 px-3 py-2 font-mono text-[11px] text-(--color-status-error)"
        >
          {errorMsg}
        </div>
      )}

      <div className="mb-3 flex flex-wrap items-center justify-between gap-2 text-[11px]">
        <input
          type="search"
          aria-label="Filter Atlas nodes"
          data-testid="atlas-node-filter"
          value={searchFilter}
          onChange={(e) => setSearchFilter(e.target.value)}
          placeholder="Filter crates, symbols, OpenWiki docs, or paths…"
          className="h-8 min-w-[200px] flex-1 rounded border border-border-strong bg-bg-surface px-3 font-mono text-xs text-text-primary placeholder:text-text-secondary"
        />
        <span data-testid="atlas-budget-pill" className="font-mono text-[11px] font-medium text-text-secondary">
          Culled Viewport: {visibleNodes.length}/{ATLAS_MAX_NODES_BUDGET} nodes ·{' '}
          {visibleEdges.length}/{ATLAS_MAX_EDGES_BUDGET} edges
        </span>
      </div>

      {visibleNodes.length === 0 ? (
        <div
          data-testid="atlas-empty-state"
          className="rounded-lg border border-dashed border-border-strong bg-bg-surface px-4 py-6 text-center font-mono text-xs font-medium text-text-secondary"
        >
          {loading
            ? 'Querying live Graphify subgraph…'
            : 'Live search shard is active; materialize Graphify atlas edges with Sync Graph or inspect chunks in Memory.'}
        </div>
      ) : (
        <div className={`grid gap-2 ${compact ? 'grid-cols-1' : 'sm:grid-cols-2 lg:grid-cols-3'}`} data-testid="atlas-node-grid">
          {visibleNodes.map((node) => {
            const isSelected = selectedNode?.id === node.id;
            const diffTone =
              node.diffStatus === 'added'
                ? 'border-(--color-status-pass)/45 bg-bg-elevated'
                : node.diffStatus === 'modified'
                  ? 'border-(--color-status-warn)/45 bg-bg-elevated'
                  : 'border-border-subtle bg-bg-elevated';
            return (
              <div
                key={node.id}
                data-testid={`atlas-node-${node.id}`}
                onClick={() => setSelectedNodeId(node.id)}
                className={`cursor-pointer rounded-lg border p-2.5 transition ${diffTone} ${isSelected ? 'ring-1 ring-brass' : ''}`}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <span className="truncate font-mono text-xs font-semibold text-text-primary">{node.label}</span>
                    <div className="mt-0.5 font-mono text-[11px] text-text-muted">
                      {node.layer} · {node.kind}
                    </div>
                  </div>
                  <span
                    data-testid={`atlas-diff-badge-${node.id}`}
                    className="rounded border border-border-subtle bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] uppercase text-text-primary"
                  >
                    {node.diffStatus}
                  </span>
                </div>

                <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
                  {node.activeAgentId && (
                    <span
                      data-testid={`atlas-agent-badge-${node.id}`}
                      className="inline-flex items-center gap-1 rounded border border-brass/35 bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] text-text-primary"
                    >
                      <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />
                      <span>{node.activeAgentId}</span>
                    </span>
                  )}
                  {node.blastRadiusCount !== undefined && (
                    <span
                      data-testid={`atlas-blast-badge-${node.id}`}
                      className="rounded border border-(--color-status-warn)/40 bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] text-(--color-status-warn)"
                    >
                      blast:{node.blastRadiusCount}
                    </span>
                  )}
                </div>

                <div className="mt-2 flex items-center justify-between border-t border-border-subtle pt-1.5 text-[11px] text-text-muted">
                  <span className="font-mono">
                    in:{node.fanIn} · out:{node.fanOut}
                  </span>
                  <button
                    type="button"
                    data-slot="button"
                    data-testid={`atlas-mention-btn-${node.id}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleMention(node);
                    }}
                    className="inline-flex h-7 items-center rounded border border-brass/50 bg-bg-surface px-2.5 font-mono text-[11px] text-text-primary hover:border-brass"
                  >
                    @ Mention in Chat
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {selectedNode && (
        <div
          data-testid="atlas-node-inspector"
          className="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-elevated px-3 py-2 font-mono text-[11px] text-text-primary"
        >
          <div>
            Selected: <strong>{selectedNode.label}</strong> ({selectedNode.filePath}) · Diff vs{' '}
            {branchOverlay}: <span className="font-semibold text-text-primary">{selectedNode.diffStatus}</span>
          </div>
          {lastMentionedToken && (
            <span data-testid="atlas-last-mention-chip" className="font-semibold text-text-primary">
              Inserted chip: {lastMentionedToken}
            </span>
          )}
        </div>
      )}
    </section>
  );
}
