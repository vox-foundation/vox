import React, { useState } from 'react';
import { axisConnectRemote, axisDisconnectRemote } from '../../../transport';
import { parseAxisError } from '../../../lib/axisError';
import { useBackendStatus } from '../../../hooks/useBackendStatus';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../../ui/Dialog';
import type { Toast } from '../../../types/tauri';

interface PendingPin {
  fingerprint: string;
  pinned: string | null;
}

export function formatFingerprint(fp: string): string {
  return (fp.match(/.{1,4}/g) ?? []).join(' ');
}

const INPUT_CLASS =
  'rounded-md border border-border-subtle bg-black/20 px-2 py-1.5 font-mono text-[12px] text-text-primary focus:outline-hidden focus:ring-1 focus:ring-brass/50';
const BUTTON_CLASS =
  'self-start rounded-md border border-brass/40 bg-brass/10 px-3 py-1.5 font-display text-[11px] uppercase tracking-[0.12em] text-brass hover:bg-brass/20 disabled:opacity-40';

export function ServerConnectionSection({ pushToast }: { pushToast: (t: Toast) => void }) {
  const { status, setStatus } = useBackendStatus();
  const [url, setUrl] = useState(status.url ?? '');
  const [token, setToken] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingPin | null>(null);

  const connect = async (acceptFingerprint: string | null) => {
    setBusy(true);
    setError(null);
    try {
      const next = await axisConnectRemote(url.trim(), token.trim() || null, acceptFingerprint);
      setPending(null);
      setToken('');
      setStatus(next);
      pushToast({ tone: 'ok', title: 'Connected to server', body: next.url ?? undefined, cause: 'backend-ok' });
    } catch (e) {
      const err = parseAxisError(e);
      if (err?.code === 'tls_pin_required' && err.fingerprint) {
        setPending({ fingerprint: err.fingerprint, pinned: null });
      } else if (err?.code === 'tls_pin_mismatch' && err.fingerprint) {
        setPending({ fingerprint: err.fingerprint, pinned: err.pinned ?? null });
      } else {
        setPending(null);
        setError(err?.message ?? String(e));
      }
    } finally {
      setBusy(false);
    }
  };

  const disconnect = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await axisDisconnectRemote());
      pushToast({ tone: 'info', title: 'Disconnected — using this machine', cause: 'backend-ok' });
    } catch (e) {
      setError(parseAxisError(e)?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-labelledby="server-connection-heading" data-testid="server-connection">
      <h2 id="server-connection-heading" className="font-display text-[18px] font-semibold tracking-tight text-text-primary">
        Connect to server
      </h2>
      <p className="mt-0.5 text-[11px] text-text-muted">
        Attach Axis to a Vox server on your network. Surfaces the server does not serve stay hidden.
      </p>

      {status.mode === 'remote' ? (
        <div className="mt-4 flex flex-col gap-2" data-testid="server-connected">
          <div className="text-[12px] text-text-secondary">
            Connected to <span className="font-mono">{status.url}</span>
          </div>
          <div className="text-[11px] text-text-muted">
            Profile: {status.profile?.profiles.join(', ') || 'unknown'} · server v{status.profile?.server_version ?? '?'}
          </div>
          <button type="button" className={BUTTON_CLASS} onClick={() => void disconnect()} disabled={busy}>
            Disconnect
          </button>
        </div>
      ) : (
        <form
          className="mt-4 flex max-w-md flex-col gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            void connect(null);
          }}
        >
          <label className="flex flex-col gap-1 text-[11px] text-text-muted">
            Server URL
            <input className={INPUT_CLASS} value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://vox.lan:3921" />
          </label>
          <label className="flex flex-col gap-1 text-[11px] text-text-muted">
            Bearer token
            <input
              className={INPUT_CLASS}
              type="password"
              autoComplete="off"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="leave empty to reuse the saved token"
            />
          </label>
          <button type="submit" className={BUTTON_CLASS} disabled={busy || url.trim() === ''}>
            {busy ? 'Connecting…' : 'Connect'}
          </button>
        </form>
      )}

      {error && (
        <p role="alert" data-testid="server-connection-error" className="mt-3 text-[11px] text-red-400">
          {error}
        </p>
      )}

      <Dialog open={pending !== null} onOpenChange={(open) => { if (!open) setPending(null); }}>
        <DialogContent>
          <DialogTitle>Confirm server certificate</DialogTitle>
          <DialogDescription>
            {pending?.pinned
              ? 'This server certificate changed since you last connected. Continue only if you expected it, for example after rotating the certificate.'
              : 'First connection to this server. Compare this fingerprint with the one shown on the server before trusting it.'}
          </DialogDescription>
          {pending?.pinned && (
            <p className="mt-3 font-mono text-[11px] text-text-muted">Previously pinned: {formatFingerprint(pending.pinned)}</p>
          )}
          <p data-testid="server-fingerprint" className="mt-3 break-all font-mono text-[12px] text-text-primary">
            SHA3-256 {formatFingerprint(pending?.fingerprint ?? '')}
          </p>
          <div className="mt-4 flex justify-end gap-2">
            <button type="button" className={BUTTON_CLASS} onClick={() => setPending(null)}>
              Cancel
            </button>
            <button type="button" className={BUTTON_CLASS} disabled={busy} onClick={() => pending && void connect(pending.fingerprint)}>
              Trust and connect
            </button>
          </div>
        </DialogContent>
      </Dialog>
    </section>
  );
}
