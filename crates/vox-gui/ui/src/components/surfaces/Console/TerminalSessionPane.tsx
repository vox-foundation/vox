import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTerminalPendingApprovals } from '../../../hooks/useTerminalPendingApprovals';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import {
  getTerminalBridgeState, requestChatAttention, requestConsoleBlockFocus, triggerManualTakeover,
  type TermControlModeId, type TermSecurityLevelId,
} from '../../../lib/terminalBridge';
import * as transport from '../../../transport';
import type { TermAttachmentDto, TermBlockDto } from '../../../transport';
import type { Toast } from '../../../types/tauri';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { Tooltip } from '../../ui/Tooltip';
import { formatBlockSearchCorpus } from './BlockCard';
import { BlockStreamView } from './BlockStreamView';
import { classifyPromptIntent, type ShellMode as PromptShellMode } from './intentRouter';
import { createInitialSessionPaneState, hydrateSessionFromAttachment, reduceSessionEvent, type SessionPaneState } from './sessionReducer';
import { TerminalPaneApprovalBanner } from './TerminalApprovalCard';
import { TERMINAL_MONO_FONT_STACK } from './terminalTheme';
import { TerminalTab, type PendingLine, type ReplaySnapshotPayload } from './TerminalTab';
import { TermSearchBar } from './TermSearchBar';
import { detectSubshellShellHint } from './useConsoleWarpActions';
import type { SearchMatchStats } from './xtermAddons';

export interface SessionMetaSnapshot {
  sessionId: string; blocksCount: number; cwd: string | null;
  gitBranch: string | null; altScreen: boolean; runningCommand: string | null;
}

export interface TerminalSessionPaneProps {
  paneId?: string; sessionId?: string | null; autoOpenSession?: boolean; compact?: boolean; activePane?: boolean;
  controlMode?: TermControlModeId; shellMode?: PromptShellMode; externalPreemptSeq?: number;
  pendingLine?: PendingLine | null; focusedBlockId?: number | null; pinnedBlockIds?: number[]; searchOpenRequestSeq?: number;
  onSessionResolved?: (sessionId: string) => void; onBlocksChange?: (blocks: TermBlockDto[]) => void;
  onBlock?: (block: TermBlockDto) => void; onPinnedBlocksChange?: (pinnedIds: number[]) => void;
  onSelectBlock?: (blockId: number | null) => void; onCopyCommand?: (block: TermBlockDto) => void;
  onCopyOutput?: (block: TermBlockDto) => void; onRerun?: (block: TermBlockDto) => void;
  onAskAgent?: (block: TermBlockDto) => void; onFocusInChat?: (block: TermBlockDto, lineRange?: string) => void;
  onSessionMetaChange?: (meta: SessionMetaSnapshot) => void;
  onSecuritySyncFromBackend?: (level: TermSecurityLevelId) => void; onControlSyncFromBackend?: (mode: TermControlModeId) => void;
  onSplitRight?: () => void; onSplitDown?: () => void; onClosePane?: () => void; onOpenConsole?: () => void; pushToast?: (toast: Toast) => void;
  isVoxified?: boolean; onVoxifySubshell?: (sessionId: string) => void;
  onInsertInPrompt?: (cmd: string) => void; onOpenSecrets?: () => void;
}

interface SharedSessionAttachEntry {
  refCount: number; inFlight: Promise<TermAttachmentDto | null> | null; lastSnap: TermAttachmentDto | null;
}
const sharedSessionAttaches = new Map<string, SharedSessionAttachEntry>();
export const resetSessionPaneAttachRegistryForTests = (): void => { sharedSessionAttaches.clear(); };

function acquireInitialAttach(sid: string): Promise<TermAttachmentDto | null> {
  let entry = sharedSessionAttaches.get(sid);
  if (!entry) { entry = { refCount: 0, inFlight: null, lastSnap: null }; sharedSessionAttaches.set(sid, entry); }
  entry.refCount += 1;
  if (entry.inFlight) return entry.inFlight;
  if (entry.lastSnap) return Promise.resolve(entry.lastSnap);
  if (typeof transport.termSessionAttach !== 'function') return Promise.resolve(null);
  const req = transport.termSessionAttach(sid).then((snap) => {
    const valid = snap && typeof snap === 'object' ? snap : null;
    const cur = sharedSessionAttaches.get(sid); if (cur) { cur.inFlight = null; cur.lastSnap = valid; }
    return valid;
  }).catch((err) => { const cur = sharedSessionAttaches.get(sid); if (cur) cur.inFlight = null; throw err; });
  entry.inFlight = req;
  return req;
}

