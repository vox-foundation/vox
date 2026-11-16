import React from 'react';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '../../ui/Resizable';
import { TerminalSessionPane } from './TerminalSessionPane';

export type PaneLeafNode = { kind: 'leaf'; paneId: string; sessionId: string | null };
export type PaneSplitNode = {
  kind: 'split';
  id: string;
  orientation: 'horizontal' | 'vertical';
  first: PaneNode;
  second: PaneNode;
};
export type PaneNode = PaneLeafNode | PaneSplitNode;

export const MAX_SPLIT_PANES = 4;

export function countPaneLeaves(node: PaneNode): number {
  return node.kind === 'leaf' ? 1 : countPaneLeaves(node.first) + countPaneLeaves(node.second);
}

export function findFirstLeaf(node: PaneNode): PaneLeafNode {
  return node.kind === 'leaf' ? node : findFirstLeaf(node.first);
}

export function findPaneLeaf(node: PaneNode, paneId: string): PaneLeafNode | null {
  if (node.kind === 'leaf') return node.paneId === paneId ? node : null;
  return findPaneLeaf(node.first, paneId) ?? findPaneLeaf(node.second, paneId);
}

export function findPaneLeafBySession(node: PaneNode, sessionId: string): PaneLeafNode | null {
  if (node.kind === 'leaf') return node.sessionId === sessionId ? node : null;
  return findPaneLeafBySession(node.first, sessionId) ?? findPaneLeafBySession(node.second, sessionId);
}

function splitLeafUnchecked(
  node: PaneNode,
  targetPaneId: string,
  orientation: 'horizontal' | 'vertical',
  newPaneId: string,
  newSessionId: string | null,
): PaneNode {
  if (node.kind === 'leaf') {
    if (node.paneId !== targetPaneId) return node;
    return {
      kind: 'split',
      id: `split-${node.paneId}-${newPaneId}`,
      orientation,
      first: node,
      second: { kind: 'leaf', paneId: newPaneId, sessionId: newSessionId },
    };
  }
  const nextFirst = splitLeafUnchecked(node.first, targetPaneId, orientation, newPaneId, newSessionId);
  if (nextFirst !== node.first) {
    return { ...node, first: nextFirst };
  }
  const nextSecond = splitLeafUnchecked(node.second, targetPaneId, orientation, newPaneId, newSessionId);
  if (nextSecond !== node.second) {
    return { ...node, second: nextSecond };
  }
  return node;
}

export function splitPaneLeaf(
  node: PaneNode,
  targetPaneId: string,
  orientation: 'horizontal' | 'vertical',
  newPaneId: string,
  newSessionId: string | null,
): PaneNode {
  if (countPaneLeaves(node) >= MAX_SPLIT_PANES) return node;
  return splitLeafUnchecked(node, targetPaneId, orientation, newPaneId, newSessionId);
}

export function closePaneLeaf(node: PaneNode, targetPaneId: string): PaneNode {
  if (node.kind === 'leaf') return node;
  if (node.first.kind === 'leaf' && node.first.paneId === targetPaneId) {
    return node.second;
  }
  if (node.second.kind === 'leaf' && node.second.paneId === targetPaneId) {
    return node.first;
  }
  const nextFirst = closePaneLeaf(node.first, targetPaneId);
  const nextSecond = closePaneLeaf(node.second, targetPaneId);
  if (nextFirst === node.first && nextSecond === node.second) return node;
  return { ...node, first: nextFirst, second: nextSecond };
}

export function assignPaneSession(node: PaneNode, targetPaneId: string, sessionId: string | null): PaneNode {
  if (node.kind === 'leaf') {
    return node.paneId !== targetPaneId || node.sessionId === sessionId ? node : { ...node, sessionId };
  }
  const first = assignPaneSession(node.first, targetPaneId, sessionId);
  const second = assignPaneSession(node.second, targetPaneId, sessionId);
  return first === node.first && second === node.second ? node : { ...node, first, second };
}

export function replaceSessionInTree(node: PaneNode, oldSid: string, newSid: string | null): PaneNode {
  if (node.kind === 'leaf') return node.sessionId === oldSid ? { ...node, sessionId: newSid } : node;
  const first = replaceSessionInTree(node.first, oldSid, newSid);
  const second = replaceSessionInTree(node.second, oldSid, newSid);
  return first === node.first && second === node.second ? node : { ...node, first, second };
}

export interface SplitLeafRenderContext {
  isActive: boolean;
  leafCount: number;
  onClosePane?: () => void;
}

export interface ConsoleSplitGridProps {
  root: PaneNode;
  activePaneId: string;
  onSelectPane: (paneId: string) => void;
  onClosePane?: (paneId: string) => void;
  renderLeaf?: (
    leaf: Extract<PaneNode, { kind: 'leaf' }>,
    context: SplitLeafRenderContext,
  ) => React.ReactNode;
}

export function ConsoleSplitGrid({
  root,
  activePaneId,
  onSelectPane,
  onClosePane,
  renderLeaf,
}: ConsoleSplitGridProps) {
  const leafCount = countPaneLeaves(root);

  const renderNode = (node: PaneNode): React.ReactNode => {
    if (node.kind === 'leaf') {
      const isActive = activePaneId === node.paneId;
      const closeHandler = leafCount > 1 && onClosePane ? () => onClosePane(node.paneId) : undefined;
      return (
        <div
          key={node.paneId}
          data-testid={`console-split-leaf-${node.paneId}`}
          data-active-pane={isActive ? 'true' : 'false'}
          onMouseDown={() => onSelectPane(node.paneId)}
          onClick={() => onSelectPane(node.paneId)}
          className={`flex h-full flex-1 flex-col overflow-hidden rounded-lg border bg-bg-base ${
            leafCount > 1 ? 'min-h-16 min-[1100px]:min-h-[180px] min-w-40' : 'min-h-0 min-w-0'
          } ${
            leafCount > 1 && isActive
              ? 'border-border-strong ring-1 ring-ring'
              : 'border-border-subtle'
          }`}
        >
          {renderLeaf ? (
            renderLeaf(node, { isActive, leafCount, onClosePane: closeHandler })
          ) : (
            <TerminalSessionPane
              paneId={node.paneId}
              sessionId={node.sessionId}
              autoOpenSession={false}
              activePane={isActive}
              onClosePane={closeHandler}
            />
          )}
        </div>
      );
    }

    return (
      <ResizablePanelGroup
        key={node.id}
        id={node.id}
        orientation={node.orientation}
        className="h-full min-h-0 w-full flex-1"
      >
        <ResizablePanel
          id={`${node.id}-first`}
          defaultSize="50%"
          minSize="20%"
          className="flex h-full min-h-0 min-w-0 flex-col"
        >
          {renderNode(node.first)}
        </ResizablePanel>
        <ResizableHandle withHandle id={`${node.id}-sep`} aria-label="Resize terminal split" />
        <ResizablePanel
          id={`${node.id}-second`}
          defaultSize="50%"
          minSize="20%"
          className="flex h-full min-h-0 min-w-0 flex-col"
        >
          {renderNode(node.second)}
        </ResizablePanel>
      </ResizablePanelGroup>
    );
  };

  return <div className="flex h-full min-h-0 min-w-0 flex-1 flex-col">{renderNode(root)}</div>;
}
