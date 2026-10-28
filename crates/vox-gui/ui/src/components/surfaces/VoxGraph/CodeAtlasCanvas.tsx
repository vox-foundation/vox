import React, { useMemo, useState } from 'react';
import { voxTransport } from '../../../transport';

export type AtlasLodLevel = 'L0_Crates' | 'L1_Files' | 'L2_Symbols';
export type AtlasDiffStatus = 'added' | 'modified' | 'unchanged';

export interface AtlasGraphNode {
  id: string;
  label: string;
  kind:
    | 'crate'
    | 'file'
    | 'fn'
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
}

export interface AtlasGraphEdge {
  id: string;
  source: string;
  target: string;
  kind: 'CrateDep' | 'ModDecl' | 'UsePath' | 'CallCandidate';
}

export const ATLAS_MAX_NODES_BUDGET = 400;
export const ATLAS_MAX_EDGES_BUDGET = 800;

const DEFAULT_ATLAS_NODES: AtlasGraphNode[] = [
  {
    id: 'crate:vox-compiler',
    label: 'vox-compiler',
    kind: 'crate',
    layer: 'L1-compiler',
    filePath: 'crates/vox-compiler/src/lib.rs',
    diffStatus: 'modified',
    activeAgentId: 'agent:compiler-verifier',
    fanIn: 14,
    fanOut: 5,
  },
  {
    id: 'crate:vox-orchestrator',
    label: 'vox-orchestrator',
    kind: 'crate',
    layer: 'L2-orchestration',
    filePath: 'crates/vox-orchestrator/src/lib.rs',
    diffStatus: 'modified',
    activeAgentId: 'agent:autonomy-guard',
    fanIn: 9,
    fanOut: 8,
  },
  {
    id: 'crate:vox-search',
    label: 'vox-search',
    kind: 'crate',
    layer: 'L2-retrieval',
    filePath: 'crates/vox-search/src/lib.rs',
    diffStatus: 'added',
    fanIn: 7,
    fanOut: 4,
  },
  {
    id: 'vox:sym:@tool:transcode_video',
    label: '@tool transcode_video',
    kind: '@tool',
    layer: 'L3-app',
    filePath: 'examples/media_pipeline.vox',
    diffStatus: 'added',
    fanIn: 3,
    fanOut: 2,
  },
  {
    id: 'vox:sym:@table:trial_campaigns',
    label: '@table trial_campaigns',
    kind: '@table',
    layer: 'L1-db',
    filePath: 'crates/vox-db/src/schema/domains/trials.rs',
    diffStatus: 'modified',
    fanIn: 6,
    fanOut: 1,
  },
];

const DEFAULT_ATLAS_EDGES: AtlasGraphEdge[] = [
  {
    id: 'e1',
    source: 'crate:vox-orchestrator',
    target: 'crate:vox-compiler',
    kind: 'CrateDep',
  },
  {
    id: 'e2',
    source: 'crate:vox-orchestrator',
    target: 'crate:vox-search',
    kind: 'CrateDep',
  },
  {
    id: 'e3',
    source: 'vox:sym:@tool:transcode_video',
    target: 'crate:vox-compiler',
    kind: 'CallCandidate',
  },
  {
    id: 'e4',
    source: 'crate:vox-search',
    target: 'vox:sym:@table:trial_campaigns',
    kind: 'UsePath',
  },
];

/**
 * Dispatch an `@` mention chip from the Code & Architecture Atlas to `Loquela.tsx` (Phase 30).
 */
export function emitAtlasMentionChip(node: AtlasGraphNode): string {
  const token =
    node.kind === 'crate' || node.kind === 'file'
      ? `@file:${node.filePath}`
      : `@symbol:${node.label.replace(/\s+/g, '_')}`;
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent('vox:atlas-mention', {
        detail: {
          token,
          nodeId: node.id,
          label: node.label,
          filePath: node.filePath,
        },
      }),
    );
  }
  return token;
}

export interface CodeAtlasCanvasProps {
  compact?: boolean;
  onMentionNode?: (token: string, node: AtlasGraphNode) => void;
}

