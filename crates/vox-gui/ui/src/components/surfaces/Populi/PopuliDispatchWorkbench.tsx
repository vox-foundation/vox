import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { invokeCmd } from '../../../transport';
import type { Toast } from '../../../types/tauri';
import type { MeshNode } from '../Mesh/MeshView';
import { Icon } from '../../ui/Icons';
import { RomanRotaryKnob } from '../../ui/RomanRotaryKnob';
import { VoxSelect } from '../../ui/Select';
import type { ProbeKey, ProbeState } from './PeerAttestationInspector';

export {
  clampGossipCadence,
  formatPeerLastSeen,
  parseOverlayFacts,
  PeerAttestationInspector,
  type ProbeKey,
  type ProbeState,
} from './PeerAttestationInspector';

export interface PopuliDispatchWorkbenchProps {
  nodes: MeshNode[];
  dispatchConfigured: boolean;
  gossipCadenceMs: number;
  probing: boolean;
  probes: Partial<Record<ProbeKey, ProbeState>>;
  gamifyEnabled?: boolean;
  pushToast: (item: Toast) => void;
  onCadenceChange: (next: number) => void;
  onCadenceCommit: (next: number) => void;
  onRunProbes: () => Promise<void>;
  onAfterDispatch: () => Promise<void>;
}

interface McpEnvelope<T> {
  tool: string;
  is_error: boolean;
  result: T;
}

