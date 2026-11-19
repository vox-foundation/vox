import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Icon } from '../ui/Icons';
import { voxTransport } from '../../transport';
import { unwrapMcpEnvelope, type McpInvokeResult } from '../../lib/mcpToolResult';
import { BackendUnavailableError, sanitizeErrorForToast } from '../../lib/backendGuard';

export interface ActiveRepoBadgeSummary {
  displayName: string;
  repositoryId?: string;
  readOnly?: boolean;
}

interface CatalogItemDto {
  repository_id?: string | null;
  display_name: string;
  root_path?: string | null;
  read_only?: boolean;
  access_mode?: string;
}

export interface RepoSwitcherPillProps {
  activeRepoSummary?: ActiveRepoBadgeSummary | null;
  onNavigate: (view: string) => void;
  onRepoSwitched?: (summary: ActiveRepoBadgeSummary) => void;
}

export function RepoSwitcherPill({
  activeRepoSummary = null,
  onNavigate,
  onRepoSwitched,
}: RepoSwitcherPillProps) {
  const [open, setOpen] = useState(false);
  const [summary, setSummary] = useState<ActiveRepoBadgeSummary>(
    activeRepoSummary ?? { displayName: 'vox', readOnly: false },
  );
  const [repos, setRepos] = useState<CatalogItemDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (activeRepoSummary) {
      setSummary(activeRepoSummary);
    }
  }, [activeRepoSummary]);

  const loadRepoCatalog = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const [stRes, catRes] = await Promise.all([
        voxTransport.invokeMcpTool('vox_repo_status', {}),
        voxTransport.invokeMcpTool('vox_repo_catalog_list', {}),
      ]);
      const st = unwrapMcpEnvelope((stRes as McpInvokeResult | undefined)?.result) as
        | {
            repository_id?: string;
            root?: string;
            active_context?: { display_name: string; repository_id: string; read_only: boolean };
          }
        | undefined;
      if (st && typeof st === 'object') {
        const displayName =
          st.active_context?.display_name ??
          st.root?.split(/[\\/]/).filter(Boolean).pop() ??
          st.repository_id ??
          summary.displayName;
        setSummary({
          displayName,
          repositoryId: st.active_context?.repository_id ?? st.repository_id,
          readOnly: st.active_context?.read_only ?? false,
        });
      }
      const cat = unwrapMcpEnvelope((catRes as McpInvokeResult | undefined)?.result) as
        | { repositories?: CatalogItemDto[] }
        | undefined;
      if (cat && Array.isArray(cat.repositories)) {
        setRepos(cat.repositories);
      }
    } catch (err) {
      if (!(err instanceof BackendUnavailableError) && !(err instanceof Error && err.name === 'BackendUnavailableError')) {
        setErrorMsg(sanitizeErrorForToast(err));
      }
    } finally {
      setLoading(false);
    }
  }, [summary.displayName]);

  useEffect(() => {
    if (!open) return;
    void loadRepoCatalog();
    const onOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (panelRef.current?.contains(target)) return;
      if (triggerRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener('mousedown', onOutside);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onOutside);
      document.removeEventListener('keydown', onKey);
    };
  }, [open, loadRepoCatalog]);

  const handleSwitch = async (item: CatalogItemDto) => {
    const target = item.repository_id ?? item.display_name;
    setLoading(true);
    setErrorMsg(null);
    try {
      await voxTransport.invokeMcpTool('vox_repo_catalog_refresh', {
        action: 'switch',
        target,
      });
      const next: ActiveRepoBadgeSummary = {
        displayName: item.display_name,
        repositoryId: item.repository_id ?? undefined,
        readOnly: item.read_only ?? item.access_mode === 'read_only_local',
      };
      setSummary(next);
      onRepoSwitched?.(next);
      setOpen(false);
    } catch (err) {
      setErrorMsg(sanitizeErrorForToast(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="relative shrink-0">
      <button
        ref={triggerRef}
        type="button"
        data-testid="bottom-status-bar-repo"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={`Workspace repository: ${summary.displayName}`}
        onClick={() => setOpen((v) => !v)}
        className="inline-flex items-center gap-1 rounded-sm border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[10px] text-text-secondary hover:border-brass/50 hover:text-text-primary transition"
      >
        <Icon.branch className="size-3 shrink-0 text-brass" aria-hidden="true" />
        <span className="max-w-[14ch] truncate">{summary.displayName}</span>
        <span className="rounded bg-bg-elevated px-1 text-[9px] uppercase text-text-muted">
          {summary.readOnly ? 'read-only' : 'hybrid'}
        </span>
      </button>

      {open && (
        <div
          ref={panelRef}
          role="dialog"
          aria-label="Workspace repository switcher"
          data-testid="bottom-status-bar-repo-popover"
          className="absolute bottom-full right-0 z-(--z-popover) mb-1 w-72 rounded-lg border border-border-subtle bg-bg-base p-2.5 shadow-2xl"
        >
          <div className="flex items-center justify-between border-b border-border-subtle pb-1.5">
            <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-primary">
              Active Repository
            </span>
            <span className="font-mono text-[10px] text-brass">
              {summary.readOnly ? 'read_only' : 'hybrid'}
            </span>
          </div>

          {errorMsg && (
            <div
              role="alert"
              className="mt-2 rounded border border-(--color-status-error)/40 bg-(--color-status-error)/10 px-2 py-1 font-mono text-[10px] text-(--color-status-error)"
            >
              {errorMsg}
            </div>
          )}

          <div className="mt-2 max-h-48 space-y-1 overflow-y-auto">
            {loading && repos.length === 0 ? (
              <div className="py-2 text-center font-mono text-[10px] text-text-muted">
                Loading repositories...
              </div>
            ) : repos.length === 0 ? (
              <div className="py-2 text-center font-mono text-[10px] text-text-muted">
                {summary.displayName} (current workspace)
              </div>
            ) : (
              repos.map((r, idx) => {
                const isCurrent =
                  (r.repository_id != null && r.repository_id === summary.repositoryId) ||
                  r.display_name === summary.displayName;
                return (
                  <button
                    key={`${r.repository_id ?? r.display_name}-${idx}`}
                    type="button"
                    data-testid={`repo-switcher-item-${r.repository_id ?? r.display_name}`}
                    onClick={() => void handleSwitch(r)}
                    className={`flex w-full items-center justify-between gap-2 rounded border px-2 py-1 text-left font-mono text-[10px] transition ${
                      isCurrent
                        ? 'border-brass/60 bg-brass/12 text-text-primary'
                        : 'border-border-subtle bg-bg-surface text-text-secondary hover:border-brass/40 hover:text-text-primary'
                    }`}
                  >
                    <span className="truncate">{r.display_name}</span>
                    <span className="shrink-0 text-[9px] uppercase text-text-muted">
                      {r.read_only ? 'ro' : 'hybrid'}
                    </span>
                  </button>
                );
              })
            )}
          </div>

          <button
            type="button"
            data-testid="repo-switcher-open-studio-btn"
            onClick={() => {
              setOpen(false);
              onNavigate('repository');
            }}
            className="mt-2 w-full rounded border border-brass/50 bg-brass/15 px-2 py-1 font-mono text-[10px] font-semibold text-text-primary hover:bg-brass/25"
          >
            Init / Manage Repositories
          </button>
        </div>
      )}
    </div>
  );
}
