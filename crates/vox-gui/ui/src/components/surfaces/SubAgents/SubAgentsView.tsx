import React, { useEffect } from 'react';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useSubAgentStore } from './subAgentStore';
import { fetchTree, listenActivity } from './subAgentClient';
import { SubAgentTree } from './SubAgentTree';
import { SubAgentActivityStream } from './SubAgentActivityStream';
import type { SubAgentNode } from './types';

/** Find a node by windowId anywhere in the tree (ignores expand state). */
function findNode(nodes: SubAgentNode[], id: string): SubAgentNode | null {
  for (const n of nodes) {
    if (n.windowId === id) return n;
    const found = findNode(n.children, id);
    if (found) return found;
  }
  return null;
}

function countAllNodes(nodes: SubAgentNode[]): { total: number; maxDepth: number } {
  let total = 0;
  let maxDepth = 0;
  const walk = (list: SubAgentNode[]) => {
    for (const n of list) {
      total += 1;
      if (n.depth > maxDepth) maxDepth = n.depth;
      walk(n.children);
    }
  };
  walk(nodes);
  return { total, maxDepth };
}

type S = ReturnType<typeof useSubAgentStore.getState>;
const selTree = (s: S) => s.tree;
const selSelected = (s: S) => s.selectedWindowId;
const selSetTree = (s: S) => s.setTree;
const selPushEvent = (s: S) => s.pushEvent;

export function SubAgentsView(_props: SurfaceDecoratorProps) {
  const tree = useSubAgentStore(selTree);
  const selected = useSubAgentStore(selSelected);
  const setTree = useSubAgentStore(selSetTree);
  const pushEvent = useSubAgentStore(selPushEvent);

  useEffect(() => { let live = true; fetchTree().then((t) => { if (live) setTree(t); }).catch(() => {}); return () => { live = false; }; }, [setTree]);
  useEffect(() => {
    let disposed = false;
    let un: (() => void) | undefined;
    listenActivity((e) => {
      // Backend does not yet stamp window_id on agent events (audit correction #1):
      // route by window_id when present, else attribute to the selected window.
      const w = (e.kind as { window_id?: string }).window_id
        ?? useSubAgentStore.getState().selectedWindowId;
      if (w) pushEvent(w, e);
    })
      .then((u) => {
        // Unmount may win the race against the async subscription resolving.
        if (disposed) u();
        else un = u;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      un?.();
    };
  }, [pushEvent]);

  const node = selected ? findNode(tree, selected) : null;
  const { total: totalNodes, maxDepth } = countAllNodes(tree);

  return (
    <div className="flex h-full gap-4 p-4">
      <div className="flex flex-[1.1] flex-col space-y-4 overflow-auto rounded-xl border border-border-subtle bg-bg-surface/60 p-4">
        <div>
          <div className="mb-3 flex items-center justify-between">
            <span className="font-display text-xs tracking-widest text-text-secondary uppercase">Delegation Tree</span>
            <span className="rounded border border-brass/30 bg-brass/10 px-2 py-0.5 font-mono text-[10px] text-brass">
              {totalNodes} active · depth {maxDepth + (totalNodes > 0 ? 1 : 0)}
            </span>
          </div>
          <SubAgentTree />
        </div>
        <div
          data-testid="subagent-topology-summary"
          className="grid grid-cols-2 gap-2 border-t border-border-subtle pt-3 text-[11px]"
        >
          <div className="rounded-lg border border-border-subtle bg-bg-base/70 p-2.5">
            <div className="font-display text-[9px] tracking-widest text-text-muted uppercase">Isolation Policy</div>
            <div className="mt-1 font-mono text-xs text-emerald-300">Worktree / Scoped</div>
            <div className="mt-0.5 text-[10px] text-text-muted">Parent-child context handoff</div>
          </div>
          <div className="rounded-lg border border-border-subtle bg-bg-base/70 p-2.5">
            <div className="font-display text-[9px] tracking-widest text-text-muted uppercase">Event Bus</div>
            <div className="mt-1 font-mono text-xs text-cyan">vox://agent-events</div>
            <div className="mt-0.5 text-[10px] text-text-muted">Live window_id stream</div>
          </div>
        </div>
      </div>
      <div className="flex flex-[1.4] flex-col justify-between overflow-auto rounded-xl border border-border-subtle bg-bg-surface/60 p-4">
        {node ? (
          <div>
            <h3 className="mb-2 font-mono text-sm font-medium text-text-primary">{node.title}</h3>
            <SubAgentActivityStream windowId={node.windowId} />
          </div>
        ) : (
          <div className="space-y-4">
            <p className="text-xs text-text-muted">Select a sub-agent</p>
            <div
              data-testid="subagent-inspector-overview"
              className="grid grid-cols-1 gap-3 sm:grid-cols-2"
            >
              <div className="rounded-lg border border-border-subtle bg-bg-base/70 p-3">
                <div className="flex items-center justify-between">
                  <span className="font-display text-[10px] tracking-widest text-brass uppercase">
                    Context Pull Protocol
                  </span>
                  <span className="font-mono text-[9px] text-text-muted">Phase D</span>
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
                  Sub-agents inherit scoped file refs and emit content-addressed <code className="text-cyan">context_pull</code> events when reading parent artifacts.
                </p>
              </div>
              <div className="rounded-lg border border-border-subtle bg-bg-base/70 p-3">
                <div className="flex items-center justify-between">
                  <span className="font-display text-[10px] tracking-widest text-cyan uppercase">
                    Token Fate Guard
                  </span>
                  <span className="font-mono text-[9px] text-emerald-300">85% warn</span>
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
                  Tracks per-window token utilization (<code className="text-brass">ok</code> / <code className="text-brass">warn</code> / <code className="text-brass">over</code>) and correlates spawns back to originating chat turns.
                </p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
