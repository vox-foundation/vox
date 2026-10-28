import React from 'react';
import { LudusProfile, xpBarPct } from '../../../lib/ludus';

export function LudusHud({ profile }: { profile: LudusProfile }) {
  return (
    <div className="rounded-xl border border-border-subtle bg-overlay-subtle p-4">
      <div className="flex items-baseline justify-between">
        <div className="font-display text-sm tracking-wider text-brass uppercase">{profile.full_title}</div>
        <div className="font-mono text-[11px] text-text-muted">Lv {profile.level} · prestige {profile.prestige_level}</div>
      </div>
      <div
        className="mt-3 h-2 overflow-hidden rounded-full bg-overlay-subtle"
        role="progressbar"
        aria-label="XP progress"
        aria-valuenow={Math.round(Math.max(0, Math.min(1, profile.xp_progress)) * 100)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div className="h-full rounded-full bg-brass/70 transition-all" style={{ width: xpBarPct(profile.xp_progress) }} />
      </div>
      <div className="mt-1 flex justify-between font-mono text-[10px] text-text-muted">
        <span>{profile.xp} XP</span>
        <span>{profile.xp_to_next_level} to next</span>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 text-[11px] sm:grid-cols-4">
        <Stat
          label="Crystals"
          value={String(profile.crystals)}
          icon={
            <svg viewBox="0 0 12 12" className="size-3 text-cyan" fill="currentColor" aria-hidden="true">
              <path d="M6 1L10.5 4.5L6 11L1.5 4.5L6 1Z" />
            </svg>
          }
        />
        <Stat label="Lumens" value={`${profile.lumens}`} />
        <Stat label="Energy" value={`${profile.energy}/${profile.max_energy}`} />
        <Stat
          label="Streak"
          value={String(profile.current_streak)}
          icon={
            <svg viewBox="0 0 12 12" className="size-3 text-amber-400" fill="currentColor" aria-hidden="true">
              <path d="M6.5 1C6.5 1 9.5 3.8 9.5 6.8C9.5 9.1 7.8 11 5.8 11C3.8 11 2.5 9.3 2.5 7.2C2.5 5.6 3.6 4.2 3.6 4.2C3.6 4.2 3.9 5.5 4.8 5.5C4.5 3.6 6.5 1 6.5 1Z" />
            </svg>
          }
        />
      </div>
    </div>
  );
}

function Stat({ label, value, icon }: { label: string; value: string; icon?: React.ReactNode }) {
  return (
    <div className="rounded-lg bg-overlay-subtle px-2 py-1.5 ring-1 ring-white/5">
      <div className="text-[9px] uppercase tracking-wide text-text-muted">{label}</div>
      <div className="inline-flex items-center gap-1 font-mono text-text-secondary">
        <span>{value}</span>
        {icon}
      </div>
    </div>
  );
}
