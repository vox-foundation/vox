import React, { useState } from 'react';
import { ActivitySurface } from '../Activity/ActivitySurface';
import { DiscoveryInbox } from '../Scientia/DiscoveryInbox';
import { DiscoveryReview } from '../Scientia/DiscoveryReview';
import { ArchivePanel } from '../Scientia/ArchivePanel';
import { DISCOVERY_PRESET_SEED_KEY } from '../../../lib/navigation';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';
import type { Toast } from '../../../types/tauri';

export type DiscoveryPreset = 'timeline' | 'inbox' | 'review' | 'archive';

const PRESETS: Array<{ id: DiscoveryPreset; label: string; summary: string }> = [
  { id: 'timeline', label: 'Timeline', summary: 'Live workspace and agent chronicle events.' },
  { id: 'inbox', label: 'Inbox', summary: 'Untriaged research findings awaiting review.' },
  { id: 'review', label: 'Review', summary: 'Epistemic confidence gate and critic triage.' },
  { id: 'archive', label: 'Archive', summary: 'Zenodo DOI and Software Heritage provenance.' },
];

function consumeSeed(): DiscoveryPreset {
  try {
    const seed = window.localStorage.getItem(DISCOVERY_PRESET_SEED_KEY);
    if (seed === 'inbox' || seed === 'review' || seed === 'archive') {
      window.localStorage.removeItem(DISCOVERY_PRESET_SEED_KEY);
      return seed;
    }
  } catch {
    /* localStorage unavailable */
  }
  return 'timeline';
}

export interface DiscoverySurfaceProps {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
  condensed?: boolean;
}

/**
 * One Discovery surface (view key `activity`) absorbing the four former
 * activity clones: Timeline (activity_query), Inbox, Review, Archive
 * (gui-ia-blueprint §4 MERGE: archive-panel/discovery-inbox/discovery-review → activity).
 */
