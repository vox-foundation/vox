import React from 'react';
import type { TermBlockDto, TermBlockOrigin } from '../../../transport';
import { Button } from '../../ui/Button';

export interface BlockStreamViewProps {
  blocks: TermBlockDto[];
  pinnedBlockIds?: number[];
  focusedBlockId?: number | null;
  onCopyCommand: (block: TermBlockDto) => void;
  onCopyOutput: (block: TermBlockDto) => void;
  onRerun: (block: TermBlockDto) => void;
  onPin: (block: TermBlockDto) => void;
  onAskAgent: (block: TermBlockDto) => void;
  onFocusInChat?: (block: TermBlockDto, lineRange?: string) => void;
  onSelectBlock?: (blockId: number) => void;
}

function formatOriginLabel(origin: TermBlockOrigin | undefined): string {
  if (!origin) return 'user';
  if (typeof origin === 'string') return origin;
  if (origin.kind === 'agent') return `agent:${origin.agent_id}`;
  return 'user';
}

function isAgentOrigin(origin: TermBlockOrigin | undefined): boolean {
  if (!origin) return false;
  if (typeof origin === 'string') return origin === 'agent';
  return origin.kind === 'agent';
}

function formatStatusLabel(block: TermBlockDto): string {
  if (block.status === 'Running') return 'running';
  if (block.status === 'Cancelled') return 'cancelled';
  if (block.exit_code !== null && block.exit_code !== undefined) {
    return `exit ${block.exit_code}`;
  }
  return block.status === 'Failed' ? 'failed' : 'ok';
}

export function isStructuredBlock(input: string, outputText: string): boolean {
  const trimmedInput = input.trim();
  if (trimmedInput.startsWith('!nu') || trimmedInput.startsWith('/nu')) {
    return true;
  }
  const trimmedOutput = outputText.trim();
  if (
    (trimmedOutput.startsWith('{') && trimmedOutput.endsWith('}')) ||
    (trimmedOutput.startsWith('[') && trimmedOutput.endsWith(']'))
  ) {
    try {
      JSON.parse(trimmedOutput);
      return true;
    } catch {
      return false;
    }
  }
  return false;
}

