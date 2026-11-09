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
  /** Optional heading level override (default 1; pass 2 when surface tests expect h2) */
  headingLevel?: 1 | 2;
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
  headingLevel = 1,
  className = '',
}: RomanSurfaceHeaderProps) {
  const HeadingTag = headingLevel === 2 ? 'h2' : 'h1';
  const tagUpper = domainTag.toUpperCase();
  const reliefVariant: 'column' | 'laurel' | 'rosette' | 'mosaic' =
    tagUpper.includes('KNOWLEDGE') || tagUpper.includes('EXPLORER')
      ? 'column'
      : tagUpper.includes('COMPUTE') || tagUpper.includes('MARKET') || tagUpper.includes('MERCATUS')
        ? 'rosette'
        : tagUpper.includes('WORKSPACE') || tagUpper.includes('COMMANDS') || tagUpper.includes('REPOSITORY')
          ? 'mosaic'
          : 'laurel';

  const badgeUpper = waxSealBadge?.toUpperCase() ?? '';
  const isErrorSeal =
    badgeUpper.includes('FAIL') ||
    badgeUpper.includes('ERROR') ||
    badgeUpper.includes('DENIED') ||
    badgeUpper.includes('CRITICAL') ||
    badgeUpper.includes('QUARANTINE');

  return (
    <header
      data-roman-frieze="true"
      style={{
        backgroundImage:
          'linear-gradient(115deg, rgba(255,252,245,0) 35%, rgba(255,248,225,0.48) 50%, rgba(255,252,245,0) 65%), linear-gradient(90deg, rgba(250,246,238,0.92) 0%, rgba(242,233,218,0.82) 48%, rgba(250,246,238,0.92) 100%), url(/assets/romanesque/carrara-acanthus-header-band.jpg)',
        backgroundSize: '240% 100%, 100% 100%, cover',
        backgroundPosition: 'center, center, center',
      }}
      className={`vox-animate-marble-sheen relative overflow-hidden rounded-xl border-2 border-[#C59B27] px-2.5 py-2 shadow-[0_6px_18px_rgba(28,20,12,0.22),inset_0_2px_0_rgba(255,255,255,0.95),inset_0_-2px_0_rgba(168,132,47,0.45)] ${className}`}
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
                <span className="font-display text-xs font-bold tracking-wider text-[#1C140C]">
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
                <HeadingTag
                  className="font-display text-sm sm:text-base font-bold tracking-[0.06em] text-[#1C140C] truncate"
                  style={{ textShadow: '0 1px 0 rgba(255, 255, 255, 0.85)' }}
                >
                  {title}
                </HeadingTag>
                {waxSealBadge && (
                  <RomanTooltip
                    title={`${title} — ${waxSealBadge}`}
                    purpose={subtitle ?? `${domainTag} surface status verified in Romanesque console.`}
                  >
                    <RomanWaxSealBadge
                      label={waxSealBadge}
                      tone={isErrorSeal ? 'crimson' : 'emerald'}
                    />
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
                <div className="vox-header-actions-tray flex flex-wrap items-center gap-1.5 rounded-lg border-1.5 border-[#B88E28] bg-linear-to-b from-[#FAF6EE] to-[#EFE6D5] px-2 py-1 text-[#1C140C] shadow-[inset_0_1px_0_rgba(255,255,255,0.92),0_2px_6px_rgba(28,20,12,0.16)]">
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
 * Roman Imperial Emerald & Gilded Laurel Wreath Working Status Signet (`<RomanWaxSealBadge>`).
 * Uses high-visibility Imperial Emerald/Malachite green (`#156B3B` -> `#0D4A27`) with a burnished
 * gold rim (`#D4AF37`) and crisp vector Corona Triumphalis laurel wreath so active/working status
 * is unmistakably green and legible at any resolution. Crimson is reserved strictly for errors.
 */
export function RomanWaxSealBadge({
  label,
  tone = 'emerald',
  className = '',
}: {
  label: string;
  tone?: 'crimson' | 'emerald' | 'gold';
  className?: string;
}) {
  const palette =
    tone === 'emerald'
      ? 'border-[#D4AF37] from-[#1E7E46] via-[#156336] to-[#0D4524] text-[#F4FBF6] shadow-[0_2px_8px_rgba(21,99,54,0.45),0_0_8px_rgba(34,139,78,0.28),inset_0_1px_1px_rgba(167,243,208,0.45)]'
      : tone === 'gold'
        ? 'border-[#785312] from-[#F2D67B] via-[#D4AF37] to-[#9A741C] text-[#1A1205] shadow-[0_2px_6px_rgba(0,0,0,0.32),inset_0_1px_1px_rgba(255,250,225,0.85)]'
        : 'border-[#FCA5A5]/65 from-[#A81C1C] via-[#7F1313] to-[#4A0808] text-[#FFF1F2] shadow-[0_2px_7px_rgba(127,19,19,0.55),inset_0_1px_1px_rgba(254,202,202,0.5)]';

  return (
    <span
      data-roman-wax-seal="true"
      data-seal-tone={tone}
      className={`inline-flex items-center gap-1.5 rounded-full border-1.5 bg-linear-to-b px-2.5 py-0.5 font-display text-[9px] font-bold uppercase tracking-[0.13em] ${palette} ${className}`}
    >
      {tone === 'emerald' ? (
        <span className="relative inline-flex size-4 shrink-0 items-center justify-center rounded-full border border-[#F3E5AB] bg-radial from-[#34D399] via-[#15803D] to-[#064E3B] shadow-[0_0_6px_rgba(52,211,153,0.6)]">
          <svg viewBox="0 0 20 20" className="size-3.5 overflow-visible" aria-hidden="true">
            <circle cx="10" cy="10" r="8.2" fill="none" stroke="#F3E5AB" strokeWidth="1.2" strokeDasharray="2.2 1.1" />
            <path d="M6.2 10.3 L8.9 13 L14.1 7.4" fill="none" stroke="#FFFDF5" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </span>
      ) : (
        <img
          src={tone === 'crimson' ? '/assets/romanesque/wax-seal-medallion.jpg' : '/assets/romanesque/roman-emerald-laurel-seal.jpg'}
          alt=""
          aria-hidden="true"
          className="size-3.5 rounded-full object-cover ring-1 ring-current/40"
        />
      )}
      <span>{label}</span>
    </span>
  );
}
