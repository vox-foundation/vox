import React, { useEffect, useMemo, useState } from 'react';
import { BackendUnavailableError, sanitizeErrorForToast } from '../../../lib/backendGuard';
import * as transport from '../../../transport';
import type { TermRunbookDto, TermRunbookParamDto } from '../../../transport';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';
import { TERMINAL_MONO_STYLE } from './terminalTheme';

const SAFE_BARE_PARAM_RE = /^[A-Za-z0-9_./:@=-]+$/;
const TEMPLATE_PARAM_RE = /\{\{\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s*=\s*([^}]*?))?\s*\}\}/g;

export interface RunbookPanelProps {
  cwd?: string;
  selectedRunbookId?: string | null;
  onInsert: (command: string) => void;
  onRun: (command: string) => void;
}

export function quoteShellParam(raw: string): string {
  if (raw === '') return "''";
  return SAFE_BARE_PARAM_RE.test(raw) ? raw : `'${raw.replace(/'/g, "'\\''")}'`;
}

export function getMissingRequiredParams(
  params: TermRunbookParamDto[],
  values: Record<string, string>,
): TermRunbookParamDto[] {
  return params.filter((p) => (values[p.name] ?? p.default_value ?? '').trim().length === 0);
}

export function interpolateRunbookCommand(
  template: string,
  values: Record<string, string>,
  params: TermRunbookParamDto[],
): string {
  const defaults = new Map<string, string>();
  for (const p of params) {
    if (p.default_value != null) defaults.set(p.name, p.default_value);
  }
  return template.replace(TEMPLATE_PARAM_RE, (_full, name: string, inlineDef?: string) => {
    const resolved = values[name] ?? defaults.get(name) ?? inlineDef?.trim() ?? '';
    return quoteShellParam(resolved);
  });
}

function resolveRunbooksListFn(): ((cwdArg?: string) => Promise<TermRunbookDto[] | null>) | null {
  const mod = transport as Record<string, unknown>;
  try {
    const vt = mod.voxTransport as { termRunbooksList?: (cwdArg?: string) => Promise<TermRunbookDto[] | null> } | undefined;
    if (typeof vt?.termRunbooksList === 'function') return vt.termRunbooksList.bind(vt);
  } catch { /* partial module mock without voxTransport */ }
  try {
    const direct = mod.termRunbooksList as ((cwdArg?: string) => Promise<TermRunbookDto[] | null>) | undefined;
    if (typeof direct === 'function') return direct;
  } catch { /* partial module mock without termRunbooksList */ }
  return null;
}

export async function loadWorkspaceRunbooksWithStatus(
  cwd?: string,
): Promise<{ items: TermRunbookDto[]; error: string | null }> {
  const fn = resolveRunbooksListFn();
  if (!fn) return { items: [], error: null };
  try {
    const list = await fn(cwd);
    return { items: Array.isArray(list) ? list : [], error: null };
  } catch (err) {
    if (err instanceof BackendUnavailableError || (err instanceof Error && err.name === 'BackendUnavailableError')) {
      return { items: [], error: null };
    }
    return { items: [], error: sanitizeErrorForToast(err).replace(/\u2014/g, '-') };
  }
}

export async function loadWorkspaceRunbooks(cwd?: string): Promise<TermRunbookDto[]> {
  const { items } = await loadWorkspaceRunbooksWithStatus(cwd);
  return items;
}