export function DiscoverySurface({ pushToast, gamifyEnabled, condensed }: DiscoverySurfaceProps) {
  const [preset, setPreset] = useState<DiscoveryPreset>(consumeSeed);
  const [visitedPresets, setVisitedPresets] = useState<Set<DiscoveryPreset>>(
    () => new Set([preset]),
  );

  const selectPreset = (next: DiscoveryPreset) => {
    setPreset(next);
    setVisitedPresets((prev) => {
      if (prev.has(next)) return prev;
      const copy = new Set(prev);
      copy.add(next);
      return copy;
    });
  };

  if (condensed) {
    // Same `preset` state the tab strip already tracks — the four nested
    // surfaces (Timeline/Inbox/Review/Archive) are each substantial widgets
    // that don't fit narrow, so condensed just names which one is active.
    const activeLabel = PRESETS.find(p => p.id === preset)?.label ?? preset;
    return (
      <div className="p-2 text-[11px] text-text-muted">
        <div className="mb-1 text-xs font-semibold uppercase tracking-wider text-text-primary">Discovery</div>
        <div>{activeLabel}</div>
      </div>
    );
  }

  const presetIndex = Math.max(0, PRESETS.findIndex((p) => p.id === preset));
  const provenanceChecksVisited = [
    visitedPresets.has('review'),
    visitedPresets.has('archive'),
  ].filter(Boolean).length;

  return (
    <div className="@container/surface flex min-h-0 min-w-0 flex-col gap-3.5">
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE · DISCOVERY"
        title="Epistemic Activity & Discovery Ledger"
        subtitle="TIMELINE · INBOX · REVIEW · ARCHIVAL PROVENANCE"
        waxSealBadge="CHRONICLE LIVE"
        icon={<Icon.bolt className="size-4 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'Presets',
            valueText: `${visitedPresets.size}/${PRESETS.length}`,
            ratio: PRESETS.length > 0 ? visitedPresets.size / PRESETS.length : 0,
            tone: 'gold',
            purpose: 'Discovery workflow lanes inspected in this session',
            actionHint: 'Click to jump to the next unvisited discovery lane',
            active: visitedPresets.size === PRESETS.length,
            onClick: () => {
              const nextUnvisited = PRESETS.find((p) => !visitedPresets.has(p.id));
              selectPreset(nextUnvisited ? nextUnvisited.id : 'timeline');
            },
          },
          {
            label: 'Active',
            valueText: preset.toUpperCase(),
            ratio: PRESETS.length > 0 ? (presetIndex + 1) / PRESETS.length : 0,
            tone: 'emerald',
            purpose: 'Current epistemic discovery pipeline stage',
            actionHint: 'Click to advance to the next discovery preset tab',
            active: preset !== 'timeline',
            onClick: () => selectPreset(PRESETS[(presetIndex + 1) % PRESETS.length]!.id),
          },
          {
            label: 'Provenance',
            valueText: `${provenanceChecksVisited}/2 GATES`,
            ratio: provenanceChecksVisited / 2,
            tone: 'gold',
            purpose: 'Review and Archival provenance lanes inspected in this session',
            actionHint:
              preset === 'review'
                ? 'Click to switch to Archive provenance ledger'
                : 'Click to open Discovery Review gate',
            active: preset === 'review' || preset === 'archive',
            onClick: () => selectPreset(preset === 'review' ? 'archive' : 'review'),
          },
        ]}
        actions={
          <div
            role="tablist"
            aria-label="Discovery presets"
            className="flex flex-wrap gap-1 rounded-lg border border-[#C59B27]/35 bg-bg-elevated p-1"
          >
            {PRESETS.map(p => (
              <button
                key={p.id}
                type="button"
                role="tab"
                aria-selected={preset === p.id}
                onClick={() => selectPreset(p.id)}
                className={`rounded-md px-3 py-1 font-display text-[10px] uppercase tracking-[0.14em] transition ${
                  preset === p.id
                    ? 'border border-[#D4AF37]/60 bg-[#D4AF37]/20 text-[#F3E5AB]'
                    : 'text-[#C8B99E] hover:bg-[#D4AF37]/10 hover:text-[#F3E5AB]'
                }`}
              >
                {p.label}
              </button>
            ))}
          </div>
        }
      />

      <BentoGrid preset="hero" className="flex-1">
        <BentoCard span="hero-main" className="min-h-0 flex-1 overflow-hidden p-3">
          <BentoScrollRegion ariaLabel={`Discovery ${preset} feed`} className="flex-1 pr-1">
            {preset === 'timeline' && <ActivitySurface pushToast={pushToast} gamifyEnabled={gamifyEnabled} />}
            {preset === 'inbox' && <DiscoveryInbox pushToast={pushToast} gamifyEnabled={gamifyEnabled} />}
            {preset === 'review' && <DiscoveryReview pushToast={pushToast} gamifyEnabled={gamifyEnabled} />}
            {preset === 'archive' && <ArchivePanel pushToast={pushToast} gamifyEnabled={gamifyEnabled} />}
          </BentoScrollRegion>
        </BentoCard>

        <BentoCard span="hero-rail" className="gap-3">
          <div className="flex flex-wrap items-center justify-between gap-1.5">
            <span className="font-display text-[11px] uppercase tracking-wider text-brass">
              Preset Explorer &amp; Provenance
            </span>
            <span className="font-mono text-[10px] text-text-muted">
              {visitedPresets.size}/{PRESETS.length} visited
            </span>
          </div>
          <BentoScrollRegion ariaLabel="Discovery provenance and lane explorer" className="max-h-[65vh] space-y-2">
            {PRESETS.map((p) => {
              const isCurrent = preset === p.id;
              const seen = visitedPresets.has(p.id);
              return (
                <button
                  key={p.id}
                  type="button"
                  aria-label={`Focus ${p.label} lane`}
                  onClick={() => selectPreset(p.id)}
                  className={`flex w-full flex-col gap-1 rounded-lg border p-2.5 text-left transition-colors ${
                    isCurrent
                      ? 'border-brass/60 bg-bg-elevated text-text-primary'
                      : 'border-border-subtle bg-overlay-subtle text-text-secondary hover:border-brass/35'
                  }`}
                >
                  <div className="flex flex-wrap items-center justify-between gap-1.5">
                    <span className="font-display text-[11px] uppercase tracking-wider text-text-primary">
                      {p.label} Lane
                    </span>
                    <span className="rounded border border-border-subtle bg-bg-surface px-1.5 py-0.5 font-mono text-[9px] text-text-muted">
                      {isCurrent ? 'active' : seen ? 'inspected' : 'pending'}
                    </span>
                  </div>
                  <span className="text-[11px] leading-snug text-text-muted">{p.summary}</span>
                </button>
              );
            })}
            <div className="rounded-lg border border-[#D4AF37]/30 bg-bg-elevated/60 p-2.5 text-[11px] text-text-secondary">
              <div className="font-display text-[10px] uppercase tracking-wider text-brass">
                Archival Provenance Gate
              </div>
              <p className="mt-1 leading-relaxed text-text-muted">
                Findings promoted in Review enter the Knowledge Base and track Zenodo DOI + SWHID archival hashes in Archive.
              </p>
            </div>
          </BentoScrollRegion>
        </BentoCard>
      </BentoGrid>
    </div>
  );
}
