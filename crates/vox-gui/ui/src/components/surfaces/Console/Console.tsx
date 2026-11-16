import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import {
  consumePendingConsoleBlockFocus, onConsoleBlockFocus, onTerminalCommand,
  publishTerminalBridgeState, requestChatAttention,
  type ConsoleFocusTarget, type TermControlModeId, type TermSecurityLevelId,
} from '../../../lib/terminalBridge';
import { orchestratorStatusErrorMessage, useOrchestratorStatus } from '../../../hooks/useOrchestratorStatus';
import {
  sendToAgent, termSessionClose, termSessionList, termSessionOpen, termSessionPin,
  termSessionSetControl, termSessionSetSecurity, type TermBlockDto, type TermSessionInfoDto,
} from '../../../transport';
import type { OrchestratorStatus, RawAgentSummary, Toast } from '../../../types/tauri';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { AgentStrip, type AgentChip } from './AgentStrip';
import { AgentTab } from './AgentTab';
import { ConsoleSessionBar } from './ConsoleSessionBar';
import {
  ConsoleSplitGrid, MAX_SPLIT_PANES, assignPaneSession, closePaneLeaf, countPaneLeaves,
  findFirstLeaf, findPaneLeaf, findPaneLeafBySession, replaceSessionInTree, splitPaneLeaf, type PaneNode,
} from './ConsoleSplitGrid';
import { DiscoveryRail } from './DiscoveryRail';
import { InputEditor, type PromptShellMode, type ShellMode } from './InputEditor';
import { SendToAgent } from './SendToAgent';
import { fromWireControlMode, fromWireSecurityLevel, toWireControlMode, toWireSecurityLevel } from './sessionReducer';
import { TerminalSessionPane, type SessionMetaSnapshot } from './TerminalSessionPane';
import type { PendingLine } from './TerminalTab';
import { useConsoleWarpActions } from './useConsoleWarpActions';
import { renderTermBlockForAgent } from './webglAddon';

interface SessionUiRecord { blocks: TermBlockDto[]; pinnedIds: number[]; securityLevel: TermSecurityLevelId; controlMode: TermControlModeId }
const DEFAULT_SESSION_UI: SessionUiRecord = { blocks: [], pinnedIds: [], securityLevel: 'l3_workspace_write', controlMode: 'co_pilot' };

function isInteractiveShortcutTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement || target.isContentEditable) return true;
  return Boolean(target.closest('button, input, textarea, select, a, [role="combobox"], [role="listbox"], [role="option"], [role="dialog"], [contenteditable="true"]'));
}

function agentsFromStatus(status: OrchestratorStatus | undefined): AgentChip[] {
  return (status?.agents ?? []).map((a: RawAgentSummary) => ({
    id: String(a.id ?? ''), name: String(a.name ?? a.codename ?? a.id ?? 'agent'), state: a.paused ? 'paused' : a.in_progress ? 'running' : 'queued',
  }));
}

interface Props { pushToast: (item: Toast) => void; gamifyEnabled?: boolean; initialAgentId?: string | null }

