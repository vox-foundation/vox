import React, { useCallback, useEffect, useState } from 'react';
import { BackendUnavailableError, sanitizeErrorForToast } from '../../../lib/backendGuard';
import { unwrapMcpEnvelope, type McpInvokeResult } from '../../../lib/mcpToolResult';
import { voxTransport } from '../../../transport';
import { BentoCard } from '../../ui/BentoGrid';
import { Icon } from '../../ui/Icons';
import type { Toast } from '../../../types/tauri';
import {
  detectedStacks,
  extractToolError,
  type CatalogPayload,
  type RepoStatusPayload,
} from './repoCatalogHelpers';

export interface RepositoryCatalogPanelProps {
  pushToast: (item: Toast) => void;
  onRepositoryChanged?: () => void;
}

export function RepositoryCatalogPanel({
  pushToast,
  onRepositoryChanged,
}: RepositoryCatalogPanelProps) {
  const [status, setStatus] = useState<RepoStatusPayload | null>(null);
  const [catalog, setCatalog] = useState<CatalogPayload | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [backendOffline, setBackendOffline] = useState(false);

  const [targetPath, setTargetPath] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [readOnlyMode, setReadOnlyMode] = useState(false);

  const fetchAll = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    setBackendOffline(false);
    try {
      const [statusRes, catalogRes] = await Promise.all([
        voxTransport.invokeMcpTool('vox_repo_status', {}),
        voxTransport.invokeMcpTool('vox_repo_catalog_list', {}),
      ]);
      const stErr = extractToolError(statusRes, 'Failed to read repository status.');
      const catErr = extractToolError(catalogRes, 'Failed to load repository catalog.');
      if (stErr) setErrorMsg(stErr);
      else if (catErr) setErrorMsg(catErr);

      const stData = unwrapMcpEnvelope((statusRes as McpInvokeResult | undefined)?.result);
      setStatus(
        stData && typeof stData === 'object' && ('repository_id' in stData || 'root' in stData)
          ? (stData as RepoStatusPayload)
          : null,
      );
      const catData = unwrapMcpEnvelope((catalogRes as McpInvokeResult | undefined)?.result);
      setCatalog(
        catData && typeof catData === 'object' && 'repositories' in catData
          ? (catData as CatalogPayload)
          : null,
      );
    } catch (err) {
      if (err instanceof BackendUnavailableError || (err instanceof Error && err.name === 'BackendUnavailableError')) {
        setBackendOffline(true);
      } else {
        setErrorMsg(sanitizeErrorForToast(err));
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchAll();
  }, [fetchAll]);

  const runCatalogAction = async (
    action: string,
    label: string,
    extraArgs: Record<string, unknown> = {},
  ) => {
    setBusyAction(action);
    setErrorMsg(null);
    try {
      const res = await voxTransport.invokeMcpTool('vox_repo_catalog_refresh', { action, ...extraArgs });
      const err = extractToolError(res, `${label} failed.`);
      if (err) {
        setErrorMsg(err);
        pushToast({ tone: 'warn', title: label, body: sanitizeErrorForToast(err), cause: 'backend-error' });
        return;
      }
      pushToast({ tone: 'ok', title: label, body: `Completed (${action})`, cause: 'backend-ok' });
      onRepositoryChanged?.();
      await fetchAll();
    } catch (err) {
      const clean = sanitizeErrorForToast(err);
      setErrorMsg(clean);
      pushToast({ tone: 'error', title: label, body: clean, cause: 'backend-error' });
    } finally {
      setBusyAction(null);
    }
  };

  const activeName =
    status?.active_context?.display_name ??
    status?.root?.split(/[\\/]/).filter(Boolean).pop() ??
    status?.repository_id ??
    'workspace';
  const activeRepoId = status?.active_context?.repository_id ?? status?.repository_id ?? null;
  const activeReadOnly = status?.active_context?.read_only ?? false;
  const stacks = detectedStacks(status?.capabilities);
  const wiki = status?.openwiki;
  const staleCount = wiki?.stale_pages?.length ?? 0;
  const repos = catalog?.repositories ?? [];

  return (
    <BentoCard
      span="full"
      data-testid="repo-catalog-panel"
      className="gap-3 border-border-subtle bg-bg-surface p-3.5 @2xl/surface:p-4"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2.5">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <Icon.branch className="size-4 shrink-0 text-brass" aria-hidden="true" />
          <h3 className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
            Multi-Repo Workspace Catalog & Onboarding
          </h3>
          {activeRepoId && (
            <span
              data-testid="repo-active-pill"
              className="rounded-full border border-brass/50 bg-brass/12 px-2 py-0.5 font-mono text-[10px] text-text-primary"
            >
              {activeName} ({activeReadOnly ? 'read_only' : 'hybrid'})
            </span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          {stacks.map((s) => (
            <span
              key={s}
              className="rounded border border-border-subtle bg-bg-elevated px-1.5 py-0.5 font-mono text-[10px] uppercase text-text-secondary"
            >
              {s}
            </span>
          ))}
          {wiki && (
            <span
              data-testid="repo-openwiki-freshness-pill"
              className={`rounded border px-1.5 py-0.5 font-mono text-[10px] ${
                wiki.initialized && staleCount === 0
                  ? 'border-(--color-status-pass)/45 bg-(--color-status-pass)/12 text-(--color-status-pass)'
                  : 'border-(--color-status-warn)/45 bg-(--color-status-warn)/12 text-(--color-status-warn)'
              }`}
            >
              OpenWiki {wiki.page_count ?? 0}/{wiki.total_pages ?? 6}{' '}
              {staleCount > 0 ? `· ${staleCount} stale` : wiki.initialized ? '· fresh' : '· uninit'}
            </span>
          )}
          <button
            type="button"
            data-testid="repo-catalog-refresh-btn"
            disabled={loading || busyAction !== null}
            onClick={() => void runCatalogAction('refresh', 'Refresh Repository Catalog')}
            className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-1 font-mono text-[10px] text-text-primary hover:border-brass/50 disabled:opacity-50"
          >
            <Icon.refresh className={`size-3 ${loading ? 'animate-spin' : ''}`} aria-hidden="true" />
            Refresh
          </button>
        </div>
      </div>

      {backendOffline && (
        <div className="rounded border border-border-subtle bg-bg-elevated px-3 py-2 font-mono text-[11px] text-text-muted">
          Desktop backend unavailable in browser preview; live repository catalog queries are disabled.
        </div>
      )}

      {errorMsg && (
        <div
          role="alert"
          data-testid="repo-catalog-error"
          className="rounded border border-(--color-status-error)/40 bg-(--color-status-error)/10 px-3 py-2 font-mono text-[11px] text-(--color-status-error)"
        >
          {errorMsg}
        </div>
      )}

      <div className="grid grid-cols-1 gap-3 @2xl/surface:grid-cols-2">
        <div className="space-y-2 rounded-lg border border-border-subtle bg-bg-elevated p-3">
          <div className="flex items-center justify-between">
            <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-secondary">
              Registered Repositories ({repos.length})
            </span>
            {activeRepoId && (
              <span className="font-mono text-[10px] text-text-muted">id: {activeRepoId}</span>
            )}
          </div>
          {repos.length === 0 ? (
            <div className="rounded border border-dashed border-border-subtle px-3 py-4 text-center font-mono text-[11px] text-text-muted">
              No repositories registered yet. Onboard or add a repository on the right.
            </div>
          ) : (
            <div className="space-y-1.5" data-testid="repo-catalog-list">
              {repos.map((r, idx) => {
                const targetKey = r.repository_id ?? r.display_name;
                const isActive =
                  (r.repository_id != null && r.repository_id === activeRepoId) ||
                  r.repository_id === catalog?.primary_repository_id;
                return (
                  <div
                    key={`${targetKey}-${idx}`}
                    data-testid={`repo-catalog-row-${targetKey}`}
                    className={`flex flex-wrap items-center justify-between gap-2 rounded border px-2.5 py-2 text-xs ${
                      isActive ? 'border-brass/60 bg-brass/10' : 'border-border-subtle bg-bg-surface'
                    }`}
                  >
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-1.5">
                        <span className="truncate font-mono text-[11px] font-semibold text-text-primary">
                          {r.display_name}
                        </span>
                        <span className="rounded border border-border-subtle bg-bg-elevated px-1 py-0.5 font-mono text-[9px] uppercase text-text-muted">
                          {r.read_only ? 'read_only' : r.access_mode}
                        </span>
                      </div>
                      <div className="truncate font-mono text-[10px] text-text-muted">
                        {r.resolved_root ?? r.root_path ?? 'remote/unresolved'}
                      </div>
                    </div>
                    <div className="flex items-center gap-1.5">
                      <button
                        type="button"
                        data-testid={`repo-switch-btn-${targetKey}`}
                        disabled={busyAction !== null}
                        onClick={() =>
                          void runCatalogAction('switch', `Switch to ${r.display_name}`, { target: targetKey })
                        }
                        className="rounded border border-brass/50 bg-brass/15 px-2 py-0.5 font-mono text-[10px] text-text-primary hover:bg-brass/25 disabled:opacity-50"
                      >
                        {isActive ? 'Active' : 'Switch'}
                      </button>
                      <button
                        type="button"
                        data-testid={`repo-remove-btn-${targetKey}`}
                        disabled={busyAction !== null}
                        onClick={() =>
                          void runCatalogAction('remove', `Remove ${r.display_name}`, { target: targetKey })
                        }
                        className="rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[10px] text-text-muted hover:text-(--color-status-error) disabled:opacity-50"
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        <div
          data-testid="repo-onboarding-wizard"
          className="space-y-2.5 rounded-lg border border-border-subtle bg-bg-elevated p-3"
        >
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-secondary">
              Init / Add Repository Wizard
            </span>
            <button
              type="button"
              data-testid="repo-access-mode-toggle"
              aria-pressed={readOnlyMode}
              onClick={() => setReadOnlyMode((v) => !v)}
              className={`rounded border px-2 py-0.5 font-mono text-[10px] transition ${
                readOnlyMode
                  ? 'border-(--color-status-warn)/50 bg-(--color-status-warn)/12 text-(--color-status-warn)'
                  : 'border-brass/50 bg-brass/15 text-text-primary'
              }`}
            >
              {readOnlyMode
                ? 'Mode: Zero-Footprint Read-Only'
                : 'Mode: Two-Tier Hybrid (docs/wiki/)'}
            </button>
          </div>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <input
              type="text"
              aria-label="Target repository path"
              data-testid="repo-wizard-path-input"
              value={targetPath}
              onChange={(e) => setTargetPath(e.target.value)}
              placeholder="Path (blank = current workspace)"
              className="rounded border border-border-subtle bg-bg-surface px-2.5 py-1.5 font-mono text-[11px] text-text-primary placeholder:text-text-muted"
            />
            <input
              type="text"
              aria-label="Repository display name"
              data-testid="repo-wizard-name-input"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="Display name (optional)"
              className="rounded border border-border-subtle bg-bg-surface px-2.5 py-1.5 font-mono text-[11px] text-text-primary placeholder:text-text-muted"
            />
          </div>
          <div className="flex flex-wrap items-center gap-1.5 pt-1">
            <button
              type="button"
              data-testid="repo-onboard-init-btn"
              disabled={busyAction !== null}
              onClick={() =>
                void runCatalogAction('init', 'Onboard Repository Harness', {
                  path: targetPath.trim() || undefined,
                  read_only: readOnlyMode,
                })
              }
              className="rounded border border-brass/50 bg-brass/15 px-2.5 py-1 font-mono text-[10px] font-semibold text-text-primary hover:bg-brass/25 disabled:opacity-50"
            >
              Onboard & Scaffold
            </button>
            <button
              type="button"
              data-testid="repo-onboard-add-btn"
              disabled={busyAction !== null || !targetPath.trim()}
              onClick={() =>
                void runCatalogAction('add', 'Add Repository to Catalog', {
                  path: targetPath.trim(),
                  display_name: displayName.trim() || undefined,
                  read_only: readOnlyMode,
                })
              }
              className="rounded border border-border-subtle bg-bg-surface px-2.5 py-1 font-mono text-[10px] text-text-primary hover:border-brass/50 disabled:opacity-50"
            >
              Add to Catalog
            </button>
            <button
              type="button"
              data-testid="repo-wiki-init-btn"
              disabled={busyAction !== null}
              onClick={() =>
                void runCatalogAction('wiki_init', 'Build 5-Layer OpenWiki', {
                  path: targetPath.trim() || undefined,
                  read_only: readOnlyMode,
                })
              }
              className="rounded border border-border-subtle bg-bg-surface px-2.5 py-1 font-mono text-[10px] text-text-primary hover:border-brass/50 disabled:opacity-50"
            >
              Build 5-Layer OpenWiki
            </button>
            <button
              type="button"
              data-testid="repo-wiki-heal-btn"
              disabled={busyAction !== null}
              onClick={() =>
                void runCatalogAction('wiki_heal_tier1', 'Auto-Heal Tier-1 OpenWiki', {
                  path: targetPath.trim() || undefined,
                  read_only: readOnlyMode,
                })
              }
              className="rounded border border-(--color-status-pass)/45 bg-(--color-status-pass)/12 px-2.5 py-1 font-mono text-[10px] text-(--color-status-pass) hover:bg-(--color-status-pass)/20 disabled:opacity-50"
            >
              Auto-Heal Tier-1
            </button>
          </div>
        </div>
      </div>
    </BentoCard>
  );
}
