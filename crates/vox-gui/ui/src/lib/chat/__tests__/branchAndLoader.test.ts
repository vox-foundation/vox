import { describe, expect, it } from 'vitest';
import {
  CHUNK_SIZE,
  computeChunkedThreadWindow,
  shouldPersistThreadMutation,
} from '../chunkedThreadLoader';
import { evaluateHighlightBudget } from '../deferredHighlight';
import { buildMessageTree, computeDivergencePoint } from '../branchTree';

describe('chunkedThreadLoader & branchTree (T3-CLIENT-BRANCH-01..04)', () => {
  it('slices reverse-chronological 16-message chunks and blocks 0-turn persistence', () => {
    const items = Array.from({ length: 40 }, (_, idx) => ({ id: `m-${idx + 1}` }));
    const win1 = computeChunkedThreadWindow(items, 1, CHUNK_SIZE);
    expect(win1.loadedCount).toBe(16);
    expect(win1.hasMoreOlder).toBe(true);
    expect(win1.visibleItems[0]?.id).toBe('m-25');

    expect(
      shouldPersistThreadMutation({
        turnCount: 0,
        hasActiveTurn: false,
        hasAttachedFiles: false,
      }),
    ).toBe(false);
    expect(
      shouldPersistThreadMutation({
        turnCount: 1,
        hasActiveTurn: false,
        hasAttachedFiles: false,
      }),
    ).toBe(true);
  });

  it('defers syntax highlighting over 20k chars or while streaming', () => {
    expect(evaluateHighlightBudget(500, false, true).deferToIdle).toBe(false);
    expect(evaluateHighlightBudget(25_000, false, true).deferToIdle).toBe(true);
    expect(evaluateHighlightBudget(100, true, true).deferToIdle).toBe(true);
  });

  it('computes exact LCA branch divergence point', () => {
    const tree = buildMessageTree([
      { id: 'm1', parentMessageId: null, role: 'user', content: 'root' },
      { id: 'm2', parentMessageId: 'm1', role: 'assistant', content: 'shared reply' },
      { id: 'm3a', parentMessageId: 'm2', role: 'user', content: 'left branch' },
      { id: 'm3b', parentMessageId: 'm2', role: 'user', content: 'right branch' },
    ]);
    const div = computeDivergencePoint(tree, 'm3a', 'm3b');
    expect(div.divergenceAncestorId).toBe('m2');
    expect(div.sharedPrefix.map((x) => x.id)).toEqual(['m1', 'm2']);
    expect(div.leftExclusive.map((x) => x.id)).toEqual(['m3a']);
    expect(div.rightExclusive.map((x) => x.id)).toEqual(['m3b']);
  });
});
