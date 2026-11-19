import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invokeCmd, listenScientiaQueue } from '../../../transport';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { PUBLICATION_STAGES, groupByStage, PublicationManifest } from '../../../lib/pipeline';
import {
  ARXIV_SUBJECT_OPTIONS,
  FINDING_CLASS_OPTIONS,
  PUBLICATION_TIER_OPTIONS,
  SPDX_LICENSE_OPTIONS,
  advancePublicationStage,
  classifyPublication,
  computeWorthinessAxes,
  draftManuscript,
  getPublicationArxivReadiness,
  listPublicationClaims,
  recommendPublicationVenue,
  renderArxivBundle,
  renderLatex,
  type ArxivReadinessReportDto,
  type PublicationClaimsDto,
  type VenueRecommendationDto,
} from '../../../api/scientia';
import { DirectPublicationDrawer } from '../Scientia/DirectPublicationDrawer';
import { WorthinessRadarPanel } from '../Scientia/WorthinessRadarPanel';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';

const STAGE_ACTIONS: ReadonlyArray<{ stage: string; actionLabel: string }> = [
  { stage: 'doi_reserved', actionLabel: 'Reserve DOI' },
  { stage: 'orcid_attributed', actionLabel: 'Bind ORCID' },
  { stage: 'approved', actionLabel: 'Approve for arXiv' },
  { stage: 'submitted', actionLabel: 'Submit Validated Bundle' },
  { stage: 'published', actionLabel: 'Record arXiv Publication' },
];

type PreviewTabId = 'imrad' | 'latex' | 'bibtex' | 'metadata' | 'checklist';

function displayManifestTitle(m: PublicationManifest): string {
  const raw = m.title?.trim();
  const idLower = m.publication_id.trim().toLowerCase();
  if (
    raw &&
    raw.length >= 8 &&
    raw.toLowerCase() !== idLower &&
    !raw.toLowerCase().startsWith('scientia finding') &&
    !raw.toLowerCase().startsWith('pub-')
  ) {
    return raw;
  }
  const cleanTokens = m.publication_id
    .replace(/^(pub|finding|commit)-/i, '')
    .split(/[-_]+/)
    .filter((tok) => tok && !/^[0-9a-f]{8,}$/i.test(tok))
    .map((tok) => tok.charAt(0).toUpperCase() + tok.slice(1));
  const subj = cleanTokens.length > 0 ? cleanTokens.join(' ') : m.publication_id;
  const clsLabel =
    FINDING_CLASS_OPTIONS.find((c) => c.id === (m.candidate_class ?? m.content_type))?.label ??
    'Empirical Study';
  return `${subj} · ${clsLabel}`;
}

