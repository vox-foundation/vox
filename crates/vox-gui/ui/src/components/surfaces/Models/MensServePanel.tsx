import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Glass } from '../../ui/Glass';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { Toast } from '../../../types/tauri';

interface MensServeStatus {
  running: boolean;
  port: number | null;
  default_port: number;
}

interface MensServePanelProps {
  pushToast: (item: Toast) => void;
}

/**
 * Start/stop panel for `vox mens serve` — the supervised long-lived child
 * process `CommandCardsView` cannot host (its `execute_command` seam awaits
 * process exit; `serve` never exits — see `crates/vox-gui/src/commands/mens_serve.rs`
 * and `CommandCardsView.tsx`'s doc comment).
 *
 * On mount this only reads status (`mens_serve_status`) — it never starts a
 * server itself. Starting/stopping is exclusively user-driven, via the
 * Start/Stop buttons below. Live reachability (the "online · N models" line)
 * is the same VoxLocal row `BackendAvailability` renders elsewhere, read here
 * from `inference_provider_status` — reusing the one live probe rather than
 * polling `mens_serve_status` for reachability too.
 */
export function MensServePanel({ pushToast }: MensServePanelProps) {
  const [model, setModel] = useState('');
  const [port, setPort] = useState<number | null>(null);
  const [running, setRunning] = useState(false);
  const [busy, setBusy] = useState(false);
  const [reachable, setReachable] = useState<boolean | null>(null);
  const [modelCount, setModelCount] = useState(0);

  useEffect(() => {
    let cancelled = false;
    invoke<MensServeStatus>('mens_serve_status')
      .then((s) => {
        if (cancelled || !s) return;
        setRunning(s.running);
        setPort(s.port ?? s.default_port);
      })
      .catch((err) => {
        if (!cancelled) {
          pushToast({ tone: 'warn', title: 'mens serve status failed', body: sanitizeErrorForToast(err) });
        }
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let cancelled = false;
    const poll = () => {
      invoke<{ provider: string; local_reachable: boolean | null; local_models: string[] }[]>(
        'inference_provider_status'
      )
        .then((statuses) => {
          if (cancelled) return;
          const row = Array.isArray(statuses) ? statuses.find((s) => s.provider === 'VoxLocal') : undefined;
          setReachable(row?.local_reachable ?? null);
          setModelCount(row?.local_models.length ?? 0);
        })
        .catch(() => {
          // Best-effort; BackendAvailability elsewhere already surfaces failures.
        });
    };
    poll();
    const id = setInterval(poll, 5000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  const start = async () => {
    setBusy(true);
    try {
      const status = await invoke<MensServeStatus>('mens_serve_start', {
        model,
        port: port ?? null,
      });
      setRunning(status.running);
      setPort(status.port ?? status.default_port);
      pushToast({ tone: 'ok', title: 'mens serve started', body: model });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'mens serve start failed', body: sanitizeErrorForToast(err) });
    } finally {
      setBusy(false);
    }
  };

  const stop = async () => {
    setBusy(true);
    try {
      const status = await invoke<MensServeStatus>('mens_serve_stop');
      setRunning(status.running);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'mens serve stop failed', body: sanitizeErrorForToast(err) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Glass className="p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">
          Vox Mens Serve
        </div>
        <div className="flex items-center gap-2">
          <span
            aria-hidden
            className={`size-2 rounded-full ${reachable ? 'bg-emerald-400' : 'bg-zinc-600'}`}
          />
          <span className="font-mono text-[10px] text-text-muted">
            {reachable ? `online · ${modelCount} models` : running ? 'starting…' : 'offline'}
          </span>
        </div>
      </div>
      <div className="flex flex-wrap items-end gap-2">
        <label className="flex flex-col gap-1 text-[10px] uppercase tracking-widest text-text-muted flex-1 min-w-[12rem]">
          Model
          <input
            aria-label="Model"
            type="text"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="/path/to/mens/run/dir"
            disabled={running}
            className="rounded-lg border border-border-subtle bg-transparent px-2 py-1.5 font-mono text-xs text-text-primary"
          />
        </label>
        <label className="flex flex-col gap-1 text-[10px] uppercase tracking-widest text-text-muted w-24">
          Port
          <input
            aria-label="Port"
            type="number"
            value={port ?? ''}
            onChange={(e) => setPort(e.target.value ? Number(e.target.value) : null)}
            disabled={running}
            className="rounded-lg border border-border-subtle bg-transparent px-2 py-1.5 font-mono text-xs text-text-primary"
          />
        </label>
        {running ? (
          <button
            type="button"
            onClick={stop}
            disabled={busy}
            className="rounded-lg border border-border-subtle px-3 py-1.5 text-[10px] uppercase tracking-widest hover:bg-overlay-subtle disabled:opacity-50"
          >
            Stop
          </button>
        ) : (
          <button
            type="button"
            onClick={start}
            disabled={busy || !model}
            className="rounded-lg border border-border-subtle px-3 py-1.5 text-[10px] uppercase tracking-widest hover:bg-overlay-subtle disabled:opacity-50"
          >
            Start
          </button>
        )}
      </div>
    </Glass>
  );
}
