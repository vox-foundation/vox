import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  conciliumBoard,
  conciliumPilotAction,
  conciliumSnapshot,
  listenAgentEvents,
  type AgentEventFrame,
} from '../../../transport';
import type {
  ConciliumPilotActionInput,
  ConciliumSnapshotDto,
} from '../../../types/concilium';
import type { Agent } from '../../../types/dashboard';
import type { SubAgentNode } from '../SubAgents/types';
import { NowPlayingStrip } from './NowPlayingStrip';
import { CausalSwimlanePanel } from './CausalSwimlanePanel';
import { SharedSequenceBoard } from './SharedSequenceBoard';
import { LiveAgentLens } from './LiveAgentLens';
import {
  DEFAULT_SNAPSHOT,
  applyAgentEventFrame,
  applyPilotActionToSnapshot,
  buildLanesFromLegacy,
  mergeBoardTail,
} from './conciliumLaneModel';

export interface ConciliumViewProps {
  initialFocus?: 'swimlanes' | 'board';
  legacyAgents?: Agent[];
  subAgentNodes?: SubAgentNode[];
  selectedAgentId?: string | null;
  onSelectAgent?: (id: string) => void;
  pushToast?: (msg: string, kind?: 'info' | 'error') => void;
  gamifyEnabled?: boolean;
  renderSelectedAgentExtra?: (selectedAgentId: string) => React.ReactNode;
}

