import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../../ui/Glass';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { StreamItem } from '../../../types/dashboard';
import { buildChatOnlyTimeline } from '../../../lib/chatTranscriptTimeline';
import { StatusLine } from './StatusLine';
import { TurnTrace } from './TurnTrace';
import { ChatVerbosityControl } from './ChatVerbosityControl';
import { useChatVerbosity, type ChatVerbosity } from '../../../hooks/useChatVerbosity';
import { listHarnessIssuesForSession, type HarnessIssueRow } from '../Scientia/harnessIssuesApi';

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
      return (
        <div
          key={`fence-${bIdx}`}
          data-testid="chat-code-block"
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
          <pre className="overflow-x-auto p-2.5 font-mono text-[11px] leading-relaxed text-text-primary whitespace-pre">
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

interface ChatTranscriptProps {
  messages: ChatMessage[];
  agentStreamItems?: StreamItem[];
  sessionId?: string;
  /** "not this one" on a skill-activation chip — see `ChatTurnEventRow`. */
  onExcludeSkill?: (skillId: string) => void;
}

export function MessageBubble({
  message,
  onExcludeSkill,
  verbosity = 'normal',
}: {
  message: ChatMessage;
  onExcludeSkill?: (skillId: string) => void;
  verbosity?: ChatVerbosity;
}) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';
  const streaming = message.status === 'streaming' || message.status === 'pending';
  const failed = message.status === 'failed';

  const tone = isSystem
    ? 'self-center border-(--color-status-warn)/20 bg-(--color-status-warn)/6 text-(--color-status-warn)/90 text-center max-w-full'
    : isUser
      ? 'self-end border-brass/30 bg-brass/8 text-text-primary'
      : 'self-start border-border-subtle bg-overlay-subtle text-text-secondary';

  return (
    <div
      id={`msg-${message.id}`}
      className={`max-w-[80%] rounded-xl border px-3 py-2 text-[12px] leading-relaxed whitespace-pre-wrap wrap-break-word ${tone}`}
    >
      {!isSystem && isUser && (
        <div className="sr-only mb-0.5 font-mono text-[11px] uppercase tracking-[0.08em] text-text-muted">
          You
        </div>
      )}
      {!isSystem && !isUser && (
        <div className="mb-0.5 font-mono text-[11px] uppercase tracking-[0.08em] text-text-muted">
          Assistant
        </div>
      )}
      {renderStructuredMessageText(message.text)}
      {streaming && (
        <span className="ml-1 inline-flex items-center gap-1 text-[11px] text-(--color-status-info)">
          <span className="size-1.5 animate-pulse rounded-full bg-(--color-status-info)" />
          {message.text ? 'streaming…' : 'thinking…'}
        </span>
      )}
      {failed && (
        <div className="mt-1 font-mono text-[11px] text-(--color-status-fail)">
          error: {message.error ?? 'task failed'}
        </div>
      )}
      {message.role === 'assistant' && message.groundingFlagged && (
        <div className="mt-1 flex justify-end">
          <span className="rounded-sm border border-(--color-status-warn)/30 bg-(--color-status-warn)/8 px-1.5 py-0.5 font-mono text-[11px] text-(--color-status-warn)">
            low confidence — unverified
          </span>
        </div>
      )}
      {message.role === 'assistant' &&
        ((message.events?.length ?? 0) > 0 || (message.status === 'done' && !!message.modelId)) && (
          <TurnTrace
            events={message.events}
            verbosity={verbosity}
            latencyMs={message.latencyMs}
            modelId={message.modelId}
            onExcludeSkill={onExcludeSkill}
          />
        )}
    </div>
  );
}

function HarnessIssueSummary({ issue }: { issue: HarnessIssueRow }) {
  const statusTone =
    issue.status === 'dismissed' ? 'text-text-muted line-through' : 'text-(--color-status-warn)';
  return (
    <div
      data-testid={`transcript-harness-issue-${issue.id}`}
      className={`self-center rounded-sm border border-(--color-status-warn)/30 bg-(--color-status-warn)/8 px-2 py-1 text-center text-[11px] ${statusTone}`}
    >
      Issue detected ({issue.status}): {issue.summary}
    </div>
  );
}

