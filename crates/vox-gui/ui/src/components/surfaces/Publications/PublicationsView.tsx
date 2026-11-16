import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invokeCmd } from '../../../transport';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { PUBLICATION_STAGES, groupByStage, PublicationManifest } from '../../../lib/pipeline';
import { useLabel } from '../../../hooks/useLanguage';
import {
  computeWorthinessAxes,
  draftManuscript,
  listPublicationClaims,
  recommendPublicationVenue,
  renderLatex,
  type PublicationClaimsDto,
  type VenueRecommendationDto,
} from '../../../api/scientia';
import { DirectPublicationDrawer } from '../Scientia/DirectPublicationDrawer';
import { WorthinessRadarPanel } from '../Scientia/WorthinessRadarPanel';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';

export function PublicationsView({ pushToast }: SurfaceDecoratorProps) {
  const [manifests, setManifests] = useState<PublicationManifest[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<PublicationManifest | null>(null);
  const [stageFilter, setStageFilter] = useState<'all' | 'published' | 'populated'>('all');

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setManifests(await invokeCmd<PublicationManifest[]>('list_publication_manifests', { limit: 200 }));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Publications load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoading(false);
    }
  }, [pushToast]);

  useEffect(() => { refresh(); }, [refresh]);

  const groups = groupByStage(manifests);
  const publishedCount = (groups['published']?.length ?? 0) + (groups['archived']?.length ?? 0);
  const populatedStageCount = PUBLICATION_STAGES.filter(
    (s) => (groups[s]?.length ?? 0) > 0,
  ).length;
  const visibleStages: readonly string[] =
    stageFilter === 'published'
      ? PUBLICATION_STAGES.filter((s) => s === 'published')
      : stageFilter === 'populated' && populatedStageCount > 0
        ? PUBLICATION_STAGES.filter((s) => (groups[s]?.length ?? 0) > 0)
        : PUBLICATION_STAGES;
  const visibleManifestCount = visibleStages.reduce<number>(
    (acc, s) => acc + (groups[s]?.length ?? 0),
    0,
  );

  return (
    <section className="@container/surface flex min-h-0 min-w-0 flex-col gap-3.5">
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE · PUBLICATIONS"
        title="Publication Pipeline & Archival Ledger"
        subtitle="Track manuscript drafts, verification gates, LaTeX rendering, and archival status across publication venues."
        waxSealBadge="ARCHIVAL READY"
        icon={<Icon.book className="size-5 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'MANIFESTS',
            valueText: `${manifests.length}`,
            ratio: manifests.length > 0 ? visibleManifestCount / manifests.length : 0,
            tone: 'gold',
            purpose: 'Total publication pipeline manifests',
            actionHint: 'Click to show all publication pipeline stages',
            active: stageFilter === 'all' && manifests.length > 0,
            onClick: () => setStageFilter('all'),
          },
          {
            label: 'PUBLISHED',
            valueText: `${publishedCount}`,
            ratio: manifests.length > 0 ? publishedCount / manifests.length : 0,
            tone: 'emerald',
            purpose: 'Published and released manuscripts out of total manifests',
            actionHint:
              stageFilter === 'published'
                ? 'Click to show all publication stages'
                : 'Click to filter board to published manuscripts',
            active: stageFilter === 'published',
            onClick: () => {
              setStageFilter((f) => (f === 'published' ? 'all' : 'published'));
              const firstPublished = groups['published']?.[0] ?? groups['archived']?.[0];
              if (firstPublished) setSelected(firstPublished);
            },
          },
          {
            label: 'STAGES',
            valueText: `${populatedStageCount}/${PUBLICATION_STAGES.length}`,
            ratio:
              PUBLICATION_STAGES.length > 0
                ? populatedStageCount / PUBLICATION_STAGES.length
                : 0,
            tone: 'gold',
            purpose: 'Pipeline verification stages currently holding manuscripts',
            actionHint:
              stageFilter === 'populated'
                ? 'Click to show all lifecycle stages'
                : 'Click to filter board to populated pipeline stages',
            active: stageFilter === 'populated',
            onClick: () => setStageFilter((f) => (f === 'populated' ? 'all' : 'populated')),
          },
        ]}
        actions={
          <button
            type="button"
            onClick={refresh}
            disabled={loading}
            className="rounded-lg border border-[#D4AF37]/40 bg-[#78350F]/20 px-3 py-1.5 font-display text-xs uppercase tracking-wider text-[#F3E5AB] hover:bg-[#78350F]/40 disabled:opacity-40 transition"
          >
            {loading ? 'Loading…' : 'Refresh'}
          </button>
        }
      />
      <BentoGrid preset="hero">
        <BentoCard span={selected ? 'hero-main' : 'full'} className="gap-2.5 border-[#D4AF37]/35">
          <div
            className={
              visibleStages.length === 1
                ? 'grid grid-cols-1 gap-2.5'
                : selected
                  ? 'grid grid-cols-1 gap-2.5 @sm/surface:grid-cols-2 @xl/surface:grid-cols-3 @4xl/surface:grid-cols-4'
                  : 'grid grid-cols-1 gap-2.5 @sm/surface:grid-cols-2 @xl/surface:grid-cols-3 @4xl/surface:grid-cols-4 @6xl/surface:grid-cols-7'
            }
          >
            {visibleStages.map(stage => (
              <div key={stage} className="min-w-0">
                <div className="mb-2 flex items-center justify-between gap-1">
                  <span className="truncate font-mono text-[10px] uppercase tracking-wide text-[#D4AF37]">{stage.replace(/_/g, ' ')}</span>
                  <span className="shrink-0 rounded-full border border-[#D4AF37]/30 bg-bg-surface px-1.5 font-mono text-[9px] text-[#F3E5AB]">{groups[stage].length}</span>
                </div>
                {groups[stage].length === 0 ? (
                  <div className="rounded-lg border border-dashed border-[#D4AF37]/20 p-2 text-center text-[10px] text-[#D6CEBF]/60">—</div>
                ) : (
                  <BentoScrollRegion
                    ariaLabel={`${stage.replace(/_/g, ' ')} stage manifests`}
                    className="max-h-60 space-y-2 pr-0.5"
                  >
                    {groups[stage].map(m => (
                      <button
                        type="button"
                        key={m.publication_id}
                        onClick={() => setSelected(m)}
                        className={`w-full rounded-lg border p-2 text-left transition-colors ${
                          selected?.publication_id === m.publication_id
                            ? 'border-[#D4AF37] bg-bg-elevated shadow-xs'
                            : 'border-[#D4AF37]/25 bg-bg-surface hover:border-[#D4AF37]/50'
                        }`}
                      >
                        <div className="truncate font-mono text-[11px] text-[#F3E5AB]">{m.publication_id}</div>
                        <div className="truncate text-[10px] text-[#D6CEBF]">{m.content_type}</div>
                      </button>
                    ))}
                  </BentoScrollRegion>
                )}
              </div>
            ))}
          </div>
        </BentoCard>

        {selected ? (
          <BentoCard span="hero-rail" className="border-[#D4AF37]/35 p-0">
            <PublicationDetail
              manifest={selected}
              onClose={() => setSelected(null)}
              pushToast={pushToast}
            />
          </BentoCard>
        ) : (
          <div data-testid="publications-pipeline-overview" className="col-span-full min-w-0">
            <BentoGrid preset="triad">
              <div className="min-w-0 rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3.5">
                <div className="flex flex-wrap items-center justify-between gap-1.5">
                  <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">IMRaD &amp; LaTeX</span>
                  <RomanWaxSealBadge label={`${manifests.length} manifests`} tone="gold" />
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
                  Select any card above to preview its generated IMRaD manuscript, compile LaTeX source, or inspect extracted epistemic claims.
                </p>
              </div>
              <div className="min-w-0 rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3.5">
                <div className="flex flex-wrap items-center justify-between gap-1.5">
                  <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Claim Verification</span>
                  <RomanWaxSealBadge label="MiniCheck" tone="gold" />
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
                  Every candidate paper undergoes automated claim extraction (<code className="text-[#F3E5AB]">vox scientia claims</code>) and confidence scoring before approval.
                </p>
              </div>
              <div className="min-w-0 rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3.5">
                <div className="flex flex-wrap items-center justify-between gap-1.5">
                  <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Archival Deposit</span>
                  <RomanWaxSealBadge label="DOI + SWHID" tone="emerald" />
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
                  Tracks Zenodo DOI reservation, ORCID attribution, and Software Heritage (SWHID) source archival across 7 lifecycle stages.
                </p>
              </div>
            </BentoGrid>
          </div>
        )}
      </BentoGrid>
    </section>
  );
}

