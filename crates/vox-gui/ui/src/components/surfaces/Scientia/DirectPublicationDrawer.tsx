import React, { useState } from 'react';
import {
  ARXIV_SUBJECT_OPTIONS,
  PUBLICATION_TIER_OPTIONS,
  advanceFlywheel,
  advancePublicationStage,
  classifyPublication,
  draftManuscript,
  executeReplay,
  renderArxivBundle,
  renderFindingPage,
  renderLatex,
  type ArxivReadinessReportDto,
  type FlywheelAdvanceDto,
} from '../../../api/scientia';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';

export interface DirectPublicationDrawerProps {
  publicationId: string;
  pushToast?: SurfaceDecoratorProps['pushToast'];
  onReplayVerified?: (score: number) => void;
  onFlywheelAdvanced?: (result: FlywheelAdvanceDto) => void;
  onStageOrClassificationUpdated?: (report: ArxivReadinessReportDto) => void;
  compact?: boolean;
}

const FLYWHEEL_STEPS = [
  {
    id: 'extract_claims',
    label: 'Extract Claims',
    targetStage: 'extract_claims',
    matchStages: ['extract_claims'],
  },
  {
    id: 'novelty_check',
    label: 'Novelty Check',
    targetStage: 'novelty_check',
    matchStages: ['novelty_check'],
  },
  {
    id: 'critic_approval',
    label: 'Critic & Digest Approval',
    targetStage: 'scaffold_imrad',
    matchStages: ['critic_approval', 'scaffold_imrad'],
  },
  {
    id: 'imrad_latex_arxiv',
    label: 'IMRaD / LaTeX / arXiv',
    targetStage: 'arxiv_bundle',
    matchStages: ['scaffold_imrad', 'render_latex', 'arxiv_bundle', 'ro_crate'],
  },
  {
    id: 'finding_page',
    label: 'Finding Page',
    targetStage: 'finding_page',
    matchStages: ['finding_page'],
  },
  {
    id: 'replay_verify',
    label: 'Replay Verify',
    targetStage: 'replay_verify',
    matchStages: ['replay_verify'],
  },
] as const;

/**
 * Direct Publication Flywheel & arXiv.org Control Drawer (STUD-02 / PUB-STUDIO-UX-04).
 * Interactive 6-stage flywheel stepper paired with arXiv classification, IMRaD autofill,
 * BibTeX/tar.gz synthesis, and 7-stage lifecycle advancement.
 */
