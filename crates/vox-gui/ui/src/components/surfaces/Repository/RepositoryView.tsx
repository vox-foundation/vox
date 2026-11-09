import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import { IsolationPanel } from './IsolationPanel';
import { conflictRows, type IsolationStatus, type IsolationStrategy } from './isolationHelpers';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useLabel } from '../../../hooks/useLanguage';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';
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
  // vcs.isolation.changed topic for future push-driven refresh).
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
  const GitIcon = (Icon as Record<string, any>).git ?? Icon.branch;

  const repoGauges = [
    {
      label: 'Roots',
      valueText: '1 Poly',
      ratio: 0.33,
      tone: 'gold' as const,
      purpose: 'Active polyrepo workspace roots ledger',
    },
    {
      label: 'Modified',
      valueText: conflictCount > 0 ? `${conflictCount} Conf` : 'Clean',
      ratio: conflictCount > 0 ? 0.75 : 0.15,
      tone: conflictCount > 0 ? ('crimson' as const) : ('emerald' as const),
      purpose: 'Working tree modifications and active conflict rows',
    },
    {
      label: 'Isolation',
      valueText: (isolation?.strategy_default ?? 'shared_branch').replace('_', ' '),
      ratio: 0.65,
      tone: 'gold' as const,
      purpose: 'VCS agent isolation strategy default',
    },
  ];

  return (
    <section className="space-y-4 rounded-xl border border-[#D4AF37]/40 bg-gradient-to-b from-[#1B150E] to-[#110D08] p-4">
      <RomanSurfaceHeader
        domainTag="WORKSPACE · REPOSITORY"
        title="Repository Polyrepo & JJ VCS Ledger"
        subtitle="Inspect working-tree status, active branches, polyrepo roots, and compiler diagnostics."
        waxSealBadge="VCS ACTIVE"
        icon={<GitIcon className="size-5 text-[#F3E5AB]" />}
        gauges={repoGauges}
      />
      <div
        style={{
          backgroundImage:
            'linear-gradient(90deg, rgba(248,243,233,0.95) 0%, rgba(235,224,205,0.93) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="flex flex-wrap items-center justify-between gap-2 rounded-lg border-2 border-[#C59B27] px-3 py-2 text-[#1C140C] shadow-[0_3px_10px_rgba(0,0,0,0.35),inset_0_1px_0_rgba(255,255,255,0.9)]"
      >
        <h2 className="font-display text-xs font-bold text-[#1C140C] tracking-wider uppercase">{useLabel('repo-harness')}</h2>
        <div className="flex flex-wrap items-center gap-2 text-[10px] font-mono">
          <span className="rounded border border-[#8C6825] bg-[#251C0E] px-2 py-0.5 text-[#D8C7A8]">
            DEFAULT: <strong className="text-[#F3E5AB] uppercase">{isolation?.strategy_default ?? 'shared_branch'}</strong>
          </span>
          <span className="rounded border border-[#8C6825] bg-[#251C0E] px-2 py-0.5 text-[#D8C7A8]">
            AGENTLANES: <strong className="text-[#F3E5AB]">{agentCount}</strong>
          </span>
          <span className={`rounded border px-2 py-0.5 ${conflictCount > 0 ? 'border-amber-400/50 bg-[#3B2308] text-amber-200' : 'border-[#8C6825] bg-[#364116] text-[#DCE7B6]'}`}>
            CONFLICTS: <strong>{conflictCount}</strong>
          </span>
        </div>
      </div>
      <div className="grid gap-2.5 sm:grid-cols-2">
        <button
          type="button"
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
            backgroundSize: 'cover',
          }}
          className="flex items-center justify-between rounded-lg border-2 border-[#C59B27] px-3.5 py-2.5 text-left text-sm font-bold text-[#1C140C] shadow-sm transition hover:border-[#8C6825]"
          disabled={busy}
          onClick={() => runAction('Workspace status', ['status'])}
        >
          <span>Workspace status</span>
          <span className="rounded border border-[#8C6825] bg-[#251C0E] px-1.5 py-0.5 font-mono text-[10px] text-[#F3E5AB]">vox status</span>
        </button>
        <button
          type="button"
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
            backgroundSize: 'cover',
          }}
          className="flex items-center justify-between rounded-lg border-2 border-[#C59B27] px-3.5 py-2.5 text-left text-sm font-bold text-[#1C140C] shadow-sm transition hover:border-[#8C6825]"
          disabled={busy}
          onClick={() => runAction('Repository health', ['check', 'workspace'])}
        >
          <span>Repo health check</span>
          <span className="rounded border border-[#8C6825] bg-[#251C0E] px-1.5 py-0.5 font-mono text-[10px] text-[#F3E5AB]">vox check workspace</span>
        </button>
        <button
          type="button"
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
            backgroundSize: 'cover',
          }}
          className="flex items-center justify-between rounded-lg border-2 border-[#C59B27] px-3.5 py-2.5 text-left text-sm font-bold text-[#1C140C] shadow-sm transition hover:border-[#8C6825]"
          disabled={busy}
          onClick={() => runAction('Vox check', ['check'])}
        >
          <span className="font-mono text-[#1C140C]">Run Vox Check</span>
          <span className="rounded border border-[#8C6825] bg-[#251C0E] px-1.5 py-0.5 font-mono text-[10px] text-[#F3E5AB]">typecheck + lint</span>
        </button>
        <button
          type="button"
          aria-label="`vox diff`"
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
            backgroundSize: 'cover',
          }}
          className="flex items-center justify-between rounded-lg border-2 border-[#C59B27] px-3.5 py-2.5 text-left text-sm font-bold text-[#1C140C] shadow-sm transition hover:border-[#8C6825]"
          disabled={busy}
          onClick={() => runAction('List changed files', ['diff'])}
        >
          <span className="font-mono text-[#1C140C]">Inspect Git Diff</span>
          <span className="rounded border border-[#8C6825] bg-[#251C0E] px-1.5 py-0.5 font-mono text-[10px] text-[#F3E5AB]">working tree</span>
        </button>
      </div>
      <pre
        aria-label="Command output"
        aria-live="polite"
        aria-busy={busy}
        className="max-h-[420px] overflow-auto rounded-lg border-2 border-[#C59B27] bg-[#18120B] p-3 font-mono text-xs text-[#F5EFE2] shadow-inner"
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
