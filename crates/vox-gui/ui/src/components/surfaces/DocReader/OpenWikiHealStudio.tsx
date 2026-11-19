import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { BackendUnavailableError, sanitizeErrorForToast } from '../../../lib/backendGuard';
import { unwrapMcpEnvelope, type McpInvokeResult } from '../../../lib/mcpToolResult';
import { voxTransport } from '../../../transport';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { extractToolError } from '../Repository/repoCatalogHelpers';
import { OpenWikiDiffPane } from './OpenWikiDiffPane';
import type { KbHealProposalDto, OpenWikiStatusDto } from './docReaderHelpers';

export interface OpenWikiHealStudioProps {
  onSelectDocPath?: (docPath: string) => void;
  compact?: boolean;
}

export function OpenWikiHealStudio({ onSelectDocPath, compact = false }: OpenWikiHealStudioProps) {
  const [wikiStatus, setWikiStatus] = useState<OpenWikiStatusDto | null>(null);
  const [proposals, setProposals] = useState<KbHealProposalDto[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [lastOutcomeMsg, setLastOutcomeMsg] = useState<string | null>(null);
  const [editingMarkdown, setEditingMarkdown] = useState<string | null>(null);

  const loadStudio = useCallback(async () => {
    if (typeof voxTransport.invokeMcpTool !== 'function') return;
    setLoading(true);
    setErrorMsg(null);
    try {
      const [statusRes, listRes] = await Promise.all([
        voxTransport.invokeMcpTool('vox_repo_catalog_refresh', { action: 'wiki_status' }),
        voxTransport.invokeMcpTool('vox_repo_catalog_refresh', { action: 'wiki_heal_list' }),
      ]);
      const stErr = extractToolError(statusRes, 'OpenWiki backend tool error.');
      if (stErr) setErrorMsg(stErr);
      const stData = unwrapMcpEnvelope((statusRes as McpInvokeResult | undefined)?.result);
      if (stData && typeof stData === 'object' && ('pages' in stData || 'initialized' in stData)) {
        setWikiStatus(stData as OpenWikiStatusDto);
      }
      const listData = unwrapMcpEnvelope((listRes as McpInvokeResult | undefined)?.result) as
        | { proposals?: KbHealProposalDto[] }
        | undefined;
      if (listData && Array.isArray(listData.proposals)) {
        setProposals(listData.proposals);
        if (listData.proposals.length > 0) {
          setSelectedId((prev) =>
            prev && listData.proposals!.some((p) => p.id === prev) ? prev : listData.proposals![0]!.id,
          );
        }
      }
    } catch (err) {
      if (!(err instanceof BackendUnavailableError) && !(err instanceof Error && err.name === 'BackendUnavailableError')) {
        setErrorMsg(sanitizeErrorForToast(err));
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadStudio();
  }, [loadStudio]);

  const runHealAction = async (
    action: 'wiki_heal_tier1' | 'wiki_heal_apply' | 'wiki_heal_demote' | 'wiki_heal_reject' | 'wiki_init',
    proposalId?: string,
  ) => {
    if (typeof voxTransport.invokeMcpTool !== 'function') return;
    setBusyAction(proposalId ? `${action}:${proposalId}` : action);
    setErrorMsg(null);
    try {
      const args: Record<string, unknown> = { action };
      if (proposalId) args.proposal_id = proposalId;
      const res = await voxTransport.invokeMcpTool('vox_repo_catalog_refresh', args);
      const err = extractToolError(res, `${action} failed.`);
      if (err) {
        setErrorMsg(err);
        return;
      }
      setLastOutcomeMsg(`${action}${proposalId ? ` (${proposalId})` : ''} completed`);
      setEditingMarkdown(null);
      await loadStudio();
    } catch (err) {
      setErrorMsg(sanitizeErrorForToast(err));
    } finally {
      setBusyAction(null);
    }
  };

  const selectedProposal = useMemo(
    () => proposals.find((p) => p.id === selectedId) ?? proposals[0] ?? null,
    [proposals, selectedId],
  );
  const stalePages = wikiStatus?.stale_pages ?? [];
  const pages = wikiStatus?.pages ?? [];

  return (
    <section
      data-testid="openwiki-heal-studio"
      className="space-y-3 rounded-lg border border-border-subtle bg-bg-surface p-3 text-xs"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2">
        <div className="flex items-center gap-2">
          <Icon.book className="size-4 shrink-0 text-brass" aria-hidden="true" />
          <h3 className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
            OpenWiki 5-Layer Self-Heal & Diff Inspector
          </h3>
          {wikiStatus && (
            <span className="rounded border border-border-subtle bg-bg-elevated px-2.5 py-0.5 font-mono text-[11px] text-text-secondary">
              {wikiStatus.page_count ?? 0}/{wikiStatus.total_pages ?? 6} pages ·{' '}
              {wikiStatus.auto_region_count ?? 0} auto-fences · {stalePages.length} stale
            </span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <Button
            variant="outline"
            size="sm"
            data-testid="openwiki-refresh-btn"
            className="h-8 px-3 text-xs"
            disabled={loading || busyAction !== null}
            onClick={() => void loadStudio()}
          >
            Refresh
          </Button>
          <Button
            variant="outline"
            size="sm"
            data-testid="openwiki-init-btn"
            className="h-8 px-3 text-xs"
            disabled={busyAction !== null}
            onClick={() => void runHealAction('wiki_init')}
          >
            Rebuild 5-Layer Wiki
          </Button>
          <Button
            variant="outline"
            size="sm"
            data-testid="openwiki-heal-tier1-btn"
            className="h-8 px-3 text-xs"
            disabled={busyAction !== null}
            onClick={() => void runHealAction('wiki_heal_tier1')}
          >
            Heal All Tier-1
          </Button>
        </div>
      </div>

      {errorMsg && (
        <div
          role="alert"
          data-testid="openwiki-error-banner"
          className="rounded border border-(--color-status-error)/40 bg-(--color-status-error)/10 px-3 py-2 font-mono text-[11px] text-(--color-status-error)"
        >
          {errorMsg}
        </div>
      )}

      {lastOutcomeMsg && (
        <div
          data-testid="openwiki-outcome-banner"
          className="rounded border border-(--color-status-pass)/40 bg-(--color-status-pass)/10 px-3 py-1.5 font-mono text-[11px] text-(--color-status-pass)"
        >
          {lastOutcomeMsg}
        </div>
      )}

      {!compact && pages.length > 0 && (
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3" data-testid="openwiki-pages-grid">
          {pages.map((pg) => {
            const isStale = stalePages.includes(pg.rel_path);
            return (
              <button
                key={pg.rel_path}
                type="button"
                onClick={() => onSelectDocPath?.(`docs/wiki/${pg.rel_path}`)}
                className="flex items-center justify-between gap-2 rounded border border-border-subtle bg-bg-elevated px-2.5 py-1.5 text-left font-mono text-[11px] text-text-primary hover:border-brass/50"
              >
                <span className="truncate">{pg.rel_path}</span>
                <span
                  className={`shrink-0 rounded px-1.5 py-0.5 text-[11px] uppercase ${
                    !pg.exists || isStale
                      ? 'bg-(--color-status-warn)/15 text-(--color-status-warn)'
                      : 'bg-(--color-status-pass)/15 text-(--color-status-pass)'
                  }`}
                >
                  {!pg.exists ? 'missing' : isStale ? 'stale' : 'fresh'}
                </span>
              </button>
            );
          })}
        </div>
      )}

      {!compact && (
        <div className="space-y-2">
          <div className="font-display text-xs font-semibold uppercase tracking-wider text-text-secondary">
            Tier-2 Staged Heal Proposals ({proposals.length})
          </div>
          {proposals.length === 0 ? (
            <div className="rounded border border-dashed border-border-subtle bg-bg-elevated px-3 py-4 text-center text-xs text-text-muted">
              No pending Tier-2 documentation heal proposals queued in VoxDb.
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-3 lg:grid-cols-12">
              <div className="space-y-1.5 lg:col-span-5" data-testid="openwiki-proposals-list">
                {proposals.map((prop) => {
                  const selected = selectedProposal?.id === prop.id;
                  return (
                    <button
                      key={prop.id}
                      type="button"
                      data-slot="button"
                      data-testid={`openwiki-proposal-row-${prop.id}`}
                      onClick={() => {
                        setSelectedId(prop.id);
                        setEditingMarkdown(null);
                      }}
                      className={`w-full rounded border p-2.5 text-left transition-colors ${
                        selected
                          ? 'border-brass/70 bg-brass/10'
                          : 'border-border-subtle bg-bg-elevated hover:border-brass/40'
                      }`}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="truncate font-mono text-xs font-semibold text-text-primary">
                          {prop.doc_path}
                        </span>
                        <span className="rounded border border-border-subtle bg-bg-surface px-1.5 py-0.5 font-mono text-[11px] uppercase text-text-secondary">
                          T{prop.tier} · {prop.status}
                        </span>
                      </div>
                      <p className="mt-1 line-clamp-2 text-xs text-text-secondary">{prop.summary}</p>
                    </button>
                  );
                })}
              </div>

              {selectedProposal && (
                <OpenWikiDiffPane
                  proposal={selectedProposal}
                  editingMarkdown={editingMarkdown}
                  busyAction={busyAction}
                  onChangeMarkdown={setEditingMarkdown}
                  onAction={(act, id) => void runHealAction(act, id)}
                />
              )}
            </div>
          )}
        </div>
      )}
    </section>
  );
}