export function PublicationsView({ pushToast }: SurfaceDecoratorProps) {
  const [manifests, setManifests] = useState<PublicationManifest[]>([]);
  const [loading, setLoading] = useState(false);
  const inFlightRef = useRef(false);
  const primedRef = useRef(false);
  const [selectedId, setSelectedId] = useState<string | null>(() => {
    try {
      const seed = window.localStorage.getItem('vox_selected_publication_id');
      if (seed) {
        window.localStorage.removeItem('vox_selected_publication_id');
        return seed;
      }
    } catch {
      /* localStorage unavailable */
    }
    return null;
  });
  const [stageFilter, setStageFilter] = useState<'all' | 'published' | 'populated'>('all');
  const [tierFilter, setTierFilter] = useState<string>('all');
  const [classFilter, setClassFilter] = useState<string>('all');
  const [archiveFilter, setArchiveFilter] = useState<string>('all');

  const refresh = useCallback(async (opts?: { silent?: boolean }) => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    const showSpinner = !opts?.silent && !primedRef.current;
    if (showSpinner) setLoading(true);
    try {
      const rows = await invokeCmd<PublicationManifest[]>('list_publication_manifests', {
        limit: 200,
      });
      setManifests(Array.isArray(rows) ? rows : []);
      primedRef.current = true;
    } catch (err) {
      if (!opts?.silent) {
        pushToast({
          tone: 'warn',
          title: 'Publications load failed',
          body: sanitizeErrorForToast(err),
          cause: 'backend-error',
        });
      }
    } finally {
      if (showSpinner) setLoading(false);
      inFlightRef.current = false;
    }
  }, [pushToast]);

  useEffect(() => {
    void refresh();
    let unlisten: (() => void) | undefined;
    listenScientiaQueue(() => {
      void refresh({ silent: true });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        /* outside Tauri — visibility-gated fallback poll below handles updates */
      });
    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        void refresh({ silent: true });
      }
    };
    const onFocus = () => void refresh({ silent: true });
    document.addEventListener('visibilitychange', onVisible);
    window.addEventListener('focus', onFocus);
    const intervalId = window.setInterval(() => {
      if (document.visibilityState !== 'hidden') {
        void refresh({ silent: true });
      }
    }, 12_000);
    return () => {
      unlisten?.();
      document.removeEventListener('visibilitychange', onVisible);
      window.removeEventListener('focus', onFocus);
      window.clearInterval(intervalId);
    };
  }, [refresh]);

  useEffect(() => {
    const onNavigate = (e: Event) => {
      const detail = (e as CustomEvent<{ view?: string; publicationId?: string }>).detail;
      if (detail?.view === 'publications' && detail.publicationId?.trim()) {
        setSelectedId(detail.publicationId.trim());
        void refresh({ silent: true });
      }
    };
    window.addEventListener('vox://navigate-surface', onNavigate as EventListener);
    return () =>
      window.removeEventListener('vox://navigate-surface', onNavigate as EventListener);
  }, [refresh]);

  const filteredManifests = useMemo(() => {
    return manifests.filter((m) => {
      if (tierFilter !== 'all') {
        const tier = m.publication_tier ?? 'arxiv_preprint';
        if (tier !== tierFilter) return false;
      }
      if (classFilter !== 'all') {
        const cls = m.candidate_class ?? m.content_type;
        if (cls !== classFilter) return false;
      }
      if (archiveFilter !== 'all') {
        const cat = m.arxiv_primary_category ?? '';
        if (cat !== archiveFilter) return false;
      }
      return true;
    });
  }, [manifests, tierFilter, classFilter, archiveFilter]);

  const selected = useMemo(
    () => manifests.find((m) => m.publication_id === selectedId) ?? null,
    [manifests, selectedId],
  );

  const groups = groupByStage(filteredManifests);
  const allGroups = groupByStage(manifests);
  const publishedCount =
    (allGroups['published']?.length ?? 0) + (allGroups['archived']?.length ?? 0);
  const arxivReadyCount = manifests.filter((m) => m.arxiv_ready).length;
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
  const hasFacetFilter =
    tierFilter !== 'all' || classFilter !== 'all' || archiveFilter !== 'all';

  return (
    <section className="@container/surface flex min-h-0 min-w-0 flex-col gap-3.5">
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE · PUBLICATIONS"
        title="Publication Classification, 7-Stage Control & arXiv.org Ledger"
        subtitle="Classify scholarly tier and arXiv subject taxonomy, autofill zero-stub IMRaD manuscripts and BibTeX, verify 12-point readiness gates, and advance manuscripts across all 7 lifecycle stages."
        waxSealBadge={arxivReadyCount > 0 ? `${arxivReadyCount} ARXIV READY` : 'ARCHIVAL LEDGER'}
        icon={<Icon.book className="size-5 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'MANIFESTS',
            valueText: `${manifests.length}`,
            ratio: manifests.length > 0 ? visibleManifestCount / manifests.length : 0,
            tone: 'gold',
            purpose: 'Total scholarly publication manifests',
            actionHint: 'Click to show all lifecycle stages',
            active: stageFilter === 'all' && manifests.length > 0,
            onClick: () => setStageFilter('all'),
          },
          {
            label: 'ARXIV READY',
            valueText: `${arxivReadyCount}`,
            ratio: manifests.length > 0 ? arxivReadyCount / manifests.length : 0,
            tone: 'emerald',
            purpose: 'Manifests meeting full IMRaD and BibTeX arXiv readiness',
            actionHint: 'Click to view populated lifecycle stages',
            active: stageFilter === 'populated',
            onClick: () => setStageFilter((f) => (f === 'populated' ? 'all' : 'populated')),
          },
          {
            label: 'PUBLISHED',
            valueText: `${publishedCount}`,
            ratio: manifests.length > 0 ? publishedCount / manifests.length : 0,
            tone: 'emerald',
            purpose: 'Published and arXiv-archived manuscripts',
            actionHint:
              stageFilter === 'published'
                ? 'Click to show all publication stages'
                : 'Click to filter board to published manuscripts',
            active: stageFilter === 'published',
            onClick: () => {
              setStageFilter((f) => (f === 'published' ? 'all' : 'published'));
              const firstPublished = allGroups['published']?.[0] ?? allGroups['archived']?.[0];
              if (firstPublished) setSelectedId(firstPublished.publication_id);
            },
          },
        ]}
      />

      {/* Recessed Architectural Publications Console Frame */}
      <div className="vox-recessed-console-frame p-3 space-y-3">
      {/* Multi-Dimensional Classification Filter Bar */}
      <div
        data-testid="publications-filter-bar"
        className="vox-recessed-tablet flex flex-wrap items-center gap-2.5 p-3"
      >
        <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-primary">
          Classification Filters
        </span>
        <div className="min-w-[165px]">
          <VoxSelect
            aria-label="Filter by publication tier"
            data-testid="filter-publication-tier"
            value={tierFilter}
            onValueChange={setTierFilter}
            options={[
              { value: 'all', label: 'All Publication Tiers' },
              ...PUBLICATION_TIER_OPTIONS.map((t) => ({ value: t.id, label: t.label })),
            ]}
          />
        </div>
        <div className="min-w-[175px]">
          <VoxSelect
            aria-label="Filter by finding class"
            data-testid="filter-finding-class"
            value={classFilter}
            onValueChange={setClassFilter}
            options={[
              { value: 'all', label: 'All Finding Classes' },
              ...FINDING_CLASS_OPTIONS.map((c) => ({ value: c.id, label: c.label })),
            ]}
          />
        </div>
        <div className="min-w-[185px]">
          <VoxSelect
            aria-label="Filter by arXiv subject"
            data-testid="filter-arxiv-category"
            value={archiveFilter}
            onValueChange={setArchiveFilter}
            options={[
              { value: 'all', label: 'All arXiv Categories' },
              ...ARXIV_SUBJECT_OPTIONS.map((a) => ({ value: a.code, label: a.name })),
            ]}
          />
        </div>
        {hasFacetFilter && (
          <button
            type="button"
            data-slot="button"
            onClick={() => {
              setTierFilter('all');
              setClassFilter('all');
              setArchiveFilter('all');
            }}
            className="inline-flex h-8 items-center rounded-lg border border-border-strong bg-bg-surface px-3 text-xs font-medium text-text-primary transition-colors hover:border-brass"
          >
            Reset Filters
          </button>
        )}
        <span className="ml-auto font-mono text-[11px] text-text-secondary">
          {filteredManifests.length}/{manifests.length} shown
        </span>
      </div>

      <BentoGrid preset="hero">
        <BentoCard
          span={selected ? 'hero-main' : 'full'}
          className="vox-marble-tablet gap-2.5 border-(--color-gold-500)/55"
        >
          <div
            className={
              visibleStages.length === 1
                ? 'grid grid-cols-1 gap-2.5'
                : selected
                  ? 'grid grid-cols-1 gap-2.5 @sm/surface:grid-cols-2 @xl/surface:grid-cols-3 @4xl/surface:grid-cols-4'
                  : 'grid grid-cols-1 gap-2.5 @sm/surface:grid-cols-2 @xl/surface:grid-cols-3 @4xl/surface:grid-cols-4 @6xl/surface:grid-cols-7'
            }
          >
            {visibleStages.map((stage) => (
              <div key={stage} className="min-w-0">
                <div className="mb-2 flex items-center justify-between gap-1">
                  <span className="truncate font-mono text-[11px] font-semibold uppercase tracking-wide text-text-primary">
                    {stage.replace(/_/g, ' ')}
                  </span>
                  <span className="shrink-0 rounded-full border border-(--color-gold-500)/50 bg-bg-elevated px-2 py-0.5 font-mono text-[11px] font-semibold text-text-primary">
                    {groups[stage].length}
                  </span>
                </div>
                {groups[stage].length === 0 ? (
                  <div className="rounded-lg border border-dashed border-border-strong p-2 text-center text-[11px] text-text-secondary">
                    —
                  </div>
                ) : (
                  <BentoScrollRegion
                    ariaLabel={`${stage.replace(/_/g, ' ')} stage manifests`}
                    className="max-h-72 space-y-2 pr-0.5"
                  >
                    {groups[stage].map((m) => {
                      const cat = m.arxiv_primary_category;
                      const tier = m.publication_tier;
                      const score = m.readiness_score;
                      return (
                        <button
                          type="button"
                          data-slot="manifest-card"
                          key={m.publication_id}
                          onClick={() => setSelectedId(m.publication_id)}
                          className={`w-full rounded-lg border p-2.5 text-left transition-colors ${
                            selected?.publication_id === m.publication_id
                              ? 'border-(--color-gold-400) bg-bg-elevated shadow-xs'
                              : 'border-border-strong bg-bg-elevated/85 hover:border-(--color-gold-500)/70'
                          }`}
                        >
                          <div className="truncate font-serif text-xs font-semibold text-text-primary">
                            {displayManifestTitle(m)}
                          </div>
                          <div className="truncate font-mono text-[11px] font-semibold text-brass">
                            {m.publication_id}
                          </div>
                          <div className="mt-1 flex flex-wrap items-center gap-1">
                            <span className="rounded border border-border-strong bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] text-text-secondary">
                              {m.candidate_class ?? m.content_type}
                            </span>
                            {cat && (
                              <span className="rounded border border-(--color-gold-500)/45 bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] text-text-primary">
                                {cat}
                              </span>
                            )}
                            {typeof score === 'number' && (
                              <span className="rounded border border-(--color-status-pass)/40 bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] text-(--color-status-pass)">
                                {score}%
                              </span>
                            )}
                          </div>
                          {(tier || m.arxiv_id || m.prereserved_doi) && (
                            <div className="mt-1 truncate font-mono text-[11px] text-text-muted">
                              {m.arxiv_id ?? m.prereserved_doi ?? tier?.replace(/_/g, ' ')}
                            </div>
                          )}
                        </button>
                      );
                    })}
                  </BentoScrollRegion>
                )}
              </div>
            ))}
          </div>
        </BentoCard>

        {selected ? (
          <BentoCard span="hero-rail" className="vox-recessed-tablet border-(--color-gold-500)/50 p-0">
            <PublicationDetail
              manifest={selected}
              onClose={() => setSelectedId(null)}
              onManifestMutated={refresh}
              pushToast={pushToast}
            />
          </BentoCard>
        ) : (
          <div data-testid="publications-pipeline-overview" className="col-span-full min-w-0">
            <BentoGrid preset="triad">
              <div className="vox-recessed-tablet min-w-0 p-3.5">
                <div className="flex flex-wrap items-center justify-between gap-1.5">
                  <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                    Scientific Classification
                  </span>
                  <RomanWaxSealBadge label={`${manifests.length} manifests`} tone="gold" />
                </div>
                <p className="mt-2 font-serif text-xs leading-relaxed text-text-secondary">
                  Assign finding class, scholarly tier, arXiv.org subject categories, SPDX license, and ORCID attribution per manuscript.
                </p>
              </div>
              <div className="vox-recessed-tablet min-w-0 p-3.5">
                <div className="flex flex-wrap items-center justify-between gap-1.5">
                  <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                    Zero-Stub arXiv LaTeX &amp; BibTeX
                  </span>
                  <RomanWaxSealBadge label="main.tex + .bib" tone="gold" />
                </div>
                <p className="mt-2 font-serif text-xs leading-relaxed text-text-secondary">
                  Autofill IMRaD sections grounded in calibrated claim evidence (<code className="font-mono font-semibold text-text-primary">95% CI</code>) and packaged into validated <code className="font-mono font-semibold text-text-primary">.tar.gz</code> bundles.
                </p>
              </div>
              <div className="vox-recessed-tablet min-w-0 p-3.5">
                <div className="flex flex-wrap items-center justify-between gap-1.5">
                  <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                    7-Stage Lifecycle Control
                  </span>
                  <RomanWaxSealBadge label="12-Point Gate" tone="emerald" />
                </div>
                <p className="mt-2 font-serif text-xs leading-relaxed text-text-secondary">
                  Advance publications through pre-reserved DOI, ORCID binding, zero-stub approval, bundle validation, and arXiv ID publication.
                </p>
              </div>
            </BentoGrid>
          </div>
        )}
      </BentoGrid>
      </div>
    </section>
  );
}

