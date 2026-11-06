import React, { useState } from 'react';
import {
  guestShareMint,
  guestShareModerate,
  sessionExportBundle,
  sessionImportBundle,
  sessionImportExternal,
  sessionListExternal,
  type DiscoveredExternalSessionDto,
  type ExternalSessionMessageDto,
  type ImportedExternalSessionDto,
  type VoxSessionExportBundleV1Dto,
} from '../../../transport';

export interface SessionInteropModalProps {
  readonly sessionId: string;
  readonly messages?: readonly ExternalSessionMessageDto[];
  readonly onSessionImported?: (imported: ImportedExternalSessionDto) => void;
  readonly onClose: () => void;
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
  const [shareToken, setShareToken] = useState<string | null>(null);
  const [moderationStatus, setModerationStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const handleDiscover = () => {
    if (busy || typeof sessionListExternal !== 'function') return;
    setBusy(true);
    void sessionListExternal(sourceHarness)
      .then((rows) => {
        setDiscoveredSessions(rows ?? []);
        if (rows?.[0]?.external_session_id) {
          setExternalSessionId(rows[0].external_session_id);
        }
      })
      .finally(() => setBusy(false));
  };

  const handleImport = () => {
    if (busy) return;
    setBusy(true);
    void sessionImportExternal({
      sourceHarness,
      externalSessionId,
      rawJsonl,
    })
      .then((res) => {
        setImportedResult(res);
        onSessionImported?.(res);
      })
      .finally(() => setBusy(false));
  };

  const handleExportBundle = () => {
    if (busy) return;
    setBusy(true);
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
      .finally(() => setBusy(false));
  };

  const handleImportBundle = () => {
    if (busy || !exportedBundle || typeof sessionImportBundle !== 'function') return;
    setBusy(true);
    void sessionImportBundle(exportedBundle)
      .then((res) => {
        setModerationStatus(`Restored bundle ${res.session_id} (${res.message_count} turns)`);
      })
      .finally(() => setBusy(false));
  };

  const handleMintGuestShare = () => {
    if (busy || typeof guestShareMint !== 'function') return;
    setBusy(true);
    void guestShareMint({ sessionId, permission: 'suggest_prompt' })
      .then((res) => {
        setShareToken(res.token);
      })
      .finally(() => setBusy(false));
  };

  const handleApproveSubmission = () => {
    if (busy || typeof guestShareModerate !== 'function') return;
    setBusy(true);
    void guestShareModerate({ submissionId: 'gsub_demo_1', approve: true })
      .then((res) => {
        setModerationStatus(`Guest submission ${res.submission_id}: ${res.status}`);
      })
      .finally(() => setBusy(false));
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
        {shareToken && (
          <button
            type="button"
            data-testid="session-interop-moderate-approve-btn"
            disabled={busy}
            onClick={handleApproveSubmission}
            className="rounded border border-(--color-status-pass)/40 bg-(--color-status-pass)/15 px-2 py-1 text-[11px] text-(--color-status-pass)"
          >
            Approve Guest Prompt
          </button>
        )}
      </div>

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
