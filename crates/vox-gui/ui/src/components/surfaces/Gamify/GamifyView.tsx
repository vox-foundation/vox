import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useLabel } from '../../../hooks/useLanguage';
import { LudusSandbox } from '../../gamify/LudusSandbox';
import { HarnessRiverPanel } from '../Harness/HarnessRiverPanel';
import { invoke } from '@tauri-apps/api/core';
import { LudusProfile } from '../../../lib/ludus';
import { LudusHud } from './LudusHud';
import { LeaderboardArenaPanel, type VerifiedLeaderboardEntry } from './LeaderboardArenaPanel';
import { GAMIFY_POLL_MS } from '../../../config/constants';
import type { Toast } from '../../../types/tauri';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

export type ViewId = string;

interface GamifyViewProps {
  pushToast: (item: Toast) => void;
  onNavigate?: (view: ViewId) => void;
}

interface LudusNotification {
  id: string;
  level: 'ok' | 'warn' | 'info';
  title: string;
  message: string;
  created_at: number;
  kind: string;
}

interface LeaderboardEntry {
  rank: number;
  user_id: string;
  level: number;
  score: number;
}

interface Companion {
  id: string;
  name: string;
  description: string | null;
  language: string;
  mood: string;
  health: number;
  max_health: number;
  energy: number;
  max_energy: number;
  code_quality: number;
  last_active: number;
  svg: string;
}

interface Quest {
  id: string;
  quest_type: string;
  description: string;
  hint: string;
  target: number;
  progress: number;
  xp_reward: number;
  crystal_reward: number;
  completed: boolean;
  status: string;
  expires_at: number;
}

type GamifySubtab = 'all' | 'urbs' | 'collegium' | 'arena';