export function ConciliumView({
  initialFocus = 'swimlanes',
  legacyAgents,
  subAgentNodes,
  selectedAgentId: controlledSelectedId,
  onSelectAgent,
  pushToast,
  renderSelectedAgentExtra,
}: ConciliumViewProps) {
  const [snapshot, setSnapshot] = useState<ConciliumSnapshotDto>(DEFAULT_SNAPSHOT);
  const [hasRemoteSnapshot, setHasRemoteSnapshot] = useState(false);
  const [focusMode, setFocusMode] = useState<'swimlanes' | 'board'>(initialFocus);
  const [selectedChannel, setSelectedChannel] = useState<string>('#all');
  const [internalSelectedId, setInternalSelectedId] = useState<string | null>(
    controlledSelectedId ?? null,
  );
  const [isLive, setIsLive] = useState(true);
  const [lastReceipt, setLastReceipt] = useState<string | null>(null);

  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);
  const hasRemoteSnapshotRef = useRef(false);
  hasRemoteSnapshotRef.current = hasRemoteSnapshot;
  const pushToastRef = useRef(pushToast);
  pushToastRef.current = pushToast;
  const legacyRef = useRef({ legacyAgents, subAgentNodes });
  legacyRef.current = { legacyAgents, subAgentNodes };

  const refreshConcilium = useCallback(() => {
    const reqId = ++refreshSeqRef.current;
    conciliumSnapshot()
      .then((snap) => {
        if (!mountedRef.current || reqId !== refreshSeqRef.current) return;
        if (snap && Array.isArray(snap.lanes) && snap.lanes.length > 0) {
          hasRemoteSnapshotRef.current = true;
          setHasRemoteSnapshot(true);
          setSnapshot((prev) => ({
            ...snap,
            board_tail:
              snap.board_tail.length > 0 ? snap.board_tail : prev.board_tail,
          }));
        }
      })
      .catch(() => {});

    conciliumBoard({ limit: 50 })
      .then((entries) => {
        if (!mountedRef.current || reqId !== refreshSeqRef.current) return;
        if (Array.isArray(entries) && entries.length > 0) {
          setSnapshot((prev) => {
            const merged = mergeBoardTail(prev.board_tail, entries);
            const maxSeq = merged.reduce((acc, e) => Math.max(acc, e.seq), prev.head_seq);
            return { ...prev, head_seq: maxSeq, board_tail: merged };
          });
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    refreshConcilium();
    return () => {
      mountedRef.current = false;
    };
  }, [refreshConcilium]);

  useEffect(() => {
    if (!isLive) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listenAgentEvents((frame: AgentEventFrame) => {
      if (disposed || !mountedRef.current) return;
      setSnapshot((prev) => applyAgentEventFrame(prev, frame));
    })
      .then((u) => {
        if (disposed) u();
        else unlisten = u;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [isLive]);

  const effectiveLanes = useMemo(() => {
    const synthesized = buildLanesFromLegacy(legacyAgents, subAgentNodes);
    return !hasRemoteSnapshot && synthesized && synthesized.length > 0
      ? synthesized
      : snapshot.lanes;
  }, [hasRemoteSnapshot, legacyAgents, subAgentNodes, snapshot.lanes]);

  const activeSelectedId =
    controlledSelectedId !== undefined
      ? controlledSelectedId
      : (internalSelectedId ?? effectiveLanes[0]?.agent_id ?? null);
  const selectedLane =
    activeSelectedId != null
      ? (effectiveLanes.find((l) => l.agent_id === activeSelectedId) ?? effectiveLanes[0] ?? null)
      : null;
  const matchingLegacyAgent =
    legacyAgents?.find((a) => a.id === selectedLane?.agent_id) ?? null;

  const handleSelectAgent = useCallback(
    (id: string) => {
      setInternalSelectedId(id);
      onSelectAgent?.(id);
    },
    [onSelectAgent],
  );

  const handlePilotAction = useCallback(async (input: ConciliumPilotActionInput) => {
    let serverSeq: number | undefined;
    let receiptText = `${input.action} -> ${input.agent_id}`;
    try {
      const res = await conciliumPilotAction(input);
      if (res) {
        serverSeq = res.seq;
        if (res.message) {
          receiptText = `${res.message} (#seq-${res.seq})`;
          if (mountedRef.current) {
            pushToastRef.current?.(res.message, res.ok === false ? 'error' : 'info');
          }
        }
        if (res.ok === false) {
          if (mountedRef.current) setLastReceipt(receiptText);
          return;
        }
      }
    } catch {
      // Offline / mock fallback continues with local optimistic lane update.
    }

    if (!mountedRef.current) return;
    const seedLanes = !hasRemoteSnapshotRef.current
      ? buildLanesFromLegacy(legacyRef.current.legacyAgents, legacyRef.current.subAgentNodes)
      : null;
    hasRemoteSnapshotRef.current = true;
    setHasRemoteSnapshot(true);
    setSnapshot((prev) => applyPilotActionToSnapshot(prev, input, seedLanes, serverSeq));
    setLastReceipt(
      serverSeq != null
        ? receiptText
        : `${input.action} -> ${input.agent_id} (local)`,
    );
  }, []);

  const handleSteerFromBoard = useCallback(
    (targetId: string, directive: string, ch: string) =>
      handlePilotAction({
        action: 'steer_agent',
        agent_id: targetId,
        channel: ch,
        directive,
      }),
    [handlePilotAction],
  );

  const swimlanesEl = (
    <CausalSwimlanePanel
      lanes={effectiveLanes}
      wakeupArcs={snapshot.wakeup_arcs}
      windowDurationMs={snapshot.window_duration_ms}
      selectedAgentId={selectedLane?.agent_id ?? null}
      onSelectAgent={handleSelectAgent}
    />
  );
  const boardEl = (
    <SharedSequenceBoard
      entries={snapshot.board_tail}
      selectedChannel={selectedChannel}
      onSelectChannel={setSelectedChannel}
      selectedAgentId={selectedLane?.agent_id ?? null}
      selectedAgentName={selectedLane?.roster_name ?? null}
      onSteerSubmit={handleSteerFromBoard}
    />
  );

  return (
    <div
      data-slot="concilium-view"
      data-testid="concilium-view"
      data-focus-mode={focusMode}
      className="@container/surface flex h-full min-h-0 min-w-0 flex-col gap-3 bg-[#0B0D12] p-3 text-slate-100"
    >
      <NowPlayingStrip
        activeWorkers={effectiveLanes.filter((l) => l.status === 'working').length}
        parkedWorkers={
          effectiveLanes.filter(
            (l) => l.status === 'parked' || l.status === 'waiting' || l.tok_per_sec === 0,
          ).length
        }
        headSeq={snapshot.head_seq}
        activeLocks={effectiveLanes.filter((l) => Boolean(l.file_lock_prefix)).length}
        tickerText={snapshot.now_playing_ticker}
        isLive={isLive}
        focusMode={focusMode}
        onToggleLive={() => setIsLive((v) => !v)}
        onRefresh={refreshConcilium}
        onFocusModeChange={setFocusMode}
      />
      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 xl:grid-cols-12">
        <div className="flex min-h-0 min-w-0 flex-col gap-3 xl:col-span-7">
          {focusMode === 'board' ? (
            <>
              {boardEl}
              {swimlanesEl}
            </>
          ) : (
            <>
              {swimlanesEl}
              {boardEl}
            </>
          )}
        </div>
        <div className="flex min-h-0 min-w-0 flex-col gap-3 xl:col-span-5">
          <LiveAgentLens
            lane={selectedLane}
            legacyAgent={matchingLegacyAgent}
            lastActionReceipt={lastReceipt}
            onPilotAction={handlePilotAction}
          >
            {selectedLane && renderSelectedAgentExtra
              ? renderSelectedAgentExtra(selectedLane.agent_id)
              : null}
          </LiveAgentLens>
        </div>
      </div>
    </div>
  );
}
