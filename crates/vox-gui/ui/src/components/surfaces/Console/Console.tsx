import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { InputEditor, type ShellMode } from './InputEditor';
import { DiscoveryRail } from './DiscoveryRail';
import { TerminalTab, type PendingLine } from './TerminalTab';
import { AgentStrip, type AgentChip } from './AgentStrip';
import { AgentTab } from './AgentTab';
import { SendToAgent } from './SendToAgent';
import { BlockStreamView } from './BlockStreamView';
import { renderTermBlockForAgent } from './webglAddon';
import {
  sendToAgent,
  termSessionList,
  termSessionOpen,
  type TermBlockDto,
  type TermSessionInfoDto,
} from '../../../transport';
import {
  onTerminalCommand,
  publishTerminalBridgeState,
  requestChatAttention,
  type TermControlModeId,
  type TermSecurityLevelId,
} from '../../../lib/terminalBridge';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { Button } from '../../ui/Button';
import {
  orchestratorStatusErrorMessage,
  useOrchestratorStatus,
} from '../../../hooks/useOrchestratorStatus';
import type { OrchestratorStatus, RawAgentSummary, Toast } from '../../../types/tauri';

function agentsFromStatus(status: OrchestratorStatus | undefined): AgentChip[] {
  return (status?.agents ?? []).map((a: RawAgentSummary) => ({
    id: String(a.id ?? ''),
    name: String(a.name ?? a.codename ?? a.id ?? 'agent'),
    state: a.paused ? 'paused' : a.in_progress ? 'running' : 'queued',
  }));
}

interface Props {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
  /** When set (e.g. via the Dashboard "Open in Console" deep link), open this
   *  agent's live event tab on mount. */
  initialAgentId?: string | null;
}

/**
 * Vox Console surface (layout A): agent strip on top, session tab bar,
 * Warp-style block stream + terminal on the left, persistent discovery rail on
 * the right, owned input editor along the bottom.
 */
