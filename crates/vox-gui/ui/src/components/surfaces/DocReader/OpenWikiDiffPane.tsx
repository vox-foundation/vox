import React, { useMemo } from 'react';
import { Button } from '../../ui/Button';
import { splitDiffColumns, type KbHealProposalDto } from './docReaderHelpers';

export interface OpenWikiDiffPaneProps {
  proposal: KbHealProposalDto;
  editingMarkdown: string | null;
  busyAction: string | null;
  onChangeMarkdown: (val: string | null) => void;
  onAction: (
    action: 'wiki_heal_apply' | 'wiki_heal_demote' | 'wiki_heal_reject',
    proposalId: string,
  ) => void;
}

export function OpenWikiDiffPane({
  proposal,
  editingMarkdown,
  busyAction,
  onChangeMarkdown,
  onAction,
}: OpenWikiDiffPaneProps) {
  const diffCols = useMemo(
    () =>
      splitDiffColumns(
        proposal.unified_diff ?? '',
        editingMarkdown ?? proposal.patched_markdown ?? '',
      ),
    [proposal.unified_diff, proposal.patched_markdown, editingMarkdown],
  );

  return (
    <div
      data-testid="openwiki-diff-inspector"
      className="space-y-2.5 rounded border border-border-subtle bg-bg-elevated p-3 lg:col-span-7"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2">
        <div className="min-w-0">
          <div className="font-mono text-xs font-semibold text-text-primary">
            {proposal.doc_path}
          </div>
          <div className="font-mono text-[11px] text-text-muted">
            Proposal {proposal.id} · Citations:{' '}
            {proposal.evidence_citations.length > 0
              ? proposal.evidence_citations.join(', ')
              : 'none'}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-1">
          {(
            [
              ['RewriteGuard', proposal.gate_report_json?.rewrite_guard_passed],
              ['DocLint', proposal.gate_report_json?.doc_lint_passed],
              ['VoxDiagram', proposal.gate_report_json?.vox_diagram_passed],
              ['Doctest', proposal.gate_report_json?.doctest_passed],
            ] as const
          ).map(([label, ok]) => (
            <span
              key={label}
              className={`rounded border px-1.5 py-0.5 font-mono text-[11px] ${
                ok === false
                  ? 'border-(--color-status-error)/50 bg-(--color-status-error)/12 text-(--color-status-error)'
                  : 'border-(--color-status-pass)/45 bg-(--color-status-pass)/12 text-(--color-status-pass)'
              }`}
            >
              {label}: {ok === false ? 'FAIL' : 'PASS'}
            </span>
          ))}
        </div>
      </div>

      {editingMarkdown !== null ? (
        <textarea
          aria-label="Edit proposed documentation markdown"
          rows={8}
          value={editingMarkdown}
          onChange={(e) => onChangeMarkdown(e.target.value)}
          className="w-full rounded border border-border-subtle bg-bg-surface p-2 font-mono text-xs text-text-primary"
        />
      ) : (
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          <div className="rounded border border-border-subtle bg-bg-surface p-2">
            <div className="mb-1 font-mono text-[11px] uppercase text-(--color-status-error)">
              Original / Removed
            </div>
            <pre className="max-h-40 overflow-auto whitespace-pre-wrap font-mono text-[11px] text-text-secondary">
              {diffCols.removed.length > 0 ? diffCols.removed.join('\n') : '(no deletions)'}
            </pre>
          </div>
          <div className="rounded border border-border-subtle bg-bg-surface p-2">
            <div className="mb-1 font-mono text-[11px] uppercase text-(--color-status-pass)">
              Proposed / Grounded Patch
            </div>
            <pre className="max-h-40 overflow-auto whitespace-pre-wrap font-mono text-[11px] text-text-primary">
              {diffCols.added.length > 0 ? diffCols.added.join('\n') : proposal.patched_markdown}
            </pre>
          </div>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-1.5 pt-1">
        <Button
          data-testid={`openwiki-apply-proposal-${proposal.id}`}
          className="text-[11px]"
          disabled={busyAction !== null}
          onClick={() => onAction('wiki_heal_apply', proposal.id)}
        >
          Apply Patch
        </Button>
        <Button
          data-testid={`openwiki-edit-proposal-${proposal.id}`}
          className="text-[11px]"
          onClick={() =>
            onChangeMarkdown(editingMarkdown === null ? proposal.patched_markdown : null)
          }
        >
          {editingMarkdown === null ? 'Edit' : 'Preview Diff'}
        </Button>
        <Button
          data-testid={`openwiki-demote-proposal-${proposal.id}`}
          className="text-[11px]"
          disabled={busyAction !== null}
          onClick={() => onAction('wiki_heal_demote', proposal.id)}
        >
          Demote to Legacy
        </Button>
        <Button
          data-testid={`openwiki-reject-proposal-${proposal.id}`}
          className="text-[11px]"
          disabled={busyAction !== null}
          onClick={() => onAction('wiki_heal_reject', proposal.id)}
        >
          Reject
        </Button>
      </div>
    </div>
  );
}
