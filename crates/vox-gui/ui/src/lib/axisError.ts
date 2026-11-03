import type { AxisErrorPayload } from '../types/tauri';

/** Decode the `{"error":{code,message,...}}` envelope Axis remote commands reject with; null otherwise. */
export function parseAxisError(err: unknown): AxisErrorPayload | null {
  const raw = typeof err === 'string' ? err : err instanceof Error ? err.message : null;
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as { error?: AxisErrorPayload };
    return parsed?.error && typeof parsed.error.code === 'string' ? parsed.error : null;
  } catch {
    return null;
  }
}
