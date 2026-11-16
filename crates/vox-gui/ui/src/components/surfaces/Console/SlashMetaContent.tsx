import React, { useState } from 'react';
import type { TermBlockDto } from '../../../transport';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { TERMINAL_MONO_STYLE } from './terminalTheme';

function parseKeyValLines(lines: string[]): Record<string, string> {
  const map: Record<string, string> = {};
  for (const line of lines) {
    const idx = line.indexOf(':');
    if (idx > 0) {
      map[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
    }
  }
  return map;
}

function deriveBudgetAlert(kv: Record<string, string>): { label: string; cls: string } {
  const tokPair = kv.context_tokens?.match(/(\d+)\s*\/\s*(\d+)/);
  const used = tokPair ? Number(tokPair[1]) : 0;
  const usable = tokPair ? Number(tokPair[2]) : 0;
  const threshMatch = kv.context_tokens?.match(/threshold\s+(\d+)/);
  const threshold = threshMatch ? Number(threshMatch[1]) : 0;
  const spendPair = kv.session_spend_usd?.match(/([\d.]+)\s*\/\s*([\d.]+)/);
  const sessionSpend = spendPair ? Number(spendPair[1]) : 0;
  const sessionBudget = spendPair ? Number(spendPair[2]) : 0;

  if ((sessionBudget > 0 && sessionSpend >= sessionBudget) || (usable > 0 && used >= usable)) {
    return { label: 'Ceiling Reached', cls: 'text-(--color-status-fail)' };
  }
  if (
    (threshold > 0 && used >= threshold) ||
    (usable > 0 && used / usable >= 0.8) ||
    (sessionBudget > 0 && sessionSpend / sessionBudget >= 0.8)
  ) {
    return { label: 'Warning', cls: 'text-(--color-status-warn)' };
  }
  if (kv.context_tokens?.startsWith('unavailable') && kv.spend_usd?.startsWith('unavailable')) {
    return { label: 'Unavailable', cls: 'text-(--color-status-warn)' };
  }
  return { label: 'Healthy', cls: 'text-(--color-status-pass)' };
}

export function SlashMetaContent({ block }: { block: TermBlockDto }) {
  const raw = Array.isArray(block.output) ? block.output.map((c) => c.text ?? '').join('') : '';
  const lines = raw.replace(/\r\n/g, '\n').split('\n').filter((l) => l.trim().length > 0);
  const head = lines[0]?.trim().toLowerCase() ?? '';
  const bodyLines = head.startsWith('[') ? lines.slice(1) : lines;
  const cmd = block.input.trim().toLowerCase();

  const renderContent = () => {
    if (cmd.startsWith('/compact') || head.startsWith('[compact digest')) {
      const countMatch = lines[0]?.match(/(\d+)\s+blocks?/i);
      const digestTitle = countMatch
        ? `Compacted ${countMatch[1]} prior blocks (pinned blocks preserved)`
        : 'Compacted prior blocks (pinned blocks preserved)';
      return (
        <div data-testid="term-compact-digest-card" className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2">
            <span className="inline-flex items-center gap-1 rounded border border-brass/40 bg-overlay-subtle px-2 py-1 text-[11px] leading-[1.25] font-semibold uppercase tracking-wider text-accent">
              <Icon.scissors className="size-3 shrink-0" aria-hidden="true" />
              <span>COMPACTED DIGEST</span>
            </span>
            <span className="text-sm leading-[1.3] font-semibold text-text-primary">
              {digestTitle}
            </span>
          </div>
          <div className="flex flex-col gap-1 text-xs leading-[1.45] text-text-primary">
            {bodyLines.map((line, i) => <div key={i}>{line}</div>)}
          </div>
        </div>
      );
    }

    if (cmd.startsWith('/budget') || head === '[budget]') {
      const kv = parseKeyValLines(bodyLines);
      const spend = kv.session_spend_usd ? `$${kv.session_spend_usd.split('/')[0]?.trim()}` : '$0.0000';
      const ctxVal = kv.context_tokens?.split('(')[0]?.trim() ?? '0 / 0';
      const stratMatch = kv.context_tokens?.match(/strategy\s+([^)]+)/);
      const strategy = stratMatch ? stratMatch[1]!.trim() : 'balanced';
      const alert = deriveBudgetAlert(kv);
      return (
        <div data-testid="term-slash-budget" className="flex flex-col gap-2">
          {kv.updated_per_session_budget_usd ? (
            <div className="inline-flex items-center gap-1 self-start rounded border border-(--color-status-pass) bg-bg-elevated px-2 py-1 text-[11px] leading-[1.25] font-semibold text-(--color-status-pass)">
              {`Updated session token ceiling to $${kv.updated_per_session_budget_usd}`}
            </div>
          ) : null}
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            <div className="rounded border border-border-subtle bg-bg-elevated p-2">
              <div className="text-[11px] leading-[1.25] font-semibold uppercase text-text-secondary">Context Tokens</div>
              <div className="text-xs font-normal text-text-primary">{ctxVal}</div>
            </div>
            <div className="rounded border border-border-subtle bg-bg-elevated p-2">
              <div className="text-[11px] leading-[1.25] font-semibold uppercase text-text-secondary">Budget Strategy</div>
              <div className="text-xs font-normal text-text-primary">{strategy}</div>
            </div>
            <div className="rounded border border-border-subtle bg-bg-elevated p-2">
              <div className="text-[11px] leading-[1.25] font-semibold uppercase text-text-secondary">LLM Spend (USD)</div>
              <div className="text-xs font-normal text-text-primary">{spend}</div>
            </div>
            <div className="rounded border border-border-subtle bg-bg-elevated p-2">
              <div className="text-[11px] leading-[1.25] font-semibold uppercase text-text-secondary">Alert State</div>
              <div data-testid="term-slash-budget-alert" className={`text-xs font-semibold ${alert.cls}`}>
                {alert.label}
              </div>
            </div>
          </div>
        </div>
      );
    }

    if (cmd.startsWith('/model') || head === '[model]') {
      const kv = parseKeyValLines(bodyLines);
      return (
        <div data-testid="term-slash-model" className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2 rounded border border-border-subtle bg-bg-elevated p-2">
            <Icon.cpu className="size-4 shrink-0 text-accent" aria-hidden="true" />
            <span className="text-sm leading-[1.3] font-semibold text-text-primary">
              {kv.active_model ?? 'auto (routed)'}
            </span>
            {kv.status === 'updated' ? (
              <span className="rounded border border-(--color-status-pass) px-2 py-1 text-[11px] leading-[1.25] font-semibold text-(--color-status-pass)">
                {`Pinned active model to ${kv.active_model ?? 'selected'}`}
              </span>
            ) : null}
          </div>
          {bodyLines.map((line, i) => <div key={i} className="text-xs text-text-secondary">{line}</div>)}
        </div>
      );
    }

    if (cmd.startsWith('/skills') || head.startsWith('[skills')) {
      const items = bodyLines.filter((l) => l.trim().startsWith('- '));
      const query = block.input.trim().replace(/^\/skills\b\s*/i, '').trim();
      if (items.length === 0 && raw.toLowerCase().includes('no matching skills')) {
        return (
          <div
            data-testid="term-slash-skills"
            role="status"
            aria-live="polite"
            className="flex flex-col items-center gap-2 rounded border border-border-subtle bg-bg-elevated p-4 text-center"
          >
            <Icon.puzzle className="size-6 text-text-muted" aria-hidden="true" />
            <h3 className="font-display text-sm leading-[1.3] font-semibold uppercase tracking-widest text-text-secondary">
              No matching skills found
            </h3>
            <p className="font-sans text-xs leading-[1.45] text-text-muted">
              {query
                ? `0 installed skills matched "${query}". Run /skills with no filter to list all discovered workspace and built-in skills.`
                : '0 installed skills matched that query. Run /skills with no filter to list discovered skills.'}
            </p>
          </div>
        );
      }
      return (
        <div data-testid="term-slash-skills" className="flex max-h-60 flex-col gap-1 overflow-y-auto">
          {bodyLines.map((line, i) => {
            const clean = line.replace(/^- /, '');
            const sep = clean.indexOf(':');
            const name = sep > 0 ? clean.slice(0, sep).trim() : clean;
            const desc = sep > 0 ? clean.slice(sep + 1).trim() : '';
            return (
              <div
                key={i}
                className="flex items-center justify-between gap-2 rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-xs"
              >
                <span className="shrink-0 font-semibold text-text-primary">{name}</span>
                {desc ? <span className="min-w-0 truncate text-text-secondary">{desc}</span> : null}
              </div>
            );
          })}
        </div>
      );
    }

    if (cmd.startsWith('/memory') || head.startsWith('[memory')) {
      const query = block.input.trim().replace(/^\/memory\b\s*/i, '').trim();
      if (raw.toLowerCase().includes('no recall hits')) {
        return (
          <div
            data-testid="term-slash-memory"
            role="status"
            aria-live="polite"
            className="flex flex-col items-center gap-2 rounded border border-border-subtle bg-bg-elevated p-4 text-center"
          >
            <Icon.memory className="size-6 text-text-muted" aria-hidden="true" />
            <h3 className="font-display text-sm leading-[1.3] font-semibold uppercase tracking-widest text-text-secondary">
              No memory entries recalled
            </h3>
            <p className="font-sans text-xs leading-[1.45] text-text-muted">
              {query
                ? `Mnemosyne returned 0 entries for "${query}". Try a broader keyword or run /memory with no arguments to inspect memory store status.`
                : 'Mnemosyne returned 0 entries. Try a broader keyword or run /memory with no arguments.'}
            </p>
          </div>
        );
      }
      return (
        <div data-testid="term-slash-memory" className="flex flex-col gap-1">
          {bodyLines.map((line, i) => (
            <div key={i} className="rounded border border-border-subtle bg-bg-elevated px-2 py-1 text-xs text-text-primary">
              {line}
            </div>
          ))}
        </div>
      );
    }

    if (cmd.startsWith('/context') || head === '[context]') {
      const kv = parseKeyValLines(bodyLines);
      return (
        <dl data-testid="term-slash-context" className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {Object.entries(kv).map(([k, v]) => (
            <div key={k} className="rounded border border-border-subtle bg-bg-elevated p-2">
              <dt className="text-[11px] leading-[1.25] font-semibold uppercase text-text-secondary">{k}</dt>
              <dd className="break-all text-xs font-normal text-text-primary">{v}</dd>
            </div>
          ))}
        </dl>
      );
    }

    return (
      <pre data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="whitespace-pre-wrap break-words text-xs leading-[1.45] text-text-primary">
        {raw}
      </pre>
    );
  };

  return (
    <div
      data-testid="term-slash-meta-card"
      data-terminal-mono="true"
      style={TERMINAL_MONO_STYLE}
      className="flex flex-col gap-2 rounded-md border border-border-subtle bg-bg-base p-2 text-xs leading-[1.45] text-text-primary"
    >
      {renderContent()}
    </div>
  );
}