function releaseInitialAttach(sid: string | null): void {
  if (!sid) return;
  const cur = sharedSessionAttaches.get(sid);
  if (cur && (cur.refCount -= 1) <= 0) sharedSessionAttaches.delete(sid);
}

const basenameOfPath = (rawPath: string): string => rawPath.replace(/[\\/]+$/, '').split(/[\\/]/).filter(Boolean).pop() ?? rawPath;
const findRunningCommand = (blocks: readonly TermBlockDto[]): string | null => {
  for (let i = blocks.length - 1; i >= 0; i -= 1) {
    const b = blocks[i]; if (b && (b.status === 'Running' || b.status === 'AwaitingInput') && b.input.trim()) return b.input.trim();
  }
  return null;
};
function countOccurrences(haystack: string, needleLower: string): number {
  if (!haystack || !needleLower) return 0;
  const lower = haystack.toLowerCase(); let count = 0;
  for (let pos = 0; ; pos += needleLower.length) { pos = lower.indexOf(needleLower, pos); if (pos === -1) return count; count += 1; }
}

export function TerminalSessionPane({
  paneId = 'pane-1', sessionId, autoOpenSession = true, compact = false, activePane = true, controlMode, shellMode = 'default', externalPreemptSeq,
  pendingLine = null, focusedBlockId = null, pinnedBlockIds, searchOpenRequestSeq,
  onSessionResolved, onBlocksChange, onBlock, onPinnedBlocksChange, onSelectBlock,
  onCopyCommand, onCopyOutput, onRerun, onAskAgent, onFocusInChat, onSessionMetaChange,
  onSecuritySyncFromBackend, onControlSyncFromBackend, onSplitRight, onSplitDown, onClosePane, onOpenConsole, pushToast,
  isVoxified = false, onVoxifySubshell, onInsertInPrompt, onOpenSecrets,
}: TerminalSessionPaneProps) {
  const paneRootRef = useRef<HTMLDivElement | null>(null);
  const [internalSid, setInternalSid] = useState<string | null>(sessionId ?? null);
  const effectiveSessionId = sessionId !== undefined && sessionId !== null ? sessionId : internalSid;
  const [paneState, setPaneState] = useState<SessionPaneState>(() => createInitialSessionPaneState({ sessionId: effectiveSessionId, ...(controlMode ? { controlMode } : {}) }));
  const stateRef = useRef<SessionPaneState>(paneState);
  const activeSidRef = useRef<string | null>(effectiveSessionId);
  activeSidRef.current = effectiveSessionId;

  const [replaySnapshot, setReplaySnapshot] = useState<ReplaySnapshotPayload | null>(null);
  const replayEpochRef = useRef(0), lastSubmittedSeqRef = useRef<number | null>(null), lastSearchReqSeqRef = useRef<number | undefined>(searchOpenRequestSeq), vtSearchSeqRef = useRef(0);
  const lastExtPreemptRef = useRef(externalPreemptSeq ?? 0), snapshotEventSeqRef = useRef(0), dispatchPreSnapSeqRef = useRef(0);
  const [viewMode, setViewMode] = useState<'blocks' | 'vt'>('blocks');
  const [clearedEmpty, setClearedEmpty] = useState(false);
  const [manualExitRawOverride, setManualExitRawOverride] = useState(false);
  const [webglFallbackNotice, setWebglFallbackNotice] = useState(false);
  const [disconnectError, setDisconnectError] = useState<string | null>(null);
  const [internalFocusedBlockId, setInternalFocusedBlockId] = useState<number | null>(null);
  const [dockInput, setDockInput] = useState(''), [searchOpen, setSearchOpen] = useState(false), [searchQuery, setSearchQuery] = useState(''), [blockMatchCursor, setBlockMatchCursor] = useState(0);
  const [vtSearchStats, setVtSearchStats] = useState<SearchMatchStats>({ activeMatchIndex: 0, totalMatches: 0 });
  const [vtSearchStep, setVtSearchStep] = useState<{ seq: number; dir: 'next' | 'prev' } | null>(null);

  const cbRef = useRef({ onSessionResolved, onBlocksChange, onBlock, onPinnedBlocksChange, onSessionMetaChange, onSecuritySyncFromBackend, onControlSyncFromBackend });
  cbRef.current = { onSessionResolved, onBlocksChange, onBlock, onPinnedBlocksChange, onSessionMetaChange, onSecuritySyncFromBackend, onControlSyncFromBackend };

  const notifySync = useCallback((sid: string, next: SessionPaneState, closedBlock?: TermBlockDto | null) => {
    cbRef.current.onBlocksChange?.(next.blocks); cbRef.current.onPinnedBlocksChange?.(next.pinnedBlockIds);
    if (closedBlock) cbRef.current.onBlock?.(closedBlock);
    cbRef.current.onSessionMetaChange?.({ sessionId: sid, blocksCount: next.blocks.length, cwd: next.cwd, gitBranch: next.gitBranch, altScreen: next.altScreen, runningCommand: findRunningCommand(next.blocks) });
  }, []);

  useEffect(() => {
    if (!controlMode || stateRef.current.controlMode === controlMode) return;
    const next = { ...stateRef.current, controlMode };
    stateRef.current = next; setPaneState(next);
  }, [controlMode]);

  const applyAttachmentSnapshot = useCallback((sid: string, snap: TermAttachmentDto, forceFullReset: boolean) => {
    const cleanState: SessionPaneState = { ...hydrateSessionFromAttachment(stateRef.current, snap), sessionId: sid, resyncRequired: false };
    stateRef.current = cleanState; setPaneState(cleanState); setDisconnectError(null);
    if (cleanState.blocks.length > 0) setClearedEmpty(false);
    if (snap.alt_screen !== undefined) setManualExitRawOverride(false);
    const shouldReset = forceFullReset || snap.resync_required === true;
    if (shouldReset || Boolean(snap.replay_utf8)) {
      replayEpochRef.current += 1;
      setReplaySnapshot({ text: snap.replay_utf8 ?? '', epoch: replayEpochRef.current, seq: cleanState.seq, append: !shouldReset });
    }
    if (snap.security_level) cbRef.current.onSecuritySyncFromBackend?.(cleanState.securityLevel);
    if (snap.control_mode) cbRef.current.onControlSyncFromBackend?.(cleanState.controlMode);
    const completed = [...cleanState.blocks].reverse().find((b) => b.status !== 'Running' && b.status !== 'AwaitingInput');
    notifySync(sid, cleanState, completed ?? null);
  }, [notifySync]);

  const latestBlockId = paneState.blocks[paneState.blocks.length - 1]?.id ?? null;
  const {
    approvals, resolvingId, preemptedBanner, diagnosingBlockId, resolveApproval, stageOrRunProposedCommand,
    dispatchTerminalLine, runExplainFix, submitBlockFeedback, cancelAgentTurn, triggerPreemptionNotice, resumeCoPilot, dismissPreemptedBanner,
  } = useTerminalPendingApprovals({
    sessionId: effectiveSessionId, controlMode: controlMode ?? paneState.controlMode, enabled: activePane, latestBlockId, pushToast,
    onControlModeChange: (mode) => {
      const next = { ...stateRef.current, controlMode: mode }; stateRef.current = next; setPaneState(next);
      cbRef.current.onControlSyncFromBackend?.(mode);
    },
    onClearedSession: () => setClearedEmpty(true),
    onSlashMutatedBlocks: async () => {
      if (snapshotEventSeqRef.current > dispatchPreSnapSeqRef.current) return;
      const sid = activeSidRef.current;
      if (!sid || typeof transport.termSessionAttach !== 'function') return;
      const snap = await transport.termSessionAttach(sid).catch(() => null);
      if (snap && typeof snap === 'object') applyAttachmentSnapshot(sid, snap, false);
    },
  });

  useEffect(() => {
    if (externalPreemptSeq !== undefined && externalPreemptSeq > lastExtPreemptRef.current) {
      lastExtPreemptRef.current = externalPreemptSeq; triggerPreemptionNotice();
    }
  }, [externalPreemptSeq, triggerPreemptionNotice]);

  const preemptAutopilot = useCallback(() => {
    const sid = activeSidRef.current;
    const cur = controlMode ?? stateRef.current.controlMode;
    if (cur === 'autopilot' || getTerminalBridgeState().controlMode === 'autopilot') {
      const next = { ...stateRef.current, controlMode: 'manual' as const };
      stateRef.current = next; setPaneState(next); cbRef.current.onControlSyncFromBackend?.('manual');
      triggerManualTakeover(sid); triggerPreemptionNotice();
      if (sid && typeof transport.termSessionSetControl === 'function') transport.termSessionSetControl(sid, 'manual').catch(() => {});
    }
  }, [controlMode, triggerPreemptionNotice]);

  useEffect(() => {
    if (sessionId !== undefined && sessionId !== null) setInternalSid(sessionId);
  }, [sessionId]);

  useEffect(() => {
    let disposed = false, acquiredSid: string | null = null, unsubEvents: (() => void) | undefined;
    (async () => {
      let sid = sessionId ?? null;
      if (!sid) {
        if (!autoOpenSession || typeof transport.termSessionOpen !== 'function') return;
        try {
          const opened = await transport.termSessionOpen(80, 24);
          if (disposed || !opened || typeof opened !== 'string') return;
          sid = opened; activeSidRef.current = sid; setInternalSid(sid); cbRef.current.onSessionResolved?.(sid);
        } catch (err) { if (!disposed) setDisconnectError(sanitizeErrorForToast(err)); return; }
      } else activeSidRef.current = sid;

      const resetBase = createInitialSessionPaneState({ sessionId: sid });
      stateRef.current = resetBase; setPaneState(resetBase); setManualExitRawOverride(false);

      if (typeof transport.listenTermEvent === 'function') {
        try {
          const u = await transport.listenTermEvent((payload) => {
            if (disposed || !payload || payload.session_id !== sid) return;
            const reduced = reduceSessionEvent(stateRef.current, payload.event);
            const payloadSeq = (payload as { seq?: unknown }).seq;
            if (typeof payloadSeq === 'number') reduced.seq = Math.max(reduced.seq, payloadSeq);
            const wasResync = reduced.resyncRequired;
            const next: SessionPaneState = wasResync ? { ...reduced, resyncRequired: false } : reduced;
            stateRef.current = next; setPaneState(next);
            let closedBlock: TermBlockDto | null = null;
            if (payload.event && typeof payload.event === 'object') {
              if ('AltScreenChanged' in payload.event) setManualExitRawOverride(false);
              if ('SnapshotResync' in payload.event) {
                setManualExitRawOverride(false);
                snapshotEventSeqRef.current += 1;
                if (next.blocks.length > 0) setClearedEmpty(false);
              }
              if ('BlockOpened' in payload.event) setClearedEmpty(false);
              if ('BlockClosed' in payload.event) {
                const closedId = Number((payload.event as { BlockClosed?: { id?: unknown } }).BlockClosed?.id);
                closedBlock = next.blocks.find((b) => b.id === closedId) ?? null;
              }
              if ('SecurityLevelChanged' in payload.event) cbRef.current.onSecuritySyncFromBackend?.(next.securityLevel);
              if ('ControlModeChanged' in payload.event) cbRef.current.onControlSyncFromBackend?.(next.controlMode);
            }
            if (wasResync) { replayEpochRef.current += 1; setReplaySnapshot({ text: '', epoch: replayEpochRef.current, seq: next.seq, append: false }); }
            notifySync(sid, next, closedBlock);
          });
          if (disposed) { if (typeof u === 'function') u(); return; }
          if (typeof u === 'function') unsubEvents = u;
        } catch { /* headless fallback */ }
      }
      if (typeof transport.termSessionAttach !== 'function') return;
      acquiredSid = sid;
      try {
        const snap = await acquireInitialAttach(sid);
        if (!disposed && snap && typeof snap === 'object') applyAttachmentSnapshot(sid, snap, true);
      } catch (err) { if (!disposed) setDisconnectError(sanitizeErrorForToast(err)); }
    })();
    return () => { disposed = true; unsubEvents?.(); releaseInitialAttach(acquiredSid); };
  }, [sessionId, autoOpenSession, applyAttachmentSnapshot, notifySync]);

  const handleReconnect = useCallback(async () => {
    const sid = activeSidRef.current;
    if (!sid || typeof transport.termSessionAttach !== 'function') return;
    try {
      const snap = await transport.termSessionAttach(sid, stateRef.current.seq);
      if (snap && typeof snap === 'object') {
        const cur = sharedSessionAttaches.get(sid);
        if (cur) cur.lastSnap = snap;
        applyAttachmentSnapshot(sid, snap, false);
      }
    } catch (err) {
      const sanitized = sanitizeErrorForToast(err);
      setDisconnectError(sanitized);
      pushToast?.({ tone: 'warn', title: 'Reconnect failed', body: sanitized, cause: 'backend-error' });
    }
  }, [applyAttachmentSnapshot, pushToast]);

  const submitShellLine = useCallback(async (shellLine: string) => {
    const sid = activeSidRef.current;
    if (!sid || typeof transport.termSessionSubmit !== 'function') return;
    try { await transport.termSessionSubmit(sid, shellLine); } catch (err) {
      pushToast?.({ tone: 'warn', title: 'Command submit failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  const submitLine = useCallback(async (line: string) => {
    preemptAutopilot();
    const sid = activeSidRef.current;
    if (!sid) return;
    const intent = classifyPromptIntent(line, shellMode);
    dispatchPreSnapSeqRef.current = snapshotEventSeqRef.current;
    await dispatchTerminalLine(sid, line, submitShellLine, { forceAgent: intent.kind === 'agent' });
  }, [dispatchTerminalLine, preemptAutopilot, shellMode, submitShellLine]);

  useEffect(() => {
    if (!pendingLine || !effectiveSessionId || pendingLine.seq === lastSubmittedSeqRef.current) return;
    lastSubmittedSeqRef.current = pendingLine.seq;
    void submitLine(pendingLine.text);
  }, [pendingLine, effectiveSessionId, submitLine]);

  useEffect(() => {
    if (searchOpenRequestSeq !== undefined && searchOpenRequestSeq !== lastSearchReqSeqRef.current) {
      lastSearchReqSeqRef.current = searchOpenRequestSeq;
      if (searchOpenRequestSeq > 0) setSearchOpen(true);
    }
  }, [searchOpenRequestSeq]);

  const handleAltScreenHint = useCallback((active: boolean) => {
    setManualExitRawOverride(false);
    const next: SessionPaneState = { ...stateRef.current, altScreen: active };
    stateRef.current = next;
    setPaneState(next);
    if (activeSidRef.current) notifySync(activeSidRef.current, next, null);
  }, [notifySync]);

  const showVt = (paneState.altScreen && !manualExitRawOverride) || viewMode === 'vt';
  const blockSearchMatches = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!searchOpen || !q) return [] as number[];
    const hits: number[] = [];
    for (const b of paneState.blocks) {
      const count = countOccurrences(formatBlockSearchCorpus(b), q);
      for (let i = 0; i < count; i += 1) hits.push(b.id);
    }
    return hits;
  }, [paneState.blocks, searchOpen, searchQuery]);

  const totalMatches = showVt ? vtSearchStats.totalMatches : blockSearchMatches.length;
  const clampedCursor = blockSearchMatches.length > 0 ? ((blockMatchCursor % blockSearchMatches.length) + blockSearchMatches.length) % blockSearchMatches.length : 0;
  const activeMatchIndex = showVt ? vtSearchStats.activeMatchIndex : blockSearchMatches.length > 0 ? clampedCursor + 1 : 0;
  const activeSearchBlockId = !showVt && blockSearchMatches.length > 0 ? (blockSearchMatches[clampedCursor] ?? null) : null;

  const closeSearchAndRestoreFocus = useCallback(() => {
    setSearchOpen(false);
    setSearchQuery('');
    const vtTa = paneRootRef.current?.querySelector<HTMLTextAreaElement>('.xterm-helper-textarea');
    if (showVt && vtTa && vtTa.tabIndex >= 0) vtTa.focus();
    else paneRootRef.current?.focus();
  }, [showVt]);

  const stepSearch = (dir: 'next' | 'prev') => {
    if (showVt) { vtSearchSeqRef.current += 1; setVtSearchStep({ seq: vtSearchSeqRef.current, dir }); }
    if (blockSearchMatches.length > 0) setBlockMatchCursor((prev) => (dir === 'next' ? prev + 1 : prev - 1));
  };

  const mergedPinnedIds = useMemo(() => (!pinnedBlockIds?.length ? paneState.pinnedBlockIds : [...new Set([...paneState.pinnedBlockIds, ...pinnedBlockIds])]), [paneState.pinnedBlockIds, pinnedBlockIds]);
  const handlePinBlock = (block: TermBlockDto) => {
    const nextPinned = !mergedPinnedIds.includes(block.id);
    const nextState = reduceSessionEvent(stateRef.current, { BlockPinned: { id: block.id, pinned: nextPinned } });
    stateRef.current = nextState;
    setPaneState(nextState);
    onPinnedBlocksChange?.(nextState.pinnedBlockIds);
    const sid = activeSidRef.current;
    if (sid && typeof transport.termSessionPin === 'function') transport.termSessionPin(sid, block.id, nextPinned).catch(() => {});
  };

  const activeSubshellHint = !compact && !isVoxified ? detectSubshellShellHint(findRunningCommand(paneState.blocks)) : null;

  const searchTriggerBtn = (
    <Button size="xs" variant="ghost" data-testid="term-search-trigger" aria-label="Search Output" onClick={() => setSearchOpen((p) => !p)} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold">
      <Icon.search className="size-4" aria-hidden="true" />{!compact ? <span>Search Output</span> : null}
    </Button>
  );

  return (
    <div ref={paneRootRef} tabIndex={-1} data-testid="terminal-session-pane" style={{ fontFamily: TERMINAL_MONO_FONT_STACK }} onKeyDown={(e) => { if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'f') { e.preventDefault(); e.stopPropagation(); setSearchOpen(true); } }} className="relative flex h-full min-h-0 w-full min-w-0 flex-1 flex-col bg-bg-base text-xs text-text-primary focus:outline-none">
      <div className="flex min-h-8 shrink-0 flex-wrap items-center justify-between gap-1 border-b border-border-subtle bg-bg-surface px-2 py-1 text-xs">
        <div className="flex min-w-0 flex-wrap items-center gap-1">
          {compact ? <span className="shrink-0 text-[11px] font-semibold text-text-secondary">{`term:${effectiveSessionId ?? 'detached'} (${paneState.blocks.length})`}</span> : null}
          {paneState.cwd ? <span title={paneState.cwd} className="inline-flex min-w-0 items-center gap-1 truncate rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-secondary"><Icon.folder className="size-3 shrink-0" aria-hidden="true" /><span className="truncate">{basenameOfPath(paneState.cwd)}</span></span> : null}
          {paneState.gitBranch ? <span title={paneState.gitBranch} className="inline-flex min-w-0 max-w-36 items-center gap-1 truncate rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-secondary"><Icon.branch className="size-3 shrink-0" aria-hidden="true" /><span className="truncate">{paneState.gitBranch}</span></span> : null}
          {webglFallbackNotice ? <span role="status" className="truncate text-[11px] text-text-muted">WebGL renderer reset - switched to DOM terminal renderer automatically.</span> : null}
        </div>
        <div className="flex flex-wrap items-center gap-1">
          <div role="group" aria-label="Terminal view mode" className="inline-flex h-6 items-center rounded border border-border-subtle bg-bg-elevated">
            <button type="button" data-testid="term-view-mode-blocks" aria-pressed={!showVt} onClick={() => { setManualExitRawOverride(true); setViewMode('blocks'); }} className={`h-full rounded-l px-2 text-[11px] font-semibold transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring ${!showVt ? 'bg-overlay-subtle text-accent' : 'text-text-secondary hover:text-text-primary'}`}>View Blocks</button>
            <button type="button" data-testid="term-view-mode-toggle" aria-pressed={showVt} onClick={() => { setManualExitRawOverride(false); setViewMode((p) => (p === 'blocks' ? 'vt' : 'blocks')); }} className={`h-full rounded-r border-l border-border-subtle px-2 text-[11px] font-semibold transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring ${showVt ? 'bg-overlay-subtle text-accent' : 'text-text-secondary hover:text-text-primary'}`}>View Raw VT</button>
          </div>
          {compact ? <Tooltip content="Search Output">{searchTriggerBtn}</Tooltip> : searchTriggerBtn}
          {onSplitRight ? <Button size="xs" variant="ghost" data-testid={activePane ? 'term-split-right' : `term-split-right-${paneId}`} aria-label="Split Pane Right" onClick={onSplitRight} className="h-6 px-2 py-1 text-[11px] font-semibold">Split Pane Right</Button> : null}
          {onSplitDown ? <Button size="xs" variant="ghost" data-testid={activePane ? 'term-split-down' : `term-split-down-${paneId}`} aria-label="Split Pane Down" onClick={onSplitDown} className="h-6 px-2 py-1 text-[11px] font-semibold">Split Pane Down</Button> : null}
          {onClosePane ? <Button size="xs" variant="ghost" data-testid={activePane ? 'term-close-pane' : `term-close-pane-${paneId}`} aria-label="Close Terminal Pane" onClick={onClosePane} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold"><Icon.close className="size-3" aria-hidden="true" /><span>Close Terminal Pane</span></Button> : null}
          {compact && onOpenConsole ? <Button size="xs" variant="ghost" data-testid="chat-term-open-console" onClick={() => { requestConsoleBlockFocus({ sessionId: effectiveSessionId ?? null, blockId: focusedBlockId ?? internalFocusedBlockId ?? null }); onOpenConsole(); }} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold text-accent"><Icon.externalLink className="size-3" aria-hidden="true" /><span>Open in Console</span></Button> : null}
        </div>
      </div>

      <TermSearchBar open={searchOpen} query={searchQuery} activeMatchIndex={activeMatchIndex} totalMatches={totalMatches} onQueryChange={(next) => { setSearchQuery(next); setBlockMatchCursor(0); }} onNext={() => stepSearch('next')} onPrev={() => stepSearch('prev')} onClose={closeSearchAndRestoreFocus} />

      {paneState.altScreen && !manualExitRawOverride ? (
        <div data-testid="term-tui-active-banner" className="flex h-8 shrink-0 items-center justify-between gap-2 border-b border-(--color-status-warn) bg-bg-elevated px-3 text-[11px] font-semibold text-text-primary">
          <div className="flex items-center gap-2"><span className="size-2 animate-pulse rounded-full bg-(--color-status-pass)" aria-hidden="true" /><Icon.terminal className="size-4 text-(--color-status-pass)" aria-hidden="true" /><span>TUI Active - Return on exit</span></div>
          <Button size="xs" variant="ghost" onClick={() => setManualExitRawOverride(true)} className="h-6 px-2 py-1 text-[11px] font-semibold">Exit Raw View</Button>
        </div>
      ) : null}

      {activeSubshellHint && effectiveSessionId && onVoxifySubshell ? (
        <div data-testid="term-subshell-voxify-banner" className="flex h-8 shrink-0 items-center justify-between gap-2 border-b border-border-subtle bg-bg-elevated px-2 text-[11px] font-semibold text-text-secondary">
          <span>Nested subshell detected - prompt framing paused</span>
          <button type="button" data-testid="term-voxify-subshell-inline" aria-label="Voxify Subshell" onClick={() => onVoxifySubshell(effectiveSessionId)} className="inline-flex h-6 items-center gap-1 rounded border border-border-subtle bg-bg-surface px-2 py-1 text-[11px] font-semibold text-accent hover:text-text-primary">
            <Icon.spark className="size-3" aria-hidden="true" />
            <span>Voxify Subshell</span>
          </button>
        </div>
      ) : null}

      {disconnectError ? (
        <div role="alert" className="flex shrink-0 items-center justify-between gap-2 border-b border-(--color-status-fail)/50 bg-bg-elevated px-3 py-2 text-xs text-text-primary">
          <span className="truncate">{`Terminal session bridge unavailable: ${disconnectError}. Click Reconnect Session to re-attach to the daemon.`}</span>
          <Button size="xs" variant="outline" onClick={() => void handleReconnect()} className="h-6 shrink-0 px-2 py-1 text-[11px] font-semibold">Reconnect Session</Button>
        </div>
      ) : null}

      <TerminalPaneApprovalBanner
        approvals={approvals} resolvingId={resolvingId} compact={compact} preemptedBanner={preemptedBanner}
        onResumeCoPilot={resumeCoPilot} onDismissPreemptedBanner={dismissPreemptedBanner} onResolveApproval={resolveApproval}
      />

      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden bg-bg-base">
        <div className={showVt ? 'hidden' : 'flex min-h-0 flex-1 flex-col'}>
          <BlockStreamView
            blocks={paneState.blocks} pinnedBlockIds={mergedPinnedIds} focusedBlockId={focusedBlockId ?? internalFocusedBlockId} compact={compact} clearedEmpty={clearedEmpty}
            searchQuery={searchOpen ? searchQuery : undefined} activeSearchBlockId={searchOpen ? activeSearchBlockId : null}
            onCopyCommand={(b) => (onCopyCommand ? onCopyCommand(b) : void navigator.clipboard?.writeText?.(b.input)?.catch(() => {}))}
            onCopyOutput={(b) => (onCopyOutput ? onCopyOutput(b) : void navigator.clipboard?.writeText?.(b.output.map((c) => c.text).join(''))?.catch(() => {}))}
            onRerun={(b) => (onRerun ? onRerun(b) : void submitLine(b.input))} onPin={handlePinBlock}
            onAskAgent={(b) => (onAskAgent ? onAskAgent(b) : requestChatAttention(`@term:${effectiveSessionId ?? 's-1'}#${b.id}`))}
            onFocusInChat={(b, r) => (onFocusInChat ? onFocusInChat(b, r) : requestChatAttention(r ? `@term:${effectiveSessionId ?? 's-1'}#${b.id}:${r}` : `@term:${effectiveSessionId ?? 's-1'}#${b.id}`))}
            onSelectBlock={(id) => { setInternalFocusedBlockId(id); onSelectBlock?.(id); }}
            onSendStdin={(_b, line) => { preemptAutopilot(); if (effectiveSessionId && typeof transport.termSessionInput === 'function') transport.termSessionInput(effectiveSessionId, `${line}\n`).catch(() => {}); }}
            onPromoteBg={(b) => { if (effectiveSessionId && typeof transport.termSessionPromoteBg === 'function') transport.termSessionPromoteBg(effectiveSessionId, b.id).catch(() => {}); }}
            onInterrupt={() => { preemptAutopilot(); if (effectiveSessionId && typeof transport.termSessionInput === 'function') transport.termSessionInput(effectiveSessionId, '\x03').catch(() => {}); }}
            onExplainFixBlock={(b, mode = 'fix') => void runExplainFix(b, mode).then((res) => { if (res?.block_id) setInternalFocusedBlockId(res.block_id); })}
            diagnosingBlockId={diagnosingBlockId}
            onRunProposedCommand={(cmd, sourceBlockId) => void stageOrRunProposedCommand(cmd, submitShellLine, sourceBlockId)}
            onInsertInPrompt={onInsertInPrompt} onOpenSecrets={onOpenSecrets}
            onCancelAgentTurn={(b) => void cancelAgentTurn(b)} onFeedback={submitBlockFeedback}
          />
        </div>
        <div data-testid="terminal-vt-layer" aria-hidden={!showVt ? 'true' : undefined} className={showVt ? 'relative z-10 min-h-0 flex-1 opacity-100 pointer-events-auto' : 'pointer-events-none absolute inset-0 -z-10 opacity-0'}>
          <TerminalTab tabId={paneId} sessionId={effectiveSessionId} skipInitialAttach active={showVt} suppressHelperTextarea={!showVt} replaySnapshot={replaySnapshot} onOutputSeq={(seq) => { stateRef.current.seq = Math.max(stateRef.current.seq, seq); }} onUserInput={preemptAutopilot} searchQuery={showVt && searchOpen ? searchQuery : undefined} searchDirectionStep={showVt && searchOpen ? vtSearchStep : null} onSearchStats={setVtSearchStats} onAltScreenHint={handleAltScreenHint} onWebglFallback={() => setWebglFallbackNotice(true)} />
        </div>
      </div>

      {compact ? (
        <form onSubmit={(e) => { e.preventDefault(); const trimmed = dockInput.trim(); if (!trimmed) return; setDockInput(''); void submitLine(trimmed); }} className="flex shrink-0 items-center gap-2 border-t border-border-subtle bg-bg-surface p-2">
          <input type="text" data-terminal-mono="true" value={dockInput} onChange={(e) => { if (e.target.value) preemptAutopilot(); setDockInput(e.target.value); }} data-testid="dock-term-input" aria-label="dock terminal input" placeholder="Run shell command..." style={{ fontFamily: TERMINAL_MONO_FONT_STACK }} className="h-6 min-w-0 flex-1 rounded border border-border-subtle bg-bg-base px-2 text-xs text-text-primary placeholder:text-text-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring" />
          <Button type="submit" size="xs" variant="secondary" className="h-6 px-2 py-1 text-[11px] font-semibold">Run Command</Button>
        </form>
      ) : null}
    </div>
  );
}
