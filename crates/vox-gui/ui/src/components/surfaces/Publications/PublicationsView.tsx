import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { PUBLICATION_STAGES, groupByStage, PublicationManifest } from '../../../lib/pipeline';
import { useLabel } from '../../../hooks/useLanguage';
import { draftManuscript, renderLatex } from '../../../api/scientia';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';

interface ExecuteOutput {
  exit_code: number;
  stdout: string;
  stderr: string;
}

interface ClaimRow {
  id?: number;
  text?: string;
  verdict_label?: string;
  confidence?: number;
}

interface ClaimsPayload {
  publication_id: string;
  claim_count: number;
  claims: ClaimRow[];
}

interface VenueRecommendation {
  candidate_class: string;
  recommended_venues: string[];
  reply_window_days: number;
  negative_result_quota: number;
  critic_allowed: boolean;
  atlas_gate_applies: boolean;
}

export function PublicationsView({ pushToast }: SurfaceDecoratorProps) {
  const [manifests, setManifests] = useState<PublicationManifest[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<PublicationManifest | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setManifests(await invoke<PublicationManifest[]>('list_publication_manifests', { limit: 200 }));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Publications load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoading(false);
    }
  }, [pushToast]);

  useEffect(() => { refresh(); }, [refresh]);

  const groups = groupByStage(manifests);

  return (
    <section className="space-y-4">
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
            ratio: Math.min(1, manifests.length / 20),
            tone: 'gold',
            purpose: 'Total publication pipeline manifests',
          },
          {
            label: 'PUBLISHED',
            valueText: `${groups['archived']?.length ?? 0}`,
            ratio: Math.min(1, (groups['archived']?.length ?? 0) / 10),
            tone: 'emerald',
            purpose: 'Archived and released publications',
          },
          {
            label: 'STAGES',
            valueText: `${PUBLICATION_STAGES.length}`,
            ratio: 1,
            tone: 'gold',
            purpose: 'Active pipeline verification stages',
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
      <div className="grid grid-cols-2 gap-2.5 pb-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-7">
        {PUBLICATION_STAGES.map(stage => (
          <div key={stage} className="min-w-0">
            <div className="mb-2 flex items-center justify-between">
              <span className="font-mono text-[10px] uppercase tracking-wide text-[#D4AF37]">{stage.replace(/_/g, ' ')}</span>
              <span className="rounded-full border border-[#D4AF37]/30 bg-bg-surface px-1.5 font-mono text-[9px] text-[#F3E5AB]">{groups[stage].length}</span>
            </div>
            <div className="space-y-2">
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
                  <div className="text-[10px] text-[#D6CEBF]">{m.content_type}</div>
                </button>
              ))}
              {groups[stage].length === 0 && <div className="rounded-lg border border-dashed border-[#D4AF37]/20 p-2 text-center text-[10px] text-[#D6CEBF]/60">—</div>}
            </div>
          </div>
        ))}
      </div>

      {selected ? (
        <PublicationDetail
          manifest={selected}
          onClose={() => setSelected(null)}
          pushToast={pushToast}
        />
      ) : (
        <div
          data-testid="publications-pipeline-overview"
          className="grid grid-cols-1 gap-3 pt-2 sm:grid-cols-3"
        >
          <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3.5">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">IMRaD &amp; LaTeX</span>
              <RomanWaxSealBadge label={`${manifests.length} manifests`} tone="gold" />
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
              Select any card above to preview its generated IMRaD manuscript, compile LaTeX source, or inspect extracted epistemic claims.
            </p>
          </div>
          <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3.5">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Claim Verification</span>
              <RomanWaxSealBadge label="MiniCheck" tone="gold" />
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
              Every candidate paper undergoes automated claim extraction (<code className="text-[#F3E5AB]">vox scientia claims</code>) and confidence scoring before approval.
            </p>
          </div>
          <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3.5">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Archival Deposit</span>
              <RomanWaxSealBadge label="DOI + SWHID" tone="emerald" />
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-[#D6CEBF]">
              Tracks Zenodo DOI reservation, ORCID attribution, and Software Heritage (SWHID) source archival across 7 lifecycle stages.
            </p>
          </div>
        </div>
      )}
    </section>
  );
}

/**
 * Drill-down panel for one publication (F4): its lifecycle state, extracted
 * claims (`vox scientia claims`), and the venue-routing recommendation for its
 * class (`vox scientia publication-venue-recommend`). Both reuse the shared
 * `execute_command` bridge over existing CLI subcommands — no new backend.
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
  const [claims, setClaims] = useState<ClaimsPayload | null>(null);
  const [venue, setVenue] = useState<VenueRecommendation | null>(null);
  const [busy, setBusy] = useState(false);
  const [previewText, setPreviewText] = useState<string | null>(null);
  const [previewKind, setPreviewKind] = useState<'imrad' | 'latex' | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setClaims(null);
    setVenue(null);
    try {
      const claimsOut = await invoke<ExecuteOutput>('execute_command', {
        path: ['scientia', 'claims'],
        args: { __argv: ['--publication-id', manifest.publication_id] },
      });
      if (claimsOut.exit_code === 0) {
        try {
          setClaims(JSON.parse(claimsOut.stdout) as ClaimsPayload);
        } catch (parseErr) {
          pushToast({ tone: 'warn', title: 'Claims (parse error)', body: `${String(parseErr)} — raw: ${claimsOut.stdout.slice(0, 200)}`, cause: 'backend-error' });
        }
      } else {
        pushToast({ tone: 'warn', title: 'Claims', body: claimsOut.stderr || `exit ${claimsOut.exit_code}`, cause: 'backend-error' });
      }

      const venueOut = await invoke<ExecuteOutput>('execute_command', {
        path: ['scientia', 'publication-venue-recommend'],
        args: { __argv: ['--candidate-class', manifest.content_type] },
      });
      if (venueOut.exit_code === 0) {
        try {
          setVenue(JSON.parse(venueOut.stdout) as VenueRecommendation);
        } catch (parseErr) {
          pushToast({ tone: 'warn', title: 'Venue (parse error)', body: `${String(parseErr)} — raw: ${venueOut.stdout.slice(0, 200)}`, cause: 'backend-error' });
        }
      }
      // A non-zero venue exit is expected when content_type isn't a known
      // finding class; leave venue null and show the "no routing" state.
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Publication detail', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  }, [manifest.publication_id, manifest.content_type, pushToast]);

  useEffect(() => { load(); }, [load]);

  // Lifecycle stepper: the ordered stages a publication passes through. The
  // current stage is derived from the manifest state via the same stage grouping
  // the board uses.
  const _stageMap = groupByStage([manifest]);
  const stageIdx = PUBLICATION_STAGES.findIndex(s => (_stageMap[s]?.length ?? 0) > 0);
  const resolvedStageIdx = stageIdx === -1 ? 0 : stageIdx;

  return (
    <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-4">
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
          <div className="mb-2 flex items-center justify-between">
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
          <pre className="max-h-52 overflow-auto whitespace-pre-wrap font-mono text-[11px] text-text-primary">
            {previewText}
          </pre>
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

      <div className="grid gap-4 md:grid-cols-2">
        {/* Claims */}
        <div>
          <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-brass">
            Claims {claims ? `(${claims.claim_count})` : ''}
          </div>
          {!claims || claims.claims.length === 0 ? (
            <div className="rounded-lg border border-[#D4AF37]/25 bg-bg-surface p-3 text-[12px] text-text-secondary">
              {busy ? 'Loading…' : 'No extracted claims. Run claim extraction first.'}
            </div>
          ) : (
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