export interface TermBlockFeedbackStripProps {
  block: TermBlockDto;
  onFeedback?: (
    block: TermBlockDto,
    verdict: 'accepted' | 'rejected' | 'corrected',
    correctedCommand?: string,
  ) => Promise<boolean | void> | boolean | void;
  onRunCommand?: (cmd: string) => void;
}

export function TermBlockFeedbackStrip({
  block,
  onFeedback,
  onRunCommand,
}: TermBlockFeedbackStripProps) {
  const [verdict, setVerdict] = useState<'accepted' | 'rejected' | 'corrected' | null>(null);
  const [correctOpen, setCorrectOpen] = useState(false);
  const [correctedCommand, setCorrectedCommand] = useState(block.input);
  const [savedCorrected, setSavedCorrected] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (block.credential_prompt === true || block.status === 'Running' || block.status === 'AwaitingInput') {
    return null;
  }

  const submitVerdict = async (next: 'accepted' | 'rejected') => {
    if (!onFeedback) return;
    setBusy(true);
    try {
      const ok = await Promise.resolve(onFeedback(block, next));
      if (ok === true) {
        setVerdict(next);
        setCorrectOpen(false);
      }
    } finally {
      setBusy(false);
    }
  };

  const submitCorrection = async () => {
    const trimmed = correctedCommand.trim();
    if (!trimmed || !onFeedback) return;
    setBusy(true);
    try {
      const ok = await Promise.resolve(onFeedback(block, 'corrected', trimmed));
      if (ok === true) {
        setVerdict('corrected');
        setSavedCorrected(trimmed);
        setCorrectOpen(false);
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={correctOpen ? 'flex w-full flex-col gap-2' : 'ml-auto flex flex-wrap items-center gap-1'}>
      <div role="group" aria-label="MENS trajectory feedback" className="ml-auto flex flex-wrap items-center gap-1">
        <Button
          size="xs"
          variant="ghost"
          data-testid="term-feedback-accept"
          aria-pressed={verdict === 'accepted'}
          aria-label={verdict === 'accepted' ? 'Accepted block for MENS training' : 'Accept block for MENS training'}
          disabled={busy}
          onClick={(e) => { e.stopPropagation(); void submitVerdict('accepted'); }}
          className={`h-6 gap-1 rounded border px-2 py-1 text-[11px] leading-[1.25] font-semibold ${
            verdict === 'accepted'
              ? 'border-(--color-status-pass) bg-overlay-subtle text-(--color-status-pass)'
              : 'border-border-subtle text-text-secondary hover:border-border-strong hover:text-text-primary'
          }`}
        >
          <Icon.check className="size-3" aria-hidden="true" />
          <span>{verdict === 'accepted' ? 'Accepted' : 'Accept'}</span>
        </Button>

        <Button
          size="xs"
          variant="ghost"
          data-testid="term-feedback-reject"
          aria-pressed={verdict === 'rejected'}
          aria-label={verdict === 'rejected' ? 'Rejected block for MENS training' : 'Reject block for MENS training'}
          disabled={busy}
          onClick={(e) => { e.stopPropagation(); void submitVerdict('rejected'); }}
          className={`h-6 gap-1 rounded border px-2 py-1 text-[11px] leading-[1.25] font-semibold ${
            verdict === 'rejected'
              ? 'border-(--color-status-fail) bg-overlay-subtle text-(--color-status-fail)'
              : 'border-border-subtle text-text-secondary hover:border-border-strong hover:text-text-primary'
          }`}
        >
          <Icon.close className="size-3" aria-hidden="true" />
          <span>{verdict === 'rejected' ? 'Rejected' : 'Reject'}</span>
        </Button>

        <Button
          size="xs"
          variant="ghost"
          data-testid="term-feedback-correct"
          aria-expanded={correctOpen}
          aria-pressed={verdict === 'corrected'}
          aria-label={verdict === 'corrected' ? 'Corrected block command for MENS training' : 'Correct block command for MENS training'}
          disabled={busy}
          onClick={(e) => {
            e.stopPropagation();
            if (!correctOpen) setCorrectedCommand(savedCorrected ?? block.input);
            setCorrectOpen((prev) => !prev);
          }}
          className={`h-6 gap-1 rounded border px-2 py-1 text-[11px] leading-[1.25] font-semibold ${
            verdict === 'corrected'
              ? 'border-brass bg-overlay-subtle text-accent'
              : 'border-border-subtle text-text-secondary hover:border-border-strong hover:text-text-primary'
          }`}
        >
          <Icon.edit className="size-3" aria-hidden="true" />
          <span>{verdict === 'corrected' ? 'Corrected' : 'Correct'}</span>
        </Button>

        {verdict === 'corrected' && savedCorrected && !correctOpen && onRunCommand ? (
          <Button
            size="xs"
            variant="outline"
            data-testid="term-feedback-run-corrected"
            aria-label="Run Corrected Command"
            onClick={(e) => { e.stopPropagation(); onRunCommand(savedCorrected); }}
            className="h-6 gap-1 rounded border border-brass bg-bg-surface px-2 py-1 text-[11px] leading-[1.25] font-semibold text-accent"
          >
            <Icon.play className="size-3" aria-hidden="true" />
            <span>Run Corrected Command</span>
          </Button>
        ) : null}
      </div>

      {correctOpen ? (
        <div
          data-testid="term-feedback-correct-bar"
          onClick={(e) => e.stopPropagation()}
          className="flex flex-wrap items-center gap-2 rounded border border-brass bg-bg-elevated p-2"
        >
          <input
            type="text"
            data-testid="term-feedback-correct-input"
            aria-label="Corrected command for MENS training"
            value={correctedCommand}
            onChange={(e) => setCorrectedCommand(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                void submitCorrection();
              } else if (e.key === 'Escape') {
                e.preventDefault();
                setCorrectOpen(false);
              }
            }}
            style={TERMINAL_MONO_STYLE}
            className="h-6 min-w-40 flex-1 rounded border border-border-strong bg-bg-base px-2 py-1 text-xs leading-[1.45] font-normal text-text-primary focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <Button
            size="xs"
            variant="primary"
            data-testid="term-feedback-correct-submit"
            aria-label="Save Correction"
            disabled={!correctedCommand.trim() || busy}
            onClick={() => void submitCorrection()}
            className="h-6 rounded border border-(--color-accent-default) bg-(--color-accent-default) px-2 py-1 text-[11px] leading-[1.25] font-semibold text-bg-base"
          >
            Save Correction
          </Button>
          <Button
            size="xs"
            variant="ghost"
            data-testid="term-feedback-correct-cancel"
            aria-label="Cancel Correction"
            onClick={() => setCorrectOpen(false)}
            className="h-6 rounded border border-border-subtle px-2 py-1 text-[11px] leading-[1.25] font-semibold text-text-secondary"
          >
            Cancel Correction
          </Button>
        </div>
      ) : null}
    </div>
  );
}
