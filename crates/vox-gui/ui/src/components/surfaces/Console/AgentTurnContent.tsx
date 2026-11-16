import React from 'react';
import type { TermBlockDto } from '../../../transport';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { TERMINAL_MONO_STYLE } from './terminalTheme';

const RUNNABLE_LANGS = new Set(['', 'sh', 'bash', 'zsh', 'fish', 'shell', 'pwsh', 'powershell', 'nu', 'vox']);

export type AgentTurnSegment =
  | { kind: 'prose'; text: string }
  | { kind: 'code'; lang: string; code: string; closed: boolean; proposedCommand: string | null }
  | { kind: 'proposed'; command: string };

export interface ParsedAgentOutput {
  modelLabel: string | null;
  bodyText: string;
  isClavisUnconfigured: boolean;
  streamErrorText: string | null;
}

export function isExplainFixInput(input: string): boolean {
  const t = input.trim();
  return /^\?\s*(?:fix|explain)\b/i.test(t) || /^\?explain-fix\b/i.test(t);
}

export function parseAgentHeaderAndBody(rawText: string): ParsedAgentOutput {
  let rest = rawText;
  let modelLabel: string | null = null;
  const headerMatch = rest.match(/^\[(?:agent-model|model):\s*([^\]]+)\]\r?\n?/i);
  if (headerMatch) {
    const rawModel = headerMatch[1]!.trim();
    const dashParts = rawModel.match(/^([^\s-]+)\s+-\s+(.+)$/);
    modelLabel = dashParts ? `${dashParts[1]}/${dashParts[2]}` : rawModel;
    rest = rest.slice(headerMatch[0].length);
  }
  const isClavisUnconfigured = rest.trimStart().startsWith('[clavis-unconfigured]');
  let streamErrorText: string | null = null;
  const errIdx = rest.indexOf('[stream-error]');
  if (errIdx >= 0) {
    streamErrorText = rest.slice(errIdx + '[stream-error]'.length).trim() || 'Stream closed unexpectedly';
    rest = rest.slice(0, errIdx).trimEnd();
  }
  return { modelLabel, bodyText: rest, isClavisUnconfigured, streamErrorText };
}

function extractFenceProposedCommand(lang: string, code: string): string | null {
  if (!RUNNABLE_LANGS.has(lang.trim().toLowerCase())) return null;
  const trimmed = code.trim();
  if (!trimmed) return null;
  if (!trimmed.includes('\n')) return trimmed.replace(/^\$\s+/, '').trim() || null;
  return trimmed;
}

export function parseAgentTurnSegments(markdown: string): AgentTurnSegment[] {
  const lines = markdown.replace(/\r\n/g, '\n').split('\n');
  const segments: AgentTurnSegment[] = [];
  let proseBuf: string[] = [];
  let inFence = false;
  let fenceLang = '';
  let fenceLines: string[] = [];

  const flushProse = () => {
    const joined = proseBuf.join('\n').trim();
    proseBuf = [];
    if (joined) segments.push({ kind: 'prose', text: joined });
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (!inFence && trimmed.startsWith('```')) {
      flushProse();
      inFence = true;
      fenceLang = trimmed.slice(3).trim();
      fenceLines = [];
      continue;
    }
    if (inFence && trimmed === '```') {
      const code = fenceLines.join('\n');
      segments.push({
        kind: 'code',
        lang: fenceLang,
        code,
        closed: true,
        proposedCommand: extractFenceProposedCommand(fenceLang, code),
      });
      inFence = false;
      fenceLang = '';
      fenceLines = [];
      continue;
    }
    if (inFence) {
      fenceLines.push(line);
      continue;
    }
    if (trimmed.startsWith('VOX_PROPOSED_COMMAND:')) {
      const cmd = trimmed.slice('VOX_PROPOSED_COMMAND:'.length).trim().replace(/^\$\s+/, '').trim();
      if (cmd) {
        flushProse();
        segments.push({ kind: 'proposed', command: cmd });
        continue;
      }
    }
    proseBuf.push(line);
  }
  if (inFence) {
    segments.push({
      kind: 'code',
      lang: fenceLang,
      code: fenceLines.join('\n'),
      closed: false,
      proposedCommand: null,
    });
  }
  flushProse();
  return segments;
}

