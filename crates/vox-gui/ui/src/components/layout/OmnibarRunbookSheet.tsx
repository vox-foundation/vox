import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { TermRunbookDto } from '../../transport';
import {
  getTerminalBridgeState,
  requestConsoleBlockFocus,
  requestOpenRunbook,
  requestTerminalCommand,
} from '../../lib/terminalBridge';
import {
  getMissingRequiredParams,
  interpolateRunbookCommand,
  loadWorkspaceRunbooks,
} from '../surfaces/Console/RunbookPanel';
import { TERMINAL_MONO_FONT_STACK } from '../surfaces/Console/terminalTheme';
import { Button } from '../ui/Button';
import { Icon } from '../ui/Icons';
import { VoxSelect } from '../ui/Select';
import { parsePaletteQuery } from './paletteSources';

const SHEET_MONO_STYLE: React.CSSProperties = {
  fontFamily: TERMINAL_MONO_FONT_STACK,
  fontVariantLigatures: 'none',
  fontFeatureSettings: '"calt" 0, "liga" 0',
  letterSpacing: '0px',
};

interface OmnibarRunbookHandlers {
  onNavigate: (viewKey: string, anchorId?: string) => void;
  onClose: () => void;
}

export function useOmnibarRunbooks(open: boolean, query: string, { onNavigate, onClose }: OmnibarRunbookHandlers) {
  const [runbooks, setRunbooks] = useState<TermRunbookDto[]>([]);
  const [pendingRunbook, setPendingRunbook] = useState<TermRunbookDto | null>(null);

  useEffect(() => {
    if (!open) {
      setPendingRunbook(null);
      return;
    }
    let active = true;
    const cwd = getTerminalBridgeState().cwd ?? undefined;
    void loadWorkspaceRunbooks(cwd).then((items) => { if (active) setRunbooks(items); });
    return () => { active = false; };
  }, [open]);

  const matchingRunbooks = useMemo(() => {
    const { mode, query: effectiveQ } = parsePaletteQuery(query);
    if (mode !== 'default' && mode !== 'commands') return [];
    const ql = effectiveQ.trim().toLowerCase();
    if (!ql) return mode === 'default' ? runbooks : [];
    if (ql === 'runbook' || ql === 'runbooks') return runbooks;
    return runbooks.filter(
      (rb) =>
        rb.title.toLowerCase().includes(ql) ||
        rb.path.toLowerCase().includes(ql) ||
        rb.description.toLowerCase().includes(ql) ||
        rb.tags.some((t) => t.toLowerCase().includes(ql)),
    );
  }, [runbooks, query]);

  const selectOmnibarRunbook = useCallback(
    (rb: TermRunbookDto) => {
      if (rb.params.length === 0) {
        onNavigate('console');
        requestTerminalCommand(rb.command_template);
        requestConsoleBlockFocus({ sessionId: null });
        onClose();
      } else {
        setPendingRunbook(rb);
      }
    },
    [onNavigate, onClose],
  );

  return { matchingRunbooks, pendingRunbook, setPendingRunbook, selectOmnibarRunbook };
}

export interface OmnibarRunbooksSectionProps {
  runbooks: TermRunbookDto[];
  onSelectRunbook: (runbook: TermRunbookDto) => void;
  startIndex?: number;
  selectedIdx?: number;
  onFocusIndex?: (index: number) => void;
}

export function OmnibarRunbooksSection({
  runbooks,
  onSelectRunbook,
  startIndex = 0,
  selectedIdx = -1,
  onFocusIndex,
}: OmnibarRunbooksSectionProps) {
  if (runbooks.length === 0) return null;
  return (
    <div>
      <div className="mt-2 flex items-center justify-between border-b border-border-subtle px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
        <span>Runbooks</span>
        <span className="rounded border border-border-subtle bg-overlay-subtle px-2 py-1 text-[11px] font-semibold uppercase text-text-muted">runbook</span>
      </div>
      {runbooks.map((rb, i) => {
        const navIdx = startIndex + i;
        const selected = navIdx === selectedIdx;
        return (
          <button
            key={rb.id}
            type="button"
            data-row="true"
            data-testid={`omnibar-runbook-row-${rb.id}`}
            aria-current={selected ? 'true' : undefined}
            onMouseEnter={() => onFocusIndex?.(navIdx)}
            onFocus={() => onFocusIndex?.(navIdx)}
            onClick={() => onSelectRunbook(rb)}
            className={`flex w-full items-center justify-between gap-2 rounded-lg px-3 py-2 text-left transition-colors ${
              selected ? 'bg-overlay-subtle ring-1 ring-brass/40' : 'hover:bg-overlay-subtle'
            }`}
          >
            <div className="flex min-w-0 items-center gap-2">
              <Icon.book className="size-4 shrink-0 text-accent" aria-hidden="true" />
              <div className="flex min-w-0 flex-col">
                <div className="flex items-center gap-2">
                  <span className="max-w-[360px] truncate text-sm leading-[1.3] font-semibold text-text-primary">{rb.title}</span>
                  <span className={`shrink-0 rounded border border-border-subtle px-2 py-1 text-[11px] leading-[1.25] font-semibold uppercase ${rb.kind === 'vox' ? 'bg-brass/15 text-accent' : 'bg-bg-elevated text-text-secondary'}`}>
                    {rb.kind === 'vox' ? '.vox' : 'MD'}
                  </span>
                </div>
                <span className="max-w-[460px] truncate text-[11px] leading-[1.25] font-normal text-text-muted">{rb.path}</span>
              </div>
            </div>
            <span className="shrink-0 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
              {rb.params.length > 0 ? 'params' : 'runs'}
            </span>
          </button>
        );
      })}
    </div>
  );
}

