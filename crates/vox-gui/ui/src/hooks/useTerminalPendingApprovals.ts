import { useCallback, useEffect, useRef, useState } from 'react';
import { APPROVALS_POLL_MS } from '../config/constants';
import { sanitizeErrorForToast } from '../lib/backendGuard';
import { unwrapMcpEnvelope, type PendingApprovalRow } from '../lib/mcpToolResult';
import type { TermControlModeId } from '../lib/terminalBridge';
import * as transport from '../transport';
import type { ResolveTerminalApprovalInput, TermBlockDto } from '../transport';
import type { Toast } from '../types/tauri';

export type ResolveApprovalDecision = Omit<ResolveTerminalApprovalInput, 'approvalId'>;
export interface ResolveApprovalResult {
  ok: boolean;
  error?: string;
}

interface LocalCoPilotEntry {
  row: PendingApprovalRow;
  execute: (cmd: string) => Promise<void> | void;
  sourceBlockId?: number | null;
}

const MULTI_SUBCOMMAND_CLIS = new Set([
  'cargo', 'git', 'pnpm', 'npm', 'yarn', 'vox', 'docker', 'kubectl', 'rustup', 'gh', 'az', 'aws', 'uv',
]);
const UNSAFE_PATTERN_CHAR_RE = /[\\/|&;$`<>(){}*?!'"~\n\r=]/;
const COMPOUND_SHELL_RE = /(?:&&|\|\||[;|`<>()]|\$\(|\n)/;
const SLASH_VERB_RE = /^\/[a-zA-Z][\w-]*(?:\s|$)/;
const AGENT_SLASH_RE = /^\/(?:ai|ask)\s+/i;

function getTransportFn<K extends keyof typeof transport>(key: K): (typeof transport)[K] | undefined {
  try {
    const fn = transport[key];
    return typeof fn === 'function' ? fn : undefined;
  } catch {
    return undefined;
  }
}

function isSafePatternToken(token: string): boolean {
  if (!token || token.startsWith('-') || UNSAFE_PATTERN_CHAR_RE.test(token)) return false;
  return /^[A-Za-z0-9_.:+-]+$/.test(token);
}

/**
 * Derive a conservative token-boundary session allowlist prefix from a command.
 * Returns null if the head token is a path, flag, single character, or contains metacharacters.
 */
export function deriveSessionAllowPattern(command: string | null | undefined): string | null {
  const firstLine = (command ?? '').split('\n')[0]?.trim() ?? '';
  const stripped = firstLine.replace(/^(?:\$\s+|!+)/, '').trim();
  if (!stripped) return null;
  const rawTokens = stripped.split(/\s+/);
  let idx = 0;
  while (idx < rawTokens.length && /^[A-Za-z_][A-Za-z0-9_]*=/.test(rawTokens[idx]!)) idx += 1;
  const head = rawTokens[idx] ?? '';
  if (!isSafePatternToken(head)) return null;
  const sub = rawTokens[idx + 1] ?? '';
  if (MULTI_SUBCOMMAND_CLIS.has(head.toLowerCase()) && isSafePatternToken(sub)) {
    const combined = `${head} ${sub}`;
    return combined.length >= 2 ? combined : null;
  }
  return head.length >= 2 ? head : null;
}

export function matchesSessionPattern(command: string, patterns: readonly string[]): boolean {
  const trimmed = command.trim();
  if (!trimmed || COMPOUND_SHELL_RE.test(trimmed)) return false;
  return patterns.some((pat) => trimmed === pat || trimmed.startsWith(`${pat} `));
}

export function extractApprovalResolutionError(
  res: { is_error?: boolean; result?: unknown } | null | undefined,
  fallback: string,
): string | null {
  if (!res) return fallback;
  const raw = res.result as { error?: unknown; success?: unknown } | string | null | undefined;
  const envelope = unwrapMcpEnvelope(res.result) as { error?: unknown; resolved?: unknown } | null;
  const failed =
    Boolean(res.is_error) ||
    envelope?.resolved === false ||
    (raw !== null && typeof raw === 'object' && raw.success === false);
  if (!failed) return null;
  const errText =
    (typeof envelope?.error === 'string' && envelope.error.trim()) ||
    (raw !== null && typeof raw === 'object' && typeof raw.error === 'string' && raw.error.trim()) ||
    (typeof raw === 'string' && raw.trim()) ||
    fallback;
  return sanitizeErrorForToast(errText);
}