export function Console({ pushToast, gamifyEnabled = false, initialAgentId = null }: Props) {
  const orchQuery = useOrchestratorStatus();
  const agents = useMemo(() => agentsFromStatus(orchQuery.data), [orchQuery.data]);
  const orchError = orchestratorStatusErrorMessage(orchQuery);
  const [shellMode, setShellMode] = useState<ShellMode>('default');
  const [securityLevel, setSecurityLevel] = useState<TermSecurityLevelId>('l2_safe_exec');
  const [controlMode, setControlMode] = useState<TermControlModeId>('manual');
  const [pending, setPending] = useState<PendingLine | null>(null);
  const [activeAction, setActiveAction] = useState<string | null>(null);
  const [openAgentId, setOpenAgentId] = useState<string | null>(initialAgentId);
  const [composing, setComposing] = useState(false);
  const [lastLine, setLastLine] = useState('');
  const [sessions, setSessions] = useState<TermSessionInfoDto[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [blocks, setBlocks] = useState<TermBlockDto[]>([]);
  const [pinnedBlockIds, setPinnedBlockIds] = useState<number[]>([]);
  const [focusedBlockId, setFocusedBlockId] = useState<number | null>(null);
  const [latestBlock, setLatestBlock] = useState<TermBlockDto | null>(null);
  const [applyLine, setApplyLine] = useState<string | null>(null);
  const seq = React.useRef(0);
  const tabId = activeSessionId ?? 'console-1';
  const nowMs = Date.now();

  const handleFocusInChat = useCallback(
    (b: TermBlockDto, lineRange?: string) => {
      const sid = activeSessionId ?? 's-1';
      const tag = lineRange ? `@term:${sid}#${b.id}:${lineRange}` : `@term:${sid}#${b.id}`;
      requestChatAttention(tag);
      pushToast({
        tone: 'ok',
        title: 'Focused in Chat',
        body: tag,
        cause: 'clipboard',
      });
    },
    [activeSessionId, pushToast],
  );

  useEffect(() => {
    publishTerminalBridgeState({
      activeSessionId,
      blocks,
      pinnedBlockIds,
      securityLevel,
      controlMode,
      shellMode,
    });
  }, [activeSessionId, blocks, pinnedBlockIds, securityLevel, controlMode, shellMode]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const isTyping =
        e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;

      if (e.key === 'Escape' && controlMode === 'autopilot') {
        setControlMode('manual');
      } else if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'm') {
        e.preventDefault();
        cycleControlMode();
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'j') {
        e.preventDefault();
        const loq = document.getElementById('loquela-composer');
        if (loq) {
          loq.focus();
        } else {
          const inp = document.querySelector<HTMLInputElement>('input[aria-label="console input"]');
          inp?.focus();
        }
      } else if (e.altKey && e.key === 'ArrowUp') {
        e.preventDefault();
        if (blocks.length > 0) {
          setFocusedBlockId((prev) => {
            if (prev === null) return blocks[blocks.length - 1].id;
            const idx = blocks.findIndex((b) => b.id === prev);
            return idx > 0 ? blocks[idx - 1].id : blocks[0].id;
          });
        }
      } else if (e.altKey && e.key === 'ArrowDown') {
        e.preventDefault();
        if (blocks.length > 0) {
          setFocusedBlockId((prev) => {
            if (prev === null) return blocks[0].id;
            const idx = blocks.findIndex((b) => b.id === prev);
            return idx >= 0 && idx < blocks.length - 1 ? blocks[idx + 1].id : blocks[blocks.length - 1].id;
          });
        }
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'l') {
        e.preventDefault();
        const target =
          blocks.find((b) => b.id === focusedBlockId) ?? latestBlock ?? blocks[blocks.length - 1];
        if (target) {
          handleFocusInChat(target);
        }
      } else if (!isTyping) {
        const target =
          blocks.find((b) => b.id === focusedBlockId) ?? latestBlock ?? blocks[blocks.length - 1];
        if (!target) return;
        if (e.key.toLowerCase() === 'p') {
          e.preventDefault();
          setPinnedBlockIds((prev) =>
            prev.includes(target.id) ? prev.filter((id) => id !== target.id) : [...prev, target.id],
          );
        } else if (e.key.toLowerCase() === 'f') {
          e.preventDefault();
          handleFocusInChat(target);
        } else if (e.key.toLowerCase() === 'a') {
          e.preventDefault();
          setLatestBlock(target);
          setComposing(true);
        } else if (e.key.toLowerCase() === 'r') {
          e.preventDefault();
          submit(target.input);
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [latestBlock, blocks, controlMode, focusedBlockId, handleFocusInChat]);

  useEffect(() => {
    return onTerminalCommand((cmd) => {
      submit(cmd);
    });
  }, []);

  const refreshSessions = useCallback(async () => {
    try {
      const list = await termSessionList();
      if (Array.isArray(list) && list.length > 0) {
        setSessions(list);
        setActiveSessionId((prev) => prev ?? list[0].id);
      } else {
        const sid = await termSessionOpen(80, 24);
        setSessions([{ id: sid, blocks: 0 }]);
        setActiveSessionId((prev) => prev ?? sid);
      }
    } catch {
      /* ignore in offline/mock environments */
    }
  }, []);

  useEffect(() => {
    void refreshSessions();
  }, [refreshSessions]);

  const handleNewSession = async () => {
    try {
      const sid = await termSessionOpen(80, 24);
      setActiveSessionId(sid);
      setSessions((prev) =>
        prev.some((s) => s.id === sid) ? prev : [...prev, { id: sid, blocks: 0 }],
      );
    } catch (e) {
      pushToast({
        tone: 'warn',
        title: 'New session failed',
        body: sanitizeErrorForToast(e),
        cause: 'backend-error',
      });
    }
  };

  const submit = (line: string) => {
    seq.current += 1;
    setLastLine(line);
    setControlMode((prev) => (prev === 'autopilot' ? 'manual' : prev));
    setPending({ text: line, seq: seq.current });
  };

  const cycleControlMode = () => {
    setControlMode((prev) =>
      prev === 'manual' ? 'co_pilot' : prev === 'co_pilot' ? 'autopilot' : 'manual',
    );
  };

  const handleBlock = useCallback(
    (block: TermBlockDto) => {
      setLatestBlock(block);
      if (block.exit_code === 0) {
        recordGamifyGuiEvent(
          'console_command_success',
          { command: block.input },
          { enabled: gamifyEnabled },
        );
      }
    },
    [gamifyEnabled],
  );

  const openAgentTab = (agentId: string) => {
    setOpenAgentId(agentId);
  };

  const blockBody = renderTermBlockForAgent(latestBlock, lastLine);

  const copyTextWithToast = async (text: string, label: string) => {
    const writeText = navigator.clipboard?.writeText?.bind(navigator.clipboard);
    if (!writeText) {
      pushToast({ tone: 'warn', title: 'Copy failed', body: 'clipboard unavailable', cause: 'backend-error' });
      return;
    }
    try {
      await writeText(text);
      pushToast({ tone: 'ok', title: 'Copied', body: label, cause: 'clipboard' });
    } catch {
      pushToast({ tone: 'warn', title: 'Copy failed', cause: 'backend-error' });
    }
  };

  const copyLastBlock = async () => {
    await copyTextWithToast(blockBody, 'last block to clipboard');
  };

  const continueInVoxTerm = async () => {
    const sid = activeSessionId ?? 's-1';
    const cmd = `vox term --attach ${sid}`;
    await copyTextWithToast(cmd, cmd);
  };

  const handleSend = (agentId: string, body: string) => {
    setComposing(false);
    sendToAgent(agentId, body)
      .then(() => pushToast({ tone: 'ok', title: 'Sent', body: `to agent ${agentId}`, cause: 'backend-ok' }))
      .catch((e) => pushToast({ tone: 'warn', title: 'Send failed', body: sanitizeErrorForToast(e), cause: 'backend-error' }));
  };

  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="console-root">
      <div className="flex shrink-0 items-center justify-between">
        <div className="min-w-0 flex-1">
          <AgentStrip agents={agents} onOpen={openAgentTab} />
          {orchError && (
            <div role="alert" aria-live="polite" className="px-2.5 pb-1 text-[11px] text-amber-400">
              {orchError}
            </div>
          )}
        </div>
        <div className="mx-2.5 flex gap-1.5">
          <Button onClick={continueInVoxTerm} className="text-[11px]">
            continue in vox term
          </Button>
          <Button onClick={copyLastBlock} className="text-[11px]">
            copy last block
          </Button>
          <Button
            disabled={agents.length === 0}
            onClick={() => setComposing(true)}
            className="text-[11px]"
          >
            send to agent
          </Button>
        </div>
      </div>
      <div
        data-testid="term-session-bar"
        aria-label="terminal sessions"
        className="flex shrink-0 items-center gap-1.5 border-b border-white/10 px-2.5 py-1 text-[11px]"
      >
        {sessions.map((s) => (
          <Button
            key={s.id}
            data-testid={`term-session-tab-${s.id}`}
            onClick={() => setActiveSessionId(s.id)}
            className={`h-6 px-2 py-0.5 text-[11px] ${
              activeSessionId === s.id ? 'border border-brass/50 font-semibold' : ''
            }`}
          >
            {`${s.id} (${s.blocks})`}
          </Button>
        ))}
        <Button
          data-testid="term-new-session"
          onClick={() => void handleNewSession()}
          className="h-6 px-2 py-0.5 text-[11px]"
        >
          + new session
        </Button>
        <div className="ml-auto flex items-center gap-1.5 font-mono">
          <select
            data-testid="term-security-level-select"
            aria-label="Terminal security level"
            value={securityLevel}
            onChange={(e) => setSecurityLevel(e.target.value as TermSecurityLevelId)}
            className="h-6 rounded border border-white/15 bg-zinc-900 px-1.5 py-0.5 text-[10px] text-zinc-200 outline-none"
          >
            <option value="l1_read_only">L1 Read-Only</option>
            <option value="l2_safe_exec">L2 Safe Exec</option>
            <option value="l3_workspace_write">L3 Workspace Write</option>
            <option value="l4_system_exec">L4 System Exec</option>
            <option value="l5_autopilot_floor">L5 Autopilot Floor</option>
          </select>
          <button
            type="button"
            data-testid="term-control-mode-pill"
            onClick={cycleControlMode}
            className={`h-6 rounded border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider transition ${
              controlMode === 'autopilot'
                ? 'border-amber-400/50 bg-amber-500/20 text-amber-300'
                : controlMode === 'co_pilot'
                  ? 'border-cyan/50 bg-cyan/15 text-cyan'
                  : 'border-white/15 bg-white/5 text-zinc-300'
            }`}
          >
            {controlMode === 'autopilot'
              ? 'Autopilot'
              : controlMode === 'co_pilot'
                ? 'Co-Pilot'
                : 'Manual'}
          </button>
        </div>
      </div>
      {composing && (
        <SendToAgent
          initialBody={blockBody}
          agents={agents}
          onSend={handleSend}
          onClose={() => setComposing(false)}
        />
      )}
      <div className="flex min-h-0 flex-1">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          {openAgentId && (
            <div className="flex shrink-0 items-center gap-2 border-b border-white/10 px-2.5 py-1 text-[11px]">
              <span>agent {openAgentId}</span>
              <Button
                onClick={() => setOpenAgentId(null)}
                aria-label="Close agent view and return to terminal"
              >
                back to terminal
              </Button>
            </div>
          )}
          <div className={`min-h-0 flex-1 ${openAgentId ? 'block' : 'hidden'}`}>
            {openAgentId ? <AgentTab agentId={openAgentId} /> : null}
          </div>
          <div className={`flex min-h-0 flex-1 flex-col ${openAgentId ? 'hidden' : 'flex'}`}>
            <BlockStreamView
              blocks={blocks}
              pinnedBlockIds={pinnedBlockIds}
              onCopyCommand={(b) => void copyTextWithToast(b.input, 'command to clipboard')}
              onCopyOutput={(b) =>
                void copyTextWithToast(
                  b.output.map((c) => c.text).join(''),
                  'output to clipboard',
                )
              }
              onRerun={(b) => submit(b.input)}
              onPin={(b) =>
                setPinnedBlockIds((prev) =>
                  prev.includes(b.id) ? prev.filter((id) => id !== b.id) : [...prev, b.id],
                )
              }
              onAskAgent={(b) => {
                setLatestBlock(b);
                setComposing(true);
              }}
              focusedBlockId={focusedBlockId}
              onFocusInChat={handleFocusInChat}
              onSelectBlock={setFocusedBlockId}
            />
            <div className="min-h-0 flex-1">
              <TerminalTab
                tabId={tabId}
                sessionId={activeSessionId}
                pendingLine={pending}
                onSessionResolved={(sid) => {
                  setActiveSessionId(sid);
                  setSessions((prev) =>
                    prev.some((s) => s.id === sid) ? prev : [...prev, { id: sid, blocks: 0 }],
                  );
                }}
                onBlocksChange={(nextBlocks) => {
                  setBlocks(nextBlocks);
                  setSessions((prev) =>
                    prev.map((s) =>
                      s.id === activeSessionId ? { ...s, blocks: nextBlocks.length } : s,
                    ),
                  );
                }}
                onBlock={handleBlock}
              />
            </div>
          </div>
          <div className="shrink-0 border-t border-white/10 px-2.5 py-1.5">
            <InputEditor
              onSubmit={submit}
              onActiveSuggestion={setActiveAction}
              applyLine={applyLine}
              onApplyLineConsumed={() => setApplyLine(null)}
              shellMode={shellMode}
              onShellModeChange={setShellMode}
            />
          </div>
        </div>
        <DiscoveryRail
          actionId={activeAction}
          nowMs={nowMs}
          gamifyEnabled={gamifyEnabled}
          onUseAction={(example) => setApplyLine(example)}
        />
      </div>
    </div>
  );
}
