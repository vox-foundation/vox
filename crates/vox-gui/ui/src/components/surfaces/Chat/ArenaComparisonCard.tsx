import React from 'react';
import type { ArenaTurnComparisonDto, ArenaVoteOutcomeDto } from '../../../transport';
import { evaluateHighlightBudget } from '../../../lib/chat/deferredHighlight';

const FILE_LINE_RE = /\b([a-zA-Z0-9_./-]+\.(?:rs|vox|tsx|ts|toml|json|md):\d+(?:-\d+)?)\b/g;

function renderInlineFormattedText(segment: string, keyPrefix: string): React.ReactNode {
  if (!segment.includes('`') && !FILE_LINE_RE.test(segment)) {
    return segment;
  }
  FILE_LINE_RE.lastIndex = 0;
  const parts = segment.split(/(`[^`\n]+`)/g);
  return parts.map((part, idx) => {
    if (part.startsWith('`') && part.endsWith('`') && part.length > 2) {
      const inner = part.slice(1, -1);
      return (
        <code
          key={`${keyPrefix}-code-${idx}`}
          data-testid="chat-inline-code"
          className="rounded border border-border-subtle bg-bg-base/80 px-1 py-0.5 font-mono text-[11px] text-brass"
        >
          {inner}
        </code>
      );
    }
    const fileSplit = part.split(FILE_LINE_RE);
    if (fileSplit.length === 1) return part;
    return fileSplit.map((chunk, fIdx) => {
      if (fIdx % 2 === 1) {
        return (
          <span
            key={`${keyPrefix}-file-${idx}-${fIdx}`}
            data-testid="chat-file-link"
            className="inline-flex items-center rounded border border-brass/30 bg-brass/10 px-1.5 py-0.2 font-mono text-[11px] text-brass"
          >
            {chunk}
          </span>
        );
      }
      return chunk;
    });
  });
}

export function renderStructuredMessageText(text: string): React.ReactNode {
  if (!text) return text;
  const hasCodeFence = text.includes('```');
  const hasInlineBacktick = text.includes('`');
  const hasHeading = /^#{1,3}\s+/m.test(text);
  FILE_LINE_RE.lastIndex = 0;
  const hasFileRef = FILE_LINE_RE.test(text);
  FILE_LINE_RE.lastIndex = 0;

  if (!hasCodeFence && !hasInlineBacktick && !hasHeading && !hasFileRef) {
    return text;
  }

  const blocks = text.split(/(```[\s\S]*?```)/g);
  return blocks.map((block, bIdx) => {
    if (block.startsWith('```') && block.endsWith('```') && block.length >= 6) {
      const inner = block.slice(3, -3);
      const firstNewline = inner.indexOf('\n');
      const lang = firstNewline >= 0 ? inner.slice(0, firstNewline).trim() || 'code' : 'code';
      const codeBody = firstNewline >= 0 ? inner.slice(firstNewline + 1) : inner;
      const highlightBudget = evaluateHighlightBudget(codeBody.length, false, true);
      return (
        <div
          key={`fence-${bIdx}`}
          data-testid="chat-code-block"
          data-highlight-deferred={String(highlightBudget.deferToIdle)}
          data-highlight-idle-delay-ms={highlightBudget.idleDelayMs}
          className="my-2 overflow-hidden rounded-lg border border-border-subtle bg-bg-base/95 font-mono text-[11px] whitespace-normal"
        >
          <div className="flex items-center justify-between border-b border-border-subtle bg-overlay-subtle px-2.5 py-1 text-[11px] text-text-muted">
            <span className="uppercase tracking-[0.08em] text-brass">{lang}</span>
            <button
              type="button"
              data-testid="chat-code-copy"
              onClick={() => {
                void navigator.clipboard?.writeText(codeBody);
              }}
              className="rounded border border-border-subtle bg-bg-base px-1.5 py-0.5 text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
            >
              Copy
            </button>
          </div>
          <pre
            tabIndex={0}
            className="overflow-x-auto p-2.5 font-mono text-[11px] leading-relaxed text-text-primary whitespace-pre-wrap break-words"
          >
            <code>{codeBody}</code>
          </pre>
        </div>
      );
    }

    if (hasHeading) {
      const lines = block.split('\n');
      return (
        <React.Fragment key={`prose-${bIdx}`}>
          {lines.map((line, lIdx) => {
            const headingMatch = /^(#{1,3})\s+(.*)$/.exec(line);
            if (headingMatch) {
              return (
                <div
                  key={`h-${bIdx}-${lIdx}`}
                  className="mt-1.5 mb-0.5 font-display text-[12px] font-semibold tracking-[0.13em] text-brass"
                >
                  {renderInlineFormattedText(headingMatch[2], `h-${bIdx}-${lIdx}`)}
                </div>
              );
            }
            return (
              <React.Fragment key={`l-${bIdx}-${lIdx}`}>
                {renderInlineFormattedText(line, `l-${bIdx}-${lIdx}`)}
                {lIdx < lines.length - 1 ? '\n' : null}
              </React.Fragment>
            );
          })}
        </React.Fragment>
      );
    }

    return (
      <React.Fragment key={`inline-${bIdx}`}>
        {renderInlineFormattedText(block, `inline-${bIdx}`)}
      </React.Fragment>
    );
  });
}

export interface ArenaComparisonCardProps {
  comparison: ArenaTurnComparisonDto;
  voteOutcome: ArenaVoteOutcomeDto | null;
  busy: boolean;
  onVote: (winner: 'a' | 'b' | 'tie') => void;
  renderContent: (text: string) => React.ReactNode;
}

/** Side-by-Side Arena turn comparison card (`ARENA-COMPARE-01..04`). */
export function ArenaComparisonCard({
  comparison,
  voteOutcome,
  busy,
  onVote,
  renderContent,
}: ArenaComparisonCardProps) {
  return (
    <div
      data-testid="chat-arena-split-card"
      className="mt-1 flex flex-col gap-2 rounded-xl border border-brass/35 bg-bg-elevated/70 p-3"
    >
      <div className="flex items-center justify-between border-b border-border-subtle pb-1.5 font-mono text-[11px]">
        <span className="text-brass">⚔ Side-by-Side Arena Turn Comparison</span>
        <span className="text-text-muted">Online Bradley-Terry + DPO Preference</span>
      </div>
      <div className="grid grid-cols-1 gap-2.5 md:grid-cols-2">
        <div className="flex flex-col gap-1 rounded-lg border border-border-subtle bg-overlay-subtle p-2.5 text-[11px]">
          <div className="flex items-center justify-between font-mono text-[11px] text-text-muted">
            <span className="font-semibold text-brass">
              Arm A · {comparison.arm_a.model_id}
            </span>
            <span>{comparison.arm_a.latency_ms}ms</span>
          </div>
          <div className="text-text-secondary whitespace-pre-wrap">
            {renderContent(comparison.arm_a.content)}
          </div>
        </div>
        <div className="flex flex-col gap-1 rounded-lg border border-border-subtle bg-overlay-subtle p-2.5 text-[11px]">
          <div className="flex items-center justify-between font-mono text-[11px] text-text-muted">
            <span className="font-semibold text-brass">
              Arm B · {comparison.arm_b.model_id}
            </span>
            <span>{comparison.arm_b.latency_ms}ms</span>
          </div>
          <div className="text-text-secondary whitespace-pre-wrap">
            {renderContent(comparison.arm_b.content)}
          </div>
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            data-testid="chat-arena-vote-a"
            disabled={busy}
            onClick={() => onVote('a')}
            className="rounded border border-brass/40 bg-brass/15 px-2.5 py-1 font-mono text-[11px] text-brass transition hover:bg-brass/25"
          >
            Prefer A ({comparison.arm_a.model_id})
          </button>
          <button
            type="button"
            data-testid="chat-arena-vote-b"
            disabled={busy}
            onClick={() => onVote('b')}
            className="rounded border border-brass/40 bg-brass/15 px-2.5 py-1 font-mono text-[11px] text-brass transition hover:bg-brass/25"
          >
            Prefer B ({comparison.arm_b.model_id})
          </button>
          <button
            type="button"
            data-testid="chat-arena-vote-tie"
            disabled={busy}
            onClick={() => onVote('tie')}
            className="rounded border border-border-subtle bg-overlay-subtle px-2.5 py-1 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
          >
            Tie / Both Good
          </button>
        </div>
        {voteOutcome && (
          <span
            data-testid="chat-arena-vote-receipt"
            className="font-mono text-[11px] text-(--color-status-pass)"
          >
            Winner: {voteOutcome.chosen_model_id} · Q(A)=
            {voteOutcome.quality_a.toFixed(2)} Q(B)=
            {voteOutcome.quality_b.toFixed(2)} · DPO saved
          </span>
        )}
      </div>
    </div>
  );
}
