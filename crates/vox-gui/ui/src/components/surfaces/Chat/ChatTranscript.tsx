import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../../ui/Glass';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { StreamItem } from '../../../types/dashboard';
import { buildChatOnlyTimeline } from '../../../lib/chatTranscriptTimeline';
import { StatusLine } from './StatusLine';
import { TurnTrace } from './TurnTrace';
import { ChatVerbosityControl } from './ChatVerbosityControl';
import { ArenaComparisonCard, renderStructuredMessageText } from './ArenaComparisonCard';
import { useChatVerbosity, type ChatVerbosity } from '../../../hooks/useChatVerbosity';
import { listHarnessIssuesForSession, type HarnessIssueRow } from '../Scientia/harnessIssuesApi';
import {
  chatArenaCompare,
  chatArenaVote,
  chatConsensusRun,
  chatForkSession,
  chatRewindToTurn,
  compactChatUpTo,
  editChatMessageInPlace,
  regenerateChatTurn,
  vcsMusePoolAdopt,
  type ArenaTurnComparisonDto,
  type ArenaVoteOutcomeDto,
} from '../../../transport';
import { ConsensusGridView, type ConsensusLegViewDto } from './ConsensusGridView';
import { BranchComparisonModal } from './BranchComparisonModal';
import {
  buildMessageTree,
  computeDivergencePoint,
  type BranchDivergenceResult,
  type BranchMessageNode,
} from '../../../lib/chat/branchTree';
import {
  CHUNK_SIZE,
  computeChunkedThreadWindow,
  shouldPersistThreadMutation,
} from '../../../lib/chat/chunkedThreadLoader';

export { renderStructuredMessageText };

interface ChatTranscriptProps {
  messages: ChatMessage[];
  agentStreamItems?: StreamItem[];
  sessionId?: string;
  /** "not this one" on a skill-activation chip — see `ChatTurnEventRow`. */
  onExcludeSkill?: (skillId: string) => void;
  /** Optional activeArenaComparison injected for testing or parent control. */
  initialArenaComparison?: ArenaTurnComparisonDto | null;
  /** Optional parent session ID when this session was branched from an earlier turn. */
  parentSessionId?: string | null;
  /** Optional message ID where this session branched from `parentSessionId`. */
  forkFromMessageId?: number | null;
  /** Callback when a session is branched from a turn (`BRANCH-REWIND-01`). */
  onSessionBranched?: (newSessionId: string, fromMessageId: number) => void;
  /** Callback when a session is rewound to a turn (`BRANCH-REWIND-03`). */
  onSessionRewound?: (toMessageId: number, checkpointOpId: string) => void;
}

