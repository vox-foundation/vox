import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { axisBackendStatus } from '../transport';
import type { BackendStatus, SurfaceAvailability } from '../types/tauri';

export const LOCAL_BACKEND_STATUS: BackendStatus = { mode: 'local', url: null, profile: null, surfaces: [] };

interface BackendStatusContextValue {
  status: BackendStatus;
  setStatus: (status: BackendStatus) => void;
  refresh: () => Promise<void>;
}

const BackendStatusContext = createContext<BackendStatusContextValue>({
  status: LOCAL_BACKEND_STATUS,
  setStatus: () => {},
  refresh: async () => {},
});

function isBackendStatus(value: unknown): value is BackendStatus {
  const v = value as BackendStatus | null;
  return !!v && (v.mode === 'local' || v.mode === 'remote') && Array.isArray(v.surfaces);
}

export function BackendStatusProvider({ children }: { children: React.ReactNode }) {
  const [status, setStatus] = useState<BackendStatus>(LOCAL_BACKEND_STATUS);
  const refresh = useCallback(async () => {
    try {
      const next = await axisBackendStatus();
      setStatus(isBackendStatus(next) ? next : LOCAL_BACKEND_STATUS);
    } catch {
      setStatus(LOCAL_BACKEND_STATUS);
    }
  }, []);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  const value = useMemo(() => ({ status, setStatus, refresh }), [status, refresh]);
  return <BackendStatusContext.Provider value={value}>{children}</BackendStatusContext.Provider>;
}

/** Fixed status, for tests and previews. */
export function StaticBackendStatusProvider({ status, children }: { status: BackendStatus; children: React.ReactNode }) {
  const value = useMemo(() => ({ status, setStatus: () => {}, refresh: async () => {} }), [status]);
  return <BackendStatusContext.Provider value={value}>{children}</BackendStatusContext.Provider>;
}

export function useBackendStatus(): BackendStatusContextValue {
  return useContext(BackendStatusContext);
}

export function surfaceAvailability(status: BackendStatus, viewKey: string): SurfaceAvailability {
  if (status.mode === 'local') return { surface: viewKey, available: true, reason: null };
  return (
    status.surfaces.find((s) => s.surface === viewKey) ?? { surface: viewKey, available: false, reason: 'not_in_profile' }
  );
}
