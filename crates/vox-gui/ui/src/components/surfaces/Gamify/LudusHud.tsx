import React from 'react';
import { LudusProfile, xpBarPct } from '../../../lib/ludus';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

export function LudusHud({ profile }: { profile: LudusProfile }) {
  const xpPct = Math.round(Math.max(0, Math.min(1, profile.xp_progress)) * 100);

  return (
    <div
      data-testid="ludus-hud"
      className="rounded-xl border border-[#C59B27]/45 bg-bg-surface p-3.5 shadow-sm text-text-primary space-y-2.5"
    >
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2.5 min-w-0">
          <RomanMedallion active size="sm">
            <Icon.laurelWreath className="size-3.5 text-text-primary" />
          </RomanMedallion>
          <div className="truncate font-serif text-sm font-bold tracking-wider text-text-primary uppercase">
            {profile.full_title}
          </div>
        </div>
        <div className="flex items-center gap-2 shrink-0 font-mono text-[11px] text-text-secondary">
          <span className="rounded border border-[#D4AF37]/30 bg-bg-elevated px-2 py-0.5 font-semibold text-text-primary">
            Level {profile.level}
          </span>
          <span className="text-text-muted">·</span>
          <span>Prestige {profile.prestige_level}</span>
        </div>
      </div>

      <RomanTooltip
        title="Engineering Experience (XP)"
        purpose="Accumulated by passing compiler gates, writing verified code, and completing engineering quests."
        statusBadge={`${profile.xp} / ${profile.xp + profile.xp_to_next_level} XP`}
        actionHint="Earn XP by compiling code and clearing harness gates"
        className="block w-full"
      >
        <div className="space-y-1 cursor-default">
          <div
            className="h-2 overflow-hidden rounded-full bg-bg-elevated border border-border-subtle"
            role="progressbar"
            aria-label="XP progress"
            aria-valuenow={xpPct}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <div
              className="h-full rounded-full bg-gradient-to-r from-[#B4881A] via-[#D4AF37] to-[#F3E5AB] transition-[width] duration-300 shadow-[0_0_8px_rgba(212,175,55,0.4)]"
              style={{ width: xpBarPct(profile.xp_progress) }}
            />
          </div>
          <div className="flex justify-between font-mono text-[10px] text-text-secondary">
            <span>{profile.xp.toLocaleString()} XP</span>
            <span>{profile.xp_to_next_level.toLocaleString()} to next level</span>
          </div>
        </div>
      </RomanTooltip>

      <div className="grid grid-cols-2 gap-2 text-[11px] sm:grid-cols-4 pt-1">
        <RomanTooltip
          title="Civilization Crystals"
          purpose="Rare engineering currency awarded for zero-cascade efficiency and optimization thrift."
          statusBadge={`${profile.crystals} ◆`}
        >
          <Stat
            label="Crystals"
            value={String(profile.crystals)}
            icon={<Icon.denariusCoin className="size-3.5 text-brass" />}
          />
        </RomanTooltip>

        <RomanTooltip
          title="Reputation Lumens"
          purpose="Permanent verified reputation score earned across crate guilds and compiler contributions."
          statusBadge={`${profile.lumens} ☼`}
        >
          <Stat
            label="Lumens"
            value={`${profile.lumens}`}
            icon={<Icon.brazierTorch className="size-3.5 text-[#F59E0B]" />}
          />
        </RomanTooltip>

        <RomanTooltip
          title="Focus Energy"
          purpose="Daily stamina allocated for autonomous agent dispatches and sandbox simulations."
          statusBadge={`${profile.energy}/${profile.max_energy}`}
        >
          <Stat
            label="Energy"
            value={`${profile.energy}/${profile.max_energy}`}
            icon={<Icon.astrolabeRadar className="size-3.5 text-brass" />}
          />
        </RomanTooltip>

        <RomanTooltip
          title="Daily Streak"
          purpose="Consecutive daily engineering sessions with green builds and verified commits."
          statusBadge={`${profile.current_streak} days`}
        >
          <Stat
            label="Streak"
            value={String(profile.current_streak)}
            icon={<Icon.waxSeal className="size-3.5 text-[#EF4444]" />}
          />
        </RomanTooltip>
      </div>
    </div>
  );
}

function Stat({ label, value, icon }: { label: string; value: string; icon?: React.ReactNode }) {
  return (
    <div className="flex flex-col justify-between rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-1.5 shadow-xs transition hover:border-[#D4AF37]/40">
      <div className="text-[9px] font-serif uppercase tracking-wider text-text-secondary">{label}</div>
      <div className="mt-0.5 inline-flex items-center gap-1.5 font-mono text-sm font-semibold text-text-primary">
        <span>{value}</span>
        {icon}
      </div>
    </div>
  );
}
