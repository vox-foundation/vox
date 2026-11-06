/**
 * Chunked reverse-chronological thread lazy-loading (`CHUNK_SIZE = 16`) +
 * zero-turn transient thread persistence gate (`T3-CLIENT-BRANCH-01..02`).
 */

export const CHUNK_SIZE = 16;

export interface ThreadTurnLike {
  readonly id: string;
  readonly createdAtMs?: number;
}

export interface ChunkedThreadWindow<T> {
  readonly visibleItems: readonly T[];
  readonly totalItems: number;
  readonly loadedCount: number;
  readonly hasMoreOlder: boolean;
}

export function computeChunkedThreadWindow<T>(
  allChronologicalItems: readonly T[],
  chunksLoaded: number,
  chunkSize: number = CHUNK_SIZE,
): ChunkedThreadWindow<T> {
  const safeChunks = Math.max(1, Math.floor(chunksLoaded));
  const maxVisible = safeChunks * chunkSize;
  const totalItems = allChronologicalItems.length;
  const startIndex = Math.max(0, totalItems - maxVisible);
  const visibleItems = allChronologicalItems.slice(startIndex);
  return {
    visibleItems,
    totalItems,
    loadedCount: visibleItems.length,
    hasMoreOlder: startIndex > 0,
  };
}

export interface ThreadPersistenceCandidate {
  readonly turnCount: number;
  readonly hasActiveTurn: boolean;
  readonly hasAttachedFiles: boolean;
}

/**
 * Prevents persisting empty `0`-turn draft threads to IndexedDB/SQLite until the
 * user sends the first prompt or attaches workspace files (`T3-CLIENT-BRANCH-02`).
 */
export function shouldPersistThreadMutation(candidate: ThreadPersistenceCandidate): boolean {
  return (
    candidate.turnCount > 0 ||
    candidate.hasActiveTurn ||
    candidate.hasAttachedFiles
  );
}
