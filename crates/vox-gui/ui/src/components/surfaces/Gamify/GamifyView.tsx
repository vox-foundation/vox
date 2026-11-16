import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useLabel } from '../../../hooks/useLanguage';
import { LudusSandbox } from '../../gamify/LudusSandbox';
import { HarnessRiverPanel } from '../Harness/HarnessRiverPanel';
import { invokeCmd } from '../../../transport';
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
        invokeCmd<LudusProfile>('get_ludus_profile'),
        invokeCmd<LudusNotification[]>('list_ludus_notifications', { limit: 20 }),
        invokeCmd<LeaderboardEntry[]>('list_gamify_leaderboard', { limit: 10 }),
        invokeCmd<Companion[]>('list_gamify_companions'),
        invokeCmd<Quest[]>('list_gamify_quests'),
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
      await invokeCmd('ack_ludus_notification', { notificationId: id });
      setNotes(curr => curr.filter(x => x.id !== id));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Ack failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const showUrbs = subtab === 'all' || subtab === 'urbs';
  const showCollegium = subtab === 'all' || subtab === 'collegium';
  const showArena = subtab === 'all' || subtab === 'arena';

  const renderNotifications = () => (
    <div className="vox-marble-tablet rounded-xl border border-[#C59B27]/75 bg-[#FAF6EE] p-3 space-y-2 text-(--color-text-primary)">
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

  const renderSanctuaryGuildStrip = () => {
    const sanctuaries = [
      {
        id: 'sanctuary-compiler',
        tier: 'I',
        title: 'Templum Minervae',
        subtitle: 'Compiler & Typecheck',
        crateName: 'vox-compiler',
        targetView: 'explorer',
        art: '/assets/romanesque/monument-temple-compiler.jpg',
        activity: `${quests.filter((q) => !q.completed).length || 2} Active Quests`,
        purpose: 'Sacred Acropolis sanctuary governing Vox syntax, HIR typecheck verification, and zero-defect effect gates.',
      },
      {
        id: 'sanctuary-arena',
        tier: 'II',
        title: 'Amphitheatrum',
        subtitle: 'CI Gates & Trials',
        crateName: 'vox-code-audit',
        targetView: 'harness',
        art: '/assets/romanesque/monument-colosseum-harness.jpg',
        activity: 'Gates Verified',
        purpose: 'Imperial Colosseum arena hosting TOESTUB static detectors, test gauntlets, and cryptographic proof trials.',
      },
      {
        id: 'sanctuary-foundry',
        tier: 'III',
        title: 'Fornax MENS',
        subtitle: 'QLoRA Model Forge',
        crateName: 'vox-mens',
        targetView: 'mens',
        art: '/assets/romanesque/monument-foundry-mens.jpg',
        activity: 'Crucible Lit',
        purpose: 'Hypocaust crucible foundry where 0.6B–32B local MENS models and 8-spoke QLoRA adapters are tempered.',
      },
      {
        id: 'sanctuary-portus',
        tier: 'IV',
        title: 'Portus Populi',
        subtitle: 'Mesh & Companion Quays',
        crateName: 'vox-populi',
        targetView: 'mesh',
        art: '/assets/romanesque/monument-portus-mesh.jpg',
        activity: `${companions.length || 3} Familiars`,
        purpose: 'Harbor emporium and canal bridge dispatching companion familiars and distributed iroh QUIC compute galleys.',
      },
    ] as const;

    return (
      <div
        data-testid="gamify-sanctuary-strip"
        className="vox-marble-tablet rounded-xl border border-[#C59B27]/80 bg-[#FAF6EE] p-3 text-(--color-text-primary) shadow-sm"
      >
        <div className="mb-2.5 flex flex-wrap items-center justify-between gap-2 border-b border-[#C59B27]/35 pb-1.5">
          <div className="flex items-center gap-2 font-display text-[11px] font-bold uppercase tracking-wider text-[#8C6214]">
            <Icon.romanColumn className="size-3.5 text-[#8C6214]" />
            <span>Collegium Guild Sanctuaries</span>
          </div>
          <span className="inline-flex items-center gap-1.5 font-sans text-[10px] font-semibold text-[#5C4428]">
            <span className="size-1.5 rounded-full bg-[#10B981] shadow-[0_0_5px_#10B981] animate-pulse" />
            <span>Hover any sanctuary for architectural halo &amp; guild telemetry · Click to enter</span>
          </span>
        </div>
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          {sanctuaries.map((s) => (
            <RomanTooltip
              key={s.id}
              title={`${s.tier} · ${s.title}`}
              purpose={s.purpose}
              previewImageSrc={s.art}
              crateBadge={s.crateName}
              statusBadge={s.activity}
              metrics={[
                { label: 'Sanctuary', value: s.tier, tone: 'gold' },
                { label: 'Status', value: 'Interactive', tone: 'emerald' },
                { label: 'Surface', value: s.targetView, tone: 'gold' },
              ]}
              actionHint={
                onNavigate
                  ? `Click sanctuary to open ${s.subtitle} (${s.targetView})`
                  : `Interactive ${s.subtitle} guild sanctuary`
              }
              className="block w-full"
            >
              <button
                type="button"
                data-testid={`gamify-${s.id}`}
                onClick={() => onNavigate?.(s.targetView)}
                className="group relative block h-36 w-full overflow-hidden rounded-xl border-2 border-[#C59B27]/75 bg-(--color-bg-elevated) text-left shadow-[0_4px_12px_rgba(28,20,12,0.22)] transition-[transform,border-color,box-shadow] duration-200 hover:-translate-y-0.5 hover:border-[#8C6214] hover:shadow-[0_8px_24px_rgba(28,20,12,0.34),0_0_24px_rgba(245,198,72,0.65)] cursor-pointer"
              >
                <img
                  src={s.art}
                  alt=""
                  aria-hidden="true"
                  className="h-full w-full object-cover object-center transition-transform duration-300 group-hover:scale-105"
                />
                {/* Mouseover Warm Sunlit Architectural Rim Halo */}
                <div
                  aria-hidden="true"
                  className="pointer-events-none absolute inset-0 rounded-xl opacity-0 ring-2 ring-inset ring-[#FFF3C4]/90 bg-[radial-gradient(circle_at_50%_38%,rgba(255,248,210,0.32),transparent_68%)] transition-opacity duration-200 group-hover:opacity-100"
                />
                {/* Top-right Live Activity Guidance Beacon */}
                <span className="absolute right-2 top-2 inline-flex items-center gap-1 rounded-[4px] border border-border-strong bg-bg-surface/95 px-1.5 py-0.5 font-mono text-[8.5px] font-bold text-text-primary shadow-sm">
                  <span className="size-1.5 rounded-full bg-(--color-status-pass) animate-pulse" />
                  <span>{s.activity}</span>
                </span>
                {/* Diegetic Plinth Architrave Inscription anchored cleanly at monument base */}
                <div className="absolute inset-x-1.5 bottom-1.5 rounded-[6px] border border-border-strong bg-bg-surface/96 px-2 py-1 shadow-[0_3px_10px_rgba(28,20,12,0.42),inset_0_1px_0_rgba(255,255,255,0.45)]">
                  <div className="flex items-center justify-between gap-1">
                    <span className="truncate font-display text-[10px] font-bold uppercase tracking-[0.06em] text-text-primary">
                      {s.tier} · {s.title}
                    </span>
                    <Icon.externalLink className="size-3 shrink-0 text-text-primary" aria-hidden="true" />
                  </div>
                  <div className="truncate font-sans text-[9px] font-semibold text-text-secondary">
                    {s.subtitle}
                  </div>
                </div>
              </button>
            </RomanTooltip>
          ))}
        </div>
      </div>
    );
  };

  const renderCompanions = () => (
    <div className="vox-marble-tablet rounded-xl border border-[#C59B27]/75 bg-[#FAF6EE] p-3 space-y-2.5 text-(--color-text-primary)">
      <div className="flex items-center justify-between gap-2 border-b border-[#C59B27]/40 pb-1.5">
        <div className="flex items-center gap-2 font-display text-[11px] font-bold uppercase tracking-wider text-[#8C6214]">
          <Icon.gladiusShield className="size-3.5 text-[#8C6214]" />
          <span>Companion Assistants &amp; Familiars</span>
        </div>
        <span className="font-mono text-[9.5px] font-semibold text-[#5C4428]">
          Hover card for familiar sanctuary &amp; vitality
        </span>
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
            const companionArt =
              idx % 3 === 0
                ? '/assets/romanesque/monument-temple-compiler.jpg'
                : idx % 3 === 1
                  ? '/assets/romanesque/monument-foundry-mens.jpg'
                  : '/assets/romanesque/monument-portus-mesh.jpg';
            return (
              <RomanTooltip
                key={c.id}
                title={c.name}
                purpose={c.description || `Dedicated ${c.language} engineering familiar (${c.mood} mood) stationed in the Collegium.`}
                previewImageSrc={companionArt}
                crateBadge={c.language}
                statusBadge={`Quality ${c.code_quality}%`}
                metrics={[
                  { label: 'Vitality', value: `${c.health}/${c.max_health}`, tone: 'emerald' },
                  { label: 'Energy', value: `${c.energy}/${c.max_energy}`, tone: 'gold' },
                  { label: 'Mood', value: c.mood, tone: 'emerald' },
                ]}
                actionHint="Active companion familiar — gains XP & energy from clean builds and quests"
                className="block w-full"
              >
                <div className="group relative rounded-xl border border-[#C59B27]/70 bg-linear-to-b from-[#FFFDF9] to-[#F2E9D8] p-3 shadow-sm transition-[transform,border-color,box-shadow] duration-200 hover:-translate-y-0.5 hover:border-[#8C6214] hover:shadow-[0_6px_20px_rgba(28,20,12,0.18),0_0_20px_rgba(212,175,55,0.42)]">
                  <span
                    aria-hidden="true"
                    className="absolute right-2.5 top-2.5 inline-flex items-center gap-1 rounded-full border border-[#10B981]/55 bg-[#D1FAE5] px-1.5 py-0.5 font-mono text-[8px] font-bold text-[#065F46]"
                  >
                    <span className="size-1.5 rounded-full bg-(--color-success,rgb(16_185_129)) animate-pulse" />
                    <span>READY</span>
                  </span>
                  <div className="relative mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-full border-2 border-[#C59B27] bg-bg-elevated shadow-[0_0_12px_rgba(212,175,55,0.28)] transition-shadow group-hover:shadow-[0_0_18px_rgba(245,198,72,0.65)]">
                    <img
                      src="/assets/romanesque/wax-seal-medallion.jpg"
                      alt=""
                      aria-hidden="true"
                      className="absolute inset-0 h-full w-full rounded-full object-cover opacity-35 mix-blend-luminosity"
                    />
                    <CompanionIcon className="relative z-10 size-6 text-brass" />
                  </div>
                  <div className="truncate text-center font-display text-[12px] font-bold text-(--color-text-primary)">
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
              </RomanTooltip>
            );
          })}
        </div>
      )}
    </div>
  );

  const renderQuests = () => (
    <div className="vox-marble-tablet rounded-xl border border-[#C59B27]/75 bg-[#FAF6EE] p-3 space-y-2.5 text-(--color-text-primary)">
      <div className="flex items-center justify-between gap-2 border-b border-[#C59B27]/40 pb-1.5">
        <div className="flex items-center gap-2 font-display text-[11px] font-bold uppercase tracking-wider text-[#8C6214]">
          <Icon.scrollTabula className="size-3.5 text-[#8C6214]" />
          <span>Active Engineering Quests</span>
        </div>
        <span className="font-mono text-[9.5px] font-semibold text-[#5C4428]">
          Hover quest tablet for reward &amp; sanctuary hint
        </span>
      </div>
      {quests.length === 0 ? (
        <div className="rounded-lg border border-[#C59B27]/40 bg-[#F4EDE0] p-2.5 text-[11px] text-[#5C4428]">
          No active quests.
        </div>
      ) : (
        <ul className="space-y-2">
          {quests.map(q => (
            <li key={q.id}>
              <RomanTooltip
                title={q.description}
                purpose={q.hint || `Complete ${q.quest_type} milestone (${q.progress}/${q.target}) to claim Collegium rewards.`}
                previewImageSrc="/assets/romanesque/monument-colosseum-harness.jpg"
                crateBadge={q.quest_type}
                statusBadge={q.completed ? 'Completed' : `${q.progress}/${q.target}`}
                metrics={[
                  { label: 'XP Yield', value: `+${q.xp_reward}`, tone: 'gold' },
                  { label: 'Crystals', value: `+${q.crystal_reward}`, tone: 'emerald' },
                  { label: 'State', value: q.completed ? 'Done' : q.status, tone: q.completed ? 'emerald' : 'gold' },
                ]}
                actionHint="Progress updates automatically as compiler checks, tests, and reviews pass"
                className="block w-full"
              >
                <div className="rounded-lg border border-[#C59B27]/60 bg-[#FFFDF8] p-2.5 shadow-xs transition-[border-color,box-shadow] duration-200 hover:border-[#8C6214] hover:shadow-[0_4px_16px_rgba(28,20,12,0.14),0_0_18px_rgba(212,175,55,0.38)]">
                  <div className="flex items-center justify-between gap-3">
                    <div className="min-w-0">
                      <div className="truncate font-display text-[12px] font-semibold text-(--color-text-primary)">
                        {q.description}
                      </div>
                      <div className="truncate text-[11px] text-[#5C4428]">{q.hint}</div>
                    </div>
                    <span
                      className={`inline-flex shrink-0 items-center gap-1 rounded-sm border px-2 py-0.5 font-mono text-[10px] uppercase font-semibold ${
                        q.completed
                          ? 'border-[#10B981]/50 bg-[#10B981]/15 text-[#065F46]'
                          : 'border-[#C59B27]/60 bg-[#F3E5AB]/55 text-(--color-text-primary)'
                      }`}
                    >
                      <span
                        className={`size-1.5 rounded-full ${
                          q.completed ? 'bg-(--color-success,rgb(16_185_129))' : 'bg-[#B8860B] animate-pulse'
                        }`}
                      />
                      <span>{q.completed ? 'done' : q.status}</span>
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
                </div>
              </RomanTooltip>
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
            <h2 className="font-display text-sm font-bold tracking-widest text-(--color-text-primary) uppercase">
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
            <span className="inline-flex items-center gap-1.5">
              <Icon.chevR className="size-3.5 shrink-0 text-text-secondary" aria-hidden="true" />
              <span>Companion Familiars, Imperial Notices &amp; Active Quests</span>
              <Icon.chevronDown className="size-3 shrink-0 text-text-muted" aria-hidden="true" />
            </span>
            <button
              type="button"
              onClick={() => setSubtab('collegium')}
              className="inline-flex items-center gap-1 rounded border border-border-strong bg-bg-elevated px-2.5 py-0.5 font-mono text-[10px] font-semibold text-text-primary hover:border-(--color-accent-default) transition-colors"
            >
              <span>Open Arena &amp; Quests</span>
              <Icon.externalLink className="size-3 shrink-0" aria-hidden="true" />
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

      {/* Diegetic Guild Sanctuary Buildings when subtab !== 'urbs' */}
      {subtab !== 'urbs' && showCollegium && renderSanctuaryGuildStrip()}

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
        <div className={`h-full rounded-full transition-[width] duration-300 ${tone}`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
