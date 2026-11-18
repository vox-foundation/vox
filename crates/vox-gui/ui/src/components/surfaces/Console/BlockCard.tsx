import React from 'react';
import type { OpenLocator, TermBlockDto, TermBlockOrigin } from '../../../transport';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { AgentTurnContent } from './AgentTurnContent';
import { JsonLedgerTable, tryParseStructuredJson } from './JsonLedgerTable';
import { RichBlockPreview, formatBlockRusageBadge, tryParseVoxFrameOrPlot } from './RichBlockPreview';
import { sgrToSpans, stripAnsi } from './sgrToSpans';
import { SlashMetaContent, TermBlockFeedbackStrip } from './SlashMetaContent';
import { TERMINAL_MONO_STYLE, useDocumentThemeAttr } from './terminalTheme';

export { TERMINAL_MONO_STYLE };

export type ExtendedTermBlockDto = TermBlockDto & {
  cwd?: string | null;
  git_branch?: string | null;
  prompt_tail?: string;
  prompt_kind?: string | null;
};

export interface BlockCardProps {
  block: ExtendedTermBlockDto;
  isPinned: boolean;
  isFocused: boolean;
  compact?: boolean;
  searchQuery?: string;
  onCopyCommand: (block: TermBlockDto) => void;
  onCopyOutput: (block: TermBlockDto) => void;
  onRerun: (block: TermBlockDto) => void;
  onPin: (block: TermBlockDto) => void;
  onAskAgent: (block: TermBlockDto) => void;
  onFocusInChat?: (block: TermBlockDto, lineRange?: string) => void;
  onSelectBlock?: (blockId: number) => void;
  onSendStdin?: (block: TermBlockDto, line: string) => void;
  onPromoteBg?: (block: TermBlockDto) => void;
  onInterrupt?: (block: TermBlockDto) => void;
  onOpenLocator?: (locator: OpenLocator) => void;
  onExplainFixBlock?: (block: TermBlockDto, mode?: 'explain' | 'fix') => void;
  diagnosingBlockId?: number | null;
  onRunProposedCommand?: (cmd: string, sourceBlockId?: number) => void;
  onInsertInPrompt?: (cmd: string) => void;
  onOpenSecrets?: () => void;
  onRetryAgentTurn?: (block: TermBlockDto) => void;
  onCancelAgentTurn?: (block: TermBlockDto) => void;
  onFeedback?: (block: TermBlockDto, verdict: 'accepted' | 'rejected' | 'corrected', correctedCommand?: string) => Promise<boolean | void> | boolean | void;
}

function formatOriginLabel(origin: TermBlockOrigin | undefined): string {
  if (!origin) return 'user';
  if (typeof origin === 'string') return origin;
  return origin.kind === 'agent' ? `agent:${origin.agent_id}` : 'user';
}

function isAgentOrigin(origin: TermBlockOrigin | undefined): boolean {
  if (!origin) return false;
  return typeof origin === 'string' ? origin === 'agent' : origin.kind === 'agent';
}

function formatStatusLabel(block: ExtendedTermBlockDto): string {
  if (block.status === 'Running') return 'running';
  if (block.status === 'AwaitingInput') return 'awaiting input';
  if (block.status === 'Cancelled') return 'cancelled';
  if (block.exit_code !== null && block.exit_code !== undefined) return `exit ${block.exit_code}`;
  return block.status === 'Failed' ? 'failed' : 'ok';
}

function formatDuration(block: ExtendedTermBlockDto): string | null {
  if (typeof block.ended_ms === 'number' && typeof block.started_ms === 'number' && block.ended_ms >= block.started_ms) {
    const ms = block.ended_ms - block.started_ms;
    return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
  }
  return null;
}

function extractRawOutput(block: ExtendedTermBlockDto): string {
  const out = block.output as unknown;
  if (typeof out === 'string') return out;
  return Array.isArray(out) ? out.map((c: { text?: string }) => c.text ?? '').join('') : '';
}

function basenameOf(pathStr: string): string {
  const parts = pathStr.replace(/[\\/]+$/, '').split(/[\\/]/);
  return parts[parts.length - 1] || pathStr;
}

export function isStructuredBlock(input: string, outputText: string): boolean {
  const trimmedInput = input.trim();
  if (trimmedInput.startsWith('!nu') || trimmedInput.startsWith('/nu')) return true;
  const trimmedOutput = outputText.trim();
  if ((trimmedOutput.startsWith('{') && trimmedOutput.endsWith('}')) || (trimmedOutput.startsWith('[') && trimmedOutput.endsWith(']'))) {
    try { JSON.parse(trimmedOutput); return true; }
    catch { return false; }
  }
  return false;
}

