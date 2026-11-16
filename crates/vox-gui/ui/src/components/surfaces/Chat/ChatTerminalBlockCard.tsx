import React, { useEffect, useState } from 'react';
import {
  extractApprovalResolutionError,
  type ResolveApprovalDecision,
  type ResolveApprovalResult,
} from '../../../hooks/useTerminalPendingApprovals';
import type { PendingApprovalRow } from '../../../lib/mcpToolResult';
import {
  getTerminalBridgeState, requestConsoleBlockFocus, subscribeTerminalBridge, type TerminalBridgeState,
} from '../../../lib/terminalBridge';
import * as transport from '../../../transport';
import type { TermBlockDto } from '../../../transport';
import type { TurnEventDto } from '../../../types/dashboard';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { TerminalApprovalCard } from '../Console/TerminalApprovalCard';
import { sgrToSpans } from '../Console/sgrToSpans';
import { TERMINAL_MONO_STYLE, useDocumentThemeAttr } from '../Console/terminalTheme';

export function isTerminalToolName(tool: unknown): tool is string {
  return typeof tool === 'string' && tool.startsWith('vox_terminal_');
}

export function isTerminalBlockToolName(tool: unknown): tool is string {
  return tool === 'vox_terminal_exec' || tool === 'vox_terminal_send_input';
}

export function isAgentTerminalBlock(block: TermBlockDto | null | undefined): block is TermBlockDto {
  if (!block || !block.origin) return false;
  return typeof block.origin === 'string' ? block.origin === 'agent' : block.origin.kind === 'agent';
}

export function findActiveAgentBlock(blocks: readonly TermBlockDto[]): TermBlockDto | null {
  for (let i = blocks.length - 1; i >= 0; i -= 1) {
    const b = blocks[i];
    if (b && typeof b.origin === 'object' && b.origin !== null && b.origin.kind === 'agent' && (b.status === 'Running' || b.status === 'AwaitingInput')) {
      return b;
    }
  }
  return null;
}

export function openTerminalInConsole(opts: {
  sessionId: string | null; blockId: number | null;
  onNavigate?: (viewKey: string) => void;
  onOpenConsole?: (viewKey: string, blockId?: number | null) => void;
}): void {
  const { sessionId, blockId, onNavigate, onOpenConsole } = opts;
  requestConsoleBlockFocus({ sessionId, blockId });
  onNavigate?.('console');
  onOpenConsole?.('console', blockId);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('vox://navigate-surface', { detail: { view: 'console', sessionId, blockId } }));
  }
}

function parseBlockId(raw: unknown): number | null {
  if (typeof raw === 'number' && Number.isFinite(raw)) return Math.trunc(raw);
  if (typeof raw === 'string' && /^\d+$/.test(raw.trim())) return Number(raw.trim());
  return null;
}

function extractOutputText(block: TermBlockDto | undefined, fallback: unknown): string {
  if (block) {
    const out = block.output as unknown;
    if (typeof out === 'string') return out;
    if (Array.isArray(out)) return out.map((c: { text?: string }) => c.text ?? '').join('');
  }
  return typeof fallback === 'string' ? fallback : '';
}

function resolveStatusSeal(block: TermBlockDto | undefined, event: TurnEventDto | undefined) {
  if (block) {
    if (block.status === 'Running') return { label: 'running', tone: 'text-text-secondary', running: true, failed: false, cancelled: false };
    if (block.status === 'AwaitingInput') return { label: 'awaiting input', tone: 'text-(--color-status-warn)', running: false, failed: false, cancelled: false };
    if (block.status === 'Cancelled') return { label: 'cancelled', tone: 'text-text-muted', running: false, failed: false, cancelled: true };
    if (typeof block.exit_code === 'number') {
      const failed = block.exit_code !== 0 || block.status === 'Failed';
      return { label: `exit ${block.exit_code}`, tone: failed ? 'text-(--color-status-fail)' : 'text-(--color-status-pass)', running: false, failed, cancelled: false };
    }
    if (block.status === 'Failed') return { label: 'exit 1', tone: 'text-(--color-status-fail)', running: false, failed: true, cancelled: false };
    return { label: 'exit 0', tone: 'text-(--color-status-pass)', running: false, failed: false, cancelled: false };
  }
  if (typeof event?.exit_code === 'number') {
    const failed = event.exit_code !== 0;
    return { label: `exit ${event.exit_code}`, tone: failed ? 'text-(--color-status-fail)' : 'text-(--color-status-pass)', running: false, failed, cancelled: false };
  }
  if (event?.fulfilled === false) {
    return { label: 'failed', tone: 'text-(--color-status-fail)', running: false, failed: true, cancelled: false };
  }
  return { label: 'completed', tone: 'text-(--color-status-pass)', running: false, failed: false, cancelled: false };
}

