import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  listenAgentEvents,
  voxTransport,
  type AgentEventFrame,
} from '../../transport';
import {
  emptyLiveState,
  type DriveState,
} from '../../lib/axisDrive';
import { recordAgentFrame } from '../../lib/driveEvents';
import {
  driveVerbNeedsCatalog,
  handleDriveRequest,
  loquelaDriveApi,
  type DriveRequest,
  type DriveSetters,
  type DriveSubmitPayload,
} from '../../lib/useDriveBus';
import { normalizeModelCard, type PickerModel, type ProviderStatus } from '../../lib/modelPicker';
import { MODEL_LIST_LIMIT } from '../../config/constants';

export interface AxisDriveHostProps {
  setters: DriveSetters;
  onSubmit: (payload: DriveSubmitPayload) => Promise<unknown> | unknown;
  sessionReady: boolean;
  /** When set, agent frames that carry `session_id` are dropped unless they match. */
  sessionId?: string | null;
}

/**
 * Subscribe to agent events for the active Drive turn. Extracted so StrictMode
 * remount / double-listen can be unit-tested without mounting the full host.
 */
export function attachDriveAgentListener(
  stateRef: { current: DriveState },
  activeTurnIdRef: { current: string | null },
  opts?: { sessionId?: string | null },
): () => void {
  let cancelled = false;
  let unlisten: UnlistenFn | undefined;
  void listenAgentEvents((frame: AgentEventFrame) => {
    if (cancelled) return;
    const turnId = activeTurnIdRef.current;
    if (!turnId) return;
    const frameSession =
      typeof frame.kind?.session_id === 'string' ? frame.kind.session_id : null;
    const expected = opts?.sessionId ?? null;
    // Filter only when both sides present — legacy frames omit session_id.
    if (frameSession && expected && frameSession !== expected) return;
    const eventState = recordAgentFrame(stateRef.current, frame, turnId);
    stateRef.current = { ...stateRef.current, ...eventState };
  })
    .then(stop => {
      if (cancelled) stop();
      else unlisten = stop;
    })
    .catch(() => {
      // Browser/vitest harness: no Tauri agent-event plane.
    });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}

export function AxisDriveHost({
  setters,
  onSubmit,
  sessionReady,
  sessionId = null,
}: AxisDriveHostProps) {
  const stateRef = useRef<DriveState>(emptyLiveState());
  const activeTurnIdRef = useRef<string | null>(null);

  useEffect(() => {
    return attachDriveAgentListener(stateRef, activeTurnIdRef, { sessionId });
  }, [sessionId]);

  useEffect(() => {
    if (!sessionReady) return;
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;
    void invoke<string>('get_drive_mode')
      .then(async mode => {
        if (cancelled || mode !== 'live') return;
        await invoke('drive_set_ready');
        if (cancelled) return;
        const stop = await listen<DriveRequest>('drive://request', async event => {
          // state/show must not wait on model IPC — a hung catalog made every
          // drive verb 504 while the window was already up.
          const needsCatalog = driveVerbNeedsCatalog(event.payload.verb);
          const models = needsCatalog ? await loadModels() : [];
          const statuses = needsCatalog ? await loadStatuses() : [];
          const merged: DriveSetters = {
            ...loquelaDriveApi(),
            ...setters,
          };
          let res;
          try {
            res = await handleDriveRequest({
              state: stateRef.current,
              models,
              statuses,
              setters: merged,
              submit: onSubmit,
              req: event.payload,
              onTurnStart: (turnId, eventState) => {
                activeTurnIdRef.current = turnId;
                stateRef.current = { ...stateRef.current, ...eventState };
              },
              getActiveEventState: () => stateRef.current,
            });
          } finally {
            if (event.payload.verb === 'send') {
              activeTurnIdRef.current = null;
            }
          }
          let orchFresh = false;
          try {
            orchFresh = await invoke<boolean>('orchestrator_daemon_ready');
          } catch {
            orchFresh = false;
          }
          const stamped = {
            ...res,
            state: { ...res.state, orch_fresh: orchFresh },
          };
          stateRef.current = stamped.state;
          await invoke('drive_respond', {
            args: {
              id: event.payload.id,
              status: stamped.status,
              body: JSON.stringify(stamped),
            },
          });
        });
        // StrictMode remount + async listen: cleanup may run before this
        // assignment. Drop the late subscription so one Drive HTTP send cannot
        // fan out into two chat_turn invokes ("reply still in progress").
        if (cancelled) {
          stop();
          return;
        }
        unlisten = stop;
      })
      .catch(() => {
        // Browser/vitest harness: no Tauri drive plane.
      });
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, [onSubmit, sessionReady, setters]);

  return null;
}

async function loadModels(): Promise<PickerModel[]> {
  try {
    const cards = await voxTransport.listModels(MODEL_LIST_LIMIT);
    return (Array.isArray(cards) ? cards : [])
      .map(c => normalizeModelCard(c))
      .filter((m): m is PickerModel => m != null);
  } catch {
    return [];
  }
}

async function loadStatuses(): Promise<ProviderStatus[]> {
  try {
    const rows = await invoke<ProviderStatus[]>('inference_provider_status');
    return Array.isArray(rows) ? rows : [];
  } catch {
    return [];
  }
}
