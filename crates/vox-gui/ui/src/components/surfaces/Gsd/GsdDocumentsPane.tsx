import React, { useState } from 'react';
import { invokeCmd } from '../../../transport';
import { Glass } from '../../ui/Glass';
import { Button } from '../../ui/Button';
import { StatusPill } from '../../ui/StatusPill';
import { EmptyState } from '../../ui/EmptyState';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { Toast } from '../../../types/tauri';

export interface GsdDocumentHeader {
  doc_id: string;
  milestone_id: string;
  phase_number?: number | null;
  kind: string;
  rel_path: string;
  cas_hash: string;
  byte_len: number;
  updated_at_ms: number;
}

interface GsdDocumentsPaneProps {
  documents: GsdDocumentHeader[];
  activeMilestoneId: string;
  pushToast: (t: Toast) => void;
  onStored: () => void;
}

export function GsdDocumentsPane({
  documents,
  activeMilestoneId,
  pushToast,
  onStored,
}: GsdDocumentsPaneProps) {
  const [selectedPath, setSelectedPath] = useState<string>('');
  const [docContent, setDocContent] = useState<string>('');
  const [newPath, setNewPath] = useState<string>('.planning/NOTES.md');
  const [newBody, setNewBody] = useState<string>('# Notes\n');
  const [busy, setBusy] = useState(false);

  const openDocument = async (docIdOrPath: string) => {
    setBusy(true);
    try {
      const res = await invokeCmd<{ document?: GsdDocumentHeader; content?: string }>('gsd_get_document', {
        docIdOrPath,
        doc_id_or_path: docIdOrPath,
      });
      setSelectedPath(res?.document?.rel_path ?? docIdOrPath);
      setDocContent(res?.content ?? '');
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Failed to load GSD document',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusy(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const relPath = newPath.trim();
    if (!relPath) return;
    setBusy(true);
    try {
      await invokeCmd('gsd_store_document', {
        relPath,
        rel_path: relPath,
        content: newBody,
        milestoneId: activeMilestoneId,
        milestone_id: activeMilestoneId,
      });
      pushToast({ tone: 'ok', title: 'Stored GSD CAS document', body: relPath, cause: 'backend-ok' });
      setSelectedPath(relPath);
      setDocContent(newBody);
      onStored();
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Failed to store GSD document',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Glass className="p-4 flex flex-col gap-3" data-testid="gsd-documents-pane">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-[13px] font-semibold text-text-primary">
          Stored Planning Artifacts (Arca CAS)
        </h3>
        <StatusPill tone="info" size="xs" label={`${documents.length} docs`} />
      </div>

      {documents.length === 0 ? (
        <EmptyState
          title="No GSD documents indexed"
          description="Store a .planning markdown artifact below or initialize a milestone."
        />
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
          {documents.map((doc) => (
            <button
              key={doc.doc_id || doc.rel_path}
              type="button"
              data-testid={`gsd-doc-item-${doc.rel_path}`}
              onClick={() => void openDocument(doc.doc_id || doc.rel_path)}
              className="flex flex-col items-start gap-1 rounded-lg border border-border-subtle bg-overlay-subtle/40 px-3 py-2 text-left hover:bg-overlay-subtle transition-colors"
            >
              <div className="flex w-full items-center justify-between gap-2">
                <span className="text-[12px] font-medium text-text-primary truncate">
                  {doc.rel_path}
                </span>
                <StatusPill tone="neutral" size="xs" label={doc.kind} />
              </div>
              <div className="flex w-full items-center justify-between text-[10px] text-text-muted">
                <span>CAS {doc.cas_hash.slice(0, 12)}</span>
                <span>{doc.byte_len} B</span>
              </div>
            </button>
          ))}
        </div>
      )}

      {selectedPath && (
        <div className="rounded-lg border border-border-subtle bg-bg-base/70 p-3 flex flex-col gap-1.5">
          <div className="text-[11px] font-semibold text-text-secondary">{selectedPath}</div>
          <pre
            data-testid="gsd-doc-preview"
            className="max-h-48 overflow-auto whitespace-pre-wrap text-[11px] text-text-primary"
          >
            {docContent}
          </pre>
        </div>
      )}

      <form onSubmit={(e) => void handleSave(e)} className="flex flex-col gap-2 pt-2 border-t border-border-subtle">
        <div className="flex flex-wrap items-center gap-2">
          <input
            type="text"
            aria-label="Document relative path"
            value={newPath}
            onChange={(e) => setNewPath(e.target.value)}
            placeholder=".planning/NOTES.md"
            className="flex-1 min-w-[200px] rounded-md border border-border-subtle bg-bg-base px-2.5 py-1.5 text-[12px] text-text-primary"
          />
          <Button type="submit" size="sm" variant="primary" disabled={busy}>
            Store CAS Doc
          </Button>
        </div>
        <textarea
          aria-label="Document markdown content"
          rows={3}
          value={newBody}
          onChange={(e) => setNewBody(e.target.value)}
          className="w-full rounded-md border border-border-subtle bg-bg-base px-2.5 py-1.5 text-[12px] text-text-primary"
        />
      </form>
    </Glass>
  );
}
