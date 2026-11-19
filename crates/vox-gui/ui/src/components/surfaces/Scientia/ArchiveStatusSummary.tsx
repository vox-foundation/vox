import React, { useCallback, useEffect, useRef, useState } from 'react';
import { listenScientiaQueue } from '../../../transport';
import {
  getArchiveStatus,
  listPublicationManifests,
  type ArchiveStatus,
} from './archiveApi';

interface ArchiveRollup {
  publication_id: string;
  state: string;
  status: ArchiveStatus;
}

function depositedCount(rows: ArchiveRollup[]): { zenodo: number; swh: number; pending: number } {
  let zenodo = 0;
  let swh = 0;
  let pending = 0;
  for (const r of rows) {
    const hasZenodo = Boolean(r.status.zenodo_doi);
    const hasSwh = Boolean(r.status.swhid);
    if (hasZenodo) zenodo += 1;
    if (hasSwh) swh += 1;
    if (!hasZenodo && !hasSwh) pending += 1;
  }
  return { zenodo, swh, pending };
}

/**
 * Compact archive deposit rollup for the Scientia dashboard: samples recent
 * scientia manifests and shows Zenodo / SWHID coverage (read-only).
 */
interface ArchiveStatusSummaryProps {
  onFetchError?: (message: string) => void;
}

export function ArchiveStatusSummary({ onFetchError }: ArchiveStatusSummaryProps) {
  const [rows, setRows] = useState<ArchiveRollup[] | null>(null);
  const [loading, setLoading] = useState(false);
  const primedRef = useRef(false);

  const load = useCallback(async (opts?: { silent?: boolean }) => {
    const showSpinner = !opts?.silent && !primedRef.current;
    if (showSpinner) setLoading(true);
    try {
      const manifests = await listPublicationManifests(8);
      const sample = manifests.slice(0, 5);
      const rollups = await Promise.all(
        sample.map(async (m) => ({
          publication_id: m.publication_id,
          state: m.state,
          status: await getArchiveStatus(m.publication_id),
        })),
      );
      primedRef.current = true;
      setRows(rollups);
    } catch (err) {
      if (!primedRef.current) setRows([]);
      if (!opts?.silent) {
        onFetchError?.(err instanceof Error ? err.message : 'Archive rollup failed');
      }
    } finally {
      if (showSpinner) setLoading(false);
    }
  }, [onFetchError]);

  useEffect(() => {
    void load();
    let unlisten: (() => void) | undefined;
    listenScientiaQueue(() => {
      void load({ silent: true });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        /* outside Tauri — visibility-gated fallback poll covers updates */
      });
    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        void load({ silent: true });
      }
    };
    document.addEventListener('visibilitychange', onVisible);
    const id = window.setInterval(() => {
      if (document.visibilityState !== 'hidden') {
        void load({ silent: true });
      }
    }, 15_000);
    return () => {
      unlisten?.();
      document.removeEventListener('visibilitychange', onVisible);
      window.clearInterval(id);
    };
  }, [load]);

  const counts = rows ? depositedCount(rows) : null;

  return (
    <div className="rounded-xl border border-border-subtle bg-bg-surface p-3.5">
      <div className="mb-2.5 flex items-center justify-between gap-2">
        <span className="font-display text-xs uppercase tracking-wider text-text-muted">
          Archive deposit status
        </span>
      </div>

      {!rows && loading && (
        <div className="text-xs text-text-muted">Loading archive rollup…</div>
      )}

      {rows && counts && (
        <>
          <div className="mb-3 grid grid-cols-3 gap-2">
            <div className="rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-2 text-center">
              <div className="font-mono text-lg font-bold text-(--color-status-pass)">{counts.zenodo}</div>
              <div className="font-sans text-[11px] font-medium uppercase tracking-wider text-text-muted">Zenodo DOI</div>
            </div>
            <div className="rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-2 text-center">
              <div className="font-mono text-lg font-bold text-brass">{counts.swh}</div>
              <div className="font-sans text-[11px] font-medium uppercase tracking-wider text-text-muted">SWHID</div>
            </div>
            <div className="rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-2 text-center">
              <div className="font-mono text-lg font-bold text-(--color-status-warn)">{counts.pending}</div>
              <div
                className="font-sans text-[11px] font-medium uppercase tracking-wider text-text-muted"
                title="Sample of recent publications without Zenodo DOI or SWHID"
              >
                Pending deposit
              </div>
            </div>
          </div>

          {rows.length === 0 ? (
            <div className="text-xs text-text-muted">No scientia publications to sample.</div>
          ) : (
            <ul className="space-y-1.5">
              {rows.map((r) => (
                <li
                  key={r.publication_id}
                  className="flex items-center gap-2 rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-1.5 font-mono text-xs"
                >
                  <span className="truncate font-medium text-text-primary">{r.publication_id}</span>
                  <span className="ml-auto shrink-0 rounded border border-border-subtle bg-bg-surface px-2 py-0.5 text-[11px] text-text-secondary">
                    {r.state}
                  </span>
                  <span className="shrink-0 rounded border border-(--color-status-pass)/35 bg-bg-surface px-2 py-0.5 text-[11px] text-(--color-status-pass)">
                    {r.status.zenodo_doi ? 'DOI' : '—'}
                  </span>
                  <span className="shrink-0 rounded border border-brass/35 bg-bg-surface px-2 py-0.5 text-[11px] text-brass">
                    {r.status.swhid ? 'SWH' : '—'}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </div>
  );
}
