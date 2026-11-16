import React, { useEffect, useRef, useState } from 'react';
import * as transport from '../../../transport';
import type { TermHistoryEntryDto } from '../../../transport';
import { fuzzyScore } from '../../../lib/intellisenseEngine';
import { Icon, KeycapIcon } from '../../ui/Icons';
import { TERMINAL_MONO_FONT_STACK } from './terminalTheme';

const HISTORY_STORAGE_KEY = 'vox.console.history.redacted.v1';
const MAX_FALLBACK_ENTRIES = 200;
let memoryFallbackRing: TermHistoryEntryDto[] = [];

// eslint-disable-next-line no-control-regex
const ANSI_ESCAPE_RE = /\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g;
const SK_TOKEN_RE = /\bsk-[A-Za-z0-9_-]{16,}\b/g;
const GH_TOKEN_RE = /\bgh[pousr]_[A-Za-z0-9]{20,}\b/g;
const AIZA_TOKEN_RE = /\bAIza[0-9A-Za-z_-]{20,}\b/g;
const BEARER_TOKEN_RE = /Bearer\s+[A-Za-z0-9._-]+/gi;
const ENV_SECRET_RE =
  /\b([A-Z0-9_]*(?:SECRET|TOKEN|PASSWORD|API_KEY|PRIVATE_KEY)[A-Z0-9_]*)=([^\s"']+|"[^"]*"|'[^']*')/gi;

function getTransportFn<K extends keyof typeof transport>(key: K): (typeof transport)[K] | undefined {
  try {
    const fn = transport[key];
    return typeof fn === 'function' ? fn : undefined;
  } catch {
    return undefined;
  }
}

function getUsableLocalStorage(): Storage | null {
  try {
    const ls = typeof window !== 'undefined' ? window.localStorage : undefined;
    if (ls && typeof ls.getItem === 'function' && typeof ls.setItem === 'function' && typeof ls.removeItem === 'function') {
      return ls;
    }
  } catch {
    // Restricted storage context
  }
  return null;
}

function readFallbackRing(): TermHistoryEntryDto[] {
  const store = getUsableLocalStorage();
  if (!store) return [...memoryFallbackRing];
  try {
    const raw = store.getItem(HISTORY_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((r): r is TermHistoryEntryDto => Boolean(r && typeof r.command === 'string' && typeof r.ts_ms === 'number'))
      : [];
  } catch {
    return [...memoryFallbackRing];
  }
}

function writeFallbackRing(entries: TermHistoryEntryDto[]): void {
  const capped = entries.slice(0, MAX_FALLBACK_ENTRIES);
  memoryFallbackRing = capped;
  const store = getUsableLocalStorage();
  if (!store) return;
  try {
    store.setItem(HISTORY_STORAGE_KEY, JSON.stringify(capped));
  } catch {
    // Ignore storage quota errors
  }
}

export function scrubHistorySecrets(rawCommand: string): string {
  return rawCommand
    .replace(ANSI_ESCAPE_RE, '')
    .replace(SK_TOKEN_RE, '[REDACTED]')
    .replace(GH_TOKEN_RE, '[REDACTED]')
    .replace(AIZA_TOKEN_RE, '[REDACTED]')
    .replace(BEARER_TOKEN_RE, 'Bearer [REDACTED]')
    .replace(ENV_SECRET_RE, '$1=[REDACTED]');
}

export function recordClientFallbackHistory(rawCommand: string, cwd?: string, exitCode?: number): void {
  if (!rawCommand || rawCommand.startsWith(' ') || rawCommand.startsWith('\t')) return;
  const scrubbed = scrubHistorySecrets(rawCommand).trim();
  if (!scrubbed) return;
  const existing = readFallbackRing().filter((item) => item.command !== scrubbed);
  writeFallbackRing([{ command: scrubbed, cwd: cwd ?? null, exit_code: exitCode ?? null, ts_ms: Date.now() }, ...existing]);
}

export function queryClientFallbackHistory(query = '', cwd?: string, limit = 50): TermHistoryEntryDto[] {
  const ring = readFallbackRing();
  const q = query.trim();
  const maxItems = Math.max(1, limit);
  if (!q) return [...ring].sort((a, b) => b.ts_ms - a.ts_ms).slice(0, maxItems);

  return ring
    .map((entry) => {
      const base = fuzzyScore(q, entry.command);
      return base > 0 ? { entry, score: base + (cwd && entry.cwd === cwd ? 25 : 0) } : null;
    })
    .filter((row): row is { entry: TermHistoryEntryDto; score: number } => row !== null)
    .sort((a, b) => b.score - a.score || b.entry.ts_ms - a.entry.ts_ms)
    .slice(0, maxItems)
    .map((row) => row.entry);
}

export function clearClientFallbackHistory(): void {
  memoryFallbackRing = [];
  const store = getUsableLocalStorage();
  if (!store) return;
  try {
    store.removeItem(HISTORY_STORAGE_KEY);
  } catch {
    // Ignore storage error
  }
}

function cwdBasename(cwd?: string | null): string | null {
  if (!cwd) return null;
  const parts = cwd.replace(/[\\/]+$/, '').split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? cwd;
}

function formatRelativeAge(tsMs: number): string {
  const diffSec = Math.max(0, Math.floor((Date.now() - tsMs) / 1000));
  if (diffSec < 60) return 'now';
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m`;
  if (diffSec < 86_400) return `${Math.floor(diffSec / 3600)}h`;
  return `${Math.floor(diffSec / 86_400)}d`;
}

function renderFuzzyPart(text: string, query: string, keyPrefix: string): React.ReactNode {
  const q = query.trim().toLowerCase();
  if (!q) return text;
  let qi = 0;
  return Array.from(text).map((ch, idx) => {
    if (qi < q.length && ch.toLowerCase() === q[qi]) {
      qi++;
      return <span key={`${keyPrefix}-${idx}`} className="font-semibold text-accent">{ch}</span>;
    }
    return ch;
  });
}

function renderCommandWithHighlights(command: string, query: string): React.ReactNode {
  const parts = command.split(/(\[REDACTED\])/g);
  return parts.map((part, idx) =>
    part === '[REDACTED]' ? (
      <span
        key={`redacted-${idx}`}
        data-testid="term-history-redacted-badge"
        className="mx-1 rounded border border-(--color-status-warn) px-1 text-[11px] font-semibold text-(--color-status-warn)"
      >
        [REDACTED]
      </span>
    ) : (
      <React.Fragment key={`seg-${idx}`}>{renderFuzzyPart(part, query, `f-${idx}`)}</React.Fragment>
    ),
  );
}

export interface HistorySearchPopoverProps {
  open: boolean;
  cwd?: string;
  initialQuery?: string;
  onSelect: (command: string, action: 'accept' | 'insert') => void;
  onCancel: () => void;
  onCleared?: () => void;
}

export function HistorySearchPopover({
  open, cwd, initialQuery = '', onSelect, onCancel, onCleared,
}: HistorySearchPopoverProps) {
  const [query, setQuery] = useState(initialQuery);
  const [entries, setEntries] = useState<TermHistoryEntryDto[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [confirmingClear, setConfirmingClear] = useState(false);
  const [refreshTick, setRefreshTick] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const reqSeqRef = useRef(0);

  useEffect(() => {
    if (!open) return;
    setQuery(initialQuery);
    setActiveIndex(0);
    setConfirmingClear(false);
    const id = requestAnimationFrame(() => inputRef.current?.focus());
    inputRef.current?.focus();
    return () => cancelAnimationFrame(id);
  }, [open, initialQuery]);

  useEffect(() => {
    if (!open) return;
    const seq = ++reqSeqRef.current;
    const queryFn = getTransportFn('termHistoryQuery');
    void (async () => {
      const ipcRows = queryFn ? await Promise.resolve(queryFn(query, cwd, 50)).catch(() => null) : null;
      if (seq !== reqSeqRef.current) return;
      setEntries(Array.isArray(ipcRows) ? ipcRows : queryClientFallbackHistory(query, cwd, 50));
      setActiveIndex(0);
    })();
  }, [open, query, cwd, refreshTick]);

  if (!open) return null;
  const clampedIndex = Math.min(Math.max(0, activeIndex), Math.max(0, entries.length - 1));

  const handleConfirmClear = async () => {
    const clearFn = getTransportFn('termHistoryClear');
    if (clearFn) await Promise.resolve(clearFn()).catch(() => false);
    clearClientFallbackHistory();
    onCleared?.();
    setConfirmingClear(false);
    setEntries([]);
    setRefreshTick((n) => n + 1);
  };

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onCancel();
    } else if (e.key === 'ArrowDown' && entries.length > 0) {
      e.preventDefault();
      setActiveIndex((prev) => (prev + 1) % entries.length);
    } else if (e.key === 'ArrowUp' && entries.length > 0) {
      e.preventDefault();
      setActiveIndex((prev) => (prev - 1 + entries.length) % entries.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (entries[clampedIndex]) onSelect(entries[clampedIndex].command, 'accept');
    } else if (e.key === 'Tab' && !e.shiftKey && entries[clampedIndex]) {
      e.preventDefault();
      onSelect(entries[clampedIndex].command, 'insert');
    }
  };

  return (
    <div
      data-testid="term-history-search"
      role="region"
      aria-label="Command history search"
      className="absolute bottom-[calc(100%+8px)] left-0 z-50 flex max-h-[min(340px,calc(100vh-140px))] w-[560px] max-w-[calc(100vw-32px)] flex-col gap-2 overflow-hidden rounded-lg border border-border-strong bg-bg-surface p-2 shadow-lg"
    >
      <div className="flex items-center justify-between gap-2 border-b border-border-subtle px-3 py-1">
        <div className="flex items-center gap-2">
          <Icon.clock className="size-4 text-accent" aria-hidden="true" />
          <span className="text-sm font-semibold text-text-primary">Command History</span>
        </div>
        <div className="flex items-center gap-1">
          {confirmingClear ? (
            <>
              <span className="px-1 text-[11px] font-normal text-text-muted">
                Remove all scrubbed workspace command history entries?
              </span>
              <button
                type="button"
                data-testid="term-history-confirm-clear"
                aria-label="Confirm Clear History"
                onClick={() => void handleConfirmClear()}
                className="h-6 rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-(--color-status-fail) transition hover:bg-overlay-subtle"
              >
                Confirm Clear History
              </button>
              <button
                type="button"
                data-testid="term-history-keep"
                aria-label="Keep History"
                onClick={() => setConfirmingClear(false)}
                className="h-6 rounded px-2 py-1 text-[11px] font-semibold text-text-secondary transition hover:bg-overlay-subtle hover:text-text-primary"
              >
                Keep History
              </button>
            </>
          ) : (
            <button
              type="button"
              data-testid="term-history-clear"
              aria-label="Clear History"
              onClick={() => setConfirmingClear(true)}
              className="h-6 rounded px-2 py-1 text-[11px] font-semibold text-text-muted transition hover:bg-overlay-subtle hover:text-text-primary"
            >
              Clear History
            </button>
          )}
          <button
            type="button"
            data-testid="term-history-close"
            aria-label="Dismiss History"
            onClick={onCancel}
            className="inline-flex h-6 items-center gap-1 rounded px-2 py-1 text-[11px] font-semibold text-text-muted transition hover:bg-overlay-subtle hover:text-text-primary"
          >
            <Icon.close className="size-3" aria-hidden="true" />
            <span>Dismiss History</span>
          </button>
        </div>
      </div>

      <div className="flex items-center gap-2 rounded border border-border-subtle bg-bg-elevated px-2 py-1">
        <Icon.search className="size-4 shrink-0 text-text-muted" aria-hidden="true" />
        <input
          ref={inputRef}
          type="search"
          data-slot="inline-search-input"
          data-terminal-mono="true"
          data-testid="term-history-input"
          aria-label="Search command history"
          placeholder="Filter command history (workspace boosted)..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleInputKeyDown}
          autoComplete="off"
          spellCheck={false}
          style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}
          className="h-6 flex-1 bg-transparent text-xs font-normal text-text-primary placeholder:text-text-muted outline-none"
        />
      </div>

      {entries.length === 0 ? (
        <div className="flex flex-col gap-1 rounded border border-border-subtle bg-bg-elevated px-3 py-2">
          <span className="text-xs font-semibold text-text-primary">
            {query.trim().length === 0 ? 'No command history recorded yet' : 'No matching commands'}
          </span>
          <span className="text-xs font-normal text-text-muted">
            {query.trim().length === 0
              ? 'Commands executed in this workspace are saved automatically with secrets scrubbed.'
              : `0 matching commands for "${query.trim()}" - edit your filter or press Esc to return to the prompt.`}
          </span>
        </div>
      ) : (
        <div role="listbox" tabIndex={0} aria-label="Matching history commands" className="flex max-h-36 min-[1100px]:max-h-60 flex-col gap-1 overflow-y-auto focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring">
          {entries.map((entry, idx) => {
            const isActive = idx === clampedIndex;
            const folderLabel = cwdBasename(entry.cwd);
            return (
              <div
                key={`${entry.ts_ms}-${idx}`}
                role="option"
                tabIndex={-1}
                aria-selected={isActive}
                data-active={isActive ? 'true' : 'false'}
                data-testid={`term-history-item-${idx}`}
                onMouseEnter={() => setActiveIndex(idx)}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => onSelect(entry.command, 'accept')}
                className={`flex cursor-pointer items-center justify-between gap-2 rounded px-2 py-1 transition ${
                  isActive ? 'bg-overlay-subtle ring-1 ring-brass/40' : 'hover:bg-overlay-subtle/60'
                }`}
              >
                <span className="flex min-w-0 flex-1 items-center justify-between gap-2 text-left">
                  <span data-terminal-mono="true" style={{ fontFamily: TERMINAL_MONO_FONT_STACK }} className="truncate text-xs font-normal text-text-primary">
                    {renderCommandWithHighlights(entry.command, query)}
                  </span>
                  <span className="flex shrink-0 items-center gap-1">
                    {folderLabel && (
                      <span className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-muted">
                        <Icon.folder className="size-3" aria-hidden="true" />
                        <span>{folderLabel}</span>
                      </span>
                    )}
                    {entry.exit_code != null && (
                      <span className={`rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold ${entry.exit_code === 0 ? 'text-(--color-status-pass)' : 'text-(--color-status-fail)'}`}>
                        exit {entry.exit_code}
                      </span>
                    )}
                    <span className="rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-muted">
                      {formatRelativeAge(entry.ts_ms)}
                    </span>
                  </span>
                </span>
                <span
                  role="presentation"
                  data-testid={`term-history-insert-${idx}`}
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={(e) => {
                    e.stopPropagation();
                    onSelect(entry.command, 'insert');
                  }}
                  className="inline-flex h-6 shrink-0 cursor-pointer items-center rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-secondary transition hover:text-text-primary"
                >
                  Insert Command
                </span>
              </div>
            );
          })}
        </div>
      )}

      <div className="flex items-center justify-between gap-2 border-t border-border-subtle px-2 pt-1 text-[11px] font-semibold text-text-muted">
        <span className="inline-flex items-center gap-1">
          <KeycapIcon keyName="Up" className="size-3" />
          <KeycapIcon keyName="Down" className="size-3" />
          <span>Navigate - Enter Run - Tab Insert - Esc Dismiss</span>
        </span>
        <span>{entries.length} shown</span>
      </div>
    </div>
  );
}
