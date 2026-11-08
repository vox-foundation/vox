import React from 'react';
import { RomanMedallion, RomanBrassGauge, RomanTooltip } from './RomanTooltip';

export interface RomanSurfaceGaugeSpec {
  label: string;
  valueText: string;
  ratio: number;
  tone?: 'gold' | 'emerald' | 'crimson';
  purpose?: string;
  onClick?: () => void;
}

export interface RomanSurfaceHeaderProps {
  /** Domain or category prefix, e.g. "COMPUTE", "MARKET", "WORKSPACE", "COMMANDS", "KNOWLEDGE" */
  domainTag: string;
  /** Primary readable humanist title */
  title: string;
  /** Concise subtitle explaining the surface */
  subtitle?: string;
  /** Optional crimson wax-seal status badge label (e.g. "LIVE SYNC", "VERIFIED", "SANDBOXED") */
  waxSealBadge?: string;
  /** Optional icon inside the embossed circular bronze medallion */
  icon?: React.ReactNode;
  /** Optional semicircular brass telemetry gauges shown in the frieze header */
  gauges?: readonly RomanSurfaceGaugeSpec[];
  /** Optional right-hand interactive controls / filter pills / actions */
  actions?: React.ReactNode;
  className?: string;
}

/**
 * Domain-varied Romanesque Architectural Framing Relief (`<RomanFlutedPillar>`).
 * Avoids repeating identical fluted columns on every page by adapting the architectural motif
 * to `variant` (`laurel` acanthus & olive-laurel relief, `rosette` cast brass astrolabe seal,
 * `mosaic` opus sectile gold geometric inlay, or `column` fluted Carrara marble pilaster).
 */