export function GamifyView({ pushToast, onNavigate }: GamifyViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [profile, setProfile] = useState<LudusProfile | null>(null);
  const [notes, setNotes] = useState<LudusNotification[]>([]);
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [companions, setCompanions] = useState<Companion[]>([]);
  const [quests, setQuests] = useState<Quest[]>([]);
  const [loading, setLoading] = useState(false);
  const [simMode, setSimMode] = useState<'urbs' | 'river'>('urbs');
  const [subtab, setSubtab] = useState<GamifySubtab>('urbs');

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [p, n, lb, comp, q] = await Promise.all([
        invoke<LudusProfile>('get_ludus_profile'),
        invoke<LudusNotification[]>('list_ludus_notifications', { limit: 20 }),
        invoke<LeaderboardEntry[]>('list_gamify_leaderboard', { limit: 10 }),
        invoke<Companion[]>('list_gamify_companions'),
        invoke<Quest[]>('list_gamify_quests'),
      ]);
      setProfile(p);
      setNotes(n ?? []);
      setLeaderboard(lb ?? []);
      setCompanions(comp ?? []);
      setQuests(q ?? []);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Ludus load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoading(false);
    }
  }, [pushToast]);

  useEffect(() => {
    refresh();
    if (embedded) return;
    const id = setInterval(refresh, GAMIFY_POLL_MS);
    return () => clearInterval(id);
  }, [refresh, embedded]);

  const ack = async (id: string) => {
    try {
      await invoke('ack_ludus_notification', { notificationId: id });
      setNotes(curr => curr.filter(x => x.id !== id));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Ack failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const showUrbs = subtab === 'all' || subtab === 'urbs';
  const showCollegium = subtab === 'all' || subtab === 'collegium';
  const showArena = subtab === 'all' || subtab === 'arena';

  const renderNotifications = () => (
    <div className="vox-marble-tablet rounded-xl border border-[#C59B27]/75 bg-[#FAF6EE] p-3 space-y-2 text-[#1C140C]">
      <div className="flex items-center gap-2 border-b border-[#C59B27]/40 pb-1.5 font-display text-[11px] font-bold uppercase tracking-wider text-[#8C6214]">
        <Icon.scrollTabula className="size-3.5 text-[#8C6214]" />
        <span>Imperial Notices &amp; Dispatches</span>
      </div>
      {notes.length === 0 ? (
        <div className="rounded-lg border border-border-subtle bg-bg-elevated p-2.5 text-[11px] text-text-secondary">
          No unread notifications.
        </div>
      ) : (
        <ul className="space-y-2">
          {notes.map(n => (
            <li
              key={n.id}
              className="flex items-start justify-between gap-3 rounded-lg border border-border-subtle bg-bg-elevated p-2.5 shadow-xs"
            >
              <div className="min-w-0">
                <div className="font-display text-[12px] font-semibold text-text-primary">{n.title}</div>
                <div className="text-[11px] text-text-secondary">{n.message}</div>
              </div>
              <RomanTooltip
                title="Acknowledge Notice"
                purpose="Marks this imperial notice as read and clears it from your active inbox."
                actionHint="Click to acknowledge notice"
              >
                <button
                  type="button"
                  onClick={() => ack(n.id)}
                  aria-label={`Acknowledge ${n.title}`}
                  className="shrink-0 rounded-md border border-border-strong bg-bg-surface px-2.5 py-1 font-mono text-[10px] font-semibold text-text-primary hover:border-(--color-accent-default) transition"
                >
                  Ack
                </button>
              </RomanTooltip>
            </li>
          ))}
        </ul>
      )}
    </div>
  );

  const renderCompanions = () => (
    <div className="vox-marble-tablet rounded-xl border border-[#C59B27]/75 bg-[#FAF6EE] p-3 space-y-2.5 text-[#1C140C]">
      <div className="flex items-center gap-2 border-b border-[#C59B27]/40 pb-1.5 font-display text-[11px] font-bold uppercase tracking-wider text-[#8C6214]">
        <Icon.gladiusShield className="size-3.5 text-[#8C6214]" />
        <span>Companion Assistants &amp; Familiars</span>
      </div>
      {companions.length === 0 ? (
        <div className="rounded-lg border border-[#C59B27]/40 bg-[#F4EDE0] p-2.5 text-[11px] text-[#5C4428]">
          No companions yet.
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {companions.map((c, idx) => {
            const CompanionIcon =
              idx % 3 === 0
                ? Icon.gladiusShield
                : idx % 3 === 1
                  ? Icon.brazierTorch
                  : Icon.romanGalley;
            return (
              <div
                key={c.id}
                className="rounded-xl border border-[#C59B27]/65 bg-linear-to-b from-[#FFFDF9] to-[#F2E9D8] p-3 shadow-sm"
              >
                <div className="relative mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-full border-2 border-[#C59B27] bg-bg-elevated shadow-[0_0_12px_rgba(212,175,55,0.28)]">
                  <img
                    src="/assets/romanesque/wax-seal-medallion.jpg"
                    alt=""
                    aria-hidden="true"
                    className="absolute inset-0 h-full w-full rounded-full object-cover opacity-35 mix-blend-luminosity"
                  />
                  <CompanionIcon className="relative z-10 size-6 text-brass" />
                </div>
                <div className="truncate text-center font-display text-[12px] font-bold text-[#1C140C]">
                  {c.name}
                </div>
                <div className="text-center font-mono text-[10px] uppercase tracking-wider text-[#5C4428]">
                  {c.mood} · {c.language}
                </div>
                <div className="mt-2 space-y-1.5">
                  <Bar label="HP" value={c.health} max={c.max_health} tone="bg-[#7B8C32]" />
                  <Bar label="EN" value={c.energy} max={c.max_energy} tone="bg-[#D4AF37]" />
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );

  const renderQuests = () => (
    <div className="vox-marble-tablet rounded-xl border border-[#C59B27]/75 bg-[#FAF6EE] p-3 space-y-2.5 text-[#1C140C]">
      <div className="flex items-center gap-2 border-b border-[#C59B27]/40 pb-1.5 font-display text-[11px] font-bold uppercase tracking-wider text-[#8C6214]">
        <Icon.scrollTabula className="size-3.5 text-[#8C6214]" />
        <span>Active Engineering Quests</span>
      </div>
      {quests.length === 0 ? (
        <div className="rounded-lg border border-[#C59B27]/40 bg-[#F4EDE0] p-2.5 text-[11px] text-[#5C4428]">
          No active quests.
        </div>
      ) : (
        <ul className="space-y-2">
          {quests.map(q => (
            <li
              key={q.id}
              className="rounded-lg border border-[#C59B27]/55 bg-[#FFFDF8] p-2.5 shadow-xs"
            >
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <div className="truncate font-display text-[12px] font-semibold text-[#1C140C]">
                    {q.description}
                  </div>
                  <div className="truncate text-[11px] text-[#5C4428]">{q.hint}</div>
                </div>
                <span
                  className={`shrink-0 rounded-sm border px-2 py-0.5 font-mono text-[10px] uppercase font-semibold ${
                    q.completed
                      ? 'border-[#10B981]/50 bg-[#10B981]/15 text-[#065F46]'
                      : 'border-[#C59B27]/60 bg-[#F3E5AB]/55 text-[#1C140C]'
                  }`}
                >
                  {q.completed ? 'done' : q.status}
                </span>
              </div>
              <div className="mt-2">
                <Bar
                  label={`${q.progress}/${q.target}`}
                  value={q.progress}
                  max={q.target}
                  tone="bg-[#D4AF37]"
                />
              </div>
              <div className="mt-1.5 flex items-center gap-2 font-mono text-[10px] text-[#5C4428]">
                <Icon.denariusCoin className="size-3 text-[#8C6214]" />
                <span>
                  +{q.xp_reward} XP · +{q.crystal_reward} Crystals · {q.quest_type}
                </span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );

  return (
    <section data-testid="gamify-view" className="space-y-1.5 text-[#E6DCB8]">
      {/* Plate 2 Luminous White Carrara Marble Frieze Header */}
      <div
        data-roman-frieze="true"
        className="vox-animate-marble-sheen relative overflow-hidden rounded-xl border-2 border-[#C59B27] px-3 py-1.5 shadow-[0_6px_18px_rgba(28,20,12,0.18),inset_0_2px_0_rgba(255,255,255,0.92)]"
        style={{
          backgroundImage: `linear-gradient(115deg, rgba(255,252,245,0) 35%, rgba(255,248,225,0.48) 50%, rgba(255,252,245,0) 65%), linear-gradient(90deg, rgba(250,246,238,0.92) 0%, rgba(242,235,221,0.82) 48%, rgba(250,246,238,0.92) 100%), url('/assets/romanesque/carrara-acanthus-header-band.jpg')`,
          backgroundSize: '240% 100%, 100% 100%, cover',
          backgroundPosition: 'center, center, center',
        }}
      >
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap items-center gap-2.5">
            <RomanMedallion active size="sm">
              <Icon.romanColumn className="size-3.5 text-[#FFF8E7]" />
            </RomanMedallion>
            <h2 className="font-display text-sm font-bold tracking-widest text-[#1C140C] uppercase">
              {useLabel('gamification')}
            </h2>
            <span className="text-[#8C6825] font-bold" aria-hidden="true">·</span>
            <span className="font-display text-xs font-bold tracking-wider text-[#2B2014] uppercase">
              Workspace Civilization &amp; Engineering Simulator
            </span>
            <span className="hidden rounded-full border border-[#785312] bg-linear-to-b from-[#F2D67B] via-[#D4AF37] to-[#9A741C] px-2.5 py-0.5 font-display text-[10px] font-bold text-[#1A1205] sm:inline-block shadow-sm">
              7 Crate Districts · 48 Modules
            </span>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {/* Single Clean Architrave Header Segmented Tabs (matching Concept Art Option A) */}
            <div
              role="group"
              aria-label="Civilization sub-surfaces"
              className="flex flex-wrap items-center rounded-lg border border-border-subtle bg-bg-elevated p-0.5"
            >
              <RomanTooltip
                title="Workspace Map"
                purpose="Interactive architectural diorama of the codebase civilization, active crates, and module districts."
                actionHint="Click to view Workspace Map"
              >
                <button
                  type="button"
                  data-testid="ludus-subtab-urbs"
                  aria-pressed={subtab === 'urbs' && simMode === 'urbs'}
                  onClick={() => {
                    setSubtab('urbs');
                    setSimMode('urbs');
                  }}
                  className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                    subtab === 'urbs' && simMode === 'urbs'
                      ? 'border border-(--color-accent-default) bg-bg-surface text-text-primary shadow-sm'
                      : 'text-text-secondary hover:text-text-primary'
                  }`}
                >
                  <Icon.romanColumn className="size-3 shrink-0" />
                  <span data-testid="gamify-sim-mode-urbs">Workspace Map</span>
                </button>
              </RomanTooltip>

              <RomanTooltip
                title="Live River View"
                purpose="Real-time hydraulic execution pipeline showing token throughput, agent trajectories, and verification gates."
                actionHint="Switch to Live River"
              >
                <button
                  type="button"
                  data-testid="gamify-sim-mode-river"
                  aria-pressed={subtab === 'urbs' && simMode === 'river'}
                  onClick={() => {
                    setSubtab('urbs');
                    setSimMode('river');
                  }}
                  className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                    subtab === 'urbs' && simMode === 'river'
                      ? 'border border-(--color-accent-default) bg-bg-surface text-text-primary shadow-sm'
                      : 'text-text-secondary hover:text-text-primary'
                  }`}
                >
                  <Icon.aqueductArch className="size-3 shrink-0" />
                  <span>Live River · Pipeline</span>
                </button>
              </RomanTooltip>

              <RomanTooltip
                title="Arena & Quests"
                purpose="Active engineering quests, compiler challenges, and companion assistants."
                actionHint="Click to view Arena & Quests"
              >
                <button
                  type="button"
                  data-testid="ludus-subtab-collegium"
                  aria-pressed={subtab === 'collegium'}
                  onClick={() => setSubtab('collegium')}
                  className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                    subtab === 'collegium'
                      ? 'border border-(--color-accent-default) bg-bg-surface text-text-primary shadow-sm'
                      : 'text-text-secondary hover:text-text-primary'
                  }`}
                >
                  <Icon.gladiusShield className="size-3 shrink-0" />
                  <span>Arena &amp; Quests</span>
                </button>
              </RomanTooltip>

              <RomanTooltip
                title="Engineering Leaderboard"
                purpose="Cryptographically verified rankings of engineers, autonomous agents, and tuned MENS models."
                actionHint="Click to view Engineering Leaderboard"
              >
                <button
                  type="button"
                  data-testid="ludus-subtab-arena"
                  aria-pressed={subtab === 'arena'}
                  onClick={() => setSubtab('arena')}
                  className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                    subtab === 'arena'
                      ? 'border border-(--color-accent-default) bg-bg-surface text-text-primary shadow-sm'
                      : 'text-text-secondary hover:text-text-primary'
                  }`}
                >
                  <Icon.laurelWreath className="size-3 shrink-0" />
                  <span>Leaderboard</span>
                </button>
              </RomanTooltip>
            </div>

            <RomanTooltip
              title="Refresh Telemetry"
              purpose="Fetches latest Ludus profile, unread imperial notices, companions, and quests from backend."
              actionHint="Click to refresh telemetry"
            >
              <button
                type="button"
                onClick={refresh}
                disabled={loading}
                className="rounded-md border border-border-subtle bg-bg-elevated px-2.5 py-1 font-mono text-[10px] text-text-secondary hover:border-border-strong hover:text-text-primary transition"
              >
                {loading ? 'Loading…' : 'Refresh'}
              </button>
            </RomanTooltip>
          </div>
        </div>
      </div>

      {/* Primary Simulator Sandbox Container */}
      {showUrbs && (
        <div className="overflow-hidden rounded-2xl border-2 border-border-strong bg-bg-surface shadow-2xl">
          {simMode === 'urbs' ? (
            <div className="h-[calc(100vh-208px)] min-h-[465px] max-h-[860px] w-full">
              <LudusSandbox energy={profile?.energy ?? 0} maxEnergy={profile?.max_energy ?? 0} onNavigate={onNavigate} />
            </div>
          ) : (
            <div data-testid="gamify-live-river-container" className="p-2">
              <HarnessRiverPanel gamifyEnabled />
            </div>
          )}
        </div>
      )}

      {/* When on Workspace Map (subtab === 'urbs'), render Collegium sections inside a progressive-disclosure accordion */}
      {subtab === 'urbs' && (
        <div
          className="vox-marble-tablet relative overflow-hidden rounded-xl border border-border-strong bg-bg-surface px-3 py-1 shadow-[0_4px_12px_rgba(0,0,0,0.25),inset_0_1px_0_rgba(255,255,255,0.25)]"
        >
          <div className="flex items-center justify-between font-serif text-[11px] font-bold tracking-wide text-text-primary">
            <span>▸ Companion Familiars, Imperial Notices &amp; Active Quests</span>
            <button
              type="button"
              onClick={() => setSubtab('collegium')}
              className="rounded border border-border-strong bg-bg-elevated px-2.5 py-0.5 font-mono text-[10px] font-semibold text-text-primary hover:border-(--color-accent-default)"
            >
              Open Arena &amp; Quests ↗
            </button>
          </div>
          <div className="pointer-events-none absolute -left-[9999px] top-0 z-[-1] w-[800px] opacity-0">
            {renderNotifications()}
            {renderCompanions()}
            {renderQuests()}
          </div>
        </div>
      )}

      {/* Ludus HUD: Only when activeTab !== 'urbs' (collegium, arena, or all) */}
      {subtab !== 'urbs' && (
        profile ? (
          <LudusHud profile={profile} />
        ) : (
          <div className="rounded-xl border border-border-subtle bg-bg-surface p-4 text-sm text-text-secondary">
            No profile yet.
          </div>
        )
      )}

      {/* Notifications Tablet when subtab !== 'urbs' */}
      {subtab !== 'urbs' && showCollegium && renderNotifications()}

      {/* Engineering Leaderboard (F3) */}
      {showArena && (
        <LeaderboardArenaPanel
          entries={leaderboard as VerifiedLeaderboardEntry[]}
          localUserHandle={profile?.user_id ?? 'local_user'}
          localXp={profile?.total_xp_earned ?? 0}
        />
      )}

      {/* Companions (F3) when subtab !== 'urbs' */}
      {subtab !== 'urbs' && showCollegium && renderCompanions()}

      {/* Quests (F3) when subtab !== 'urbs' */}
      {subtab !== 'urbs' && showCollegium && renderQuests()}
    </section>
  );
}

/** A compact labeled progress bar. `max <= 0` renders an empty track. */
function Bar({ label, value, max, tone }: { label: string; value: number; max: number; tone: string }) {
  const pct = max > 0 ? Math.max(0, Math.min(100, (value / max) * 100)) : 0;
  return (
    <div className="flex items-center gap-2">
      <span className="w-12 shrink-0 font-mono text-[9px] uppercase tracking-wider text-text-secondary">{label}</span>
      <div
        className="h-1.5 flex-1 overflow-hidden rounded-full bg-bg-elevated border border-border-subtle"
        role="progressbar"
        aria-label={label}
        aria-valuenow={Math.round(pct)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div className={`h-full rounded-full transition-all duration-300 ${tone}`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
