import React, { useCallback, useEffect, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invokeCmd, listenScientiaQueue } from '../../../transport';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useLabel } from '../../../hooks/useLanguage';
import { PipelineTimeline } from '../../PipelineTimeline';
import { RESEARCH_STAGES, deriveStages } from '../../../lib/pipeline';
import {
  startResearchAsync,
  getResearchEngineStatus,
  safeParseJson,
  type ResearchEngineStatusDto,
  type EpistemicDagDto,
} from './researchActions';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import type { ResearchClaimRow } from './ResearchClaimAccordion';
import { VirtualizedClaimsTable } from './VirtualizedClaimsTable';
import { VirtualizedFrameChunkTable, type FrameChunkPage } from './VirtualizedFrameChunkTable';
import { HeadlineVerdictBanner } from './HeadlineVerdictBanner';
import { ResearchReportMarkdown } from './ResearchReportMarkdown';
import { DocPublishModal } from './DocPublishModal';
import { ResearchDagCanvas, type DagNode, type DagEdge } from './ResearchDagCanvas';
import { SandboxReplModal } from './SandboxReplModal';
import { LiveSourceProber } from './LiveSourceProber';
import { JudgeInspector } from './JudgeInspector';
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '../../ui/Dialog';
import { SafeExternalLink } from './SafeExternalLink';
import { ResearchEngineDrawer } from './ResearchEngineDrawer';
import { MultiWaveProgressTimeline } from './MultiWaveProgressTimeline';
import { ShoppingDecisionCard, type ShoppingDecisionPayload } from './ShoppingDecisionCard';
import { CodeGenVerificationCard, type CodeGenVerificationPayload } from './CodeGenVerificationCard';
import { BeforeAfterAuditInspector } from '../Scientia/BeforeAfterAuditInspector';
import { promoteCandidate } from '../../../api/scientia';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';

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
  wave_count?: number;
  wave_stability?: number;
  quality_score?: number;
  synthesis_model?: string;
  domain_mode?: 'general' | 'shopping' | 'codegen' | 'scholarly';
  shopping_decision?: ShoppingDecisionPayload;
  codegen_verification?: CodeGenVerificationPayload;
  epistemic_dag?: EpistemicDagDto;
  claims_total?: number;
  sources_total?: number;
  has_spilled_artifact?: boolean;
  frame_stream_id?: string | null;
  frame_chunk?: FrameChunkPage | null;
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
      await invokeCmd('flag_research_misleading', { params });
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
            className="inline-flex items-center justify-center text-text-muted hover:text-text-secondary text-lg leading-none px-2 py-1 rounded"
          >
            <Icon.close className="size-4 shrink-0" aria-hidden="true" />
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
            <VoxSelect
              id="defect-class-selector"
              aria-label="Defect Classification"
              data-testid="defect-class-selector"
              value={defectClass}
              onValueChange={setDefectClass}
              options={[
                { value: 'inelegant_code', label: 'Inelegant Code' },
                { value: 'fails_to_run', label: 'Fails to Run' },
                { value: 'user_correction', label: 'User Corrected' },
              ]}
              className="w-full border-border-subtle bg-black/40 text-xs text-text-primary"
            />
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

export interface ResearchViewProps {
  pushToast?: SurfaceDecoratorProps['pushToast'];
  gamifyEnabled?: boolean;
  initialLowEvidence?: boolean;
  initialLane?: 'fast' | 'deep';
  onOpenEngineDrawer?: () => void;
}

