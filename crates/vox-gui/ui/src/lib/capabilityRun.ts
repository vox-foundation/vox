/**
 * Palette run decisions (D-01, D-18) and the destructive-op refusal marker (D-03).
 * Pure: type-only imports, no IPC.
 */
import type { CommandCatalogEntry } from '../types/catalog';
import type { Toast } from '../types/tauri';

export type CliActivation = { kind: 'run' } | { kind: 'form' } | { kind: 'explain'; reason: string };

export interface RunOutput {
  exit_code: number;
  stdout: string;
  stderr: string;
}

/**
 * Prefix of the refusal `execute_command` returns for a destructive verb that was not
 * confirmed. Rust twin: `CONFIRMATION_REQUIRED_PREFIX` in crates/vox-cli/src/command_catalog.rs;
 * the parity test in capabilityRun.test.ts reads that file and fails on drift.
 */
export const CONFIRMATION_REQUIRED_PREFIX = 'confirmation_required: ';

/** D-18: `none` counts as read-only. An unclassified (null) verb is treated as side-effecting. */
export function isReadOnlyClass(s: string | null): boolean {
  return s === 'read_only' || s === 'none';
}

/**
 * D-01: what Enter does on a CLI row. `run` ONLY for a read-only/none, compiled-in leaf with no
 * required argument; a verb the live catalog lacks (or this build did not compile in) explains
 * itself; everything else opens the prefilled form, where running needs an explicit Run click.
 */
export function decideCliActivation(
  row: { sideEffect: string | null },
  entry: CommandCatalogEntry | undefined,
): CliActivation {
  if (!entry) return { kind: 'explain', reason: "not in this build's command catalog" };
  if (!entry.compiled_in) return { kind: 'explain', reason: 'not compiled into this build' };
  const needsInput = (entry.arguments ?? []).some((a) => a.required);
  if (isReadOnlyClass(row.sideEffect) && !entry.has_subcommands && !needsInput) return { kind: 'run' };
  return { kind: 'form' };
}

/** The row id inside a `confirmation_required: <id>` refusal, else null. Never stringifies `err`. */
export function confirmationRequiredId(err: unknown): string | null {
  let msg: string | null = null;
  if (typeof err === 'string') msg = err;
  else if (err && typeof err === 'object' && typeof (err as { message?: unknown }).message === 'string') {
    msg = (err as { message: string }).message;
  }
  if (msg === null || !msg.startsWith(CONFIRMATION_REQUIRED_PREFIX)) return null;
  const id = msg.slice(CONFIRMATION_REQUIRED_PREFIX.length).trim();
  return id.length > 0 ? id : null;
}

/** D-04: short run toast (<=400 char excerpt) with a button that opens the full output. */
export function toastForRun(path: string[], out: RunOutput, onViewFull: () => void): Toast {
  const ok = out.exit_code === 0;
  const text = [out.stdout, out.stderr].filter(Boolean).join('\n').trim();
  return {
    tone: ok ? 'ok' : 'warn',
    title: `vox ${path.join(' ')} — exit ${out.exit_code}`,
    body: text.slice(0, 400) || 'no output',
    cause: ok ? 'backend-ok' : 'backend-error',
    action: { label: 'View full output', onClick: onViewFull },
  };
}
