import type { BackendMode } from '../../../types/tauri';

/** Sections backed by host/db-class modules: they configure this machine's local Vox. */
export const LOCAL_MACHINE_SECTIONS: ReadonlySet<string> = new Set([
  'orchestrator', 'scaling', 'llm', 'routing', 'runtime', 'voice', 'mesh', 'signing', 'secrets', 'onboarding', 'gamify',
]);

/** Sections that configure this Axis install itself (shell-class); usable in both modes. */
export const AXIS_LOCAL_SECTIONS: ReadonlySet<string> = new Set(['server', 'telemetry', 'keybinds', 'theme', 'display']);

export function sectionLockedRemotely(sectionId: string, mode: BackendMode): boolean {
  return mode === 'remote' && LOCAL_MACHINE_SECTIONS.has(sectionId);
}
