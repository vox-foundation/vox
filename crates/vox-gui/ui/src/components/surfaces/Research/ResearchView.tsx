import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import { listenScientiaQueue } from '../../../transport';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useLabel } from '../../../hooks/useLanguage';
import { PipelineTimeline } from '../../PipelineTimeline';
import { RESEARCH_STAGES, deriveStages } from '../../../lib/pipeline';
import { startResearchAsync } from './researchActions';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { ResearchClaimAccordion, type ResearchClaimRow } from './ResearchClaimAccordion';
import { HeadlineVerdictBanner } from './HeadlineVerdictBanner';
import { ResearchReportMarkdown } from './ResearchReportMarkdown';
import { DocPublishModal } from './DocPublishModal';
import { ResearchDagCanvas, type DagNode, type DagEdge } from './ResearchDagCanvas';
import { SandboxReplModal } from './SandboxReplModal';
import { LiveSourceProber } from './LiveSourceProber';
import { JudgeInspector } from './JudgeInspector';
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '../../ui/Dialog';
import { SafeExternalLink } from './SafeExternalLink';

interface ResearchSession { id: number; status: string; query_text: string; started_at_ms: number; finished_at_ms: number | null; }

/** Raw claim shape as emitted by `get_research_session_detail` (snake_case,
 * matching `ResearchClaimDto` in `crates/vox-gui/src/commands/scientia.rs`). */
interface ResearchDetailClaim {
  claim_id: string;
  text: string;
  verdict: string;
  confidence: number;
  resample_stability: number;
  citation_urls: string[];
  corroboration_count: number;
}

// confidence_tier / claims / source_count / citation_precision are parsed
// out of the session's `artifact_json` on the Rust side (see
// `extract_research_summary` in scientia.rs) and are `undefined`/empty only
// when no artifact was persisted for the session (or it failed to parse) —
// the trust UI below stays hidden in that case rather than fabricating data.
interface ResearchDetail {
  session: ResearchSession;
  report_markdown: string | null;
  artifact_json: string | null;
  confidence_tier?: 'Direct' | 'Light' | 'DeepResearch';
  claims?: ResearchDetailClaim[];
  source_count?: number;
  citation_precision?: number;
  citations?: Array<{ url: string }>;
}

/**
 * Map the backend's flat claim DTO onto `ResearchClaimAccordion`'s
 * `ResearchClaimRow` shape. Trust is derived from `corroboration_count`,
 * the pipeline's real distinct-domain independent-source count (see
 * `vox_search::corroboration` / `compute_corroboration_counts` in
 * `vox-research-shim`'s pipeline.rs) — a claim backed by 2+ distinct-domain
 * supporting citations reads as corroborated, 0 or 1 as uncorroborated.
 */
function toClaimRows(claims: ResearchDetailClaim[] | undefined): ResearchClaimRow[] {
  if (!claims) return [];
  return claims.map((c) => {
    const distinctUrls = Array.from(new Set(c.citation_urls));
    return {
      claimId: c.claim_id,
      text: c.text,
      verdict: c.verdict,
      confidence: c.confidence,
      resampleStability: c.resample_stability,
      citations: distinctUrls.map((url) => ({
        url,
        trust:
          c.corroboration_count >= 2
            ? { kind: 'corroborated' as const, sourceCount: c.corroboration_count }
            : { kind: 'uncorroborated' as const },
      })),
    };
  });
}

interface MisguidanceFlagModalProps {
  isOpen: boolean;
  onClose: () => void;
  sessionId: number | null;
  queryText: string;
  culpritUrl: string | null;
  pushToast?: SurfaceDecoratorProps['pushToast'];
}

