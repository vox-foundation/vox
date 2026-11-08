import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { sanitizeErrorForToast } from '../../lib/backendGuard';
import type { Toast } from '../../types/tauri';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../ui/RomanSurfaceHeader';
import { RomanMedallion } from '../ui/RomanTooltip';
import { Icon } from '../ui/Icons';

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

  const healthyCount = Object.values(results).filter(
    (r) => r.kind === 'ok' && r.out.exit_code === 0,
  ).length;

  return (
    <section className="space-y-4">
      <RomanSurfaceHeader
        domainTag="COMPUTE · RUNTIME"
        title={title}
        subtitle={subtitle}
        waxSealBadge={loading ? 'PROBING' : 'VERIFIED'}
        icon={<Icon.bolt className="size-5 text-[#F3E5AB]" aria-hidden="true" />}
        gauges={[
          {
            label: 'Probes',
            valueText: `${healthyCount}/${cards.length} OK`,
            ratio: cards.length > 0 ? healthyCount / cards.length : 1,
            tone: 'emerald',
          },
          {
            label: 'Seam',
            valueText: 'Tier-3 CLI',
            ratio: 0.9,
            tone: 'gold',
          },
        ]}
        actions={
          <button
            type="button"
            className="rounded-lg border border-[#D4AF37]/55 bg-linear-to-b from-[#332613] to-[#1A1309] px-3 py-1.5 font-display text-xs font-semibold uppercase tracking-wider text-[#F3E5AB] shadow-sm hover:border-[#F3E5AB] disabled:opacity-40"
            disabled={loading}
            onClick={refresh}
          >
            {loading ? 'Refreshing…' : 'Refresh'}
          </button>
        }
      />
      <div className="grid gap-3 lg:grid-cols-2">
        {cards.map((card) => {
          const r = results[card.key];
          return (
            <div
              key={card.key}
              style={{
                backgroundImage:
                  'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(238,229,211,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
                backgroundSize: 'cover',
                backgroundPosition: 'center',
              }}
              className="rounded-xl border-2 border-[#C59B27] p-4 text-[#1C140C] shadow-[0_8px_22px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)]"
            >
              <div className="mb-1.5 flex items-center justify-between gap-2 border-b border-[#9E7B2F]/45 pb-2">
                <div className="flex items-center gap-2">
                  <RomanMedallion active size="sm">
                    <Icon.command className="size-3.5 text-[#F3E5AB]" aria-hidden="true" />
                  </RomanMedallion>
                  <span className="font-display text-sm font-bold tracking-wide text-[#1C140C]">
                    {card.title}
                  </span>
                </div>
                <span className="rounded border border-[#8C6825] bg-[#251C0E] px-2 py-0.5 font-mono text-[10px] font-semibold text-[#F3E5AB]">
                  vox {card.path.join(' ')}
                </span>
              </div>
              <div className="mb-2.5 text-[10.5px] font-bold uppercase tracking-wider text-[#5C441E]">
                {card.description}
              </div>
              {!r && <div className="font-mono text-xs text-[#5C441E]">Loading…</div>}
              {r && r.kind === 'error' && (
                <div className="font-mono text-xs text-[#881B1B] font-semibold">{r.message}</div>
              )}
              {r && r.kind === 'ok' && (
                <>
                  <div
                    className={`mb-2 flex items-center justify-between font-mono text-[10px] font-semibold ${
                      r.out.exit_code === 0 ? 'text-[#3E4A16]' : 'text-[#881B1B]'
                    }`}
                  >
                    <span>exit {r.out.exit_code} · vox {card.path.join(' ')}</span>
                    <RomanWaxSealBadge
                      label={r.out.exit_code === 0 ? 'OK' : 'ERR'}
                      tone={r.out.exit_code === 0 ? 'emerald' : 'crimson'}
                    />
                  </div>
                  <pre className="max-h-56 overflow-auto whitespace-pre-wrap rounded-lg border-2 border-[#9E7B2F] bg-[#18120B] p-3 font-mono text-[11px] text-[#F5F0E6] shadow-inner">
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
        <div
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(249,245,235,0.95) 0%, rgba(235,224,205,0.94) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="rounded-xl border-2 border-[#C59B27] p-3.5 text-[#1C140C] shadow-[0_6px_16px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.9)]"
        >
          <div className="flex items-center justify-between border-b border-[#9E7B2F]/40 pb-1.5">
            <span className="font-display text-[10.5px] font-bold tracking-widest text-[#1C140C] uppercase">
              Execution Seam
            </span>
            <RomanWaxSealBadge label="Tier-3 CLI" tone="gold" />
          </div>
          <p className="mt-1.5 text-[11px] font-medium leading-relaxed text-[#2B2013]">
            Routes read-only diagnostics through <code className="rounded bg-[#251C0E] px-1 py-0.5 text-[#F3E5AB]">execute_command</code> with zero background daemon polling overhead.
          </p>
        </div>
        <div
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(249,245,235,0.95) 0%, rgba(235,224,205,0.94) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="rounded-xl border-2 border-[#C59B27] p-3.5 text-[#1C140C] shadow-[0_6px_16px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.9)]"
        >
          <div className="flex items-center justify-between border-b border-[#9E7B2F]/40 pb-1.5">
            <span className="font-display text-[10.5px] font-bold tracking-widest text-[#1C140C] uppercase">
              Probe Status
            </span>
            <RomanWaxSealBadge label={`${healthyCount}/${cards.length} healthy`} tone="emerald" />
          </div>
          <p className="mt-1.5 text-[11px] font-medium leading-relaxed text-[#2B2013]">
            Each card captures stdout/stderr exit telemetry and surfaces non-zero exit codes via non-blocking toast alerts.
          </p>
        </div>
        <div
          style={{
            backgroundImage:
              'linear-gradient(180deg, rgba(249,245,235,0.95) 0%, rgba(235,224,205,0.94) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="rounded-xl border-2 border-[#C59B27] p-3.5 text-[#1C140C] shadow-[0_6px_16px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.9)]"
        >
          <div className="flex items-center justify-between border-b border-[#9E7B2F]/40 pb-1.5">
            <span className="font-display text-[10.5px] font-bold tracking-widest text-[#1C140C] uppercase">
              CLI Parity
            </span>
            <RomanWaxSealBadge label="SSOT Sync" />
          </div>
          <p className="mt-1.5 text-[11px] font-medium leading-relaxed text-[#2B2013]">
            Need additional flags or interactive parameters? Open the full Command Catalog (<code className="rounded bg-[#251C0E] px-1 py-0.5 text-[#F3E5AB]">⌘K</code>) to configure arguments.
          </p>
        </div>
      </div>
      <div
        style={{
          backgroundImage:
            'linear-gradient(180deg, rgba(250,246,237,0.96) 0%, rgba(236,226,207,0.95) 100%), url(/assets/romanesque/travertine-parchment-tablet.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="rounded-xl border-2 border-[#C59B27] p-4 text-[#1C140C] shadow-[0_8px_22px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)]"
      >
        <div className="mb-2.5 flex flex-wrap items-center justify-between gap-2 border-b border-[#9E7B2F]/45 pb-2">
          <div className="flex items-center gap-2">
            <RomanMedallion active size="sm">
              <Icon.link className="size-3.5 text-[#F3E5AB]" aria-hidden="true" />
            </RomanMedallion>
            <div>
              <div className="font-display text-xs font-bold uppercase tracking-[0.14em] text-[#1C140C]">
                Distributed Compute &amp; Runtime Topology Ledger
              </div>
              <div className="font-serif text-[11px] text-[#4A3718]">
                Active QUIC mesh transport, Ed25519 peer attestation, and local daemon supervision
              </div>
            </div>
          </div>
          <RomanWaxSealBadge label="QUIC MESH · ED25519" tone="gold" />
        </div>
        <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
          <div className="rounded-lg border border-[#9E7B2F] bg-[#241B11] px-3 py-2 text-[#F6F0E4]">
            <div className="font-display text-[9px] uppercase tracking-widest text-[#D4AF37]">Transport Protocol</div>
            <div className="mt-0.5 font-mono text-xs font-semibold text-[#F3E5AB]">QUIC + TLS 1.3</div>
          </div>
          <div className="rounded-lg border border-[#9E7B2F] bg-[#241B11] px-3 py-2 text-[#F6F0E4]">
            <div className="font-display text-[9px] uppercase tracking-widest text-[#D4AF37]">Peer Attestation</div>
            <div className="mt-0.5 font-mono text-xs font-semibold text-[#F3E5AB]">Ed25519 Signed</div>
          </div>
          <div className="rounded-lg border border-[#9E7B2F] bg-[#241B11] px-3 py-2 text-[#F6F0E4]">
            <div className="font-display text-[9px] uppercase tracking-widest text-[#D4AF37]">Spoke Dispatch</div>
            <div className="mt-0.5 font-mono text-xs font-semibold text-[#F3E5AB]">8-Way QLoRA</div>
          </div>
          <div className="rounded-lg border border-[#9E7B2F] bg-[#241B11] px-3 py-2 text-[#F6F0E4]">
            <div className="font-display text-[9px] uppercase tracking-widest text-[#D4AF37]">Sandbox Boundary</div>
            <div className="mt-0.5 font-mono text-xs font-semibold text-[#F3E5AB]">Clavis Guarded</div>
          </div>
        </div>
      </div>
    </section>
  );
}