export function RunbookPanel({ cwd, selectedRunbookId, onInsert, onRun }: RunbookPanelProps) {
  const [runbooks, setRunbooks] = useState<TermRunbookDto[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [reloadSeq, setReloadSeq] = useState(0);
  const [filterQuery, setFilterQuery] = useState('');
  const [expandedId, setExpandedId] = useState<string | null>(selectedRunbookId ?? null);
  const [paramValuesByRunbook, setParamValuesByRunbook] = useState<Record<string, Record<string, string>>>({});

  useEffect(() => {
    if (selectedRunbookId) setExpandedId(selectedRunbookId);
  }, [selectedRunbookId]);

  useEffect(() => {
    let active = true;
    void loadWorkspaceRunbooksWithStatus(cwd).then(({ items, error }) => {
      if (!active) return;
      setRunbooks(items);
      setScanError(error);
      setLoaded(true);
    });
    return () => { active = false; };
  }, [cwd, reloadSeq]);

  const filteredRunbooks = useMemo(() => {
    const q = filterQuery.trim().toLowerCase();
    if (!q) return runbooks;
    return runbooks.filter(
      (rb) =>
        rb.title.toLowerCase().includes(q) ||
        rb.description.toLowerCase().includes(q) ||
        rb.path.toLowerCase().includes(q) ||
        rb.tags.some((t) => t.toLowerCase().includes(q)),
    );
  }, [runbooks, filterQuery]);

  const activeExpandedId = useMemo(
    () => (expandedId && filteredRunbooks.some((rb) => rb.id === expandedId) ? expandedId : (filteredRunbooks[0]?.id ?? null)),
    [expandedId, filteredRunbooks],
  );

  const setParamValue = (runbookId: string, paramName: string, nextValue: string) => {
    setParamValuesByRunbook((prev) => ({
      ...prev,
      [runbookId]: { ...(prev[runbookId] ?? {}), [paramName]: nextValue },
    }));
  };

  return (
    <div data-testid="term-runbook-panel" className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex h-8 items-center gap-2 rounded-md border border-border-subtle bg-bg-base px-2">
        <Icon.search className="size-4 shrink-0 text-text-muted" aria-hidden="true" />
        <input
          type="search" data-slot="inline-search-input" data-testid="term-runbook-filter" value={filterQuery}
          onChange={(e) => setFilterQuery(e.target.value)}
          placeholder="Filter runbooks..." aria-label="Filter runbooks"
          className="h-6 min-w-0 flex-1 bg-transparent text-xs font-normal text-text-primary placeholder:text-text-muted outline-none"
        />
        {filterQuery.length > 0 && (
          <Button size="xs" variant="ghost" aria-label="Clear filter" onClick={() => setFilterQuery('')} className="h-6 px-2 py-1 text-[11px] font-semibold">
            Clear Filter
          </Button>
        )}
      </div>

      {scanError ? (
        <div role="alert" data-testid="term-runbook-error" className="flex flex-col gap-2 rounded-lg border border-(--color-status-fail) bg-bg-elevated p-3 text-xs">
          <span className="font-normal text-(--color-status-fail)">
            {`Runbook scan failed: ${scanError}. Click Reload Runbooks to retry.`}
          </span>
          <Button
            size="xs"
            variant="secondary"
            data-testid="term-runbook-reload"
            aria-label="Reload Runbooks"
            onClick={() => { setLoaded(false); setReloadSeq((n) => n + 1); }}
            className="h-6 self-start rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold"
          >
            Reload Runbooks
          </Button>
        </div>
      ) : loaded && runbooks.length === 0 ? (
        <div className="flex flex-col gap-2 rounded-lg border border-border-subtle bg-bg-elevated p-4 text-xs">
          <div className="text-sm leading-[1.3] font-semibold text-text-primary">No workspace runbooks discovered</div>
          <p className="text-xs leading-[1.45] font-normal text-text-secondary">
            Add a .vox script under scripts/ or .vox/runbooks/, or tag a guide in docs/src/how-to/ with @runbook.
          </p>
        </div>
      ) : filteredRunbooks.length === 0 && filterQuery.length > 0 ? (
        <div className="flex flex-col gap-2 rounded-lg border border-border-subtle bg-bg-elevated p-3 text-xs text-text-muted">
          <span>{`0 runbooks match "${filterQuery}" - click Clear Filter to show all runbooks.`}</span>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
          {filteredRunbooks.map((rb) => {
            const isExpanded = rb.id === activeExpandedId;
            const currentValues = paramValuesByRunbook[rb.id] ?? {};
            const missingParams = getMissingRequiredParams(rb.params, currentValues);
            const missingNames = new Set(missingParams.map((p) => p.name));
            const interpolated = interpolateRunbookCommand(rb.command_template, currentValues, rb.params);
            return (
              <div key={rb.id} data-testid={`term-runbook-card-${rb.id}`} className="flex flex-col gap-2 rounded-lg border border-border-subtle bg-bg-elevated p-2">
                <button
                  type="button" aria-expanded={isExpanded} aria-label={`${rb.title} runbook details`}
                  onClick={() => setExpandedId(rb.id)} className="flex w-full items-start justify-between gap-2 text-left"
                >
                  <div className="flex min-w-0 flex-col gap-1">
                    <span className="truncate text-sm leading-[1.3] font-semibold text-text-primary">{rb.title}</span>
                    <span className="truncate text-[11px] leading-[1.25] font-normal text-text-muted">{rb.path}</span>
                  </div>
                  <span className={`shrink-0 rounded border border-border-subtle px-2 py-1 text-[11px] leading-[1.25] font-semibold uppercase ${rb.kind === 'vox' ? 'bg-brass/15 text-accent' : 'bg-bg-surface text-text-secondary'}`}>
                    {rb.kind === 'vox' ? '.vox' : 'MD'}
                  </span>
                </button>

                {isExpanded && (
                  <div className="flex flex-col gap-2 border-t border-border-subtle pt-2">
                    {rb.description && <p className="text-xs leading-[1.45] font-normal text-text-secondary">{rb.description}</p>}
                    {rb.params.map((param) => {
                      const val = currentValues[param.name] ?? param.default_value ?? '';
                      const isMissing = missingNames.has(param.name);
                      return (
                        <div key={param.name} className="flex flex-col gap-1">
                          <label className="text-[11px] leading-[1.25] font-semibold text-text-secondary">{param.name}</label>
                          {param.options.length > 0 ? (
                            <VoxSelect
                              size="xs" data-testid={`term-runbook-param-${param.name}`} aria-label={`Parameter ${param.name}`}
                              value={val} onValueChange={(next) => setParamValue(rb.id, param.name, next)}
                              options={param.options.map((opt) => ({ value: opt, label: opt }))} className="h-6 px-2 py-1 text-[11px] font-normal"
                            />
                          ) : (
                            <input
                              type="text" data-testid={`term-runbook-param-${param.name}`} aria-label={`Parameter ${param.name}`}
                              aria-invalid={isMissing || undefined}
                              value={val} onChange={(e) => setParamValue(rb.id, param.name, e.target.value)}
                              placeholder={param.default_value ?? param.name}
                              className={`h-6 w-full rounded border bg-bg-base px-2 py-1 text-xs font-normal text-text-primary placeholder:text-text-muted outline-none focus:border-border-strong ${isMissing ? 'border-(--color-status-fail)' : 'border-border-subtle'}`}
                            />
                          )}
                        </div>
                      );
                    })}
                    {missingParams.length > 0 && (
                      <span role="alert" data-testid="term-runbook-param-error" className="text-[11px] font-semibold text-(--color-status-fail)">
                        {`Fill required parameter ${missingParams[0].name} before running.`}
                      </span>
                    )}
                    <pre data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="whitespace-pre-wrap break-words rounded border border-border-subtle bg-bg-base p-2 text-xs leading-[1.45] font-normal text-text-primary">
                      {interpolated}
                    </pre>
                    <div className="flex flex-wrap items-center justify-end gap-2">
                      <Button size="xs" variant="secondary" data-testid="term-runbook-insert" aria-label={`Insert into prompt ${rb.title}`} disabled={missingParams.length > 0} onClick={() => { if (missingParams.length === 0) onInsert(interpolated); }} className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold">
                        Insert into Prompt
                      </Button>
                      <Button size="xs" variant="primary" data-testid="term-runbook-run" aria-label={`Run runbook ${rb.title}`} disabled={missingParams.length > 0} onClick={() => { if (missingParams.length === 0) onRun(interpolated); }} className="h-6 rounded px-2 py-1 text-[11px] font-semibold">
                        Run Runbook
                      </Button>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
