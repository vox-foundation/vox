import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import { IsolationPanel } from './IsolationPanel';
import { conflictRows, type IsolationStatus, type IsolationStrategy } from './isolationHelpers';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useLabel } from '../../../hooks/useLanguage';
import type { Toast } from '../../../types/tauri';

interface RepositoryViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
  condensed?: boolean;
}

interface ExecuteOutput {
  exit_code: number;
  stdout: string;
  stderr: string;
}

async function run(path: string[], argv: string[] = []): Promise<ExecuteOutput> {
  return invoke<ExecuteOutput>('execute_command', {
    path,
    args: { __argv: argv },
  });
}

export function RepositoryView({ pushToast, gamifyEnabled, condensed }: RepositoryViewProps) {
  const [output, setOutput] = useState('Ready — select a workspace probe above to stream CLI output.');
  const [busy, setBusy] = useState(false);

  // Live VCS isolation state (default strategy + per-agent + active conflicts),
  // bridged from the orchestrator daemon via Tauri invoke.
  const [isolation, setIsolation] = useState<IsolationStatus | null>(null);
  const [isolationBusy, setIsolationBusy] = useState(false);
  const [isolationError, setIsolationError] = useState<string | null>(null);

  const refetchIsolation = useCallback(async () => {
    try {
      const status = await invoke<IsolationStatus>('get_vcs_isolation');
      setIsolation(status);
      setIsolationError(null);
    } catch (err) {
      setIsolation(null);
      setIsolationError(sanitizeErrorForToast(err));
    }
  }, []);

  // Fetch once on mount. Mutations refetch inline (the daemon also publishes a
  // `vcs.isolation.changed` topic for future push-driven refresh).
  useEffect(() => {
    void refetchIsolation();
  }, [refetchIsolation]);

  const handleSetDefault = useCallback(
    async (strategy: IsolationStrategy) => {
      setIsolationBusy(true);
      try {
        const status = await invoke<IsolationStatus>('set_vcs_isolation_strategy', {
          default: strategy,
          agentId: null,
          strategy: null,
        });
        setIsolation(status);
        setIsolationError(null);
        pushToast({ tone: 'ok', title: 'Isolation strategy', body: `Default → ${strategy}`, cause: 'backend-ok' });
        void recordGamifyGuiEvent(
          'isolation_strategy_set',
          { strategy, scope: 'default' },
          { enabled: gamifyEnabled },
        );
      } catch (err) {
        setIsolationError(sanitizeErrorForToast(err));
        pushToast({ tone: 'warn', title: 'Isolation strategy', body: sanitizeErrorForToast(err), cause: 'backend-error' });
        // Reconcile against authoritative daemon state after a failed write.
        void refetchIsolation();
      } finally {
        setIsolationBusy(false);
      }
    },
    [pushToast, refetchIsolation, gamifyEnabled],
  );

  const runAction = async (label: string, path: string[], argv: string[] = []) => {
    setBusy(true);
    setOutput(`Running: vox ${path.join(' ')} ${argv.join(' ')}`.trim());
    try {
      const result = await run(path, argv);
      const text = [result.stdout, result.stderr].filter(Boolean).join('\n').trim();
      setOutput(text || '(no output)');
      pushToast({
        tone: result.exit_code === 0 ? 'ok' : 'warn',
        title: label,
        body: result.exit_code === 0 ? 'Completed' : `Failed (exit ${result.exit_code})`,
        cause: 'backend-ok',
      });
      if (result.exit_code === 0) {
        void recordGamifyGuiEvent(
          'isolation_scan_complete',
          { label, path: path.join('/') },
          { enabled: gamifyEnabled },
        );
      }
    } catch (err) {
      setOutput(sanitizeErrorForToast(err));
      pushToast({ tone: 'warn', title: label, body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  if (condensed) {
    const conflictCount = conflictRows(isolation).length;
    return (
      <section className="space-y-2 text-[11px] text-text-muted">
        <h2 className="font-display text-xs text-text-primary tracking-wider uppercase">{useLabel('repo-harness')}</h2>
        <div>{conflictCount} active conflict{conflictCount === 1 ? '' : 's'}</div>
      </section>
    );
  }

  const conflictCount = conflictRows(isolation).length;
  const agentCount = Object.keys(isolation?.per_agent ?? {}).length;

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="font-display text-lg text-text-primary tracking-wider uppercase">{useLabel('repo-harness')}</h2>
        <div className="flex flex-wrap items-center gap-2 text-[10px] font-mono">
          <span className="rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 text-text-secondary">
            DEFAULT: <strong className="text-brass uppercase">{isolation?.strategy_default ?? 'shared_branch'}</strong>
          </span>
          <span className="rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 text-text-secondary">
            AGENTLANES: <strong className="text-text-primary">{agentCount}</strong>
          </span>
          <span className={`rounded border px-2 py-0.5 ${conflictCount > 0 ? 'border-amber-400/35 bg-amber-400/10 text-amber-300' : 'border-emerald-400/30 bg-emerald-400/10 text-emerald-300'}`}>
            CONFLICTS: <strong>{conflictCount}</strong>
          </span>
        </div>
      </div>
      <div className="grid gap-2.5 sm:grid-cols-2">
        <button
          type="button"
          className="flex items-center justify-between rounded-lg border border-border-subtle bg-overlay-subtle px-3.5 py-2.5 text-left text-sm font-medium text-text-primary transition hover:border-brass/40 hover:bg-brass/5"
          disabled={busy}
          onClick={() => runAction('Workspace status', ['status'])}
        >
          <span>Workspace status</span>
          <span className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[10px] text-brass">vox status</span>
        </button>
        <button
          type="button"
          className="flex items-center justify-between rounded-lg border border-border-subtle bg-overlay-subtle px-3.5 py-2.5 text-left text-sm font-medium text-text-primary transition hover:border-brass/40 hover:bg-brass/5"
          disabled={busy}
          onClick={() => runAction('Repository health', ['check', 'workspace'])}
        >
          <span>Repo health check</span>
          <span className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[10px] text-brass">vox check workspace</span>
        </button>
        <button
          type="button"
          className="flex items-center justify-between rounded-lg border border-border-subtle bg-overlay-subtle px-3.5 py-2.5 text-left text-sm font-medium text-text-primary transition hover:border-brass/40 hover:bg-brass/5"
          disabled={busy}
          onClick={() => runAction('Vox check', ['check'])}
        >
          <span className="font-mono text-brass">`vox check`</span>
          <span className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[10px] text-text-secondary">typecheck + lint</span>
        </button>
        <button
          type="button"
          className="flex items-center justify-between rounded-lg border border-border-subtle bg-overlay-subtle px-3.5 py-2.5 text-left text-sm font-medium text-text-primary transition hover:border-brass/40 hover:bg-brass/5"
          disabled={busy}
          onClick={() => runAction('List changed files', ['diff'])}
        >
          <span className="font-mono text-brass">`vox diff`</span>
          <span className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[10px] text-text-secondary">working tree</span>
        </button>
      </div>
      <pre
        aria-label="Command output"
        aria-live="polite"
        aria-busy={busy}
        className="max-h-[420px] overflow-auto rounded-lg border border-border-subtle bg-bg-base/80 p-3 font-mono text-xs text-text-secondary"
      >
        {output}
      </pre>
      <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-3">
        <IsolationPanel
          status={isolation}
          onSetDefault={handleSetDefault}
          busy={isolationBusy}
          unavailableNote={
            isolationError
              ? `Live isolation status unavailable: ${isolationError}`
              : 'Live isolation status is loading from the orchestrator daemon…'
          }
        />
      </div>
    </section>
  );
}
