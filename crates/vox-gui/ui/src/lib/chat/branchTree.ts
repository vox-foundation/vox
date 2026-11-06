/**
 * Conversation branch tree reconstruction and Lowest Common Ancestor (LCA)
 * divergence point calculation (`T3-CLIENT-BRANCH-03..04`).
 */

export interface BranchMessageNode {
  readonly id: string;
  readonly parentMessageId: string | null;
  readonly role: 'user' | 'assistant' | 'system';
  readonly content: string;
  readonly modelId?: string;
}

export interface MessageTreeIndex {
  readonly byId: ReadonlyMap<string, BranchMessageNode>;
  readonly childrenByParent: ReadonlyMap<string | null, readonly BranchMessageNode[]>;
}

export function buildMessageTree(messages: readonly BranchMessageNode[]): MessageTreeIndex {
  const byId = new Map<string, BranchMessageNode>();
  const childrenMap = new Map<string | null, BranchMessageNode[]>();

  for (const msg of messages) {
    byId.set(msg.id, msg);
    const parentKey = msg.parentMessageId ?? null;
    const list = childrenMap.get(parentKey);
    if (list) {
      list.push(msg);
    } else {
      childrenMap.set(parentKey, [msg]);
    }
  }

  return {
    byId,
    childrenByParent: childrenMap,
  };
}

export function getRootToLeafPath(
  index: MessageTreeIndex,
  leafId: string,
): readonly BranchMessageNode[] {
  const path: BranchMessageNode[] = [];
  const visited = new Set<string>();
  let cursor: string | null = leafId;
  while (cursor !== null) {
    if (visited.has(cursor)) {
      break;
    }
    visited.add(cursor);
    const node = index.byId.get(cursor);
    if (!node) {
      break;
    }
    path.push(node);
    cursor = node.parentMessageId;
  }
  return path.reverse();
}

export interface BranchDivergenceResult {
  readonly divergenceAncestorId: string | null;
  readonly sharedPrefix: readonly BranchMessageNode[];
  readonly leftExclusive: readonly BranchMessageNode[];
  readonly rightExclusive: readonly BranchMessageNode[];
}

export function computeDivergencePoint(
  index: MessageTreeIndex,
  leftLeafId: string,
  rightLeafId: string,
): BranchDivergenceResult {
  const leftPath = getRootToLeafPath(index, leftLeafId);
  const rightPath = getRootToLeafPath(index, rightLeafId);
  const minLen = Math.min(leftPath.length, rightPath.length);
  let splitIdx = 0;
  while (splitIdx < minLen && leftPath[splitIdx]?.id === rightPath[splitIdx]?.id) {
    splitIdx += 1;
  }
  const sharedPrefix = leftPath.slice(0, splitIdx);
  const divergenceAncestorId =
    sharedPrefix.length > 0 ? (sharedPrefix[sharedPrefix.length - 1]?.id ?? null) : null;
  return {
    divergenceAncestorId,
    sharedPrefix,
    leftExclusive: leftPath.slice(splitIdx),
    rightExclusive: rightPath.slice(splitIdx),
  };
}
