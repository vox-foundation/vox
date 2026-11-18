import React from 'react';
import { Icon } from '../../ui/Icons';
import { voxTransport, type OpenLocator } from '../../../transport';
import {
  encodeKittyKeyEvent, extractOsc52Clipboard, extractOsc8Links, renderWithOsc8,
  type Osc8LinkEntry,
} from './oscProtocols';
export { encodeKittyKeyEvent, extractOsc52Clipboard, extractOsc8Links, type Osc8LinkEntry };
import {
  DARK_ANSI_16_PALETTE,
  LIGHT_ANSI_16_PALETTE,
  resolveAnsi16PaletteArray,
  resolveTerminalWellSpec,
} from './terminalTheme';

export { LIGHT_ANSI_16_PALETTE, DARK_ANSI_16_PALETTE };

const CUBE_LEVELS = [0, 95, 135, 175, 215, 255] as const;

export interface SgrSpan {
  text: string; fg?: string; bg?: string;
  bold?: boolean; dim?: boolean; italic?: boolean; underline?: boolean; inverse?: boolean; strikethrough?: boolean;
}

export interface SgrOptions { theme?: string | null; palette?: readonly string[]; }

export interface SgrRenderOptions extends SgrOptions {
  searchQuery?: string; onOpenLocator?: (locator: OpenLocator) => void;
}

