import React, { useState } from 'react';
import {
  advanceFlywheel,
  draftManuscript,
  executeReplay,
  renderFindingPage,
  renderLatex,
  type FlywheelAdvanceDto,
} from '../../../api/scientia';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { Icon } from '../../ui/Icons';

export interface DirectPublicationDrawerProps {
  publicationId: string;
  pushToast?: SurfaceDecoratorProps['pushToast'];
  onReplayVerified?: (score: number) => void;
  onFlywheelAdvanced?: (result: FlywheelAdvanceDto) => void;
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
 * Direct Publication Flywheel Drawer (STUD-02).
 * Interactive 6-stage progression stepper from claim extraction to deterministic replay
 * verification, backed by `scientia_publication_flywheel_advance` and direct artifact renders.
 */
export function DirectPublicationDrawer({
  publicationId,
  pushToast,
  onReplayVerified,
  onFlywheelAdvanced,
  compact = false,
}: DirectPublicationDrawerProps) {
  const [busy, setBusy] = useState(false);
  const [stagesDone, setStagesDone] = useState<string[]>([]);
  const [previewTitle, setPreviewTitle] = useState<string | null>(null);
  const [previewContent, setPreviewContent] = useState<string | null>(null);
  const [roCrateDigest, setRoCrateDigest] = useState<string | null>(null);

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
      className="rounded-xl border border-border-subtle bg-overlay-subtle p-3.5 space-y-3"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Icon.flow className="size-4 text-brass shrink-0" aria-hidden="true" />
          <span className="font-display text-[11px] uppercase tracking-[0.18em] text-text-muted">
            Direct Publication Flywheel
          </span>
          {roCrateDigest && (
            <span className="rounded border border-emerald-400/30 bg-emerald-400/10 px-2 py-0.5 font-mono text-[9px] text-emerald-200">
              RO-Crate {roCrateDigest.slice(0, 18)}
            </span>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            data-testid="flywheel-advance-btn"
            disabled={!trimmedId || busy}
            onClick={() => runAdvance('replay_verify')}
            className="inline-flex items-center gap-1.5 rounded border border-[#D4AF37]/50 bg-[#78350F]/30 px-2.5 py-1 font-mono text-[11px] font-semibold text-[#F3E5AB] hover:bg-[#78350F]/50 disabled:opacity-40"
          >
            <Icon.play className="size-3 shrink-0" aria-hidden="true" />
            <span>{busy ? 'Advancing…' : 'Advance Full Flywheel'}</span>
          </button>
          <button
            type="button"
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
            className="rounded border border-brass/30 bg-brass/10 px-2.5 py-1 font-mono text-[11px] text-brass hover:bg-brass/20 disabled:opacity-40"
          >
            Draft IMRaD
          </button>
          <button
            type="button"
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
            className="rounded border border-cyan/30 bg-cyan/10 px-2.5 py-1 font-mono text-[11px] text-cyan hover:bg-cyan/20 disabled:opacity-40"
          >
            Render LaTeX
          </button>
          <button
            type="button"
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
            className="rounded border border-violet-400/30 bg-violet-400/10 px-2.5 py-1 font-mono text-[11px] text-violet-200 hover:bg-violet-400/20 disabled:opacity-40"
          >
            Finding Page HTML
          </button>
          <button
            type="button"
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
            className="rounded border border-emerald-400/30 bg-emerald-400/10 px-2.5 py-1 font-mono text-[11px] text-emerald-200 hover:bg-emerald-400/20 disabled:opacity-40"
          >
            Execute Replay
          </button>
        </div>
      </div>

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
              data-testid={`flywheel-step-${step.id}`}
              disabled={!trimmedId || busy}
              onClick={() => runAdvance(step.targetStage)}
              className={`flex items-center justify-between gap-1.5 rounded-lg border px-2.5 py-2 text-left font-mono text-[10px] transition ${
                done
                  ? 'border-emerald-400/45 bg-emerald-400/10 text-emerald-200'
                  : 'border-border-subtle bg-black/25 text-text-secondary hover:border-brass/40 hover:text-text-primary'
              } disabled:opacity-40`}
            >
              <span className="truncate">
                {index + 1}. {step.label}
              </span>
              {done ? (
                <Icon.check className="size-3 shrink-0 text-emerald-300" aria-hidden="true" />
              ) : (
                <Icon.chevR className="size-3 shrink-0 text-text-muted" aria-hidden="true" />
              )}
            </button>
          );
        })}
      </div>

      {previewContent && (
        <div
          data-testid="studio-artifact-preview"
          className="rounded-lg border border-brass/30 bg-black/50 p-3"
        >
          <div className="mb-2 flex items-center justify-between">
            <span className="font-mono text-xs font-semibold text-brass">{previewTitle}</span>
            <button
              type="button"
              onClick={() => {
                setPreviewContent(null);
                setPreviewTitle(null);
              }}
              className="font-mono text-[11px] text-text-muted hover:text-text-secondary"
            >
              Close Preview
            </button>
          </div>
          <pre className="max-h-56 overflow-auto whitespace-pre-wrap font-mono text-[11px] text-text-secondary">
            {previewContent}
          </pre>
        </div>
      )}
    </div>
  );
}
