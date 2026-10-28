import { useVirtualizer, type Range, type VirtualItem } from '@tanstack/react-virtual';
import type React from 'react';

export interface UseVirtualListOptions {
  containerRef: React.RefObject<HTMLElement>;
  count: number;
  estimateSize: (index: number) => number;
  overscan?: number;
  rangeExtractor?: (range: Range) => number[];
}

export interface UseVirtualListResult {
  virtualizer: ReturnType<typeof useVirtualizer>;
  totalSize: number;
  virtualItems: VirtualItem[];
}

export function useVirtualList({
  containerRef,
  count,
  estimateSize,
  overscan = 3,
  rangeExtractor,
}: UseVirtualListOptions): UseVirtualListResult {
  const virtualizer = useVirtualizer({
    count,
    getScrollElement: () => containerRef.current,
    estimateSize,
    overscan,
    ...(rangeExtractor ? { rangeExtractor } : {}),
  });

  return {
    virtualizer,
    totalSize: virtualizer.getTotalSize(),
    virtualItems: virtualizer.getVirtualItems(),
  };
}
