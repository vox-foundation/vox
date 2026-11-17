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
      className="flex min-h-0 min-w-0 flex-col gap-2.5 rounded-xl border border-[#D4AF37]/30 bg-[#0B0D12] p-3 text-slate-100 shadow-inner"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#D4AF37]/20 pb-2">
        <div className="flex items-center gap-2">
          <Icon.message className="size-4 text-[#D4AF37]" aria-hidden="true" />
          <span className="font-display text-xs font-bold uppercase tracking-[0.12em] text-[#F3E5AB]">
            Shared #seq Sequence Board
          </span>
          <span className="rounded border border-[#D4AF37]/30 bg-slate-900 px-1.5 py-0.5 font-mono text-[9px] text-slate-300">
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
                    ? 'border-[#D4AF37] bg-[#D4AF37]/20 text-[#F3E5AB]'
                    : 'border-slate-700 bg-slate-900 text-slate-400 hover:border-slate-500 hover:text-slate-200'
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
          <div className="rounded-lg border border-dashed border-slate-700 bg-slate-950/60 p-4 text-center text-xs text-slate-400">
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
                    ? 'border-rose-500/60 bg-rose-950/25'
                    : 'border-[#D4AF37]/25 bg-slate-950/85'
                }`}
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <span className="rounded border border-[#D4AF37]/50 bg-[#D4AF37]/15 px-1.5 py-0.5 text-[10px] font-bold text-[#F3E5AB]">
                      #seq-{entry.seq}
                    </span>
                    <span className="rounded border border-slate-700 bg-slate-900 px-1.5 py-0.5 text-[9px] text-slate-300">
                      {chNorm}
                    </span>
                    <span className="font-bold text-[#F3E5AB]">
                      @{entry.sender.replace(/^@/, '')}
                    </span>
                    {entry.is_watchdog_alert && (
                      <span
                        data-slot="concilium-watchdog-badge"
                        className="inline-flex items-center gap-1 rounded border border-rose-400/70 bg-rose-500/20 px-1.5 py-0.5 text-[9px] font-bold uppercase text-rose-200"
                      >
                        <Icon.alert className="size-2.5 text-rose-300" aria-hidden="true" />
                        <span>WATCHDOG ALERT</span>
                      </span>
                    )}
                    {entry.receipt_badge && (
                      <span
                        data-slot="concilium-receipt-badge"
                        className="inline-flex items-center gap-1 rounded border border-emerald-400/50 bg-emerald-500/15 px-1.5 py-0.5 text-[9px] font-bold text-emerald-200"
                      >
                        <Icon.check className="size-2.5 text-emerald-300" aria-hidden="true" />
                        <span>{entry.receipt_badge}</span>
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-1.5 text-[9px] text-slate-400">
                    <span>jj:{entry.workspace_mode}</span>
                    <span title={entry.traceparent}>tp:{entry.traceparent.slice(0, 16)}</span>
                  </div>
                </div>

                <p className="mt-1.5 leading-relaxed text-slate-200">{entry.body}</p>

                {(entry.mentions.length > 0 || entry.file_lock_prefix) && (
                  <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
                    {entry.mentions.map((m, mIdx) => (
                      <span
                        key={`${entry.seq}-${m}-${mIdx}`}
                        data-slot="concilium-mention-chip"
                        className="rounded border border-[#D4AF37]/45 bg-[#D4AF37]/15 px-1.5 py-0.5 text-[9px] font-bold text-[#F3E5AB]"
                      >
                        @{m.replace(/^@/, '')}
                      </span>
                    ))}
                    {entry.file_lock_prefix && (
                      <span className="inline-flex items-center gap-1 rounded border border-rose-500/40 bg-rose-950/35 px-1.5 py-0.5 text-[9px] text-rose-200">
                        <Icon.lockKeyhole className="size-2.5 text-rose-300" aria-hidden="true" />
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
        className="flex items-center gap-2 border-t border-[#D4AF37]/20 pt-2"
      >
        <input
          type="text"
          data-slot="input"
          data-testid="concilium-board-steer-input"
          aria-label="Steer @agent directive"
          value={steerInput}
          onChange={(e) => setSteerInput(e.target.value)}
          placeholder={`@steer @${(selectedAgentName ?? selectedAgentId ?? 'coordinator').replace(/^@/, '')} on ${normFilter === '#all' ? '#main' : normFilter}...`}
          className="min-w-0 flex-1 rounded-md border border-[#D4AF37]/35 bg-slate-950 px-2.5 py-1.5 font-mono text-xs text-slate-100 placeholder:text-slate-500 focus:border-[#D4AF37] focus:outline-hidden"
        />
        <button
          type="submit"
          tabIndex={0}
          data-slot="button"
          data-testid="concilium-board-steer-submit"
          disabled={!steerInput.trim()}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-[#D4AF37]/60 bg-[#D4AF37]/20 px-3 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-[#F3E5AB] hover:bg-[#D4AF37]/30 disabled:cursor-not-allowed disabled:opacity-45"
        >
          <Icon.send className="size-3" aria-hidden="true" />
          <span>Post @steer</span>
        </button>
      </form>
    </section>
  );
}