export function RomanFlutedPillar({
  side,
  variant = 'laurel',
  className = '',
}: {
  side: 'left' | 'right';
  variant?: 'column' | 'laurel' | 'rosette' | 'mosaic';
  className?: string;
}) {
  if (variant === 'laurel') {
    return (
      <div
        data-roman-pillar="true"
        data-pillar-side={side}
        data-pillar-variant="laurel"
        aria-hidden="true"
        className={`pointer-events-none flex w-3.5 sm:w-4 shrink-0 flex-col items-center justify-between py-0.5 select-none ${className}`}
      >
        <svg viewBox="0 0 20 56" className="w-full h-full max-h-14 drop-shadow-[0_1px_1px_rgba(255,255,255,0.85)]">
          <rect x="2" y="2" width="16" height="52" rx="3" fill="#F5EFE2" stroke="#B88E28" strokeWidth="1.2" />
          <path d="M10 6 L10 50" stroke="#9A741C" strokeWidth="1.1" strokeDasharray="2 2" />
          <path d="M10 14 C5 11, 4 17, 10 18 C16 17, 15 11, 10 14 Z" fill="#6E7B2B" stroke="#4D5D1E" strokeWidth="0.7" />
          <path d="M10 26 C5 23, 4 29, 10 30 C16 29, 15 23, 10 26 Z" fill="#C59B27" stroke="#785312" strokeWidth="0.7" />
          <path d="M10 38 C5 35, 4 41, 10 42 C16 41, 15 35, 10 38 Z" fill="#6E7B2B" stroke="#4D5D1E" strokeWidth="0.7" />
          <circle cx="10" cy="8" r="2" fill="#D4AF37" />
          <circle cx="10" cy="48" r="2" fill="#D4AF37" />
        </svg>
      </div>
    );
  }

  if (variant === 'rosette') {
    return (
      <div
        data-roman-pillar="true"
        data-pillar-side={side}
        data-pillar-variant="rosette"
        aria-hidden="true"
        className={`pointer-events-none flex w-3.5 sm:w-4 shrink-0 flex-col items-center justify-between py-0.5 select-none ${className}`}
      >
        <svg viewBox="0 0 20 56" className="w-full h-full max-h-14 drop-shadow-[0_1px_1px_rgba(255,255,255,0.85)]">
          <rect x="2" y="2" width="16" height="52" rx="4" fill="#EFE6D4" stroke="#9E7B2F" strokeWidth="1.2" />
          <circle cx="10" cy="13" r="4.2" fill="#D4AF37" stroke="#6B4C12" strokeWidth="0.9" />
          <circle cx="10" cy="13" r="1.6" fill="#FFF8E7" />
          <line x1="10" y1="19" x2="10" y2="37" stroke="#8C6825" strokeWidth="1.4" />
          <polygon points="10,24 14,28 10,32 6,28" fill="#881B1B" stroke="#D4AF37" strokeWidth="0.8" />
          <circle cx="10" cy="43" r="4.2" fill="#D4AF37" stroke="#6B4C12" strokeWidth="0.9" />
          <circle cx="10" cy="43" r="1.6" fill="#FFF8E7" />
        </svg>
      </div>
    );
  }

  if (variant === 'mosaic') {
    return (
      <div
        data-roman-pillar="true"
        data-pillar-side={side}
        data-pillar-variant="mosaic"
        aria-hidden="true"
        className={`pointer-events-none flex w-3.5 sm:w-4 shrink-0 flex-col items-center justify-between py-0.5 select-none ${className}`}
      >
        <svg viewBox="0 0 20 56" className="w-full h-full max-h-14 drop-shadow-[0_1px_1px_rgba(255,255,255,0.85)]">
          <rect x="2" y="2" width="16" height="52" rx="2" fill="#F7F2E7" stroke="#B88E28" strokeWidth="1.2" />
          <polygon points="10,6 15,12 10,18 5,12" fill="#D4AF37" stroke="#785312" strokeWidth="0.8" />
          <polygon points="10,22 15,28 10,34 5,28" fill="#EDE2CE" stroke="#8C6825" strokeWidth="0.9" />
          <polygon points="10,38 15,44 10,50 5,44" fill="#D4AF37" stroke="#785312" strokeWidth="0.8" />
        </svg>
      </div>
    );
  }

  return (
    <div
      data-roman-pillar="true"
      data-pillar-side={side}
      data-pillar-variant="column"
      aria-hidden="true"
      className={`pointer-events-none flex w-4 sm:w-5 shrink-0 flex-col items-center justify-between select-none ${className}`}
    >
      {/* Ionic Volute & Acanthus Capital in Carved White Carrara Marble */}
      <svg viewBox="0 0 28 14" className="w-full h-3.5 shrink-0 drop-shadow-[0_1px_2px_rgba(0,0,0,0.35)]">
        <defs>
          <linearGradient id={`marble-cap-${side}`} x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stopColor="#D5C7B0" />
            <stop offset="30%" stopColor="#FBF8F1" />
            <stop offset="50%" stopColor="#E6DAC6" />
            <stop offset="75%" stopColor="#F8F3E9" />
            <stop offset="100%" stopColor="#C4B399" />
          </linearGradient>
        </defs>
        <rect x="1" y="1" width="26" height="3" rx="1" fill={`url(#marble-cap-${side})`} stroke="#9E7B2F" strokeWidth="0.8" />
        <circle cx="5" cy="7.5" r="3.8" fill={`url(#marble-cap-${side})`} stroke="#8C6825" strokeWidth="0.9" />
        <circle cx="5" cy="7.5" r="1.6" fill="none" stroke="#8C6825" strokeWidth="0.7" />
        <circle cx="23" cy="7.5" r="3.8" fill={`url(#marble-cap-${side})`} stroke="#8C6825" strokeWidth="0.9" />
        <circle cx="23" cy="7.5" r="1.6" fill="none" stroke="#8C6825" strokeWidth="0.7" />
        <rect x="4" y="9.5" width="20" height="3.5" rx="1" fill={`url(#marble-cap-${side})`} stroke="#9E7B2F" strokeWidth="0.8" />
      </svg>

      {/* Fluted Carrara Marble Shaft */}
      <div
        className="w-3 sm:w-3.5 flex-1 border-x border-[#9E7B2F]/65 shadow-[inset_0_0_4px_rgba(0,0,0,0.22)]"
        style={{
          backgroundImage:
            'repeating-linear-gradient(90deg, #FBF8F2 0px, #EAE0CE 2px, #CBBBA0 3.5px, #F6F0E4 5px)',
        }}
      />

      {/* Attic Molded Marble Plinth Base */}
      <svg viewBox="0 0 28 10" className="w-full h-2.5 shrink-0 drop-shadow-[0_1px_2px_rgba(0,0,0,0.4)]">
        <rect x="3" y="1" width="22" height="3.5" rx="1" fill="#EFE7D8" stroke="#9E7B2F" strokeWidth="0.8" />
        <rect x="1" y="5" width="26" height="4" rx="1" fill="#DFD2BC" stroke="#8C6825" strokeWidth="0.9" />
      </svg>
    </div>
  );
}

