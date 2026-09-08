import { useState, useEffect } from 'react';

/**
 * Browser-local persistence for GUI shell state. Canonical key names live in
 * `lib/shellPersistence.ts` / `contracts/gui/shell-persistence.v1.yaml`.
 * Prefer `voxTransport.getGuiPreference` for Tier-A prefs when backend sync is required.
 */
export function useLocalStorage<T>(
  key: string,
  initialValue: T,
  options?: { skipRead?: boolean },
) {
  const [storedValue, setStoredValue] = useState<T>(() => {
    if (options?.skipRead) return initialValue;
    try {
      const item = window.localStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch (error) {
      console.warn(error);
      return initialValue;
    }
  });

  useEffect(() => {
    if (options?.skipRead) return;
    try {
      window.localStorage.setItem(key, JSON.stringify(storedValue));
    } catch (error) {
      console.warn(error);
    }
  }, [key, storedValue, options?.skipRead]);

  return [storedValue, setStoredValue] as const;
}
