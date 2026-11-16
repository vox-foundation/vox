import React, { useEffect, useState } from 'react';
import type { TermControlModeId, TermSecurityLevelId } from '../../../lib/terminalBridge';
import type { TermSessionInfoDto } from '../../../transport';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';
import { MAX_SPLIT_PANES } from './ConsoleSplitGrid';
import { fromWireSecurityLevel } from './sessionReducer';
import { detectSubshellShellHint } from './useConsoleWarpActions';

export const SECURITY_LEVEL_OPTIONS = [
  { value: 'l1_read_only', label: 'L1 Read-Only' },
  { value: 'l2_safe_exec', label: 'L2 Safe Exec' },
  { value: 'l3_workspace_write', label: 'L3 Workspace Write' },
  { value: 'l4_system_exec', label: 'L4 System Exec' },
  { value: 'l5_autopilot_floor', label: 'L5 Autopilot Floor' },
] as const;

export interface ConsoleSessionBarProps {
  sessions: readonly TermSessionInfoDto[];
  activeSessionId: string | null;
  runningCommandBySession?: Record<string, string | null | undefined>;
  leafCount: number;
  securityLevel: TermSecurityLevelId;
  controlMode: TermControlModeId;
  onSelectSession: (sessionId: string) => void;
  onNewSession: () => void;
  onCloseSession?: (sessionId: string) => void;
  onSplitRight: () => void;
  onSplitDown: () => void;
  onSecurityLevelChange: (next: TermSecurityLevelId) => void;
  onCycleControlMode: () => void;
  onVoxifySubshell?: (sessionId: string) => void;
  voxifiedSessionIds?: ReadonlySet<string>;
}

const basenameOfPath = (rawPath: string): string =>
  rawPath.replace(/[\\/]+$/, '').split(/[\\/]/).filter(Boolean).pop() ?? rawPath;

function deriveSessionPrimaryLabel(session: TermSessionInfoDto, runningCmd: string | null | undefined): string {
  const trimmedCmd = runningCmd?.trim();
  if (trimmedCmd) {
    const shortCmd = trimmedCmd.length > 20 ? `${trimmedCmd.slice(0, 20)}...` : trimmedCmd;
    return `${shortCmd} (${session.blocks})`;
  }
  const trimmedCwd = session.cwd?.trim();
  return trimmedCwd ? `${basenameOfPath(trimmedCwd)} (${session.blocks})` : `${session.id} (${session.blocks})`;
}

