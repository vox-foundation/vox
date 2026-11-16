import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  guestShareList,
  guestShareMint,
  guestShareModerate,
  sessionExportBundle,
  sessionImportBundle,
  sessionImportExternal,
  sessionListExternal,
  type DiscoveredExternalSessionDto,
  type ExternalSessionMessageDto,
  type GuestInviteSummaryDto,
  type GuestSubmissionSummaryDto,
  type ImportedExternalSessionDto,
  type VoxSessionExportBundleV1Dto,
} from '../../../transport';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';

export interface SessionInteropModalProps {
  readonly sessionId: string;
  readonly messages?: readonly ExternalSessionMessageDto[];
  readonly onSessionImported?: (imported: ImportedExternalSessionDto) => void;
  readonly onClose: () => void;
}

function readUploadedText(file: File): Promise<string> {
  if (typeof file.text === 'function') {
    return file.text();
  }
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(typeof reader.result === 'string' ? reader.result : '');
    reader.onerror = () => reject(reader.error ?? new Error('Failed to read uploaded file'));
    reader.readAsText(file);
  });
}

export const SessionInteropModal: React.FC<SessionInteropModalProps> = ({
  sessionId,
  messages = [],
  onSessionImported,
  onClose,
}) => {
  const [sourceHarness, setSourceHarness] = useState<'claude' | 'codex' | 'cursor'>('claude');
  const [externalSessionId, setExternalSessionId] = useState('claude-session-1');
  const [rawJsonl, setRawJsonl] = useState(
    '{"role":"user","content":"Import this session from external CLI"}\n{"role":"assistant","content":"Session imported into Vox with full provenance"}',
  );
  const [importedResult, setImportedResult] = useState<ImportedExternalSessionDto | null>(null);
  const [exportedBundle, setExportedBundle] = useState<VoxSessionExportBundleV1Dto | null>(null);
  const [discoveredSessions, setDiscoveredSessions] = useState<DiscoveredExternalSessionDto[]>([]);
  const [invites, setInvites] = useState<GuestInviteSummaryDto[]>([]);
  const [submissions, setSubmissions] = useState<GuestSubmissionSummaryDto[]>([]);
  const [shareToken, setShareToken] = useState<string | null>(null);
  const [moderationStatus, setModerationStatus] = useState<string | null>(null);
  const [interopError, setInteropError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const refreshGuestShares = useCallback(() => {
    if (typeof guestShareList !== 'function') return;
    void guestShareList(sessionId)
      .then((res) => {
        setInvites(Array.isArray(res?.invites) ? res.invites : []);
        setSubmissions(Array.isArray(res?.submissions) ? res.submissions : []);
      })
      .catch((err: unknown) => {
        setInteropError(sanitizeErrorForToast(err));
      });
  }, [sessionId]);

  useEffect(() => {
    refreshGuestShares();
  }, [refreshGuestShares]);

  const handleDiscover = () => {
    if (busy || typeof sessionListExternal !== 'function') return;
    setBusy(true);
    setInteropError(null);
    void sessionListExternal(sourceHarness)
      .then((rows) => {
        setDiscoveredSessions(rows ?? []);
        if (rows?.[0]?.external_session_id) {
          setExternalSessionId(rows[0].external_session_id);
        }
      })
      .catch((err: unknown) => setInteropError(sanitizeErrorForToast(err)))
      .finally(() => setBusy(false));
  };

  const handleImport = () => {
    if (busy) return;
    setBusy(true);
    setInteropError(null);
    void sessionImportExternal({
      sourceHarness,
      externalSessionId,
      rawJsonl,
    })
      .then((res) => {
        setImportedResult(res);
        onSessionImported?.(res);
      })
      .catch((err: unknown) => setInteropError(sanitizeErrorForToast(err)))
      .finally(() => setBusy(false));
  };

  const handleExportBundle = () => {
    if (busy) return;
    setBusy(true);
    setInteropError(null);
    void sessionExportBundle({
      sessionId,
      title: `Session ${sessionId}`,
      messages:
        messages.length > 0
          ? [...messages]
          : [{ role: 'user', content: 'Exported Vox session turn' }],
    })
      .then((bundle) => {
        setExportedBundle(bundle);
      })
      .catch((err: unknown) => setInteropError(sanitizeErrorForToast(err)))
      .finally(() => setBusy(false));
  };

  const handleImportBundle = () => {
    if (busy || !exportedBundle || typeof sessionImportBundle !== 'function') return;
    setBusy(true);
    setInteropError(null);
    void sessionImportBundle(exportedBundle)
      .then((res) => {
        setModerationStatus(`Restored bundle ${res.session_id} (${res.message_count} turns)`);
      })
      .catch((err: unknown) => setInteropError(sanitizeErrorForToast(err)))
      .finally(() => setBusy(false));
  };

  const handleMintGuestShare = () => {
    if (busy || typeof guestShareMint !== 'function') return;
    setBusy(true);
    setInteropError(null);
    void guestShareMint({ sessionId, permission: 'suggest_prompt' })
      .then((res) => {
        setShareToken(res.token);
        refreshGuestShares();
      })
      .catch((err: unknown) => setInteropError(sanitizeErrorForToast(err)))
      .finally(() => setBusy(false));
  };

  const handleModerateSubmission = (submissionId: string, approve: boolean) => {
    if (busy || typeof guestShareModerate !== 'function') return;
    setBusy(true);
    setInteropError(null);
    void guestShareModerate({ submissionId, approve })
      .then((res) => {
        setModerationStatus(`Guest submission ${res.submission_id}: ${res.status}`);
        refreshGuestShares();
      })
      .catch((err: unknown) => setInteropError(sanitizeErrorForToast(err)))
      .finally(() => setBusy(false));
  };

  const handleFileSelected = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setInteropError(null);
    setBusy(true);
    void readUploadedText(file)
      .then(async (text) => {
        const trimmed = text.trim();
        if (file.name.endsWith('.json') || (trimmed.startsWith('{') && !trimmed.includes('\n{'))) {
          try {
            const parsed = JSON.parse(trimmed) as Partial<VoxSessionExportBundleV1Dto>;
            if (parsed && parsed.schema_version === 1 && Array.isArray(parsed.messages)) {
              const bundle: VoxSessionExportBundleV1Dto = {
                schema_version: 1,
                session_id: parsed.session_id ?? sessionId,
                title: parsed.title ?? (file.name.replace(/\.json$/i, '') || `Session ${sessionId}`),
                messages: parsed.messages,
              };
              setExportedBundle(bundle);
              if (typeof sessionImportBundle === 'function') {
                const imported = await sessionImportBundle(bundle);
                setModerationStatus(
                  `Restored bundle ${imported.session_id} (${imported.message_count} turns)`,
                );
              }
              return;
            }
          } catch {
            // Fall through and populate rawJsonl for line-delimited JSONL payloads
          }
        }
        setRawJsonl(text);
        if (file.name) {
          setExternalSessionId(file.name.replace(/\.(jsonl|json)$/i, '') || externalSessionId);
        }
      })
      .catch((err: unknown) => setInteropError(sanitizeErrorForToast(err)))
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <section
      data-testid="session-interop-modal"
      aria-label="External CLI Session Import and Export"
      className="flex flex-col gap-2.5 rounded-lg border border-border-subtle bg-bg-base/95 p-3 font-mono text-[11px] text-text-secondary"
    >
      <header className="flex items-center justify-between border-b border-border-subtle pb-1.5">
        <span className="font-display text-[11px] font-semibold uppercase tracking-[0.13em] text-brass">
          CLI Session Interop (~/.claude · ~/.codex · ~/.cursor)
        </span>
        <button
          type="button"
          data-testid="session-interop-close"
          onClick={onClose}
          className="rounded border border-border-subtle px-2 py-0.5 text-[11px] text-text-muted hover:text-text-primary"
        >
          Close
        </button>
      </header>

      <div className="flex flex-wrap items-center gap-1.5">
        {(['claude', 'codex', 'cursor'] as const).map((harness) => (
          <button
            key={harness}
            type="button"
            data-testid={`session-interop-harness-${harness}`}
            aria-pressed={sourceHarness === harness}
            onClick={() => setSourceHarness(harness)}
            className={`rounded border px-2 py-0.5 text-[11px] uppercase ${
              sourceHarness === harness
                ? 'border-brass/50 bg-brass/20 text-brass'
                : 'border-border-subtle bg-overlay-subtle text-text-muted'
            }`}
          >
            {harness}
          </button>
        ))}
        <input
          type="text"
          data-testid="session-interop-ext-id"
          aria-label="External session identifier"
          value={externalSessionId}
          onChange={(e) => setExternalSessionId(e.target.value)}
          className="min-w-0 flex-1 rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 text-[11px] text-text-primary"
        />
        <input
          ref={fileInputRef}
          type="file"
          accept=".jsonl,.json"
          data-testid="session-interop-file-input"
          aria-label="Load session JSONL or bundle file"
          onChange={handleFileSelected}
          className="sr-only"
        />
        <button
          type="button"
          data-testid="session-interop-file-trigger"
          disabled={busy}
          onClick={() => fileInputRef.current?.click()}
          className="rounded border border-border-subtle bg-overlay-subtle px-2 py-0.5 text-[11px] text-text-secondary hover:border-brass/40 hover:text-brass"
        >
          Load .jsonl / .json File
        </button>
      </div>

      <textarea
        rows={2}
        data-testid="session-interop-jsonl-input"
        aria-label="External session JSONL transcript"
        value={rawJsonl}
        onChange={(e) => setRawJsonl(e.target.value)}
        className="w-full rounded border border-border-subtle bg-overlay-subtle p-1.5 font-mono text-[11px] text-text-primary"
      />

      <div className="flex flex-wrap items-center justify-between gap-2">
        <button
          type="button"
          data-testid="session-interop-discover-btn"
          disabled={busy}
          onClick={handleDiscover}
          className="rounded border border-border-subtle bg-overlay-subtle px-2 py-1 text-[11px] text-text-secondary hover:border-brass/40 hover:text-brass"
        >
          Discover Local Sessions
        </button>
        <button
          type="button"
          data-testid="session-interop-import-btn"
          disabled={busy}
          onClick={handleImport}
          className="rounded border border-brass/40 bg-brass/15 px-2.5 py-1 text-[11px] text-brass hover:bg-brass/25"
        >
          Import External Session
        </button>
        <button
          type="button"
          data-testid="session-interop-export-btn"
          disabled={busy}
          onClick={handleExportBundle}
          className="rounded border border-border-subtle bg-overlay-subtle px-2.5 py-1 text-[11px] text-text-secondary hover:border-brass/40 hover:text-brass"
        >
          Export Portable JSON Bundle
        </button>
        {exportedBundle && (
          <button
            type="button"
            data-testid="session-interop-restore-bundle-btn"
            disabled={busy}
            onClick={handleImportBundle}
            className="rounded border border-brass/40 bg-brass/15 px-2 py-1 text-[11px] text-brass hover:bg-brass/25"
          >
            Re-Import Bundle
          </button>
        )}
        <button
          type="button"
          data-testid="session-interop-share-mint-btn"
          disabled={busy}
          onClick={handleMintGuestShare}
          className="rounded border border-border-subtle bg-overlay-subtle px-2 py-1 text-[11px] text-text-secondary hover:border-brass/40 hover:text-brass"
        >
          Mint Guest Share Link
        </button>
      </div>

      <div
        data-testid="session-interop-guest-submissions"
        className="space-y-1.5 rounded border border-border-subtle bg-overlay-subtle p-2 text-[11px]"
      >
        <div className="flex items-center justify-between text-[11px] uppercase tracking-[0.08em] text-text-muted">
          <span>Guest Submissions ({submissions.length})</span>
          <span>Active Invites: {invites.length}</span>
        </div>
        {submissions.length === 0 ? (
          <div
            data-testid="session-interop-empty-submissions"
            className="text-[11px] text-text-muted"
          >
            No pending guest submissions
          </div>
        ) : (
          <div className="space-y-1">
            {submissions.map((sub) => (
              <div
                key={sub.submission_id}
                data-testid={`session-interop-submission-${sub.submission_id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded border border-border-subtle bg-bg-base/70 px-2 py-1"
              >
                <div className="min-w-0 flex-1 truncate text-text-primary">
                  <span className="font-semibold text-brass">{sub.guest_name}</span>:{' '}
                  <span>{sub.prompt}</span>{' '}
                  <span className="text-[11px] text-text-muted">({sub.status})</span>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <button
                    type="button"
                    data-testid={`session-interop-approve-${sub.submission_id}`}
                    disabled={busy}
                    onClick={() => handleModerateSubmission(sub.submission_id, true)}
                    className="rounded border border-(--color-status-pass)/40 bg-(--color-status-pass)/15 px-2 py-0.5 text-[11px] text-(--color-status-pass)"
                  >
                    Approve
                  </button>
                  <button
                    type="button"
                    data-testid={`session-interop-reject-${sub.submission_id}`}
                    disabled={busy}
                    onClick={() => handleModerateSubmission(sub.submission_id, false)}
                    className="rounded border border-(--color-status-fail)/40 bg-(--color-status-fail)/15 px-2 py-0.5 text-[11px] text-(--color-status-fail)"
                  >
                    Reject
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {interopError && (
        <div
          data-testid="session-interop-error"
          className="rounded border border-(--color-status-fail)/40 bg-(--color-status-fail)/10 px-2 py-1 text-[11px] text-(--color-status-fail)"
        >
          {interopError}
        </div>
      )}

      {discoveredSessions.length > 0 && (
        <div
          data-testid="session-interop-discovered-list"
          className="rounded border border-border-subtle bg-overlay-subtle px-2 py-1 text-[11px] text-text-secondary"
        >
          Discovered {discoveredSessions.length} session(s):{' '}
          {discoveredSessions.map((s) => s.external_session_id).join(', ')}
        </div>
      )}

      {shareToken && (
        <div
          data-testid="session-interop-share-token"
          className="rounded border border-brass/30 bg-brass/10 px-2 py-1 text-[11px] text-brass"
        >
          Guest Share Token: {shareToken}
        </div>
      )}

      {moderationStatus && (
        <div
          data-testid="session-interop-moderation-status"
          className="rounded border border-(--color-status-pass)/30 bg-(--color-status-pass)/10 px-2 py-1 text-[11px] text-(--color-status-pass)"
        >
          {moderationStatus}
        </div>
      )}

      {importedResult && (
        <div
          data-testid="session-interop-import-result"
          className="rounded border border-(--color-status-pass)/30 bg-(--color-status-pass)/10 px-2 py-1 text-[11px] text-(--color-status-pass)"
        >
          Imported {importedResult.messages.length} turns from {importedResult.source_harness} (
          {importedResult.external_session_id})
        </div>
      )}

      {exportedBundle && (
        <div
          data-testid="session-interop-export-result"
          className="rounded border border-brass/30 bg-brass/10 px-2 py-1 text-[11px] text-brass"
        >
          Exported bundle v{exportedBundle.schema_version} ({exportedBundle.messages.length} turns)
        </div>
      )}
    </section>
  );
};