export interface ChatTerminalBlockCardProps {
  event?: TurnEventDto;
  liveBlock?: TermBlockDto;
  pendingApprovals?: PendingApprovalRow[];
  resolvingApprovalId?: string | null;
  onResolveApproval?: (
    approval: PendingApprovalRow,
    decision: ResolveApprovalDecision,
  ) => Promise<ResolveApprovalResult | void> | void;
  onNavigate?: (viewKey: string) => void;
  onOpenConsole?: (viewKey: string, blockId?: number | null) => void;
}

export function ChatTerminalBlockCard({
  event,
  liveBlock,
  pendingApprovals,
  resolvingApprovalId = null,
  onResolveApproval,
  onNavigate,
  onOpenConsole,
}: ChatTerminalBlockCardProps) {
  useDocumentThemeAttr();
  const [bridgeState, setBridgeState] = useState<TerminalBridgeState>(() => getTerminalBridgeState());
  useEffect(() => subscribeTerminalBridge(setBridgeState), []);

  const eventBlockId = parseBlockId(event?.block_id);
  const correlatedBlock: TermBlockDto | undefined = liveBlock
    ? (bridgeState.blocks.find((b) => b.id === liveBlock.id) ?? liveBlock)
    : eventBlockId !== null
      ? bridgeState.blocks.find((b) => b.id === eventBlockId)
      : undefined;

  const sessionId = (typeof event?.session_id === 'string' && event.session_id.length > 0 ? event.session_id : bridgeState.activeSessionId) ?? null;
  const blockId: number | null = correlatedBlock?.id ?? eventBlockId ?? null;
  const eventCommand =
    typeof event?.command === 'string' && event.command.trim().length > 0 ? event.command.trim()
      : typeof event?.input === 'string' && event.input.trim().length > 0 ? event.input.trim() : null;
  const command = correlatedBlock?.input?.trim() || eventCommand || (typeof event?.tool === 'string' && event.tool.length > 0 ? event.tool : 'vox_terminal_exec');
  const outputText = extractOutputText(correlatedBlock, event?.output);
  const seal = resolveStatusSeal(correlatedBlock, event);

  const eventApprovalId = typeof event?.approval_id === 'string' ? event.approval_id : null;
  const sessionApprovals = (pendingApprovals ?? []).filter(
    (a) => !a.session_id || !sessionId || a.session_id === sessionId,
  );
  const isActiveCard = correlatedBlock
    ? correlatedBlock.status === 'Running' || correlatedBlock.status === 'AwaitingInput'
    : event?.fulfilled === false && typeof event?.exit_code !== 'number';
  const hasFallbackToolCmd =
    command === 'vox_terminal_exec' || command === 'vox_terminal_send_input';
  const matchingApproval =
    (eventApprovalId
      ? (pendingApprovals ?? []).find((a) => a.approval_id === eventApprovalId)
      : undefined) ??
    sessionApprovals.find((a) => {
      if (a.command?.trim() && a.command.trim() === command) return true;
      if (!isActiveCard) return false;
      if (!a.command?.trim() || hasFallbackToolCmd) return sessionApprovals.length === 1;
      return false;
    });

  const handleResolveInline = async (
    approval: PendingApprovalRow,
    decision: ResolveApprovalDecision,
  ): Promise<ResolveApprovalResult | void> => {
    if (onResolveApproval) return onResolveApproval(approval, decision);
    try {
      const fn = transport.resolveTerminalApproval;
      if (typeof fn !== 'function') {
        return { ok: false, error: 'Approval transport unavailable.' };
      }
      const res = await fn({ approvalId: approval.approval_id, ...decision });
      const err = extractApprovalResolutionError(res, 'Could not resolve terminal approval.');
      return err ? { ok: false, error: err } : { ok: true };
    } catch {
      return { ok: false, error: 'Could not resolve terminal approval.' };
    }
  };

  const handlePromoteBg = () => {
    if (sessionId && blockId !== null && typeof transport.termSessionPromoteBg === 'function') {
      void transport.termSessionPromoteBg(sessionId, blockId).catch(() => {});
    }
  };
  const handleOpenConsole = () => openTerminalInConsole({ sessionId, blockId, onNavigate, onOpenConsole });

  return (
    <div data-testid="chat-term-block-card" className="w-full max-w-[680px] overflow-hidden rounded-lg border border-border-subtle bg-bg-surface text-xs">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle bg-bg-elevated px-3 py-2">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <Icon.terminal className="size-4 shrink-0 text-accent" aria-hidden="true" />
          <code data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="truncate text-sm leading-[1.3] font-semibold text-text-primary">{`$ ${command}`}</code>
          <span className="rounded border border-border-subtle bg-bg-surface px-2 py-1 text-[11px] leading-[1.25] font-semibold text-text-secondary">
            {`term:${sessionId ?? 'ready'}`}
          </span>
        </div>
        <span className={`inline-flex items-center gap-1 text-[11px] leading-[1.25] font-semibold ${seal.tone}`}>
          {seal.running ? <span className="size-2 animate-pulse rounded-full bg-(--color-status-info)" aria-hidden="true" /> : null}
          {seal.cancelled ? <Icon.ban className="size-3" aria-hidden="true" /> : null}
          {!seal.running && !seal.cancelled && correlatedBlock?.status !== 'AwaitingInput' ? (
            seal.failed ? <Icon.alert className="size-3" aria-hidden="true" /> : <Icon.check className="size-3" aria-hidden="true" />
          ) : null}
          <span>{seal.label}</span>
        </span>
      </div>

      {outputText.length > 0 ? (
        <pre
          data-testid="chat-term-block-output"
          data-terminal-well="true"
          data-terminal-mono="true"
          style={TERMINAL_MONO_STYLE}
          className="max-h-40 overflow-y-auto whitespace-pre-wrap break-words rounded border border-(--terminal-well-border) bg-(--terminal-well-bg) px-3 py-2 text-xs leading-[1.45] text-(--terminal-well-fg)"
        >
          {sgrToSpans(outputText)}
        </pre>
      ) : null}

      {matchingApproval ? (
        <div className="border-t border-border-subtle bg-bg-surface p-2">
          <TerminalApprovalCard
            key={matchingApproval.approval_id}
            compact
            approval={matchingApproval}
            resolving={resolvingApprovalId === matchingApproval.approval_id}
            onResolve={handleResolveInline}
          />
        </div>
      ) : null}

      {correlatedBlock?.status === 'AwaitingInput' ? (
        <div data-testid="chat-term-awaiting-input" className="flex flex-wrap items-center justify-between gap-2 border-t border-border-subtle bg-bg-elevated px-3 py-2 text-xs">
          <span className="inline-flex items-center gap-1 text-[11px] leading-[1.25] font-semibold text-(--color-status-warn)">
            <Icon.terminal className="size-4 shrink-0" aria-hidden="true" />
            <span>Process awaiting input</span>
          </span>
          <Button size="xs" variant="outline" data-testid="chat-term-promote-bg" onClick={handlePromoteBg} className="h-6 px-2 py-1 text-[11px] font-semibold">
            Promote to Background
          </Button>
        </div>
      ) : null}

      <div className="flex items-center justify-end gap-2 border-t border-border-subtle bg-bg-surface px-3 py-2">
        <Button size="xs" variant="ghost" data-testid="chat-term-open-console" onClick={handleOpenConsole} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold text-accent">
          <Icon.externalLink className="size-3 shrink-0" aria-hidden="true" />
          <span>Open in Console</span>
        </Button>
      </div>
    </div>
  );
}