/**
 * Drill-down panel for one publication (F4 / STUD-01 / STUD-02): its lifecycle state,
 * 6-axis worthiness radar, direct publication flywheel drawer, extracted claims
 * (`scientia_publication_claims`), and venue-routing recommendation (`scientia_publication_venue_recommend`).
 */
function PublicationDetail({
  manifest,
  onClose,
  pushToast,
}: {
  manifest: PublicationManifest;
  onClose: () => void;
  pushToast: SurfaceDecoratorProps['pushToast'];
}) {
  const [claims, setClaims] = useState<PublicationClaimsDto | null>(null);
  const [venue, setVenue] = useState<VenueRecommendationDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [previewText, setPreviewText] = useState<string | null>(null);
  const [previewKind, setPreviewKind] = useState<'imrad' | 'latex' | null>(null);
  const [replayScore, setReplayScore] = useState<number | undefined>(undefined);

  const load = useCallback(async () => {
    setBusy(true);
    setClaims(null);
    setVenue(null);
    try {
      const claimsPayload = await listPublicationClaims(manifest.publication_id);
      setClaims(claimsPayload);
      const venuePayload = await recommendPublicationVenue(manifest.content_type);
      setVenue(venuePayload);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Publication detail', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  }, [manifest.publication_id, manifest.content_type, pushToast]);

  useEffect(() => { load(); }, [load]);

  const worthinessAxes = computeWorthinessAxes({
    totalClaims: claims?.claim_count ?? 0,
    supportedClaims: (claims?.claims ?? []).filter((c) =>
      (c.verdict ?? c.verdict_label ?? '').toLowerCase().includes('support'),
    ).length,
    contradictedClaims: (claims?.claims ?? []).filter((c) => {
      const v = (c.verdict ?? c.verdict_label ?? '').toLowerCase();
      return v.includes('contradict') || v.includes('refut');
    }).length,
    replayScore,
  });

  // Lifecycle stepper: the ordered stages a publication passes through. The
  // current stage is derived from the manifest state via the same stage grouping
  // the board uses.
  const _stageMap = groupByStage([manifest]);
  const stageIdx = PUBLICATION_STAGES.findIndex(s => (_stageMap[s]?.length ?? 0) > 0);
  const resolvedStageIdx = stageIdx === -1 ? 0 : stageIdx;

  return (
    <div className="min-w-0 p-3.5 @2xl/surface:p-4">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate font-mono text-sm text-[#F3E5AB]">{manifest.publication_id}</div>
          <div className="font-mono text-[11px] text-[#D6CEBF]">{manifest.content_type} · {manifest.state}</div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            data-testid="pub-preview-imrad-btn"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                const res = await draftManuscript(manifest.publication_id);
                setPreviewKind('imrad');
                setPreviewText(res.markdown);
              } catch (err) {
                pushToast({ tone: 'warn', title: 'IMRaD preview failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
              } finally {
                setBusy(false);
              }
            }}
            className="rounded-lg border border-[#D4AF37]/40 bg-[#78350F]/20 px-3 py-1.5 text-xs text-[#F3E5AB] hover:bg-[#78350F]/40 transition"
          >
            Preview IMRaD
          </button>
          <button
            type="button"
            data-testid="pub-preview-latex-btn"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                const res = await renderLatex(manifest.publication_id);
                setPreviewKind('latex');
                setPreviewText(res.latex);
              } catch (err) {
                pushToast({ tone: 'warn', title: 'LaTeX preview failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
              } finally {
                setBusy(false);
              }
            }}
            className="rounded-lg border border-[#D4AF37]/40 bg-[#78350F]/20 px-3 py-1.5 text-xs text-[#F3E5AB] hover:bg-[#78350F]/40 transition"
          >
            Preview LaTeX
          </button>
          <button type="button" onClick={load} disabled={busy}
            className="rounded-lg border border-[#D4AF37]/30 bg-bg-elevated px-3 py-1.5 text-xs text-[#D6CEBF] hover:bg-[#78350F]/30 hover:text-[#F3E5AB]">
            {busy ? 'Loading…' : 'Reload'}
          </button>
          <button type="button" onClick={onClose}
            className="rounded-lg border border-[#D4AF37]/30 bg-bg-elevated px-3 py-1.5 text-xs text-[#D6CEBF] hover:bg-[#78350F]/30 hover:text-[#F3E5AB]">
            Close
          </button>
        </div>
      </div>

      {previewText && (
        <div data-testid="publication-artifact-preview" className="mb-4 rounded-lg border border-[#D4AF37]/30 bg-bg-surface p-3">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <span className="font-mono text-xs uppercase tracking-wider text-brass">
              {previewKind === 'latex' ? 'LaTeX Document Preview' : 'IMRaD Manuscript Preview'}
            </span>
            <button
              type="button"
              onClick={() => {
                setPreviewText(null);
                setPreviewKind(null);
              }}
              className="font-mono text-[11px] text-text-secondary hover:text-text-primary"
            >
              Close Preview
            </button>
          </div>
          <BentoScrollRegion ariaLabel="Manuscript preview content" className="max-h-52">
            <pre className="whitespace-pre-wrap font-mono text-[11px] text-text-primary">
              {previewText}
            </pre>
          </BentoScrollRegion>
        </div>
      )}

      {/* Lifecycle stepper */}
      <div className="mb-4 flex flex-wrap items-center gap-1.5">
        {PUBLICATION_STAGES.map((s, i) => (
          <React.Fragment key={s}>
            <span className={`rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider ${
              i <= resolvedStageIdx ? 'border border-[#D4AF37]/50 bg-bg-elevated text-text-primary' : 'bg-bg-surface text-text-secondary'
            }`}>{s.replace(/_/g, ' ')}</span>
            {i < PUBLICATION_STAGES.length - 1 && <span className="text-[#D4AF37]/40">›</span>}
          </React.Fragment>
        ))}
      </div>

      <div className="mb-4 space-y-3">
        <WorthinessRadarPanel axes={worthinessAxes} compact />
        <DirectPublicationDrawer
          publicationId={manifest.publication_id}
          pushToast={pushToast}
          compact
          onReplayVerified={(score) => setReplayScore(score)}
          onFlywheelAdvanced={() => {
            void load();
          }}
        />
      </div>

      <div className="grid grid-cols-1 gap-3 @xl/surface:grid-cols-2">
        {/* Claims */}
        <div className="min-w-0">
          <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-brass">
            Claims {claims ? `(${claims.claim_count})` : ''}
          </div>
          {!claims || claims.claims.length === 0 ? (
            <div className="rounded-lg border border-[#D4AF37]/25 bg-bg-surface p-3 text-[12px] text-text-secondary">
              {busy ? 'Loading…' : 'No extracted claims. Run claim extraction first.'}
            </div>
          ) : (
            <BentoScrollRegion ariaLabel="Extracted manuscript claims" className="max-h-52">
              <ul className="space-y-1">
                {claims.claims.map((c, i) => (
                  <li key={c.id ?? i} className="rounded-lg border border-[#D4AF37]/25 bg-bg-surface p-2">
                    <div className="text-[12px] text-text-primary">{c.text ?? '(no text)'}</div>
                    {c.verdict_label && (
                      <div className="mt-0.5 font-mono text-[10px] text-brass">
                        {c.verdict_label}{typeof c.confidence === 'number' ? ` · ${c.confidence.toFixed(2)}` : ''}
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            </BentoScrollRegion>
          )}
        </div>

        {/* Venue routing */}
        <div>
          <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-brass">Venue routing</div>
          {!venue ? (
            <div className="rounded-lg border border-[#D4AF37]/25 bg-bg-surface p-3 text-[12px] text-text-secondary">
              {busy ? 'Loading…' : `No routing for class "${manifest.content_type}".`}
            </div>
          ) : (
            <div className="space-y-2 rounded-lg border border-[#D4AF37]/25 bg-bg-surface p-3">
              <div className="flex flex-wrap gap-1.5">
                {venue.recommended_venues.length === 0 ? (
                  <span className="font-mono text-[11px] text-text-secondary">No recommended venues.</span>
                ) : venue.recommended_venues.map(v => (
                  <span key={v} className="rounded-sm border border-[#D4AF37]/35 bg-bg-elevated px-2 py-0.5 font-mono text-[11px] text-text-primary">{v}</span>
                ))}
              </div>
              <div className="grid grid-cols-2 gap-x-3 gap-y-1 font-mono text-[11px] text-text-muted">
                <span>Reply window</span><span className="text-text-secondary">{venue.reply_window_days}d</span>
                <span>Neg-result quota</span><span className="text-text-secondary">{venue.negative_result_quota}</span>
                <span>Critic allowed</span><span className={venue.critic_allowed ? 'text-emerald-300' : 'text-amber-300'}>{venue.critic_allowed ? 'yes' : 'no'}</span>
                <span>Atlas gate</span><span className="text-text-secondary">{venue.atlas_gate_applies ? 'applies' : '—'}</span>
              </div>
              {!venue.critic_allowed && (
                <div className="rounded-sm border border-amber-500/20 bg-amber-500/4 p-2 text-[11px] text-amber-300/90">
                  Venue forbids LLM-critic approvals — add a second human approver.
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

