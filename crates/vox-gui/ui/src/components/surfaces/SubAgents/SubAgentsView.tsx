import React, { useCallback, useEffect } from 'react';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useSubAgentStore } from './subAgentStore';
import { fetchTree, listenActivity } from './subAgentClient';
import { SubAgentActivityStream } from './SubAgentActivityStream';
import type { SubAgentNode } from './types';
import { ConciliumView } from '../Concilium/ConciliumView';

function findNode(nodes: SubAgentNode[], id: string): SubAgentNode | null {
  for (const n of nodes) {
    if (n.windowId === id) return n;
    const found = findNode(n.children, id);
    if (found) return found;
  }
  return null;
}

type S = ReturnType<typeof useSubAgentStore.getState>;
const selTree = (s: S) => s.tree;
const selSelected = (s: S) => s.selectedWindowId;
const selSetTree = (s: S) => s.setTree;
const selPushEvent = (s: S) => s.pushEvent;

/**
 * Mode 1 Compatibility Route (`surfaceDecorators['sub-agents']`).
 * Consolidates SubAgents onto `<ConciliumView initialFocus="board" />` while preserving
 * `useSubAgentStore`, `fetchTree`, and `listenActivity` contracts.
 */
export function SubAgentsView(props: SurfaceDecoratorProps) {
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
      const w =
        (e.kind as { window_id?: string }).window_id ??
        useSubAgentStore.getState().selectedWindowId;
      if (w) pushEvent(w, e);
    })
      .then((u) => {
        if (disposed) u();
        else un = u;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      un?.();
    };
  }, [pushEvent]);

  const handleSelectAgent = useCallback((id: string) => {
    useSubAgentStore.getState().select(id);
  }, []);

  const handlePushToast = useCallback(
    (msg: string, kind?: 'info' | 'error') => {
      props.pushToast({
        tone: kind === 'error' ? 'error' : 'info',
        title: kind === 'error' ? 'Vox Concilium Error' : 'Vox Concilium',
        body: msg,
        cause: kind === 'error' ? 'backend-error' : 'backend-ok',
      });
    },
    [props.pushToast],
  );

  const renderSelectedAgentExtra = useCallback(
    (windowId: string) =>
      findNode(tree, windowId) ? <SubAgentActivityStream windowId={windowId} /> : null,
    [tree],
  );

  return (
    <ConciliumView
      initialFocus="board"
      subAgentNodes={tree}
      selectedAgentId={selected}
      onSelectAgent={handleSelectAgent}
      pushToast={handlePushToast}
      gamifyEnabled={props.gamifyEnabled}
      renderSelectedAgentExtra={renderSelectedAgentExtra}
    />
  );
}
