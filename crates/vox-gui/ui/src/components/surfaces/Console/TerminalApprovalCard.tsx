import React, { useEffect, useId, useRef, useState } from 'react';
import {
  deriveSessionAllowPattern,
  type ResolveApprovalDecision,
  type ResolveApprovalResult,
} from '../../../hooks/useTerminalPendingApprovals';
import type { PendingApprovalRow } from '../../../lib/mcpToolResult';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { TERMINAL_MONO_STYLE } from './terminalTheme';

export interface TerminalApprovalCardProps {
  approval: PendingApprovalRow;
  compact?: boolean;
  resolving?: boolean;
  queueTotal?: number;
  onResolve: (
    approval: PendingApprovalRow,
    decision: ResolveApprovalDecision,
  ) => Promise<ResolveApprovalResult | void> | void;
  onRecordCorrection?: (originalCommand: string, editedCommand: string) => void;
}

export function TerminalApprovalCard({
  approval,
  compact = false,
  resolving = false,
  queueTotal = 1,
  onResolve,
  onRecordCorrection,
}: TerminalApprovalCardProps) {
  const [activeMode, setActiveMode] = useState<'idle' | 'edit' | 'steer'>('idle');
  const proposedCommand = approval.command ?? approval.summary ?? '';
  const [editedCommand, setEditedCommand] = useState(proposedCommand);
  const [editError, setEditError] = useState<string | null>(null);
  const [steeringNote, setSteeringNote] = useState('');
  const [localBusy, setLocalBusy] = useState(false);

  const editInputId = useId();
  const steerInputId = useId();
  const editInputRef = useRef<HTMLTextAreaElement | null>(null);
  const steerInputRef = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (activeMode === 'edit') editInputRef.current?.focus();
    else if (activeMode === 'steer') steerInputRef.current?.focus();
  }, [activeMode]);
  const busy = resolving || localBusy;
  const isHighRisk = Boolean(approval.credential_prompt || approval.floor_category);
  const derivedPattern = deriveSessionAllowPattern(proposedCommand);

  const sealLabel = approval.credential_prompt
    ? 'CREDENTIAL GATE'
    : approval.floor_category
      ? 'SAFETY FLOOR GATE'
      : 'CO-PILOT GATE';

  const reasonSummary = approval.credential_prompt
    ? 'Credential prompt - user input required'
    : approval.floor_category
      ? `Safety floor - ${approval.floor_category}`
      : approval.summary || 'Co-Pilot review - workspace command';

  const runDecision = async (decision: ResolveApprovalDecision): Promise<boolean> => {
    setLocalBusy(true);
    try {
      const res = await Promise.resolve(onResolve(approval, decision));
      if (res && typeof res === 'object' && res.ok === false) {
        if (decision.outcome === 'modified') {
          setEditError(res.error ?? 'Edited command rejected by ExecPolicy.');
        }
        return false;
      }
      return true;
    } finally {
      setLocalBusy(false);
    }
  };

  const handleEditSubmit = async () => {
    const trimmed = editedCommand.trim();
    if (!trimmed) {
      setEditError('Edited command cannot be empty.');
      return;
    }
    setEditError(null);
    const ok = await runDecision({ outcome: 'modified', modifiedCommand: trimmed });
    if (ok) {
      if (trimmed !== proposedCommand.trim()) {
        onRecordCorrection?.(proposedCommand, trimmed);
      }
      setActiveMode('idle');
    }
  };

  const handleRejectSubmit = async () => {
    const note = steeringNote.trim();
    const ok = await runDecision({
      outcome: 'rejected',
      ...(note ? { steeringNote: note } : {}),
    });
    if (ok) {
      setSteeringNote('');
      setActiveMode('idle');
    }
  };

  return (
    <section
      role="region"
      aria-label="Co-Pilot command approval required"
      data-testid="term-approval-card"
      style={TERMINAL_MONO_STYLE}
      className={`flex flex-col gap-1 min-[1100px]:gap-2 rounded-lg border ${
        isHighRisk ? 'border-(--color-status-fail)' : 'border-brass'
      } bg-bg-elevated ${compact ? 'p-2' : 'p-2 min-[1100px]:p-3'} text-xs text-text-primary`}
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <span
            data-roman-wax-seal="true"
            data-seal-tone={isHighRisk ? 'crimson' : 'gold'}
            className={`inline-flex items-center gap-1 rounded-full border px-2 py-1 text-[11px] leading-[1.25] font-semibold uppercase tracking-wider ${
              isHighRisk
                ? 'border-(--color-status-fail) bg-bg-base text-(--color-status-fail)'
                : 'border-brass bg-bg-base text-accent'
            }`}
          >
            <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />
            <span>{sealLabel}</span>
          </span>
          <span data-testid="term-approval-reason" className="text-[11px] leading-[1.25] font-semibold text-text-secondary max-[1099px]:hidden">
            {reasonSummary}
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-2 max-[1099px]:hidden">
          {queueTotal > 1 ? (
            <span
              data-testid="term-approval-queue-count"
              className="rounded border border-border-subtle bg-bg-surface px-2 py-1 text-[11px] leading-[1.25] font-semibold text-accent"
            >
              {`1 of ${queueTotal} pending`}
            </span>
          ) : null}
          <span className="rounded border border-border-subtle bg-bg-surface px-2 py-1 text-[11px] leading-[1.25] font-semibold text-text-secondary">
            {`term:${approval.session_id ?? 'active'}`}
          </span>
          <span
            data-testid="term-approval-pattern-pill"
            className={`rounded border bg-bg-surface px-2 py-1 text-[11px] leading-[1.25] font-semibold ${
              !isHighRisk && derivedPattern
                ? 'border-border-strong text-text-primary'
                : 'border-(--color-status-warn) text-(--color-status-warn)'
            }`}
          >
            {!isHighRisk && derivedPattern
              ? `Scope: ${derivedPattern} (session ${approval.session_id ?? 'active'})`
              : 'Single-command approval only'}
          </span>
        </div>
      </div>

      <pre
        tabIndex={0}
        data-testid="term-approval-command"
        data-terminal-mono="true"
        style={TERMINAL_MONO_STYLE}
        className="max-h-10 min-[1100px]:max-h-32 overflow-auto whitespace-pre-wrap break-words rounded border border-border-subtle bg-bg-base px-2 py-1 min-[1100px]:p-2 text-xs leading-[1.45] font-normal text-text-primary"
      >
        {`$ ${proposedCommand}`}
      </pre>

      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="xs"
          variant="primary"
          data-testid="term-approval-approve"
          aria-label="Approve proposed command"
          disabled={busy}
          onClick={() => void runDecision({ outcome: 'approved' })}
          className="h-6 gap-1 rounded border border-(--color-accent-default) bg-(--color-accent-default) px-2 py-1 text-[11px] font-semibold text-bg-base"
        >
          <Icon.check className="size-3" aria-hidden="true" />
          <span>Approve</span>
        </Button>
        <Button
          size="xs"
          variant="outline"
          data-testid="term-approval-edit-toggle"
          aria-expanded={activeMode === 'edit'}
          aria-label="Edit & Run proposed command"
          disabled={busy}
          onClick={() => {
            setEditedCommand(proposedCommand);
            setEditError(null);
            setActiveMode((prev) => (prev === 'edit' ? 'idle' : 'edit'));
          }}
          className="h-6 gap-1 rounded border border-border-strong bg-bg-surface px-2 py-1 text-[11px] font-semibold text-text-primary"
        >
          <Icon.edit className="size-3" aria-hidden="true" />
          <span>Edit & Run</span>
        </Button>
        <Button
          size="xs"
          variant="secondary"
          data-testid="term-approval-allow-pattern"
          aria-label={
            derivedPattern && !isHighRisk
              ? `Allow Pattern in Session ${derivedPattern}`
              : 'Allow Pattern in Session unavailable'
          }
          disabled={busy || isHighRisk || !derivedPattern}
          onClick={() =>
            void runDecision({
              outcome: 'approved',
              ...(derivedPattern ? { allowPattern: derivedPattern } : {}),
            })
          }
          className="h-6 gap-1 rounded border border-border-strong bg-bg-surface px-2 py-1 text-[11px] font-semibold text-text-primary"
        >
          <Icon.shield className="size-3" aria-hidden="true" />
          <span>Allow Pattern in Session</span>
        </Button>
        <Button
          size="xs"
          variant="ghost"
          data-testid="term-approval-reject-toggle"
          aria-expanded={activeMode === 'steer'}
          aria-label="Reject + Steer proposed command"
          disabled={busy}
          onClick={() => setActiveMode((prev) => (prev === 'steer' ? 'idle' : 'steer'))}
          className="h-6 gap-1 rounded border border-(--color-status-fail) bg-bg-surface px-2 py-1 text-[11px] font-semibold text-(--color-status-fail)"
        >
          <Icon.ban className="size-3" aria-hidden="true" />
          <span>Reject + Steer</span>
        </Button>
      </div>

      {activeMode === 'edit' ? (
        <div
          data-testid="term-approval-edit-panel"
          className="flex flex-col gap-2 rounded border border-border-strong bg-bg-surface p-2"
        >
          <label htmlFor={editInputId} className="text-[11px] leading-[1.25] font-semibold text-text-secondary">
            Edit proposed command before running (re-checked against ExecPolicy AST)
          </label>
          <textarea
            ref={editInputRef}
            id={editInputId}
            rows={2}
            data-testid="term-approval-edit-input"
            data-terminal-mono="true"
            aria-label="Edit proposed command before running"
            value={editedCommand}
            onChange={(e) => setEditedCommand(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.preventDefault();
                setActiveMode('idle');
                setEditError(null);
              }
            }}
            style={TERMINAL_MONO_STYLE}
            className="min-h-12 w-full resize-y rounded border border-border-strong bg-bg-base p-2 text-xs leading-[1.45] font-normal text-text-primary focus:outline-none focus:ring-1 focus:ring-ring"
          />
          {editError ? (
            <div
              role="alert"
              data-testid="term-approval-edit-error"
              className="text-[11px] leading-[1.25] font-semibold text-(--color-status-fail)"
            >
              {editError}
            </div>
          ) : null}
          <div className="flex items-center justify-end gap-2">
            <Button
              size="xs"
              variant="ghost"
              data-testid="term-approval-edit-cancel"
              aria-label="Cancel Edit"
              onClick={() => {
                setActiveMode('idle');
                setEditError(null);
              }}
              className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold text-text-secondary"
            >
              Cancel Edit
            </Button>
            <Button
              size="xs"
              variant="primary"
              data-testid="term-approval-edit-submit"
              aria-label="Run Edited Command"
              disabled={!editedCommand.trim() || busy}
              onClick={() => void handleEditSubmit()}
              className="h-6 gap-1 rounded border border-(--color-accent-default) bg-(--color-accent-default) px-2 py-1 text-[11px] font-semibold text-bg-base"
            >
              <Icon.play className="size-3" aria-hidden="true" />
              <span>Run Edited Command</span>
            </Button>
          </div>
        </div>
      ) : null}

      {activeMode === 'steer' ? (
        <div
          data-testid="term-approval-steer-panel"
          className="flex flex-col gap-2 rounded border border-(--color-status-fail) bg-bg-surface p-2"
        >
          <label htmlFor={steerInputId} className="text-[11px] leading-[1.25] font-semibold text-text-secondary">
            Optional steering note for the agent
          </label>
          <input
            ref={steerInputRef}
            id={steerInputId}
            type="text"
            data-testid="term-approval-steer-input"
            aria-label="Optional steering note for the agent"
            value={steeringNote}
            onChange={(e) => setSteeringNote(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                void handleRejectSubmit();
              } else if (e.key === 'Escape') {
                e.preventDefault();
                setActiveMode('idle');
              }
            }}
            placeholder="Steer the agent toward an alternative approach (optional)..."
            className="h-6 w-full rounded border border-border-strong bg-bg-base px-2 py-1 font-sans text-xs leading-[1.45] font-normal text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <div className="flex items-center justify-end gap-2">
            <Button
              size="xs"
              variant="ghost"
              data-testid="term-approval-reject-cancel"
              aria-label="Cancel Reject"
              onClick={() => setActiveMode('idle')}
              className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold text-text-secondary"
            >
              Cancel Reject
            </Button>
            <Button
              size="xs"
              variant="secondary"
              data-testid="term-approval-reject-submit"
              aria-label="Confirm Reject & Steer"
              disabled={busy}
              onClick={() => void handleRejectSubmit()}
              className="h-6 gap-1 rounded border border-(--color-status-fail) bg-(--color-status-fail) px-2 py-1 text-[11px] font-semibold text-bg-base hover:opacity-90"
            >
              <Icon.ban className="size-3" aria-hidden="true" />
              <span>Confirm Reject & Steer</span>
            </Button>
          </div>
        </div>
      ) : null}
    </section>
  );
}

