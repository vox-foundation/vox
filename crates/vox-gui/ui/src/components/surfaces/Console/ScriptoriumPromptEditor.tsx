import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import * as transport from '../../../transport';
import { getTerminalBridgeState, subscribeTerminalBridge, type TermShellModeId, type TerminalBridgeState } from '../../../lib/terminalBridge';
import { Icon } from '../../ui/Icons';
import { IntelliSensePopover } from '../../ui/IntelliSensePopover';
import { VoxSelect } from '../../ui/Select';
import { HistorySearchPopover, queryClientFallbackHistory, recordClientFallbackHistory } from './HistorySearchPopover';
import { classifyPromptIntent, type ResolvedIntentKind, type ShellMode } from './intentRouter';
import { PROMPT_TOKEN_CLASSES, isIncompleteContinuation, tokenizePromptBuffer } from './promptHighlighter';
import { applyTerminalSuggestion, computeLocalTerminalCompletions, resolveTerminalCompletions, type TerminalCompletionsResult, type TerminalSuggestion } from './terminalCompletions';
import { TERMINAL_MONO_FONT_STACK, TERMINAL_MONO_STYLE } from './terminalTheme';

export interface ExternalDraftRequest {
  seq: number;
  value: string;
  mode?: 'insert' | 'replace';
}

export interface ScriptoriumPromptEditorProps {
  onSubmit: (raw: string) => void;
  onActiveSuggestion?: (actionId: string | null) => void;
  applyLine?: string | null;
  onApplyLineConsumed?: () => void;
  shellMode?: ShellMode;
  onShellModeChange?: ((mode: ShellMode) => void) | ((mode: TermShellModeId) => void);
  disabled?: boolean;
  knownSetKeys?: string[];
  sessionId?: string | null;
  cwd?: string;
  historyOpenRequestSeq?: number;
  externalDraftRequest?: ExternalDraftRequest | null;
}

const SHELL_MODE_OPTIONS = [
  { value: 'default', label: 'default' }, { value: 'nu', label: 'nu' }, { value: 'vox', label: 'vox' },
  { value: 'shell', label: 'shell' }, { value: 'agent', label: 'agent' }, { value: 'slash', label: 'slash' },
] as const;

const MODE_BADGE_TONES: Record<ResolvedIntentKind, string> = {
  vox: 'border-brass/50 bg-brass/15 text-accent',
  agent: 'border-(--color-status-warn) bg-overlay-subtle text-(--color-status-warn)',
  slash: 'border-brass/50 bg-brass/15 text-accent',
  nu: 'border-border-strong bg-bg-elevated text-text-primary',
  shell: 'border-(--color-status-pass)/50 bg-overlay-subtle text-(--color-status-pass)',
};

function getTransportFn<K extends keyof typeof transport>(key: K): (typeof transport)[K] | undefined {
  try { const fn = transport[key]; return typeof fn === 'function' ? fn : undefined; } catch { return undefined; }
}