export function ResearchView({
  pushToast,
  initialLowEvidence = false,
  initialLane = 'fast',
  onOpenEngineDrawer,
}: ResearchViewProps = {}) {
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
  const [showAuditInspector, setShowAuditInspector] = useState(false);
  const [showJudgeInspector, setShowJudgeInspector] = useState(false);
  const [isFlagModalOpen, setIsFlagModalOpen] = useState(false);
  const [flagTargetUrl, setFlagTargetUrl] = useState<string | null>(null);
  const [isSavingToKb, setIsSavingToKb] = useState(false);

  const [lane, setLane] = useState<'fast' | 'deep'>(initialLane);
  const [domainMode, setDomainMode] = useState<'general' | 'shopping' | 'codegen' | 'scholarly'>('general');
  const [scope, setScope] = useState<'web' | 'local' | 'both'>('both');
  const [waves, setWaves] = useState<1 | 2 | 3 | 4 | 5>(1);
  const [siteScope, setSiteScope] = useState('');
  const [isPromotingScientia, setIsPromotingScientia] = useState(false);
  const [engineStatus, setEngineStatus] = useState<ResearchEngineStatusDto | null>(null);
  const [lowEvidence, setLowEvidence] = useState(initialLowEvidence);
  const [isEngineDrawerOpen, setIsEngineDrawerOpen] = useState(false);
  const [liveWave, setLiveWave] = useState<number | null>(null);
  const [liveTotalWaves, setLiveTotalWaves] = useState<number | null>(null);
  const [liveStability, setLiveStability] = useState<number | null>(null);
  const [resyncNotice, setResyncNotice] = useState<{ oldestAvailableSeq?: number; latestSeq?: number } | null>(null);
  const [loadingMoreClaims, setLoadingMoreClaims] = useState(false);
  const [framePage, setFramePage] = useState<FrameChunkPage | null>(null);
  const [loadingFrameChunk, setLoadingFrameChunk] = useState(false);
  const lastSeqRef = useRef<number>(0);

  const openEngineDrawer = () => {
    if (onOpenEngineDrawer) {
      onOpenEngineDrawer();
    } else {
      setIsEngineDrawerOpen(true);
    }
  };

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
        if (el && typeof el.scrollIntoView === 'function') {
          el.scrollIntoView({ behavior: 'smooth' });
        } else {
          setTimeout(() => {
            if (typeof document !== 'undefined') {
              const target = document.getElementById(targetId);
              if (target && typeof target.scrollIntoView === 'function') {
                target.scrollIntoView({ behavior: 'smooth' });
              }
            }
          }, 50);
        }
      }
    },
    [detail?.claims]
  );

  const loadEngineStatus = useCallback(async () => {
    try {
      const status = await getResearchEngineStatus();
      setEngineStatus(status);
    } catch {
      // offline or testing fallback
    }
  }, []);

  const loadHistory = useCallback(async () => {
    try {
      setSessions(await invokeCmd<ResearchSession[]>('list_research_sessions', { limit: 25 }));
    } catch (err) {
      pushToast?.({ tone: 'warn', title: 'History load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  const openDetail = useCallback(async (id: number) => {
    try {
      const res = await invokeCmd<ResearchDetail>('get_research_session_detail', { sessionId: id });
      setDetail(res);
      setFramePage(res.frame_chunk ?? null);
      if (
        res.session.status === 'failed' ||
        (res.claims && res.claims.length === 0) ||
        (res.source_count !== undefined && res.source_count === 0)
      ) {
        setLowEvidence(true);
      } else {
        setLowEvidence(false);
      }
    } catch (err) {
      pushToast?.({ tone: 'warn', title: 'Session load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  const loadMoreClaims = useCallback(async () => {
    if (!detail?.session?.id || loadingMoreClaims) return;
    const offset = detail.claims?.length ?? 0;
    setLoadingMoreClaims(true);
    try {
      const resp = await fetch(`/api/v2/research/sessions/${detail.session.id}/claims?offset=${offset}&limit=100`);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const json = (await resp.json()) as Record<string, any>;
      const payload = json?.data ?? json;
      const nextClaims: ResearchDetailClaim[] = Array.isArray(payload?.claims)
        ? payload.claims
        : Array.isArray(payload?.items)
          ? payload.items
          : Array.isArray(payload)
            ? payload
            : [];
      if (nextClaims.length > 0) {
        setDetail((prev) => {
          if (!prev) return prev;
          const existing = prev.claims ?? [];
          const seen = new Set(existing.map((c) => c.claim_id));
          const merged = [...existing, ...nextClaims.filter((c) => !seen.has(c.claim_id))];
          return { ...prev, claims: merged };
        });
      }
    } catch (err) {
      pushToast?.({ tone: 'warn', title: 'Claims page load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoadingMoreClaims(false);
    }
  }, [detail?.session?.id, detail?.claims?.length, loadingMoreClaims, pushToast]);

  const loadFrameChunk = useCallback(async (streamId: string, offset = 0, limit = 100) => {
    if (!streamId || loadingFrameChunk) return;
    setLoadingFrameChunk(true);
    try {
      const resp = await fetch(
        `/api/v2/research/frame-chunks/${encodeURIComponent(streamId)}?offset=${offset}&limit=${limit}`,
      );
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const json = (await resp.json()) as Record<string, any>;
      const page = (json?.data ?? json) as FrameChunkPage;
      setFramePage((prev) => {
        if (!prev || offset === 0) return { ...page, stream_id: page.stream_id ?? streamId };
        return {
          ...page,
          stream_id: page.stream_id ?? streamId,
          offset: prev.offset,
          rows: [...prev.rows, ...page.rows],
        };
      });
    } catch (err) {
      pushToast?.({ tone: 'warn', title: 'Frame chunk load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoadingFrameChunk(false);
    }
  }, [loadingFrameChunk, pushToast]);

  useEffect(() => {
    loadEngineStatus();
    loadHistory();
  }, [loadEngineStatus, loadHistory]);

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

  // STRM-01/04: resumable ?from_seq= SSE delta subscription for active research runs.
  useEffect(() => {
    if (embedded || activeSessionId == null || typeof EventSource === 'undefined') return;
    const url = `/api/v2/research/sessions/${activeSessionId}/events?from_seq=${lastSeqRef.current}`;
    const es = new EventSource(url);

    const handleDeltaData = (raw: string, explicitType?: string) => {
      const parsed = safeParseJson<Record<string, any>>(raw);
      if (!parsed) return;
      if (typeof parsed.seq === 'number') {
        lastSeqRef.current = Math.max(lastSeqRef.current, parsed.seq);
      }
      const msgType = explicitType ?? parsed.type ?? parsed.event?.event_type ?? parsed.event_type;
      if (msgType === 'resync_required') {
        setResyncNotice({
          oldestAvailableSeq: typeof parsed.oldest_available_seq === 'number' ? parsed.oldest_available_seq : undefined,
          latestSeq: typeof parsed.latest_seq === 'number' ? parsed.latest_seq : undefined,
        });
        if (typeof parsed.latest_seq === 'number') {
          lastSeqRef.current = parsed.latest_seq;
        }
        void openDetail(activeSessionId);
        void loadHistory();
        return;
      }
      const evt = (parsed.event && typeof parsed.event === 'object' ? parsed.event : parsed) as Record<string, any>;
      const evtType = evt.event_type ?? msgType;
      if (evtType === 'WaveStarted') {
        if (typeof evt.wave === 'number') setLiveWave(evt.wave);
        if (typeof evt.max_waves === 'number') setLiveTotalWaves(evt.max_waves);
      } else if (evtType === 'WaveCompleted') {
        if (typeof evt.wave === 'number') setLiveWave(evt.wave);
        if (typeof evt.stability_score === 'number') setLiveStability(evt.stability_score);
      } else if (evtType === 'EpistemicDagDelta' || evtType === 'ClaimVerified' || evtType === 'ClaimExtracted') {
        if (typeof evt.wave === 'number') setLiveWave(evt.wave);
        void openDetail(activeSessionId);
      }
    };

    es.onmessage = (ev) => handleDeltaData(ev.data);
    es.addEventListener('resync_required', (ev) => handleDeltaData((ev as MessageEvent).data, 'resync_required'));
    es.addEventListener('delta', (ev) => handleDeltaData((ev as MessageEvent).data));
    return () => { es.close(); };
  }, [activeSessionId, embedded, openDetail, loadHistory]);

  // Once the active run reaches a terminal status, stop the running indicator
  // and open its detail so the answer (report_markdown ?? artifact_json) shows.
  useEffect(() => {
    if (activeSessionId == null) return;
    const s = sessions.find((x) => x.id === activeSessionId);
    if (s && (s.status === 'completed' || s.status === 'failed')) {
      setRunning(false);
      void openDetail(activeSessionId);
      if (s.status === 'failed') {
        setLowEvidence(true);
      }
      setActiveSessionId(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- react to session completion transitions
  }, [sessions, activeSessionId]);

  const run = async (overrideLane?: 'fast' | 'deep') => {
    const activeQuery = query.trim() || detail?.session.query_text;
    if (!activeQuery) return;
    if (!query.trim() && activeQuery) {
      setQuery(activeQuery);
    }
    const activeLane = overrideLane ?? lane;
    lastSeqRef.current = 0;
    setLiveWave(null);
    setLiveTotalWaves(null);
    setLiveStability(null);
    setResyncNotice(null);
    setRunning(true);
    setLowEvidence(false);
    setActiveSessionId(null);
    try {
      // A2: fire-and-forget via the persistent daemon's async executor. Returns
      // {session_id, task_id, status: "running"} immediately — does NOT block on
      // the pipeline. Status transitions arrive through the queue watcher below.
      const handle = await startResearchAsync({
        query: activeQuery,
        verifyClaims: true,
        lane: activeLane,
        scope,
        waves,
        domainMode,
        siteScope: siteScope.trim() || undefined,
      });
      setActiveSessionId(handle.session_id);
      await loadHistory();
    } catch (err) {
      setRunning(false);
      pushToast?.({ tone: 'warn', title: 'Research run failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const activeProviderCount =
    engineStatus?.providers.filter((p) => p.has_key || p.is_keyless).length ?? 0;
  const totalProviderCount = engineStatus?.providers.length ?? 0;

  return (
    <section className="@container/surface flex min-w-0 flex-col gap-4">
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE · RESEARCH"
        title="Research Syntheses & Epistemic Investigation"
        subtitle="Autonomous web research, epistemic DAG synthesis, and cross-source verification."
        waxSealBadge="EPISTEMIC SYNC"
        icon={<Icon.search className="size-5 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'SESSIONS',
            valueText: `${sessions.length}`,
            ratio: Math.max(0, Math.min(1, sessions.length / 20)),
            tone: 'gold',
            purpose: 'Total completed research investigation sessions',
            actionHint: 'Refresh research session history',
            onClick: () => {
              void loadHistory();
            },
          },
          {
            label: 'PROVIDERS',
            valueText: `${activeProviderCount}`,
            ratio: totalProviderCount > 0 ? Math.max(0, Math.min(1, activeProviderCount / totalProviderCount)) : 0,
            tone: activeProviderCount > 0 ? 'emerald' : 'gold',
            purpose: 'Active research providers and quota status',
            actionHint: 'Configure research providers and API keys',
            active: isEngineDrawerOpen,
            onClick: openEngineDrawer,
          },
          {
            label: 'LANE',
            valueText: lane === 'deep' ? 'DEEP' : 'FAST',
            ratio: lane === 'deep' ? 1 : Math.max(0, Math.min(1, waves / 5)),
            tone: 'gold',
            purpose: 'Active research synthesis execution lane',
            actionHint: 'Toggle Fast / Deep Research lane',
            active: lane === 'deep',
            onClick: () => setLane((prev) => (prev === 'fast' ? 'deep' : 'fast')),
          },
        ]}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <span className="sr-only">{useLabel('research')}</span>
            <div role="tablist" aria-label="Research Lane" className="flex flex-wrap items-center rounded-lg border border-[#C59B27]/35 bg-bg-elevated p-0.5 text-xs">
              <button
                type="button"
                role="tab"
                data-testid="lane-switch-fast"
                aria-selected={lane === 'fast'}
                onClick={() => setLane('fast')}
                className={`inline-flex items-center gap-1 rounded px-3 py-1 text-xs font-medium transition-colors ${
                  lane === 'fast'
                    ? 'bg-[#78350F]/25 text-text-primary font-semibold shadow-xs'
                    : 'text-text-secondary hover:text-text-primary'
                }`}
              >
                <Icon.bolt className="size-3 text-brass" aria-hidden="true" />
                <span>Fast</span>
              </button>
              <button
                type="button"
                role="tab"
                data-testid="lane-switch-deep"
                aria-selected={lane === 'deep'}
                onClick={() => setLane('deep')}
                className={`inline-flex items-center gap-1 rounded px-3 py-1 text-xs font-medium transition-colors ${
                  lane === 'deep'
                    ? 'bg-[#78350F]/25 text-text-primary font-semibold shadow-xs'
                    : 'text-text-secondary hover:text-text-primary'
                }`}
              >
                <Icon.search className="size-3 text-brass" aria-hidden="true" />
                <span>Deep Research</span>
              </button>
            </div>
          </div>
        }
      />

      {engineStatus && (
        <div data-testid="provider-badge-strip" className="flex flex-wrap items-center gap-1.5 text-xs">
          {engineStatus.providers.map((p) => {
            if (p.is_keyless) {
              return (
                <span
                  key={p.id}
                  className="inline-flex items-center gap-1 rounded border border-emerald-600/35 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-mono text-(--color-status-pass)"
                >
                  <Icon.check className="size-3 shrink-0" aria-hidden="true" />
                  <span>{p.name}</span>
                </span>
              );
            }
            if (p.quota_usage) {
              const remaining = Math.max(0, p.quota_usage.units_limit - p.quota_usage.units_spent);
              return (
                <span
                  key={p.id}
                  className="inline-flex items-center gap-1 rounded border border-brass/40 bg-brass/10 px-2 py-0.5 text-[11px] font-mono text-brass"
                >
                  <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />
                  <span>{p.name} ({remaining}/{p.quota_usage.units_limit} left)</span>
                </span>
              );
            }
            if (p.has_key) {
              return (
                <span
                  key={p.id}
                  className="inline-flex items-center gap-1 rounded border border-brass/30 bg-brass/5 px-2 py-0.5 text-[11px] font-mono text-brass/80"
                >
                  <Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />
                  <span>{p.name}</span>
                </span>
              );
            }
            const offer = engineStatus.free_key_offers.find((o) => o.provider_id === p.id);
            const label = offer ? `+ ${p.name} (${offer.quota_summary} available)` : `+ ${p.name}`;
            return (
              <button
                key={p.id}
                type="button"
                onClick={openEngineDrawer}
                className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 text-[11px] font-mono text-text-muted hover:text-text-secondary hover:border-brass/40 transition-colors"
              >
                {label}
              </button>
            );
          })}
          <button
            type="button"
            data-testid="configure-engines-btn"
            aria-label="Configure search sources and API keys"
            onClick={openEngineDrawer}
            className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 text-[11px] text-text-secondary hover:text-text-primary hover:border-brass/40 transition-colors ml-auto"
          >
            <Icon.settings className="size-3 text-brass" aria-hidden="true" />
            <span>Sources &amp; Keys</span>
          </button>
        </div>
      )}

      {(() => {
        const tavilyUnconfigured = !engineStatus?.providers.find((p) => p.id === 'tavily')?.has_key;
        const searxngUnconfigured = !engineStatus?.providers.find((p) => p.id === 'searxng')?.is_enabled;
        const isDegraded = Boolean(engineStatus && tavilyUnconfigured && searxngUnconfigured);
        if (!isDegraded) return null;
        return (
          <div
            data-testid="degraded-setup-banner"
            className="rounded-lg border border-amber-600/35 bg-amber-500/10 p-3 text-xs text-(--color-status-warn) flex flex-wrap items-center justify-between gap-3"
            role="alert"
          >
            <div className="flex flex-wrap items-center gap-2 min-w-0">
              <span className="inline-flex items-center gap-1 font-semibold shrink-0">
                <Icon.alert className="size-3.5" aria-hidden="true" />
                <span>Research quality degraded</span>
              </span>
              <span className="text-text-secondary leading-relaxed">
                No premium search providers configured (Tavily key missing and SearXNG unconfigured). Research will rely on fallback sources.
              </span>
            </div>
            <button
              type="button"
              data-testid="degraded-setup-configure-btn"
              onClick={openEngineDrawer}
              className="rounded border border-amber-600/40 bg-amber-500/20 px-2.5 py-1 text-[11px] font-medium text-text-primary hover:bg-amber-500/30 transition-colors shrink-0"
            >
              Configure Keys
            </button>
          </div>
        );
      })()}

      {lowEvidence && (
        <div
          data-testid="empty-results-notice"
          className="rounded-lg border border-amber-600/35 bg-amber-500/10 p-3 text-xs text-(--color-status-warn) space-y-2"
          role="alert"
        >
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-semibold flex items-center gap-1.5">
              <Icon.alert className="size-3.5" aria-hidden="true" />
              <span>Insufficient Evidence Grounding</span>
            </span>
            <button
              type="button"
              data-testid="rerun-deep-lane-btn"
              onClick={() => {
                setLane('deep');
                void run('deep');
              }}
              className="rounded border border-amber-600/40 bg-amber-500/20 px-2.5 py-1 text-[11px] font-medium text-text-primary hover:bg-amber-500/30 transition-colors"
            >
              Re-run in Deep Lane
            </button>
          </div>
          <p className="text-text-secondary leading-relaxed">
            No verifiable claims could be corroborated across the active search providers with high confidence.
            Try switching to Deep Research lane for multi-hop retrieval or configuring additional search keys.
          </p>
        </div>
      )}

      <div className="flex flex-wrap gap-2">
        <input value={query} onChange={e => setQuery(e.target.value)} placeholder="Ask a research question…"
          aria-label="Research question"
          onKeyDown={e => { if (e.key === 'Enter') void run(); }}
          className="min-w-[12rem] flex-1 rounded-lg border border-border-subtle bg-bg-elevated px-3 py-2 text-sm text-text-primary outline-hidden focus:border-brass/40" />
        <button
          type="button"
          data-testid="toggle-diagnostics-btn"
          onClick={() => setShowDiagnostics(prev => !prev)}
          className={`rounded-lg border px-3 py-2 text-sm transition-colors ${
            showDiagnostics
              ? 'border-brass bg-brass/20 text-brass'
              : 'border-border-subtle bg-bg-elevated text-text-secondary hover:text-text-primary'
          }`}
        >
          Diagnostic Prober
        </button>
        <button
          type="button"
          data-testid="toggle-audit-inspector-btn"
          aria-expanded={showAuditInspector}
          onClick={() => setShowAuditInspector(prev => !prev)}
          className={`rounded-lg border px-3 py-2 text-sm transition-colors ${
            showAuditInspector
              ? 'border-brass bg-brass/20 text-brass'
              : 'border-border-subtle bg-bg-elevated text-text-secondary hover:text-text-primary'
          }`}
        >
          Before / After Audit Inspector
        </button>
        <button type="button" onClick={() => run()} disabled={running}
          className="rounded-lg border border-brass/30 bg-brass/10 px-4 py-2 text-sm text-brass hover:bg-brass/20 disabled:opacity-50">
          {running ? 'Running…' : 'Run'}
        </button>
      </div>

      <div
        data-testid="research-power-control-bar"
        className="flex flex-wrap items-center gap-3 rounded-lg border border-border-subtle bg-bg-elevated px-3 py-2 text-xs"
      >
        <div
          data-testid="research-domain-mode-group"
          role="group"
          aria-label="Domain Mode"
          className="flex items-center gap-1"
        >
          <span className="font-mono text-[10px] uppercase tracking-wider text-text-muted mr-1">Mode</span>
          {(['general', 'shopping', 'codegen', 'scholarly'] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              data-testid={`domain-mode-${mode}`}
              aria-pressed={domainMode === mode}
              onClick={() => setDomainMode(mode)}
              className={`rounded px-2 py-0.5 text-[11px] font-medium capitalize transition-colors ${
                domainMode === mode
                  ? 'border border-brass/40 bg-brass/20 text-brass'
                  : 'border border-transparent text-text-muted hover:text-text-secondary'
              }`}
            >
              {mode === 'codegen' ? 'CodeGen' : mode}
            </button>
          ))}
        </div>

        <div
          data-testid="research-scope-group"
          role="group"
          aria-label="Retrieval Scope"
          className="flex items-center gap-1"
        >
          <span className="font-mono text-[10px] uppercase tracking-wider text-text-muted mr-1">Scope</span>
          {(['web', 'local', 'both'] as const).map((s) => (
            <button
              key={s}
              type="button"
              data-testid={`research-scope-${s}`}
              aria-pressed={scope === s}
              onClick={() => setScope(s)}
              className={`rounded px-2 py-0.5 text-[11px] font-medium capitalize transition-colors ${
                scope === s
                  ? 'border border-brass/40 bg-brass/20 text-brass'
                  : 'border border-transparent text-text-muted hover:text-text-secondary'
              }`}
            >
              {s}
            </button>
          ))}
        </div>

        <div
          data-testid="research-waves-group"
          role="group"
          aria-label="Adversarial Waves"
          className="flex items-center gap-1"
        >
          <span className="font-mono text-[10px] uppercase tracking-wider text-text-muted mr-1">Waves</span>
          {([1, 2, 3, 4, 5] as const).map((w) => (
            <button
              key={w}
              type="button"
              data-testid={`research-waves-${w}`}
              aria-pressed={waves === w}
              onClick={() => setWaves(w)}
              className={`rounded px-2 py-0.5 text-[11px] font-mono transition-colors ${
                waves === w
                  ? 'border border-brass/40 bg-brass/20 text-brass'
                  : 'border border-transparent text-text-muted hover:text-text-secondary'
              }`}
            >
              {w}W
            </button>
          ))}
        </div>

        <div className="flex items-center gap-1.5 @2xl/surface:ml-auto">
          <label htmlFor="research-site-scope-input" className="font-mono text-[10px] uppercase tracking-wider text-text-muted">
            Site
          </label>
          <input
            id="research-site-scope-input"
            data-testid="research-site-scope-input"
            type="text"
            value={siteScope}
            onChange={(e) => setSiteScope(e.target.value)}
            placeholder="docs.rs, arxiv.org"
            className="w-32 max-w-full @sm/surface:w-36 rounded border border-border-subtle bg-black/40 px-2 py-0.5 text-[11px] font-mono text-text-secondary outline-hidden focus:border-brass/40"
          />
        </div>
      </div>

      {resyncNotice && (
        <div
          data-testid="research-resync-banner"
          role="status"
          className="rounded-lg border border-amber-600/35 bg-amber-500/10 px-3 py-2 text-xs text-(--color-status-warn) flex flex-wrap items-center justify-between gap-2"
        >
          <span className="inline-flex items-center gap-1.5">
            <Icon.alert className="size-3.5 shrink-0" aria-hidden="true" />
            <span>
              Stream delta window lagged — resynchronized session snapshot
              {resyncNotice.latestSeq != null ? ` (seq ${resyncNotice.latestSeq})` : ''}.
            </span>
          </span>
          <button
            type="button"
            data-testid="dismiss-resync-banner-btn"
            onClick={() => setResyncNotice(null)}
            className="rounded border border-amber-600/40 bg-amber-500/20 px-2 py-0.5 text-[11px] text-text-primary hover:bg-amber-500/30"
          >
            Dismiss
          </button>
        </div>
      )}
      {showDiagnostics && (
        <LiveSourceProber initialQuery={query} />
      )}
      {showAuditInspector && (
        <BeforeAfterAuditInspector compact />
      )}
      {running && (
        <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-3 space-y-3" aria-live="polite">
          <PipelineTimeline stages={RESEARCH_STAGES} statuses={deriveStages('active')} />
          {(waves > 1 || lane === 'deep' || (liveWave != null && liveWave > 0)) && (
            <MultiWaveProgressTimeline
              currentWave={liveWave ?? detail?.wave_count ?? 1}
              totalWaves={liveTotalWaves ?? (waves > 1 ? waves : 3)}
              stabilityScore={liveStability ?? undefined}
            />
          )}
          <div className="mt-2 text-[11px] text-text-muted">
            Running in the background{activeSessionId != null ? ` (session ${activeSessionId})` : ''} — the answer opens automatically when it completes.
          </div>
        </div>
      )}

      <BentoGrid preset="hero" className="items-start">
        <BentoCard
          as="section"
          className={detail ? 'gap-2 @3xl/surface:col-span-4' : 'col-span-full gap-2'}
        >
          <div className="flex items-center justify-between gap-2">
            <span className="font-display text-[12px] uppercase tracking-wide text-text-muted">Recent sessions</span>
            <button type="button" onClick={loadHistory} className="text-[11px] text-text-muted hover:text-text-secondary">Refresh</button>
          </div>
          <BentoScrollRegion ariaLabel="Recent research sessions" className="max-h-96 pr-1">
            {sessions.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border-subtle py-6 text-center text-[11px] text-text-muted">
                No research sessions yet — ask a question above to run one.
              </div>
            ) : (
              <ul className="space-y-1">
                {sessions.map((s) => (
                  <li key={s.id} className="flex items-center gap-1.5 min-w-0">
                    <button
                      type="button"
                      onClick={() => openDetail(s.id)}
                      className="flex min-w-0 flex-1 items-center justify-between rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-2 text-left hover:bg-overlay-subtle transition-colors"
                    >
                      <span className="truncate text-[12px] text-text-secondary">{s.query_text}</span>
                      <span className="ml-3 shrink-0 font-mono text-[10px] text-text-muted">{s.status}</span>
                    </button>
                    <button
                      type="button"
                      data-testid={`rerun-session-${s.id}`}
                      title="Re-run query"
                      aria-label={`Re-run query "${s.query_text}"`}
                      onClick={() => {
                        setQuery(s.query_text);
                        const inputEl = document.querySelector<HTMLInputElement>('input[aria-label="Research question"]');
                        inputEl?.focus();
                      }}
                      className="inline-flex items-center justify-center rounded-lg border border-border-subtle bg-overlay-subtle p-2 text-text-muted hover:text-brass hover:border-brass/40 transition-colors text-xs leading-none shrink-0"
                    >
                      <Icon.rotateCcw className="size-3.5 shrink-0" aria-hidden="true" />
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </BentoScrollRegion>
        </BentoCard>

      {detail && (() => {
        const parsedArtifact = safeParseJson<any>(detail.artifact_json);
        const parsedResult = parsedArtifact?.result;
        const parsedMeta = parsedResult?.research_metadata;
        const effectiveWaveCount = detail.wave_count ?? parsedMeta?.wave_count;
        const effectiveWaveStability = detail.wave_stability ?? parsedMeta?.wave_stability;
        const effectiveQualityScore = detail.quality_score ?? parsedMeta?.quality_score;
        const effectiveSynthesisModel = detail.synthesis_model ?? parsedMeta?.synthesis_model;
        const shoppingDecision: ShoppingDecisionPayload | undefined =
          detail.shopping_decision ?? parsedResult?.shopping_decision;
        const codegenVerification: CodeGenVerificationPayload | undefined =
          detail.codegen_verification ?? parsedResult?.codegen_verification;
        const epistemicDagDto: EpistemicDagDto | undefined =
          detail.epistemic_dag ?? parsedResult?.epistemic_dag;

        return (
          <BentoCard as="section" className="gap-2 @3xl/surface:col-span-8">
            <div className="mb-2 flex items-center justify-between flex-wrap gap-2">
              <div className="flex items-center gap-2 flex-wrap">
                <span className="text-[12px] font-medium text-text-primary">Session {detail.session.id}</span>
                {(parsedMeta?.subquery_count != null || effectiveWaveCount != null || effectiveWaveStability != null) && (
                  <span data-testid="session-metadata-stats" className="text-[11px] font-mono text-text-muted">
                    {[
                      parsedMeta?.subquery_count != null ? `${parsedMeta.subquery_count} subqueries` : null,
                      effectiveWaveCount != null ? `${effectiveWaveCount} waves` : null,
                      effectiveWaveStability != null ? `stability ${Math.round(effectiveWaveStability * 100)}%` : null,
                    ].filter(Boolean).join(' · ')}
                  </span>
                )}
                {(effectiveSynthesisModel != null || effectiveQualityScore != null || effectiveWaveCount != null) && (
                  <div
                    data-testid="research-telemetry-ribbon"
                    className="inline-flex items-center gap-1.5 rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 text-[10px] font-mono text-text-secondary"
                  >
                    {effectiveSynthesisModel && (
                      <span className="inline-flex items-center gap-1 text-brass">
                        <Icon.cpu className="size-3" aria-hidden="true" />
                        <span>{effectiveSynthesisModel}</span>
                      </span>
                    )}
                    {effectiveQualityScore != null && (
                      <span className="inline-flex items-center gap-1 text-(--color-status-pass)">
                        <Icon.star className="size-3 shrink-0" aria-hidden="true" />
                        <span>{effectiveQualityScore}/5</span>
                      </span>
                    )}
                    {effectiveWaveCount != null && (
                      <span>{effectiveWaveCount}W</span>
                    )}
                  </div>
                )}
              </div>
              <div className="flex items-center gap-2 flex-wrap">
                <button
                  type="button"
                  data-testid="toggle-judge-btn"
                  onClick={() => setShowJudgeInspector(prev => !prev)}
                  className={`rounded border px-2.5 py-1 text-[11px] transition-colors ${
                    showJudgeInspector
                      ? 'border-brass bg-brass/20 text-brass'
                      : 'border-border-subtle bg-bg-elevated text-text-secondary hover:text-text-primary'
                  }`}
                >
                  Judge Inspector
                </button>
                <button
                  type="button"
                  onClick={() => setIsReplModalOpen(true)}
                  className="rounded border border-border-subtle bg-bg-elevated px-2.5 py-1 text-[11px] text-text-secondary hover:text-text-primary"
                >
                  Sandbox REPL
                </button>
                <button
                  type="button"
                  data-testid="save-to-kb-button"
                  disabled={isSavingToKb}
                  onClick={async () => {
                    if (!detail?.session?.id) return;
                    setIsSavingToKb(true);
                    try {
                      const count = await invokeCmd<number>('save_research_session_to_kb', { sessionId: detail.session.id });
                      pushToast?.({
                        tone: 'ok',
                        title: 'Filed for review',
                        body: `${count} source(s) filed for review; not yet searchable.`,
                        cause: 'backend-ok',
                      });
                    } catch (err) {
                      pushToast?.({
                        tone: 'warn',
                        title: 'Save to KB failed',
                        body: sanitizeErrorForToast(err),
                        cause: 'backend-error',
                      });
                    } finally {
                      setIsSavingToKb(false);
                    }
                  }}
                  className="rounded border border-emerald-500/40 bg-emerald-500/10 px-2.5 py-1 text-[11px] text-emerald-300 hover:bg-emerald-500/20 disabled:opacity-50 transition-colors"
                >
                  {isSavingToKb ? 'Saving…' : 'Save to KB'}
                </button>
                <button
                  type="button"
                  data-testid="promote-to-scientia-btn"
                  disabled={isPromotingScientia}
                  onClick={async () => {
                    if (!detail?.session?.id) return;
                    setIsPromotingScientia(true);
                    try {
                      const promoted = await promoteCandidate(`finding-${detail.session.id}`);
                      pushToast?.({
                        tone: 'ok',
                        title: 'Promoted to Scientia',
                        body: `Seeded publication ${promoted.publication_id} (${promoted.claims_seeded} claims).`,
                        cause: 'backend-ok',
                      });
                    } catch (err) {
                      pushToast?.({
                        tone: 'warn',
                        title: 'Scientia promotion failed',
                        body: sanitizeErrorForToast(err),
                        cause: 'backend-error',
                      });
                    } finally {
                      setIsPromotingScientia(false);
                    }
                  }}
                  className="rounded border border-purple-500/40 bg-purple-500/10 px-2.5 py-1 text-[11px] text-purple-300 hover:bg-purple-500/20 disabled:opacity-50 transition-colors"
                >
                  {isPromotingScientia ? 'Promoting…' : 'Promote to Scientia Publication'}
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
            {(effectiveWaveCount != null && effectiveWaveCount > 1) && (
              <div className="mt-2" data-testid="detail-multi-wave-timeline">
                <MultiWaveProgressTimeline
                  currentWave={detail.session.status === 'completed' ? effectiveWaveCount + 1 : (liveWave ?? effectiveWaveCount)}
                  totalWaves={Math.min(5, Math.max(effectiveWaveCount, 3))}
                  stabilityScore={effectiveWaveStability}
                  earlyExitTriggered={Boolean(effectiveWaveStability != null && effectiveWaveStability >= 0.85 && effectiveWaveCount < Math.max(effectiveWaveCount, 3))}
                />
              </div>
            )}
            {parsedMeta?.planner_degraded && (
              <div
                data-testid="planner-degraded-chip"
                role="status"
                className="mt-2 flex items-center gap-2 rounded-lg border border-amber-600/35 bg-amber-500/10 px-3 py-1.5 text-xs text-(--color-status-warn)"
              >
                <span className="inline-flex items-center gap-1 font-semibold">
                  <Icon.alert className="size-3.5" aria-hidden="true" />
                  <span>Planner degraded</span>
                </span>
                <span className="text-text-secondary text-[11px]">
                  Subquery quality may be reduced due to degraded planner mode.
                </span>
              </div>
            )}
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

            const dagNodes: DagNode[] = epistemicDagDto?.nodes?.length
              ? epistemicDagDto.nodes.map((n) => ({
                  id: n.id,
                  label: n.label,
                  wave: n.wave,
                  kind: n.kind,
                  status:
                    n.status ??
                    (n.verdict?.toLowerCase() === 'refuted' || n.verdict?.toLowerCase() === 'contradicted'
                      ? 'contradicted'
                      : n.verdict
                      ? 'verified'
                      : 'pending'),
                  confidence: n.confidence,
                  excerpt: n.excerpt ?? n.snippet,
                  url: n.url,
                }))
              : claimRows.map((c, i) => {
                  const isContradicted =
                    c.verdict.toLowerCase() === 'refuted' || c.verdict.toLowerCase() === 'contradicted';
                  const isVerified =
                    c.verdict.toLowerCase() === 'verified' || c.verdict.toLowerCase() === 'supported';
                  return {
                    id: c.claimId,
                    label: `${c.claimId}: ${c.text}`,
                    wave:
                      claimRows.length <= 10
                        ? Math.floor(i / 2)
                        : Math.min(4, Math.floor((i * 5) / claimRows.length)),
                    kind: 'claim',
                    status: isContradicted ? 'contradicted' : isVerified ? 'verified' : 'pending',
                    confidence: c.confidence,
                  };
                });
            const dagEdges: DagEdge[] = epistemicDagDto?.edges?.length
              ? epistemicDagDto.edges.map((e) => ({
                  srcId: e.src_id ?? e.source ?? '',
                  dstId: e.dst_id ?? e.target ?? '',
                  relation: e.relation ?? (e.kind === 'contradicts' ? 'contradicts' : 'supports'),
                }))
              : dagNodes.slice(1).map((node, i) => ({
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
                {shoppingDecision && (
                  <div className="mt-2">
                    <ShoppingDecisionCard decision={shoppingDecision} />
                  </div>
                )}
                {codegenVerification && (
                  <div className="mt-2">
                    <CodeGenVerificationCard verification={codegenVerification} />
                  </div>
                )}
                <BentoScrollRegion ariaLabel="Research synthesis report" className="mt-2 max-h-[360px] pr-1">
                  <ResearchReportMarkdown
                    markdown={detail.report_markdown ?? detail.artifact_json ?? '(no artifact persisted)'}
                    onCitationClick={handleCitationClick}
                  />
                </BentoScrollRegion>
                {claimRows.length > 0 && (
                  <div className="mt-2">
                    <VirtualizedClaimsTable
                      claims={claimRows}
                      sourceCount={detail.source_count ?? 0}
                      totalClaims={detail.claims_total ?? claimRows.length}
                      highlightedClaimId={highlightedClaimId ?? undefined}
                      hasMore={Boolean(detail.claims_total != null && claimRows.length < detail.claims_total)}
                      loadingMore={loadingMoreClaims}
                      onLoadMore={() => { void loadMoreClaims(); }}
                    />
                  </div>
                )}
                {framePage ? (
                  <div className="mt-3">
                    <VirtualizedFrameChunkTable
                      page={framePage}
                      loadingMore={loadingFrameChunk}
                      onLoadNextChunk={(nextOffset, nextLimit) => {
                        const sid = framePage.stream_id ?? detail.frame_stream_id;
                        if (sid) void loadFrameChunk(sid, nextOffset, nextLimit);
                      }}
                    />
                  </div>
                ) : detail.frame_stream_id ? (
                  <div className="mt-3 flex items-center justify-between rounded-xl border border-border-subtle bg-overlay-subtle px-3 py-2 text-xs">
                    <span className="font-mono text-[11px] text-text-secondary">
                      DataFrame stream attached ({detail.frame_stream_id})
                    </span>
                    <button
                      type="button"
                      data-testid="load-frame-stream-btn"
                      disabled={loadingFrameChunk}
                      onClick={() => { void loadFrameChunk(detail.frame_stream_id!, 0, 100); }}
                      className="rounded border border-brass/40 bg-brass/10 px-2.5 py-1 font-mono text-[11px] text-brass hover:bg-brass/20 disabled:opacity-50"
                    >
                      {loadingFrameChunk ? 'Streaming…' : 'Load Frame Preview'}
                    </button>
                  </div>
                ) : null}
                {allCitations.length > 0 && (
                  <div className="mt-3 rounded-lg border border-border-subtle bg-black/20 p-3">
                    <div className="mb-2 text-[11px] font-mono uppercase tracking-wider text-text-muted flex flex-wrap items-center justify-between gap-2">
                      <span>Citations & Sources</span>
                      {parsedMeta?.retrieval_diagnostics?.dropped_source_count != null &&
                        parsedMeta.retrieval_diagnostics.dropped_source_count > 0 && (
                          <span
                            data-testid="dropped-source-count"
                            className="text-amber-300/90 font-normal lowercase tracking-normal"
                          >
                            {parsedMeta.retrieval_diagnostics.dropped_source_count} sources filtered by novelty
                          </span>
                        )}
                    </div>
                    <BentoScrollRegion ariaLabel="Research citations and sources" className="max-h-60 pr-1">
                      <ul className="space-y-2" role="list">
                        {allCitations.map((url) => (
                          <li key={url} role="listitem" className="flex items-center justify-between gap-2 min-w-0">
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
                    </BentoScrollRegion>
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
            onPublished={(path, status) => {
              pushToast?.({
                tone: 'ok',
                title: status === 'duplicate' ? 'Already pending review' : 'Filed for review',
                body: `Proposed ${path} is pending review and not yet searchable.`,
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
          </BentoCard>
        );
      })()}
      </BentoGrid>
      <ResearchEngineDrawer
        isOpen={isEngineDrawerOpen}
        onClose={() => setIsEngineDrawerOpen(false)}
        onConfigSaved={loadEngineStatus}
        pushToast={pushToast}
      />
    </section>
  );
}