export interface TerminalPaneApprovalBannerProps {
  approvals: PendingApprovalRow[];
  resolvingId?: string | null;
  compact?: boolean;
  preemptedBanner: boolean;
  onResumeCoPilot: () => void;
  onDismissPreemptedBanner: () => void;
  onResolveApproval: (
    approval: PendingApprovalRow,
    decision: ResolveApprovalDecision,
  ) => Promise<ResolveApprovalResult | void> | void;
  onRecordCorrection?: (originalCommand: string, editedCommand: string) => void;
}

export function TerminalPaneApprovalBanner({
  approvals,
  resolvingId = null,
  compact = false,
  preemptedBanner,
  onResumeCoPilot,
  onDismissPreemptedBanner,
  onResolveApproval,
  onRecordCorrection,
}: TerminalPaneApprovalBannerProps) {
  if (!preemptedBanner && approvals.length === 0) return null;
  const activeApproval = approvals[0];

  return (
    <div
      tabIndex={0}
      className="flex max-h-44 min-h-0 shrink-0 flex-col gap-1 overflow-y-auto px-2 py-1 min-[1100px]:max-h-none min-[1100px]:gap-2 min-[1100px]:overflow-visible"
    >
      {preemptedBanner ? (
        <div
          role="status"
          aria-live="polite"
          data-testid="term-autopilot-preempted-banner"
          className="flex min-h-8 shrink-0 flex-wrap items-center justify-between gap-2 rounded-lg border border-(--color-status-warn) bg-bg-elevated px-3 py-1 text-xs text-text-primary"
        >
          <span className="inline-flex items-center gap-2 text-[11px] leading-[1.25] font-semibold text-(--color-status-warn)">
            <Icon.gladiusShield className="size-4 shrink-0" aria-hidden="true" />
            <span>Autopilot preempted to Manual on operator input - queued agent commands paused.</span>
          </span>
          <div className="flex items-center gap-2">
            <Button
              size="xs"
              variant="secondary"
              data-testid="term-preempt-resume-copilot"
              aria-label="Resume Co-Pilot mode"
              onClick={onResumeCoPilot}
              className="h-6 rounded border border-brass bg-bg-base px-2 py-1 text-[11px] font-semibold text-accent"
            >
              Resume Co-Pilot
            </Button>
            <Button
              size="xs"
              variant="ghost"
              data-testid="term-preempt-dismiss"
              aria-label="Dismiss Notice"
              onClick={onDismissPreemptedBanner}
              className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold text-text-secondary"
            >
              Dismiss Notice
            </Button>
          </div>
        </div>
      ) : null}

      {activeApproval ? (
        <div data-testid="term-approval-tray">
          <TerminalApprovalCard
            key={activeApproval.approval_id}
            approval={activeApproval}
            compact={compact}
            resolving={resolvingId === activeApproval.approval_id}
            queueTotal={approvals.length}
            onResolve={onResolveApproval}
            onRecordCorrection={onRecordCorrection}
          />
        </div>
      ) : null}
    </div>
  );
}