export function CodeAtlasCanvas({ compact = false, onMentionNode }: CodeAtlasCanvasProps) {
  const [lod, setLod] = useState<AtlasLodLevel>('L0_Crates');
  const [branchOverlay, setBranchOverlay] = useState<string>('origin/main');
  const [onlyChanged, setOnlyChanged] = useState<boolean>(false);
  const [searchFilter, setSearchFilter] = useState<string>('');
  const [watcherEvents, setWatcherEvents] = useState<number>(3);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(DEFAULT_ATLAS_NODES[0].id);
  const [lastMentionedToken, setLastMentionedToken] = useState<string | null>(null);

  const visibleNodes = useMemo(() => {
    const filtered = DEFAULT_ATLAS_NODES.filter((n) => {
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
  }, [onlyChanged, searchFilter]);

  const visibleEdges = useMemo(() => {
    const nodeIds = new Set(visibleNodes.map((n) => n.id));
    return DEFAULT_ATLAS_EDGES.filter(
      (e) => nodeIds.has(e.source) && nodeIds.has(e.target),
    ).slice(0, ATLAS_MAX_EDGES_BUDGET);
  }, [visibleNodes]);

  const selectedNode = useMemo(
    () => visibleNodes.find((n) => n.id === selectedNodeId) ?? visibleNodes[0] ?? null,
    [visibleNodes, selectedNodeId],
  );

  const handleMention = (node: AtlasGraphNode) => {
    const token = emitAtlasMentionChip(node);
    setLastMentionedToken(token);
    onMentionNode?.(token, node);
  };

  const handleSyncWatcher = () => {
    setWatcherEvents((c) => c + 1);
    if (typeof voxTransport.invokeMcpTool === 'function') {
      voxTransport
        .invokeMcpTool('vox_search_atlas', {
          lod,
          compare_ref: branchOverlay,
          max_nodes: ATLAS_MAX_NODES_BUDGET,
          max_edges: ATLAS_MAX_EDGES_BUDGET,
        })
        .catch(() => {});
    }
  };

  return (
    <section
      data-testid="code-atlas-canvas"
      className={`rounded-xl border border-white/10 bg-zinc-950/60 ${compact ? 'p-2.5' : 'p-4'}`}
    >
      {/* Header & Controls: Phase 26 LOD, Phase 27 Branch/Worktree Diff Overlay, Phase 29 Watcher */}
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2 border-b border-white/10 pb-2.5">
        <div className="flex items-center gap-2">
          <h3 className="font-display text-xs font-semibold uppercase tracking-wider text-amber-300">
            Unified Code & Architecture Atlas (v5)
          </h3>
          <span
            data-testid="atlas-watcher-badge"
            className="inline-flex items-center gap-1 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-mono text-[10px] text-emerald-300"
          >
            Live Watcher · {watcherEvents} events
          </span>
        </div>

        <div className="flex flex-wrap items-center gap-2 text-[11px]">
          <select
            aria-label="Atlas LOD level"
            data-testid="atlas-lod-select"
            value={lod}
            onChange={(e) => setLod(e.target.value as AtlasLodLevel)}
            className="rounded border border-white/10 bg-zinc-900 px-2 py-1 font-mono text-[11px] text-zinc-200"
          >
            <option value="L0_Crates">LOD: L0 Crates</option>
            <option value="L1_Files">LOD: L1 Files</option>
            <option value="L2_Symbols">LOD: L2 Symbols + .vox AST</option>
          </select>

          <select
            aria-label="Branch or worktree diff overlay"
            data-testid="atlas-diff-branch-select"
            value={branchOverlay}
            onChange={(e) => setBranchOverlay(e.target.value)}
            className="rounded border border-white/10 bg-zinc-900 px-2 py-1 font-mono text-[11px] text-zinc-200"
          >
            <option value="origin/main">Diff vs origin/main</option>
            <option value="feat/unified-2026-q4">Worktree: feat/unified-2026-q4</option>
            <option value="jj:@-">jj change @-</option>
          </select>

          <label className="inline-flex items-center gap-1 text-[11px] text-zinc-300">
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
            data-testid="atlas-sync-watcher-btn"
            onClick={handleSyncWatcher}
            className="rounded border border-cyan-500/30 bg-cyan-500/10 px-2 py-1 font-mono text-[10px] text-cyan-300 hover:bg-cyan-500/20"
          >
            Sync Graph
          </button>
        </div>
      </div>

      {/* Filter & Budget Bar */}
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2 text-[11px]">
        <input
          type="search"
          aria-label="Filter Atlas nodes"
          data-testid="atlas-node-filter"
          value={searchFilter}
          onChange={(e) => setSearchFilter(e.target.value)}
          placeholder="Filter crates, .vox @table/@tool/@workflow symbols, or paths…"
          className="min-w-[200px] flex-1 rounded border border-white/10 bg-zinc-900/80 px-2.5 py-1 font-mono text-[11px] text-zinc-200"
        />
        <span data-testid="atlas-budget-pill" className="font-mono text-[10px] text-zinc-400">
          Culled Viewport: {visibleNodes.length}/{ATLAS_MAX_NODES_BUDGET} nodes ·{' '}
          {visibleEdges.length}/{ATLAS_MAX_EDGES_BUDGET} edges
        </span>
      </div>

      {/* Viewport-Culled Node Grid + Active Agent Badges + Click-to-@ Chip */}
      <div
        className={`grid gap-2 ${compact ? 'grid-cols-1' : 'sm:grid-cols-2 lg:grid-cols-3'}`}
        data-testid="atlas-node-grid"
      >
        {visibleNodes.map((node) => {
          const isSelected = selectedNode?.id === node.id;
          const diffTone =
            node.diffStatus === 'added'
              ? 'border-emerald-500/40 bg-emerald-500/5'
              : node.diffStatus === 'modified'
                ? 'border-amber-500/40 bg-amber-500/5'
                : 'border-white/10 bg-zinc-900/40';
          return (
            <div
              key={node.id}
              data-testid={`atlas-node-${node.id}`}
              onClick={() => setSelectedNodeId(node.id)}
              className={`cursor-pointer rounded-lg border p-2.5 transition ${diffTone} ${
                isSelected ? 'ring-1 ring-amber-400' : ''
              }`}
            >
              <div className="flex items-start justify-between gap-2">
                <div>
                  <span className="font-mono text-xs font-semibold text-zinc-100">
                    {node.label}
                  </span>
                  <div className="mt-0.5 font-mono text-[10px] text-zinc-400">
                    {node.layer} · {node.kind}
                  </div>
                </div>
                <span
                  data-testid={`atlas-diff-badge-${node.id}`}
                  className="rounded px-1.5 py-0.5 font-mono text-[10px] uppercase text-amber-300 bg-amber-500/10"
                >
                  {node.diffStatus}
                </span>
              </div>

              {/* Phase 29: Active Agent Badge */}
              {node.activeAgentId && (
                <div
                  data-testid={`atlas-agent-badge-${node.id}`}
                  className="mt-1.5 inline-flex items-center gap-1 rounded bg-sky-500/15 px-1.5 py-0.5 font-mono text-[10px] text-sky-300"
                >
                  ⚡ {node.activeAgentId}
                </div>
              )}

              <div className="mt-2 flex items-center justify-between border-t border-white/5 pt-1.5 text-[10px] text-zinc-400">
                <span className="font-mono">
                  in:{node.fanIn} · out:{node.fanOut}
                </span>
                {/* Phase 30: Click-to-@ Composer Chip */}
                <button
                  type="button"
                  data-testid={`atlas-mention-btn-${node.id}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    handleMention(node);
                  }}
                  className="rounded border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 font-mono text-[10px] text-amber-300 hover:bg-amber-500/20"
                >
                  @ Mention in Chat
                </button>
              </div>
            </div>
          );
        })}
      </div>

      {/* Selected Node Inspector & Mention Confirmation */}
      {selectedNode && (
        <div
          data-testid="atlas-node-inspector"
          className="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-white/10 bg-zinc-900/60 px-3 py-2 font-mono text-[11px] text-zinc-300"
        >
          <div>
            Selected: <strong>{selectedNode.label}</strong> ({selectedNode.filePath}) · Diff vs{' '}
            {branchOverlay}: <span className="text-amber-300">{selectedNode.diffStatus}</span>
          </div>
          {lastMentionedToken && (
            <span data-testid="atlas-last-mention-chip" className="text-emerald-300">
              Inserted chip: {lastMentionedToken}
            </span>
          )}
        </div>
      )}
    </section>
  );
}
