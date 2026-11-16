import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { voxTransport, type ExecuteOutput } from '../../../transport';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useMeshNodesFull } from '../../../hooks/useMeshNodes';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import type { MeshNode } from '../Mesh/MeshView';
import type { Toast } from '../../../types/tauri';
import { BentoScrollRegion, MasterDetailWorkbench } from '../../ui/BentoGrid';
import { RomanRotaryKnob } from '../../ui/RomanRotaryKnob';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';
import { DiegeticFxStage } from '../../fx/DiegeticFxStage';

export interface PopuliMeshBentoViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
}

type ProbeKey = 'status' | 'registry';
type ProbeState = { kind: 'ok'; out: ExecuteOutput } | { kind: 'error'; message: string };
type PeerFilter = 'all' | 'online' | 'degraded';

const PREF_CADENCE = 'vox.populi.gossip_cadence_ms.v1';
const REFRESH_MS = 5000;

function clampCadence(raw: string | null | undefined, fallback = 250): number {
  const n = raw ? Number(raw) : NaN;
  if (!Number.isFinite(n)) return fallback;
  const snapped = Math.round(n / 50) * 50;
  return Math.max(100, Math.min(2000, snapped));
}

function formatPeerLastSeen(ms?: number): string {
  if (!ms || !Number.isFinite(ms) || ms <= 0) return 'unreported';
  const sec = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (sec < 5) return 'just now';
  if (sec < 60) return `${sec}s ago`;
  if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
  return sec < 86400 ? `${Math.floor(sec / 3600)}h ago` : `${Math.floor(sec / 86400)}d ago`;
}

function parseOverlayFacts(probe: ProbeState | undefined, fallback: string): Array<{ label: string; value: string }> {
  if (!probe) return [{ label: fallback, value: 'Probing...' }];
  if (probe.kind === 'error') return [{ label: 'Transport', value: probe.message }];
  const parts = [probe.out.stdout, probe.out.stderr].filter(Boolean).join(' | ').split(/[|\n]+/).map((s) => s.trim()).filter(Boolean).slice(0, 4);
  if (parts.length === 0) return [{ label: fallback, value: probe.out.exit_code === 0 ? 'Healthy' : `Exit ${probe.out.exit_code}` }];
  return parts.map((tok, i) => {
    const idx = tok.indexOf(':');
    return idx > 0 && idx < tok.length - 1
      ? { label: tok.slice(0, idx).trim(), value: tok.slice(idx + 1).trim() }
      : { label: i === 0 ? fallback : `Segment ${i + 1}`, value: tok };
  });
}

function computePeerPosition(index: number, total: number): { x: number; y: number } {
  if (total === 1) return { x: 615, y: 82 };
  if (total === 2) return index === 0 ? { x: 145, y: 82 } : { x: 615, y: 82 };
  const angle = -Math.PI / 2 + (index / total) * Math.PI * 2;
  return { x: Math.round(380 + Math.cos(angle) * 245), y: Math.round(82 + Math.sin(angle) * 56) };
}