/**
 * Plate 2 Luminous White & Cream Carrara Marble Bas-Relief Frieze Surface Header (`<RomanSurfaceHeader>`).
 * Matches the approved Concept Art plates (`v61_concept_compute_and_market`, `v61_concept_workspace_and_commands`,
 * `v61_concept_knowledge_explorer_urbs_refined`) with:
 * - Sculpted white/cream Carrara marble bas-relief frieze (`/assets/romanesque/white-marble-frieze-banner.jpg`)
 * - Left & right fluted Ionic Carrara marble pillars (`<RomanFlutedPillar>`)
 * - 3D embossed antique bronze medallion (`/assets/romanesque/bronze-medallion-emblem.jpg`)
 * - Engraved Roman `Cinzel` typography in deep bronze ink (`#1E150C`) on luminous ivory marble
 * - Physical analog semicircular brass dial gauges with cream marble dial faces & animated needles
 */
export function RomanSurfaceHeader({
  domainTag,
  title,
  subtitle,
  waxSealBadge,
  icon,
  gauges,
  actions,
  className = '',
}: RomanSurfaceHeaderProps) {
  const tagUpper = domainTag.toUpperCase();
  const reliefVariant: 'column' | 'laurel' | 'rosette' | 'mosaic' =
    tagUpper.includes('KNOWLEDGE') || tagUpper.includes('EXPLORER')
      ? 'column'
      : tagUpper.includes('COMPUTE') || tagUpper.includes('MARKET') || tagUpper.includes('MERCATUS')
        ? 'rosette'
        : tagUpper.includes('WORKSPACE') || tagUpper.includes('COMMANDS') || tagUpper.includes('REPOSITORY')
          ? 'mosaic'
          : 'laurel';

  return (
    <header
      data-roman-frieze="true"
      style={{
        backgroundImage:
          'linear-gradient(115deg, rgba(255,252,245,0) 35%, rgba(255,248,225,0.48) 50%, rgba(255,252,245,0) 65%), linear-gradient(90deg, rgba(246,240,229,0.88) 0%, rgba(238,229,212,0.74) 48%, rgba(246,240,229,0.88) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
        backgroundSize: '240% 100%, 100% 100%, cover',
        backgroundPosition: 'center',
      }}
      className={`vox-animate-marble-sheen relative overflow-hidden rounded-xl border-2 border-[#C59B27] px-2.5 py-2 shadow-[0_10px_28px_rgba(0,0,0,0.75),inset_0_2px_0_rgba(255,255,255,0.92),inset_0_-2px_0_rgba(168,132,47,0.55)] ${className}`}
    >
      {/* Top & bottom burnished gold cornice fillets */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-1 bg-linear-to-r from-[#8C6825] via-[#F3E5AB] to-[#8C6825] opacity-90"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 bottom-0 h-1 bg-linear-to-r from-[#8C6825] via-[#D4AF37] to-[#8C6825] opacity-90"
      />

      <div className="relative flex items-stretch justify-between gap-2.5">
        {/* Left Domain-Varied Romanesque Relief */}
        <RomanFlutedPillar side="left" variant={reliefVariant} />

        {/* Central White Carrara Marble Frieze Content */}
        <div className="flex flex-1 flex-wrap items-center justify-between gap-3 py-1">
          <div className="flex min-w-0 items-center gap-3">
            <RomanMedallion active size="lg">
              {icon ?? (
                <span className="font-display text-xs font-bold tracking-wider text-[#FFF8E7]">
                  {domainTag.slice(0, 2).toUpperCase()}
                </span>
              )}
            </RomanMedallion>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <span
                  data-roman-brass-plaque="true"
                  className="inline-flex items-center gap-1 rounded border border-[#785312] bg-linear-to-b from-[#F2D67B] via-[#D4AF37] to-[#9A741C] px-2.5 py-0.5 font-display text-[9px] font-bold uppercase tracking-[0.16em] text-[#1A1205] shadow-[0_2px_5px_rgba(0,0,0,0.35),inset_0_1px_0_rgba(255,250,225,0.85)]"
                >
                  <span className="size-1 rounded-full bg-[#3A2707]" aria-hidden="true" />
                  <span>{domainTag}</span>
                  <span className="size-1 rounded-full bg-[#3A2707]" aria-hidden="true" />
                </span>
                <h1
                  className="font-display text-sm sm:text-base font-bold tracking-[0.06em] text-[#1C140C] truncate"
                  style={{ textShadow: '0 1px 0 rgba(255, 255, 255, 0.85)' }}
                >
                  {title}
                </h1>
                {waxSealBadge && (
                  <RomanTooltip
                    title={`${title} — ${waxSealBadge}`}
                    purpose={subtitle ?? `${domainTag} surface status verified in Plate 2 Romanesque console.`}
                  >
                    <span
                      data-roman-wax-seal="true"
                      className="inline-flex items-center gap-1.5 rounded-full border-2 border-[#FCA5A5]/65 bg-linear-to-b from-[#A81C1C] via-[#7F1313] to-[#4A0808] px-2.5 py-0.5 font-display text-[9px] font-bold uppercase tracking-[0.14em] text-[#FFF1F2] shadow-[0_3px_10px_rgba(127,19,19,0.55),inset_0_1px_1px_rgba(254,202,202,0.55)]"
                    >
                      <img
                        src="/assets/romanesque/wax-seal-medallion.jpg"
                        alt=""
                        aria-hidden="true"
                        className="size-4 rounded-full object-cover ring-1 ring-[#FDE68A]/70"
                      />
                      <span>{waxSealBadge}</span>
                    </span>
                  </RomanTooltip>
                )}
              </div>
              {subtitle && (
                <p
                  className="mt-0.5 font-serif text-[11.5px] font-medium leading-snug text-[#3B2E1E] line-clamp-1"
                  style={{ textShadow: '0 1px 0 rgba(255, 255, 255, 0.7)' }}
                >
                  {subtitle}
                </p>
              )}
            </div>
          </div>

          {(gauges && gauges.length > 0) || actions ? (
            <div className="flex flex-wrap items-center gap-2 shrink-0">
              {gauges?.map((g) => (
                <RomanBrassGauge
                  key={g.label}
                  label={g.label}
                  valueText={g.valueText}
                  ratio={g.ratio}
                  tone={g.tone}
                  purpose={g.purpose}
                  onClick={g.onClick}
                />
              ))}
              {actions && (
                <div className="flex flex-wrap items-center gap-1.5 rounded-lg border border-[#A8842F]/55 bg-[#231B12]/90 px-2 py-1 text-[#F6F0E4] shadow-[inset_0_1px_2px_rgba(0,0,0,0.5)]">
                  {actions}
                </div>
              )}
            </div>
          ) : null}
        </div>

        {/* Right Domain-Varied Romanesque Relief */}
        <RomanFlutedPillar side="right" variant={reliefVariant} />
      </div>
    </header>
  );
}

/**
 * Plate 2 Embossed Crimson / Olive-Laurel / Gold Wax-Seal Status Pill (`<RomanWaxSealBadge>`).
 */
export function RomanWaxSealBadge({
  label,
  tone = 'crimson',
  className = '',
}: {
  label: string;
  tone?: 'crimson' | 'emerald' | 'gold';
  className?: string;
}) {
  const palette =
    tone === 'emerald'
      ? 'border-[#C59B27]/85 from-[#5B6B24] via-[#435118] to-[#2C360E] text-[#FBF5E6] shadow-[0_2px_7px_rgba(44,54,14,0.55),inset_0_1px_1px_rgba(243,229,171,0.45)]'
      : tone === 'gold'
        ? 'border-[#785312] from-[#F2D67B] via-[#D4AF37] to-[#9A741C] text-[#1A1205] shadow-[0_2px_6px_rgba(0,0,0,0.4),inset_0_1px_1px_rgba(255,250,225,0.8)]'
        : 'border-[#FCA5A5]/65 from-[#A81C1C] via-[#7F1313] to-[#4A0808] text-[#FFF1F2] shadow-[0_2px_7px_rgba(127,19,19,0.55),inset_0_1px_1px_rgba(254,202,202,0.5)]';

  return (
    <span
      data-roman-wax-seal="true"
      className={`inline-flex items-center gap-1.5 rounded-full border bg-linear-to-b px-2.5 py-0.5 font-display text-[9px] font-bold uppercase tracking-[0.12em] ${palette} ${className}`}
    >
      <img
        src="/assets/romanesque/wax-seal-medallion.jpg"
        alt=""
        aria-hidden="true"
        className="size-3.5 rounded-full object-cover ring-1 ring-current/40"
      />
      <span>{label}</span>
    </span>
  );
}
