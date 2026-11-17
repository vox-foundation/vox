import React, { useState } from 'react';
import type { SequenceBoardEntry } from '../../../types/concilium';
import { Icon } from '../../ui/Icons';

export const CONCILIUM_CHANNELS = [
  '#all',
  '#main',
  '#parser',
  '#verify',
  '#scientia',
  '#ops',
] as const;

export type ConciliumChannelFilter = (typeof CONCILIUM_CHANNELS)[number] | string;

export interface SharedSequenceBoardProps {
  entries: SequenceBoardEntry[];
  selectedChannel: string;
  onSelectChannel: (channel: string) => void;
  selectedAgentId?: string | null;
  selectedAgentName?: string | null;
  onSteerSubmit: (targetAgentId: string, directive: string, channel: string) => Promise<void> | void;
}

function normalizeChannel(ch: string): string {
  const trimmed = ch.trim().toLowerCase();
  if (!trimmed) return '#main';
  return trimmed.startsWith('#') ? trimmed : `#${trimmed}`;
}

export function SharedSequenceBoard({
  entries,
  selectedChannel,
  onSelectChannel,
  selectedAgentId,
  selectedAgentName,
  onSteerSubmit,
}: SharedSequenceBoardProps) {
  const [steerInput, setSteerInput] = useState('');

  const normFilter = normalizeChannel(selectedChannel);
  const filteredEntries =
    normFilter === '#all'
      ? entries
      : entries.filter((e) => normalizeChannel(e.channel) === normFilter);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = steerInput.trim();
    if (!trimmed) return;
    const targetId = selectedAgentId ?? 'coordinator';
    const targetChannel = normFilter === '#all' ? '#main' : normFilter;
    void onSteerSubmit(targetId, trimmed, targetChannel);
    setSteerInput('');
  };

  return (
    <section
      data-slot="concilium-seq-board"
      data-testid="concilium-seq-board"
      aria-label="Channel-filtered #seq sequence board"
      className="flex min-h-0 min-w-0 flex-col gap-2.5 rounded-xl border border-border-strong bg-bg-surface p-3 text-text-primary shadow-inner"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2">
        <div className="flex items-center gap-2">
          <Icon.message className="size-4 text-[var(--color-accent-default)]" aria-hidden="true" />
          <span className="font-display text-xs font-bold uppercase tracking-[0.12em] text-text-primary">
            Shared #seq Sequence Board
          </span>
          <span className="rounded border border-border-subtle bg-bg-elevated px-1.5 py-0.5 font-mono text-[9px] text-text-secondary">
            {filteredEntries.length} posts
          </span>
        </div>

        <div
          data-slot="concilium-channel-filters"
          className="flex flex-wrap items-center gap-1"
          role="group"
          aria-label="Sequence board channel filters"
        >
          {CONCILIUM_CHANNELS.map((ch) => {
            const active = normFilter === ch;
            return (
              <button
                key={ch}
                type="button"
                tabIndex={0}
                data-slot="button"
                data-testid={`concilium-channel-${ch.slice(1)}`}
                aria-pressed={active}
                onClick={() => onSelectChannel(ch)}
                className={`rounded-md border px-2 py-0.5 font-mono text-[10px] font-semibold transition-colors ${
                  active
                    ? 'border-brass bg-brass/20 text-text-primary'
                    : 'border-border-subtle bg-bg-elevated text-text-muted hover:border-border-strong hover:text-text-primary'
                }`}
              >
                {ch}
              </button>
            );
          })}
        </div>
      </div>

      <div
        data-testid="concilium-seq-entries"
        className="max-h-60 min-h-[120px] space-y-2 overflow-y-auto pr-1 font-mono text-[11px]"
      >
        {filteredEntries.length === 0 ? (
          <div className="rounded-lg border border-dashed border-border-subtle bg-overlay-subtle p-4 text-center text-xs text-text-muted">
            No #seq messages in {normFilter} yet.
          </div>
        ) : (
          filteredEntries.map((entry, idx) => {
            const chNorm = normalizeChannel(entry.channel);
            return (
              <article
                key={`seq-${entry.seq}-${idx}`}
                data-slot="concilium-seq-entry"
                data-testid={`concilium-seq-entry-${entry.seq}`}
                className={`rounded-lg border p-2.5 ${
                  entry.is_watchdog_alert
                    ? 'border-[var(--color-status-fail)]/60 bg-[var(--color-status-fail)]/10'
                    : 'border-border-subtle bg-bg-elevated'
                }`}
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <span className="rounded border border-brass/50 bg-brass/15 px-1.5 py-0.5 text-[10px] font-bold text-text-primary">
                      #seq-{entry.seq}
                    </span>
                    <span className="rounded border border-border-subtle bg-bg-surface px-1.5 py-0.5 text-[9px] text-text-secondary">
                      {chNorm}
                    </span>
                    <span className="font-bold text-text-primary">
                      @{entry.sender.replace(/^@/, '')}
                    </span>
                    {entry.is_watchdog_alert && (
                      <span
                        data-slot="concilium-watchdog-badge"
                        className="inline-flex items-center gap-1 rounded border border-[var(--color-status-fail)]/60 bg-[var(--color-status-fail)]/15 px-1.5 py-0.5 text-[9px] font-bold uppercase text-[var(--color-status-fail)]"
                      >
                        <Icon.alert className="size-2.5" aria-hidden="true" />
                        <span>WATCHDOG ALERT</span>
                      </span>
                    )}
                    {entry.receipt_badge && (
                      <span
                        data-slot="concilium-receipt-badge"
                        className="inline-flex items-center gap-1 rounded border border-[var(--color-status-pass)]/50 bg-[var(--color-status-pass)]/12 px-1.5 py-0.5 text-[9px] font-bold text-[var(--color-status-pass)]"
                      >
                        <Icon.check className="size-2.5" aria-hidden="true" />
                        <span>{entry.receipt_badge}</span>
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-1.5 text-[9px] text-text-muted">
                    <span>jj:{entry.workspace_mode}</span>
                    <span title={entry.traceparent}>tp:{entry.traceparent.slice(0, 16)}</span>
                  </div>
                </div>

                <p className="mt-1.5 leading-relaxed text-text-secondary">{entry.body}</p>

                {(entry.mentions.length > 0 || entry.file_lock_prefix) && (
                  <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
                    {entry.mentions.map((m, mIdx) => (
                      <span
                        key={`${entry.seq}-${m}-${mIdx}`}
                        data-slot="concilium-mention-chip"
                        className="rounded border border-brass/45 bg-brass/15 px-1.5 py-0.5 text-[9px] font-bold text-text-primary"
                      >
                        @{m.replace(/^@/, '')}
                      </span>
                    ))}
                    {entry.file_lock_prefix && (
                      <span className="inline-flex items-center gap-1 rounded border border-[var(--color-status-fail)]/40 bg-[var(--color-status-fail)]/10 px-1.5 py-0.5 text-[9px] text-[var(--color-status-fail)]">
                        <Icon.lockKeyhole className="size-2.5" aria-hidden="true" />
                        <span>[FileLock] {entry.file_lock_prefix}</span>
                      </span>
                    )}
                  </div>
                )}
              </article>
            );
          })
        )}
      </div>

      <form
        onSubmit={handleSubmit}
        data-slot="concilium-steer-bar"
        className="flex items-center gap-2 border-t border-border-subtle pt-2"
      >
        <input
          type="text"
          data-slot="input"
          data-testid="concilium-board-steer-input"
          aria-label="Steer @agent directive"
          value={steerInput}
          onChange={(e) => setSteerInput(e.target.value)}
          placeholder={`@steer @${(selectedAgentName ?? selectedAgentId ?? 'coordinator').replace(/^@/, '')} on ${normFilter === '#all' ? '#main' : normFilter}...`}
          className="min-w-0 flex-1 rounded-md border border-border-strong bg-bg-elevated px-2.5 py-1.5 font-mono text-xs text-text-primary placeholder:text-text-muted focus:border-brass focus:outline-hidden"
        />
        <button
          type="submit"
          tabIndex={0}
          data-slot="button"
          data-testid="concilium-board-steer-submit"
          disabled={!steerInput.trim()}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-brass/60 bg-brass/20 px-3 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-text-primary hover:bg-brass/30 disabled:cursor-not-allowed disabled:opacity-45"
        >
          <Icon.send className="size-3" aria-hidden="true" />
          <span>Post @steer</span>
        </button>
      </form>
    </section>
  );
}
