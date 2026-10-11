export type ActionId =
  | 'open-palette' | 'toggle-sidebar' | 'dispatch-intent'
  | 'pause-resume-agent' | 'toggle-inspector' | 'show-shortcuts';
export interface ActionDef { id: ActionId; label: string }
export const ACTION_REGISTRY: ActionDef[] = [
  { id: 'open-palette',  label: 'Open command palette' },
  { id: 'toggle-sidebar', label: 'Toggle sidebar width' },
  { id: 'dispatch-intent', label: 'Dispatch intent (in composer)' },
  { id: 'pause-resume-agent', label: 'Pause/resume selected agent' },
  { id: 'toggle-inspector', label: 'Toggle pipeline inspector drawer' },
  { id: 'show-shortcuts', label: 'Show keyboard shortcuts' },
];
export type Bindings = Record<string, string>;
/**
 * `show-shortcuts` defaults to Mod+/. On layouts where '/' needs Shift the chord
 * is unreachable (chordFromEvent would add Shift); those users rebind in
 * Settings. Not engineered around. The bare `?` key always works (D-11).
 */
export const DEFAULT_BINDINGS: Bindings = {
  'open-palette': 'Mod+K',
  'toggle-sidebar': 'Mod+B',
  'dispatch-intent': 'Mod+Enter',
  'pause-resume-agent': 'Mod+.',
  'toggle-inspector': 'Mod+Shift+D',
  'show-shortcuts': 'Mod+/',
};
export function chordFromEvent(e: Pick<KeyboardEvent,'key'|'metaKey'|'ctrlKey'|'shiftKey'|'altKey'>): string {
  const parts: string[] = [];
  if (e.metaKey || e.ctrlKey) parts.push('Mod');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');
  const k = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  parts.push(k);
  return parts.join('+');
}
export function matchAction(chord: string, bindings: Bindings): ActionId | null {
  const hit = (Object.keys(bindings) as ActionId[]).find(id => bindings[id] === chord);
  return hit ?? null;
}
export function serializeBindings(b: Bindings): string { return JSON.stringify(b); }
export function parseBindings(json: string | null): Bindings {
  if (!json) return { ...DEFAULT_BINDINGS };
  try { return { ...DEFAULT_BINDINGS, ...JSON.parse(json) }; } catch { return { ...DEFAULT_BINDINGS }; }
}

/** Render a stored chord ("Mod+Shift+D") for display: ⌘⇧D on macOS, Ctrl+Shift+D elsewhere. */
export function formatChord(chord: string, isMac: boolean): string {
  if (!chord) return '—';
  const parts = chord.split('+').map(p =>
    p === 'Mod' ? (isMac ? '⌘' : 'Ctrl')
    : p === 'Shift' ? (isMac ? '⇧' : 'Shift')
    : p === 'Alt' ? (isMac ? '⌥' : 'Alt')
    : p);
  return parts.join(isMac ? '' : '+');
}

export function isMacPlatform(): boolean {
  return typeof navigator !== 'undefined' && /Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent || '');
}

type TargetLike = { tagName?: string; isContentEditable?: boolean; closest?: (s: string) => unknown };
/** True when keystrokes in `t` are text entry (input, textarea, select, contenteditable or a child of one). */
export function isEditableTarget(t: EventTarget | null): boolean {
  const el = t as TargetLike | null;
  if (!el) return false;
  const tag = (el.tagName ?? '').toUpperCase();
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  return !!el.isContentEditable || !!el.closest?.('[contenteditable]');
}

/**
 * Bare `?` opens the shortcut sheet, but must never hijack typing (RESEARCH F-11, T-06.3-37).
 * Ctrl without Alt is rejected (a real Ctrl+? chord); Ctrl+Alt is AltGr on some layouts, so allowed.
 */
export function isShortcutHelpKey(
  e: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'defaultPrevented' | 'target'>,
): boolean {
  return e.key === '?'
    && !e.metaKey
    && !(e.ctrlKey && !e.altKey)
    && !e.defaultPrevented
    && !isEditableTarget(e.target);
}

/** Registry rows that have a runnable handler, in registry order (D-13; consumed by plan 06.3-11). Equal chords stay separate rows. */
export function paletteActionRows(
  bindings: Bindings,
  handlerIds: ReadonlySet<string>,
  isMac: boolean,
): { id: ActionId; label: string; chord: string }[] {
  return ACTION_REGISTRY
    .filter(a => handlerIds.has(a.id))
    .map(a => ({ id: a.id, label: a.label, chord: formatChord(bindings[a.id] ?? DEFAULT_BINDINGS[a.id], isMac) }));
}