function extractDomain(url?: string | null): string {
  if (!url) return 'unknown';
  try {
    const parsed = new URL(url);
    return parsed.hostname;
  } catch {
    return url.replace(/^https?:\/\//, '').split('/')[0] || 'unknown';
  }
}

export function MisguidanceFlagModal({
  isOpen,
  onClose,
  sessionId,
  queryText,
  culpritUrl,
  pushToast,
}: MisguidanceFlagModalProps) {
  const [defectClass, setDefectClass] = useState<string>('inelegant_code');
  const [notes, setNotes] = useState<string>('');
  const [submitting, setSubmitting] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setDefectClass('inelegant_code');
      setNotes('');
      setError(null);
      setSubmitting(false);
    }
  }, [isOpen]);

  const handleSubmit = async () => {
    setSubmitting(true);
    setError(null);
    try {
      const domain = extractDomain(culpritUrl);
      const params = {
        session_id: sessionId ?? null,
        defect_class: defectClass,
        culprit_url: culpritUrl ?? null,
        culprit_domain: domain,
        claim_id: null,
        research_query: queryText || 'Research citation',
        misleading_excerpt: null,
        generated_code_snippet: null,
        failure_diagnostic: null,
        correction_diff: notes.trim() || null,
        reporter: 'user',
        domain_penalty: 0.2,
      };
      await invoke('flag_research_misleading', { params });
      pushToast?.({
        tone: 'ok',
        title: 'Citation Flagged',
        body: `Reported misguidance for ${domain}`,
        cause: 'backend-ok',
      });
      onClose();
    } catch (err) {
      setError(sanitizeErrorForToast(err));
    } finally {
      setSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="z-50 max-w-md bg-overlay-subtle border border-border-subtle text-text-primary rounded-xl p-6 shadow-xl">
        <div className="flex items-start justify-between pb-3 border-b border-border-subtle">
          <div>
            <DialogTitle className="font-display text-base font-semibold text-text-primary tracking-wide">
              Flag Misleading Research
            </DialogTitle>
            <DialogDescription className="mt-0.5 text-xs text-text-muted">
              Report misleading citations or defective output to penalize low-quality sources.
            </DialogDescription>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="text-text-muted hover:text-text-secondary text-lg leading-none px-2 py-1 rounded"
          >
            ✕
          </button>
        </div>

        {error && (
          <div className="mt-3 p-2.5 rounded border border-red-500/30 bg-red-500/10 text-xs text-red-400">
            {error}
          </div>
        )}

        <div className="mt-4 space-y-4">
          {culpritUrl && (
            <div className="text-[11px] font-mono text-text-muted truncate">
              Source: <span className="text-brass">{culpritUrl}</span>
            </div>
          )}

          <div className="space-y-1.5">
            <label htmlFor="defect-class-selector" className="block text-xs font-medium text-text-secondary">
              Defect Classification
            </label>
            <select
              id="defect-class-selector"
              data-testid="defect-class-selector"
              value={defectClass}
              onChange={(e) => setDefectClass(e.target.value)}
              className="w-full rounded border border-border-subtle bg-black/40 px-3 py-2 text-xs text-text-primary outline-none focus:border-brass/50"
            >
              <option value="inelegant_code">Inelegant Code</option>
              <option value="fails_to_run">Fails to Run</option>
              <option value="user_correction">User Corrected</option>
            </select>
          </div>

          <div className="space-y-1.5">
            <label htmlFor="flag-notes-input" className="block text-xs font-medium text-text-secondary">
              Notes / Diff
            </label>
            <textarea
              id="flag-notes-input"
              data-testid="flag-notes-input"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Describe why this source is misleading or provide a correction diff…"
              rows={3}
              className="w-full rounded border border-border-subtle bg-black/40 px-3 py-2 text-xs text-text-secondary outline-none focus:border-brass/50 resize-none leading-relaxed font-mono"
            />
          </div>
        </div>

        <div className="mt-6 flex items-center justify-end gap-2 pt-3 border-t border-border-subtle">
          <button
            type="button"
            onClick={onClose}
            disabled={submitting}
            className="rounded border border-border-subtle px-3 py-1.5 text-xs text-text-muted hover:text-text-secondary hover:border-border-base transition-colors disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            disabled={submitting}
            className="rounded border border-brass/40 bg-brass/20 hover:bg-brass/30 text-brass px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-50"
          >
            {submitting ? 'Submitting…' : 'Submit Flag'}
          </button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function ResearchView({ pushToast }: SurfaceDecoratorProps) {
  const embedded = useIsEmbeddedSurface();
  const [query, setQuery] = useState('');
  const [running, setRunning] = useState(false);
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);
  const [sessions, setSessions] = useState<ResearchSession[]>([]);
  const [detail, setDetail] = useState<ResearchDetail | null>(null);
  const [highlightedClaimId, setHighlightedClaimId] = useState<string | null>(null);
  const [isPublishModalOpen, setIsPublishModalOpen] = useState(false);
  const [isReplModalOpen, setIsReplModalOpen] = useState(false);
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [showJudgeInspector, setShowJudgeInspector] = useState(false);
  const [isFlagModalOpen, setIsFlagModalOpen] = useState(false);
  const [flagTargetUrl, setFlagTargetUrl] = useState<string | null>(null);

  const handleCitationClick = useCallback(
    (num: number) => {
      const claimRows = toClaimRows(detail?.claims);
      if (claimRows.length === 0) return;

      const matchingClaim =
        claimRows.find((c) => c.claimId === String(num) || c.claimId === `c${num}`) ??
        (num >= 1 && num <= claimRows.length ? claimRows[num - 1] : claimRows[0]);

      if (matchingClaim) {
        setHighlightedClaimId(matchingClaim.claimId);
        const targetId = `claim-${matchingClaim.claimId}`;
        const el = document.getElementById(targetId);
        if (el) {
          el.scrollIntoView({ behavior: 'smooth' });
        } else {
          setTimeout(() => {
            document.getElementById(targetId)?.scrollIntoView({ behavior: 'smooth' });
          }, 50);
        }
      }
    },
    [detail?.claims]
  );

  const loadHistory = useCallback(async () => {
    try {
      setSessions(await invoke<ResearchSession[]>('list_research_sessions', { limit: 25 }));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'History load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  const openDetail = useCallback(async (id: number) => {
    try {
      setDetail(await invoke<ResearchDetail>('get_research_session_detail', { sessionId: id }));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Session load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  useEffect(() => { loadHistory(); }, [loadHistory]);

  // A2: refetch history whenever the persistent daemon's Scientia-queue watcher
  // signals a research-session transition; a 10 s interval is the fallback
  // (e.g. outside Tauri, where listen() rejects), mirroring ScientiaDashboard.
  useEffect(() => {
    // Embedded mini-render: the initial loadHistory() (separate effect above)
    // already populated the thumbnail; skip the repeating poll + subscription.
    if (embedded) return;
    const id = setInterval(loadHistory, 10_000);
    let unlisten: (() => void) | undefined;
    listenScientiaQueue(() => { void loadHistory(); })
      .then((fn) => { unlisten = fn; })
      .catch(() => { /* not in Tauri — interval fallback covers it */ });
    return () => { clearInterval(id); unlisten?.(); };
  }, [loadHistory, embedded]);

  // Once the active run reaches a terminal status, stop the running indicator
  // and open its detail so the answer (report_markdown ?? artifact_json) shows.
  useEffect(() => {
    if (activeSessionId == null) return;
    const s = sessions.find((x) => x.id === activeSessionId);
    if (s && (s.status === 'completed' || s.status === 'failed')) {
      setRunning(false);
      void openDetail(activeSessionId);
      setActiveSessionId(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessions, activeSessionId]);

  const run = async () => {
    if (!query.trim()) return;
    setRunning(true);
    setActiveSessionId(null);
    try {
      // A2: fire-and-forget via the persistent daemon's async executor. Returns
      // {session_id, task_id, status: "running"} immediately — does NOT block on
      // the pipeline. Status transitions arrive through the queue watcher below.
      const handle = await startResearchAsync({ query, verifyClaims: true });
      setActiveSessionId(handle.session_id);
      await loadHistory();
    } catch (err) {
      setRunning(false);
      pushToast({ tone: 'warn', title: 'Research run failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  return (
    <section className="space-y-4">
      <h2 className="font-display text-lg text-text-primary tracking-wider uppercase">{useLabel('research')}</h2>

      <div className="flex gap-2">
        <input value={query} onChange={e => setQuery(e.target.value)} placeholder="Ask a research question…"
          aria-label="Research question"
          onKeyDown={e => { if (e.key === 'Enter') void run(); }}
          className="flex-1 rounded-lg border border-border-subtle bg-black/40 px-3 py-2 text-sm text-text-secondary outline-hidden focus:border-brass/40" />
        <button
          type="button"
          data-testid="toggle-diagnostics-btn"
          onClick={() => setShowDiagnostics(prev => !prev)}
          className={`rounded-lg border px-3 py-2 text-sm transition-colors ${
            showDiagnostics
              ? 'border-brass bg-brass/20 text-brass'
              : 'border-border-subtle bg-black/40 text-text-secondary hover:text-text-primary'
          }`}
        >
          Diagnostic Prober
        </button>
        <button type="button" onClick={run} disabled={running}
          className="rounded-lg border border-brass/30 bg-brass/10 px-4 py-2 text-sm text-brass hover:bg-brass/20 disabled:opacity-50">
          {running ? 'Running…' : 'Run'}
        </button>
      </div>
      {showDiagnostics && (
        <LiveSourceProber initialQuery={query} />
      )}
      {running && (
        <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-3" aria-live="polite">
          <PipelineTimeline stages={RESEARCH_STAGES} statuses={deriveStages('active')} />
          <div className="mt-2 text-[11px] text-text-muted">
            Running in the background{activeSessionId != null ? ` (session ${activeSessionId})` : ''} — the answer opens automatically when it completes.
          </div>
        </div>
      )}

      <div>
        <div className="mb-2 flex items-center justify-between">
          <span className="font-display text-[12px] uppercase tracking-wide text-text-muted">Recent sessions</span>
          <button type="button" onClick={loadHistory} className="text-[11px] text-text-muted hover:text-text-secondary">Refresh</button>
        </div>
        {sessions.length === 0 ? (
          <div className="rounded-lg border border-dashed border-border-subtle py-6 text-center text-[11px] text-text-muted">
            No research sessions yet — ask a question above to run one.
          </div>
        ) : (
          <ul className="space-y-1">
            {sessions.map(s => (
              <li key={s.id}>
                <button type="button" onClick={() => openDetail(s.id)}
                  className="flex w-full items-center justify-between rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-2 text-left hover:bg-overlay-subtle">
                  <span className="truncate text-[12px] text-text-secondary">{s.query_text}</span>
                  <span className="ml-3 shrink-0 font-mono text-[10px] text-text-muted">{s.status}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      {detail && (
        <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-3">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-[12px] text-text-secondary">Session {detail.session.id}</span>
            <div className="flex items-center gap-2">
              <button
                type="button"
                data-testid="toggle-judge-btn"
                onClick={() => setShowJudgeInspector(prev => !prev)}
                className={`rounded border px-2.5 py-1 text-[11px] transition-colors ${
                  showJudgeInspector
                    ? 'border-brass bg-brass/20 text-brass'
                    : 'border-border-subtle bg-black/40 text-text-secondary hover:text-text-primary hover:bg-black/60'
                }`}
              >
                Judge Inspector
              </button>
              <button
                type="button"
                onClick={() => setIsReplModalOpen(true)}
                className="rounded border border-border-subtle bg-black/40 px-2.5 py-1 text-[11px] text-text-secondary hover:text-text-primary hover:bg-black/60"
              >
                Sandbox REPL
              </button>
              <button
                type="button"
                onClick={() => setIsPublishModalOpen(true)}
                className="rounded border border-brass/40 bg-brass/10 px-2.5 py-1 text-[11px] text-brass hover:bg-brass/20"
              >
                Publish Architecture SSOT
              </button>
              <button type="button" onClick={() => setDetail(null)} className="text-[11px] text-text-muted hover:text-text-secondary">Close</button>
            </div>
          </div>
          <PipelineTimeline stages={RESEARCH_STAGES} statuses={deriveStages(detail.session.status)} />
          {showJudgeInspector && (
            <div className="mt-2">
              <JudgeInspector
                confidenceTier={detail.confidence_tier}
                sourceCount={detail.source_count}
                citationPrecision={detail.citation_precision}
                claims={detail.claims}
              />
            </div>
          )}
          {(() => {
            const claimRows = toClaimRows(detail.claims);
            const distinctCorroboratingSources = new Set(
              claimRows.flatMap((c) =>
                c.citations.filter((cite) => cite.trust.kind === 'corroborated').map((cite) => cite.url)
              )
            ).size;
            const contradictedClaims = claimRows.filter(
              (c) => c.verdict.toLowerCase() === 'refuted' || c.verdict.toLowerCase() === 'contradicted'
            ).length;
            const contestedClaims = claimRows.filter((c) => c.verdict === 'Contested').length;
            const allCitations = Array.from(
              new Set([
                ...(detail.claims?.flatMap((c) => c.citation_urls) ?? []),
                ...(detail.citations?.map((c) => c.url) ?? []),
              ])
            ).filter(Boolean);

            const dagNodes: DagNode[] = claimRows.map((c, i) => {
              const isContradicted =
                c.verdict.toLowerCase() === 'refuted' || c.verdict.toLowerCase() === 'contradicted';
              const isVerified =
                c.verdict.toLowerCase() === 'verified' || c.verdict.toLowerCase() === 'supported';
              return {
                id: c.claimId,
                label: `${c.claimId}: ${c.text}`,
                wave: Math.floor(i / 2),
                status: isContradicted ? 'contradicted' : isVerified ? 'verified' : 'pending',
              };
            });
            const dagEdges: DagEdge[] = dagNodes.slice(1).map((node, i) => ({
              srcId: dagNodes[i].id,
              dstId: node.id,
              relation: node.status === 'contradicted' ? 'contradicts' : 'supports',
            }));

            return (
              <>
                {claimRows.length > 0 && (
                  <HeadlineVerdictBanner
                    confidenceTier={detail.confidence_tier ?? 'DeepResearch'}
                    corroboratingSources={distinctCorroboratingSources}
                    contestedClaims={contestedClaims}
                    contradictedClaims={contradictedClaims}
                    totalClaims={claimRows.length}
                  />
                )}
                <div className="mt-2 max-h-[360px] overflow-auto">
                  <ResearchReportMarkdown
                    markdown={detail.report_markdown ?? detail.artifact_json ?? '(no artifact persisted)'}
                    onCitationClick={handleCitationClick}
                  />
                </div>
                {claimRows.length > 0 && (
                  <div className="mt-2">
                    <ResearchClaimAccordion
                      claims={claimRows}
                      sourceCount={detail.source_count ?? 0}
                      highlightedClaimId={highlightedClaimId ?? undefined}
                    />
                  </div>
                )}
                {allCitations.length > 0 && (
                  <div className="mt-3 rounded-lg border border-border-subtle bg-black/20 p-3">
                    <div className="mb-2 text-[11px] font-mono uppercase tracking-wider text-text-muted">
                      Citations & Sources
                    </div>
                    <ul className="space-y-2" role="list">
                      {allCitations.map((url) => (
                        <li key={url} role="listitem" className="flex items-center justify-between gap-2">
                          <SafeExternalLink
                            url={url}
                            className="truncate text-[12px] text-brass underline decoration-dotted hover:text-brass/80"
                          />
                          <button
                            type="button"
                            data-testid="flag-citation-misleading"
                            onClick={() => {
                              setFlagTargetUrl(url);
                              setIsFlagModalOpen(true);
                            }}
                            className="shrink-0 rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 text-[11px] text-text-secondary hover:text-text-primary hover:border-brass/40 transition-colors"
                          >
                            Flag Citation
                          </button>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
                {dagNodes.length > 0 && (
                  <div className="mt-3">
                    <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-text-muted">
                      Epistemic Research DAG
                    </div>
                    <ResearchDagCanvas nodes={dagNodes} edges={dagEdges} />
                  </div>
                )}
              </>
            );
          })()}
          <DocPublishModal
            sessionId={detail.session.id}
            isOpen={isPublishModalOpen}
            onClose={() => setIsPublishModalOpen(false)}
            onPublished={(path) => {
              pushToast?.({
                tone: 'ok',
                title: 'Published Architecture SSOT',
                body: `Published ${path}`,
                cause: 'backend-ok',
              });
            }}
          />
          <SandboxReplModal
            isOpen={isReplModalOpen}
            onClose={() => setIsReplModalOpen(false)}
          />
          <MisguidanceFlagModal
            isOpen={isFlagModalOpen}
            onClose={() => {
              setIsFlagModalOpen(false);
              setFlagTargetUrl(null);
            }}
            sessionId={detail.session.id}
            queryText={detail.session.query_text}
            culpritUrl={flagTargetUrl}
            pushToast={pushToast}
          />
        </div>
      )}
    </section>
  );
}