export function Console({ pushToast, gamifyEnabled = false, initialAgentId = null }: Props) {
  const orchQuery = useOrchestratorStatus();
  const agents = useMemo(() => agentsFromStatus(orchQuery.data), [orchQuery.data]);
  const orchError = orchestratorStatusErrorMessage(orchQuery);

  const consoleRootRef = useRef<HTMLDivElement | null>(null);
  const seqRef = useRef(0);
  const paneSeqRef = useRef(1);
  const splittingRef = useRef(false);

  const [shellMode, setShellMode] = useState<PromptShellMode>('default');
  const [sessionUiById, setSessionUiById] = useState<Record<string, SessionUiRecord>>({});
  const [fallbackSessionUi, setFallbackSessionUi] = useState<SessionUiRecord>(DEFAULT_SESSION_UI);
  const [pending, setPending] = useState<PendingLine | null>(null);
  const [activeAction, setActiveAction] = useState<string | null>(null);
  const [openAgentId, setOpenAgentId] = useState<string | null>(initialAgentId);
  const [composing, setComposing] = useState(false);
  const [lastLine, setLastLine] = useState('');
  const [sessions, setSessions] = useState<TermSessionInfoDto[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [splitRoot, setSplitRoot] = useState<PaneNode>({ kind: 'leaf', paneId: 'pane-1', sessionId: null });
  const [activePaneId, setActivePaneId] = useState<string>('pane-1');
  const [runningBySession, setRunningBySession] = useState<Record<string, string | null>>({});
  const [focusedBlockId, setFocusedBlockId] = useState<number | null>(null);
  const [latestBlock, setLatestBlock] = useState<TermBlockDto | null>(null);
  const [applyLine, setApplyLine] = useState<string | null>(null);
  const [searchOpenSeq, setSearchOpenSeq] = useState(0);
  const [externalPreemptSeq, setExternalPreemptSeq] = useState(0);

  const activeUi = (activeSessionId && sessionUiById[activeSessionId]) ?? fallbackSessionUi;
  const { blocks, pinnedIds: pinnedBlockIds, securityLevel, controlMode } = activeUi;
  const activeCwd = sessions.find((s) => s.id === activeSessionId)?.cwd;

  const activeSessionRef = useRef<string | null>(activeSessionId); activeSessionRef.current = activeSessionId;
  const controlModeRef = useRef<TermControlModeId>(controlMode); controlModeRef.current = controlMode;
  const activePaneRef = useRef<string>(activePaneId); activePaneRef.current = activePaneId;
  const splitRootRef = useRef<PaneNode>(splitRoot); splitRootRef.current = splitRoot;
  const sessionsRef = useRef<TermSessionInfoDto[]>(sessions); sessionsRef.current = sessions;

  const patchSessionUi = useCallback((sid: string | null, patch: Partial<SessionUiRecord>) => {
    if (!sid) { setFallbackSessionUi((prev) => ({ ...prev, ...patch })); return; }
    setSessionUiById((prev) => ({ ...prev, [sid]: { ...(prev[sid] ?? DEFAULT_SESSION_UI), ...patch } }));
  }, []);

  const setAndSyncControlMode = useCallback((next: TermControlModeId, sidOverride?: string | null) => {
    const sid = sidOverride ?? activeSessionRef.current;
    controlModeRef.current = next;
    patchSessionUi(sid, { controlMode: next });
    if (sid) setFallbackSessionUi((prev) => ({ ...prev, controlMode: next }));
    if (sid && typeof termSessionSetControl === 'function') termSessionSetControl(sid, toWireControlMode(next)).catch(() => {});
  }, [patchSessionUi]);

  const handleSecurityLevelChange = useCallback((next: TermSecurityLevelId) => {
    const sid = activeSessionRef.current;
    patchSessionUi(sid, { securityLevel: next });
    if (sid) setFallbackSessionUi((prev) => ({ ...prev, securityLevel: next }));
    if (sid && typeof termSessionSetSecurity === 'function') termSessionSetSecurity(sid, toWireSecurityLevel(next)).catch(() => {});
  }, [patchSessionUi]);

  const cycleControlMode = useCallback(() => {
    const cur = controlModeRef.current;
    setAndSyncControlMode(cur === 'manual' ? 'co_pilot' : cur === 'co_pilot' || cur === 'copilot' ? 'autopilot' : 'manual');
  }, [setAndSyncControlMode]);

  const preemptAutopilotFromConsole = useCallback(() => {
    if (controlModeRef.current === 'autopilot') {
      setAndSyncControlMode('manual');
      setExternalPreemptSeq((n) => n + 1);
    }
  }, [setAndSyncControlMode]);

  const submitRaw = useCallback((line: string) => {
    seqRef.current += 1;
    setLastLine(line);
    preemptAutopilotFromConsole();
    setPending({ text: line, seq: seqRef.current });
  }, [preemptAutopilotFromConsole]);

  const {
    historyOpenRequestSeq, openHistorySearch, railTab, setRailTab,
    selectedRunbookId, voxifiedSessionIds, handleVoxifySubshell, resetSessionVoxifiedIfSubshell,
  } = useConsoleWarpActions({ runningBySession, submit: submitRaw, setApplyLine, pushToast });

  const submit = useCallback((line: string) => {
    resetSessionVoxifiedIfSubshell(activeSessionRef.current, line);
    submitRaw(line);
  }, [resetSessionVoxifiedIfSubshell, submitRaw]);

  const handleFocusInChat = useCallback((b: TermBlockDto, lineRange?: string) => {
    const sid = activeSessionRef.current ?? 's-1';
    const tag = lineRange ? `@term:${sid}#${b.id}:${lineRange}` : `@term:${sid}#${b.id}`;
    requestChatAttention(tag);
    pushToast({ tone: 'ok', title: 'Focused in Chat', body: tag, cause: 'clipboard' });
  }, [pushToast]);

  useEffect(() => {
    const bridgeShellMode: ShellMode = shellMode === 'nu' || shellMode === 'vox' ? shellMode : 'default';
    publishTerminalBridgeState({ activeSessionId, cwd: activeCwd ?? null, blocks, pinnedBlockIds, securityLevel, controlMode, shellMode: bridgeShellMode });
  }, [activeSessionId, activeCwd, blocks, pinnedBlockIds, securityLevel, controlMode, shellMode]);

  useEffect(() => onTerminalCommand((cmd) => submit(cmd)), [submit]);

  useEffect(() => {
    const applyFocusTarget = (target: ConsoleFocusTarget) => {
      setOpenAgentId(null);
      if (target.sessionId) {
        setActiveSessionId(target.sessionId);
        const existing = findPaneLeafBySession(splitRootRef.current, target.sessionId);
        if (existing) setActivePaneId(existing.paneId);
        else setSplitRoot((prev) => assignPaneSession(prev, activePaneRef.current, target.sessionId));
      }
      if (target.blockId !== undefined) setFocusedBlockId(target.blockId);
    };
    const pendingTarget = consumePendingConsoleBlockFocus();
    if (pendingTarget) applyFocusTarget(pendingTarget);
    return onConsoleBlockFocus(applyFocusTarget);
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const isInteractive = isInteractiveShortcutTarget(e.target);
      const insideConsole = e.target instanceof Node && Boolean(consoleRootRef.current?.contains(e.target));
      const target = blocks.find((b) => b.id === focusedBlockId) ?? latestBlock ?? blocks[blocks.length - 1];
      if (e.key === 'Escape' && controlModeRef.current === 'autopilot') {
        preemptAutopilotFromConsole();
      } else if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'f') {
        const activeEl = document.activeElement;
        if (!activeEl || activeEl === document.body || consoleRootRef.current?.contains(activeEl)) {
          e.preventDefault(); setSearchOpenSeq((n) => n + 1);
        }
      } else if (e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'r') {
        const activeEl = document.activeElement;
        if (!activeEl || activeEl === document.body || consoleRootRef.current?.contains(activeEl)) {
          e.preventDefault(); openHistorySearch();
        }
      } else if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'm') {
        e.preventDefault(); cycleControlMode();
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'j') {
        e.preventDefault();
        (document.getElementById('loquela-composer') ?? document.querySelector<HTMLElement>('[aria-label="console input"]'))?.focus();
      } else if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown') && blocks.length > 0) {
        e.preventDefault();
        setFocusedBlockId((prev) => {
          if (prev === null) return e.key === 'ArrowUp' ? blocks[blocks.length - 1]!.id : blocks[0]!.id;
          const idx = blocks.findIndex((b) => b.id === prev);
          if (e.key === 'ArrowUp') return idx > 0 ? blocks[idx - 1]!.id : blocks[0]!.id;
          return idx >= 0 && idx < blocks.length - 1 ? blocks[idx + 1]!.id : blocks[blocks.length - 1]!.id;
        });
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'l') {
        e.preventDefault(); if (target) handleFocusInChat(target);
      } else if (!isInteractive && !e.metaKey && !e.ctrlKey && !e.altKey && insideConsole && target) {
        const k = e.key.toLowerCase();
        if (k === 'p') {
          e.preventDefault();
          const nextPinned = !pinnedBlockIds.includes(target.id);
          const nextPins = nextPinned ? [...pinnedBlockIds, target.id] : pinnedBlockIds.filter((id) => id !== target.id);
          patchSessionUi(activeSessionRef.current, { pinnedIds: nextPins });
          if (activeSessionRef.current && typeof termSessionPin === 'function') termSessionPin(activeSessionRef.current, target.id, nextPinned).catch(() => {});
        } else if (k === 'f') { e.preventDefault(); handleFocusInChat(target); }
        else if (k === 'a') { e.preventDefault(); setLatestBlock(target); setComposing(true); }
        else if (k === 'r') { e.preventDefault(); submit(target.input); }
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [blocks, focusedBlockId, latestBlock, pinnedBlockIds, cycleControlMode, handleFocusInChat, openHistorySearch, patchSessionUi, preemptAutopilotFromConsole, submit]);

  useEffect(() => {
    (async () => {
      try {
        const list = await termSessionList();
        const hasList = Array.isArray(list) && list.length > 0;
        const sid = hasList ? list[0]!.id : await termSessionOpen(80, 24);
        const resolvedList = hasList ? list : [{ id: sid, blocks: 0 }];
        setSessions(resolvedList);
        setSessionUiById((prev) => {
          const next = { ...prev };
          for (const item of resolvedList) {
            const cur = next[item.id] ?? DEFAULT_SESSION_UI;
            next[item.id] = {
              ...cur,
              securityLevel: item.security_level ? fromWireSecurityLevel(item.security_level) : cur.securityLevel,
              controlMode: item.control_mode ? fromWireControlMode(item.control_mode) : cur.controlMode,
            };
          }
          return next;
        });
        setActiveSessionId((prev) => prev ?? sid);
        setSplitRoot((prev) => (prev.kind === 'leaf' && prev.sessionId === null ? { ...prev, sessionId: sid } : prev));
      } catch { /* offline/mock fallback */ }
    })();
  }, []);

  const handleSelectSession = (sid: string) => {
    setOpenAgentId(null);
    setFocusedBlockId(null);
    setActiveSessionId(sid);
    const existing = findPaneLeafBySession(splitRootRef.current, sid);
    if (existing) setActivePaneId(existing.paneId);
    else setSplitRoot((prev) => assignPaneSession(prev, activePaneRef.current, sid));
  };
  const handleNewSession = async () => {
    try {
      const sid = await termSessionOpen(80, 24);
      setOpenAgentId(null); setFocusedBlockId(null); setActiveSessionId(sid);
      patchSessionUi(sid, DEFAULT_SESSION_UI);
      setSessions((prev) => (prev.some((s) => s.id === sid) ? prev : [...prev, { id: sid, blocks: 0 }]));
      setSplitRoot((prev) => assignPaneSession(prev, activePaneRef.current, sid));
    } catch (e) { pushToast({ tone: 'warn', title: 'New session failed', body: sanitizeErrorForToast(e), cause: 'backend-error' }); }
  };
  const handleSplit = async (orientation: 'horizontal' | 'vertical') => {
    if (splittingRef.current || countPaneLeaves(splitRootRef.current) >= MAX_SPLIT_PANES) return;
    splittingRef.current = true;
    const targetPane = activePaneRef.current;
    try {
      const sid = await termSessionOpen(80, 24);
      if (countPaneLeaves(splitRootRef.current) >= MAX_SPLIT_PANES) {
        if (typeof termSessionClose === 'function') termSessionClose(sid).catch(() => {});
        return;
      }
      const newPaneId = `pane-${(paneSeqRef.current += 1)}`;
      setOpenAgentId(null); setFocusedBlockId(null); setActivePaneId(newPaneId); setActiveSessionId(sid);
      patchSessionUi(sid, DEFAULT_SESSION_UI);
      setSessions((prev) => (prev.some((s) => s.id === sid) ? prev : [...prev, { id: sid, blocks: 0 }]));
      setSplitRoot((prev) => splitPaneLeaf(prev, targetPane, orientation, newPaneId, sid));
    } catch (e) {
      pushToast({ tone: 'warn', title: 'Split pane failed', body: sanitizeErrorForToast(e), cause: 'backend-error' });
    } finally {
      splittingRef.current = false;
    }
  };
  const handleClosePane = (paneId: string) => {
    const nextRoot = closePaneLeaf(splitRootRef.current, paneId);
    setSplitRoot(nextRoot);
    if (activePaneRef.current === paneId || !findPaneLeaf(nextRoot, activePaneRef.current)) {
      const first = findFirstLeaf(nextRoot);
      setActivePaneId(first.paneId);
      setFocusedBlockId(null);
      if (first.sessionId) setActiveSessionId(first.sessionId);
    }
  };

  const handleCloseSession = async (sid: string) => {
    try {
      await termSessionClose(sid);
      const remaining = sessionsRef.current.filter((s) => s.id !== sid);
      const nextSid = remaining.length > 0 ? remaining[0]!.id : await termSessionOpen(80, 24);
      setRunningBySession((prev) => {
        if (!(sid in prev)) return prev;
        const copy = { ...prev }; delete copy[sid]; return copy;
      });
      setSessionUiById((prev) => {
        const copy = { ...prev }; delete copy[sid];
        if (!copy[nextSid]) copy[nextSid] = { ...DEFAULT_SESSION_UI };
        return copy;
      });
      setSessions(remaining.length > 0 ? remaining : [{ id: nextSid, blocks: 0 }]);
      setSplitRoot((prev) => replaceSessionInTree(prev, sid, nextSid));
      if (activeSessionRef.current === sid || remaining.length === 0) {
        setFocusedBlockId(null);
        setActiveSessionId(nextSid);
      }
    } catch (e) {
      pushToast({ tone: 'warn', title: 'Close session failed', body: sanitizeErrorForToast(e), cause: 'backend-error' });
    }
  };

  const handleSessionMeta = useCallback((meta: SessionMetaSnapshot) => {
    setRunningBySession((prev) => (prev[meta.sessionId] === meta.runningCommand ? prev : { ...prev, [meta.sessionId]: meta.runningCommand }));
    setSessions((prev) => prev.map((s) => (s.id !== meta.sessionId ? s : {
      ...s, blocks: meta.blocksCount >= 0 ? meta.blocksCount : s.blocks,
      cwd: meta.cwd ?? s.cwd, git_branch: meta.gitBranch ?? s.git_branch, alt_screen: meta.altScreen,
    })));
  }, []);

  const handleBlock = useCallback((block: TermBlockDto) => {
    setLatestBlock(block);
    if (block.exit_code === 0) recordGamifyGuiEvent('console_command_success', { command: block.input }, { enabled: gamifyEnabled });
  }, [gamifyEnabled]);

  const copyTextWithToast = async (text: string, label: string) => {
    const writeText = navigator.clipboard?.writeText?.bind(navigator.clipboard);
    if (!writeText) { pushToast({ tone: 'warn', title: 'Copy failed', body: 'clipboard unavailable', cause: 'backend-error' }); return; }
    try {
      await writeText(text);
      pushToast({ tone: 'ok', title: 'Copied', body: label, cause: 'clipboard' });
    } catch (e) {
      pushToast({ tone: 'warn', title: 'Copy failed', body: sanitizeErrorForToast(e), cause: 'backend-error' });
    }
  };

  const blockBody = renderTermBlockForAgent(latestBlock, lastLine);
  const controlTiers: readonly TermControlModeId[] = ['manual', 'co_pilot', 'autopilot'];
  const controlTierRatio = (Math.max(0, controlTiers.indexOf(controlMode === 'copilot' ? 'co_pilot' : controlMode)) + 1) / controlTiers.length;
  const leafCount = countPaneLeaves(splitRoot);

  const consoleGauges = [
    { label: 'Sessions', valueText: `${sessions.length} Active`, ratio: Math.max(0, Math.min(1, sessions.length / 5)), tone: 'gold' as const, purpose: 'Active terminal sessions attached to daemon', actionHint: 'Focus active terminal session', active: openAgentId === null, onClick: () => { setOpenAgentId(null); if (sessions.length > 1 && activeSessionId) handleSelectSession(sessions[(sessions.findIndex((s) => s.id === activeSessionId) + 1) % sessions.length]!.id); } },
    { label: 'Blocks', valueText: `${blocks.length} Items`, ratio: Math.max(0, Math.min(1, blocks.length / 40)), tone: 'gold' as const, purpose: 'Captured terminal block stream history', actionHint: 'Focus latest terminal block', active: focusedBlockId !== null, onClick: () => { setOpenAgentId(null); if (blocks.length > 0) setFocusedBlockId((prev) => (prev !== null ? null : blocks[blocks.length - 1]!.id)); } },
    { label: 'Control', valueText: controlMode === 'autopilot' ? 'Autopilot' : controlMode === 'co_pilot' || controlMode === 'copilot' ? 'Co-Pilot' : 'Manual', ratio: controlTierRatio, tone: controlMode === 'autopilot' ? ('emerald' as const) : ('gold' as const), purpose: 'Terminal control bridge automation tier', actionHint: 'Cycle Manual / Co-Pilot / Autopilot mode', active: controlMode !== 'manual', onClick: cycleControlMode },
  ];

  return (
    <div ref={consoleRootRef} className="flex h-full min-h-0 flex-col gap-1 min-[1100px]:gap-2 rounded-xl border border-brass/40 bg-bg-surface p-2 min-[1100px]:p-3" data-testid="console-root">
      <RomanSurfaceHeader domainTag="WORKSPACE - CONSOLE" title="Warp Console & Terminal Bridge" subtitle="Interactive Warp-style block stream, multi-agent terminal tabs, and capability execution." waxSealBadge="CONSOLE READY" icon={<Icon.terminal className="size-5 text-text-primary" />} gauges={consoleGauges} className="max-[1099px]:py-1" />
      <div className="flex shrink-0 items-center justify-between gap-2">
        <div className="min-w-0 flex-1">
          <AgentStrip agents={agents} onOpen={(id) => setOpenAgentId(id)} />
          {orchError ? <div role="alert" aria-live="polite" className="px-2 pb-1 text-[11px] text-(--color-status-warn)">{orchError}</div> : null}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button size="xs" variant="outline" onClick={() => void copyTextWithToast(`vox term --attach ${activeSessionId ?? 's-1'}`, `vox term --attach ${activeSessionId ?? 's-1'}`)} className="h-6 shrink-0 whitespace-nowrap px-2 py-1 text-[11px] font-semibold">continue in vox term</Button>
          <Button size="xs" variant="outline" onClick={() => void copyTextWithToast(blockBody, 'last block to clipboard')} className="h-6 shrink-0 whitespace-nowrap px-2 py-1 text-[11px] font-semibold">copy last block</Button>
          <Button size="xs" variant="outline" disabled={agents.length === 0} onClick={() => setComposing(true)} className="h-6 shrink-0 whitespace-nowrap px-2 py-1 text-[11px] font-semibold">send to agent</Button>
        </div>
      </div>
      <ConsoleSessionBar
        sessions={sessions} activeSessionId={activeSessionId} runningCommandBySession={runningBySession} leafCount={leafCount}
        securityLevel={securityLevel} controlMode={controlMode} onSelectSession={handleSelectSession}
        onNewSession={() => void handleNewSession()} onCloseSession={(sid) => void handleCloseSession(sid)}
        onSplitRight={() => void handleSplit('horizontal')} onSplitDown={() => void handleSplit('vertical')}
        onSecurityLevelChange={handleSecurityLevelChange} onCycleControlMode={cycleControlMode}
        onVoxifySubshell={(sid) => void handleVoxifySubshell(sid)} voxifiedSessionIds={voxifiedSessionIds}
      />
      {composing ? (
        <SendToAgent initialBody={blockBody} agents={agents} onClose={() => setComposing(false)} onSend={(agentId, body) => {
          setComposing(false);
          sendToAgent(agentId, body)
            .then(() => pushToast({ tone: 'ok', title: 'Sent', body: `to agent ${agentId}`, cause: 'backend-ok' }))
            .catch((e) => pushToast({ tone: 'warn', title: 'Send failed', body: sanitizeErrorForToast(e), cause: 'backend-error' }));
        }} />
      ) : null}
      <div className="relative flex min-h-0 min-w-0 flex-1 gap-2">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-1 min-[1100px]:gap-2">
          {openAgentId ? (
            <div className="flex shrink-0 items-center gap-2 border-b border-border-subtle px-2 py-1 text-[11px] font-semibold">
              <span>{`Agent ${openAgentId}`}</span>
              <Button size="xs" variant="ghost" onClick={() => setOpenAgentId(null)} className="h-6 px-2 py-1 text-[11px] font-semibold">Return to Terminal</Button>
            </div>
          ) : null}
          <div className={`min-h-0 flex-1 ${openAgentId ? 'block' : 'hidden'}`}>{openAgentId ? <AgentTab agentId={openAgentId} /> : null}</div>
          <div className={`min-h-16 flex-1 flex-col overflow-hidden ${openAgentId ? 'hidden' : 'flex'}`}>
            <ConsoleSplitGrid
              root={splitRoot} activePaneId={activePaneId} onClosePane={handleClosePane}
              onSelectPane={(paneId) => { setActivePaneId(paneId); const leaf = findPaneLeaf(splitRootRef.current, paneId); if (leaf?.sessionId && leaf.sessionId !== activeSessionRef.current) { setFocusedBlockId(null); setActiveSessionId(leaf.sessionId); } }}
              renderLeaf={(leaf, { isActive, onClosePane: closeLeaf }) => {
                const leafSid = leaf.sessionId;
                const leafUi = (leafSid && sessionUiById[leafSid]) ?? fallbackSessionUi;
                return (
                  <TerminalSessionPane
                    paneId={leaf.paneId} sessionId={leafSid} autoOpenSession={false} activePane={isActive}
                    controlMode={leafUi.controlMode} shellMode={isActive ? shellMode : 'default'}
                    externalPreemptSeq={isActive ? externalPreemptSeq : undefined}
                    isVoxified={Boolean(leafSid && voxifiedSessionIds.has(leafSid))}
                    onVoxifySubshell={(sid) => void handleVoxifySubshell(sid)}
                    pendingLine={isActive ? pending : null} focusedBlockId={isActive ? focusedBlockId : null}
                    pinnedBlockIds={leafUi.pinnedIds} searchOpenRequestSeq={isActive ? searchOpenSeq : undefined}
                    onBlocksChange={(nextBlocks) => patchSessionUi(leafSid, { blocks: nextBlocks })}
                    onBlock={isActive ? handleBlock : undefined}
                    onPinnedBlocksChange={(nextPins) => patchSessionUi(leafSid, { pinnedIds: nextPins })}
                    onSelectBlock={isActive ? setFocusedBlockId : undefined}
                    onCopyCommand={(b) => void copyTextWithToast(b.input, 'command to clipboard')}
                    onCopyOutput={(b) => void copyTextWithToast(b.output.map((c) => c.text).join(''), 'output to clipboard')}
                    onRerun={(b) => submit(b.input)} onAskAgent={(b) => { setLatestBlock(b); setComposing(true); }}
                    onFocusInChat={handleFocusInChat} onSessionMetaChange={handleSessionMeta}
                    onSecuritySyncFromBackend={(lvl) => patchSessionUi(leafSid, { securityLevel: lvl })}
                    onControlSyncFromBackend={(mode) => patchSessionUi(leafSid, { controlMode: mode })}
                    onInsertInPrompt={(cmd) => { setApplyLine(cmd); document.querySelector<HTMLElement>('[aria-label="console input"]')?.focus(); }}
                    onClosePane={closeLeaf} pushToast={pushToast}
                  />
                );
              }}
            />
          </div>
          <div
            className="shrink-0 border-t border-border-subtle px-2 py-1 min-[1100px]:py-2"
            onChangeCapture={preemptAutopilotFromConsole}
            onKeyDownCapture={(e) => { if (!e.metaKey && !e.ctrlKey && !e.altKey && e.key.length === 1) preemptAutopilotFromConsole(); }}
          >
            <InputEditor
              onSubmit={submit} onActiveSuggestion={setActiveAction} applyLine={applyLine} onApplyLineConsumed={() => setApplyLine(null)}
              shellMode={shellMode} onShellModeChange={setShellMode} sessionId={activeSessionId} cwd={activeCwd} historyOpenRequestSeq={historyOpenRequestSeq}
            />
          </div>
        </div>
        <DiscoveryRail
          actionId={activeAction} nowMs={Date.now()} gamifyEnabled={gamifyEnabled} activeTab={railTab} onTabChange={setRailTab}
          selectedRunbookId={selectedRunbookId} cwd={activeCwd} onExecuteRunbook={submit}
          onInsertRunbook={(cmd) => { setApplyLine(cmd); document.querySelector<HTMLElement>('[aria-label="console input"]')?.focus(); }}
          onUseAction={(example) => setApplyLine(example)}
        />
      </div>
    </div>
  );
}