function renderInlineSpans(text: string): React.ReactNode[] {
  const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g);
  return parts.map((part, idx) => {
    if (part.startsWith('`') && part.endsWith('`') && part.length >= 2) {
      return (
        <code key={idx} data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="rounded bg-bg-elevated px-1 text-text-primary">
          {part.slice(1, -1)}
        </code>
      );
    }
    if (part.startsWith('**') && part.endsWith('**') && part.length >= 4) {
      return (
        <strong key={idx} className="font-semibold text-text-primary">
          {part.slice(2, -2)}
        </strong>
      );
    }
    return <React.Fragment key={idx}>{part}</React.Fragment>;
  });
}

function renderProseBlock(text: string, keyPrefix: string) {
  const nodes: React.ReactNode[] = [];
  const rawLines = text.split('\n');
  let ulItems: string[] = [];
  let olItems: string[] = [];

  const flushLists = (keyIdx: number) => {
    if (ulItems.length > 0) {
      nodes.push(
        <ul key={`ul-${keyIdx}`} className="list-disc space-y-1 pl-4">
          {ulItems.map((item, j) => (
            <li key={j}>{renderInlineSpans(item)}</li>
          ))}
        </ul>,
      );
      ulItems = [];
    }
    if (olItems.length > 0) {
      nodes.push(
        <ol key={`ol-${keyIdx}`} className="list-decimal space-y-1 pl-4">
          {olItems.map((item, j) => (
            <li key={j}>{renderInlineSpans(item)}</li>
          ))}
        </ol>,
      );
      olItems = [];
    }
  };

  rawLines.forEach((line, i) => {
    const t = line.trim();
    if (!t) {
      flushLists(i);
      return;
    }
    if (t.startsWith('#')) {
      flushLists(i);
      nodes.push(
        <div key={i} className="text-sm leading-[1.3] font-semibold text-text-primary">
          {renderInlineSpans(t.replace(/^#+\s*/, ''))}
        </div>,
      );
      return;
    }
    const ulMatch = t.match(/^[-*]\s+(.+)$/);
    if (ulMatch) {
      if (olItems.length > 0) flushLists(i);
      ulItems.push(ulMatch[1]!);
      return;
    }
    const olMatch = t.match(/^\d+\.\s+(.+)$/);
    if (olMatch) {
      if (ulItems.length > 0) flushLists(i);
      olItems.push(olMatch[1]!);
      return;
    }
    flushLists(i);
    nodes.push(<p key={i}>{renderInlineSpans(t)}</p>);
  });
  flushLists(rawLines.length);

  return (
    <div key={keyPrefix} className="flex flex-col gap-1 font-sans text-xs leading-[1.45] font-normal text-text-primary">
      {nodes}
    </div>
  );
}

export interface AgentTurnContentProps {
  block: TermBlockDto;
  showStopInBody?: boolean;
  onRunProposedCommand?: (cmd: string) => void;
  onEditInPrompt?: (cmd: string) => void;
  onOpenSecrets?: () => void;
  onRunSecretsDoctor?: () => void;
  onRetryAgentTurn?: (block: TermBlockDto) => void;
  onCancelAgentTurn?: (block: TermBlockDto) => void;
}

export function AgentTurnContent({
  block,
  showStopInBody = true,
  onRunProposedCommand,
  onEditInPrompt,
  onOpenSecrets,
  onRunSecretsDoctor,
  onRetryAgentTurn,
  onCancelAgentTurn,
}: AgentTurnContentProps) {
  const rawText = Array.isArray(block.output) ? block.output.map((c) => c.text ?? '').join('') : '';
  const { modelLabel, bodyText, isClavisUnconfigured, streamErrorText } = parseAgentHeaderAndBody(rawText);
  const segments = parseAgentTurnSegments(bodyText);
  const isRunning = block.status === 'Running';
  const isRepairTurn = isExplainFixInput(block.input);

  const openKeysAndSecrets = () => {
    try {
      window.localStorage?.setItem('vox_settings_seed', JSON.stringify({ section: 'secrets' }));
      window.dispatchEvent(new CustomEvent('vox-settings-seed'));
    } catch {
      /* ignore storage errors in restricted environments */
    }
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('vox://navigate-surface', { detail: { view: 'settings' } }));
    }
    onOpenSecrets?.();
  };

  const renderProposalCard = (cmd: string, key: string) => (
    <div
      key={key}
      data-testid="term-proposed-command-card"
      className="flex flex-wrap items-center justify-between gap-2 rounded border border-border-strong bg-bg-elevated p-2"
    >
      <code data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="min-w-0 flex-1 break-all text-xs leading-[1.45] font-normal text-text-primary">
        {`$ ${cmd}`}
      </code>
      <div className="flex min-w-0 max-w-full flex-wrap items-center gap-2">
        <Button
          size="xs"
          variant="primary"
          data-testid={isRepairTurn ? 'term-proposed-fix-run' : 'term-proposed-cmd-run'}
          aria-label={isRepairTurn ? `Run Fix ${cmd}` : `Run Command ${cmd}`}
          onClick={(e) => { e.stopPropagation(); onRunProposedCommand?.(cmd); }}
          className="h-6 gap-1 rounded border border-(--color-accent-default) bg-(--color-accent-default) px-2 py-1 text-[11px] font-semibold text-bg-base"
        >
          <Icon.play className="size-3" aria-hidden="true" />
          <span>{isRepairTurn ? 'Run Fix' : 'Run Command'}</span>
        </Button>
        <Button
          size="xs"
          variant="outline"
          data-testid={isRepairTurn ? 'term-proposed-fix-edit' : 'term-proposed-cmd-edit'}
          aria-label={`Edit in Prompt ${cmd}`}
          onClick={(e) => { e.stopPropagation(); onEditInPrompt?.(cmd); }}
          className="h-6 gap-1 rounded border border-border-strong bg-bg-surface px-2 py-1 text-[11px] font-semibold text-text-primary"
        >
          <Icon.edit className="size-3" aria-hidden="true" />
          <span>Edit in Prompt</span>
        </Button>
        <Button
          size="xs"
          variant="ghost"
          data-testid="term-proposed-cmd-copy"
          aria-label={`Copy ${cmd}`}
          onClick={(e) => {
            e.stopPropagation();
            void navigator.clipboard?.writeText?.(cmd)?.catch(() => {});
          }}
          className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] font-semibold text-text-secondary"
        >
          Copy
        </Button>
      </div>
    </div>
  );

  return (
    <div
      data-testid="term-agent-turn-content"
      tabIndex={0}
      aria-live="polite"
      aria-busy={isRunning}
      className="flex max-h-80 flex-col gap-2 overflow-y-auto rounded-md border border-border-subtle bg-bg-base p-2 text-xs leading-[1.45] text-text-primary"
    >
      {modelLabel ? (
        <span
          data-testid="term-agent-model-pill"
          className="inline-flex items-center gap-1 self-start rounded border border-border-strong bg-bg-elevated px-2 py-1 text-[11px] leading-[1.25] font-semibold text-accent"
        >
          <Icon.cpu className="size-3 shrink-0" aria-hidden="true" />
          <span>{modelLabel}</span>
        </span>
      ) : null}

      {isRunning ? (
        <div
          data-testid="term-agent-streaming-status"
          className="flex flex-wrap items-center justify-between gap-2 text-[11px] leading-[1.25] font-semibold text-text-secondary"
        >
          <span className="inline-flex items-center gap-2">
            <span className="size-2 animate-pulse rounded-full bg-(--color-status-info)" aria-hidden="true" />
            <span>
              {bodyText.trim().length === 0
                ? 'Routing prompt through model registry...'
                : modelLabel
                  ? `Streaming response from ${modelLabel}...`
                  : 'Streaming response...'}
            </span>
          </span>
          {showStopInBody && onCancelAgentTurn ? (
            <Button
              size="xs"
              variant="ghost"
              onClick={(e) => { e.stopPropagation(); onCancelAgentTurn(block); }}
              className="h-6 gap-1 rounded border border-(--color-status-fail)/50 px-2 py-1 text-[11px] font-semibold text-(--color-status-fail)"
            >
              <Icon.stop className="size-3" aria-hidden="true" />
              <span>Stop Stream</span>
            </Button>
          ) : null}
        </div>
      ) : null}

      {isClavisUnconfigured ? (
        <div
          data-testid="term-agent-clavis-diagnostic"
          role="alert"
          className="flex flex-col gap-2 rounded-lg border border-(--color-status-warn) bg-bg-elevated p-3"
        >
          <div className="flex items-center gap-2 text-sm leading-[1.3] font-semibold text-text-primary">
            <Icon.lockKeyhole className="size-4 shrink-0 text-(--color-status-warn)" aria-hidden="true" />
            <span>No LLM Provider Configured in Clavis</span>
          </div>
          <p className="font-sans text-xs leading-[1.45] font-normal text-text-secondary">
            Configure OpenRouter, Anthropic, OpenAI, Gemini, or VOX_LOCAL_ENDPOINT in Clavis to stream terminal agent turns. Run vox secrets doctor or open Keys &amp; Secrets.
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="xs"
              variant="primary"
              data-testid="term-clavis-open-secrets"
              aria-label="Open Keys & Secrets in Settings"
              onClick={(e) => { e.stopPropagation(); openKeysAndSecrets(); }}
              className="h-6 gap-1 rounded border border-(--color-accent-default) bg-(--color-accent-default) px-2 py-1 text-[11px] font-semibold text-bg-base"
            >
              <Icon.lockKeyhole className="size-3" aria-hidden="true" />
              <span>Open Keys &amp; Secrets</span>
            </Button>
            <Button
              size="xs"
              variant="outline"
              data-testid="term-clavis-run-doctor"
              aria-label="Run Secrets Doctor command"
              onClick={(e) => {
                e.stopPropagation();
                if (onRunSecretsDoctor) onRunSecretsDoctor();
                else onRunProposedCommand?.('vox secrets doctor');
              }}
              className="h-6 gap-1 rounded border border-border-strong bg-bg-surface px-2 py-1 text-[11px] font-semibold text-text-primary"
            >
              <Icon.terminal className="size-3" aria-hidden="true" />
              <span>Run Secrets Doctor</span>
            </Button>
          </div>
        </div>
      ) : (
        segments.map((seg, idx) => {
          if (seg.kind === 'prose') return renderProseBlock(seg.text, `p-${idx}`);
          if (seg.kind === 'proposed') return renderProposalCard(seg.command, `prop-${idx}`);
          return (
            <div key={`c-${idx}`} className="flex flex-col gap-2">
              <pre
                data-terminal-mono="true"
                style={TERMINAL_MONO_STYLE}
                className="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded border border-border-subtle bg-bg-elevated p-2 text-xs leading-[1.45] text-text-primary"
              >
                {seg.code}
              </pre>
              {seg.closed && seg.proposedCommand ? renderProposalCard(seg.proposedCommand, `cmd-${idx}`) : null}
            </div>
          );
        })
      )}

      {streamErrorText ? (
        <div
          data-testid="term-agent-stream-error"
          role="alert"
          className="flex flex-wrap items-center justify-between gap-2 rounded border border-(--color-status-fail) bg-bg-elevated p-2 text-xs text-(--color-status-fail)"
        >
          <span>{`Agent stream interrupted: ${streamErrorText}. Click Retry Agent Turn to request a fresh response.`}</span>
          {onRetryAgentTurn ? (
            <Button
              size="xs"
              variant="outline"
              data-testid="term-agent-retry"
              aria-label="Retry Agent Turn"
              onClick={(e) => { e.stopPropagation(); onRetryAgentTurn(block); }}
              className="h-6 gap-1 rounded border border-border-strong bg-bg-surface px-2 py-1 text-[11px] font-semibold text-text-primary"
            >
              <Icon.refresh className="size-3" aria-hidden="true" />
              <span>Retry Agent Turn</span>
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
