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
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { RomanRotaryKnob } from '../../ui/RomanRotaryKnob';
import { BentoCard, BentoScrollRegion } from '../../ui/BentoGrid';
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
          <div className="mt-3 p-2.5 rounded border border-(--color-status-fail)/35 bg-bg-elevated text-xs text-(--color-status-fail)">
            {error}
          </div>
        )}

        <div className="mt-4 space-y-4">
          {culpritUrl && (
            <div className="text-xs font-mono text-text-muted truncate">
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
              className="w-full border-border-subtle bg-bg-elevated text-xs text-text-primary"
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
              className="w-full rounded border border-border-subtle bg-bg-elevated px-3 py-2 text-xs text-text-primary outline-none focus:border-brass/50 resize-none leading-relaxed font-mono"
            />
          </div>
        </div>

        <div className="mt-6 flex items-center justify-end gap-2 pt-3 border-t border-border-subtle">
          <button
            type="button"
            data-slot="button"
            onClick={onClose}
            disabled={submitting}
            className="h-8 rounded border border-border-subtle bg-bg-elevated px-3 text-xs text-text-muted hover:text-text-secondary transition-colors disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="button"
            data-slot="button"
            onClick={handleSubmit}
            disabled={submitting}
            className="h-8 rounded border border-brass/50 bg-brass/20 hover:bg-brass/30 text-brass px-4 text-xs font-semibold transition-colors disabled:opacity-50"
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
  const [query, setQuery] = useState(() => {
    try {
      const seed = window.localStorage.getItem('vox_research_seed_query');
      if (seed) {
        window.localStorage.removeItem('vox_research_seed_query');
        return seed;
      }
    } catch {
      /* localStorage unavailable */
    }
    return '';
  });
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
  const [detailWorkspaceTab, setDetailWorkspaceTab] = useState<'overview' | 'synthesis' | 'dag' | 'diagnostics'>('overview');
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
  const [minConfidenceGate, setMinConfidenceGate] = useState<number>(75);
  const [steerPrompt, setSteerPrompt] = useState<string>('');
  const [sessionSearchQuery, setSessionSearchQuery] = useState<string>('');
  const [researchStorageScope, setResearchStorageScope] = useState<'repo_local' | 'global_library'>('repo_local');
  const [isDirectIndexingKb, setIsDirectIndexingKb] = useState<boolean>(false);
  const [kbPreflightHits, setKbPreflightHits] = useState<
    Array<{ source?: string; path?: string | null; snippet?: string; score?: number }>
  >([]);
  const [kbPreflightLoading, setKbPreflightLoading] = useState<boolean>(false);
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
    const id = setInterval(() => {
      if (document.visibilityState !== 'hidden') {
        void loadHistory();
      }
    }, 10_000);
    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        void loadHistory();
      }
    };
    document.addEventListener('visibilitychange', onVisible);
    let unlisten: (() => void) | undefined;
    listenScientiaQueue(() => { void loadHistory(); })
      .then((fn) => { unlisten = fn; })
      .catch(() => { /* not in Tauri — interval fallback covers it */ });
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', onVisible);
      unlisten?.();
    };
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

  // Debounced Federated KB Prior-Art Preflight: checks existing docs/syntheses
  // across vox_baseline, global_library, and repo_local before web search runs.
  useEffect(() => {
    if (embedded) return;
    const trimmed = query.trim();
    if (trimmed.length < 4) {
      setKbPreflightHits([]);
      setKbPreflightLoading(false);
      return;
    }
    let cancelled = false;
    setKbPreflightLoading(true);
    const timer = window.setTimeout(() => {
      invokeCmd<{
        hits?: Array<{ source?: string; path?: string | null; snippet?: string; score?: number }>;
      } | null>('vox_search_query', {
        query: trimmed,
        scope: ['memory', 'knowledge', 'chunk'],
        limit: 3,
      })
        .then((res) => {
          if (cancelled) return;
          const rows = Array.isArray(res?.hits) ? res.hits.slice(0, 3) : [];
          setKbPreflightHits(rows);
        })
        .catch(() => {
          if (!cancelled) setKbPreflightHits([]);
        })
        .finally(() => {
          if (!cancelled) setKbPreflightLoading(false);
        });
    }, 350);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query, embedded]);

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

  const run = async (
    overrideLane?: 'fast' | 'deep',
    overrideScope?: 'web' | 'local' | 'both',
    overrideWaves?: 1 | 2 | 3 | 4 | 5,
    overrideQuery?: string,
  ) => {
    const activeQuery = (overrideQuery ?? (query.trim() || detail?.session.query_text))?.trim();
    if (!activeQuery) return;
    if ((!query.trim() || overrideQuery) && activeQuery) {
      setQuery(activeQuery);
    }
    const activeLane = overrideLane ?? lane;
    const activeScope = overrideScope ?? scope;
    const activeWaves = overrideWaves ?? waves;
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
        scope: activeScope,
        waves: activeWaves,
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
    <section className="@container/surface flex min-w-0 flex-col gap-3">
      <span className="sr-only">{useLabel('research')}</span>
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE · RESEARCH"
        title="Research Syntheses & Epistemic Investigation"
        subtitle="Autonomous web research, epistemic DAG synthesis, and cross-source verification."
        waxSealBadge={lowEvidence ? 'LOW EVIDENCE' : 'VERIFICATA'}
        icon={<Icon.search className="size-5 text-text-primary" />}
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
            valueText: `${activeProviderCount}/${totalProviderCount || activeProviderCount}`,
            ratio: totalProviderCount > 0 ? Math.max(0, Math.min(1, activeProviderCount / totalProviderCount)) : 0,
            tone: activeProviderCount > 0 ? 'emerald' : 'gold',
            purpose: 'Active research providers and quota status',
            actionHint: 'Configure research providers and API keys',
            active: isEngineDrawerOpen,
            onClick: openEngineDrawer,
          },
          {
            label: 'CONFIDENCE',
            valueText: detail?.citation_precision != null
              ? `${Math.round(detail.citation_precision * 100)}%`
              : `${Math.round((detail?.wave_stability ?? 0.85) * 100)}%`,
            ratio: detail?.citation_precision ?? detail?.wave_stability ?? 0.85,
            tone: 'emerald',
            purpose: 'Epistemic citation precision and multi-wave corroboration stability',
            actionHint: 'Toggle Fast / Deep Research lane',
            active: lane === 'deep',
            onClick: () => setLane((prev) => (prev === 'fast' ? 'deep' : 'fast')),
          },
        ]}
      />

      {/* Unified Sculpted Cast-Brass Command Console Bar (Concept Art Parity) */}
      <div className="vox-recessed-console-frame p-3 space-y-2.5">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            data-slot="button"
            data-roman-plaque="true"
            onClick={() => run()}
            disabled={running}
            className="vox-brass-plaque-btn inline-flex h-9 items-center gap-1.5 rounded-md px-4 text-xs disabled:opacity-50 transition-all"
          >
            <Icon.play className="size-3.5 shrink-0" aria-hidden="true" />
            <span>{running ? 'Running…' : 'Run'}</span>
          </button>

          <div
            role="tablist"
            aria-label="Research Lane"
            className="vox-segmented-brass-bar shrink-0 text-xs"
          >
            <button
              type="button"
              role="tab"
              data-slot="tab"
              data-testid="lane-switch-fast"
              aria-selected={lane === 'fast'}
              title="Single-wave synthesis (~5-15s) for rapid lookup and single-fact verification"
              onClick={() => setLane('fast')}
              className="inline-flex h-9 items-center gap-1.5 px-3 text-xs font-medium transition-colors"
            >
              <Icon.bolt className="size-3.5 shrink-0" aria-hidden="true" />
              <span>Fast</span>
            </button>
            <button
              type="button"
              role="tab"
              data-slot="tab"
              data-testid="lane-switch-deep"
              aria-selected={lane === 'deep'}
              title="Multi-wave adversarial query decomposition, atomic claim extraction, and contradiction audit"
              onClick={() => setLane('deep')}
              className="inline-flex h-9 items-center gap-1.5 px-3 text-xs font-medium transition-colors"
            >
              <Icon.search className="size-3.5 shrink-0" aria-hidden="true" />
              <span>Deep Research</span>
            </button>
          </div>

          <input
            value={query}
            onChange={e => setQuery(e.target.value)}
            placeholder="Ask a research question…"
            aria-label="Research question"
            onKeyDown={e => { if (e.key === 'Enter') void run(); }}
            className="h-9 min-w-[8rem] flex-1 rounded-md border border-border-strong bg-bg-surface px-2.5 font-serif text-sm text-text-primary outline-hidden focus:border-brass"
          />

          <div
            role="tablist"
            aria-label="Research Workspace Views"
            className="vox-segmented-brass-bar shrink-0 text-xs"
          >
            {([
              { id: 'overview', label: 'Overview' },
              { id: 'synthesis', label: 'Synthesis' },
              { id: 'dag', label: 'DAG & Claims' },
              { id: 'diagnostics', label: 'Diagnostics' },
            ] as const).map((ws) => (
              <button
                key={ws.id}
                type="button"
                role="tab"
                data-slot="tab"
                data-testid={`research-workspace-tab-${ws.id}`}
                aria-selected={detailWorkspaceTab === ws.id}
                onClick={() => setDetailWorkspaceTab(ws.id)}
                className="inline-flex h-9 items-center px-2.5 text-xs font-medium transition-colors"
              >
                {ws.label}
              </button>
            ))}
          </div>

          <button
            type="button"
            data-slot="button"
            data-roman-plaque="true"
            data-testid="configure-engines-btn"
            aria-label="Configure search sources and API keys"
            onClick={openEngineDrawer}
            className="vox-brass-plaque-btn inline-flex h-9 items-center gap-1 rounded-md px-2.5 text-xs transition-colors"
          >
            <Icon.settings className="size-3.5 shrink-0" aria-hidden="true" />
            <span>Sources</span>
          </button>
        </div>

        {/* 4 Statistical Research Archetype Presets (compressed inline with KB Preflight status pill so reports sit above the fold) */}
        <div
          role="group"
          aria-label="Statistical Research Archetypes"
          data-testid="research-archetype-launchpad"
          className="vox-recessed-tablet flex flex-wrap items-center justify-between gap-2 px-3 py-1.5"
        >
          <div className="flex flex-wrap items-center gap-1.5">
            <span
              className="font-display text-[11px] font-bold uppercase tracking-wider text-text-secondary mr-1"
              title="One-click research presets derived from real user investigation distributions"
            >
              Archetypes
            </span>
            {([
              {
                id: 'prior-art',
                label: 'Prior-Art & KB Survey (35%)',
                tip: 'Deep scholarly survey fusing local KB + web prior art (2 waves)',
                lane: 'deep' as const,
                mode: 'scholarly' as const,
                scope: 'both' as const,
                waves: 2 as const,
                seed: 'Prior-art survey and epistemic gap analysis for ',
              },
              {
                id: 'arch-audit',
                label: 'Architecture Audit (25%)',
                tip: 'Multi-crate code and architectural tradeoff verification (3 waves)',
                lane: 'deep' as const,
                mode: 'codegen' as const,
                scope: 'both' as const,
                waves: 3 as const,
                seed: 'Architectural tradeoff and implementation verification for ',
              },
              {
                id: 'claim-check',
                label: 'Claim & Contradiction Check (25%)',
                tip: 'Rapid web corroboration and source contradiction triage (2 waves)',
                lane: 'fast' as const,
                mode: 'general' as const,
                scope: 'web' as const,
                waves: 2 as const,
                seed: 'Empirical claim corroboration and contradiction check: ',
              },
              {
                id: 'empirical-arxiv',
                label: 'Empirical Preprint (15%)',
                tip: 'High-depth empirical synthesis ready for 95% CI Scientia & arXiv publication (4 waves)',
                lane: 'deep' as const,
                mode: 'scholarly' as const,
                scope: 'both' as const,
                waves: 4 as const,
                seed: 'Quantitative empirical evaluation and 95% CI synthesis: ',
              },
            ]).map((preset) => (
              <button
                key={preset.id}
                type="button"
                data-slot="button"
                data-roman-plaque="true"
                data-testid={`archetype-${preset.id}`}
                title={preset.tip}
                onClick={() => {
                  setLane(preset.lane);
                  setDomainMode(preset.mode);
                  setScope(preset.scope);
                  setWaves(preset.waves);
                  if (!query.trim()) {
                    setQuery(preset.seed);
                  }
                }}
                className="vox-brass-plaque-btn inline-flex h-8 items-center rounded-md px-2.5 text-[11px] transition-colors"
              >
                <span>{preset.label}</span>
              </button>
            ))}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              data-slot="button"
              data-roman-plaque="true"
              data-testid="research-domain-mode-pill"
              title="Switch domain extractor mode: General synthesis, CodeGen AST/docs verification, Scholarly 95% CI, or Product/Shopping spec comparison"
              onClick={() => {
                const order: Array<'general' | 'codegen' | 'scholarly' | 'shopping'> = [
                  'general',
                  'codegen',
                  'scholarly',
                  'shopping',
                ];
                setDomainMode((prev) => order[(order.indexOf(prev) + 1) % order.length]!);
              }}
              className="vox-brass-plaque-btn inline-flex h-8 items-center gap-1 rounded px-2.5 text-[11px] transition-colors"
            >
              <span>
                Mode:{' '}
                {domainMode === 'codegen'
                  ? 'CodeGen'
                  : domainMode === 'scholarly'
                    ? 'Scholarly'
                    : domainMode === 'shopping'
                      ? 'Shopping'
                      : 'General'}
              </span>
            </button>
            {kbPreflightHits.length === 0 && (
              <span
                data-testid="research-kb-preflight-panel"
                className="font-mono text-[11px] text-text-muted"
                title='Scope "both" checks vox_baseline, global_library, and repo_local before web expansion'
              >
                {query.trim().length >= 4 && kbPreflightLoading ? 'Checking KB…' : 'KB Preflight · both-scope'}
              </span>
            )}
          </div>
        </div>

        {/* Expanded Federated KB Prior-Art Preflight Bar (shown only when matching baseline/published nodes exist) */}
        {kbPreflightHits.length > 0 && (
          <div
            data-testid="research-kb-preflight-panel"
            className="vox-recessed-tablet flex flex-wrap items-center justify-between gap-2 px-3 py-1.5 text-xs"
          >
            <div className="flex flex-wrap items-center gap-1.5 min-w-0">
              <span className="font-display text-[11px] font-bold uppercase tracking-wider text-brass shrink-0">
                KB Prior-Art Preflight ({kbPreflightHits.length})
              </span>
              {kbPreflightHits.slice(0, 3).map((h, idx) => {
                const targetPath = (h.path ?? h.source ?? '').trim();
                const label = targetPath || 'kb-node';
                return targetPath ? (
                  <button
                    key={`${targetPath}-${idx}`}
                    type="button"
                    data-slot="button"
                    title={`Open ${targetPath} in non-modal Doc Dock`}
                    onClick={() =>
                      window.dispatchEvent(
                        new CustomEvent('vox://open-doc', { detail: { path: targetPath } }),
                      )
                    }
                    className="inline-flex h-7 max-w-[16rem] items-center truncate rounded border border-border-subtle bg-bg-surface px-2 font-mono text-[11px] text-brass hover:border-brass transition-colors"
                  >
                    <span className="truncate">{label}</span>
                  </button>
                ) : (
                  <span key={`kb-${idx}`} className="truncate font-serif text-xs text-text-secondary">
                    {label}
                  </span>
                );
              })}
            </div>
            <button
              type="button"
              data-slot="button"
              data-roman-plaque="true"
              data-testid="run-kb-gap-synthesis-btn"
              title="Run Deep Research across both Federated KB and live Web to fill evidence gaps beyond existing baseline notes"
              onClick={() => {
                const targetWaves = waves < 2 ? (2 as const) : waves;
                setScope('both');
                setLane('deep');
                setWaves(targetWaves);
                void run('deep', 'both', targetWaves);
              }}
              className="vox-brass-plaque-btn inline-flex h-8 items-center rounded-md px-2.5 text-[11px] transition-colors"
            >
              <span>Synthesize KB + Web Gaps</span>
            </button>
          </div>
        )}

        {/* Progressive-Disclosure Secondary Ledger for Providers & Retrieval Parameters */}
        <details className="vox-recessed-tablet px-3 py-2 text-xs">
          <summary className="vox-brass-drawer-summary flex cursor-pointer items-center justify-between gap-2 select-none font-display text-xs font-semibold uppercase tracking-wider text-text-secondary">
            <span>
              Retrieval Parameters &amp; Providers · Mode: {domainMode} · Scope: {scope} · {waves}W · {activeProviderCount} active providers
            </span>
            <Icon.chevR className="size-3.5 shrink-0 text-text-muted rotate-90" aria-hidden="true" />
          </summary>
          <div className="mt-2.5 space-y-2 border-t border-border-subtle pt-2.5">
            {engineStatus && (
              <div data-testid="provider-badge-strip" className="flex flex-wrap items-center gap-2 text-xs">
                {engineStatus.providers.map((p) => {
                  if (p.is_keyless) {
                    return (
                      <span
                        key={p.id}
                        className="inline-flex h-8 items-center gap-1.5 rounded border border-(--color-status-pass)/35 bg-bg-elevated px-2.5 text-xs font-medium text-(--color-status-pass)"
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
                        className="inline-flex h-8 items-center gap-1.5 rounded border border-brass/40 bg-bg-elevated px-2.5 text-xs font-mono text-brass"
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
                        className="inline-flex h-8 items-center gap-1.5 rounded border border-brass/35 bg-bg-elevated px-2.5 text-xs font-medium text-brass"
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
                      data-slot="button"
                      onClick={openEngineDrawer}
                      className="inline-flex h-8 items-center gap-1.5 rounded border border-border-subtle bg-bg-elevated px-2.5 text-xs font-medium text-text-secondary hover:text-text-primary hover:border-brass/45 transition-colors"
                    >
                      {label}
                    </button>
                  );
                })}
              </div>
            )}

            <div
              data-testid="research-power-control-bar"
              className="flex flex-wrap items-center gap-3 rounded-lg border border-border-subtle bg-bg-elevated px-3 py-2 text-xs"
            >
              <div
                data-testid="research-domain-mode-group"
                role="group"
                aria-label="Domain Mode"
                className="flex flex-wrap items-center gap-1"
              >
                <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-muted mr-1">Mode</span>
                {(['general', 'shopping', 'codegen', 'scholarly'] as const).map((mode) => {
                  const modeTips: Record<typeof mode, string> = {
                    general: 'Broad multi-source web + KB factual investigation',
                    shopping: 'Structured product comparison with citation verification',
                    codegen: 'Repository, crate architecture, APIs, and code verification',
                    scholarly: 'Scientific literature review, 95% CI methodology, and arXiv rigor',
                  };
                  return (
                    <button
                      key={mode}
                      type="button"
                      data-slot="chip"
                      data-testid={`domain-mode-${mode}`}
                      aria-pressed={domainMode === mode}
                      title={modeTips[mode]}
                      onClick={() => setDomainMode(mode)}
                      className={`h-8 rounded border px-2.5 text-xs font-medium capitalize transition-colors ${
                        domainMode === mode
                          ? 'border-brass/50 bg-brass/20 text-brass font-semibold'
                          : 'border-transparent text-text-secondary hover:bg-bg-surface hover:text-text-primary'
                      }`}
                    >
                      {mode === 'codegen' ? 'CodeGen' : mode}
                    </button>
                  );
                })}
              </div>

              <div
                data-testid="research-scope-group"
                role="group"
                aria-label="Retrieval Scope"
                className="flex flex-wrap items-center gap-1"
              >
                <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-muted mr-1">Scope</span>
                {(['web', 'local', 'both'] as const).map((s) => {
                  const scopeTips: Record<typeof s, string> = {
                    web: 'Live external web and academic providers only',
                    local: 'Federated local KB only (vox_baseline, global_library, repo_local)',
                    both: 'Cross-corroborate local Federated KB with live web evidence',
                  };
                  return (
                    <button
                      key={s}
                      type="button"
                      data-slot="chip"
                      data-testid={`research-scope-${s}`}
                      aria-pressed={scope === s}
                      title={scopeTips[s]}
                      onClick={() => setScope(s)}
                      className={`h-8 rounded border px-2.5 text-xs font-medium capitalize transition-colors ${
                        scope === s
                          ? 'border-brass/50 bg-brass/20 text-brass font-semibold'
                          : 'border-transparent text-text-secondary hover:bg-bg-surface hover:text-text-primary'
                      }`}
                    >
                      {s}
                    </button>
                  );
                })}
              </div>

              <div
                data-testid="research-waves-group"
                role="group"
                aria-label="Adversarial Waves"
                className="flex flex-wrap items-center gap-1"
              >
                <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-muted mr-1">Waves</span>
                {([1, 2, 3, 4, 5] as const).map((w) => (
                  <button
                    key={w}
                    type="button"
                    data-slot="chip"
                    data-testid={`research-waves-${w}`}
                    aria-pressed={waves === w}
                    title={`Execute up to ${w} adversarial corroboration wave(s); exits early when stability >= Confidence Gate`}
                    onClick={() => setWaves(w)}
                    className={`h-8 rounded border px-2.5 text-xs font-mono transition-colors ${
                      waves === w
                        ? 'border-brass/50 bg-brass/20 text-brass font-semibold'
                        : 'border-transparent text-text-secondary hover:bg-bg-surface hover:text-text-primary'
                    }`}
                  >
                    {w}W
                  </button>
                ))}
              </div>

              <div className="flex items-center gap-1.5 @2xl/surface:ml-auto">
                <label htmlFor="research-site-scope-input" className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-muted">
                  Site
                </label>
                <input
                  id="research-site-scope-input"
                  data-testid="research-site-scope-input"
                  type="text"
                  value={siteScope}
                  onChange={(e) => setSiteScope(e.target.value)}
                  placeholder="docs.rs, arxiv.org"
                  className="h-8 w-36 max-w-full @sm/surface:w-40 rounded border border-border-subtle bg-bg-surface px-2.5 text-xs font-mono text-text-primary outline-hidden focus:border-brass/50"
                />
              </div>
            </div>
          </div>
        </details>
      </div>

      {(() => {
        const tavilyUnconfigured = !engineStatus?.providers.find((p) => p.id === 'tavily')?.has_key;
        const searxngUnconfigured = !engineStatus?.providers.find((p) => p.id === 'searxng')?.is_enabled;
        const isDegraded = Boolean(engineStatus && tavilyUnconfigured && searxngUnconfigured);
        if (!isDegraded) return null;
        return (
          <div
            data-testid="degraded-setup-banner"
            className="rounded-lg border border-(--color-status-warn)/40 bg-bg-surface p-3 text-xs text-(--color-status-warn) flex flex-wrap items-center justify-between gap-3"
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
              data-slot="button"
              data-testid="degraded-setup-configure-btn"
              onClick={openEngineDrawer}
              className="h-8 rounded border border-(--color-status-warn)/45 bg-bg-elevated px-3 text-xs font-medium text-text-primary hover:border-brass/50 transition-colors shrink-0"
            >
              Configure Keys
            </button>
          </div>
        );
      })()}

      {lowEvidence && (
        <div
          data-testid="empty-results-notice"
          className="rounded-lg border border-(--color-status-warn)/40 bg-bg-surface p-3 text-xs text-(--color-status-warn) space-y-2"
          role="alert"
        >
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-semibold flex items-center gap-1.5">
              <Icon.alert className="size-3.5" aria-hidden="true" />
              <span>Insufficient Evidence Grounding</span>
            </span>
            <button
              type="button"
              data-slot="button"
              data-testid="rerun-deep-lane-btn"
              onClick={() => {
                setLane('deep');
                void run('deep');
              }}
              className="h-8 rounded border border-(--color-status-warn)/45 bg-bg-elevated px-3 text-xs font-medium text-text-primary hover:border-brass/50 transition-colors"
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

      {resyncNotice && (
        <div
          data-testid="research-resync-banner"
          role="status"
          className="rounded-lg border border-(--color-status-warn)/40 bg-bg-surface px-3 py-2 text-xs text-(--color-status-warn) flex flex-wrap items-center justify-between gap-2"
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
            data-slot="button"
            data-testid="dismiss-resync-banner-btn"
            onClick={() => setResyncNotice(null)}
            className="h-8 rounded border border-(--color-status-warn)/45 bg-bg-elevated px-2.5 text-xs font-medium text-text-primary hover:border-brass/50"
          >
            Dismiss
          </button>
        </div>
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
          <div className="mt-2 text-xs text-text-muted">
            Running in the background{activeSessionId != null ? ` (session ${activeSessionId})` : ''} — the answer opens automatically when it completes.
          </div>
        </div>
      )}

      {(() => {
        const parsedArtifact = detail ? safeParseJson<any>(detail.artifact_json) : null;
        const parsedResult = parsedArtifact?.result;
        const parsedMeta = parsedResult?.research_metadata;
        const effectiveWaveCount = detail?.wave_count ?? parsedMeta?.wave_count;
        const effectiveWaveStability = detail?.wave_stability ?? parsedMeta?.wave_stability;
        const effectiveQualityScore = detail?.quality_score ?? parsedMeta?.quality_score;
        const effectiveSynthesisModel = detail?.synthesis_model ?? parsedMeta?.synthesis_model;
        const shoppingDecision: ShoppingDecisionPayload | undefined =
          detail?.shopping_decision ?? parsedResult?.shopping_decision;
        const codegenVerification: CodeGenVerificationPayload | undefined =
          detail?.codegen_verification ?? parsedResult?.codegen_verification;
        const epistemicDagDto: EpistemicDagDto | undefined =
          detail?.epistemic_dag ?? parsedResult?.epistemic_dag;

        const claimRows = toClaimRows(detail?.claims);
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
            ...(detail?.claims?.flatMap((c) => c.citation_urls) ?? []),
            ...(detail?.citations?.map((c) => c.url) ?? []),
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

        const streamPreviewPane = framePage ? (
          <VirtualizedFrameChunkTable
            page={framePage}
            loadingMore={loadingFrameChunk}
            onLoadNextChunk={(nextOffset, nextLimit) => {
              const sid = framePage.stream_id ?? detail?.frame_stream_id;
              if (sid) void loadFrameChunk(sid, nextOffset, nextLimit);
            }}
          />
        ) : detail?.frame_stream_id ? (
          <div className="flex items-center justify-between rounded-xl border border-border-subtle bg-bg-surface px-3 py-2 text-xs">
            <span className="font-mono text-xs text-text-secondary">
              DataFrame stream attached ({detail.frame_stream_id})
            </span>
            <button
              type="button"
              data-slot="button"
              data-testid="load-frame-stream-btn"
              disabled={loadingFrameChunk}
              onClick={() => { void loadFrameChunk(detail.frame_stream_id!, 0, 100); }}
              className="inline-flex h-8 items-center rounded border border-brass/45 bg-bg-elevated px-3 text-xs font-medium text-brass hover:bg-brass/15 disabled:opacity-50"
            >
              {loadingFrameChunk ? 'Streaming…' : 'Load Frame Preview'}
            </button>
          </div>
        ) : null;

        const stabilityPercent = Math.round(
          (liveStability ?? effectiveWaveStability ?? 0.85) * 100,
        );

        return (
          <div className="vox-recessed-console-frame p-3 @2xl/surface:p-4">
            <div className="grid grid-cols-1 @3xl/surface:grid-cols-12 gap-4 items-start">
              {/* LEFT RECESSED TABLET: SCIENTIA · EPISTEMIC SYNTHESIS & SESSIONS */}
              <BentoCard
                as="section"
                className="vox-recessed-tablet @3xl/surface:col-span-7 gap-3 p-4"
              >
                <div className="flex flex-col items-center gap-1.5 pb-1">
                  <div className="flex w-full items-center justify-between gap-2">
                    <span className="font-display text-xs font-bold uppercase tracking-[0.16em] text-text-primary">
                      Scientia · Epistemic Synthesis
                    </span>
                  </div>
                  <div className="vox-rule w-full" aria-hidden="true" />
                </div>

                {/* Recent Research Sessions Ledger (auto-collapses cleanly when session is opened so prose sits above the fold) */}
                <details
                  open={!detail}
                  className="group rounded-lg border border-border-strong bg-bg-surface px-2.5 py-2 space-y-2"
                >
                  <summary className="vox-brass-drawer-summary flex cursor-pointer items-center justify-between gap-2 select-none">
                    <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-secondary">
                      Recent sessions ({sessions.length}){detail ? ` · Active: ${detail.session.query_text}` : ''}
                    </span>
                    <Icon.chevR className="size-3.5 text-text-muted transition-transform rotate-90 group-open:-rotate-90 shrink-0" aria-hidden="true" />
                  </summary>
                  <BentoScrollRegion
                    ariaLabel="Recent research sessions"
                    className={detail ? 'max-h-32 pr-1' : 'max-h-80 pr-1'}
                  >
                    {sessions.length > 0 && (
                      <input
                        type="text"
                        data-testid="research-history-filter-input"
                        aria-label="Filter past research sessions"
                        value={sessionSearchQuery}
                        onChange={(e) => setSessionSearchQuery(e.target.value)}
                        placeholder="Search past sessions by query or status…"
                        className="mb-1.5 h-8 w-full rounded border border-border-subtle bg-bg-elevated px-2.5 font-sans text-xs text-text-primary placeholder:text-text-muted outline-hidden focus:border-brass"
                      />
                    )}
                    {(() => {
                      const needle = sessionSearchQuery.trim().toLowerCase();
                      const filteredSessions = needle
                        ? sessions.filter(
                            (s) =>
                              s.query_text.toLowerCase().includes(needle) ||
                              s.status.toLowerCase().includes(needle),
                          )
                        : sessions;
                      if (sessions.length === 0) {
                        return (
                          <div className="rounded-lg border border-dashed border-border-subtle py-5 text-center text-xs text-text-muted">
                            No research sessions yet — ask a question above to run one.
                          </div>
                        );
                      }
                      if (filteredSessions.length === 0) {
                        return (
                          <div className="rounded-lg border border-dashed border-border-subtle py-4 text-center text-xs text-text-muted">
                            No saved research sessions match &ldquo;{sessionSearchQuery.trim()}&rdquo;.
                          </div>
                        );
                      }
                      return (
                        <ul className="space-y-1.5" role="list">
                          {filteredSessions.map((s) => (
                            <li key={s.id} role="listitem" className="flex items-center gap-1.5 min-w-0">
                              <button
                                type="button"
                                data-slot="session-row"
                                data-testid={`research-session-${s.id}`}
                                onClick={() => openDetail(s.id)}
                                className={`flex min-w-0 flex-1 items-center justify-between rounded-md border px-3 py-1.5 text-left transition-colors ${
                                  detail?.session.id === s.id
                                    ? 'border-brass bg-bg-elevated font-semibold'
                                    : 'border-border-subtle bg-bg-surface hover:bg-bg-elevated'
                                }`}
                              >
                                <span className="truncate font-serif text-xs font-medium text-text-primary">{s.query_text}</span>
                                <span className="ml-3 shrink-0 font-mono text-[11px] text-text-muted">{s.status}</span>
                              </button>
                              <button
                                type="button"
                                data-slot="button"
                                data-testid={`rerun-session-${s.id}`}
                                title="Re-run query"
                                aria-label={`Re-run query "${s.query_text}"`}
                                onClick={() => {
                                  setQuery(s.query_text);
                                  const inputEl = document.querySelector<HTMLInputElement>('input[aria-label="Research question"]');
                                  inputEl?.focus();
                                }}
                                className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border-subtle bg-bg-elevated text-text-muted hover:text-brass hover:border-brass/45 transition-colors text-xs leading-none shrink-0"
                              >
                                <Icon.rotateCcw className="size-3.5 shrink-0" aria-hidden="true" />
                              </button>
                            </li>
                          ))}
                        </ul>
                      );
                    })()}
                  </BentoScrollRegion>
                </details>

                {detail && (
                  <div className="space-y-2.5">
                    <div className="flex items-center justify-between flex-wrap gap-1.5 border-b border-border-subtle pb-2">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-display text-xs font-bold uppercase tracking-wide text-text-primary truncate max-w-[18rem]">
                          Session {detail.session.id} · {detail.session.query_text}
                        </span>
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
                            className="inline-flex items-center gap-1.5 rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 text-[11px] font-mono text-text-secondary"
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
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <button
                          type="button"
                          data-slot="button"
                          data-testid="toggle-judge-btn"
                          aria-pressed={showJudgeInspector}
                          title="Inspect claim-level citation precision, grounding verdicts, and epistemic verifier confidence"
                          onClick={() => setShowJudgeInspector(prev => !prev)}
                          className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2 text-xs transition-colors"
                        >
                          Judge
                        </button>
                        <button
                          type="button"
                          data-slot="button"
                          title="Verify empirical hypotheses or reproduce numbers interactively in a sandboxed session"
                          onClick={() => setIsReplModalOpen(true)}
                          className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2 text-xs transition-colors"
                        >
                          Sandbox REPL
                        </button>
                        <button
                          type="button"
                          data-slot="button"
                          data-testid="save-to-kb-button"
                          title="Quarantine verified session sources into the KB Findings Inbox for curation and indexing"
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
                          className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2 text-xs disabled:opacity-50 transition-colors"
                        >
                          {isSavingToKb ? 'Saving…' : 'Save to KB'}
                        </button>
                        <button
                          type="button"
                          data-slot="button"
                          data-testid="promote-to-scientia-btn"
                          aria-label="Promote to Scientia Publication"
                          title="Create a structured Scientia publication manifest preserving empirical class and extracted atomic claims"
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
                          className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2 text-xs disabled:opacity-50 transition-colors"
                        >
                          {isPromotingScientia ? 'Promoting…' : 'Promote'}
                        </button>
                        <button
                          type="button"
                          data-slot="button"
                          onClick={() => setIsPublishModalOpen(true)}
                          className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2 text-xs transition-colors"
                        >
                          Publish SSOT
                        </button>
                        <button
                          type="button"
                          data-slot="button"
                          data-roman-plaque="true"
                          data-testid="open-synthesis-in-doc-dock-btn"
                          title="Seed this synthesized report with valid YAML frontmatter into the non-modal Doc Dock Split editor"
                          onClick={() => {
                            if (!detail?.session) return;
                            const rawSlug = detail.session.query_text
                              .toLowerCase()
                              .replace(/[^a-z0-9]+/g, '-')
                              .replace(/^-+|-+$/g, '')
                              .slice(0, 48);
                            const docPath = `docs/src/architecture/${rawSlug || `research-session-${detail.session.id}`}-findings-2026.md`;
                            const escapedTitle = detail.session.query_text.replace(/"/g, '\\"');
                            const reportBody =
                              detail.report_markdown ??
                              detail.artifact_json ??
                              detail.session.query_text;
                            const citationSection =
                              allCitations.length > 0
                                ? `\n\n## Sources\n\n${allCitations.map((u, i) => `${i + 1}. ${u}`).join('\n')}`
                                : '';
                            const seeded = [
                              '---',
                              `title: "${escapedTitle}"`,
                              `description: "Verified Vox Deep Research synthesis for session #${detail.session.id}."`,
                              'category: "Architecture SSOTs"',
                              'status: "research"',
                              'topic: "scientia-research"',
                              'domain: "knowledge"',
                              'authorship: "llm-assisted"',
                              '---',
                              '',
                              `# ${detail.session.query_text}`,
                              '',
                              '<!-- kb:summary -->',
                              `Synthesized from Vox Deep Research session #${detail.session.id} across ${allCitations.length} grounded source citations.`,
                              '<!-- /kb:summary -->',
                              '',
                              reportBody,
                              citationSection,
                            ]
                              .filter((s) => s !== undefined)
                              .join('\n');
                            window.dispatchEvent(
                              new CustomEvent('vox://open-doc', {
                                detail: {
                                  path: docPath,
                                  title: detail.session.query_text,
                                  initialContent: seeded,
                                  initialMode: 'split',
                                },
                              }),
                            );
                          }}
                          className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2 text-xs transition-colors"
                        >
                          Edit in Doc Dock
                        </button>
                        <button
                          type="button"
                          data-slot="button"
                          onClick={() => setDetail(null)}
                          className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-elevated px-2 text-xs font-medium text-text-muted hover:text-text-primary transition-colors"
                        >
                          Close
                        </button>
                      </div>
                    </div>

                    {detail.session.status !== 'completed' && (
                      <PipelineTimeline stages={RESEARCH_STAGES} statuses={deriveStages(detail.session.status)} />
                    )}

                    {parsedMeta?.planner_degraded && (
                      <div
                        data-testid="planner-degraded-chip"
                        role="status"
                        className="flex items-center gap-2 rounded-lg border border-(--color-status-warn)/40 bg-bg-surface px-3 py-1.5 text-xs text-(--color-status-warn)"
                      >
                        <span className="inline-flex items-center gap-1 font-semibold">
                          <Icon.alert className="size-3.5" aria-hidden="true" />
                          <span>Planner degraded</span>
                        </span>
                        <span className="text-text-secondary text-xs">
                          Subquery quality may be reduced due to degraded planner mode.
                        </span>
                      </div>
                    )}

                    {showJudgeInspector && (
                      <JudgeInspector
                        confidenceTier={detail.confidence_tier}
                        sourceCount={detail.source_count}
                        citationPrecision={detail.citation_precision}
                        claims={detail.claims}
                      />
                    )}

                    {detailWorkspaceTab !== 'diagnostics' && (
                      <>
                        {detailWorkspaceTab === 'dag' && (
                          <div
                            data-testid="dag-claims-focus-banner"
                            className="rounded-lg border border-brass/40 bg-bg-elevated px-3 py-2 text-xs flex flex-wrap items-center justify-between gap-2"
                          >
                            <span className="font-display text-[11px] font-bold uppercase tracking-wider text-brass">
                              DAG &amp; Claims Verification Focus · {claimRows.length} claims ({contradictedClaims + contestedClaims} disputed)
                            </span>
                            <span className="font-mono text-[11px] text-text-secondary">
                              Gate {minConfidenceGate}% min · Observed {stabilityPercent}% ({stabilityPercent >= minConfidenceGate ? 'meets gate' : 'below gate'})
                            </span>
                          </div>
                        )}
                        {claimRows.length > 0 && (
                          <HeadlineVerdictBanner
                            confidenceTier={detail.confidence_tier ?? 'DeepResearch'}
                            corroboratingSources={distinctCorroboratingSources}
                            contestedClaims={contestedClaims}
                            contradictedClaims={contradictedClaims}
                            totalClaims={claimRows.length}
                          />
                        )}
                        {shoppingDecision && <ShoppingDecisionCard decision={shoppingDecision} />}
                        {codegenVerification && <CodeGenVerificationCard verification={codegenVerification} />}

                        <div className="rounded-lg border border-border-strong bg-bg-surface p-3.5 shadow-[inset_0_1px_4px_rgba(120,83,18,0.1)]">
                          <BentoScrollRegion
                            ariaLabel="Research synthesis report"
                            className={`${detailWorkspaceTab === 'synthesis' ? 'max-h-[620px]' : 'max-h-[165px]'} pr-1`}
                          >
                            <ResearchReportMarkdown
                              markdown={detail.report_markdown ?? detail.artifact_json ?? '(no artifact persisted)'}
                              onCitationClick={handleCitationClick}
                            />
                          </BentoScrollRegion>
                        </div>
                      </>
                    )}

                    {/* Research Process Observability, Scoped Storage & Mid-Flight Wave Steering (positioned below synthesis so conclusions lead above the fold) */}
                    {(() => {
                      const rawSubqueries: string[] = Array.isArray(parsedMeta?.subqueries)
                        ? parsedMeta.subqueries.map(String)
                        : Array.isArray(parsedMeta?.search_queries)
                          ? parsedMeta.search_queries.map(String)
                          : [detail.session.query_text];
                      const sourceDomains = Array.from(
                        new Set(
                          allCitations
                            .map((u) => {
                              try {
                                return new URL(u).hostname.replace(/^www\./, '');
                              } catch {
                                return '';
                              }
                            })
                            .filter(Boolean),
                        ),
                      ).slice(0, 4);
                      const droppedCount = parsedMeta?.retrieval_diagnostics?.dropped_source_count ?? 0;

                      return (
                        <div className="space-y-2 rounded-lg border border-border-subtle bg-bg-elevated/70 p-2.5">
                          <div
                            data-testid="research-process-control-strip"
                            className="flex flex-wrap items-center gap-2"
                          >
                            <div className="flex flex-wrap items-center gap-1.5 min-w-0 flex-1 font-mono text-[11px] text-text-secondary">
                              {detail.session.status === 'completed' ? (
                                <span
                                  data-testid="research-completed-summary-pill"
                                  className="inline-flex items-center gap-1 rounded border border-(--color-status-pass)/40 bg-bg-surface px-2 py-0.5 font-mono text-[11px] text-(--color-status-pass)"
                                >
                                  <Icon.check className="size-3 shrink-0" aria-hidden="true" />
                                  <span>
                                    Completed · {parsedMeta?.subquery_count ?? parsedMeta?.subqueries?.length ?? 1} subqueries ·{' '}
                                    {effectiveWaveCount ?? 1} waves · {allCitations.length} sources
                                  </span>
                                </span>
                              ) : (
                                <span className="font-display font-bold uppercase tracking-wider text-brass">
                                  Process
                                </span>
                              )}
                              <span className="truncate">
                                Subqueries: {rawSubqueries.slice(0, 2).join(' | ')}
                              </span>
                              {sourceDomains.length > 0 && (
                                <span className="text-text-muted">
                                  · Domains: {sourceDomains.join(', ')}
                                </span>
                              )}
                              {droppedCount > 0 && (
                                <span className="text-(--color-status-warn)">
                                  · {droppedCount} low-novelty pruned
                                </span>
                              )}
                            </div>
                            <div className="flex flex-wrap items-center gap-1.5">
                              <input
                                type="text"
                                data-testid="research-steer-input"
                                aria-label="Steer next refinement wave"
                                value={steerPrompt}
                                onChange={(e) => setSteerPrompt(e.target.value)}
                                placeholder="Steer next wave: focus benchmarks, exclude blogs…"
                                className="h-8 w-56 max-w-full rounded border border-border-strong bg-bg-surface px-2.5 font-sans text-xs text-text-primary placeholder:text-text-muted outline-hidden focus:border-brass"
                              />
                              <button
                                type="button"
                                data-slot="button"
                                data-roman-plaque="true"
                                data-testid="research-steer-wave-btn"
                                disabled={running}
                                title="Run an additional deep refinement wave incorporating your steering constraint"
                                onClick={() => {
                                  const baseWaves = effectiveWaveCount ?? waves;
                                  const nextWaves = Math.min(5, Math.max(1, baseWaves + 1)) as
                                    | 1
                                    | 2
                                    | 3
                                    | 4
                                    | 5;
                                  const trimmedSteer = steerPrompt.trim();
                                  const refinedQuery = trimmedSteer
                                    ? `${detail.session.query_text} [Refine: ${trimmedSteer}]`
                                    : detail.session.query_text;
                                  setLane('deep');
                                  setWaves(nextWaves);
                                  setSteerPrompt('');
                                  void run('deep', scope, nextWaves, refinedQuery);
                                }}
                                className="vox-brass-plaque-btn inline-flex h-8 items-center gap-1 rounded px-2.5 text-[11px] disabled:opacity-50 transition-colors"
                              >
                                <Icon.play className="size-3 shrink-0" aria-hidden="true" />
                                <span>+1 Wave Refinement</span>
                              </button>
                            </div>
                          </div>

                          <div
                            data-testid="research-storage-control-bar"
                            className="flex flex-wrap items-center justify-between gap-2 border-t border-border-subtle/70 pt-2 text-[11px]"
                          >
                            <div className="flex flex-wrap items-center gap-1.5">
                              <span className="font-display font-bold uppercase tracking-wider text-text-secondary">
                                Storage Target
                              </span>
                              <button
                                type="button"
                                data-slot="button"
                                data-roman-plaque="true"
                                data-testid="research-storage-scope-toggle"
                                title="Choose whether direct indexing writes strictly to this repository KB or graduates to your global user library (~/.vox/library)"
                                onClick={() =>
                                  setResearchStorageScope((prev) =>
                                    prev === 'repo_local' ? 'global_library' : 'repo_local',
                                  )
                                }
                                className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2.5 text-[11px] transition-colors"
                              >
                                <span>
                                  {researchStorageScope === 'global_library'
                                    ? 'Global Library'
                                    : 'Repo-Local KB'}
                                </span>
                              </button>
                              <button
                                type="button"
                                data-slot="button"
                                data-roman-plaque="true"
                                data-testid="direct-index-kb-btn"
                                disabled={isDirectIndexingKb}
                                title="Index synthesis report directly into searchable Knowledge Base under the selected scope for immediate 0-click recall"
                                onClick={async () => {
                                  if (!detail?.session) return;
                                  setIsDirectIndexingKb(true);
                                  try {
                                    const content =
                                      detail.report_markdown ??
                                      detail.artifact_json ??
                                      detail.session.query_text;
                                    const nodeId = await invokeCmd<string>('ingest_text', {
                                      title: detail.session.query_text,
                                      content,
                                      sourceUrl: allCitations[0] ?? null,
                                      targetScope: researchStorageScope,
                                    });
                                    pushToast?.({
                                      tone: 'ok',
                                      title: 'Indexed into searchable KB',
                                      body: `${researchStorageScope === 'global_library' ? 'Global Library' : 'Repo-Local KB'} (${String(nodeId)}) ready for recall.`,
                                      cause: 'backend-ok',
                                    });
                                  } catch (err) {
                                    pushToast?.({
                                      tone: 'warn',
                                      title: 'Direct index failed',
                                      body: sanitizeErrorForToast(err),
                                      cause: 'backend-error',
                                    });
                                  } finally {
                                    setIsDirectIndexingKb(false);
                                  }
                                }}
                                className="vox-brass-plaque-btn inline-flex h-8 items-center gap-1 rounded px-2.5 text-[11px] disabled:opacity-50 transition-colors"
                              >
                                <Icon.bolt className="size-3 shrink-0" aria-hidden="true" />
                                <span>{isDirectIndexingKb ? 'Indexing…' : 'Index Now'}</span>
                              </button>
                            </div>
                            <span className="font-mono text-[11px] text-text-muted">
                              Index Now = immediate recall · Save to KB = Findings quarantine review
                            </span>
                          </div>
                        </div>
                      );
                    })()}

                    {detailWorkspaceTab !== 'diagnostics' && (
                      <>
                        {allCitations.length > 0 && (
                          <details open className="rounded-lg border border-border-subtle bg-bg-surface p-3">
                            <summary className="cursor-pointer font-display text-xs uppercase tracking-wider text-text-muted flex flex-wrap items-center justify-between gap-2 select-none">
                              <span>Citations &amp; Sources ({allCitations.length})</span>
                              {parsedMeta?.retrieval_diagnostics?.dropped_source_count != null &&
                                parsedMeta.retrieval_diagnostics.dropped_source_count > 0 && (
                                  <span
                                    data-testid="dropped-source-count"
                                    className="font-sans text-xs text-(--color-status-warn) font-normal normal-case tracking-normal"
                                  >
                                    {parsedMeta.retrieval_diagnostics.dropped_source_count} sources filtered by novelty
                                  </span>
                                )}
                            </summary>
                            <BentoScrollRegion ariaLabel="Research citations and sources" className="mt-2.5 max-h-52 pr-1">
                              <ul className="space-y-2" role="list">
                                {allCitations.map((url) => (
                                  <li key={url} role="listitem" className="flex items-center justify-between gap-2 min-w-0">
                                    <SafeExternalLink
                                      url={url}
                                      className="truncate text-xs text-brass underline decoration-dotted hover:text-brass/80"
                                    />
                                    <button
                                      type="button"
                                      data-slot="button"
                                      data-testid="flag-citation-misleading"
                                      onClick={() => {
                                        setFlagTargetUrl(url);
                                        setIsFlagModalOpen(true);
                                      }}
                                      className="inline-flex h-7 shrink-0 items-center rounded border border-border-subtle bg-bg-elevated px-2.5 text-[11px] font-medium text-text-secondary hover:text-text-primary hover:border-brass/45 transition-colors"
                                    >
                                      Flag Citation
                                    </button>
                                  </li>
                                ))}
                              </ul>
                            </BentoScrollRegion>
                          </details>
                        )}

                        {/* Enameled Malachite Green & Crimson Verdict Action Strip */}
                        <div className="flex flex-wrap items-center gap-3 pt-1">
                          <button
                            type="button"
                            data-slot="button"
                            data-roman-plaque="true"
                            data-roman-wax-seal="true"
                            onClick={() => setDetailWorkspaceTab('dag')}
                            className="vox-enameled-emerald-btn inline-flex h-9 items-center gap-2 rounded-md px-4 text-xs"
                          >
                            <Icon.check className="size-3.5 shrink-0" aria-hidden="true" />
                            <span>Corroborated · {distinctCorroboratingSources}</span>
                          </button>
                          <button
                            type="button"
                            data-slot="button"
                            data-roman-plaque="true"
                            data-roman-wax-seal="true"
                            onClick={() => setDetailWorkspaceTab('dag')}
                            className="vox-enameled-crimson-btn inline-flex h-9 items-center gap-2 rounded-md px-4 text-xs"
                          >
                            <Icon.alert className="size-3.5 shrink-0" aria-hidden="true" />
                            <span>Contested · {contestedClaims + contradictedClaims}</span>
                          </button>
                        </div>

                        {(effectiveWaveCount != null && effectiveWaveCount > 1) && detailWorkspaceTab !== 'synthesis' && detailWorkspaceTab !== 'dag' && (
                          <div className="mt-1" data-testid="detail-multi-wave-timeline">
                            <MultiWaveProgressTimeline
                              currentWave={detail.session.status === 'completed' ? effectiveWaveCount + 1 : (liveWave ?? effectiveWaveCount)}
                              totalWaves={Math.min(5, Math.max(effectiveWaveCount, 3))}
                              stabilityScore={effectiveWaveStability}
                              earlyExitTriggered={Boolean(effectiveWaveStability != null && effectiveWaveStability >= 0.85 && effectiveWaveCount < Math.max(effectiveWaveCount, 3))}
                            />
                          </div>
                        )}
                      </>
                    )}

                    {detailWorkspaceTab === 'diagnostics' && (
                      <div className="space-y-3">
                        {!showJudgeInspector && (
                          <JudgeInspector
                            confidenceTier={detail.confidence_tier}
                            sourceCount={detail.source_count}
                            citationPrecision={detail.citation_precision}
                            claims={detail.claims}
                          />
                        )}
                        {streamPreviewPane}
                      </div>
                    )}

                    {detailWorkspaceTab === 'overview' && streamPreviewPane}
                  </div>
                )}
              </BentoCard>

              {/* RIGHT RECESSED TABLET: MULTI-WAVE ENGINE & DAG + KNURLED BRASS DIALS + DRAWERS */}
              <BentoCard
                as="section"
                className="vox-recessed-tablet @3xl/surface:col-span-5 gap-3 p-4"
              >
                <div className="flex flex-col items-center gap-1.5">
                  <span className="font-display text-xs font-bold uppercase tracking-[0.16em] text-text-primary text-center">
                    Multi-Wave Engine &amp; DAG
                  </span>
                  <div className="vox-rule w-full" aria-hidden="true" />
                </div>

                {detail && dagNodes.length > 0 ? (
                  <div className="rounded-lg border border-border-strong bg-bg-surface p-3">
                    <div className="mb-2 flex items-center justify-between">
                      <span className="font-display text-xs font-semibold uppercase tracking-wider text-text-secondary">
                        Epistemic Research DAG
                      </span>
                      <span className="font-mono text-[11px] text-text-muted">
                        {dagNodes.length} nodes · {dagEdges.length} edges
                      </span>
                    </div>
                    <ResearchDagCanvas nodes={dagNodes} edges={dagEdges} />
                  </div>
                ) : (
                  <div className="rounded-lg border border-dashed border-border-strong bg-bg-surface p-5 text-center space-y-1.5">
                    <div className="font-display text-xs font-bold uppercase tracking-wider text-text-primary">
                      Awaiting Wave Graph Telemetry
                    </div>
                    <p className="font-serif text-xs text-text-secondary">
                      Select a research session or run a query to visualize claim verification nodes and cross-wave corroboration edges.
                    </p>
                  </div>
                )}

                {/* Physical 3D Knurled Brass Rotary Instrument Bay (`WAVE DEPTH` + `CONFIDENCE GATE`) — directly under DAG */}
                <div className="vox-recessed-subwell flex flex-wrap items-center justify-around gap-4 rounded-xl p-3">
                  <RomanRotaryKnob
                    ariaLabel="Wave depth"
                    label="Wave Depth"
                    value={waves}
                    min={1}
                    max={5}
                    step={1}
                    detents={[1, 2, 3, 4, 5]}
                    size="md"
                    formatValue={(v) => `${v} Waves`}
                    onValueChange={(v) => setWaves(v as 1 | 2 | 3 | 4 | 5)}
                  />
                  <div className="flex flex-col items-center gap-1">
                    <RomanRotaryKnob
                      ariaLabel="Minimum confidence gate"
                      label="Confidence Gate"
                      value={minConfidenceGate}
                      min={50}
                      max={95}
                      step={5}
                      detents={[60, 70, 75, 85, 95]}
                      size="md"
                      formatValue={(v) => `${v}% min`}
                      onValueChange={(v) => setMinConfidenceGate(v)}
                    />
                    <span
                      data-testid="confidence-gate-status"
                      className={`font-mono text-[11px] ${
                        stabilityPercent >= minConfidenceGate
                          ? 'text-(--color-status-pass)'
                          : 'text-(--color-status-warn)'
                      }`}
                    >
                      Observed {stabilityPercent}% ({stabilityPercent >= minConfidenceGate ? 'meets gate' : 'below gate'})
                    </span>
                  </div>
                </div>

                {/* Full-Width Beveled Cast-Brass Drawer Bars */}
                <div className="space-y-2">
                  <button
                    type="button"
                    data-slot="button"
                    data-roman-plaque="true"
                    data-testid="toggle-audit-inspector-btn"
                    aria-expanded={showAuditInspector}
                    onClick={() => setShowAuditInspector(prev => !prev)}
                    className="vox-brass-plaque-btn flex h-9 w-full items-center justify-between rounded-md px-3.5 text-xs transition-colors"
                  >
                    <span>Before / After Audit Inspector</span>
                    <Icon.chevR className={`size-3.5 shrink-0 transition-transform ${showAuditInspector ? '-rotate-90' : 'rotate-90'}`} aria-hidden="true" />
                  </button>
                  {showAuditInspector && (
                    <BeforeAfterAuditInspector compact />
                  )}

                  <button
                    type="button"
                    data-slot="button"
                    data-roman-plaque="true"
                    data-testid="toggle-diagnostics-btn"
                    aria-expanded={showDiagnostics}
                    onClick={() => setShowDiagnostics(prev => !prev)}
                    className="vox-brass-plaque-btn flex h-9 w-full items-center justify-between rounded-md px-3.5 text-xs transition-colors"
                  >
                    <span>Diagnostic Prober</span>
                    <Icon.chevR className={`size-3.5 shrink-0 transition-transform ${showDiagnostics ? '-rotate-90' : 'rotate-90'}`} aria-hidden="true" />
                  </button>
                  {showDiagnostics && (
                    <LiveSourceProber initialQuery={query} />
                  )}
                </div>

                {detail && claimRows.length > 0 && (
                  <VirtualizedClaimsTable
                    claims={claimRows}
                    sourceCount={detail.source_count ?? 0}
                    totalClaims={detail.claims_total ?? claimRows.length}
                    highlightedClaimId={highlightedClaimId ?? undefined}
                    hasMore={Boolean(detail.claims_total != null && claimRows.length < detail.claims_total)}
                    loadingMore={loadingMoreClaims}
                    onLoadMore={() => { void loadMoreClaims(); }}
                  />
                )}
              </BentoCard>
            </div>

            {detail && (
              <>
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
              </>
            )}
          </div>
        );
      })()}
      <ResearchEngineDrawer
        isOpen={isEngineDrawerOpen}
        onClose={() => setIsEngineDrawerOpen(false)}
        onConfigSaved={loadEngineStatus}
        pushToast={pushToast}
      />
    </section>
  );
}
