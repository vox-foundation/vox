import React from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { OpenLocator, TermBlockDto } from '../../../transport';
import { EmptyState } from '../../ui/EmptyState';
import { Icon } from '../../ui/Icons';
import {
  BlockCard,
  TERMINAL_MONO_STYLE,
  isStructuredBlock,
  type ExtendedTermBlockDto,
} from './BlockCard';

export { isStructuredBlock };
export type { ExtendedTermBlockDto };

const VIRTUALIZE_THRESHOLD = 24;

export interface BlockStreamViewProps {
  blocks: ExtendedTermBlockDto[];
  pinnedBlockIds?: number[];
  focusedBlockId?: number | null;
  compact?: boolean;
  clearedEmpty?: boolean;
  searchQuery?: string;
  activeSearchBlockId?: number | null;
  onCopyCommand: (block: TermBlockDto) => void;
  onCopyOutput: (block: TermBlockDto) => void;
  onRerun: (block: TermBlockDto) => void;
  onPin: (block: TermBlockDto) => void;
  onAskAgent: (block: TermBlockDto) => void;
  onFocusInChat?: (block: TermBlockDto, lineRange?: string) => void;
  onSelectBlock?: (blockId: number) => void;
  onSendStdin?: (block: TermBlockDto, line: string) => void;
  onPromoteBg?: (block: TermBlockDto) => void;
  onInterrupt?: (block: TermBlockDto) => void;
  onOpenLocator?: (locator: OpenLocator) => void;
  onExplainFixBlock?: (block: TermBlockDto, mode?: 'explain' | 'fix') => void;
  diagnosingBlockId?: number | null;
  onRunProposedCommand?: (cmd: string, sourceBlockId?: number) => void;
  onInsertInPrompt?: (cmd: string) => void;
  onOpenSecrets?: () => void;
  onCancelAgentTurn?: (block: TermBlockDto) => void;
  onFeedback?: (block: TermBlockDto, verdict: 'accepted' | 'rejected' | 'corrected', correctedCommand?: string) => Promise<boolean | void> | boolean | void;
}

