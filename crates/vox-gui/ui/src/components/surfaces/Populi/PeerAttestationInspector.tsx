import React from 'react';
import type { MeshNode } from '../Mesh/MeshView';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';

export type ProbeKey = 'status' | 'registry';
export type ProbeState =
  | { kind: 'ok'; stdout: string; stderr: string; exit_code: number }
  | { kind: 'error'; message: string };

export function clampGossipCadence(raw: string | null | undefined, fallback = 250): number {
  const n = raw ? Number(raw) : NaN;
  return Number.isFinite(n) ? Math.max(100, Math.min(2000, Math.round(n / 50) * 50)) : fallback;
}

export function formatPeerLastSeen(ms?: number): string {
  if (!ms || !Number.isFinite(ms) || ms <= 0) return 'unreported';
  const sec = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (sec < 5) return 'just now';
  if (sec < 60) return `${sec}s ago`;
  if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
  return sec < 86400 ? `${Math.floor(sec / 3600)}h ago` : `${Math.floor(sec / 86400)}d ago`;
}

export function parseOverlayFacts(
  probe: ProbeState | undefined,
  fallback: string,
): Array<{ label: string; value: string }> {
  if (!probe) return [{ label: fallback, value: 'Probing...' }];
  if (probe.kind === 'error') return [{ label: 'Transport', value: probe.message }];
  const parts = [probe.stdout, probe.stderr]
    .filter(Boolean)
    .join(' | ')
    .split(/[|\n]+/)
    .map((s) => s.trim())
    .filter(Boolean)
    .slice(0, 4);
  if (parts.length === 0) {
    return [{ label: fallback, value: probe.exit_code === 0 ? 'Healthy' : `Exit ${probe.exit_code}` }];
  }
  return parts.map((tok, i) => {
    const idx = tok.indexOf(':');
    return idx > 0 && idx < tok.length - 1
      ? { label: tok.slice(0, idx).trim(), value: tok.slice(idx + 1).trim() }
      : { label: i === 0 ? fallback : `Segment ${i + 1}`, value: tok };
  });
}

export function PeerAttestationInspector({
  inspectedNode,
  probes,
}: {
  inspectedNode: MeshNode | null;
  probes: Partial<Record<ProbeKey, ProbeState>>;
}) {
  const statusFacts = parseOverlayFacts(probes.status, 'Overlay');
  const registryFacts = parseOverlayFacts(probes.registry, 'Snapshot');
  return (
    <div data-testid="populi-peer-inspector" className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-2">
        <h2 className="font-display text-xs font-bold tracking-wider text-text-primary uppercase">
          Peer Attestation Inspector
        </h2>
        {inspectedNode && (
          <RomanWaxSealBadge label={inspectedNode.trust_tier ?? 'unverified'} tone="gold" />
        )}
      </div>
      {inspectedNode ? (
        <div className="space-y-2 text-xs">
          <div className="grid grid-cols-2 gap-2">
            {[
              ['Peer ID', inspectedNode.id],
              ['Status', inspectedNode.status],
              ['Host Triple', inspectedNode.host_triple ?? 'unreported'],
              ['VRAM Pool', inspectedNode.vram_gb != null ? `${inspectedNode.vram_gb} GB` : 'unreported'],
              ['Trust Tier', inspectedNode.trust_tier ?? 'unverified'],
              ['Last Seen', formatPeerLastSeen(inspectedNode.last_seen_unix_ms)],
            ].map(([k, v]) => (
              <div key={k} className="rounded-md border border-border-subtle bg-bg-surface p-2">
                <div className="font-display text-[9px] tracking-wider text-brass uppercase">{k}</div>
                <div className="mt-0.5 font-mono text-[11px] font-semibold text-text-primary break-words">
                  {v}
                </div>
              </div>
            ))}
          </div>
          <div className="rounded-md border border-border-subtle bg-bg-surface p-2">
            <div className="font-display text-[9px] tracking-wider text-brass uppercase">
              GPU Summary
            </div>
            <div className="mt-0.5 font-mono text-[11px] text-text-primary">
              {inspectedNode.gpu_summary ?? 'No accelerator summary reported'}
            </div>
          </div>
          <div className="rounded-md border border-border-subtle bg-bg-surface p-2">
            <div className="font-display text-[9px] tracking-wider text-brass uppercase">
              Advertised Models
            </div>
            <div className="mt-1 flex flex-wrap gap-1.5">
              {inspectedNode.advertised_models && inspectedNode.advertised_models.length > 0 ? (
                inspectedNode.advertised_models.map((m, idx) => (
                  <span
                    key={`${m}-${idx}`}
                    className="rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[10.5px] text-text-secondary"
                  >
                    {m}
                  </span>
                ))
              ) : (
                <span className="font-mono text-[10.5px] text-text-muted">none advertised</span>
              )}
            </div>
          </div>
        </div>
      ) : (
        <div className="rounded-lg border border-dashed border-border-subtle bg-bg-surface/60 p-4 text-xs text-text-muted">
          Select a peer in the QUIC topology ring or peer matrix to inspect Ed25519 attestation
          metadata.
        </div>
      )}
      <div
        data-testid="populi-snapshot-summary"
        className="rounded-lg border border-border-subtle bg-bg-surface p-2.5"
      >
        <div className="mb-1.5 font-display text-[10px] font-bold tracking-wider text-text-primary uppercase">
          Structured Snapshot Summary
        </div>
        <div className="flex flex-wrap gap-1.5">
          {[...statusFacts, ...registryFacts].map((f, idx) => (
            <span
              key={`${f.label}-${idx}`}
              className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[10px] text-text-secondary"
            >
              <strong className="font-display text-[8.5px] tracking-wider text-brass uppercase">
                {f.label}:
              </strong>
              <span>{f.value}</span>
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}
