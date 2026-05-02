/**
 * Toast notifications — replaces vscode.window.showInformationMessage,
 * showWarningMessage, and showErrorMessage for ported features.
 *
 * Consumers import: import { toasts } from '../host';
 */

export type ToastSeverity = 'info' | 'warning' | 'error';

export interface Toast {
  id: string;
  severity: ToastSeverity;
  message: string;
  /** Auto-dismiss after this many ms (0 = sticky). Default: 4000 */
  durationMs: number;
}

type ToastListener = (toasts: Toast[]) => void;

let _toasts: Toast[] = [];
let _counter = 0;
const _listeners: Set<ToastListener> = new Set();

function notify() {
  const snapshot = [..._toasts];
  for (const fn of _listeners) fn(snapshot);
}

export const toasts = {
  show(severity: ToastSeverity, message: string, durationMs = 4000): string {
    const id = `toast-${++_counter}`;
    _toasts = [..._toasts, { id, severity, message, durationMs }];
    notify();
    if (durationMs > 0) {
      setTimeout(() => toasts.dismiss(id), durationMs);
    }
    return id;
  },

  info(message: string, durationMs?: number) {
    return this.show('info', message, durationMs);
  },
  warn(message: string, durationMs?: number) {
    return this.show('warning', message, durationMs);
  },
  error(message: string, durationMs?: number) {
    return this.show('error', message, durationMs ?? 0);
  },

  dismiss(id: string) {
    _toasts = _toasts.filter((t) => t.id !== id);
    notify();
  },

  getAll(): Toast[] {
    return [..._toasts];
  },

  subscribe(fn: ToastListener): () => void {
    _listeners.add(fn);
    fn([..._toasts]);
    return () => _listeners.delete(fn);
  },
};
