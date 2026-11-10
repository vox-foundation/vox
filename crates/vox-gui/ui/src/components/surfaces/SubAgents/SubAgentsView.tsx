import React, { useEffect } from 'react';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useSubAgentStore } from './subAgentStore';
import { fetchTree, listenActivity } from './subAgentClient';
import { SubAgentTree } from './SubAgentTree';
import { SubAgentActivityStream } from './SubAgentActivityStream';
import type { SubAgentNode } from './types';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';

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

function collectAllNodes(nodes: SubAgentNode[]): SubAgentNode[] {
  const out: SubAgentNode[] = [];
  const walk = (list: SubAgentNode[]) => {
    for (const n of list) {
      out.push(n);
      walk(n.children);
    }
  };
  walk(nodes);
  return out;
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

  useEffect(() => {
    let live = true;
    fetchTree()
      .then((t) => {
        if (!live) return;
        setTree(t);
        const st = useSubAgentStore.getState();
        if (st.expanded.size === 0) {
          for (const root of t) {
            if (root.children.length > 0) {
              st.toggleExpand(root.windowId);
            }
          }
        }
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [setTree]);
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
  const allNodes = collectAllNodes(tree);

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <RomanSurfaceHeader
        domainTag="AGENTS · SUB-AGENTS"
        title="Centuria Delegation Hierarchy & Context Pull Inspector"
        subtitle="WORKTREE SCOPED ISOLATION · ORIGIN TURN CORRELATION · TOKEN FATE GUARD"
        waxSealBadge="HIERARCHY LIVE"
        icon={<Icon.flow className="size-4 text-[#F3E5AB]" />}
        gauges={[
          { label: 'Active Windows', valueText: `${totalNodes}`, ratio: Math.min(1, Math.max(0.2, totalNodes / 8)), tone: 'gold' },
          { label: 'Max Depth', valueText: `${maxDepth + (totalNodes > 0 ? 1 : 0)}`, ratio: Math.min(1, (maxDepth + 1) / 4), tone: 'emerald' },
          { label: 'Fate Guard', valueText: '85% WARN', ratio: 0.85, tone: 'gold' },
        ]}
      />
      <div className="flex min-h-0 flex-1 gap-4">
        <div className="flex flex-[1.1] flex-col space-y-4 overflow-auto rounded-xl border border-[#D4AF37]/40 bg-bg-surface p-4 shadow-[inset_0_1px_0_rgba(243,229,171,0.16)]">
          <div>
            <div className="mb-3 flex items-center justify-between">
              <span className="font-display text-xs tracking-widest text-text-primary uppercase">Delegation Tree</span>
              <RomanWaxSealBadge
                label={`${totalNodes} active · depth ${maxDepth + (totalNodes > 0 ? 1 : 0)}`}
                tone="gold"
              />
            </div>
            <SubAgentTree />
          </div>
          <div
            data-testid="subagent-topology-summary"
            className="grid grid-cols-2 gap-2 border-t border-[#D4AF37]/25 pt-3 text-[11px]"
          >
            <div className="rounded-lg border border-[#D4AF37]/30 bg-bg-elevated p-2.5">
              <div className="font-display text-[9px] tracking-widest text-text-secondary uppercase">Isolation Policy</div>
              <div className="mt-1 font-mono text-xs text-emerald-600">Worktree / Scoped</div>
              <div className="mt-0.5 text-[10px] text-text-secondary">Parent-child context handoff</div>
            </div>
            <div className="rounded-lg border border-[#D4AF37]/30 bg-bg-elevated p-2.5">
              <div className="font-display text-[9px] tracking-widest text-text-secondary uppercase">Event Bus</div>
              <div className="mt-1 font-mono text-xs text-brass">vox://agent-events</div>
              <div className="mt-0.5 text-[10px] text-text-secondary">Live window_id stream</div>
            </div>
          </div>
          {allNodes.length > 0 && (
            <div className="space-y-2 border-t border-[#D4AF37]/25 pt-3">
              <div className="flex items-center justify-between">
                <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">
                  Active Delegation Roster
                </span>
                <span className="font-mono text-[10px] text-text-secondary">
                  {allNodes.length} window{allNodes.length === 1 ? '' : 's'}
                </span>
              </div>
              <div className="space-y-1.5">
                {allNodes.map((item) => (
                  <div
                    key={`roster-${item.windowId}`}
                    className="flex items-center justify-between rounded-lg border border-[#D4AF37]/30 bg-bg-elevated px-2.5 py-1.5 text-[11px]"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="rounded border border-[#D4AF37]/40 bg-[#D4AF37]/15 px-1.5 py-0.5 font-mono text-[10px] text-text-primary shrink-0">
                        {item.windowId}
                      </span>
                      <span className="truncate font-mono text-text-secondary">
                        {item.parentWindowId ? `← ${item.parentWindowId}` : 'root'}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <span className="rounded border border-[#D4AF37]/30 bg-bg-surface px-1.5 py-0.5 font-mono text-[9px] text-text-primary">
                        d={item.depth}
                      </span>
                      <span className="rounded border border-emerald-400/30 bg-emerald-400/10 px-1.5 py-0.5 font-mono text-[9px] uppercase text-emerald-600">
                        {item.status}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
        <div className="flex flex-[1.4] flex-col justify-between overflow-auto rounded-xl border border-[#D4AF37]/40 bg-bg-surface p-4 shadow-[inset_0_1px_0_rgba(243,229,171,0.16)]">
          {node ? (
            <div>
              <h3 className="mb-2 font-mono text-sm font-medium text-text-primary">{node.title}</h3>
              <SubAgentActivityStream windowId={node.windowId} />
            </div>
          ) : (
            <div className="space-y-4">
              <div className="flex items-center justify-between rounded-lg border border-[#D4AF37]/35 bg-bg-elevated px-3 py-2">
                <p className="text-xs text-text-secondary">Select a sub-agent</p>
                <RomanWaxSealBadge label="INSPECTOR READY" tone="emerald" />
              </div>
              <div
                data-testid="subagent-inspector-overview"
                className="grid grid-cols-1 gap-3 sm:grid-cols-2"
              >
                <div className="rounded-lg border border-[#D4AF37]/35 bg-bg-elevated p-3">
                  <div className="flex items-center justify-between">
                    <span className="font-display text-[10px] tracking-widest text-brass uppercase">
                      Context Pull Protocol
                    </span>
                    <span className="font-mono text-[9px] text-text-secondary">Phase D</span>
                  </div>
                  <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
                    Sub-agents inherit scoped file refs and emit content-addressed <code className="text-text-primary">context_pull</code> events when reading parent artifacts.
                  </p>
                </div>
                <div className="rounded-lg border border-[#D4AF37]/35 bg-bg-elevated p-3">
                  <div className="flex items-center justify-between">
                    <span className="font-display text-[10px] tracking-widest text-brass uppercase">
                      Token Fate Guard
                    </span>
                    <span className="font-mono text-[9px] text-emerald-600">85% warn</span>
                  </div>
                  <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
                    Tracks per-window token utilization (<code className="text-text-primary">ok</code> / <code className="text-text-primary">warn</code> / <code className="text-text-primary">over</code>) and correlates spawns back to originating chat turns.
                  </p>
                </div>
              </div>
              <div className="rounded-lg border border-[#D4AF37]/35 bg-bg-elevated p-3">
                <div className="mb-2 flex items-center justify-between">
                  <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">
                    Handoff & Escalation Telemetry
                  </span>
                  <span className="rounded border border-[#D4AF37]/35 bg-bg-surface px-1.5 py-0.5 font-mono text-[9px] text-text-primary">
                    list_subagent_tree
                  </span>
                </div>
                <div className="grid grid-cols-3 gap-2 text-[11px]">
                  <div className="rounded border border-[#D4AF37]/30 bg-bg-surface p-2">
                    <div className="font-mono text-[9px] uppercase text-text-secondary">Spawn Correlation</div>
                    <div className="mt-0.5 font-mono text-xs text-text-primary">origin_turn_id</div>
                  </div>
                  <div className="rounded border border-[#D4AF37]/30 bg-bg-surface p-2">
                    <div className="font-mono text-[9px] uppercase text-text-secondary">Session Scope</div>
                    <div className="mt-0.5 font-mono text-xs text-brass">chat_session_id</div>
                  </div>
                  <div className="rounded border border-[#D4AF37]/30 bg-bg-surface p-2">
                    <div className="font-mono text-[9px] uppercase text-text-secondary">Ring Buffer</div>
                    <div className="mt-0.5 font-mono text-xs text-emerald-600">200 events/win</div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
