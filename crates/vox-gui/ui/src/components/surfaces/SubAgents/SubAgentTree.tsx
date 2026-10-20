import React from 'react';
import { ChevronRight, ChevronDown } from 'lucide-react';
import { useSubAgentStore } from './subAgentStore';
import { flattenTree, tokenFate } from './types';

// Hoisted stable selectors satisfy React 19 useSyncExternalStore getSnapshot
// stability requirement: same function reference = no false positive tearing check.
type S = ReturnType<typeof useSubAgentStore.getState>;
const sel = {
  tree: (s: S) => s.tree,
  expanded: (s: S) => s.expanded,
  selected: (s: S) => s.selectedWindowId,
  toggleExpand: (s: S) => s.toggleExpand,
  select: (s: S) => s.select,
};

export function SubAgentTree() {
  const tree = useSubAgentStore(sel.tree);
  const expanded = useSubAgentStore(sel.expanded);
  const selected = useSubAgentStore(sel.selected);
  const toggleExpand = useSubAgentStore(sel.toggleExpand);
  const select = useSubAgentStore(sel.select);
  const rows = flattenTree(tree, expanded);

  return (
    <div role="tree" aria-label="Sub-agent activity">
      {rows.map((r) => {
        const fate = tokenFate(r.node.usedTokens, r.node.model.maxTokens);
        return (
          <div role="treeitem" key={r.windowId} aria-selected={selected === r.windowId}
               aria-expanded={r.hasChildren ? expanded.has(r.windowId) : undefined}
               style={{ paddingLeft: 8 + r.depth * 16, display: 'flex', gap: 6, alignItems: 'center' }}
               className="mb-1.5 cursor-pointer rounded-lg border border-border-subtle bg-bg-base/70 px-3 py-2 text-xs text-text-primary transition hover:border-white/20"
               onClick={() => select(r.windowId)}>
            {r.hasChildren ? (
              <button aria-label={`expand ${r.node.title}`} onClick={(e) => { e.stopPropagation(); toggleExpand(r.windowId); }}>
                {expanded.has(r.windowId) ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </button>
            ) : <span style={{ width: 14 }} />}
            <span className="font-medium text-text-primary">{r.node.title}</span>
            {r.node.skill && <span className="pill rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[10px] text-brass">{r.node.skill}</span>}
            <span className="ml-auto font-mono text-xs text-text-secondary">{r.node.model.id}</span>
            {r.node.model.maxTokens > 0 && (
              <span data-testid={`budget-${r.windowId}`} data-fate={fate} className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[10px] text-text-secondary">
                {r.node.usedTokens}/{r.node.model.maxTokens}
              </span>
            )}
            <span data-status={r.node.status} className="rounded border border-emerald-400/30 bg-emerald-400/10 px-1.5 py-0.5 font-mono text-[9px] text-emerald-300 uppercase">{r.node.status}</span>
          </div>
        );
      })}
    </div>
  );
}
