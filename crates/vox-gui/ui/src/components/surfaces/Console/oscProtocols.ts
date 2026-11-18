import React from 'react';
import { voxTransport, type OpenLocator } from '../../../transport';

export interface Osc8LinkEntry {
  uri: string;
  label: string;
}

const OSC8_SEQ_RE =
  /\x1b\]8;[^;\x07\x1b]*;([^\x07\x1b]+)(?:\x07|\x1b\\)([^\x1b]*?)\x1b\]8;;(?:\x07|\x1b\\)/g;
const OSC52_SEQ_RE = /\x1b\]52;[cp0-7]*;([A-Za-z0-9+/=]{1,87384})(?:\x07|\x1b\\)/g;
const OSC8_SENTINEL_RE = /\uE000(\d+)\uE001/g;

export function renderWithOsc8(
  text: string,
  osc8Links: Map<string, Osc8LinkEntry> | undefined,
  keyPrefix: string,
  cls: string,
  renderPlainFn: (chunk: string, kp: string) => React.ReactNode,
  searchHighlightFn: (txt: string, kp: string) => React.ReactNode,
  sanitizeLocatorFn: (raw: string) => { path: string; line?: number; col?: number } | null,
  dispatchLocatorFn: (raw: string, detail: { path: string; line?: number; col?: number }, openLocator?: (loc: OpenLocator) => void) => void,
  onOpenLocator?: (loc: OpenLocator) => void,
): React.ReactNode {
  if (!osc8Links || osc8Links.size === 0 || !text.includes('\uE000')) {
    return renderPlainFn(text, keyPrefix);
  }
  const out: React.ReactNode[] = [];
  let pos = 0; let m: RegExpExecArray | null; let k = 0;
  OSC8_SENTINEL_RE.lastIndex = 0;
  while ((m = OSC8_SENTINEL_RE.exec(text)) !== null) {
    if (m.index > pos) out.push(renderPlainFn(text.slice(pos, m.index), `${keyPrefix}-p${k++}`));
    const entry = osc8Links.get(m[1]!);
    if (entry) out.push(renderOsc8Segment(entry, `${keyPrefix}-o${k++}`, cls, searchHighlightFn, sanitizeLocatorFn, dispatchLocatorFn, onOpenLocator));
    pos = m.index + m[0].length;
  }
  if (pos < text.length) out.push(renderPlainFn(text.slice(pos), `${keyPrefix}-p${k++}`));
  return out.length === 1 ? out[0] : out;
}

export function extractOsc8Links(raw: string): {
  cleaned: string;
  links: Map<string, Osc8LinkEntry>;
} {
  const links = new Map<string, Osc8LinkEntry>();
  if (!raw || !raw.includes('\x1b]8;')) {
    return { cleaned: raw, links };
  }
  let idx = 0;
  const cleaned = raw.replace(OSC8_SEQ_RE, (_m, uri: string, label: string) => {
    const id = String(idx++);
    links.set(id, { uri: uri.trim(), label });
    return `\uE000${id}\uE001`;
  });
  return { cleaned, links };
}

export function extractOsc52Clipboard(raw: string): {
  cleaned: string;
  clipboardText: string | null;
} {
  if (!raw || !raw.includes('\x1b]52;')) {
    return { cleaned: raw, clipboardText: null };
  }
  let clipboardText: string | null = null;
  const cleaned = raw.replace(OSC52_SEQ_RE, (_m, b64: string) => {
    try {
      if (typeof atob === 'function') {
        const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
        if (bytes.byteLength <= 64 * 1024) {
          clipboardText = new TextDecoder('utf-8', { fatal: false }).decode(bytes);
        }
      }
    } catch {
      clipboardText = null;
    }
    return '';
  });
  return { cleaned, clipboardText };
}

export function encodeKittyKeyEvent(e: {
  key: string;
  ctrlKey?: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
  metaKey?: boolean;
}): string | null {
  let mod = 1;
  if (e.shiftKey) mod += 1;
  if (e.altKey) mod += 2;
  if (e.ctrlKey) mod += 4;
  if (e.metaKey) mod += 8;
  if (mod === 1) return null;

  const cp =
    e.key === 'Enter'
      ? 13
      : e.key === 'Tab'
        ? 9
        : e.key === 'Backspace'
          ? 127
          : e.key === 'Escape'
            ? 27
            : null;
  if (cp === null) return null;
  return `\x1b[${cp};${mod}u`;
}

export function renderOsc8Segment(
  entry: Osc8LinkEntry,
  key: string,
  cls: string,
  searchHighlightFn: (txt: string, kp: string) => React.ReactNode,
  sanitizeLocatorFn: (raw: string) => { path: string; line?: number; col?: number } | null,
  dispatchLocatorFn: (raw: string, detail: { path: string; line?: number; col?: number }, openLocator?: (loc: OpenLocator) => void) => void,
  onOpenLocator?: (loc: OpenLocator) => void,
): React.ReactNode {
  const { uri, label } = entry;
  const lower = uri.toLowerCase();
  if (lower.startsWith('http://') || lower.startsWith('https://')) {
    const onClick = (e: React.MouseEvent) => {
      e.stopPropagation();
      if (onOpenLocator) onOpenLocator({ kind: 'web', value: uri });
      else void Promise.resolve(voxTransport.openLocator?.({ kind: 'web', value: uri })).catch(() => {});
    };
    return React.createElement(
      'button',
      { key, type: 'button', title: uri, 'data-testid': 'term-output-osc8-link', className: cls, onClick },
      searchHighlightFn(label || uri, `${key}-l`),
    );
  }
  const rel = lower.startsWith('file:///')
    ? uri.slice(8)
    : lower.startsWith('file://localhost/')
      ? uri.slice(17)
      : lower.startsWith('file://')
        ? uri.slice(7)
        : uri;
  const safeFile = sanitizeLocatorFn(rel);
  if (!safeFile) return searchHighlightFn(label || uri, `${key}-t`);
  const onClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    dispatchLocatorFn(rel, safeFile, onOpenLocator);
  };
  return React.createElement(
    'button',
    { key, type: 'button', title: uri, 'data-testid': 'term-output-osc8-link', className: cls, onClick },
    searchHighlightFn(label || rel, `${key}-f`),
  );
}