export function BlockStreamView({
  blocks,
  pinnedBlockIds = [],
  focusedBlockId = null,
  compact = false,
  clearedEmpty = false,
  searchQuery,
  activeSearchBlockId = null,
  onCopyCommand,
  onCopyOutput,
  onRerun,
  onPin,
  onAskAgent,
  onFocusInChat,
  onSelectBlock,
  onSendStdin,
  onPromoteBg,
  onInterrupt,
  onOpenLocator,
  onExplainFixBlock,
  diagnosingBlockId = null,
  onRunProposedCommand,
  onInsertInPrompt,
  onOpenSecrets,
  onCancelAgentTurn,
  onFeedback,
}: BlockStreamViewProps) {
  const containerRef = React.useRef<HTMLElement | null>(null);
  const prevCountRef = React.useRef<number>(blocks.length);

  const virtualizer = useVirtualizer({
    count: blocks.length,
    getScrollElement: () => containerRef.current,
    estimateSize: () => 136,
    overscan: 6,
    observeElementRect: (instance, cb) => {
      const el = instance.scrollElement;
      if (!el) return;
      const emit = () => {
        const rect = el.getBoundingClientRect();
        const width = el.clientWidth || el.offsetWidth || rect.width || 0;
        const height = el.clientHeight || el.offsetHeight || rect.height || 0;
        cb({ width: Math.round(width), height: Math.round(height) });
      };
      emit();
      if (typeof ResizeObserver === 'undefined') return;
      const ro = new ResizeObserver(() => emit());
      ro.observe(el);
      return () => ro.disconnect();
    },
  });

  const virtualItems = virtualizer.getVirtualItems();
  const isVirtualized =
    blocks.length > VIRTUALIZE_THRESHOLD &&
    (containerRef.current?.clientHeight ?? 0) > 0 &&
    virtualItems.length > 0;

  React.useEffect(() => {
    if (blocks.length > prevCountRef.current && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
    prevCountRef.current = blocks.length;
  }, [blocks.length]);

  React.useEffect(() => {
    const targetId = activeSearchBlockId ?? focusedBlockId;
    if (targetId == null) return;
    const idx = blocks.findIndex((b) => b.id === targetId);
    if (idx < 0) return;
    if (isVirtualized) {
      virtualizer.scrollToIndex(idx, { align: 'auto' });
    } else {
      const el = containerRef.current?.querySelector<HTMLElement>(`[data-block-id="${targetId}"]`);
      el?.scrollIntoView?.({ block: 'nearest' });
    }
  }, [activeSearchBlockId, focusedBlockId, blocks, isVirtualized, virtualizer]);

  const handleRetryAgentTurn = React.useCallback(
    (agentBlock: TermBlockDto) => {
      const match = agentBlock.input.match(/^\?\s*(explain|fix)\s+block\s+#(\d+)/i);
      if (match && onExplainFixBlock) {
        const target = blocks.find((b) => b.id === Number(match[2]));
        if (target) {
          if (match[1]!.toLowerCase() === 'explain') onExplainFixBlock(target, 'explain');
          else onExplainFixBlock(target);
          return;
        }
      }
      onRerun(agentBlock);
    },
    [blocks, onExplainFixBlock, onRerun],
  );

  const renderCard = (block: ExtendedTermBlockDto) => {
    const isPinned = pinnedBlockIds.includes(block.id) || Boolean(block.pinned);
    const isFocused = focusedBlockId === block.id || activeSearchBlockId === block.id;
    return (
      <BlockCard
        key={block.id}
        block={block}
        isPinned={isPinned}
        isFocused={isFocused}
        compact={compact}
        searchQuery={searchQuery}
        onCopyCommand={onCopyCommand}
        onCopyOutput={onCopyOutput}
        onRerun={onRerun}
        onPin={onPin}
        onAskAgent={onAskAgent}
        onFocusInChat={onFocusInChat}
        onSelectBlock={onSelectBlock}
        onSendStdin={onSendStdin}
        onPromoteBg={onPromoteBg}
        onInterrupt={onInterrupt}
        onOpenLocator={onOpenLocator}
        onExplainFixBlock={onExplainFixBlock}
        diagnosingBlockId={diagnosingBlockId}
        onRunProposedCommand={onRunProposedCommand}
        onInsertInPrompt={onInsertInPrompt}
        onOpenSecrets={onOpenSecrets}
        onRetryAgentTurn={handleRetryAgentTurn}
        onCancelAgentTurn={onCancelAgentTurn}
        onFeedback={onFeedback}
      />
    );
  };

  return (
    <section
      ref={containerRef}
      tabIndex={0}
      data-testid="block-stream-view"
      aria-label="command blocks"
      className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-3 py-2 text-xs"
    >
      {blocks.length === 0 ? (
        <EmptyState
          icon={<Icon.terminal className="size-6 text-text-muted" aria-hidden="true" />}
          title={clearedEmpty ? 'Session viewport cleared' : 'Terminal session ready'}
          description={
            clearedEmpty
              ? 'Cleared unpinned blocks from view while preserving scrubbed session history in the terminal journal. Enter a command, ?prompt, or /slash command below.'
              : 'No command blocks recorded yet. Enter a shell command below or press Cmd+J to focus the prompt.'
          }
        />
      ) : isVirtualized ? (
        <div
          style={{
            height: `${virtualizer.getTotalSize()}px`,
            position: 'relative',
            width: '100%',
          }}
        >
          {virtualItems.map((vItem) => {
            const block = blocks[vItem.index];
            if (!block) return null;
            return (
              <div
                key={block.id}
                data-index={vItem.index}
                ref={(el) => {
                  if (el && el.offsetHeight > 0) virtualizer.measureElement(el);
                }}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  transform: `translateY(${vItem.start}px)`,
                }}
                className="pb-2"
              >
                {renderCard(block)}
              </div>
            );
          })}
        </div>
      ) : (
        blocks.map(renderCard)
      )}
    </section>
  );
}