export function BlockStreamView({
  blocks,
  pinnedBlockIds = [],
  focusedBlockId = null,
  onCopyCommand,
  onCopyOutput,
  onRerun,
  onPin,
  onAskAgent,
  onFocusInChat,
  onSelectBlock,
}: BlockStreamViewProps) {
  const containerRef = React.useRef<HTMLElement | null>(null);

  React.useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [blocks.length]);

  return (
    <section
      ref={containerRef}
      data-testid="block-stream-view"
      aria-label="command blocks"
      className="flex max-h-[340px] shrink-0 flex-col gap-2 overflow-y-auto border-b border-white/10 px-3 py-2 text-xs scroll-smooth"
    >
      {blocks.length === 0 ? (
        <div className="py-2 text-center text-[11px] text-zinc-500 font-mono">
          No command blocks recorded yet. Enter a command below.
        </div>
      ) : (
        blocks.map((block) => {
          const originLabel = formatOriginLabel(block.origin);
          const agentOrigin = isAgentOrigin(block.origin);
          const statusLabel = formatStatusLabel(block);
          const durationMs =
            block.ended_ms !== null && block.ended_ms >= block.started_ms
              ? `${block.ended_ms - block.started_ms}ms`
              : null;
          const outputText = block.output.map((c) => c.text).join('');
          const isPinned = pinnedBlockIds.includes(block.id);
          const isFocused = focusedBlockId === block.id;
          const isStructured = isStructuredBlock(block.input, outputText);
          let displayOutput = outputText;
          if (isStructured && outputText.trim()) {
            try {
              const parsed = JSON.parse(outputText.trim());
              displayOutput = JSON.stringify(parsed, null, 2);
            } catch {
              // preserve original output
            }
          }

          return (
            <div
              key={block.id}
              data-testid="term-block-card"
              data-block-id={String(block.id)}
              data-pinned={isPinned ? 'true' : 'false'}
              data-focused={isFocused ? 'true' : 'false'}
              onClick={() => onSelectBlock?.(block.id)}
              className={`flex flex-col gap-1.5 rounded-lg border p-2.5 shadow-sm transition-all ${
                isPinned
                  ? 'border-amber-400/80 bg-amber-950/25 shadow-[0_0_12px_rgba(251,191,36,0.12)]'
                  : isFocused
                    ? 'border-cyan/60 bg-cyan-950/20 ring-1 ring-cyan/50'
                    : 'border-white/10 bg-zinc-950/70 hover:border-white/20'
              }`}
            >
              <div className="flex flex-wrap items-center justify-between gap-2 border-b border-white/5 pb-1">
                <div className="flex items-center gap-1.5 font-mono">
                  <span
                    className={`rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider ${
                      agentOrigin
                        ? 'border border-amber-500/30 bg-amber-500/20 text-amber-300'
                        : 'border border-white/10 bg-white/10 text-zinc-300'
                    }`}
                  >
                    {originLabel}
                  </span>
                  {isStructured && (
                    <span
                      data-testid="term-block-structured-badge"
                      className="rounded border border-cyan/40 bg-cyan/15 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-cyan"
                    >
                      structured json
                    </span>
                  )}
                  <span className="font-semibold text-zinc-100">{`$ ${block.input}`}</span>
                </div>
                <div className="flex items-center gap-2 text-[11px] font-mono">
                  <span
                    className={`font-semibold ${
                      block.status === 'Failed' || (block.exit_code ?? 0) !== 0
                        ? 'text-red-400'
                        : 'text-emerald-400'
                    }`}
                  >
                    {statusLabel}
                  </span>
                  {durationMs && <span className="text-zinc-500">{durationMs}</span>}
                </div>
              </div>
              {displayOutput && (
                <pre className="max-h-28 overflow-y-auto whitespace-pre-wrap rounded bg-black/50 p-2 font-mono text-[11px] text-zinc-300 border border-white/5">
                  {displayOutput}
                </pre>
              )}
              <div className="flex flex-wrap items-center gap-1.5 pt-0.5">
                <Button
                  variant="ghost"
                  size="xs"
                  onClick={() => onCopyCommand(block)}
                  className="h-5 px-2 py-0 text-[10px] text-zinc-400 hover:text-zinc-100 border border-white/10 hover:border-white/25 rounded"
                >
                  Copy cmd
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  onClick={() => onCopyOutput(block)}
                  className="h-5 px-2 py-0 text-[10px] text-zinc-400 hover:text-zinc-100 border border-white/10 hover:border-white/25 rounded"
                >
                  Copy output
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  onClick={() => onRerun(block)}
                  className="h-5 px-2 py-0 text-[10px] text-zinc-400 hover:text-zinc-100 border border-white/10 hover:border-white/25 rounded"
                >
                  Re-run
                </Button>
                <Button
                  variant={isPinned ? 'primary' : 'ghost'}
                  size="xs"
                  onClick={() => onPin(block)}
                  className={`h-5 px-2 py-0 text-[10px] rounded border ${
                    isPinned
                      ? 'border-amber-400/80 bg-amber-500/25 text-amber-300 font-semibold'
                      : 'text-zinc-400 hover:text-zinc-100 border-white/10 hover:border-white/25'
                  }`}
                >
                  {isPinned ? 'Pinned' : 'Pin'}
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  data-testid="term-block-focus-chat"
                  onClick={() => {
                    const sel =
                      typeof window !== 'undefined'
                        ? window.getSelection()?.toString()?.trim()
                        : undefined;
                    let lineRange: string | undefined = undefined;
                    if (sel && displayOutput.includes(sel)) {
                      const allLines = displayOutput.split('\n');
                      const selLines = sel.split('\n');
                      const startIdx = allLines.findIndex((l) => l.includes(selLines[0]));
                      if (startIdx >= 0) {
                        const endIdx = startIdx + selLines.length - 1;
                        lineRange = `L${startIdx + 1}-${endIdx + 1}`;
                      }
                    }
                    onFocusInChat?.(block, lineRange);
                  }}
                  className="h-5 px-2 py-0 text-[10px] text-cyan hover:text-cyan-200 border border-cyan/30 hover:border-cyan/50 rounded font-medium"
                >
                  Focus in Chat
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  onClick={() => onAskAgent(block)}
                  className="h-5 px-2 py-0 text-[10px] text-cyan hover:text-cyan-light border border-cyan/30 hover:border-cyan/50 bg-cyan/10 rounded font-medium"
                >
                  Ask Agent
                </Button>
              </div>
            </div>
          );
        })
      )}
    </section>
  );
}