export interface UseTerminalPendingApprovalsOptions {
  sessionId?: string | null;
  controlMode?: TermControlModeId;
  enabled?: boolean;
  latestBlockId?: number | null;
  pushToast?: (toast: Toast) => void;
  onControlModeChange?: (mode: TermControlModeId) => void;
  onClearedSession?: () => void;
  onSlashMutatedBlocks?: () => Promise<void> | void;
}

export function useTerminalPendingApprovals({
  sessionId,
  controlMode = 'co_pilot',
  enabled = true,
  latestBlockId: _latestBlockId = null,
  pushToast,
  onControlModeChange,
  onClearedSession,
  onSlashMutatedBlocks,
}: UseTerminalPendingApprovalsOptions) {
  const [daemonApprovals, setDaemonApprovals] = useState<PendingApprovalRow[]>([]);
  const [localEntries, setLocalEntries] = useState<LocalCoPilotEntry[]>([]);
  const [resolvingId, setResolvingId] = useState<string | null>(null);
  const [sessionPatterns, setSessionPatterns] = useState<string[]>([]);
  const [preemptedBanner, setPreemptedBanner] = useState(false);
  const [diagnosingBlockId, setDiagnosingBlockId] = useState<number | null>(null);

  const inFlightPollRef = useRef(false);
  const localSeqRef = useRef(0);
  const sessionIdRef = useRef(sessionId);
  sessionIdRef.current = sessionId;

  const refreshApprovals = useCallback(async () => {
    const listFn = getTransportFn('listTerminalPendingApprovals');
    if (!enabled || !listFn || inFlightPollRef.current) return;
    inFlightPollRef.current = true;
    try {
      const rows = await listFn(sessionId ?? null);
      if (Array.isArray(rows)) setDaemonApprovals(rows);
    } catch {
      /* background poll ignores transient offline errors */
    } finally {
      inFlightPollRef.current = false;
    }
  }, [enabled, sessionId]);

  useEffect(() => {
    if (!enabled) {
      setDaemonApprovals([]);
      return;
    }
    let disposed = false;
    const poll = async () => {
      if (disposed) return;
      await refreshApprovals();
    };
    void poll();
    const id = window.setInterval(() => {
      void poll();
    }, APPROVALS_POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(id);
    };
  }, [enabled, refreshApprovals]);

  useEffect(() => {
    setLocalEntries([]);
    setSessionPatterns([]);
    setPreemptedBanner(false);
  }, [sessionId]);

  const recordCorrectionSignal = useCallback(
    async (
      blockId: number | null | undefined,
      correctedCommand: string,
      originalCommand?: string,
    ) => {
      const sid = sessionIdRef.current;
      const fbFn = getTransportFn('termBlockFeedback');
      const orig = originalCommand?.trim();
      const targetId = blockId ?? (orig ? 0 : null);
      if (!sid || targetId == null || !fbFn) return;
      await fbFn(sid, targetId, 'corrected', correctedCommand, orig || undefined).catch(() => {});
    },
    [],
  );

  const resolveApproval = useCallback(
    async (
      approval: PendingApprovalRow,
      decision: ResolveApprovalDecision,
    ): Promise<ResolveApprovalResult> => {
      setResolvingId(approval.approval_id);
      try {
        const original = (approval.command ?? approval.summary).trim();
        if (approval.approval_id.startsWith('local-copilot-')) {
          const entry = localEntries.find((item) => item.row.approval_id === approval.approval_id);
          setLocalEntries((prev) => prev.filter((item) => item.row.approval_id !== approval.approval_id));
          if (decision.outcome === 'approved' || decision.outcome === 'modified') {
            if (decision.allowPattern && !approval.floor_category && !approval.credential_prompt) {
              setSessionPatterns((prev) =>
                prev.includes(decision.allowPattern!) ? prev : [...prev, decision.allowPattern!],
              );
            }
            const nextCmd =
              decision.outcome === 'modified' && decision.modifiedCommand
                ? decision.modifiedCommand.trim()
                : original;
            if (decision.outcome === 'modified' && nextCmd && nextCmd !== original && !approval.credential_prompt) {
              await recordCorrectionSignal(entry?.sourceBlockId ?? 0, nextCmd, original);
            }
            if (entry && nextCmd) await Promise.resolve(entry.execute(nextCmd));
          }
          return { ok: true };
        }

        const resolveFn = getTransportFn('resolveTerminalApproval');
        if (!resolveFn) return { ok: false, error: 'Approval transport unavailable.' };

        const res = await resolveFn({ approvalId: approval.approval_id, ...decision });
        const errMsg = extractApprovalResolutionError(
          res,
          `Could not ${decision.outcome} approval ${approval.approval_id}.`,
        );
        if (errMsg) {
          if (decision.outcome !== 'modified') {
            pushToast?.({ tone: 'warn', title: 'Approval failed', body: errMsg, cause: 'backend-error' });
          }
          return { ok: false, error: errMsg };
        }

        setDaemonApprovals((prev) => prev.filter((item) => item.approval_id !== approval.approval_id));
        if (
          (decision.outcome === 'approved' || decision.outcome === 'modified') &&
          decision.allowPattern &&
          !approval.floor_category &&
          !approval.credential_prompt
        ) {
          setSessionPatterns((prev) =>
            prev.includes(decision.allowPattern!) ? prev : [...prev, decision.allowPattern!],
          );
        }
        if (
          decision.outcome === 'modified' &&
          decision.modifiedCommand &&
          decision.modifiedCommand.trim() !== original &&
          !approval.credential_prompt
        ) {
          await recordCorrectionSignal(0, decision.modifiedCommand.trim(), original);
        }
        void refreshApprovals();
        return { ok: true };
      } catch (err) {
        const msg = sanitizeErrorForToast(err);
        pushToast?.({ tone: 'warn', title: 'Approval failed', body: msg, cause: 'backend-error' });
        return { ok: false, error: msg };
      } finally {
        setResolvingId(null);
      }
    },
    [localEntries, pushToast, recordCorrectionSignal, refreshApprovals],
  );

  const stageOrRunProposedCommand = useCallback(
    async (
      command: string,
      executeFn: (cmd: string) => Promise<void> | void,
      sourceBlockId?: number | null,
    ) => {
      const trimmed = command.trim();
      if (!trimmed) return;
      const isCoPilot = controlMode === 'co_pilot' || controlMode === 'copilot';
      if (isCoPilot && !matchesSessionPattern(trimmed, sessionPatterns)) {
        const id = `local-copilot-${(localSeqRef.current += 1)}`;
        const row: PendingApprovalRow = {
          approval_id: id,
          tool: 'vox_terminal_exec',
          summary: 'Co-Pilot review - proposed agent command',
          requested_at_ms: Date.now(),
          session_id: sessionIdRef.current ?? 'active',
          command: trimmed,
          control_mode: 'co_pilot',
        };
        setLocalEntries((prev) => [...prev, { row, execute: executeFn, sourceBlockId }]);
        return;
      }
      await Promise.resolve(executeFn(trimmed));
    },
    [controlMode, sessionPatterns],
  );

  const dispatchTerminalLine = useCallback(
    async (
      sid: string,
      rawLine: string,
      onFallbackSubmit: (shellLine: string) => Promise<void> | void,
      opts?: { forceAgent?: boolean },
    ) => {
      const trimmed = rawLine.trim();
      if (!trimmed) return;
      try {
        const isBareAgentSlash = /^\/(?:ai|ask)$/i.test(trimmed);
        if (!isBareAgentSlash && (trimmed.startsWith('?') || AGENT_SLASH_RE.test(trimmed) || opts?.forceAgent)) {
          const prompt = trimmed.startsWith('?')
            ? trimmed.slice(1).trim()
            : AGENT_SLASH_RE.test(trimmed)
              ? trimmed.replace(AGENT_SLASH_RE, '').trim()
              : trimmed;
          if (!prompt) return;
          const agentFn = getTransportFn('termAgentPrompt');
          if (agentFn) {
            await agentFn(sid, prompt);
            return;
          }
        }
        if (SLASH_VERB_RE.test(trimmed)) {
          const slashFn = getTransportFn('termSlashExec');
          if (slashFn) {
            const res = await slashFn(sid, trimmed);
            if (res?.command === '/clear') {
              onClearedSession?.();
              await onSlashMutatedBlocks?.();
            } else if (res?.command === '/compact' && res.block_id != null) {
              await onSlashMutatedBlocks?.();
            }
            return;
          }
        }
        await Promise.resolve(onFallbackSubmit(rawLine));
      } catch (err) {
        pushToast?.({
          tone: 'warn',
          title: 'Agent turn failed',
          body: sanitizeErrorForToast(err),
          cause: 'backend-error',
        });
      }
    },
    [onClearedSession, onSlashMutatedBlocks, pushToast],
  );

  const runExplainFix = useCallback(
    async (block: TermBlockDto, mode: 'explain' | 'fix' = 'fix'): Promise<transport.TermAgentTurnDto | undefined> => {
      const sid = sessionIdRef.current;
      const fn = getTransportFn('termAgentExplainFix');
      if (!sid || block.credential_prompt || !fn) return undefined;
      setDiagnosingBlockId(block.id);
      try {
        return await fn(sid, block.id, mode);
      } catch (err) {
        pushToast?.({
          tone: 'warn',
          title: 'Explain / Fix failed',
          body: sanitizeErrorForToast(err),
          cause: 'backend-error',
        });
        return undefined;
      } finally {
        setDiagnosingBlockId((cur) => (cur === block.id ? null : cur));
      }
    },
    [pushToast],
  );

  const submitBlockFeedback = useCallback(
    async (
      block: TermBlockDto,
      verdict: 'accepted' | 'rejected' | 'corrected',
      correctedCommand?: string,
    ): Promise<boolean> => {
      const sid = sessionIdRef.current;
      const fn = getTransportFn('termBlockFeedback');
      if (!sid || block.credential_prompt || !fn) return false;
      try {
        const res = await fn(sid, block.id, verdict, correctedCommand);
        if (!res || res.recorded !== true) {
          pushToast?.({
            tone: 'warn',
            title: 'Trajectory feedback failed',
            body: 'Could not record trajectory feedback. Click Accept, Reject, or Correct again to retry.',
            cause: 'backend-error',
          });
          return false;
        }
        return true;
      } catch (err) {
        pushToast?.({
          tone: 'warn',
          title: 'Trajectory feedback failed',
          body: sanitizeErrorForToast(err),
          cause: 'backend-error',
        });
        return false;
      }
    },
    [pushToast],
  );

  const cancelAgentTurn = useCallback(async (block: TermBlockDto) => {
    const sid = sessionIdRef.current;
    const fn = getTransportFn('termAgentCancel');
    if (!sid || !fn) return;
    await fn(sid, block.id).catch(() => false);
  }, []);

  const triggerPreemptionNotice = useCallback(() => {
    setSessionPatterns([]);
    setPreemptedBanner(true);
  }, []);

  const resumeCoPilot = useCallback(() => {
    setPreemptedBanner(false);
    onControlModeChange?.('co_pilot');
    const sid = sessionIdRef.current;
    const setCtrlFn = getTransportFn('termSessionSetControl');
    if (sid && setCtrlFn) void setCtrlFn(sid, 'co_pilot').catch(() => {});
  }, [onControlModeChange]);

  const dismissPreemptedBanner = useCallback(() => {
    setPreemptedBanner(false);
  }, []);

  const approvals = [
    ...localEntries.map((item) => item.row),
    ...daemonApprovals,
  ];

  return {
    approvals,
    resolvingId,
    sessionPatterns,
    preemptedBanner,
    setPreemptedBanner,
    diagnosingBlockId,
    resolveApproval,
    stageOrRunProposedCommand,
    dispatchTerminalLine,
    runExplainFix,
    submitBlockFeedback,
    cancelAgentTurn,
    triggerPreemptionNotice,
    resumeCoPilot,
    dismissPreemptedBanner,
  };
}
