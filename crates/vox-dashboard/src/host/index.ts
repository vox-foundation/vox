/**
 * Host shell barrel — replaces VS Code APIs for ported features.
 *
 * Import from here rather than individual modules so the abstraction layer
 * stays consistent:
 *
 *   import { toasts, settings, keymap } from '../host';
 */

export { toasts } from './toasts';
export type { Toast, ToastSeverity } from './toasts';

export { settings } from './settings';

export { keymap } from './keymap';
export type { KeyBinding } from './keymap';
