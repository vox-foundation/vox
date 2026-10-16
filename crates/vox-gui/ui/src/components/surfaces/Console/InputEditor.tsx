import React, { useEffect, useMemo, useRef, useState } from 'react';
import { discoverySuggest, type Suggestion } from '../../../transport';
import {
  computeIntelliSenseItems,
  parseCompletionTrigger,
  type IntelliSenseItem,
} from '../../../lib/intellisenseEngine';
import { IntelliSensePopover } from '../../ui/IntelliSensePopover';
import {
  getTerminalBridgeState,
  subscribeTerminalBridge,
  type TerminalBridgeState,
} from '../../../lib/terminalBridge';

export type ShellMode = 'default' | 'nu' | 'vox';

interface Props {
  onSubmit: (line: string) => void;
  /** Called with the currently-highlighted suggestion's action id (for the rail). */
  onActiveSuggestion: (actionId: string | null) => void;
  /** When set, replaces the current input with this line (Discovery rail "Use"). */
  applyLine?: string | null;
  onApplyLineConsumed?: () => void;
  shellMode?: ShellMode;
  onShellModeChange?: (mode: ShellMode) => void;
}

/**
 * The console prompt. The shell never receives a keystroke until Enter, so we own
 * completion entirely: as the user types, the Unified IntelliSense engine merges
 * CLI catalog commands/flags, `/slash` commands, `!nu` pipelines, and `@term` context
 * into inline ghost text + a categorized, keyboard-navigable popover.
 */
