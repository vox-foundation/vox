import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { useMeshNodesFull } from '../../../hooks/useMeshNodes';
import {
  conciliumPilotAction,
  conciliumPlacement,
  invokeCmd,
  listenAgentEvents,
  listenOrchStatus,
  voxTransport,
} from '../../../transport';
import type { HybridComputeNodeCardDto, PopuliPlacementDto } from '../../../types/concilium';
import type { Toast } from '../../../types/tauri';
import type { MeshNode } from '../Mesh/MeshView';
import { synthesizeFallbackComputeNodes } from './HybridComputeNodeCard';
import {
  clampGossipCadence,
  type ProbeKey,
  type ProbeState,
} from './PeerAttestationInspector';

const PREF_CADENCE = 'vox.populi.gossip_cadence_ms.v1';

export function usePopuliTopologyData(params: {
  embedded: boolean;
  entrySurface: 'populi' | 'mesh';
  pushToast: (item: Toast) => void;
  selectedNodeId: string | null;
  selectedWorkerId: string | null;
  peerFilter: 'all' | 'online' | 'degraded';
}) {
  const { embedded, entrySurface, pushToast, selectedNodeId, selectedWorkerId, peerFilter } =
    params;
  const [migratingNodeId, setMigratingNodeId] = useState<string | null>(null);
  const [gossipCadenceMs, setGossipCadenceMs] = useState<number>(250);
  const [probes, setProbes] = useState<Partial<Record<ProbeKey, ProbeState>>>({});
  const [probing, setProbing] = useState<boolean>(false);
  const [placement, setPlacement] = useState<PopuliPlacementDto | null>(null);
  const [pendingCount, setPendingCount] = useState<number | null>(null);

  const mountedRef = useRef<boolean>(true);
  const pushToastRef = useRef(pushToast);
  pushToastRef.current = pushToast;
  const probeSeqRef = useRef<number>(0);
  const writeSeqRef = useRef<number>(0);
  const confirmedSeqRef = useRef<number>(0);
  const confirmedCadenceRef = useRef<number>(250);
  const pushInFlightRef = useRef<boolean>(false);
  const pushPendingRef = useRef<boolean>(false);

  const handleNodesError = useCallback(
    (err: unknown) => {
      if (!mountedRef.current) return;
      pushToastRef.current({
        tone: 'warn',
        title: entrySurface === 'mesh' ? 'Mesh refresh failed' : 'Populi mesh poll failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    },
    [entrySurface],
  );

  // Zero 5000ms blind intervals: embedded: true runs one initial fetch; updates arrive via events or manual refresh.
  const { nodes, meta, loading, refresh: refreshNodes } = useMeshNodesFull(0, {
    embedded: true,
    onError: handleNodesError,
  });

  const refreshPlacement = useCallback(async () => {
    try {
      const dto = await conciliumPlacement();
      if (mountedRef.current && dto && Array.isArray(dto.nodes)) setPlacement(dto);
    } catch {
      // Fallback synthesis handles unmocked test harnesses or offline daemons.
    }
  }, []);

  const refreshQueue = useCallback(async () => {
    try {
      const res = await invokeCmd<{ result?: { pending_count?: number | null } }>(
        'invoke_mcp_tool',
        { tool: 'vox_mesh_queue_stats', args: {} },
      );
      if (mountedRef.current && typeof res?.result?.pending_count === 'number') {
        setPendingCount(res.result.pending_count);
      }
    } catch {
      // Non-fatal: falls back to meta.queue_depth.
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    let cancelled = false;
    void refreshPlacement();
    void refreshQueue();
    if (typeof voxTransport.getGuiPreference === 'function') {
      void voxTransport
        .getGuiPreference(PREF_CADENCE)
        .then((raw) => {
          if (cancelled || !mountedRef.current || raw === null || writeSeqRef.current > 0) return;
          const c = clampGossipCadence(raw, 250);
          confirmedCadenceRef.current = c;
          setGossipCadenceMs(c);
        })
        .catch(() => {});
    }
    return () => {
      cancelled = true;
      mountedRef.current = false;
    };
  }, [refreshPlacement, refreshQueue]);

  useEffect(() => {
    if (embedded) return;
    let disposed = false;
    const unsubs: Array<() => void> = [];
    const drainPush = async () => {
      if (disposed || !mountedRef.current) return;
      if (pushInFlightRef.current) {
        pushPendingRef.current = true;
        return;
      }
      pushInFlightRef.current = true;
      try {
        do {
          pushPendingRef.current = false;
          await Promise.all([refreshPlacement(), refreshQueue(), refreshNodes()]);
        } while (!disposed && mountedRef.current && pushPendingRef.current);
      } finally {
        pushInFlightRef.current = false;
      }
    };
    const onPush = () => {
      void drainPush();
    };
    void listenAgentEvents(onPush)
      .then((u) => (disposed ? u() : unsubs.push(u)))
      .catch(() => {});
    void listenOrchStatus(onPush)
      .then((u) => (disposed ? u() : unsubs.push(u)))
      .catch(() => {});
    return () => {
      disposed = true;
      for (const u of unsubs) u();
    };
  }, [embedded, refreshNodes, refreshPlacement, refreshQueue]);

  const runOverlayProbes = useCallback(
    async (alsoRefresh = false) => {
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
            next[spec.key] = {
              kind: 'ok',
              stdout: out.stdout,
              stderr: out.stderr,
              exit_code: out.exit_code,
            };
          } catch (err) {
            next[spec.key] = { kind: 'error', message: sanitizeErrorForToast(err) };
          }
        }),
        ...(alsoRefresh ? [refreshNodes(), refreshPlacement(), refreshQueue()] : []),
      ]);
      if (!mountedRef.current || reqId !== probeSeqRef.current) return;
      setProbes(next);
      setProbing(false);
      const failures = Object.values(next).filter(
        (r) => r?.kind === 'error' || (r?.kind === 'ok' && r.exit_code !== 0),
      ).length;
      if (failures > 0) {
        pushToastRef.current({
          tone: 'warn',
          title: 'Vox Populi',
          body: `${failures} of 2 QUIC overlay probes reported warnings.`,
          cause: 'backend-error',
        });
      }
    },
    [refreshNodes, refreshPlacement, refreshQueue],
  );

  useEffect(() => {
    void runOverlayProbes(false);
  }, [runOverlayProbes]);

  const commitCadence = useCallback((next: number) => {
    const clamped = clampGossipCadence(String(next), 250);
    const reqId = ++writeSeqRef.current;
    setGossipCadenceMs(clamped);
    void voxTransport
      .setGuiPreference(PREF_CADENCE, String(clamped))
      .then(() => {
        if (reqId > confirmedSeqRef.current) {
          confirmedSeqRef.current = reqId;
          confirmedCadenceRef.current = clamped;
        }
      })
      .catch((err) => {
        if (reqId !== writeSeqRef.current || !mountedRef.current) return;
        setGossipCadenceMs(confirmedCadenceRef.current);
        pushToastRef.current({
          tone: 'warn',
          title: 'Gossip cadence rollback',
          body: sanitizeErrorForToast(err),
          cause: 'backend-error',
        });
      });
  }, []);

  const computeNodes = useMemo(
    () =>
      placement && placement.nodes.length > 0
        ? placement.nodes
        : synthesizeFallbackComputeNodes(nodes),
    [placement, nodes],
  );
  const allWorkers = useMemo(() => computeNodes.flatMap((n) => n.docked_workers), [computeNodes]);
  const activeWorker = useMemo(
    () => allWorkers.find((w) => w.agent_id === selectedWorkerId) ?? allWorkers[0] ?? null,
    [allWorkers, selectedWorkerId],
  );

  const handleMigrateWorkerHere = useCallback(
    async (node: HybridComputeNodeCardDto) => {
      if (!activeWorker) return;
      setMigratingNodeId(node.node_id);
      try {
        const res = await conciliumPilotAction({
          action: 'swap_agent_compute',
          agent_id: activeWorker.agent_id,
          target_tier: node.tier,
          target_node_id: node.node_id,
        });
        if (mountedRef.current) {
          pushToastRef.current({
            tone: res.ok ? 'ok' : 'warn',
            title: res.ok ? 'Worker compute migrated' : 'Migration rejected',
            body: res.message || `@${activeWorker.roster_name} -> ${node.display_name}`,
            cause: res.ok ? 'backend-ok' : 'backend-error',
          });
        }
        await refreshPlacement();
      } catch (err) {
        if (mountedRef.current) {
          pushToastRef.current({
            tone: 'warn',
            title: 'Compute migration failed',
            body: sanitizeErrorForToast(err),
            cause: 'backend-error',
          });
        }
      } finally {
        if (mountedRef.current) setMigratingNodeId(null);
      }
    },
    [activeWorker, refreshPlacement],
  );

  const onlineNodes = useMemo(() => nodes.filter((n) => n.status === 'online'), [nodes]);
  const totalVramGb = useMemo(() => nodes.reduce((acc, n) => acc + (n.vram_gb ?? 0), 0), [nodes]);
  const onlineVramGb = useMemo(
    () => onlineNodes.reduce((acc, n) => acc + (n.vram_gb ?? 0), 0),
    [onlineNodes],
  );
  const visibleNodes = useMemo<MeshNode[]>(
    () =>
      peerFilter === 'online'
        ? nodes.filter((n) => n.status === 'online')
        : peerFilter === 'degraded'
          ? nodes.filter((n) => n.status !== 'online')
          : nodes,
    [nodes, peerFilter],
  );
  const inspectedNode = useMemo(
    () =>
      (selectedNodeId ? (nodes.find((n) => n.id === selectedNodeId) ?? null) : null) ??
      visibleNodes[0] ??
      nodes[0] ??
      null,
    [nodes, selectedNodeId, visibleNodes],
  );
  const healthyProbes = useMemo(
    () => Object.values(probes).filter((p) => p?.kind === 'ok' && p.exit_code === 0).length,
    [probes],
  );
  const failedProbes = useMemo(
    () =>
      Object.values(probes).filter(
        (p) => p?.kind === 'error' || (p?.kind === 'ok' && p.exit_code !== 0),
      ).length,
    [probes],
  );
  const refreshAll = useCallback(async () => {
    await Promise.all([refreshNodes(), refreshPlacement(), refreshQueue()]);
  }, [refreshNodes, refreshPlacement, refreshQueue]);

  return {
    nodes,
    meta,
    loading,
    placement,
    computeNodes,
    activeWorker,
    migratingNodeId,
    gossipCadenceMs,
    setGossipCadenceMs,
    commitCadence,
    probes,
    probing,
    runOverlayProbes,
    handleMigrateWorkerHere,
    onlineNodes,
    totalVramGb,
    onlineVramGb,
    visibleNodes,
    inspectedNode,
    healthyProbes,
    failedProbes,
    effectivePending: pendingCount ?? meta.queue_depth ?? null,
    dispatchConfigured: (meta.source ?? placement?.source) === 'control_plane',
    refreshAll,
  };
}