const OSC_RE = /\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g;
const LINE_TOKEN_RE = /\x1b\[[0-9;:]*m|\x1b\[([012])?K|\x1b\[[0-9;?]*[A-LN-Za-ln-z]|\r|[^\x1b\r]+/g;
const SGR_ONLY_RE = /\x1b\[[0-9;:]*m/g;
const SGR_SPLIT_RE = /\x1b\[([0-9;:]*)m/g;
const LINK_TOKEN_RE =
  /(\bhttps?:\/\/[^\s<>"')\]]+|\blocalhost:\d+(?:\/[^\s<>"')\]]*)?)|((?:[A-Za-z]:[\\/]|(?:\.{1,2}[\\/])|(?:[\w@-]+[\\/])+)[\w.-]+\.[A-Za-z0-9]+(?::\d+(?::\d+)?|\(\d+,\d+\)))/g;

const clampByte = (n: number): number => (Number.isFinite(n) ? Math.max(0, Math.min(255, Math.trunc(n))) : 0);

function resolve256Color(index: number, palette: readonly string[]): string | undefined {
  if (!Number.isFinite(index) || index < 0 || index > 255) return undefined;
  const idx = Math.trunc(index);
  if (idx < 16) return palette[idx];
  if (idx < 232) {
    const c = idx - 16;
    return `rgb(${CUBE_LEVELS[Math.floor(c / 36) % 6]!}, ${CUBE_LEVELS[Math.floor(c / 6) % 6]!}, ${CUBE_LEVELS[c % 6]!})`;
  }
  const gray = 8 + 10 * (idx - 232);
  return `rgb(${gray}, ${gray}, ${gray})`;
}

function resolvePalette(opts?: SgrOptions): readonly string[] {
  if (opts?.palette && opts.palette.length >= 16) return opts.palette;
  return resolveAnsi16PaletteArray(opts?.theme);
}

const isFullSgrReset = (tok: string): boolean => tok === '\x1b[0m' || tok === '\x1b[m';

export function normalizeTerminalText(raw: string): string {
  if (!raw) return '';
  let activeSgr = '';
  let emittedSgr = '';
  return raw.replace(OSC_RE, '').replace(/\r\n/g, '\n').split('\n').map((line) => {
    let cells: Array<{ ch: string; sgr: string }> = [];
    let col = 0;
    LINE_TOKEN_RE.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = LINE_TOKEN_RE.exec(line)) !== null) {
      const tok = m[0];
      if (tok === '\r') {
        col = 0;
      } else if (tok.startsWith('\x1b[') && tok.endsWith('m')) {
        activeSgr = isFullSgrReset(tok) ? '' : activeSgr + tok;
      } else if (tok.startsWith('\x1b[') && tok.endsWith('K')) {
        const mode = m[1] ?? '0';
        if (mode === '0') {
          cells.length = Math.min(cells.length, col);
        } else if (mode === '1') {
          const end = Math.min(col, cells.length - 1);
          for (let i = 0; i <= end; i++) cells[i] = { ch: ' ', sgr: '' };
        } else if (mode === '2') {
          cells = [];
          col = 0;
        }
      } else if (!tok.startsWith('\x1b[')) {
        for (const ch of tok) {
          cells[col] = { ch, sgr: activeSgr };
          col += 1;
        }
      }
    }
    let out = '';
    for (let i = 0; i < cells.length; i++) {
      const cell = cells[i] ?? { ch: ' ', sgr: '' };
      if (cell.sgr !== emittedSgr) {
        if (emittedSgr !== '') out += '\x1b[0m';
        if (cell.sgr !== '') out += cell.sgr;
        emittedSgr = cell.sgr;
      }
      out += cell.ch;
    }
    if (activeSgr !== emittedSgr) {
      if (emittedSgr !== '') out += '\x1b[0m';
      if (activeSgr !== '') out += activeSgr;
      emittedSgr = activeSgr;
    }
    return out;
  }).join('\n');
}

export function stripAnsi(raw: string): string {
  return normalizeTerminalText(raw).replace(SGR_ONLY_RE, '');
}

interface BrushState {
  fg?: string; bg?: string;
  bold: boolean; dim: boolean; italic: boolean; underline: boolean; inverse: boolean; strikethrough: boolean;
}

function applySgrSequence(paramStr: string, state: BrushState, palette: readonly string[]): void {
  const parts = paramStr === '' ? ['0'] : paramStr.split(';');
  for (let i = 0; i < parts.length; i++) {
    const seg = parts[i] ?? '';
    if (seg.includes(':')) {
      const sub = seg.split(':');
      const head = Number(sub[0] || 0);
      if ((head === 38 || head === 48) && Number(sub[1]) === 5 && sub[2] !== undefined) {
        const c = resolve256Color(Number(sub[2]), palette);
        if (head === 38) state.fg = c; else state.bg = c;
      } else if ((head === 38 || head === 48) && Number(sub[1]) === 2) {
        const o = sub.length >= 6 ? 3 : 2;
        const c = `rgb(${clampByte(Number(sub[o]))}, ${clampByte(Number(sub[o + 1]))}, ${clampByte(Number(sub[o + 2]))})`;
        if (head === 38) state.fg = c; else state.bg = c;
      } else if (head === 4) state.underline = Number(sub[1]) !== 0;
      continue;
    }
    const code = seg === '' ? 0 : Number(seg);
    if (code === 0) Object.assign(state, { fg: undefined, bg: undefined, bold: false, dim: false, italic: false, underline: false, inverse: false, strikethrough: false });
    else if (code === 1) state.bold = true;
    else if (code === 2) state.dim = true;
    else if (code === 3) state.italic = true;
    else if (code === 4) state.underline = true;
    else if (code === 7) state.inverse = true;
    else if (code === 9) state.strikethrough = true;
    else if (code === 22) { state.bold = false; state.dim = false; }
    else if (code === 23) state.italic = false;
    else if (code === 24) state.underline = false;
    else if (code === 27) state.inverse = false;
    else if (code === 29) state.strikethrough = false;
    else if (code >= 30 && code <= 37) state.fg = palette[code - 30];
    else if (code === 39) state.fg = undefined;
    else if (code >= 40 && code <= 47) state.bg = palette[code - 40];
    else if (code === 49) state.bg = undefined;
    else if (code >= 90 && code <= 97) state.fg = palette[code - 90 + 8];
    else if (code >= 100 && code <= 107) state.bg = palette[code - 100 + 8];
    else if (code === 38 || code === 48) {
      const mode = Number(parts[i + 1]);
      if (mode === 5 && parts[i + 2] !== undefined) {
        const c = resolve256Color(Number(parts[i + 2]), palette);
        if (code === 38) state.fg = c; else state.bg = c;
        i += 2;
      } else if (mode === 2 && parts[i + 4] !== undefined) {
        const c = `rgb(${clampByte(Number(parts[i + 2]))}, ${clampByte(Number(parts[i + 3]))}, ${clampByte(Number(parts[i + 4]))})`;
        if (code === 38) state.fg = c; else state.bg = c;
        i += 4;
      }
    }
  }
}

function buildSpan(text: string, state: BrushState, wellFg: string, wellBg: string): SgrSpan {
  const fg = state.inverse ? (state.bg ?? wellBg) : state.fg;
  const bg = state.inverse ? (state.fg ?? wellFg) : state.bg;
  return {
    text, ...(fg ? { fg } : {}), ...(bg ? { bg } : {}),
    ...(state.bold ? { bold: true } : {}), ...(state.dim ? { dim: true } : {}),
    ...(state.italic ? { italic: true } : {}), ...(state.underline ? { underline: true } : {}),
    ...(state.inverse ? { inverse: true } : {}), ...(state.strikethrough ? { strikethrough: true } : {}),
  };
}

const sameSpanStyle = (a: SgrSpan, b: SgrSpan): boolean =>
  a.fg === b.fg && a.bg === b.bg && a.bold === b.bold && a.dim === b.dim &&
  a.italic === b.italic && a.underline === b.underline && a.inverse === b.inverse && a.strikethrough === b.strikethrough;

export function parseSgrSpans(raw: string, opts?: SgrOptions): SgrSpan[] {
  const normalized = normalizeTerminalText(raw);
  if (!normalized) return [];
  const palette = resolvePalette(opts);
  const wellSpec = resolveTerminalWellSpec(opts?.theme);
  const state: BrushState = { bold: false, dim: false, italic: false, underline: false, inverse: false, strikethrough: false };
  const spans: SgrSpan[] = [];
  let cursor = 0;
  SGR_SPLIT_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  const pushText = (chunk: string) => {
    if (!chunk) return;
    const next = buildSpan(chunk, state, wellSpec.wellFg, wellSpec.wellBg);
    const prev = spans[spans.length - 1];
    if (prev && sameSpanStyle(prev, next)) prev.text += chunk; else spans.push(next);
  };
  while ((m = SGR_SPLIT_RE.exec(normalized)) !== null) {
    if (m.index > cursor) pushText(normalized.slice(cursor, m.index));
    applySgrSequence(m[1] ?? '', state, palette);
    cursor = m.index + m[0].length;
  }
  if (cursor < normalized.length) pushText(normalized.slice(cursor));
  return spans;
}

function renderHighlightedText(text: string, query: string | undefined, keyPrefix: string): React.ReactNode {
  const q = query?.trim();
  if (!q) return text;
  const lowerText = text.toLowerCase();
  const lowerQuery = q.toLowerCase();
  const nodes: React.ReactNode[] = [];
  let idx = 0;
  let hit = lowerText.indexOf(lowerQuery, idx);
  let k = 0;
  while (hit !== -1) {
    if (hit > idx) nodes.push(text.slice(idx, hit));
    nodes.push(React.createElement('mark', { key: `${keyPrefix}-m${k++}`, 'data-testid': 'term-search-highlight', className: 'rounded-xs bg-brass/30 text-(--terminal-well-fg)' }, text.slice(hit, hit + q.length)));
    idx = hit + q.length;
    hit = lowerText.indexOf(lowerQuery, idx);
  }
  if (idx < text.length) nodes.push(text.slice(idx));
  return nodes.length === 1 ? nodes[0] : nodes;
}

export interface WorkspaceFileLocatorDetail {
  path: string;
  line?: number;
  col?: number;
}

export function hasUnsafeSchemePrefix(source: string, matchIndex: number): boolean {
  if (matchIndex <= 0) return false;
  const prev = source[matchIndex - 1];
  if (prev === ':' || prev === '/' || prev === '\\' || prev === ',' || prev === '-') {
    return true;
  }
  const leading = source.slice(0, matchIndex);
  const wsIdx = leading.search(/\S+$/);
  if (wsIdx < 0) return false;
  const tokenPrefix = leading.slice(wsIdx).replace(/^[([{<"']+/, '').toLowerCase();
  if (!tokenPrefix) return false;
  if (
    tokenPrefix.startsWith('-') ||
    tokenPrefix.startsWith('/') ||
    tokenPrefix.startsWith('\\') ||
    tokenPrefix.includes('://') ||
    /^[a-z][a-z0-9+.-]*:/.test(tokenPrefix)
  ) {
    return true;
  }
  return false;
}

export function parseFileLocatorDetail(locatorText: string): WorkspaceFileLocatorDetail {
  const parenMatch = locatorText.match(/^(.*)\((\d+),(\d+)\)$/);
  if (parenMatch) {
    return { path: parenMatch[1]!, line: Number(parenMatch[2]), col: Number(parenMatch[3]) };
  }
  const colonMatch = locatorText.match(/^(.*?):(\d+)(?::(\d+))?$/);
  if (colonMatch) {
    return {
      path: colonMatch[1]!,
      line: Number(colonMatch[2]),
      ...(colonMatch[3] ? { col: Number(colonMatch[3]) } : {}),
    };
  }
  return { path: locatorText };
}

export function sanitizeWorkspaceFileLocator(raw: string): WorkspaceFileLocatorDetail | null {
  const detail = parseFileLocatorDetail(raw.trim());
  const p = detail.path;
  if (
    !p ||
    p.startsWith('-') ||
    p.startsWith('/') ||
    p.startsWith('\\') ||
    /^[A-Za-z]:[\\/]/.test(p) ||
    p.includes(':') ||
    p.includes('\0')
  ) {
    return null;
  }
  const rawSegments = p.split(/[\\/]+/);
  if (rawSegments.some((seg) => !seg || seg === '..')) {
    return null;
  }
  const cleanSegments = rawSegments.filter((seg) => seg !== '.');
  if (cleanSegments.length === 0 || cleanSegments[0]!.startsWith('-')) {
    return null;
  }
  return {
    ...detail,
    path: cleanSegments.join('/'),
  };
}

export function dispatchWorkspaceFileLocator(
  rawMatched: string,
  detail: WorkspaceFileLocatorDetail,
  openLocator?: (locator: OpenLocator) => void,
): void {
  const locator: OpenLocator = { kind: 'file', value: detail.path };
  if (openLocator) {
    openLocator(locator);
  } else {
    void Promise.resolve(voxTransport.openLocator?.(locator)).catch(() => {});
  }
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent('vox://open-file-location', {
        detail: { ...detail, value: rawMatched },
      }),
    );
  }
}

const LINK_BTN_CLS = 'inline-flex max-w-full flex-wrap items-center gap-1 break-all text-left text-(--terminal-well-cursor) underline decoration-dotted hover:opacity-90 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring';

function renderPlainLinkedText(text: string, searchQuery: string | undefined, onOpenLocator: ((loc: OpenLocator) => void) | undefined, keyPrefix: string): React.ReactNode {
  const parts: React.ReactNode[] = [];
  let cursor = 0;
  let k = 0;
  LINK_TOKEN_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  const openWebLocator = onOpenLocator ?? ((loc: OpenLocator) => {
    void Promise.resolve(voxTransport.openLocator?.(loc)).catch(() => {});
  });

  while ((m = LINK_TOKEN_RE.exec(text)) !== null) {
    if (hasUnsafeSchemePrefix(text, m.index)) continue;
    const trimmed = m[0].replace(/[.,;:!?]+$/, '');
    if (!trimmed) continue;

    if (m[1]) {
      const normalizedUrl = trimmed.startsWith('localhost:') ? `http://${trimmed}` : trimmed;
      if (normalizedUrl.startsWith('http://') || normalizedUrl.startsWith('https://')) {
        if (m.index > cursor) parts.push(renderHighlightedText(text.slice(cursor, m.index), searchQuery, `${keyPrefix}-t${k++}`));
        const onClick = (e: React.MouseEvent) => { e.stopPropagation(); openWebLocator({ kind: 'web', value: normalizedUrl }); };
        parts.push(React.createElement('button', { key: `${keyPrefix}-u${k++}`, type: 'button', 'data-testid': 'term-output-url-link', className: LINK_BTN_CLS, onClick },
          renderHighlightedText(trimmed, searchQuery, `${keyPrefix}-ul${k}`),
          React.createElement(Icon.externalLink, { className: 'size-3 shrink-0', 'aria-hidden': true })));
        cursor = m.index + trimmed.length;
        LINK_TOKEN_RE.lastIndex = cursor;
      }
    } else {
      const safeFile = sanitizeWorkspaceFileLocator(trimmed);
      if (!safeFile) continue;
      if (m.index > cursor) parts.push(renderHighlightedText(text.slice(cursor, m.index), searchQuery, `${keyPrefix}-t${k++}`));
      const onClick = (e: React.MouseEvent) => {
        e.stopPropagation();
        dispatchWorkspaceFileLocator(trimmed, safeFile, onOpenLocator);
      };
      parts.push(React.createElement('button', { key: `${keyPrefix}-f${k++}`, type: 'button', 'data-testid': 'term-output-file-link', className: LINK_BTN_CLS, onClick },
        renderHighlightedText(trimmed, searchQuery, `${keyPrefix}-fl${k}`)));
      cursor = m.index + trimmed.length;
      LINK_TOKEN_RE.lastIndex = cursor;
    }
  }
  if (cursor < text.length) parts.push(renderHighlightedText(text.slice(cursor), searchQuery, `${keyPrefix}-e`));
  return parts.length === 1 ? parts[0] : parts;
}

export function sgrToSpans(raw: string, opts?: SgrRenderOptions): React.ReactNode {
  const { cleaned, links } = extractOsc8Links(raw);
  const spans = parseSgrSpans(cleaned, opts);
  if (spans.length === 0) return null;
  return spans.map((span, idx) => {
    const children = renderWithOsc8(
      span.text, links, `s${idx}`, LINK_BTN_CLS,
      (txt, kp) => renderPlainLinkedText(txt, opts?.searchQuery, opts?.onOpenLocator, kp),
      (txt, kp) => renderHighlightedText(txt, opts?.searchQuery, kp),
      sanitizeWorkspaceFileLocator, dispatchWorkspaceFileLocator, opts?.onOpenLocator,
    );
    const classes: string[] = [];
    if (span.bold) classes.push('font-semibold');
    if (span.dim && !span.fg) classes.push('text-(--terminal-well-muted)');
    if (span.italic) classes.push('italic');
    if (span.underline && span.strikethrough) classes.push('underline line-through');
    else if (span.underline) classes.push('underline');
    else if (span.strikethrough) classes.push('line-through');
    const style: React.CSSProperties = {};
    if (span.fg) style.color = span.fg;
    if (span.bg) style.backgroundColor = span.bg;
    if (classes.length === 0 && !span.fg && !span.bg) return React.createElement(React.Fragment, { key: `s${idx}` }, children);
    return React.createElement('span', { key: `s${idx}`, ...(classes.length > 0 ? { className: classes.join(' ') } : {}), ...(span.fg || span.bg ? { style } : {}) }, children);
  });
}