export function PopuliDispatchWorkbench({
  nodes,
  dispatchConfigured,
  gossipCadenceMs,
  probing,
  probes,
  gamifyEnabled,
  pushToast,
  onCadenceChange,
  onCadenceCommit,
  onRunProbes,
  onAfterDispatch,
}: PopuliDispatchWorkbenchProps) {
  const [targetNode, setTargetNode] = useState<string>('');
  const [taskKind, setTaskKind] = useState<string>('');
  const [source, setSource] = useState<string>('');
  const [dispatching, setDispatching] = useState(false);
  const [dispatchResult, setDispatchResult] = useState<string>('');
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const nodeOptions = useMemo(() => {
    const seen = new Set<string>();
    const opts = [{ value: '', label: 'Auto (control plane picks)' }];
    for (const n of nodes) {
      if (!seen.has(n.id)) {
        seen.add(n.id);
        opts.push({ value: n.id, label: n.id });
      }
    }
    return opts;
  }, [nodes]);

  const healthyCount = Object.values(probes).filter(
    (p) => p?.kind === 'ok' && p.exit_code === 0,
  ).length;

  const dispatch = useCallback(async () => {
    if (!source.trim()) {
      pushToast({
        tone: 'warn',
        title: 'Dispatch needs source',
        body: 'Enter .vox source to run.',
        cause: 'validation',
      });
      return;
    }
    setDispatching(true);
    setDispatchResult('');
    try {
      const args: Record<string, unknown> = { source };
      if (targetNode) args.node_id = targetNode;
      if (taskKind.trim()) args.task_kind = taskKind.trim();
      const res = await invokeCmd<McpEnvelope<Record<string, unknown>>>('invoke_mcp_tool', {
        tool: 'vox_mesh_dispatch',
        args,
      });
      if (!mountedRef.current) return;
      if (res?.is_error) {
        const msg = String(res?.result?.error ?? JSON.stringify(res?.result));
        setDispatchResult(sanitizeErrorForToast(msg));
        pushToast({
          tone: 'warn',
          title: 'Dispatch failed',
          body: sanitizeErrorForToast(msg),
          cause: 'backend-error',
        });
      } else {
        const r = (res?.result ?? {}) as Record<string, unknown>;
        const id = String(r.node_id ?? '(unknown node)');
        const ok = Boolean(r.success);
        setDispatchResult(
          `node=${id} success=${String(ok)} exit=${String(r.exit_code ?? 'n/a')} (${String(r.duration_ms ?? 0)}ms)\n${String(r.output ?? '')}`,
        );
        pushToast({
          tone: ok ? 'ok' : 'warn',
          title: ok ? 'Dispatched' : 'Dispatch returned failure',
          body: `node ${id}`,
          cause: 'backend-ok',
        });
        if (ok) {
          void recordGamifyGuiEvent(
            'mesh_dispatch_success',
            { node_id: id, task_kind: taskKind.trim() || null },
            { enabled: gamifyEnabled },
          );
        }
      }
      await onAfterDispatch();
    } catch (err) {
      if (!mountedRef.current) return;
      setDispatchResult(sanitizeErrorForToast(err));
      pushToast({
        tone: 'warn',
        title: 'Dispatch failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      if (mountedRef.current) setDispatching(false);
    }
  }, [source, targetNode, taskKind, pushToast, gamifyEnabled, onAfterDispatch]);

  return (
    <section
      data-slot="populi-dispatch-workbench"
      data-testid="populi-dispatch-workbench"
      aria-label="Populi Mesh Dispatch and Gossip Cadence Workbench"
      className="flex min-w-0 flex-col gap-3 rounded-xl border border-[#D4AF37]/25 bg-[#0B0D12]/95 p-3.5 text-slate-100"
    >
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[#D4AF37]/15 pb-2.5">
        <div className="flex items-center gap-2">
          <span className="flex size-7 items-center justify-center rounded-lg border border-[#D4AF37]/30 bg-[#D4AF37]/10 text-[#D4AF37]">
            <Icon.send className="size-3.5" aria-hidden="true" />
          </span>
          <div>
            <h2 className="font-display text-xs font-bold tracking-[0.14em] text-[#F3E5AB] uppercase">
              Dispatch Job &amp; QUIC Gossip Control
            </h2>
            <p className="font-mono text-[10px] text-slate-400">
              Remote .vox workload dispatch, overlay probe health ({healthyCount}/2 OK), and gossip cadence
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <RomanRotaryKnob
            ariaLabel="Populi gossip heartbeat cadence"
            label="Gossip"
            size="sm"
            min={100}
            max={2000}
            step={50}
            value={gossipCadenceMs}
            formatValue={(v) => `${v}ms`}
            onValueChange={onCadenceChange}
            onValueCommit={onCadenceCommit}
          />
          <button
            type="button"
            data-slot="button"
            data-testid="populi-run-probe-btn"
            disabled={probing}
            onClick={() => void onRunProbes()}
            className="inline-flex items-center gap-1.5 rounded-lg border border-[#D4AF37]/55 bg-[#D4AF37]/15 px-3 py-1.5 font-display text-[11px] font-bold tracking-wider text-[#F3E5AB] uppercase transition-colors hover:border-[#D4AF37] hover:bg-[#D4AF37]/25 disabled:opacity-45"
          >
            <Icon.refresh className="size-3.5 text-[#D4AF37]" aria-hidden="true" />
            <span>{probing ? 'Probing...' : 'Run Overlay Probe'}</span>
          </button>
        </div>
      </div>

      {!dispatchConfigured && (
        <div className="flex items-start gap-2 rounded-md border border-[#D4AF37]/25 bg-[#D4AF37]/5 px-3 py-2 text-[11px] text-[#F3E5AB]">
          <Icon.alert className="mt-0.5 size-3.5 shrink-0 text-[#D4AF37]" aria-hidden="true" />
          <span>
            Dispatch is disabled: no populi control plane is reachable. Configure{' '}
            <code className="font-mono text-[#F3E5AB]">VOX_ORCHESTRATOR_MESH_CONTROL_URL</code> (or{' '}
            <code className="font-mono text-[#F3E5AB]">populi_control_url</code>) and rebuild with{' '}
            <code className="font-mono text-[#F3E5AB]">populi-transport</code>.
          </span>
        </div>
      )}

      <div className="grid grid-cols-12 gap-3">
        <div className="col-span-12 @xl/surface:col-span-6">
          <label
            htmlFor="mesh-target-node"
            className="mb-1 block font-display text-[10px] tracking-wider text-slate-400 uppercase"
          >
            Target node (optional)
          </label>
          <VoxSelect
            id="mesh-target-node"
            aria-label="Target node (optional)"
            value={targetNode}
            onValueChange={setTargetNode}
            disabled={!dispatchConfigured}
            options={nodeOptions}
            className="w-full border-[#D4AF37]/25 bg-black/45 text-xs text-slate-200"
          />
        </div>
        <div className="col-span-12 @xl/surface:col-span-6">
          <label
            htmlFor="mesh-task-kind"
            className="mb-1 block font-display text-[10px] tracking-wider text-slate-400 uppercase"
          >
            Task kind (optional)
          </label>
          <input
            id="mesh-task-kind"
            data-slot="input"
            value={taskKind}
            onChange={(e) => setTaskKind(e.target.value)}
            disabled={!dispatchConfigured}
            placeholder="e.g. text_infer"
            className="w-full rounded-md border border-[#D4AF37]/25 bg-black/45 px-2.5 py-1.5 font-mono text-xs text-slate-200 disabled:opacity-40"
          />
        </div>
        <div className="col-span-12">
          <label
            htmlFor="mesh-source"
            className="mb-1 block font-display text-[10px] tracking-wider text-slate-400 uppercase"
          >
            Source (.vox)
          </label>
          <textarea
            id="mesh-source"
            data-slot="textarea"
            value={source}
            onChange={(e) => setSource(e.target.value)}
            disabled={!dispatchConfigured}
            rows={4}
            placeholder={'fn main() {\n  print("hello from the mesh")\n}'}
            className="w-full rounded-md border border-[#D4AF37]/25 bg-black/45 p-2.5 font-mono text-xs text-slate-200 disabled:opacity-40"
          />
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2">
        <button
          type="button"
          data-slot="button"
          data-testid="populi-dispatch-btn"
          onClick={() => void dispatch()}
          disabled={!dispatchConfigured || dispatching || !source.trim()}
          className="inline-flex items-center gap-1.5 rounded-md border border-[#D4AF37]/40 bg-[#D4AF37]/15 px-4 py-1.5 font-display text-[11px] font-bold tracking-wider text-[#F3E5AB] uppercase transition-colors hover:bg-[#D4AF37]/25 disabled:cursor-not-allowed disabled:opacity-40"
        >
          <Icon.send aria-hidden="true" className="size-3.5" />
          <span>{dispatching ? 'Dispatching...' : 'Dispatch'}</span>
        </button>
      </div>

      {dispatchResult && (
        <pre
          role="region"
          tabIndex={0}
          aria-label="Dispatch result"
          aria-live="polite"
          className="max-h-56 overflow-auto rounded-md border border-[#D4AF37]/20 bg-black/50 p-3 font-mono text-[11px] text-slate-200"
        >
          {dispatchResult}
        </pre>
      )}
    </section>
  );
}