export interface OmnibarRunbookParamsSheetProps {
  runbook: TermRunbookDto;
  onNavigate: (viewKey: string, anchorId?: string) => void;
  onClose: () => void;
  onBack: () => void;
}

export function OmnibarRunbookParamsSheet({ runbook, onNavigate, onClose, onBack }: OmnibarRunbookParamsSheetProps) {
  const [values, setValues] = useState<Record<string, string>>({});
  const firstInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    setValues({});
    firstInputRef.current?.focus();
  }, [runbook.id]);

  const missingParams = useMemo(() => getMissingRequiredParams(runbook.params, values), [runbook.params, values]);
  const missingNames = useMemo(() => new Set(missingParams.map((p) => p.name)), [missingParams]);

  const interpolated = useMemo(
    () => interpolateRunbookCommand(runbook.command_template, values, runbook.params),
    [runbook, values],
  );

  const handleInsert = () => {
    if (missingParams.length > 0) return;
    onNavigate('console');
    requestOpenRunbook({ runbookId: runbook.id, command: interpolated, mode: 'insert' });
    requestConsoleBlockFocus({ sessionId: null });
    onClose();
  };

  const handleRun = () => {
    if (missingParams.length > 0) return;
    onNavigate('console');
    requestTerminalCommand(interpolated);
    requestConsoleBlockFocus({ sessionId: null });
    onClose();
  };

  return (
    <div data-testid="omnibar-runbook-params" className="flex flex-col gap-2 px-4 py-2">
      <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-2">
        <div className="flex min-w-0 flex-col gap-1">
          <span className="truncate text-sm leading-[1.3] font-semibold text-text-primary">{runbook.title}</span>
          <span className="truncate text-[11px] leading-[1.25] font-normal text-text-muted">{runbook.path}</span>
        </div>
        <Button size="xs" variant="ghost" onClick={onBack} className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold">Back</Button>
      </div>

      {runbook.params.map((param, idx) => {
        const currentVal = values[param.name] ?? param.default_value ?? '';
        const isMissing = missingNames.has(param.name);
        return (
          <div key={param.name} className="flex flex-col gap-1">
            <label className="text-[11px] leading-[1.25] font-semibold text-text-secondary">{param.name}</label>
            {param.options.length > 0 ? (
              <VoxSelect
                size="xs" data-testid={`omnibar-runbook-param-${param.name}`} aria-label={`Parameter ${param.name}`}
                value={currentVal} onValueChange={(next) => setValues((prev) => ({ ...prev, [param.name]: next }))}
                options={param.options.map((opt) => ({ value: opt, label: opt }))} className="h-6 px-2 py-1 text-[11px] font-normal"
              />
            ) : (
              <input
                ref={idx === 0 ? firstInputRef : undefined} type="text"
                data-testid={`omnibar-runbook-param-${param.name}`} aria-label={`Parameter ${param.name}`}
                aria-invalid={isMissing || undefined}
                value={currentVal} onChange={(e) => setValues((prev) => ({ ...prev, [param.name]: e.target.value }))}
                onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); handleRun(); } }}
                placeholder={param.default_value ?? param.name}
                className={`h-8 w-full rounded border bg-bg-base px-2 py-1 text-xs font-normal text-text-primary placeholder:text-text-muted outline-none focus:border-border-strong ${isMissing ? 'border-(--color-status-fail)' : 'border-border-subtle'}`}
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

      <pre style={SHEET_MONO_STYLE} className="whitespace-pre-wrap break-words rounded border border-border-subtle bg-bg-base p-2 text-xs leading-[1.45] font-normal text-text-primary">
        {interpolated}
      </pre>

      <div className="flex items-center justify-end gap-2">
        <Button size="xs" variant="secondary" data-testid="term-runbook-insert" aria-label={`Insert into prompt ${runbook.title}`} disabled={missingParams.length > 0} onClick={handleInsert} className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold">
          Insert into Prompt
        </Button>
        <Button size="xs" variant="primary" data-testid="term-runbook-run" aria-label={`Run runbook ${runbook.title}`} disabled={missingParams.length > 0} onClick={handleRun} className="h-6 rounded px-2 py-1 text-[11px] font-semibold">
          Run Runbook
        </Button>
      </div>
    </div>
  );
}
