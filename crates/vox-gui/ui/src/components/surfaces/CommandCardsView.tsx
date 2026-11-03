import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { sanitizeErrorForToast } from '../../lib/backendGuard';
import type { Toast } from '../../types/tauri';

interface ExecuteOutput {
  exit_code: number;
  stdout: string;
  stderr: string;
}

type CardResult =
  | { kind: 'ok'; out: ExecuteOutput }
  | { kind: 'error'; message: string };

/** A single read-only command rendered as a dashboard card. */
export interface SurfaceCard {
  key: string;
  title: string;
  description: string;
  /** CLI path passed verbatim to `execute_command` (e.g. ['research', 'status']). */
  path: string[];
  /**
   * Extra CLI args forwarded verbatim as `__argv` (e.g. `['--detailed']`).
   * Defaults to `[]` — most cards are arg-free reads; a card that needs a
   * flag (e.g. the GPU Probe card's `--detailed` fit check) sets this.
   */
  argv?: string[];
}

interface CommandCardsViewProps {
  title: string;
  subtitle: string;
  cards: SurfaceCard[];
  pushToast: (item: Toast) => void;
}

/**
 * Generic decorator body: runs a set of arg-free, read-only CLI commands through
 * the shared `execute_command` Tauri path (the runAction seam) on mount and on
 * Refresh, rendering each result in a card. Used by every surface decorator so
 * Scientia / Mens / Populi / Research share one implementation.
 *
 * Not for `vox mens serve`: `execute_command` awaits process exit, and
 * `serve` never exits — see `Models/MensServePanel.tsx` for the supervised
 * child-process pattern that one needs instead.
 */
export function CommandCardsView({ title, subtitle, cards, pushToast }: CommandCardsViewProps) {
  const [results, setResults] = useState<Record<string, CardResult>>({});
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    const next: Record<string, CardResult> = {};
    await Promise.all(
      cards.map(async (card) => {
        try {
          const out = await invoke<ExecuteOutput>('execute_command', {
            path: card.path,
            args: { __argv: card.argv ?? [] },
          });
          next[card.key] = { kind: 'ok', out };
        } catch (err) {
          next[card.key] = { kind: 'error', message: sanitizeErrorForToast(err) };
        }
      })
    );
    setResults(next);
    setLoading(false);
    const failures = Object.values(next).filter(
      (r) => r.kind === 'error' || (r.kind === 'ok' && r.out.exit_code !== 0)
    ).length;
    if (failures > 0) {
      pushToast({
        tone: 'warn',
        title,
        body: `${failures} of ${cards.length} reads did not complete cleanly`,
        cause: 'backend-error',
      });
    }
  }, [cards, title, pushToast]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="font-display text-lg tracking-wider text-text-primary uppercase">{title}</h2>
          <p className="font-mono text-xs text-text-muted">{subtitle}</p>
        </div>
          <button
            type="button"
            className="rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-1.5 text-xs uppercase tracking-wider hover:bg-overlay-subtle disabled:opacity-40"
            disabled={loading}
            onClick={refresh}
          >
            {loading ? 'Refreshing…' : 'Refresh'}
          </button>
        </div>
        <div className="grid gap-3 lg:grid-cols-2">
          {cards.map((card) => {
            const r = results[card.key];
            return (
              <div key={card.key} className="rounded-xl border border-border-subtle bg-bg-surface/80 p-3.5 shadow-sm">
                <div className="mb-1 flex items-center justify-between gap-2">
                  <span className="font-display text-sm tracking-wide text-text-primary">{card.title}</span>
                  <span className="rounded border border-border-subtle bg-bg-base px-1.5 py-0.5 font-mono text-[10px] text-brass">
                    vox {card.path.join(' ')}
                  </span>
                </div>
                <div className="mb-2 text-[10px] uppercase tracking-wider text-text-muted">{card.description}</div>
                {!r && <div className="font-mono text-xs text-text-muted">Loading…</div>}
                {r && r.kind === 'error' && (
                  <div className="font-mono text-xs text-red-400">{r.message}</div>
                )}
                {r && r.kind === 'ok' && (
                  <>
                    <div
                      className={`mb-1.5 flex items-center justify-between font-mono text-[10px] ${
                        r.out.exit_code === 0 ? 'text-emerald-400' : 'text-red-400'
                      }`}
                    >
                      <span>exit {r.out.exit_code} · vox {card.path.join(' ')}</span>
                      <span className="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[9px] text-emerald-300">
                        {r.out.exit_code === 0 ? 'OK' : 'ERR'}
                      </span>
                    </div>
                    <pre className="max-h-56 overflow-auto whitespace-pre-wrap rounded-lg border border-border-subtle bg-bg-base p-2.5 font-mono text-[11px] text-text-primary">
                      {[r.out.stdout, r.out.stderr].filter(Boolean).join('\n').trim() || '(no output)'}
                    </pre>
                  </>
                )}
              </div>
            );
          })}
        </div>
        <div
          data-testid="command-cards-telemetry-deck"
          className="grid grid-cols-1 gap-3 pt-1 sm:grid-cols-3"
        >
          <div className="rounded-xl border border-border-subtle bg-bg-surface/70 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-brass uppercase">Execution Seam</span>
              <span className="rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[9px] text-brass">
                Tier-3 CLI
              </span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Routes read-only diagnostics through <code className="text-cyan">execute_command</code> with zero background daemon polling overhead.
            </p>
          </div>
          <div className="rounded-xl border border-border-subtle bg-bg-surface/70 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-cyan uppercase">Probe Status</span>
              <span className="font-mono text-[10px] text-emerald-300">
                {Object.values(results).filter((r) => r.kind === 'ok' && r.out.exit_code === 0).length}/{cards.length} healthy
              </span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Each card captures stdout/stderr exit telemetry and surfaces non-zero exit codes via non-blocking toast alerts.
            </p>
          </div>
          <div className="rounded-xl border border-border-subtle bg-bg-surface/70 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">CLI Parity</span>
              <span className="font-mono text-[9px] text-text-muted">SSOT Sync</span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Need additional flags or interactive parameters? Open the full Command Catalog (<code className="text-brass">⌘K</code>) to configure arguments.
            </p>
          </div>
        </div>
      </section>
  );
}