export function MessageBubble({
  message,
  onExcludeSkill,
  verbosity = 'normal',
  onForkTurn,
  onRewindTurn,
  onEditInPlace,
  onRegenerateTurn,
  onCompactUpTo,
}: {
  message: ChatMessage;
  onExcludeSkill?: (skillId: string) => void;
  verbosity?: ChatVerbosity;
  onForkTurn?: (message: ChatMessage) => void;
  onRewindTurn?: (message: ChatMessage) => void;
  onEditInPlace?: (message: ChatMessage, newContent: string) => void;
  onRegenerateTurn?: (message: ChatMessage) => void;
  onCompactUpTo?: (message: ChatMessage) => void;
}) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';
  const streaming = message.status === 'streaming' || message.status === 'pending';
  const failed = message.status === 'failed';
  const [editing, setEditing] = useState(false);
  const [draftText, setDraftText] = useState(message.text ?? '');
  const [editedText, setEditedText] = useState<string | null>(null);
  const displayContent = editedText ?? message.text ?? '';

  const tone = isSystem
    ? 'self-center border-(--color-status-warn)/20 bg-(--color-status-warn)/6 text-(--color-status-warn)/90 text-center max-w-full'
    : isUser
      ? 'self-end border-brass/30 bg-brass/8 text-text-primary'
      : 'self-start border-border-subtle bg-overlay-subtle text-text-secondary';

  return (
    <div
      id={`msg-${message.id}`}
      className={`max-w-[92%] sm:max-w-[85%] rounded-xl border px-3 py-2 text-[12px] leading-relaxed whitespace-pre-wrap wrap-break-word ${tone}`}
    >
      {!isSystem && isUser && (
        <div className="sr-only mb-0.5 font-mono text-[11px] uppercase tracking-[0.08em] text-text-muted">
          You
        </div>
      )}
      {!isSystem && !isUser && (
        <div className="mb-1 flex items-center justify-between gap-2 font-mono text-[11px] uppercase tracking-[0.08em] text-text-muted">
          <span className="inline-flex items-center gap-1.5">
            Assistant
            {editedText != null && (
              <span
                data-testid={`chat-turn-edited-badge-${message.id}`}
                className="rounded border border-brass/35 bg-brass/10 px-1 py-0.2 font-mono text-[11px] normal-case text-brass"
              >
                [Edited]
              </span>
            )}
          </span>
          {(message.latencyMs != null || (message.events?.length ?? 0) > 0) && (
            <span className="rounded-full border border-brass/30 bg-brass/10 px-2 py-0.5 font-mono text-[11px] normal-case tracking-normal text-brass">
              Thought for {message.latencyMs ? `${(message.latencyMs / 1000).toFixed(1)}s` : '1.8s'}
              {(message.events?.filter((e) => e.kind === 'tool_start').length ?? 0) > 0
                ? ` · ${message.events?.filter((e) => e.kind === 'tool_start').length} tool calls ▸`
                : ''}
            </span>
          )}
        </div>
      )}
      {editing ? (
        <div className="my-1 flex flex-col gap-1.5">
          <textarea
            data-testid={`chat-turn-edit-textarea-${message.id}`}
            value={draftText}
            onChange={(e) => setDraftText(e.target.value)}
            rows={3}
            className="w-full rounded border border-brass/40 bg-bg-base/90 p-1.5 font-mono text-[11px] text-text-primary focus:outline-none"
          />
          <div className="flex justify-end gap-1.5">
            <button
              type="button"
              onClick={() => setEditing(false)}
              className="rounded border border-border-subtle px-2 py-0.5 font-mono text-[11px] text-text-muted"
            >
              Cancel
            </button>
            <button
              type="button"
              data-testid={`chat-turn-edit-save-btn-${message.id}`}
              onClick={() => {
                setEditedText(draftText);
                setEditing(false);
                onEditInPlace?.(message, draftText);
              }}
              className="rounded border border-brass/50 bg-brass/20 px-2 py-0.5 font-mono text-[11px] text-brass"
            >
              Save edit
            </button>
          </div>
        </div>
      ) : displayContent.startsWith('LLM error:') ? (
        <div className="my-1 rounded-lg border border-brass/45 bg-bg-base/80 px-2.5 py-2 text-[11px] text-text-primary">
          <div className="mb-1 flex items-center justify-between gap-2 font-display text-[11px] font-bold uppercase tracking-[0.13em] text-brass">
            <span>⚡ Local Inference Endpoint Offline</span>
            {onRegenerateTurn && (
              <button
                type="button"
                onClick={() => onRegenerateTurn(message)}
                className="rounded border border-brass/55 bg-brass/15 px-2 py-0.5 font-mono text-[11px] text-brass hover:bg-brass/25"
              >
                ↻ Retry on Auto Router
              </button>
            )}
          </div>
          <div className="text-[11px] text-text-secondary">
            {renderStructuredMessageText(displayContent)}
          </div>
        </div>
      ) : (
        renderStructuredMessageText(displayContent)
      )}
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
      {!isSystem && (onForkTurn || onRewindTurn || onEditInPlace || onRegenerateTurn || onCompactUpTo) && (
        <div className="mt-1.5 flex flex-wrap items-center justify-end gap-1 border-t border-border-subtle/50 pt-1">
          {onEditInPlace && (
            <button
              type="button"
              data-testid={`chat-turn-edit-btn-${message.id}`}
              title="Edit message in-place without rewinding subsequent turns"
              onClick={() => {
                setDraftText(displayContent);
                setEditing(true);
              }}
              className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              ✎ Edit
            </button>
          )}
          {onRegenerateTurn && (
            <button
              type="button"
              data-testid={`chat-turn-regenerate-btn-${message.id}`}
              title="Truncate transcript from this message and regenerate the assistant turn"
              onClick={() => onRegenerateTurn(message)}
              className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              ↻ Regenerate
            </button>
          )}
          {onCompactUpTo && (
            <button
              type="button"
              data-testid={`chat-turn-compact-btn-${message.id}`}
              title="Compact turns up to this message while preserving thought signatures"
              onClick={() => onCompactUpTo(message)}
              className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              ✂ Compact Here
            </button>
          )}
          {onForkTurn && (
            <button
              type="button"
              data-testid={`chat-turn-fork-btn-${message.id}`}
              title="Branch a new conversation session from this turn without mutating the current thread"
              onClick={() => onForkTurn(message)}
              className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              ⑂ Branch here
            </button>
          )}
          {onRewindTurn && (
            <button
              type="button"
              data-testid={`chat-turn-rewind-btn-${message.id}`}
              title="Rewind conversation and restore Jujutsu (vox-vcs) checkpoint to this turn"
              onClick={() => onRewindTurn(message)}
              className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              ↺ Rewind
            </button>
          )}
        </div>
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
export function ChatTranscript({
  messages,
  agentStreamItems,
  sessionId,
  onExcludeSkill,
  initialArenaComparison = null,
  parentSessionId = null,
  forkFromMessageId = null,
  onSessionBranched,
  onSessionRewound,
}: ChatTranscriptProps) {
  const [verbosity, setVerbosity] = useChatVerbosity();
  const timeline = buildChatOnlyTimeline(messages, agentStreamItems ?? [], { verbosity });
  const [harnessIssues, setHarnessIssues] = useState<HarnessIssueRow[]>([]);
  const [arenaModeEnabled, setArenaModeEnabled] = useState(Boolean(initialArenaComparison));
  const [arenaComparison, setArenaComparison] = useState<ArenaTurnComparisonDto | null>(
    initialArenaComparison,
  );
  const [arenaVoteOutcome, setArenaVoteOutcome] = useState<ArenaVoteOutcomeDto | null>(null);
  const [arenaBusy, setArenaBusy] = useState(false);
  const [consensusLegs, setConsensusLegs] = useState<ConsensusLegViewDto[] | null>(null);
  const [consensusSynthesis, setConsensusSynthesis] = useState<string | null>(null);
  const [consensusBusy, setConsensusBusy] = useState(false);
  const [branchComparison, setBranchComparison] = useState<BranchDivergenceResult | null>(null);
  const [chunksLoaded, setChunksLoaded] = useState(1);
  const [branchBanner, setBranchBanner] = useState<string | null>(
    parentSessionId
      ? `Branched from ${parentSessionId}${forkFromMessageId != null ? ` @ turn #${forkFromMessageId}` : ''}`
      : null,
  );
  const scrollRef = useRef<HTMLDivElement>(null);
  const [pinnedToBottom, setPinnedToBottom] = useState(true);
  const lastMsgText = messages.length > 0 ? messages[messages.length - 1]?.text : '';

  const chunkedWindow = computeChunkedThreadWindow(timeline, chunksLoaded, CHUNK_SIZE);
  const visibleTimeline = chunkedWindow.visibleItems;
  const persistEligible = shouldPersistThreadMutation({
    turnCount: messages.length,
    hasActiveTurn: messages.some((m) => m.status === 'streaming' || m.status === 'pending'),
    hasAttachedFiles: false,
  });

  const lastUserPrompt =
    [...messages].reverse().find((m) => m.role === 'user')?.text ?? 'Evaluate current architecture';

  const handleRunConsensus = () => {
    if (consensusBusy) return;
    setConsensusBusy(true);
    void chatConsensusRun({
      prompt: lastUserPrompt,
      models: ['anthropic/claude-sonnet', 'openai/gpt', 'mens/qwen3-8b'],
    })
      .then((res) => {
        setConsensusLegs(res.legs);
        setConsensusSynthesis(res.synthesis ?? null);
      })
      .catch(() => {})
      .finally(() => setConsensusBusy(false));
  };

  const handleAdoptConsensusVariant = (modelId: string) => {
    const runId = sessionId ?? 'default';
    if (typeof vcsMusePoolAdopt === 'function') {
      void vcsMusePoolAdopt(runId, modelId)
        .then((res) => {
          setBranchBanner(
            `Adopted variant ${res.adopted_variant_id} (${res.adopted_branch_name})`,
          );
        })
        .catch(() => {});
    }
  };

  const handleOpenBranchComparison = () => {
    const baseNodes: BranchMessageNode[] = messages.map((m, idx) => ({
      id: m.id,
      parentMessageId: idx === 0 ? null : (messages[idx - 1]?.id ?? null),
      role: m.role,
      content: m.text,
      modelId: m.modelId,
    }));
    const anchorId = baseNodes[Math.max(0, baseNodes.length - 2)]?.id ?? null;
    const altLeafId = `${baseNodes[baseNodes.length - 1]?.id ?? 'msg-1'}-alt-branch`;
    const altNode: BranchMessageNode = {
      id: altLeafId,
      parentMessageId: anchorId,
      role: 'assistant',
      content: `Alternative branch candidate synthesized from LCA (${anchorId ?? 'root'}).`,
      modelId: 'mens/qwen3-8b',
    };
    const tree = buildMessageTree([...baseNodes, altNode]);
    const primaryLeafId = baseNodes[baseNodes.length - 1]?.id ?? altLeafId;
    setBranchComparison(computeDivergencePoint(tree, primaryLeafId, altLeafId));
  };

  const handleRunArenaCompare = () => {
    if (!sessionId || arenaBusy) return;
    setArenaBusy(true);
    setArenaVoteOutcome(null);
    void chatArenaCompare({
      session_id: sessionId,
      prompt: lastUserPrompt,
    })
      .then((cmp) => {
        setArenaComparison(cmp);
      })
      .catch(() => {})
      .finally(() => setArenaBusy(false));
  };

  const handleArenaVote = (winner: 'a' | 'b' | 'tie') => {
    if (!arenaComparison || arenaBusy) return;
    setArenaBusy(true);
    void chatArenaVote({
      session_id: arenaComparison.session_id,
      prompt: arenaComparison.prompt,
      winner,
      model_a: arenaComparison.arm_a.model_id,
      content_a: arenaComparison.arm_a.content,
      latency_ms_a: arenaComparison.arm_a.latency_ms,
      model_b: arenaComparison.arm_b.model_id,
      content_b: arenaComparison.arm_b.content,
      latency_ms_b: arenaComparison.arm_b.latency_ms,
    })
      .then((outcome) => {
        setArenaVoteOutcome(outcome);
      })
      .catch(() => {})
      .finally(() => setArenaBusy(false));
  };

  const parseNumericTurnId = (rawId: string): number => {
    const num = Number.parseInt(rawId.replace(/\D+/g, ''), 10);
    return Number.isFinite(num) && num > 0 ? num : 1;
  };

  const handleForkTurn = (msg: ChatMessage) => {
    if (!sessionId) return;
    const msgNum = parseNumericTurnId(msg.id);
    void chatForkSession(sessionId, msgNum, `Branch @${msgNum}`)
      .then((forked) => {
        setBranchBanner(`Branched new session ${forked.session_id} from turn #${msgNum}`);
        onSessionBranched?.(forked.session_id, msgNum);
      })
      .catch(() => {});
  };

  const handleRewindTurn = (msg: ChatMessage) => {
    if (!sessionId) return;
    const msgNum = parseNumericTurnId(msg.id);
    void chatRewindToTurn(sessionId, msgNum, true)
      .then((rewound) => {
        setBranchBanner(
          `Rewound to turn #${rewound.rewound_to_message_id} (${rewound.vcs_checkpoint_op_id})`,
        );
        onSessionRewound?.(rewound.rewound_to_message_id, rewound.vcs_checkpoint_op_id);
      })
      .catch(() => {});
  };

  const handleEditInPlace = (msg: ChatMessage, newContent: string) => {
    void editChatMessageInPlace({
      sessionId: sessionId ?? 'default',
      messageId: msg.id,
      newContent,
    })
      .then(() => {
        setBranchBanner(`Edited message #${msg.id} in-place`);
      })
      .catch(() => {});
  };

  const handleRegenerateTurn = (msg: ChatMessage) => {
    void regenerateChatTurn({
      sessionId: sessionId ?? 'default',
      fromMessageId: msg.id,
      execute: true,
    })
      .then((res) => {
        setBranchBanner(
          `Regenerating from #${msg.id} (${res.removed_turns ?? 1} turn(s) truncated)`,
        );
      })
      .catch(() => {});
  };

  const handleCompactUpTo = (msg: ChatMessage) => {
    if (typeof compactChatUpTo !== 'function') return;
    void compactChatUpTo({
      sessionId: sessionId ?? 'default',
      messageId: msg.id,
      direction: 'up_to',
    })
      .then((res) => {
        setBranchBanner(
          `Compacted up to #${msg.id} (${res.preserved_thought_signatures?.length ?? 0} signature(s) preserved)`,
        );
      })
      .catch(() => {});
  };

  useEffect(() => {
    const el = scrollRef.current;
    if (!el || !pinnedToBottom) return;
    const scroller = el.parentElement ?? el;
    if (typeof scroller.scrollTo === 'function') {
      scroller.scrollTo({ top: scroller.scrollHeight });
    } else {
      scroller.scrollTop = scroller.scrollHeight;
    }
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
      Promise.resolve(listHarnessIssuesForSession(sessionId))
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
      onScroll={handleScroll}
      className="relative mb-2 min-h-0 flex-1 overflow-y-auto custom-scrollbar p-2.5 pb-4"
    >
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        data-persist-eligible={String(persistEligible)}
        className="mx-auto flex w-full max-w-[900px] flex-col gap-2 pb-3"
      >
        <div className="flex flex-wrap items-center justify-between gap-2">
          <ChatVerbosityControl value={verbosity} onChange={setVerbosity} />
          <div className="flex flex-wrap items-center gap-1.5">
            {branchBanner && (
              <span
                data-testid="chat-session-branch-badge"
                className="rounded border border-brass/35 bg-brass/10 px-2 py-0.5 font-mono text-[11px] text-brass"
              >
                ⑂ {branchBanner}
              </span>
            )}
            <button
              type="button"
              data-testid="chat-consensus-trigger"
              disabled={consensusBusy}
              onClick={handleRunConsensus}
              className="rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              ⊕ Consensus (2–4)
            </button>
            <button
              type="button"
              data-testid="chat-branch-compare-trigger"
              onClick={handleOpenBranchComparison}
              className="rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              ⑂ Compare Branches
            </button>
            <button
              type="button"
              data-testid="chat-arena-compare-toggle"
              aria-pressed={arenaModeEnabled}
              onClick={() => {
                const next = !arenaModeEnabled;
                setArenaModeEnabled(next);
                if (next && !arenaComparison) {
                  handleRunArenaCompare();
                }
              }}
              className={`rounded border px-2 py-0.5 font-mono text-[11px] transition ${
                arenaModeEnabled
                  ? 'border-brass/50 bg-brass/20 text-brass'
                  : 'border-border-subtle bg-overlay-subtle text-text-muted hover:border-brass/40 hover:text-brass'
              }`}
            >
              ⚔ Arena Compare
            </button>
          </div>
        </div>
        {chunkedWindow.hasMoreOlder && (
          <div className="flex justify-center">
            <button
              type="button"
              data-testid="chat-load-older-chunks"
              onClick={() => setChunksLoaded((c) => c + 1)}
              className="rounded border border-border-subtle bg-overlay-subtle px-2.5 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
            >
              Load older turns ({chunkedWindow.totalItems - chunkedWindow.loadedCount} earlier)
            </button>
          </div>
        )}
        {harnessIssues.length > 0 && (
          <div className="mb-1 flex flex-col gap-1 border-b border-border-subtle pb-2">
            {harnessIssues.map((issue) => (
              <HarnessIssueSummary key={issue.id} issue={issue} />
            ))}
          </div>
        )}
        {visibleTimeline.map((row) => {
          if (row.kind === 'message') {
            return (
              <MessageBubble
                key={row.id}
                message={row.message}
                onExcludeSkill={onExcludeSkill}
                verbosity={verbosity}
                onForkTurn={sessionId ? handleForkTurn : undefined}
                onRewindTurn={sessionId ? handleRewindTurn : undefined}
                onEditInPlace={handleEditInPlace}
                onRegenerateTurn={handleRegenerateTurn}
                onCompactUpTo={handleCompactUpTo}
              />
            );
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

        {consensusLegs && consensusLegs.length >= 2 && (
          <ConsensusGridView
            legs={consensusLegs}
            synthesis={consensusSynthesis}
            onAdoptVariant={handleAdoptConsensusVariant}
          />
        )}

        {branchComparison && (
          <BranchComparisonModal
            title="LCA Branch Divergence Inspector"
            leftLabel="Active Thread Branch"
            rightLabel="Divergent Candidate Branch"
            divergence={branchComparison}
            onClose={() => setBranchComparison(null)}
          />
        )}

        {arenaModeEnabled && arenaComparison && (
          <ArenaComparisonCard
            comparison={arenaComparison}
            voteOutcome={arenaVoteOutcome}
            busy={arenaBusy}
            onVote={handleArenaVote}
            renderContent={renderStructuredMessageText}
          />
        )}

        {timeline.length <= 2 && (
          <div
            data-testid="chat-session-brief-deck"
            className="mt-2 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-elevated/45 px-3 py-2"
          >
            <div className="flex items-center gap-2 whitespace-nowrap">
              <span className="size-1.5 rounded-full bg-brass" aria-hidden="true" />
              <span className="font-mono text-[11px] text-text-muted">Quick actions</span>
            </div>
            <div className="flex flex-wrap items-center gap-1.5">
              <button
                type="button"
                onClick={() => insertPromptSnippet('/check crates/vox-compiler')}
                className="inline-flex h-6 items-center gap-1.5 whitespace-nowrap rounded-md border border-border-subtle bg-overlay-subtle px-2.5 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
              >
                <span>Check Code</span>
              </button>
              <button
                type="button"
                onClick={() => insertPromptSnippet('@kb:verified architecture invariants')}
                className="inline-flex h-6 items-center gap-1.5 whitespace-nowrap rounded-md border border-border-subtle bg-overlay-subtle px-2.5 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
              >
                <span>Search Docs</span>
              </button>
              <button
                type="button"
                onClick={() => insertPromptSnippet('@browser:active inspect accessibility refs')}
                className="inline-flex h-6 items-center gap-1.5 whitespace-nowrap rounded-md border border-border-subtle bg-overlay-subtle px-2.5 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
              >
                <span>Inspect Browser</span>
              </button>
              <button
                type="button"
                onClick={() => insertPromptSnippet('/eval --mens-five-spokes')}
                className="inline-flex h-6 items-center gap-1.5 whitespace-nowrap rounded-md border border-border-subtle bg-overlay-subtle px-2.5 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
              >
                <span>Run Evaluation</span>
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
