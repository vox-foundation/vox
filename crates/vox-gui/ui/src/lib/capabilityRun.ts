/**
 * Palette run decisions (D-01, D-18, D-03). RED stub: behaviour lands in the next commit.
 */
import type { CommandCatalogEntry } from '../types/catalog';
import type { Toast } from '../types/tauri';

export type CliActivation = { kind: 'run' } | { kind: 'form' } | { kind: 'explain'; reason: string };

export interface RunOutput {
  exit_code: number;
  stdout: string;
  stderr: string;
}

// Rust twin: CONFIRMATION_REQUIRED_PREFIX in crates/vox-cli/src/command_catalog.rs.
export const CONFIRMATION_REQUIRED_PREFIX = '';

export function isReadOnlyClass(_s: string | null): boolean {
  return false;
}

export function decideCliActivation(
  _row: { sideEffect: string | null },
  _entry: CommandCatalogEntry | undefined,
): CliActivation {
  return { kind: 'form' };
}

export function confirmationRequiredId(_err: unknown): string | null {
  return null;
}

export function toastForRun(path: string[], out: RunOutput, _onViewFull: () => void): Toast {
  return { tone: 'info', title: path.join(' '), cause: 'backend-ok', body: out.stdout };
}
