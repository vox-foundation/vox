import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { listenAgentEvents, voxTransport } from '../../transport';
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

export interface AxisDriveHostProps {
  setters: DriveSetters;
  onSubmit: (payload: DriveSubmitPayload) => Promise<unknown> | unknown;
  sessionReady: boolean;
}

export function AxisDriveHost({ setters, onSubmit, sessionReady }: AxisDriveHostProps) {
  const stateRef = useRef<DriveState>(emptyLiveState());
  const activeTurnIdRef = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;
    void listenAgentEvents(frame => {
      const turnId = activeTurnIdRef.current;
      if (!turnId) return;
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
  }, []);

  useEffect(() => {
    if (!sessionReady) return;
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;
    void invoke<string>('get_drive_mode')
      .then(async mode => {
        if (cancelled || mode !== 'live') return;
        await invoke('drive_set_ready');
        unlisten = await listen<DriveRequest>('drive://request', async event => {
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
          stateRef.current = res.state;
          await invoke('drive_respond', {
            args: {
              id: event.payload.id,
              status: res.status,
              body: JSON.stringify(res),
            },
          });
        });
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
    const cards = await voxTransport.listModels();
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
