import React from 'react';
import type { ComputeTier, FileLockCellDto } from '../../../types/concilium';
import { Icon } from '../../ui/Icons';

export interface FileAffinityLockMatrixProps {
  locks: FileLockCellDto[];
  headSeq?: number;
}

function tierShortLabel(tier: ComputeTier): string {
  switch (tier) {
    case 'local_mens_gpu':
      return 'MENS GPU';
    case 'populi_quic_peer':
      return 'QUIC PEER';
    case 'cloud_bandit':
      return 'CLOUD BANDIT';
    case 'inline_cpu':
      return 'INLINE CPU';
  }
}

export function FileAffinityLockMatrix({ locks, headSeq }: FileAffinityLockMatrixProps) {
  const contendedCount = locks.filter((l) => l.contended || l.waiters.length > 0).length;

  return (
    <section
      data-slot="populi-lock-matrix"
      data-testid="populi-lock-matrix"
      aria-label="FileAffinityMap Lock Matrix"
      className="flex min-w-0 flex-col gap-2.5 rounded-xl border border-border-strong bg-bg-surface p-3.5 text-text-primary"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2">
        <div className="flex items-center gap-2">
          <span className="flex size-7 items-center justify-center rounded-lg border border-brass/30 bg-brass/10 text-[var(--color-accent-default)]">
            <Icon.lockKeyhole className="size-3.5" aria-hidden="true" />
          </span>
          <div>
            <h2 className="font-display text-xs font-bold tracking-[0.14em] text-text-primary uppercase">
              FileAffinityMap Lock Matrix
            </h2>
            <p className="font-mono text-[10px] text-text-muted">
              Rust-enforced directory and prefix leases across swarm workspace modes
            </p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 font-mono text-[10px]">
          <span className="rounded border border-border-strong bg-bg-elevated px-2 py-0.5 text-text-primary">
            {locks.length} active lease{locks.length === 1 ? '' : 's'}
          </span>
          {contendedCount > 0 && (
            <span
              data-testid="lock-matrix-contended-badge"
              className="inline-flex items-center gap-1 rounded border border-[var(--color-status-fail)]/50 bg-[var(--color-status-fail)]/12 px-2 py-0.5 text-[var(--color-status-fail)]"
            >
              <Icon.shield className="size-3" aria-hidden="true" />
              <span>{contendedCount} contended</span>
            </span>
          )}
          {headSeq != null && headSeq > 0 && (
            <span className="rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 text-text-secondary">
              head #seq-{headSeq}
            </span>
          )}
        </div>
      </div>

      {locks.length === 0 ? (
        <div
          data-testid="populi-lock-matrix-empty"
          className="rounded-lg border border-dashed border-border-subtle bg-overlay-subtle px-3 py-5 text-center font-mono text-[11px] text-text-muted"
        >
          No active FileAffinityMap prefix leases. Workers hold clean uncontended scopes.
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left font-mono text-[11px]">
            <thead>
              <tr className="border-b border-border-subtle text-[10px] tracking-wider text-text-muted uppercase">
                <th scope="col" className="px-2 py-1.5 font-display">
                  Prefix
                </th>
                <th scope="col" className="px-2 py-1.5 font-display">
                  Lease
                </th>
                <th scope="col" className="px-2 py-1.5 font-display">
                  Owner
                </th>
                <th scope="col" className="px-2 py-1.5 font-display">
                  Compute Tier
                </th>
                <th scope="col" className="px-2 py-1.5 font-display">
                  Workspace
                </th>
                <th scope="col" className="px-2 py-1.5 font-display">
                  Seq Watermark
                </th>
                <th scope="col" className="px-2 py-1.5 font-display">
                  Contention Shield
                </th>
              </tr>
            </thead>
            <tbody>
              {locks.map((cell, idx) => {
                const isContended = cell.contended || cell.waiters.length > 0;
                return (
                  <tr
                    key={`${cell.prefix}-${cell.holder_agent}-${cell.acquired_seq}-${idx}`}
                    data-testid={`lock-matrix-row-${cell.prefix}`}
                    data-contended={isContended ? 'true' : 'false'}
                    className={`border-t border-border-subtle ${
                      isContended ? 'bg-[var(--color-status-fail)]/8' : 'hover:bg-overlay-hover'
                    }`}
                  >
                    <td className="px-2 py-2 font-semibold text-text-primary break-all">
                      {cell.prefix}
                    </td>
                    <td className="px-2 py-2">
                      <span
                        className={`rounded border px-1.5 py-0.5 text-[10px] uppercase ${
                          cell.exclusive
                            ? 'border-brass/45 bg-brass/15 text-text-primary'
                            : 'border-[var(--color-status-pass)]/40 bg-[var(--color-status-pass)]/10 text-[var(--color-status-pass)]'
                        }`}
                      >
                        {cell.exclusive ? 'exclusive' : 'shared'}
                      </span>
                    </td>
                    <td className="px-2 py-2 text-[var(--color-status-pass)]">@{cell.holder_agent}</td>
                    <td className="px-2 py-2 text-text-secondary">{tierShortLabel(cell.compute_tier)}</td>
                    <td className="px-2 py-2">
                      <span className="rounded border border-border-subtle bg-bg-elevated px-1.5 py-0.5 text-[10px] text-text-secondary uppercase">
                        {cell.workspace_mode}
                      </span>
                    </td>
                    <td className="px-2 py-2 text-text-secondary">#seq-{cell.acquired_seq}</td>
                    <td className="px-2 py-2">
                      {isContended ? (
                        <span
                          data-testid={`lock-contention-shield-${cell.prefix}`}
                          className="inline-flex items-center gap-1 rounded border border-[var(--color-status-fail)]/55 bg-[var(--color-status-fail)]/12 px-2 py-0.5 text-[10px] text-[var(--color-status-fail)]"
                        >
                          <Icon.shield className="size-3 shrink-0" aria-hidden="true" />
                          <span>
                            [FileLock] wait:{' '}
                            {cell.waiters.length > 0
                              ? cell.waiters.map((w) => `@${w}`).join(', ')
                              : 'contended'}
                          </span>
                        </span>
                      ) : (
                        <span className="text-[10px] text-[var(--color-status-pass)]">clear</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