/**
 * Drill-down studio for one scholarly publication: multi-dimensional classification editor,
 * 7-stage lifecycle progression controller, 12-point arXiv.org readiness checklist,
 * 6-axis worthiness radar, multi-artifact preview (IMRaD / LaTeX / BibTeX / Metadata / Checklist),
 * extracted claims ledger, and venue routing.
 */
function PublicationDetail({
  manifest,
  onClose,
  onManifestMutated,
  pushToast,
}: {
  manifest: PublicationManifest;
  onClose: () => void;
  onManifestMutated: () => Promise<void>;
  pushToast: SurfaceDecoratorProps['pushToast'];
}) {
  const [claims, setClaims] = useState<PublicationClaimsDto | null>(null);
  const [venue, setVenue] = useState<VenueRecommendationDto | null>(null);
  const [readiness, setReadiness] = useState<ArxivReadinessReportDto | null>(null);
  const [busy, setBusy] = useState(false);

  const [previewTab, setPreviewTab] = useState<PreviewTabId>('latex');
  const [previewText, setPreviewText] = useState<string | null>(null);
  const [previewMeta, setPreviewMeta] = useState<Record<PreviewTabId, string>>({
    imrad: '',
    latex: '',
    bibtex: '',
    metadata: '',
    checklist: '',
  });
  const [replayScore, setReplayScore] = useState<number | undefined>(undefined);

  // Editable Classification & Identity state
  const [candidateClass, setCandidateClass] = useState<string>(
    manifest.candidate_class ?? 'empirical_science',
  );
  const [publicationTier, setPublicationTier] = useState<string>(
    manifest.publication_tier ?? 'arxiv_preprint',
  );
  const [arxivPrimary, setArxivPrimary] = useState<string>(
    manifest.arxiv_primary_category ?? 'cs.AI',
  );
  const [arxivSecondaries, setArxivSecondaries] = useState<string>(
    (manifest.arxiv_secondary_categories ?? ['cs.SE', 'stat.ML']).join(', '),
  );
  const [licenseSpdx, setLicenseSpdx] = useState<string>(manifest.license_spdx ?? 'CC-BY-4.0');
  const [orcid, setOrcid] = useState<string>(manifest.orcid ?? '');
  const [prereservedDoi, setPrereservedDoi] = useState<string>(manifest.prereserved_doi ?? '');
  const [arxivId, setArxivId] = useState<string>(manifest.arxiv_id ?? '');

  useEffect(() => {
    setCandidateClass(manifest.candidate_class ?? 'empirical_science');
    setPublicationTier(manifest.publication_tier ?? 'arxiv_preprint');
    setArxivPrimary(manifest.arxiv_primary_category ?? 'cs.AI');
    setArxivSecondaries((manifest.arxiv_secondary_categories ?? ['cs.SE', 'stat.ML']).join(', '));
    setLicenseSpdx(manifest.license_spdx ?? 'CC-BY-4.0');
    setOrcid(manifest.orcid ?? '');
    setPrereservedDoi(manifest.prereserved_doi ?? '');
    setArxivId(manifest.arxiv_id ?? '');
  }, [manifest]);

  const applyReadinessSnapshot = useCallback((rep: ArxivReadinessReportDto) => {
    setReadiness(rep);
    setCandidateClass(rep.candidate_class);
    setPublicationTier(rep.publication_tier);
    setArxivPrimary(rep.arxiv_primary_category);
    setArxivSecondaries(rep.arxiv_secondary_categories.join(', '));
    setLicenseSpdx(rep.license_spdx);
    if (rep.orcid != null) setOrcid(rep.orcid);
    if (rep.prereserved_doi != null) setPrereservedDoi(rep.prereserved_doi);
    if (rep.arxiv_id != null) setArxivId(rep.arxiv_id);
    setPreviewMeta((prev) => ({
      ...prev,
      latex: rep.main_tex_preview,
      bibtex: rep.references_bib_preview,
      metadata: rep.arxiv_metadata_json,
      checklist: rep.upload_checklist_md,
    }));
  }, []);

  const load = useCallback(async () => {
    setBusy(true);
    setClaims(null);
    setVenue(null);
    try {
      const effectiveClass = manifest.candidate_class ?? manifest.content_type;
      const [claimsPayload, venuePayload, readinessPayload] = await Promise.all([
        listPublicationClaims(manifest.publication_id),
        recommendPublicationVenue(effectiveClass),
        getPublicationArxivReadiness(manifest.publication_id),
      ]);
      setClaims(claimsPayload);
      setVenue(venuePayload);
      applyReadinessSnapshot(readinessPayload);
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Publication detail',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusy(false);
    }
  }, [manifest.publication_id, manifest.candidate_class, manifest.content_type, applyReadinessSnapshot, pushToast]);

  useEffect(() => {
    load();
  }, [load]);

  const handleSaveClassification = async (autofillImrad: boolean) => {
    setBusy(true);
    try {
      const secs = arxivSecondaries
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);
      const report = await classifyPublication({
        publicationId: manifest.publication_id,
        candidateClass,
        publicationTier,
        arxivPrimaryCategory: arxivPrimary,
        arxivSecondaryCategories: secs,
        licenseSpdx,
        orcid: orcid.trim() || null,
        prereservedDoi: prereservedDoi.trim() || null,
        arxivId: arxivId.trim() || null,
        autofillImrad,
      });
      applyReadinessSnapshot(report);
      const newVenue = await recommendPublicationVenue(report.candidate_class);
      if (newVenue) setVenue(newVenue);
      if (autofillImrad) {
        setPreviewTab('latex');
        setPreviewText(report.main_tex_preview);
      }
      await onManifestMutated();
      toastOk(
        pushToast,
        autofillImrad ? 'IMRaD sections & classification saved' : 'Classification saved',
        `${report.arxiv_primary_category} · ${report.readiness_score}% arXiv ready`,
      );
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Save classification failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusy(false);
    }
  };

  const handleAdvanceStage = async (targetStage: string) => {
    setBusy(true);
    try {
      const report = await advancePublicationStage({
        publicationId: manifest.publication_id,
        targetStage,
        orcid: orcid.trim() || null,
        prereservedDoi: prereservedDoi.trim() || null,
        arxivId: arxivId.trim() || null,
        autofillImrad: true,
      });
      applyReadinessSnapshot(report);
      await onManifestMutated();
      toastOk(
        pushToast,
        `Stage advanced to ${report.current_stage.replace(/_/g, ' ')}`,
        `${report.publication_id} · ${report.readiness_score}% readiness`,
      );
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Stage transition failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusy(false);
    }
  };

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
    completeness: readiness?.readiness_score ?? manifest.readiness_score ?? undefined,
  });

  const currentStage = readiness?.current_stage ?? manifest.state;
  const stageIdx = PUBLICATION_STAGES.indexOf(
    currentStage as (typeof PUBLICATION_STAGES)[number],
  );
  const resolvedStageIdx = stageIdx === -1 ? 0 : stageIdx;

  return (
    <div className="min-w-0 space-y-4 p-3.5 @2xl/surface:p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          {manifest.title && (
            <div className="truncate font-serif text-sm font-semibold text-text-primary">
              {manifest.title}
            </div>
          )}
          <div className="truncate font-mono text-xs font-semibold text-brass">
            {manifest.publication_id}
          </div>
          <div className="font-mono text-[11px] text-text-secondary">
            {candidateClass} · {publicationTier} · {arxivPrimary} · {currentStage}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            data-slot="button"
            data-testid="pub-preview-imrad-btn"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                const res = await draftManuscript(manifest.publication_id);
                setPreviewMeta((prev) => ({ ...prev, imrad: res.markdown }));
                setPreviewTab('imrad');
                setPreviewText(res.markdown);
              } catch (err) {
                pushToast({
                  tone: 'warn',
                  title: 'IMRaD preview failed',
                  body: sanitizeErrorForToast(err),
                  cause: 'backend-error',
                });
              } finally {
                setBusy(false);
              }
            }}
            className="inline-flex h-8 items-center rounded-lg border border-brass/45 bg-bg-elevated px-3 text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
          >
            Preview IMRaD
          </button>
          <button
            type="button"
            data-slot="button"
            data-testid="pub-preview-latex-btn"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                const res = await renderLatex(manifest.publication_id);
                setPreviewMeta((prev) => ({
                  ...prev,
                  latex: res.latex,
                  bibtex: res.references_bib ?? prev.bibtex,
                }));
                setPreviewTab('latex');
                setPreviewText(res.latex);
              } catch (err) {
                pushToast({
                  tone: 'warn',
                  title: 'LaTeX preview failed',
                  body: sanitizeErrorForToast(err),
                  cause: 'backend-error',
                });
              } finally {
                setBusy(false);
              }
            }}
            className="inline-flex h-8 items-center rounded-lg border border-brass/45 bg-bg-elevated px-3 text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
          >
            Preview LaTeX
          </button>
          <button
            type="button"
            data-slot="button"
            data-testid="pub-preview-bibtex-btn"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                const bundle = await renderArxivBundle(manifest.publication_id);
                setPreviewMeta({
                  imrad: previewMeta.imrad,
                  latex: bundle.main_tex,
                  bibtex: bundle.references_bib,
                  metadata: bundle.arxiv_metadata_json,
                  checklist: bundle.upload_checklist_md,
                });
                setPreviewTab('bibtex');
                setPreviewText(bundle.references_bib);
              } catch (err) {
                pushToast({
                  tone: 'warn',
                  title: 'BibTeX preview failed',
                  body: sanitizeErrorForToast(err),
                  cause: 'backend-error',
                });
              } finally {
                setBusy(false);
              }
            }}
            className="inline-flex h-8 items-center rounded-lg border border-brass/45 bg-bg-elevated px-3 text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
          >
            Preview BibTeX &amp; Bundle
          </button>
          <button
            type="button"
            data-slot="button"
            onClick={load}
            disabled={busy}
            className="inline-flex h-8 items-center rounded-lg border border-border-strong bg-bg-elevated px-3 text-xs font-medium text-text-secondary transition-colors hover:border-brass/45 hover:text-text-primary disabled:opacity-40"
          >
            {busy ? 'Loading…' : 'Reload'}
          </button>
          <button
            type="button"
            data-slot="button"
            onClick={onClose}
            className="inline-flex h-8 items-center rounded-lg border border-border-strong bg-bg-elevated px-3 text-xs font-medium text-text-secondary transition-colors hover:border-brass/45 hover:text-text-primary"
          >
            Close
          </button>
        </div>
      </div>

      {/* Classification & arXiv.org Taxonomy Control Tablet */}
      <div
        data-testid="publication-classification-tablet"
        className="vox-marble-tablet space-y-3 rounded-xl border border-(--color-gold-500)/45 bg-bg-surface p-3"
      >
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
            Scientific Classification &amp; arXiv Taxonomy
          </span>
          <div className="flex flex-wrap items-center gap-2">
            {readiness && (
              <RomanWaxSealBadge
                label={
                  readiness.arxiv_ready
                    ? `ARXIV READY ${readiness.readiness_score}%`
                    : `${readiness.readiness_score}% READINESS`
                }
                tone={readiness.arxiv_ready ? 'emerald' : 'gold'}
              />
            )}
          </div>
        </div>

        <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
          <div className="space-y-1">
            <label className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary">
              Finding Class
            </label>
            <VoxSelect
              aria-label="Finding class"
              data-testid="detail-finding-class"
              value={candidateClass}
              onValueChange={setCandidateClass}
              options={FINDING_CLASS_OPTIONS.map((c) => ({ value: c.id, label: c.label }))}
            />
          </div>
          <div className="space-y-1">
            <label className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary">
              Publication Tier
            </label>
            <VoxSelect
              aria-label="Publication tier"
              data-testid="detail-publication-tier"
              value={publicationTier}
              onValueChange={setPublicationTier}
              options={PUBLICATION_TIER_OPTIONS.map((t) => ({ value: t.id, label: t.label }))}
            />
          </div>
          <div className="space-y-1">
            <label className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary">
              arXiv Primary
            </label>
            <VoxSelect
              aria-label="Primary arXiv category"
              data-testid="detail-arxiv-primary"
              value={arxivPrimary}
              onValueChange={setArxivPrimary}
              options={ARXIV_SUBJECT_OPTIONS.map((a) => ({ value: a.code, label: a.name }))}
            />
          </div>
          <div className="space-y-1">
            <label className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary">
              SPDX License
            </label>
            <VoxSelect
              aria-label="SPDX license"
              data-testid="detail-license-spdx"
              value={licenseSpdx}
              onValueChange={setLicenseSpdx}
              options={SPDX_LICENSE_OPTIONS.map((lic) => ({ value: lic, label: lic }))}
            />
          </div>
        </div>

        <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
          <div className="space-y-1">
            <label
              htmlFor="pub-cross-list-input"
              className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary"
            >
              Cross-List Categories
            </label>
            <input
              id="pub-cross-list-input"
              type="text"
              value={arxivSecondaries}
              onChange={(e) => setArxivSecondaries(e.target.value)}
              placeholder="cs.SE, stat.ML"
              className="h-8 w-full rounded-md border border-input bg-bg-elevated px-2.5 font-mono text-xs text-text-primary focus:border-brass focus:outline-none"
            />
          </div>
          <div className="space-y-1">
            <label
              htmlFor="pub-orcid-input"
              className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary"
            >
              Author ORCID
            </label>
            <input
              id="pub-orcid-input"
              type="text"
              value={orcid}
              onChange={(e) => setOrcid(e.target.value)}
              placeholder="0000-0002-1825-0097"
              className="h-8 w-full rounded-md border border-input bg-bg-elevated px-2.5 font-mono text-xs text-text-primary focus:border-brass focus:outline-none"
            />
          </div>
          <div className="space-y-1">
            <label
              htmlFor="pub-doi-input"
              className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary"
            >
              Pre-Reserved DOI
            </label>
            <input
              id="pub-doi-input"
              type="text"
              value={prereservedDoi}
              onChange={(e) => setPrereservedDoi(e.target.value)}
              placeholder="10.48550/arXiv.2611.xxxxx"
              className="h-8 w-full rounded-md border border-input bg-bg-elevated px-2.5 font-mono text-xs text-text-primary focus:border-brass focus:outline-none"
            />
          </div>
          <div className="space-y-1">
            <label
              htmlFor="pub-arxiv-id-input"
              className="block font-mono text-[11px] uppercase tracking-wider text-text-secondary"
            >
              arXiv Identifier
            </label>
            <input
              id="pub-arxiv-id-input"
              type="text"
              value={arxivId}
              onChange={(e) => setArxivId(e.target.value)}
              placeholder="arXiv:2611.04820v1"
              className="h-8 w-full rounded-md border border-input bg-bg-elevated px-2.5 font-mono text-xs text-text-primary focus:border-brass focus:outline-none"
            />
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            data-slot="button"
            data-testid="pub-save-classification-btn"
            disabled={busy}
            onClick={() => handleSaveClassification(false)}
            className="inline-flex h-8 items-center rounded-lg border border-brass/50 bg-bg-elevated px-3 text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
          >
            Save Classification
          </button>
          <button
            type="button"
            data-slot="button"
            data-testid="pub-autofill-imrad-btn"
            disabled={busy}
            onClick={() => handleSaveClassification(true)}
            className="inline-flex h-8 items-center rounded-lg border border-(--color-status-pass)/50 bg-bg-elevated px-3 text-xs font-medium text-(--color-status-pass) transition-colors hover:border-(--color-status-pass) disabled:opacity-40"
          >
            Autofill Scientific IMRaD (0-Stub LaTeX)
          </button>
        </div>
      </div>

      {/* 7-Stage Lifecycle Gate & Progression Controller */}
      <div
        data-testid="publication-stage-controller"
        className="vox-marble-tablet space-y-3 rounded-xl border border-(--color-gold-500)/45 bg-bg-surface p-3"
      >
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
            7-Stage Lifecycle Controller
          </span>
          <span className="font-mono text-[11px] text-text-secondary">
            Current: <strong className="text-text-primary">{currentStage}</strong>
          </span>
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {PUBLICATION_STAGES.map((s, i) => (
            <React.Fragment key={s}>
              <span
                className={`rounded px-2 py-0.5 font-mono text-[11px] uppercase tracking-wider ${
                  i <= resolvedStageIdx
                    ? 'border border-(--color-gold-500)/60 bg-bg-elevated font-semibold text-text-primary'
                    : 'border border-border-strong bg-bg-surface text-text-muted'
                }`}
              >
                {s.replace(/_/g, ' ')}
              </span>
              {i < PUBLICATION_STAGES.length - 1 && (
                <span className="text-text-muted" aria-hidden="true">
                  /
                </span>
              )}
            </React.Fragment>
          ))}
        </div>

        <div role="group" aria-label="Lifecycle stage actions" className="flex flex-wrap gap-2">
          {STAGE_ACTIONS.map((act) => {
            const done =
              PUBLICATION_STAGES.indexOf(act.stage as (typeof PUBLICATION_STAGES)[number]) <=
                resolvedStageIdx && currentStage !== 'failed';
            return (
              <button
                key={act.stage}
                type="button"
                data-slot="button"
                data-testid={`stage-advance-${act.stage}-btn`}
                disabled={busy}
                onClick={() => handleAdvanceStage(act.stage)}
                className={`inline-flex h-8 items-center rounded-lg border px-3 text-xs font-medium transition-colors disabled:opacity-40 ${
                  done
                    ? 'border-(--color-status-pass)/50 bg-bg-elevated text-(--color-status-pass)'
                    : 'border-brass/45 bg-bg-elevated text-text-primary hover:border-brass'
                }`}
              >
                {act.actionLabel}
              </button>
            );
          })}
        </div>

        {readiness && (
          <details className="rounded-lg border border-border-strong bg-bg-elevated p-2.5">
            <summary className="cursor-pointer select-none font-mono text-xs font-semibold text-text-primary">
              12-Point arXiv.org Readiness Gate ({readiness.checks.filter((c) => c.passed).length}/
              {readiness.checks.length} passed · {readiness.todo_stub_count} TODO stubs ·{' '}
              {readiness.bundle_tar_gz_bytes} B bundle)
            </summary>
            <ul className="mt-2.5 grid grid-cols-1 gap-1.5 sm:grid-cols-2">
              {readiness.checks.map((chk) => (
                <li
                  key={chk.id}
                  className="flex items-start justify-between gap-2 rounded border border-border-strong bg-bg-surface px-2.5 py-1.5 text-[11px]"
                >
                  <div className="min-w-0">
                    <div className="font-semibold text-text-primary">{chk.label}</div>
                    <div className="truncate font-mono text-text-secondary">{chk.detail}</div>
                  </div>
                  <span
                    className={`shrink-0 font-mono font-semibold uppercase ${
                      chk.passed
                        ? 'text-(--color-status-pass)'
                        : chk.blocking
                          ? 'text-(--color-status-error)'
                          : 'text-(--color-status-warn)'
                    }`}
                  >
                    {chk.passed ? 'PASS' : chk.blocking ? 'BLOCK' : 'PENDING'}
                  </span>
                </li>
              ))}
            </ul>
          </details>
        )}
      </div>

      {previewText && (
        <div
          data-testid="publication-artifact-preview"
          className="rounded-xl border border-(--color-gold-500)/45 bg-bg-surface p-3"
        >
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label="Artifact preview tabs">
              {(
                [
                  ['imrad', 'IMRaD Markdown'],
                  ['latex', 'main.tex'],
                  ['bibtex', 'references.bib'],
                  ['metadata', 'arxiv-metadata.json'],
                  ['checklist', 'UPLOAD-CHECKLIST.md'],
                ] as const
              ).map(([tabId, label]) => (
                <button
                  key={tabId}
                  type="button"
                  data-slot="button"
                  onClick={() => {
                    setPreviewTab(tabId);
                    if (previewMeta[tabId]) setPreviewText(previewMeta[tabId]);
                  }}
                  className={`inline-flex h-8 items-center rounded-lg border px-2.5 font-mono text-[11px] transition-colors ${
                    previewTab === tabId
                      ? 'border-(--color-gold-500)/70 bg-bg-elevated font-semibold text-text-primary'
                      : 'border-border-strong bg-bg-surface text-text-secondary hover:text-text-primary'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
            <button
              type="button"
              data-slot="button"
              onClick={() => {
                setPreviewText(null);
              }}
              className="inline-flex h-8 items-center rounded-lg border border-border-strong bg-bg-elevated px-2.5 text-xs text-text-secondary hover:text-text-primary"
            >
              Close Preview
            </button>
          </div>
          <BentoScrollRegion ariaLabel="Manuscript preview content" className="max-h-60">
            <pre className="whitespace-pre-wrap font-mono text-[11px] text-text-primary">
              {previewText}
            </pre>
          </BentoScrollRegion>
        </div>
      )}

      <div className="space-y-3">
        <WorthinessRadarPanel axes={worthinessAxes} compact />
        <DirectPublicationDrawer
          publicationId={manifest.publication_id}
          pushToast={pushToast}
          compact
          onReplayVerified={(score) => setReplayScore(score)}
          onStageOrClassificationUpdated={async (rep) => {
            applyReadinessSnapshot(rep);
            await onManifestMutated();
          }}
          onFlywheelAdvanced={() => {
            void load();
            void onManifestMutated();
          }}
        />
      </div>

      <div className="grid grid-cols-1 gap-3 @xl/surface:grid-cols-2">
        {/* Claims */}
        <div className="min-w-0">
          <div className="mb-2 font-display text-xs uppercase tracking-wider text-brass">
            Claims {claims ? `(${claims.claim_count})` : ''}
          </div>
          {!claims || claims.claims.length === 0 ? (
            <div className="rounded-lg border border-(--color-gold-500)/35 bg-bg-surface p-3 text-xs text-text-secondary">
              {busy ? 'Loading…' : 'No extracted claims. Run claim extraction first.'}
            </div>
          ) : (
            <BentoScrollRegion ariaLabel="Extracted manuscript claims" className="max-h-52">
              <ul className="space-y-1.5">
                {claims.claims.map((c, i) => (
                  <li
                    key={c.id ?? i}
                    className="rounded-lg border border-(--color-gold-500)/35 bg-bg-surface p-2.5"
                  >
                    <div className="text-xs leading-relaxed text-text-primary">
                      {c.text ?? '(no text)'}
                    </div>
                    {c.verdict_label && (
                      <div className="mt-1 font-mono text-[11px] text-brass">
                        {c.verdict_label}
                        {typeof c.confidence === 'number' ? ` · ${c.confidence.toFixed(2)}` : ''}
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
          <div className="mb-2 font-display text-xs uppercase tracking-wider text-brass">
            Venue Routing &amp; Policy
          </div>
          {!venue ? (
            <div className="rounded-lg border border-(--color-gold-500)/35 bg-bg-surface p-3 text-xs text-text-secondary">
              {busy ? 'Loading…' : `No routing for class "${candidateClass}".`}
            </div>
          ) : (
            <div className="space-y-2 rounded-lg border border-(--color-gold-500)/35 bg-bg-surface p-3">
              <div className="flex flex-wrap gap-1.5">
                {venue.recommended_venues.length === 0 ? (
                  <span className="font-mono text-xs text-text-secondary">
                    No recommended venues.
                  </span>
                ) : (
                  venue.recommended_venues.map((v) => (
                    <span
                      key={v}
                      className="rounded border border-(--color-gold-500)/45 bg-bg-elevated px-2 py-0.5 font-mono text-[11px] text-text-primary"
                    >
                      {v}
                    </span>
                  ))
                )}
              </div>
              <div className="grid grid-cols-2 gap-x-3 gap-y-1 font-mono text-[11px] text-text-secondary">
                <span title="Open peer-review and contradiction quarantine window before final archival lock">
                  Reply window
                </span>
                <span className="text-text-primary">{venue.reply_window_days}d</span>
                <span title="Target share of null or negative empirical findings published to curb publication bias">
                  Neg-result quota
                </span>
                <span className="text-text-primary">{venue.negative_result_quota}</span>
                <span title="Whether calibrated LLM critic sign-off is permitted alongside human review">
                  Critic allowed
                </span>
                <span
                  className={
                    venue.critic_allowed
                      ? 'text-(--color-status-pass)'
                      : 'text-(--color-status-warn)'
                  }
                >
                  {venue.critic_allowed ? 'yes' : 'no'}
                </span>
                <span title="Enforces multi-provider benchmark reproducibility gates for Capability and Reliability Atlas tracks">
                  Atlas gate
                </span>
                <span className="text-text-primary">
                  {venue.atlas_gate_applies ? 'applies' : 'inactive'}
                </span>
              </div>
              <p className="font-serif text-[11px] text-text-muted">
                Reply window enforces pre-archival peer objection time; Atlas gate requires multi-provider replayability benchmarks.
              </p>
              {!venue.critic_allowed && (
                <div className="rounded border border-(--color-status-warn)/35 bg-bg-elevated p-2 text-xs text-(--color-status-warn)">
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

function toastOk(
  pushToast: SurfaceDecoratorProps['pushToast'],
  title: string,
  body: string,
) {
  pushToast({ tone: 'ok', title, body, cause: 'backend-ok' });
}