export function formatRenderedBlockOutput(input: string, rawOutput: string): {
  plainOutput: string;
  isStructured: boolean;
  renderedOutput: string;
} {
  const plainOutput = stripAnsi(rawOutput);
  const isStructured = isStructuredBlock(input, plainOutput);
  let renderedOutput = plainOutput;
  if (isStructured && plainOutput.trim()) {
    try { renderedOutput = JSON.stringify(JSON.parse(plainOutput.trim()), null, 2); }
    catch { renderedOutput = plainOutput; }
  }
  return { plainOutput, isStructured, renderedOutput };
}

export function formatBlockSearchCorpus(block: ExtendedTermBlockDto): string {
  const rawOutput = extractRawOutput(block);
  const { renderedOutput } = formatRenderedBlockOutput(block.input, rawOutput);
  return `$ ${stripAnsi(block.input)}\n${renderedOutput}`;
}

const ACTION_BTN_CLS =
  'h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold text-text-secondary hover:border-border-strong hover:text-text-primary';
const META_CHIP_CLS =
  'inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] leading-[1.25] font-semibold text-text-secondary';

export function BlockCard({
  block, isPinned, isFocused, compact = false, searchQuery,
  onCopyCommand, onCopyOutput, onRerun, onPin, onAskAgent,
  onFocusInChat, onSelectBlock, onSendStdin, onPromoteBg, onInterrupt, onOpenLocator,
  onExplainFixBlock, diagnosingBlockId = null, onRunProposedCommand, onInsertInPrompt, onOpenSecrets, onRetryAgentTurn, onCancelAgentTurn, onFeedback,
}: BlockCardProps) {
  useDocumentThemeAttr();
  const [collapsed, setCollapsed] = React.useState(false);
  const [stdinValue, setStdinValue] = React.useState('');
  const [ledgerView, setLedgerView] = React.useState<'json' | 'table'>('json');

  React.useEffect(() => {
    if (block.status === 'Failed') setCollapsed(false);
    if (block.status !== 'AwaitingInput') setStdinValue('');
  }, [block.status]);

  const originLabel = formatOriginLabel(block.origin);
  const agentOrigin = isAgentOrigin(block.origin);
  const statusLabel = formatStatusLabel(block);
  const durationLabel = formatDuration(block);
  const rusageBadge = formatBlockRusageBadge(block);
  const rawOutput = extractRawOutput(block);
  const { plainOutput, isStructured, renderedOutput: formattedStructured } = formatRenderedBlockOutput(block.input, rawOutput);
  const richPreview = React.useMemo(() => (plainOutput ? tryParseVoxFrameOrPlot(plainOutput) : null), [plainOutput]);
  const structuredData = React.useMemo(() => (!richPreview && plainOutput ? tryParseStructuredJson(plainOutput) : null), [plainOutput, richPreview]);

  const lineCount = plainOutput ? plainOutput.replace(/\n$/, '').split('\n').length : 0;
  const isAgentTurn = block.kind === 'AgentTurn';
  const isSlashMeta = block.kind === 'SlashCommand' || block.kind === 'Meta';
  const explainFixMatch = block.input.match(/^\?\s*(?:explain|fix)\s+block\s+#(\d+)|^\?explain-fix\s+#(\d+)/i);
  const explainTargetId = explainFixMatch ? (explainFixMatch[1] ?? explainFixMatch[2] ?? null) : null;
  const isFailed =
    block.status !== 'Cancelled' &&
    (block.status === 'Failed' || (block.exit_code !== null && block.exit_code !== undefined && block.exit_code !== 0));
  const canExplainFix = isFailed && block.status !== 'Running' && block.status !== 'AwaitingInput' && !block.credential_prompt && !isAgentTurn && !isSlashMeta;
  const isDiagnosing = diagnosingBlockId === block.id;

  const cardTone = isPinned ? 'border-brass bg-overlay-subtle shadow-xs'
    : block.status === 'AwaitingInput' ? 'border-(--color-status-warn) bg-bg-surface ring-1 ring-(--color-status-warn)/40'
    : isFailed ? 'border-(--color-status-fail)/50 bg-bg-surface'
    : block.status === 'Running' ? 'border-border-strong bg-bg-surface'
    : block.status === 'Cancelled' ? 'border-border-subtle bg-bg-surface opacity-80'
    : 'border-border-subtle bg-bg-surface hover:border-border-strong';

  const statusTone = block.status === 'AwaitingInput' ? 'text-(--color-status-warn)'
    : block.status === 'Cancelled' ? 'text-text-muted'
    : block.status === 'Running' ? 'text-text-secondary'
    : isFailed ? 'text-(--color-status-fail)' : 'text-(--color-status-pass)';

  const submitStdin = () => { const line = stdinValue; setStdinValue(''); onSendStdin?.(block, line); };
  const handleCtrlB = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && (e.key === 'b' || e.key === 'B')) { e.preventDefault(); e.stopPropagation(); onPromoteBg?.(block); }
  };

  const handleFocusInChat = (e: React.MouseEvent) => {
    e.stopPropagation();
    const sel = typeof window !== 'undefined' ? window.getSelection()?.toString()?.trim() : undefined;
    const refText = isStructured ? formattedStructured : plainOutput;
    let lineRange: string | undefined;
    if (sel && refText.includes(sel)) {
      const allLines = refText.split('\n');
      const selLines = sel.split('\n');
      const startIdx = allLines.findIndex((l) => l.includes(selLines[0] ?? ''));
      if (startIdx >= 0) lineRange = `L${startIdx + 1}-${startIdx + selLines.length}`;
    }
    onFocusInChat?.(block, lineRange);
  };

  return (
    <div
      data-testid="term-block-card" data-block-id={String(block.id)} data-block-status={block.status}
      data-pinned={isPinned ? 'true' : 'false'} data-focused={isFocused ? 'true' : 'false'}
      onClick={() => onSelectBlock?.(block.id)} style={TERMINAL_MONO_STYLE}
      className={`flex flex-col gap-2 rounded-lg border ${compact ? 'p-2' : 'p-3'} transition-colors ${cardTone} ${isFocused ? 'ring-1 ring-ring border-border-strong' : ''}`}
    >
      <div className="sticky top-0 z-10 flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle bg-bg-surface/95 py-2 backdrop-blur-xs">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button" data-testid={`term-block-collapse-${block.id}`}
            aria-label={collapsed ? 'Expand command output' : 'Collapse command output'}
            onClick={(e) => { e.stopPropagation(); setCollapsed((prev) => !prev); }}
            className="inline-flex size-6 items-center justify-center rounded text-text-muted hover:bg-overlay-hover hover:text-text-primary"
          >
            {collapsed ? <Icon.chevR className="size-4" aria-hidden="true" /> : <Icon.chevronDown className="size-4" aria-hidden="true" />}
          </button>
          <span className={`rounded border px-2 py-1 text-[11px] leading-[1.25] font-semibold uppercase tracking-wider ${
            isAgentTurn ? 'inline-flex items-center gap-1 border-brass/40 bg-overlay-subtle text-accent'
              : isSlashMeta ? 'inline-flex items-center gap-1 border-border-strong bg-bg-elevated text-accent'
              : agentOrigin ? 'border-border-strong bg-overlay-subtle text-accent'
              : 'border-border-subtle bg-bg-elevated text-text-secondary'
          }`}>
            {isAgentTurn ? <Icon.spark className="size-3" aria-hidden="true" /> : isSlashMeta ? <Icon.command className="size-3" aria-hidden="true" /> : null}
            <span>{isAgentTurn ? 'agent' : isSlashMeta ? 'slash' : originLabel}</span>
          </span>
          {explainTargetId ? (
            <span data-testid="term-explain-fix-context" className={META_CHIP_CLS}>{`Scrubbed #${explainTargetId}`}</span>
          ) : null}
          {isPinned && (
            <span className="inline-flex items-center gap-1 rounded border border-brass bg-overlay-subtle px-2 py-1 text-[11px] leading-[1.25] font-semibold text-accent">
              <Icon.pin className="size-3" aria-hidden="true" /><span>Pinned</span>
            </span>
          )}
          {isStructured && (
            <span data-testid="term-block-structured-badge" className="rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] leading-[1.25] font-semibold uppercase tracking-wider text-text-secondary">
              structured json
            </span>
          )}
          {structuredData !== null && (
            <div
              data-testid="term-ledger-view-toggle"
              role="tablist"
              aria-label="Structured output view"
              onClick={(e) => e.stopPropagation()}
              className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated p-1"
            >
              <button
                type="button"
                role="tab"
                data-testid="term-ledger-tab-json"
                aria-selected={ledgerView === 'json'}
                onClick={() => setLedgerView('json')}
                className={`rounded px-2 py-1 text-[11px] leading-[1.25] font-semibold transition-colors ${
                  ledgerView === 'json'
                    ? 'bg-bg-surface text-accent shadow-xs'
                    : 'text-text-secondary hover:text-text-primary'
                }`}
              >
                JSON
              </button>
              <button
                type="button"
                role="tab"
                data-testid="term-ledger-tab-table"
                aria-selected={ledgerView === 'table'}
                onClick={() => setLedgerView('table')}
                className={`rounded px-2 py-1 text-[11px] leading-[1.25] font-semibold transition-colors ${
                  ledgerView === 'table'
                    ? 'bg-bg-surface text-accent shadow-xs'
                    : 'text-text-secondary hover:text-text-primary'
                }`}
              >
                Table
              </button>
            </div>
          )}
          {block.cwd && <span title={block.cwd} className={META_CHIP_CLS}><Icon.folder className="size-3" aria-hidden="true" /><span>{basenameOf(block.cwd)}</span></span>}
          {block.git_branch && <span className={META_CHIP_CLS}><Icon.branch className="size-3" aria-hidden="true" /><span>{block.git_branch}</span></span>}
          <code data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="text-sm leading-[1.3] font-semibold text-text-primary">
            {isAgentTurn || isSlashMeta
              ? (searchQuery?.trim() ? sgrToSpans(block.input, { searchQuery, onOpenLocator }) : block.input)
              : (searchQuery?.trim() ? sgrToSpans(`$ ${block.input}`, { searchQuery, onOpenLocator }) : `$ ${block.input}`)}
          </code>
        </div>

        <div className="flex items-center gap-2 text-[11px] leading-[1.25]">
          <span className={`inline-flex items-center gap-1 font-semibold ${statusTone}`}>
            {block.status === 'Running' && <span className="size-2 animate-pulse rounded-full bg-(--color-status-info)" aria-hidden="true" />}
            {block.status === 'Cancelled' && <Icon.ban className="size-3" aria-hidden="true" />}
            {block.status !== 'Running' && block.status !== 'Cancelled' && block.status !== 'AwaitingInput' && (
              isFailed ? <Icon.alert className="size-3" aria-hidden="true" /> : <Icon.check className="size-3" aria-hidden="true" />
            )}
            <span>{statusLabel}</span>
          </span>
          {block.status === 'Running' && (isAgentTurn ? (onCancelAgentTurn || onInterrupt) : onInterrupt) && (
            <Button variant="ghost" size="xs" onClick={(e) => { e.stopPropagation(); (isAgentTurn ? (onCancelAgentTurn ?? onInterrupt) : onInterrupt)?.(block); }} className="h-6 gap-1 rounded border border-(--color-status-fail)/50 px-2 py-1 text-[11px] font-semibold text-(--color-status-fail) hover:bg-overlay-hover">
              <Icon.stop className="size-3" aria-hidden="true" /><span>{isAgentTurn ? 'Stop Stream' : 'Interrupt Process'}</span>
            </Button>
          )}
          {durationLabel && <span className="text-text-muted">{durationLabel}</span>}
          {rusageBadge && <span data-testid="term-block-rusage" className="text-text-muted">{rusageBadge}</span>}
        </div>
      </div>

      {collapsed ? (
        <div className="rounded-md border border-border-subtle bg-bg-base px-3 py-1 text-[11px] leading-[1.25] font-semibold text-text-muted">
          {`${lineCount} lines hidden`}
        </div>
      ) : isAgentTurn ? (
        <AgentTurnContent
          block={block}
          showStopInBody={false}
          onRunProposedCommand={(cmd) => onRunProposedCommand?.(cmd, block.id)}
          onEditInPrompt={onInsertInPrompt}
          onOpenSecrets={onOpenSecrets}
          onRunSecretsDoctor={() => onRunProposedCommand?.('vox secrets doctor', block.id)}
          onRetryAgentTurn={onRetryAgentTurn ?? onRerun}
          onCancelAgentTurn={onCancelAgentTurn ?? onInterrupt}
        />
      ) : isSlashMeta ? (
        <SlashMetaContent block={block} />
      ) : richPreview ? (
        <div onClick={(e) => e.stopPropagation()}><RichBlockPreview spec={richPreview} /></div>
      ) : structuredData !== null && ledgerView === 'table' ? (
        <div onClick={(e) => e.stopPropagation()}>
          <JsonLedgerTable data={structuredData} />
        </div>
      ) : rawOutput ? (
        <pre
          tabIndex={0}
          data-testid="term-block-output"
          data-terminal-well="true"
          data-terminal-mono="true"
          style={TERMINAL_MONO_STYLE}
          className="max-h-80 overflow-y-auto whitespace-pre-wrap break-words rounded border border-(--terminal-well-border) bg-(--terminal-well-bg) px-3 py-2 text-xs leading-[1.45] text-(--terminal-well-fg)"
        >
          {isStructured ? (searchQuery ? sgrToSpans(formattedStructured, { searchQuery, onOpenLocator }) : formattedStructured) : sgrToSpans(rawOutput, { searchQuery, onOpenLocator })}
        </pre>
      ) : null}

      {block.status === 'AwaitingInput' ? (
        <div data-testid="term-block-stdin-bar" onClick={(e) => e.stopPropagation()} onKeyDown={handleCtrlB} className="flex flex-wrap items-center gap-2 rounded-md border border-(--color-status-warn) bg-bg-elevated p-2">
          <span className="inline-flex items-center gap-1 text-[11px] leading-[1.25] font-semibold text-(--color-status-warn)">
            {block.credential_prompt ? <Icon.lockKeyhole className="size-4" aria-hidden="true" /> : <Icon.terminal className="size-4" aria-hidden="true" />}
            <span>Process awaiting input</span>
          </span>
          <input
            aria-label="Command stdin input" type={block.credential_prompt ? 'password' : 'text'} autoComplete="off"
            value={stdinValue} onChange={(e) => setStdinValue(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); submitStdin(); } else handleCtrlB(e); }}
            className="h-6 min-w-40 flex-1 rounded border border-border-strong bg-bg-base px-2 py-1 text-xs text-text-primary focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <Button variant="secondary" size="xs" onClick={submitStdin} className="h-6 px-2 py-1 text-[11px] font-semibold">Send Stdin Line</Button>
          <Button variant="ghost" size="xs" data-testid="term-block-promote-bg" onClick={() => onPromoteBg?.(block)} className={`gap-1 ${ACTION_BTN_CLS}`}>
            <span>Promote to Background</span><span className="text-text-muted">(Ctrl+B)</span>
          </Button>
        </div>
      ) : null}

      <div className="flex flex-wrap items-center gap-2 pt-1">
        {canExplainFix ? (
          <Button
            variant="secondary"
            size="xs"
            data-testid="term-explain-fix-block"
            aria-label={isDiagnosing ? 'Diagnosing Failed Block...' : 'Explain / Fix Failed Block'}
            disabled={isDiagnosing}
            onClick={(e) => { e.stopPropagation(); onExplainFixBlock?.(block); }}
            className="h-6 gap-1 rounded border border-brass bg-overlay-subtle px-2 py-1 text-[11px] font-semibold text-accent hover:border-border-strong"
          >
            <Icon.spark className="size-3 shrink-0" aria-hidden="true" />
            <span>{isDiagnosing ? 'Diagnosing Failed Block...' : 'Explain / Fix Failed Block'}</span>
          </Button>
        ) : null}
        <Button variant="ghost" size="xs" onClick={(e) => { e.stopPropagation(); onCopyCommand(block); }} className={ACTION_BTN_CLS}>Copy cmd</Button>
        <Button variant="ghost" size="xs" onClick={(e) => { e.stopPropagation(); onCopyOutput(block); }} className={ACTION_BTN_CLS}>Copy output</Button>
        <Button variant="ghost" size="xs" onClick={(e) => { e.stopPropagation(); onRerun(block); }} className={ACTION_BTN_CLS}>Re-run</Button>
        <Button
          variant={isPinned ? 'primary' : 'ghost'} size="xs" data-testid={`term-block-pin-${block.id}`}
          onClick={(e) => { e.stopPropagation(); onPin(block); }}
          className={`h-6 gap-1 rounded border px-2 py-1 text-[11px] font-semibold ${isPinned ? 'border-brass bg-overlay-subtle text-accent' : 'border-border-subtle text-text-secondary hover:border-border-strong hover:text-text-primary'}`}
        >
          <Icon.pin className="size-3" aria-hidden="true" /><span>{isPinned ? 'Unpin' : 'Pin'}</span>
        </Button>
        <Button variant="ghost" size="xs" data-testid="term-block-focus-chat" onClick={handleFocusInChat} className={ACTION_BTN_CLS}>
          Focus in Chat
        </Button>
        <Button variant="ghost" size="xs" onClick={(e) => { e.stopPropagation(); onAskAgent(block); }} className={ACTION_BTN_CLS}>
          Ask Agent
        </Button>
        {!compact && onFeedback ? (
          <TermBlockFeedbackStrip
            block={block}
            onFeedback={onFeedback}
            onRunCommand={(cmd) => onRunProposedCommand?.(cmd, block.id)}
          />
        ) : null}
      </div>
    </div>
  );
}