export function PopuliMeshBentoView({ pushToast }: PopuliMeshBentoViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [peerFilter, setPeerFilter] = useState<PeerFilter>('all');
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [gossipCadenceMs, setGossipCadenceMs] = useState<number>(250);
  const [probes, setProbes] = useState<Partial<Record<ProbeKey, ProbeState>>>({});
  const [probing, setProbing] = useState<boolean>(false);
  const mountedRef = useRef<boolean>(true);
  const probeSeqRef = useRef<number>(0);
  const writeSeqRef = useRef<number>(0);
  const confirmedSeqRef = useRef<number>(0);
  const committedCadenceRef = useRef<number>(250);
  const confirmedCadenceRef = useRef<number>(250);

  const handleNodesError = useCallback(
    (err: unknown) => {
      if (!mountedRef.current) return;
      pushToast({ tone: 'warn', title: 'Populi mesh poll failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    },
    [pushToast],
  );
  const { nodes, meta, loading, refresh: refreshNodes } = useMeshNodesFull(REFRESH_MS, { embedded, onError: handleNodesError });

  useEffect(() => {
    mountedRef.current = true;
    let cancelled = false;
    if (typeof voxTransport.getGuiPreference === 'function') {
      void voxTransport.getGuiPreference(PREF_CADENCE).then((raw) => {
        if (cancelled || !mountedRef.current || raw === null || writeSeqRef.current > 0) return;
        const c = clampCadence(raw, 250);
        committedCadenceRef.current = c;
        confirmedCadenceRef.current = c;
        setGossipCadenceMs(c);
      }).catch(() => {});
    }
    return () => {
      cancelled = true;
      mountedRef.current = false;
    };
  }, []);

  const runOverlayProbes = useCallback(async (alsoRefreshNodes = false) => {
    const reqId = ++probeSeqRef.current;
    setProbing(true);
    const specs: Array<{ key: ProbeKey; path: string[] }> = [
      { key: 'status', path: ['populi', 'status'] },
      { key: 'registry', path: ['populi', 'registry-snapshot'] },
    ];
    const next: Partial<Record<ProbeKey, ProbeState>> = {};
    await Promise.all([
      ...specs.map(async (spec) => {
        try {
          const out = await voxTransport.executeCommand(spec.path, {});
          next[spec.key] = { kind: 'ok', out };
        } catch (err) {
          next[spec.key] = { kind: 'error', message: sanitizeErrorForToast(err) };
        }
      }),
      ...(alsoRefreshNodes ? [refreshNodes()] : []),
    ]);
    if (!mountedRef.current || reqId !== probeSeqRef.current) return;
    setProbes(next);
    setProbing(false);
    const failures = Object.values(next).filter((r) => r?.kind === 'error' || (r?.kind === 'ok' && r.out.exit_code !== 0)).length;
    if (failures > 0) {
      pushToast({ tone: 'warn', title: 'Vox Populi', body: `${failures} of 2 QUIC overlay probes reported warnings.`, cause: 'backend-error' });
    }
  }, [pushToast, refreshNodes]);

  useEffect(() => { void runOverlayProbes(false); }, [runOverlayProbes]);

  const commitCadence = useCallback((next: number) => {
    const clamped = clampCadence(String(next), 250);
    const reqId = ++writeSeqRef.current;
    committedCadenceRef.current = clamped;
    setGossipCadenceMs(clamped);
    void voxTransport.setGuiPreference(PREF_CADENCE, String(clamped))
      .then(() => {
        if (reqId > confirmedSeqRef.current) {
          confirmedSeqRef.current = reqId;
          confirmedCadenceRef.current = clamped;
        }
      })
      .catch((err) => {
        if (reqId !== writeSeqRef.current || !mountedRef.current) return;
        committedCadenceRef.current = confirmedCadenceRef.current;
        setGossipCadenceMs(confirmedCadenceRef.current);
        pushToast({ tone: 'warn', title: 'Gossip cadence rollback', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      });
  }, [pushToast]);

  const onlineNodes = useMemo(() => nodes.filter((n) => n.status === 'online'), [nodes]);
  const totalVramGb = useMemo(() => nodes.reduce((acc, n) => acc + (n.vram_gb ?? 0), 0), [nodes]);
  const onlineVramGb = useMemo(() => onlineNodes.reduce((acc, n) => acc + (n.vram_gb ?? 0), 0), [onlineNodes]);
  const visibleNodes = useMemo<MeshNode[]>(() => {
    if (peerFilter === 'online') return nodes.filter((n) => n.status === 'online');
    if (peerFilter === 'degraded') return nodes.filter((n) => n.status !== 'online');
    return nodes;
  }, [nodes, peerFilter]);

  const selectedNode = useMemo(() => (selectedNodeId ? nodes.find((n) => n.id === selectedNodeId) ?? null : null), [nodes, selectedNodeId]);
  const inspectedNode = selectedNode ?? visibleNodes[0] ?? nodes[0] ?? null;

  const healthyProbes = useMemo(() => Object.values(probes).filter((p) => p?.kind === 'ok' && p.out.exit_code === 0).length, [probes]);
  const failedProbes = useMemo(() => Object.values(probes).filter((p) => p?.kind === 'error' || (p?.kind === 'ok' && p.out.exit_code !== 0)).length, [probes]);
  const statusFacts = useMemo(() => parseOverlayFacts(probes.status, 'Overlay'), [probes.status]);
  const registryFacts = useMemo(() => parseOverlayFacts(probes.registry, 'Snapshot'), [probes.registry]);

  return (
    <section data-testid="populi-mesh-bento-view" data-peer-filter={peerFilter} className="@container/surface flex min-h-0 min-w-0 flex-col gap-3.5">
      <RomanSurfaceHeader
        domainTag="COMPUTE · QUIC MESH"
        title="Vox Populi"
        subtitle="Distributed QUIC overlay topology, Ed25519 peer attestation & gossip cadence"
        waxSealBadge={probing ? 'PROBING' : failedProbes > 0 ? 'ERROR' : 'QUORUM OK'}
        icon={<Icon.users className="size-5 text-brass" aria-hidden="true" />}
        gauges={[
          { label: 'Peers', valueText: `${onlineNodes.length}/${nodes.length} UP`,
            ratio: nodes.length > 0 ? onlineNodes.length / nodes.length : 0,
            tone: nodes.length > 0 && onlineNodes.length === nodes.length ? 'emerald' : 'gold',
            purpose: 'Discovered QUIC overlay peers online vs total registered',
            actionHint: 'Click to filter peer matrix to online nodes',
            active: peerFilter === 'online', onClick: () => setPeerFilter((f) => (f === 'online' ? 'all' : 'online')) },
          { label: 'VRAM Pool', valueText: `${onlineVramGb} GB`,
            ratio: totalVramGb > 0 ? onlineVramGb / totalVramGb : 0, tone: 'gold',
            purpose: 'Online pooled GPU memory across attested Populi peers',
            actionHint: 'Click to show all registered mesh peers',
            active: peerFilter === 'all', onClick: () => setPeerFilter('all') },
          { label: 'Probes', valueText: `${healthyProbes}/2 OK`,
            ratio: healthyProbes / 2, tone: failedProbes > 0 ? 'crimson' : 'emerald',
            purpose: 'Populi status and registry-snapshot CLI probes (exit 0)',
            actionHint: 'Click to filter degraded or quarantined peers',
            active: peerFilter === 'degraded', onClick: () => setPeerFilter((f) => (f === 'degraded' ? 'all' : 'degraded')) },
        ]}
      />

      <div data-testid="populi-topology-stage" className="relative w-full overflow-hidden rounded-xl border-2 border-border-strong bg-bg-elevated p-3 shadow-md">
        <div className="pointer-events-none absolute inset-0 bg-cover bg-center opacity-75" style={{ backgroundImage: "url('/assets/romanesque/aqueduct-sluice-cascade-bg.jpg')" }} />
        <DiegeticFxStage preset="sunlit-colonnade" activeBoost={1.25} intensity={1.15} testId="populi-fx-sunlit-colonnade" />
        <div className="relative z-10 mb-1.5 flex flex-wrap items-center justify-between gap-2">
          <span className="rounded border border-border-strong bg-bg-surface/90 px-2.5 py-0.5 font-display text-[9.5px] font-bold tracking-[0.14em] text-text-primary uppercase">
            QUIC + TLS 1.3 OVERLAY RING · SOURCE {(meta.source ?? 'LOCAL').toUpperCase()}
          </span>
          <RomanWaxSealBadge label={`GOSSIP ${gossipCadenceMs}MS`} tone="emerald" />
        </div>
        <div className="relative z-10 w-full">
          <svg data-cv-chart="true" data-testid="populi-quic-topology-svg" viewBox="0 0 760 164" className="block h-40 w-full select-none" role="group" aria-label="Populi QUIC mesh topology">
            <ellipse cx="380" cy="82" rx="245" ry="56" fill="none" stroke="var(--color-border-strong)" strokeOpacity="0.55" strokeWidth="1.4" strokeDasharray="5 4" />
            {visibleNodes.map((peer, idx) => {
              const pos = computePeerPosition(idx, visibleNodes.length);
              const isSelected = inspectedNode?.id === peer.id;
              const isOnline = peer.status === 'online';
              return (
                <g key={peer.id}>
                  <line x1="380" y1="82" x2={pos.x} y2={pos.y} stroke={isOnline ? 'var(--color-primary)' : 'var(--color-status-fail)'} strokeWidth={isSelected ? 2.6 : 1.7} strokeDasharray={isOnline ? undefined : '4 3'} />
                  <g role="button" tabIndex={0} data-testid={`populi-topology-node-${peer.id}`} aria-label={`Inspect mesh peer ${peer.id}`}
                    onClick={() => setSelectedNodeId(peer.id)}
                    onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setSelectedNodeId(peer.id); } }}
                    className="cursor-pointer">
                    <rect x={pos.x - 58} y={pos.y - 20} width="116" height="40" rx="8" fill="var(--color-bg-surface)" stroke={isSelected ? 'var(--color-text-primary)' : isOnline ? 'var(--color-primary)' : 'var(--color-status-fail)'} strokeWidth={isSelected ? 2.4 : 1.6} />
                    <text x={pos.x} y={pos.y - 3} textAnchor="middle" fill="var(--color-text-primary)" fontSize="11" fontWeight="700">{peer.id}</text>
                    <text x={pos.x} y={pos.y + 12} textAnchor="middle" fill="var(--color-text-secondary)" fontSize="9.5">{peer.vram_gb ? `${peer.vram_gb}GB` : peer.status} · {peer.trust_tier ?? 'peer'}</text>
                  </g>
                </g>
              );
            })}
            <g>
              <circle cx="380" cy="82" r="30" fill="var(--color-bg-surface)" stroke="var(--color-primary)" strokeWidth="2.4" />
              <circle cx="380" cy="82" r="24" fill="var(--color-bg-elevated)" stroke="var(--color-border-strong)" strokeWidth="1.2" />
              <text x="380" y="79" textAnchor="middle" fill="var(--color-text-primary)" fontSize="9.5" fontWeight="700">LOCAL HUB</text>
              <text x="380" y="92" textAnchor="middle" fill="var(--color-text-secondary)" fontSize="8.5">{gossipCadenceMs}ms</text>
            </g>
          </svg>
        </div>
      </div>

      <MasterDetailWorkbench
        storageKey="vox.layout.populi.v4"
        detailTitle="Peer Attestation Inspector"
        detailDescription="Ed25519 peer identity, GPU capacity, advertised models, and local registry snapshot."
        hasActiveDetail={Boolean(selectedNode)}
        master={
          <div className="flex min-h-0 flex-1 flex-col gap-3">
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border-subtle bg-bg-elevated/55 p-2.5">
              <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label="Peer status filter">
                {(['all', 'online', 'degraded'] as const).map((f) => (
                  <button key={f} type="button" data-testid={`populi-filter-${f}`} aria-pressed={peerFilter === f} onClick={() => setPeerFilter(f)}
                    className={`rounded-md border px-2.5 py-1 font-display text-[10px] tracking-wider uppercase transition-colors ${
                      peerFilter === f ? 'border-brass bg-brass/20 text-text-primary' : 'border-border-subtle bg-bg-surface text-text-secondary hover:border-brass/45'
                    }`}>
                    {f}
                  </button>
                ))}
              </div>
              <div className="flex items-center gap-3">
                <RomanRotaryKnob
                  ariaLabel="Populi gossip heartbeat cadence"
                  label="Gossip"
                  size="sm"
                  min={100}
                  max={2000}
                  step={50}
                  value={gossipCadenceMs}
                  formatValue={(v) => `${v}ms`}
                  onValueChange={setGossipCadenceMs}
                  onValueCommit={commitCadence}
                />
                <button type="button" data-testid="populi-run-probe-btn" disabled={probing} onClick={() => void runOverlayProbes(true)}
                  className="inline-flex items-center gap-1.5 rounded-lg border border-brass/55 bg-brass/15 px-3 py-1.5 font-display text-[11px] font-bold tracking-wider text-text-primary uppercase transition-colors hover:border-brass hover:bg-brass/25 disabled:opacity-45">
                  <Icon.refresh className="size-3.5 text-brass" aria-hidden="true" />
                  <span>{probing ? 'Probing...' : 'Run Overlay Probe'}</span>
                </button>
              </div>
            </div>

            <BentoScrollRegion ariaLabel="Populi mesh peer matrix" className="max-h-72 flex-1 pr-1">
              {loading && nodes.length === 0 ? (
                <div className="rounded-lg border border-border-subtle bg-bg-elevated/40 p-4 text-xs text-text-muted">Discovering QUIC mesh peers...</div>
              ) : visibleNodes.length === 0 ? (
                <div data-testid="populi-empty-peers" className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border-subtle bg-bg-elevated/35 px-4 py-8 text-center">
                  <RomanMedallion size="sm"><Icon.users className="size-3.5 text-text-primary" aria-hidden="true" /></RomanMedallion>
                  <div className="font-display text-xs font-bold tracking-wider text-text-primary uppercase">No Matching Mesh Peers</div>
                  <p className="max-w-xs text-[11px] text-text-muted">Adjust the status filter or enroll a peer with vox populi join.</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {visibleNodes.map((node) => {
                    const active = inspectedNode?.id === node.id;
                    return (
                      <button key={node.id} type="button" data-testid={`populi-peer-row-${node.id}`} onClick={() => setSelectedNodeId(node.id)}
                        className={`flex w-full flex-wrap items-center justify-between gap-2 rounded-lg border p-2.5 text-left transition-colors ${
                          active ? 'border-brass bg-brass/12' : 'border-border-subtle bg-bg-elevated/45 hover:border-brass/45'
                        }`}>
                        <div className="min-w-0">
                          <div className="font-mono text-xs font-bold text-text-primary">{node.id}</div>
                          <div className="font-mono text-[10.5px] text-text-muted">{node.gpu_summary ?? node.host_triple ?? 'host unreported'}</div>
                        </div>
                        <div className="flex items-center gap-2">
                          {node.vram_gb != null && (
                            <span className="rounded border border-border-subtle bg-bg-surface px-2 py-0.5 font-mono text-[10px] text-text-secondary">{node.vram_gb} GB</span>
                          )}
                          <RomanWaxSealBadge label={node.status} tone={node.status === 'online' ? 'emerald' : 'crimson'} />
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            </BentoScrollRegion>
          </div>
        }
        detail={
          <div data-testid="populi-peer-inspector" className="flex min-h-0 flex-1 flex-col gap-3">
            <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-2">
              <h2 className="font-display text-xs font-bold tracking-wider text-text-primary uppercase">Peer Attestation Inspector</h2>
              {inspectedNode && <RomanWaxSealBadge label={inspectedNode.trust_tier ?? 'unverified'} tone="gold" />}
            </div>

            {inspectedNode ? (
              <div className="space-y-2.5 text-xs">
                <div className="grid grid-cols-2 gap-2">
                  {[
                    ['Peer ID', inspectedNode.id],
                    ['Status', inspectedNode.status],
                    ['Host Triple', inspectedNode.host_triple ?? 'unreported'],
                    ['VRAM Pool', inspectedNode.vram_gb != null ? `${inspectedNode.vram_gb} GB` : 'unreported'],
                    ['Trust Tier', inspectedNode.trust_tier ?? 'unverified'],
                    ['Last Seen', `${formatPeerLastSeen(inspectedNode.last_seen_unix_ms)}${inspectedNode.last_seen_unix_ms ? ` (${inspectedNode.last_seen_unix_ms})` : ''}`],
                  ].map(([k, v]) => (
                    <div key={k} className="rounded-md border border-border-subtle bg-bg-surface p-2">
                      <div className="font-display text-[9px] tracking-wider text-brass uppercase">{k}</div>
                      <div className="mt-0.5 font-mono text-[11px] font-semibold text-text-primary break-words">{v}</div>
                    </div>
                  ))}
                </div>
                <div className="rounded-md border border-border-subtle bg-bg-surface p-2">
                  <div className="font-display text-[9px] tracking-wider text-brass uppercase">GPU Summary</div>
                  <div className="mt-0.5 font-mono text-[11px] text-text-primary">{inspectedNode.gpu_summary ?? 'No accelerator summary reported'}</div>
                </div>
                <div className="rounded-md border border-border-subtle bg-bg-surface p-2">
                  <div className="font-display text-[9px] tracking-wider text-brass uppercase">Advertised Models</div>
                  <div className="mt-1 flex flex-wrap gap-1.5">
                    {inspectedNode.advertised_models && inspectedNode.advertised_models.length > 0 ? (
                      inspectedNode.advertised_models.map((m) => (
                        <span key={m} className="rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[10.5px] text-text-secondary">{m}</span>
                      ))
                    ) : (
                      <span className="font-mono text-[10.5px] text-text-muted">none advertised</span>
                    )}
                  </div>
                </div>
              </div>
            ) : (
              <div className="rounded-lg border border-dashed border-border-subtle bg-bg-surface/60 p-4 text-xs text-text-muted">
                Select a peer in the QUIC topology ring or peer matrix to inspect Ed25519 attestation metadata.
              </div>
            )}

            <div data-testid="populi-snapshot-summary" className="rounded-lg border border-border-subtle bg-bg-surface p-2.5">
              <div className="mb-1.5 font-display text-[10px] font-bold tracking-wider text-text-primary uppercase">Structured Snapshot Summary</div>
              <div className="flex flex-wrap gap-1.5">
                {[...statusFacts, ...registryFacts].map((f, idx) => (
                  <span key={`${f.label}-${idx}`} className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[10px] text-text-secondary">
                    <strong className="font-display text-[8.5px] tracking-wider text-brass uppercase">{f.label}:</strong>
                    <span>{f.value}</span>
                  </span>
                ))}
              </div>
            </div>
          </div>
        }
      />
    </section>
  );
}