export function DirectPublicationDrawer({
  publicationId,
  pushToast,
  onReplayVerified,
  onFlywheelAdvanced,
  onStageOrClassificationUpdated,
  compact = false,
}: DirectPublicationDrawerProps) {
  const [busy, setBusy] = useState(false);
  const [stagesDone, setStagesDone] = useState<string[]>([]);
  const [previewTitle, setPreviewTitle] = useState<string | null>(null);
  const [previewContent, setPreviewContent] = useState<string | null>(null);
  const [roCrateDigest, setRoCrateDigest] = useState<string | null>(null);
  const [tier, setTier] = useState<string>('arxiv_preprint');
  const [arxivCat, setArxivCat] = useState<string>('cs.AI');
  const [readinessScore, setReadinessScore] = useState<number | null>(null);

  const toast = pushToast ?? (() => {});
  const trimmedId = publicationId.trim();

  const runAdvance = async (targetStage?: string) => {
    if (!trimmedId) return;
    setBusy(true);
    try {
      const res = await advanceFlywheel(trimmedId, targetStage);
      setStagesDone(res.stages_done);
      if (res.artifacts?.ro_crate_digest) {
        setRoCrateDigest(res.artifacts.ro_crate_digest);
      }
      if (typeof res.artifacts?.replayability === 'number') {
        onReplayVerified?.(res.artifacts.replayability);
      } else if (typeof res.worthiness_inputs?.replayability_score === 'number') {
        onReplayVerified?.(res.worthiness_inputs.replayability_score);
      }
      if (res.artifacts?.markdown && targetStage === 'scaffold_imrad') {
        setPreviewTitle(`IMRaD Manuscript (${trimmedId})`);
        setPreviewContent(res.artifacts.markdown);
      } else if (res.artifacts?.latex && targetStage === 'arxiv_bundle') {
        setPreviewTitle(`LaTeX / arXiv Bundle (${trimmedId})`);
        setPreviewContent(res.artifacts.latex);
      } else if (res.artifacts?.html && targetStage === 'finding_page') {
        setPreviewTitle('Finding Page HTML + Highwire Meta');
        setPreviewContent(res.artifacts.html);
      }
      onFlywheelAdvanced?.(res);
      toast({
        tone: 'ok',
        title: 'Flywheel advanced',
        body: `${res.publication_id} · stage ${res.stage_completed}`,
        cause: 'backend-ok',
      });
    } catch (err) {
      toast({
        tone: 'warn',
        title: 'Flywheel advance failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      data-testid="direct-publication-drawer"
      data-slot="direct-publication-drawer"
      className="vox-marble-tablet space-y-3 rounded-xl border border-(--color-gold-500)/45 bg-bg-elevated p-3.5"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <Icon.flow className="size-4 shrink-0 text-brass" aria-hidden="true" />
          <span className="font-display text-[11px] uppercase tracking-[0.16em] text-text-primary">
            Direct Publication Flywheel
          </span>
          {readinessScore !== null && (
            <span className="rounded border border-(--color-status-pass)/45 bg-bg-surface px-2 py-0.5 font-mono text-[11px] font-semibold text-(--color-status-pass)">
              arXiv Ready {readinessScore}%
            </span>
          )}
          {roCrateDigest && (
            <span className="rounded border border-(--color-status-pass)/40 bg-bg-surface px-2 py-0.5 font-mono text-[11px] text-(--color-status-pass)">
              RO-Crate {roCrateDigest.slice(0, 18)}
            </span>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            data-slot="button"
            data-testid="flywheel-advance-btn"
            disabled={!trimmedId || busy}
            onClick={() => runAdvance('replay_verify')}
            className="inline-flex h-8 items-center gap-1.5 rounded-lg border border-(--color-gold-500)/60 bg-bg-surface px-3 font-display text-[11px] font-semibold uppercase tracking-wider text-text-primary transition-colors hover:border-(--color-gold-400) disabled:opacity-40"
          >
            <Icon.play className="size-3 shrink-0 text-brass" aria-hidden="true" />
            <span>{busy ? 'Advancing…' : 'Advance Full Flywheel'}</span>
          </button>
          {!compact && (
            <>
              <button
                type="button"
                data-slot="button"
                data-testid="studio-draft-imrad-btn"
                disabled={!trimmedId || busy}
                onClick={async () => {
                  setBusy(true);
                  try {
                    const res = await draftManuscript(trimmedId);
                    setPreviewTitle(`IMRaD Manuscript (${res.title ?? trimmedId})`);
                    setPreviewContent(res.markdown);
                    setStagesDone((prev) =>
                      prev.includes('scaffold_imrad') ? prev : [...prev, 'scaffold_imrad'],
                    );
                  } catch (err) {
                    toast({
                      tone: 'warn',
                      title: 'Draft IMRaD failed',
                      body: sanitizeErrorForToast(err),
                      cause: 'backend-error',
                    });
                  } finally {
                    setBusy(false);
                  }
                }}
                className="inline-flex h-8 items-center rounded-lg border border-brass/45 bg-bg-surface px-3 font-sans text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
              >
                Draft IMRaD
              </button>
              <button
                type="button"
                data-slot="button"
                data-testid="studio-render-latex-btn"
                disabled={!trimmedId || busy}
                onClick={async () => {
                  setBusy(true);
                  try {
                    const res = await renderLatex(trimmedId);
                    setPreviewTitle(`LaTeX Source (${res.title ?? trimmedId})`);
                    setPreviewContent(res.latex);
                    setStagesDone((prev) =>
                      prev.includes('render_latex') ? prev : [...prev, 'render_latex'],
                    );
                  } catch (err) {
                    toast({
                      tone: 'warn',
                      title: 'Render LaTeX failed',
                      body: sanitizeErrorForToast(err),
                      cause: 'backend-error',
                    });
                  } finally {
                    setBusy(false);
                  }
                }}
                className="inline-flex h-8 items-center rounded-lg border border-brass/45 bg-bg-surface px-3 font-sans text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
              >
                Render LaTeX
              </button>
              <button
                type="button"
                data-slot="button"
                data-testid="studio-arxiv-bundle-btn"
                disabled={!trimmedId || busy}
                onClick={async () => {
                  setBusy(true);
                  try {
                    const bundle = await renderArxivBundle(trimmedId);
                    setPreviewTitle(`BibTeX & arXiv Bundle (${bundle.bytes_len} B)`);
                    setPreviewContent(
                      `% === references.bib ===\n${bundle.references_bib}\n% === arxiv-metadata.json ===\n${bundle.arxiv_metadata_json}`,
                    );
                    setStagesDone((prev) =>
                      prev.includes('arxiv_bundle') ? prev : [...prev, 'arxiv_bundle'],
                    );
                  } catch (err) {
                    toast({
                      tone: 'warn',
                      title: 'arXiv bundle failed',
                      body: sanitizeErrorForToast(err),
                      cause: 'backend-error',
                    });
                  } finally {
                    setBusy(false);
                  }
                }}
                className="inline-flex h-8 items-center rounded-lg border border-brass/45 bg-bg-surface px-3 font-sans text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
              >
                BibTeX &amp; Bundle
              </button>
            </>
          )}
          <button
            type="button"
            data-slot="button"
            data-testid="studio-finding-page-btn"
            disabled={!trimmedId || busy}
            onClick={async () => {
              setBusy(true);
              try {
                const res = await renderFindingPage(trimmedId);
                setPreviewTitle('Finding Page HTML + Highwire Meta');
                setPreviewContent(res.html);
                setStagesDone((prev) =>
                  prev.includes('finding_page') ? prev : [...prev, 'finding_page'],
                );
              } catch (err) {
                toast({
                  tone: 'warn',
                  title: 'Finding page failed',
                  body: sanitizeErrorForToast(err),
                  cause: 'backend-error',
                });
              } finally {
                setBusy(false);
              }
            }}
            className="inline-flex h-8 items-center rounded-lg border border-border-strong bg-bg-surface px-3 font-sans text-xs font-medium text-text-secondary transition-colors hover:border-brass/45 hover:text-text-primary disabled:opacity-40"
          >
            Finding Page HTML
          </button>
          <button
            type="button"
            data-slot="button"
            data-testid="studio-execute-replay-btn"
            disabled={!trimmedId || busy}
            onClick={async () => {
              setBusy(true);
              try {
                const res = await executeReplay(trimmedId, 0.94, 120);
                onReplayVerified?.(res.artifact_replayability);
                setStagesDone((prev) =>
                  prev.includes('replay_verify') ? prev : [...prev, 'replay_verify'],
                );
                toast({
                  tone: 'ok',
                  title: 'Replay verified',
                  body: `Replayability score ${(res.artifact_replayability * 100).toFixed(0)}% recorded`,
                  cause: 'backend-ok',
                });
              } catch (err) {
                toast({
                  tone: 'warn',
                  title: 'Replay failed',
                  body: sanitizeErrorForToast(err),
                  cause: 'backend-error',
                });
              } finally {
                setBusy(false);
              }
            }}
            className="inline-flex h-8 items-center rounded-lg border border-(--color-status-pass)/45 bg-bg-surface px-3 font-sans text-xs font-medium text-(--color-status-pass) transition-colors hover:border-(--color-status-pass) disabled:opacity-40"
          >
            Execute Replay
          </button>
        </div>
      </div>

      {/* Quick Classification & arXiv Stage Controls (hidden in compact Studio view where Classification Tablet & Stage Controller sit directly above) */}
      {!compact && (
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border-strong bg-bg-surface p-2.5">
        <div className="min-w-[160px] flex-1">
          <VoxSelect
            aria-label="Publication tier"
            data-testid="drawer-tier-select"
            value={tier}
            onValueChange={setTier}
            options={PUBLICATION_TIER_OPTIONS.map((t) => ({ value: t.id, label: t.label }))}
          />
        </div>
        <div className="min-w-[170px] flex-1">
          <VoxSelect
            aria-label="arXiv category"
            data-testid="drawer-arxiv-select"
            value={arxivCat}
            onValueChange={setArxivCat}
            options={ARXIV_SUBJECT_OPTIONS.map((c) => ({ value: c.code, label: c.name }))}
          />
        </div>
        <button
          type="button"
          data-slot="button"
          data-testid="drawer-autofill-imrad-btn"
          disabled={!trimmedId || busy}
          onClick={async () => {
            setBusy(true);
            try {
              const report = await classifyPublication({
                publicationId: trimmedId,
                publicationTier: tier,
                arxivPrimaryCategory: arxivCat,
                autofillImrad: true,
              });
              setReadinessScore(report.readiness_score);
              setPreviewTitle(`Zero-Stub Scientific main.tex (${report.readiness_score}%)`);
              setPreviewContent(report.main_tex_preview);
              onStageOrClassificationUpdated?.(report);
              toast({
                tone: 'ok',
                title: 'IMRaD & classification saved',
                body: `${report.arxiv_primary_category} · ${report.readiness_score}% ready · ${report.todo_stub_count} TODO stubs`,
                cause: 'backend-ok',
              });
            } catch (err) {
              toast({
                tone: 'warn',
                title: 'Classification update failed',
                body: sanitizeErrorForToast(err),
                cause: 'backend-error',
              });
            } finally {
              setBusy(false);
            }
          }}
          className="inline-flex h-8 items-center rounded-lg border border-brass/50 bg-bg-elevated px-3 font-sans text-xs font-medium text-text-primary transition-colors hover:border-brass disabled:opacity-40"
        >
          Autofill Scientific IMRaD
        </button>
        <button
          type="button"
          data-slot="button"
          data-testid="drawer-approve-stage-btn"
          disabled={!trimmedId || busy}
          onClick={async () => {
            setBusy(true);
            try {
              const report = await advancePublicationStage({
                publicationId: trimmedId,
                targetStage: 'approved',
                autofillImrad: true,
              });
              setReadinessScore(report.readiness_score);
              onStageOrClassificationUpdated?.(report);
              toast({
                tone: 'ok',
                title: 'Manifest approved for arXiv',
                body: `${report.publication_id} · stage ${report.current_stage} · ${report.readiness_score}%`,
                cause: 'backend-ok',
              });
            } catch (err) {
              toast({
                tone: 'warn',
                title: 'Stage advance failed',
                body: sanitizeErrorForToast(err),
                cause: 'backend-error',
              });
            } finally {
              setBusy(false);
            }
          }}
          className="inline-flex h-8 items-center rounded-lg border border-(--color-status-pass)/50 bg-bg-elevated px-3 font-sans text-xs font-medium text-(--color-status-pass) transition-colors hover:border-(--color-status-pass) disabled:opacity-40"
        >
          Approve for arXiv
        </button>
      </div>
      )}

      {/* 6-Stage Flywheel Stepper */}
      <div
        role="group"
        aria-label="Direct publication flywheel stages"
        className={
          compact
            ? 'grid grid-cols-2 gap-1.5 sm:grid-cols-3'
            : 'grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6'
        }
      >
        {FLYWHEEL_STEPS.map((step, index) => {
          const done = step.matchStages.some((s) => stagesDone.includes(s));
          return (
            <button
              key={step.id}
              type="button"
              data-slot="button"
              data-testid={`flywheel-step-${step.id}`}
              disabled={!trimmedId || busy}
              onClick={() => runAdvance(step.targetStage)}
              className={`flex h-8 items-center justify-between gap-1.5 rounded-lg border px-2.5 text-left font-mono text-[11px] transition-colors ${
                done
                  ? 'border-(--color-status-pass)/50 bg-bg-surface text-(--color-status-pass)'
                  : 'border-border-strong bg-bg-surface text-text-secondary hover:border-brass/50 hover:text-text-primary'
              } disabled:opacity-40`}
            >
              <span className="truncate">
                {index + 1}. {step.label}
              </span>
              {done ? (
                <Icon.check className="size-3.5 shrink-0 text-(--color-status-pass)" aria-hidden="true" />
              ) : (
                <Icon.chevR className="size-3.5 shrink-0 text-text-muted" aria-hidden="true" />
              )}
            </button>
          );
        })}
      </div>

      {previewContent && (
        <div
          data-testid="studio-artifact-preview"
          className="rounded-lg border border-(--color-gold-500)/45 bg-bg-surface p-3"
        >
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="font-mono text-xs font-semibold text-brass">{previewTitle}</span>
            <button
              type="button"
              data-slot="button"
              onClick={() => {
                setPreviewContent(null);
                setPreviewTitle(null);
              }}
              className="h-8 rounded-lg border border-border-strong bg-bg-elevated px-2.5 font-sans text-xs text-text-secondary hover:text-text-primary"
            >
              Close Preview
            </button>
          </div>
          <pre className="max-h-56 overflow-auto whitespace-pre-wrap font-mono text-[11px] text-text-primary">
            {previewContent}
          </pre>
        </div>
      )}
    </div>
  );
}
