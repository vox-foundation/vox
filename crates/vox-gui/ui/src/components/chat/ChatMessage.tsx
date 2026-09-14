import React from 'react';
import type { ResearchSummary } from '../../lib/types';
import { ResearchSummaryCard } from './ResearchSummaryCard';
import { cn } from '../../lib/cn';

export interface ChatMessageItemData {
  id?: string;
  role?: 'user' | 'assistant' | 'system' | string;
  text?: string;
  content?: string;
  status?: string;
  error?: string;
  sessionId?: string;
  research?: ResearchSummary;
  researchSessionId?: string;
  research_session_id?: string;
  modelId?: string;
  [key: string]: unknown;
}

export interface ChatMessageProps {
  message: ChatMessageItemData;
  onOpenResearch?: (sessionId: string) => void;
  className?: string;
}

export function ChatMessage({ message, onOpenResearch, className }: ChatMessageProps) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';
  const isStreaming = message.status === 'streaming' || message.status === 'pending';
  const isFailed = message.status === 'failed';

  const textContent = message.text ?? message.content ?? '';

  const researchSessionId =
    message.research?.sessionId ?? message.researchSessionId ?? message.research_session_id;
  const hasResearch = Boolean(message.research || researchSessionId);

  const tone = isSystem
    ? 'self-center border-amber-400/20 bg-amber-400/[0.06] text-amber-100/90 text-center max-w-full'
    : isUser
      ? 'self-end border-brass/30 bg-brass/[0.08] text-text-primary'
      : 'self-start border-border-subtle bg-overlay-subtle text-text-secondary';

  return (
    <div
      data-testid={`chat-message-${message.id ?? 'item'}`}
      className={cn(
        'flex flex-col gap-1.5 max-w-[85%] rounded-xl border px-3 py-2 text-[12px] leading-relaxed break-words',
        tone,
        className
      )}
    >
      {!isSystem && (
        <div className="font-mono text-[9px] uppercase tracking-wide text-text-muted">
          {isUser ? 'You' : 'Assistant'}
        </div>
      )}

      {textContent && (
        <div className="whitespace-pre-wrap">{textContent}</div>
      )}

      {isStreaming && (
        <span className="inline-flex items-center gap-1 text-[10px] text-cyan-300">
          <span className="size-1.5 animate-pulse rounded-full bg-cyan-300" />
          {textContent ? 'streaming…' : 'thinking…'}
        </span>
      )}

      {isFailed && (
        <div className="font-mono text-[10px] text-rose-400">
          error: {message.error ?? 'task failed'}
        </div>
      )}

      {hasResearch && (
        <ResearchSummaryCard
          summary={message.research}
          sessionId={researchSessionId}
          onOpenResearch={onOpenResearch}
        />
      )}
    </div>
  );
}