export function InputEditor({
  onSubmit,
  onActiveSuggestion,
  applyLine,
  onApplyLineConsumed,
  shellMode: propShellMode,
  onShellModeChange,
}: Props) {
  const [internalShellMode, setInternalShellMode] = useState<ShellMode>('default');
  const shellMode = propShellMode ?? internalShellMode;
  const setShellMode = onShellModeChange ?? setInternalShellMode;
  const [value, setValue] = useState('');
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [forceOpen, setForceOpen] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [termBridge, setTermBridge] = useState<TerminalBridgeState>(getTerminalBridgeState);
  const debounce = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reqSeq = useRef(0);

  useEffect(() => subscribeTerminalBridge(setTermBridge), []);

  useEffect(() => {
    if (!applyLine) return;
    setValue(
      applyLine.startsWith('vox') || applyLine.startsWith('/') || applyLine.startsWith('!')
        ? applyLine
        : `vox ${applyLine}`,
    );
    onApplyLineConsumed?.();
  }, [applyLine, onApplyLineConsumed]);

  useEffect(() => {
    setDismissed(false);
    setActiveIndex(0);
    if (debounce.current) clearTimeout(debounce.current);
    const trimmed = value.trimStart();
    const shouldQueryCatalog =
      trimmed.startsWith('vox') ||
      trimmed.startsWith('/') ||
      trimmed.startsWith('!nu') ||
      (shellMode === 'vox' && trimmed.length > 0);
    if (!shouldQueryCatalog) {
      setSuggestions([]);
      return;
    }
    debounce.current = setTimeout(() => {
      const req = ++reqSeq.current;
      discoverySuggest(value, 8)
        .then((s) => {
          if (req !== reqSeq.current) return;
          setSuggestions(Array.isArray(s) ? s : []);
        })
        .catch(() => {
          if (req !== reqSeq.current) return;
          setSuggestions([]);
        });
    }, 80);
    return () => {
      if (debounce.current) clearTimeout(debounce.current);
    };
  }, [value, shellMode]);

  const items = useMemo(
    () =>
      computeIntelliSenseItems({
        input: value,
        surface: 'terminal',
        shellMode,
        catalogSuggestions: suggestions,
        termBridge,
        forceOpen,
      }),
    [value, shellMode, suggestions, termBridge, forceOpen],
  );

  const activeItem: IntelliSenseItem | undefined =
    items[Math.min(activeIndex, Math.max(0, items.length - 1))] ?? items[0];

  useEffect(() => {
    onActiveSuggestion(activeItem?.actionId ?? suggestions[0]?.action_id ?? null);
  }, [activeItem, suggestions, onActiveSuggestion]);

  const top = suggestions[0];
  const typedTail = value.replace(/^vox\s*/, '');
  const catalogGhost =
    top && top.completion.startsWith(typedTail) && typedTail.length > 0
      ? top.completion.slice(typedTail.length)
      : '';
  const ghost = activeItem?.ghostSuffix || catalogGhost;

  const popoverOpen =
    !dismissed &&
    items.length > 0 &&
    (forceOpen ||
      value.trim().length > 0);

  const acceptItem = (item?: IntelliSenseItem) => {
    const target = item ?? activeItem;
    if (target) {
      const trig = parseCompletionTrigger(value, value.length, 'terminal');
      if (
        trig.kind === 'at_term' ||
        trig.kind === 'at_file' ||
        trig.kind === 'at_skill' ||
        trig.kind === 'at_mcp' ||
        trig.kind === 'at_symbol' ||
        trig.kind === 'at_all'
      ) {
        setValue(`${value.slice(0, trig.replaceStart)}${target.insertText} `);
      } else {
        setValue(target.insertText);
      }
      setForceOpen(false);
      setDismissed(true);
      return;
    }
    if (catalogGhost && top) {
      setValue(`vox ${top.completion}`);
      setDismissed(true);
    }
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.ctrlKey && e.key === ' ') {
      e.preventDefault();
      setForceOpen(true);
      setDismissed(false);
      return;
    }
    if (popoverOpen && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
      e.preventDefault();
      setActiveIndex((prev) => {
        const delta = e.key === 'ArrowDown' ? 1 : -1;
        return (prev + delta + items.length) % Math.max(1, items.length);
      });
      return;
    }
    if (popoverOpen && e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      setDismissed(true);
      setForceOpen(false);
      return;
    }
    if ((e.key === 'Tab' || e.key === 'ArrowRight') && (ghost || popoverOpen)) {
      e.preventDefault();
      acceptItem();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      let line = value.trim();
      if (line) {
        if (shellMode === 'nu' && !line.startsWith('/') && !line.startsWith('?')) {
          if (!line.startsWith('!nu ') && line !== '!nu') {
            line = `!nu ${line}`;
          }
        }
        onSubmit(line);
      }
      setValue('');
      setSuggestions([]);
      setForceOpen(false);
      setDismissed(false);
    }
  };

  let modeBadge = {
    label: 'vox',
    tone: 'border-brass/40 bg-brass/10 text-brass',
  };
  if (value.startsWith('?')) {
    modeBadge = {
      label: 'agent',
      tone: 'border-amber-500/40 bg-amber-500/15 text-amber-300',
    };
  } else if (value.startsWith('/')) {
    modeBadge = {
      label: 'route',
      tone: 'border-cyan/40 bg-cyan/15 text-cyan',
    };
  } else if (value.startsWith('!nu') || (shellMode === 'nu' && !value.startsWith('!'))) {
    modeBadge = {
      label: 'nu',
      tone: 'border-cyan/40 bg-cyan/15 text-cyan',
    };
  } else if (value.startsWith('!')) {
    modeBadge = {
      label: 'shell',
      tone: 'border-emerald-500/40 bg-emerald-500/15 text-emerald-300',
    };
  }

  return (
    <div className="relative flex items-center gap-2 rounded-md border border-white/10 bg-black/40 px-2.5 py-1.5 focus-within:border-cyan/50 focus-within:ring-1 focus-within:ring-cyan/20">
      <IntelliSensePopover
        open={popoverOpen}
        items={items}
        activeIndex={activeIndex}
        onSelectIndex={setActiveIndex}
        onAcceptItem={(it) => acceptItem(it)}
        onClose={() => {
          setDismissed(true);
          setForceOpen(false);
        }}
      />
      <select
        data-testid="term-shell-mode-select"
        aria-label="shell mode selector"
        value={shellMode}
        onChange={(e) => setShellMode(e.target.value as ShellMode)}
        className="shrink-0 rounded border border-white/10 bg-zinc-900 px-1.5 py-0.5 font-mono text-[9px] font-semibold text-zinc-300 outline-none hover:border-white/20 focus:border-cyan/50"
      >
        <option value="default">default</option>
        <option value="nu">nu</option>
        <option value="vox">vox</option>
      </select>
      <span
        data-testid="console-input-mode-badge"
        className={`shrink-0 rounded px-1.5 py-0.5 font-mono text-[9px] font-semibold uppercase tracking-wider border ${modeBadge.tone}`}
      >
        {modeBadge.label}
      </span>
      <div className="relative flex-1 font-mono text-xs">
        <span
          aria-hidden
          className="pointer-events-none absolute left-0 top-0 text-zinc-500"
        >
          {value}
          <span data-testid="ghost" className="text-zinc-600">{ghost}</span>
        </span>
        <input
          role="textbox"
          aria-label="console input"
          aria-autocomplete="inline"
          placeholder={
            shellMode === 'nu'
              ? 'Type a Nushell command, ?ask-agent, or /model route...'
              : 'Type a command, !shell, ?ask-agent, or /model route...'
          }
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={onKeyDown}
          spellCheck={false}
          autoComplete="off"
          className="w-full bg-transparent text-zinc-100 placeholder:text-zinc-600 outline-none border-none font-mono text-xs"
        />
        <span
          role="status"
          aria-live="polite"
          className="sr-only"
        >
          {activeItem
            ? `Suggestion: ${activeItem.label}`
            : top
              ? `Suggestion: ${top.completion}`
              : ''}
        </span>
      </div>
    </div>
  );
}