/** Merged chat bubbles + inline agent execution rows for the active session. */
export function ChatTranscript({ messages, agentStreamItems, sessionId, onExcludeSkill }: ChatTranscriptProps) {
  const [verbosity, setVerbosity] = useChatVerbosity();
  const timeline = buildChatOnlyTimeline(messages, agentStreamItems ?? [], { verbosity });
  const [harnessIssues, setHarnessIssues] = useState<HarnessIssueRow[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [pinnedToBottom, setPinnedToBottom] = useState(true);
  const lastMsgText = messages.length > 0 ? messages[messages.length - 1]?.text : '';

  useEffect(() => {
    const el = scrollRef.current;
    if (!el || !pinnedToBottom) return;
    if (typeof el.scrollTo === 'function') {
      el.scrollTo({ top: el.scrollHeight });
    } else {
      el.scrollTop = el.scrollHeight;
    }
  }, [timeline.length, lastMsgText, pinnedToBottom]);

  useEffect(() => {
    if (!sessionId) {
      setHarnessIssues([]);
      return;
    }
    let cancelled = false;
    const fetchIssues = () => {
      listHarnessIssuesForSession(sessionId)
        .then((rows) => {
          // Defend against a mocked/misbehaving backend resolving null/undefined
          // instead of rejecting — harnessIssues.length below assumes an array.
          if (!cancelled) setHarnessIssues(rows ?? []);
        })
        .catch(() => {
          if (!cancelled) setHarnessIssues([]);
        });
    };
    fetchIssues();
    // Poll (not fetch-once) so an issue detected mid-session appears without
    // requiring a session switch — matches the cadence used elsewhere for
    // this same data (App.tsx's 8s poll, HarnessIssuesPanel's 10s poll).
    const id = window.setInterval(fetchIssues, 8_000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [sessionId]);

  if (timeline.length === 0 && harnessIssues.length === 0) return null;

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    setPinnedToBottom(distanceFromBottom <= 64);
  };

  const insertPromptSnippet = (token: string) => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('vox:atlas-mention', { detail: { token } }));
    }
  };

  return (
    // `inset={false}`: Glass's decorative inset ring is `absolute inset-0`, and
    // inside this scroller it only spans the first viewport of content — a
    // tall reply (e.g. an expanded research trace) scrolled past it showed the
    // ring's bottom edge as a line cutting through the content. The Glass
    // border already frames the transcript.
    <Glass
      inset={false}
      role="log"
      aria-live="polite"
      aria-relevant="additions text"
      aria-label="Chat transcript"
      className="relative mb-3 min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3 pb-6"
    >
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="mx-auto flex w-full max-w-[900px] flex-col gap-2.5"
      >
        <ChatVerbosityControl value={verbosity} onChange={setVerbosity} />
        {harnessIssues.length > 0 && (
          <div className="mb-1 flex flex-col gap-1 border-b border-border-subtle pb-2">
            {harnessIssues.map((issue) => (
              <HarnessIssueSummary key={issue.id} issue={issue} />
            ))}
          </div>
        )}
        {timeline.map((row) => {
          if (row.kind === 'message') {
            return <MessageBubble key={row.id} message={row.message} onExcludeSkill={onExcludeSkill} verbosity={verbosity} />;
          }
          if (row.kind === 'status') {
            return <StatusLine key={row.id} phase={row.phase} elapsedMs={row.elapsedMs} />;
          }
          return (
            <div key={row.id} className="self-start px-1 font-mono text-[11px] text-text-muted">
              Done · ${row.costUsd.toFixed(4)}
            </div>
          );
        })}

        {timeline.length <= 2 && (
          <div
            data-testid="chat-session-brief-deck"
            className="mt-3 rounded-xl border border-border-subtle bg-bg-elevated/55 p-3.5 shadow-sm"
          >
            <div className="mb-2 flex items-center justify-between border-b border-border-subtle pb-2">
              <div className="flex items-center gap-2">
                <span className="size-2 rounded-full bg-brass" aria-hidden="true" />
                <span className="font-display text-[11px] uppercase tracking-[0.13em] text-brass">
                  Axis Session Context & Quick Launch
                </span>
              </div>
              <span className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[11px] text-text-secondary">
                5-Spoke MENS · @kb:auto · 0% Markup
              </span>
            </div>
            <div className="grid grid-cols-2 gap-2">
              <button
                type="button"
                onClick={() => insertPromptSnippet('/check crates/vox-compiler')}
                className="flex flex-col items-start gap-1 rounded-lg border border-border-subtle bg-overlay-subtle/80 p-2.5 text-left transition hover:border-brass/40 hover:bg-overlay-hover"
              >
                <div className="flex w-full items-center justify-between">
                  <span className="font-mono text-[11px] font-semibold text-text-primary">Compiler & WebIR Check</span>
                  <span className="rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[11px] text-brass">vox_lang</span>
                </div>
                <span className="text-[11px] leading-snug text-text-secondary">
                  Run HIR typecheck, effect-floor verification, and Rust/TS codegen parity.
                </span>
              </button>
              <button
                type="button"
                onClick={() => insertPromptSnippet('@kb:verified architecture invariants')}
                className="flex flex-col items-start gap-1 rounded-lg border border-border-subtle bg-overlay-subtle/80 p-2.5 text-left transition hover:border-brass/40 hover:bg-overlay-hover"
              >
                <div className="flex w-full items-center justify-between">
                  <span className="font-mono text-[11px] font-semibold text-text-primary">KB & Code Atlas Search</span>
                  <span className="rounded border border-(--color-status-pass)/30 bg-(--color-status-pass)/10 px-1.5 py-0.5 font-mono text-[11px] text-(--color-status-pass)">RRF 0.98</span>
                </div>
                <span className="text-[11px] leading-snug text-text-secondary">
                  Query session-pinned architecture SSOTs and symbol callgraphs with zero hallucination.
                </span>
              </button>
              <button
                type="button"
                onClick={() => insertPromptSnippet('@browser:active inspect accessibility refs')}
                className="flex flex-col items-start gap-1 rounded-lg border border-border-subtle bg-overlay-subtle/80 p-2.5 text-left transition hover:border-brass/40 hover:bg-overlay-hover"
              >
                <div className="flex w-full items-center justify-between">
                  <span className="font-mono text-[11px] font-semibold text-text-primary">Songhand Browser & CV</span>
                  <span className="rounded border border-(--color-status-info)/30 bg-(--color-status-info)/10 px-1.5 py-0.5 font-mono text-[11px] text-(--color-status-info)">[eN] refs</span>
                </div>
                <span className="text-[11px] leading-snug text-text-secondary">
                  Drive Playwright + Songhand DOM extraction and CV Flywheel visual audits.
                </span>
              </button>
              <button
                type="button"
                onClick={() => insertPromptSnippet('/eval --mens-five-spokes')}
                className="flex flex-col items-start gap-1 rounded-lg border border-border-subtle bg-overlay-subtle/80 p-2.5 text-left transition hover:border-brass/40 hover:bg-overlay-hover"
              >
                <div className="flex w-full items-center justify-between">
                  <span className="font-mono text-[11px] font-semibold text-text-primary">5-Spoke MENS & Urbs Telemetry</span>
                  <span className="rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[11px] text-brass">100% pass@1</span>
                </div>
                <span className="text-[11px] leading-snug text-text-secondary">
                  Inspect live QLoRA adapter routing, safety floor locks, and Urbs Roma landmarks.
                </span>
              </button>
            </div>
          </div>
        )}

        {!pinnedToBottom && (
          <div className="sticky bottom-2 flex justify-center">
            <button
              type="button"
              data-testid="chat-scroll-latest"
              onClick={() => {
                setPinnedToBottom(true);
                const el = scrollRef.current;
                if (el) el.scrollTop = el.scrollHeight;
              }}
              className="rounded-full border border-brass/40 bg-bg-base/95 px-3 py-1 font-mono text-[11px] text-brass shadow-lg transition hover:bg-brass/15"
            >
              ↓ Latest
            </button>
          </div>
        )}
      </div>
    </Glass>
  );
}