export function ScriptoriumPromptEditor({
  onSubmit, onActiveSuggestion, applyLine, onApplyLineConsumed, shellMode: propShellMode,
  onShellModeChange, disabled = false, knownSetKeys, cwd, historyOpenRequestSeq, externalDraftRequest,
}: ScriptoriumPromptEditorProps) {
  const [internalShellMode, setInternalShellMode] = useState<ShellMode>('default');
  const [extendedModeOverride, setExtendedModeOverride] = useState<ShellMode | null>(null);
  const effectiveMode: ShellMode = extendedModeOverride ?? propShellMode ?? internalShellMode;

  const [value, setValue] = useState('');
  const [cursorOffset, setCursorOffset] = useState(0);
  const [editorHeightPx, setEditorHeightPx] = useState(24);
  const [activeIndex, setActiveIndex] = useState(0);
  const [completionNavigated, setCompletionNavigated] = useState(false);
  const [forceOpen, setForceOpen] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [historyPopoverOpen, setHistoryPopoverOpen] = useState(false);
  const [pastedLineCount, setPastedLineCount] = useState(0);
  const [historyRecallCursor, setHistoryRecallCursor] = useState(-1);
  const [termBridge, setTermBridge] = useState<TerminalBridgeState>(getTerminalBridgeState);
  const [asyncSnap, setAsyncSnap] = useState<{ line: string; cursor: number; mode: ShellMode; result: TerminalCompletionsResult } | null>(null);

  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const mirrorRef = useRef<HTMLDivElement | null>(null);
  const reqSeqRef = useRef(0);
  const stashedBeforeCtrlR = useRef('');
  const liveDraftRef = useRef('');
  const ipcHistoryCacheRef = useRef<string[]>([]);
  const lastHistorySeqRef = useRef(historyOpenRequestSeq ?? 0);
  const lastExternalSeqRef = useRef(externalDraftRequest?.seq ?? 0);

  useEffect(() => subscribeTerminalBridge(setTermBridge), []);
  useEffect(() => { setExtendedModeOverride(null); }, [propShellMode]);

  const refreshIpcHistoryCache = useCallback(async () => {
    const queryFn = getTransportFn('termHistoryQuery');
    if (!queryFn) return;
    const rows = await Promise.resolve(queryFn('', cwd, 50)).catch(() => null);
    if (Array.isArray(rows)) ipcHistoryCacheRef.current = rows.map((r) => r.command);
  }, [cwd]);

  useEffect(() => { void refreshIpcHistoryCache(); }, [refreshIpcHistoryCache]);

  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.setProperty('background-color', 'transparent', 'important');
    el.style.setProperty('border', 'none', 'important');
    el.style.setProperty('box-shadow', 'none', 'important');
    el.style.setProperty('color', value.length > 0 ? 'transparent' : 'var(--color-text-primary)', 'important');
    el.style.setProperty('caret-color', 'var(--color-text-primary)', 'important');
    el.style.setProperty('font-family', TERMINAL_MONO_FONT_STACK, 'important');
    el.style.height = 'auto';
    const next = Math.min(Math.max(el.scrollHeight || 24, 24), 160);
    el.style.height = `${next}px`;
    setEditorHeightPx(next);
  }, [value]);

  useEffect(() => {
    if (applyLine == null) return;
    setValue(applyLine); setCursorOffset(applyLine.length); setCompletionNavigated(false); setPastedLineCount(0);
    onApplyLineConsumed?.();
  }, [applyLine, onApplyLineConsumed]);

  useEffect(() => {
    if (!externalDraftRequest || externalDraftRequest.seq === lastExternalSeqRef.current) return;
    lastExternalSeqRef.current = externalDraftRequest.seq;
    setValue((prev) => {
      const next = externalDraftRequest.mode === 'insert' && prev ? `${prev}${externalDraftRequest.value}` : externalDraftRequest.value;
      setCursorOffset(next.length);
      return next;
    });
    setCompletionNavigated(false); setPastedLineCount(0);
  }, [externalDraftRequest]);

  useEffect(() => {
    if (historyOpenRequestSeq === undefined || historyOpenRequestSeq === lastHistorySeqRef.current) return;
    lastHistorySeqRef.current = historyOpenRequestSeq;
    if (historyOpenRequestSeq > 0) { stashedBeforeCtrlR.current = value; setDismissed(true); setHistoryPopoverOpen(true); }
  }, [historyOpenRequestSeq, value]);

  const getRecallHistoryList = useCallback((): string[] =>
    ipcHistoryCacheRef.current.length > 0 ? ipcHistoryCacheRef.current : queryClientFallbackHistory('', cwd, 50).map((r) => r.command),
  [cwd]);

  const localCompletion = useMemo(
    () => computeLocalTerminalCompletions(value, cursorOffset, { cwd, shellMode: effectiveMode, knownSetKeys, termBridge, forceOpen, historyCommands: getRecallHistoryList() }),
    [value, cursorOffset, cwd, effectiveMode, knownSetKeys, termBridge, forceOpen, getRecallHistoryList],
  );

  useEffect(() => {
    if (historyPopoverOpen || (!value.trim() && !forceOpen)) return;
    const seq = ++reqSeqRef.current;
    const timer = setTimeout(() => {
      void resolveTerminalCompletions(value, cursorOffset, { cwd, shellMode: effectiveMode, knownSetKeys, termBridge, forceOpen, historyCommands: getRecallHistoryList() }).then((result) => {
        if (seq === reqSeqRef.current) setAsyncSnap({ line: value, cursor: cursorOffset, mode: effectiveMode, result });
      });
    }, 10);
    return () => clearTimeout(timer);
  }, [value, cursorOffset, cwd, effectiveMode, knownSetKeys, termBridge, forceOpen, historyPopoverOpen, getRecallHistoryList]);

  const effectiveCompletion =
    asyncSnap && asyncSnap.line === value && asyncSnap.cursor === cursorOffset && asyncSnap.mode === effectiveMode
      ? asyncSnap.result
      : localCompletion;

  const suggestions = effectiveCompletion.items;
  const clampedIdx = Math.min(Math.max(0, activeIndex), Math.max(0, suggestions.length - 1));
  const activeItem: TerminalSuggestion | undefined = suggestions[clampedIdx] ?? suggestions[0];
  const ghostSuffix = effectiveCompletion.ghostSuffix;

  useEffect(() => {
    onActiveSuggestion?.(activeItem?.actionId ?? effectiveCompletion.activeActionId ?? null);
  }, [activeItem, effectiveCompletion.activeActionId, onActiveSuggestion]);

  const hasMenuSuggestion = suggestions.some((s) => s.badge !== 'HIST');
  const popoverOpen = !historyPopoverOpen && !dismissed && suggestions.length > 0 && (forceOpen || (value.trim().length > 0 && hasMenuSuggestion));
  const intent = useMemo(() => classifyPromptIntent(value, effectiveMode), [value, effectiveMode]);
  const tokens = useMemo(() => tokenizePromptBuffer(value), [value]);

  const updateBuffer = (nextValue: string, nextCursor = nextValue.length) => {
    setValue(nextValue); setCursorOffset(nextCursor); setCompletionNavigated(false); setDismissed(false); setActiveIndex(0);
    requestAnimationFrame(() => {
      if (textareaRef.current && document.activeElement === textareaRef.current) textareaRef.current.setSelectionRange(nextCursor, nextCursor);
    });
  };

  const acceptSuggestion = (item?: TerminalSuggestion) => {
    const target = item ?? activeItem;
    if (!target) return;
    const applied = applyTerminalSuggestion(value, target);
    setValue(applied.value); setCursorOffset(applied.cursor); setCompletionNavigated(false); setForceOpen(false); setDismissed(true);
    requestAnimationFrame(() => { textareaRef.current?.focus(); textareaRef.current?.setSelectionRange(applied.cursor, applied.cursor); });
  };

  const executeCommandLine = (rawCmd: string) => {
    if (disabled) return;
    if (rawCmd.trim()) {
      const resolved = classifyPromptIntent(rawCmd, effectiveMode);
      const recordFn = getTransportFn('termHistoryRecord');
      if (recordFn) void Promise.resolve(recordFn(rawCmd, cwd)).catch(() => null);
      recordClientFallbackHistory(rawCmd, cwd);
      void refreshIpcHistoryCache();
      onSubmit(resolved.toSubmitPayload(rawCmd));
    }
    setValue(''); setCursorOffset(0); setPastedLineCount(0); setHistoryRecallCursor(-1);
    liveDraftRef.current = ''; setCompletionNavigated(false); setForceOpen(false); setDismissed(false);
  };

  const submitCurrentBuffer = () => executeCommandLine(value);

  const getFilteredRecallList = (draft: string): string[] => {
    const all = getRecallHistoryList();
    return draft ? all.filter((cmd) => cmd.startsWith(draft)) : all;
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const selStart = e.currentTarget.selectionStart ?? value.length;
    const selEnd = e.currentTarget.selectionEnd ?? value.length;

    if (e.ctrlKey && !e.metaKey && !e.altKey && e.key.toLowerCase() === 'r') {
      e.preventDefault(); stashedBeforeCtrlR.current = value; setDismissed(true); setHistoryPopoverOpen(true); return;
    }
    if (e.ctrlKey && e.key === ' ') {
      e.preventDefault(); setForceOpen(true); setDismissed(false); return;
    }
    if (e.key === 'Escape') {
      if (popoverOpen) { e.preventDefault(); e.stopPropagation(); setDismissed(true); setForceOpen(false); }
      else if (pastedLineCount > 0) { e.preventDefault(); setValue(''); setCursorOffset(0); setPastedLineCount(0); }
      return;
    }
    if (popoverOpen && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
      e.preventDefault(); setCompletionNavigated(true);
      setActiveIndex((prev) => (prev + (e.key === 'ArrowDown' ? 1 : -1) + suggestions.length) % Math.max(1, suggestions.length));
      return;
    }
    if (!popoverOpen && !historyPopoverOpen && e.key === 'ArrowUp' && !value.slice(0, selStart).includes('\n')) {
      const draft = historyRecallCursor === -1 ? value : liveDraftRef.current;
      const list = getFilteredRecallList(draft);
      if (list.length > 0) {
        e.preventDefault();
        if (historyRecallCursor === -1) liveDraftRef.current = value;
        const nextIdx = historyRecallCursor === -1 ? 0 : Math.min(historyRecallCursor + 1, list.length - 1);
        setHistoryRecallCursor(nextIdx); setValue(list[nextIdx]); setCursorOffset(list[nextIdx].length); setDismissed(true); return;
      }
    }
    if (!popoverOpen && !historyPopoverOpen && e.key === 'ArrowDown' && historyRecallCursor >= 0 && !value.slice(selEnd).includes('\n')) {
      e.preventDefault();
      const list = getFilteredRecallList(liveDraftRef.current);
      if (historyRecallCursor === 0 || list.length === 0) {
        setHistoryRecallCursor(-1); setValue(liveDraftRef.current); setCursorOffset(liveDraftRef.current.length);
      } else {
        const nextIdx = Math.min(historyRecallCursor - 1, list.length - 1);
        const recalled = list[nextIdx] ?? liveDraftRef.current;
        setHistoryRecallCursor(nextIdx); setValue(recalled); setCursorOffset(recalled.length);
      }
      setDismissed(true); return;
    }
    if ((e.key === 'Tab' || (e.key === 'ArrowRight' && selEnd === value.length && ghostSuffix)) && (ghostSuffix || popoverOpen)) {
      e.preventDefault(); acceptSuggestion(); return;
    }
    if (e.key === 'Enter' && e.shiftKey) {
      e.preventDefault(); updateBuffer(`${value.slice(0, selStart)}\n${value.slice(selEnd)}`, selStart + 1); return;
    }
    if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      if (popoverOpen && completionNavigated && activeItem) acceptSuggestion(activeItem);
      else if (isIncompleteContinuation(value)) updateBuffer(`${value.slice(0, selStart)}\n${value.slice(selEnd)}`, selStart + 1);
      else submitCurrentBuffer();
    }
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const normalized = (e.clipboardData?.getData('text/plain') ?? '').replace(/\r\n/g, '\n');
    if (!normalized.includes('\n')) return;
    e.preventDefault();
    const selStart = e.currentTarget.selectionStart ?? value.length;
    const selEnd = e.currentTarget.selectionEnd ?? value.length;
    setValue(`${value.slice(0, selStart)}${normalized}${value.slice(selEnd)}`);
    setCursorOffset(selStart + normalized.length);
    setPastedLineCount(Math.max(2, normalized.split('\n').length));
    setCompletionNavigated(false); setDismissed(true);
  };

  return (
    <div data-testid="scriptorium-prompt-editor" className="relative flex flex-col gap-1">
      <HistorySearchPopover
        open={historyPopoverOpen}
        cwd={cwd}
        initialQuery={value.includes('\n') ? '' : value}
        onCleared={() => { ipcHistoryCacheRef.current = []; setHistoryRecallCursor(-1); }}
        onSelect={(cmd, action) => {
          setHistoryPopoverOpen(false);
          if (action === 'accept') {
            executeCommandLine(cmd);
            textareaRef.current?.focus();
          } else {
            setValue(cmd); setCursorOffset(cmd.length);
            setPastedLineCount(0); setCompletionNavigated(false); setDismissed(true); textareaRef.current?.focus();
          }
        }}
        onCancel={() => {
          setHistoryPopoverOpen(false); setValue(stashedBeforeCtrlR.current);
          setCursorOffset(stashedBeforeCtrlR.current.length); textareaRef.current?.focus();
        }}
      />
      <IntelliSensePopover
        open={popoverOpen}
        items={suggestions}
        activeIndex={clampedIdx}
        onSelectIndex={(idx) => { setActiveIndex(idx); setCompletionNavigated(true); }}
        onAcceptItem={(it) => acceptSuggestion(it as TerminalSuggestion)}
        onClose={() => { setDismissed(true); setForceOpen(false); }}
      />
      {pastedLineCount >= 2 && (
        <div
          data-testid="term-paste-safety-banner"
          className="flex items-center justify-between gap-2 rounded border border-(--color-status-warn) bg-bg-elevated px-3 py-1 text-xs font-normal text-text-primary"
        >
          <span>{`Pasted ${pastedLineCount} lines into editor - review script and press Enter to execute.`}</span>
          <button
            type="button"
            aria-label="Clear prompt"
            onClick={() => { setValue(''); setCursorOffset(0); setPastedLineCount(0); textareaRef.current?.focus(); }}
            className="h-6 rounded border border-border-subtle bg-bg-surface px-2 py-1 text-[11px] font-semibold text-text-secondary hover:text-text-primary"
          >
            Clear Prompt
          </button>
        </div>
      )}
      <div className="relative flex items-start gap-2 rounded-md border border-border-subtle bg-bg-surface p-2 focus-within:border-accent focus-within:ring-1 focus-within:ring-brass/30">
        <VoxSelect
          data-testid="term-shell-mode-select"
          aria-label="Terminal input mode"
          value={effectiveMode}
          onValueChange={(next) => {
            const nextMode = next as ShellMode;
            setInternalShellMode(nextMode);
            if (nextMode === 'default' || nextMode === 'nu' || nextMode === 'vox') {
              setExtendedModeOverride(null);
              (onShellModeChange as ((m: TermShellModeId) => void) | undefined)?.(nextMode);
            } else {
              setExtendedModeOverride(nextMode);
              (onShellModeChange as ((m: ShellMode) => void) | undefined)?.(nextMode);
            }
          }}
          options={SHELL_MODE_OPTIONS}
          disabled={disabled}
          className="h-6 w-auto shrink-0 border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-primary"
        />
        <span
          data-testid="console-input-mode-badge"
          data-terminal-mono="true"
          style={TERMINAL_MONO_STYLE}
          className={`inline-flex h-6 shrink-0 items-center rounded border px-2 py-1 text-[11px] font-semibold uppercase tracking-wider ${MODE_BADGE_TONES[intent.modeBadge]}`}
        >
          {intent.modeBadge}
        </span>
        {intent.showAiAssistPill && (
          <button
            type="button"
            data-testid="term-ai-assist-pill"
            onClick={() => {
              const trimmed = value.trimStart();
              const next = trimmed.startsWith('?') ? trimmed : `?${trimmed}`;
              setValue(next); setCursorOffset(next.length); textareaRef.current?.focus();
            }}
            className="inline-flex h-6 shrink-0 items-center gap-1 rounded border border-brass/50 bg-brass/15 px-2 py-1 text-[11px] font-semibold text-accent transition hover:bg-brass/25"
          >
            <Icon.spark className="size-3" aria-hidden="true" />
            <span>AI Assist</span>
          </button>
        )}
        <div className="relative min-w-0 flex-1">
          <div
            ref={mirrorRef}
            aria-hidden="true"
            data-testid="scriptorium-highlight-layer"
            data-terminal-mono="true"
            style={{ ...TERMINAL_MONO_STYLE, height: `${editorHeightPx}px`, scrollbarGutter: 'stable' }}
            className="pointer-events-none absolute inset-0 m-0 min-h-6 overflow-y-auto px-2 py-1 text-xs leading-[1.45] font-normal whitespace-pre-wrap break-words"
          >
            {tokens.map((tok, idx) => <span key={idx} className={PROMPT_TOKEN_CLASSES[tok.kind]}>{tok.text}</span>)}
            <span data-testid="ghost" className="text-text-muted opacity-60">
              {cursorOffset === value.length ? ghostSuffix : ''}
            </span>
            {value.endsWith('\n') ? ' ' : null}
          </div>
          <textarea
            ref={textareaRef}
            rows={1}
            aria-label="console input"
            aria-autocomplete="inline"
            data-testid="term-prompt-textarea"
            data-terminal-mono="true"
            disabled={disabled}
            placeholder={effectiveMode === 'nu' ? 'Type a Nushell command, ?ask-agent, or /model route...' : 'Type a command, !shell, ?ask-agent, or /model route...'}
            value={value}
            onChange={(e) => {
              const nextVal = e.target.value;
              setValue(nextVal); setCursorOffset(e.target.selectionStart ?? nextVal.length);
              setCompletionNavigated(false); setDismissed(false); setActiveIndex(0); setHistoryRecallCursor(-1);
              if (!nextVal.includes('\n')) setPastedLineCount(0);
            }}
            onSelect={(e) => setCursorOffset(e.currentTarget.selectionStart ?? value.length)}
            onScroll={(e) => { if (mirrorRef.current) mirrorRef.current.scrollTop = e.currentTarget.scrollTop; }}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            spellCheck={false}
            autoComplete="off"
            style={{ ...TERMINAL_MONO_STYLE, height: `${editorHeightPx}px`, scrollbarGutter: 'stable' }}
            className={`relative z-10 block min-h-6 w-full resize-none overflow-y-auto border-none bg-transparent px-2 py-1 text-xs leading-[1.45] font-normal selection:bg-brass/25 placeholder:text-text-muted outline-none ${
              value.length > 0 ? 'text-transparent caret-(--color-text-primary)' : 'text-text-primary'
            }`}
          />
          <span role="status" aria-live="polite" className="sr-only">{activeItem ? `Suggestion: ${activeItem.label}` : ''}</span>
        </div>
        <button
          type="button"
          data-testid="term-history-trigger"
          aria-label="Search command history"
          title="Search history (Ctrl+R)"
          disabled={disabled}
          onClick={() => {
            setHistoryPopoverOpen((prev) => {
              if (!prev) { stashedBeforeCtrlR.current = value; setDismissed(true); }
              return !prev;
            });
          }}
          className="inline-flex h-6 shrink-0 items-center justify-center rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-secondary transition hover:text-text-primary disabled:opacity-50"
        >
          <Icon.clock className="size-3" aria-hidden="true" />
          <span className="sr-only">Search command history</span>
        </button>
        <button
          type="button"
          data-testid="term-prompt-submit"
          aria-label={intent.kind === 'agent' ? 'Ask agent' : 'Run command'}
          disabled={disabled || !value.trim()}
          onClick={submitCurrentBuffer}
          className="inline-flex h-6 shrink-0 items-center gap-1 rounded border border-brass/50 bg-brass/15 px-2 py-1 text-[11px] font-semibold text-accent transition hover:bg-brass/25 disabled:pointer-events-none disabled:opacity-50"
        >
          <span>{intent.kind === 'agent' ? 'Ask Agent' : 'Run Command'}</span>
        </button>
      </div>
    </div>
  );
}