export function ConsoleSessionBar({
  sessions, activeSessionId, runningCommandBySession, leafCount, securityLevel, controlMode,
  onSelectSession, onNewSession, onCloseSession, onSplitRight, onSplitDown,
  onSecurityLevelChange, onCycleControlMode, onVoxifySubshell, voxifiedSessionIds,
}: ConsoleSessionBarProps) {
  const [confirmCloseSid, setConfirmCloseSid] = useState<string | null>(null);
  const selectValue = fromWireSecurityLevel(securityLevel);
  const isVoxified = Boolean(activeSessionId && voxifiedSessionIds?.has(activeSessionId));
  const subshellDetected = Boolean(activeSessionId && detectSubshellShellHint(runningCommandBySession?.[activeSessionId]));
  const showVoxifyButton = Boolean(activeSessionId && onVoxifySubshell && (subshellDetected || isVoxified));
  const controlLabel = controlMode === 'autopilot' ? 'Autopilot'
    : controlMode === 'co_pilot' || controlMode === 'copilot' ? 'Co-Pilot' : 'Manual';

  useEffect(() => {
    if (!confirmCloseSid) return;
    const stillExists = sessions.some((s) => s.id === confirmCloseSid);
    const stillRunning = Boolean(runningCommandBySession?.[confirmCloseSid]?.trim());
    if (!stillExists || !stillRunning) setConfirmCloseSid(null);
  }, [confirmCloseSid, sessions, runningCommandBySession]);

  useEffect(() => {
    if (!confirmCloseSid) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        setConfirmCloseSid(null);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [confirmCloseSid]);

  return (
    <div className="flex shrink-0 flex-col gap-1">
      <div
        data-testid="term-session-bar"
        aria-label="terminal sessions"
        className="flex shrink-0 flex-wrap items-center gap-2 rounded-lg border border-border-strong bg-bg-surface px-2 py-1 text-[11px] text-text-primary"
      >
        {sessions.map((s) => {
          const isActive = activeSessionId === s.id;
          const runningCmd = runningCommandBySession?.[s.id] ?? null;
          const primaryLabel = deriveSessionPrimaryLabel(s, runningCmd);
          return (
            <div
              key={s.id}
              role="group"
              aria-label={`Session ${s.id}`}
              className={`inline-flex min-h-7 items-center gap-1 rounded border pl-2 pr-1 ${
                isActive ? 'border-brass bg-bg-elevated font-semibold text-text-primary' : 'border-border-subtle bg-bg-base text-text-secondary'
              }`}
            >
              <button
                type="button"
                data-testid={`term-session-tab-${s.id}`}
                title={runningCmd?.trim() || s.cwd || s.id}
                onClick={() => onSelectSession(s.id)}
                className={`inline-flex max-w-56 items-center gap-1 truncate font-semibold focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring ${
                  isActive ? 'text-sm leading-[1.3]' : 'text-[11px] leading-[1.25]'
                }`}
              >
                <span className="min-w-12 truncate">{primaryLabel}</span>
                {s.git_branch ? (
                  <span className="hidden min-[1100px]:inline-flex max-w-28 items-center gap-1 rounded bg-overlay-subtle px-1 text-[11px] text-text-secondary">
                    <Icon.branch className="size-3 shrink-0" aria-hidden="true" />
                    <span className="truncate">{s.git_branch}</span>
                  </span>
                ) : null}
              </button>
              {onCloseSession ? (
                <button
                  type="button"
                  data-testid={`term-session-close-${s.id}`}
                  aria-label={`Close session ${s.id}`}
                  title={`Close session ${s.id}`}
                  onClick={() => (runningCmd?.trim() ? setConfirmCloseSid(s.id) : onCloseSession(s.id))}
                  className="inline-flex size-6 shrink-0 items-center justify-center rounded text-text-muted hover:bg-overlay-hover hover:text-text-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                >
                  <Icon.close className="size-3" aria-hidden="true" />
                </button>
              ) : null}
            </div>
          );
        })}

        <Button size="xs" variant="secondary" data-testid="term-new-session" onClick={onNewSession} className="h-6 border border-border-strong bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-primary">
          + new session
        </Button>

        <Button size="xs" variant="ghost" data-testid="term-split-right" aria-label="Split Pane Right" disabled={leafCount >= MAX_SPLIT_PANES} onClick={onSplitRight} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold">
          <Icon.splitRight className="size-4" aria-hidden="true" />
          <span>Split Pane Right</span>
        </Button>

        <Button size="xs" variant="ghost" data-testid="term-split-down" aria-label="Split Pane Down" disabled={leafCount >= MAX_SPLIT_PANES} onClick={onSplitDown} className="h-6 gap-1 px-2 py-1 text-[11px] font-semibold">
          <Icon.splitDown className="size-4" aria-hidden="true" />
          <span>Split Pane Down</span>
        </Button>

        <div className="ml-auto flex items-center gap-2">
          {showVoxifyButton && activeSessionId && onVoxifySubshell ? (
            <button
              type="button"
              data-testid="term-voxify-subshell"
              aria-label={isVoxified ? 'Subshell Voxified - click to re-inject hooks' : 'Voxify Subshell - inject OSC 633 prompt hooks'}
              onClick={() => onVoxifySubshell(activeSessionId)}
              className={`inline-flex h-6 items-center gap-1 rounded border px-2 py-1 text-[11px] font-semibold transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring ${
                isVoxified
                  ? 'border-border-subtle bg-bg-elevated text-text-secondary hover:text-text-primary'
                  : 'border-brass bg-overlay-subtle text-accent hover:bg-brass/20'
              }`}
            >
              {isVoxified ? <Icon.check className="size-3 text-(--color-status-pass)" aria-hidden="true" /> : <Icon.spark className="size-3 text-accent" aria-hidden="true" />}
              <span>{isVoxified ? 'Subshell Voxified' : 'Voxify Subshell'}</span>
            </button>
          ) : null}
          <VoxSelect
            size="xs"
            data-testid="term-security-level-select"
            aria-label="Terminal security level"
            value={selectValue}
            onValueChange={(v) => onSecurityLevelChange(v as TermSecurityLevelId)}
            options={SECURITY_LEVEL_OPTIONS}
            className="h-6 w-auto shrink-0 border-border-strong bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-primary"
          />
          <button
            type="button"
            data-testid="term-control-mode-pill"
            onClick={onCycleControlMode}
            className={`h-6 rounded border px-2 py-1 text-[11px] font-semibold uppercase tracking-wider transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring ${
              controlMode === 'autopilot' ? 'border-(--color-status-pass) bg-overlay-subtle text-(--color-status-pass)' : 'border-border-strong bg-bg-elevated text-text-primary'
            }`}
          >
            {controlLabel}
          </button>
        </div>
      </div>

      {confirmCloseSid ? (
        <div role="alert" data-testid="term-close-confirm-bar" className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-(--color-status-warn) bg-bg-elevated px-2 py-1 text-xs text-text-primary">
          <span className="font-semibold text-(--color-status-warn)">
            Foreground command still running. Terminate process and close session?
          </span>
          <div className="flex items-center gap-2">
            <Button size="xs" variant="ghost" onClick={() => { const sid = confirmCloseSid; setConfirmCloseSid(null); onCloseSession?.(sid); }} className="h-6 rounded border border-(--color-status-fail) bg-(--color-status-fail) px-2 py-1 text-[11px] font-semibold text-bg-base hover:opacity-90">
              Confirm Close Session
            </Button>
            <Button size="xs" variant="ghost" onClick={() => setConfirmCloseSid(null)} className="h-6 px-2 py-1 text-[11px] font-semibold">
              Keep Session Open
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
