import type { IDisposable, ILink, ITerminalAddon, Terminal } from '@xterm/xterm';
import { voxTransport, type OpenLocator } from '../../../transport';
import {
  dispatchWorkspaceFileLocator,
  hasUnsafeSchemePrefix,
  sanitizeWorkspaceFileLocator,
} from './sgrToSpans';

export interface SearchMatchStats {
  activeMatchIndex: number;
  totalMatches: number;
}

interface BufferMatch { row: number; col: number; length: number; }

export function unicode11Wcwidth(cp: number): 0 | 1 | 2 {
  if (cp === 0 || cp < 32 || (cp >= 0x7f && cp < 0xa0)) return 0;
  if (
    (cp >= 0x0300 && cp <= 0x036f) || (cp >= 0x1ab0 && cp <= 0x1aff) || (cp >= 0x1dc0 && cp <= 0x1dff) ||
    (cp >= 0x200b && cp <= 0x200f) || (cp >= 0x20d0 && cp <= 0x20ff) || (cp >= 0xfe00 && cp <= 0xfe2f)
  ) return 0;
  if (
    cp >= 0x1100 && (
      cp <= 0x115f || cp === 0x2329 || cp === 0x232a || (cp >= 0x2e80 && cp <= 0xa4cf && cp !== 0x303f) ||
      (cp >= 0xac00 && cp <= 0xd7a3) || (cp >= 0xf900 && cp <= 0xfaff) || (cp >= 0xfe10 && cp <= 0xfe19) ||
      (cp >= 0xfe30 && cp <= 0xfe6f) || (cp >= 0xff00 && cp <= 0xff60) || (cp >= 0xffe0 && cp <= 0xffe6) ||
      (cp >= 0x1f300 && cp <= 0x1faff) || (cp >= 0x20000 && cp <= 0x3fffd)
    )
  ) return 2;
  return 1;
}

export function stringCellWidth(str: string): number {
  let cells = 0;
  for (const ch of str) {
    const cp = ch.codePointAt(0);
    if (cp !== undefined) cells += unicode11Wcwidth(cp);
  }
  return cells;
}

export class SearchTerminalAddon implements ITerminalAddon {
  private terminal: Terminal | null = null;
  private lastQuery = '';
  private cursorIdx = -1;

  activate(terminal: Terminal): void { this.terminal = terminal; }

  private collectMatches(query: string): BufferMatch[] {
    const buf = this.terminal?.buffer?.active;
    if (!query || !buf || typeof buf.length !== 'number') return [];
    const needle = query.toLowerCase();
    const matches: BufferMatch[] = [];
    for (let row = 0; row < buf.length; row++) {
      const lineText = buf.getLine?.(row)?.translateToString?.(true) ?? '';
      if (!lineText) continue;
      const hay = lineText.toLowerCase();
      for (let from = 0; from <= hay.length - needle.length; ) {
        const utf16Col = hay.indexOf(needle, from);
        if (utf16Col < 0) break;
        const col = stringCellWidth(lineText.slice(0, utf16Col));
        const length = Math.max(1, stringCellWidth(lineText.slice(utf16Col, utf16Col + needle.length)));
        matches.push({ row, col, length });
        from = utf16Col + Math.max(1, needle.length);
      }
    }
    return matches;
  }

  private stepMatch(query: string, dir: 1 | -1): SearchMatchStats {
    if (!query) {
      this.clear();
      return { activeMatchIndex: 0, totalMatches: 0 };
    }
    const matches = this.collectMatches(query);
    if (matches.length === 0) {
      this.lastQuery = query;
      this.cursorIdx = -1;
      this.terminal?.clearSelection?.();
      return { activeMatchIndex: 0, totalMatches: 0 };
    }
    if (query !== this.lastQuery || this.cursorIdx < 0) {
      this.lastQuery = query;
      this.cursorIdx = dir === 1 ? 0 : matches.length - 1;
    } else {
      this.cursorIdx = (this.cursorIdx + dir + matches.length) % matches.length;
    }
    const hit = matches[this.cursorIdx]!;
    this.terminal?.select?.(hit.col, hit.row, hit.length);
    this.terminal?.scrollToLine?.(hit.row);
    return { activeMatchIndex: this.cursorIdx + 1, totalMatches: matches.length };
  }

  findNext(query: string): SearchMatchStats { return this.stepMatch(query, 1); }
  findPrevious(query: string): SearchMatchStats { return this.stepMatch(query, -1); }

  clear(): void {
    this.lastQuery = '';
    this.cursorIdx = -1;
    try { this.terminal?.clearSelection?.(); } catch { /* ignore */ }
  }

  dispose(): void {
    this.clear();
    this.terminal = null;
  }
}

export interface WebLinksAddonOptions {
  onOpenLocator?: (locator: OpenLocator) => void;
}

const HTTP_URL_RE = /\bhttps?:\/\/[^\s<>"')\]]+/g;
const LOCALHOST_URL_RE = /\blocalhost:\d+(?:\/[^\s<>"')\]]*)?/g;
const FILE_LOCATOR_RE =
  /(?:[A-Za-z]:[\\/]|(?:\.{1,2}[\\/])|(?:[\w@-]+[\\/])+)[\w.-]+\.[A-Za-z0-9]+(?::\d+(?::\d+)?|\(\d+,\d+\))/g;
const TRAILING_PUNCT_RE = /[.,;:!?]+$/;

function isSafeHttpUrl(candidate: string): boolean {
  try {
    const parsed = new URL(candidate);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

export class WebLinksTerminalAddon implements ITerminalAddon {
  private disposable: IDisposable | null = null;
  private readonly onOpenLocator?: (locator: OpenLocator) => void;

  constructor(options?: WebLinksAddonOptions) {
    this.onOpenLocator = options?.onOpenLocator;
  }

  private dispatchWebLocator(url: string): void {
    const locator: OpenLocator = { kind: 'web', value: url };
    if (this.onOpenLocator) this.onOpenLocator(locator);
    else voxTransport.openLocator(locator).catch(() => {});
  }

  activate(terminal: Terminal): void {
    try {
      if (!terminal?.registerLinkProvider) return;
      this.disposable = terminal.registerLinkProvider({
        provideLinks: (bufferLineNumber: number, callback: (links: ILink[] | undefined) => void) => {
          const text = terminal.buffer?.active?.getLine?.(bufferLineNumber - 1)?.translateToString?.(true) ?? '';
          if (!text) { callback(undefined); return; }
          const hits: Array<{ startIdx: number; endIdx: number; link: ILink }> = [];
          const pushHit = (startIdx: number, matched: string, activateFn: () => void, checkOverlap: boolean) => {
            const endIdx = startIdx + matched.length;
            if (checkOverlap && hits.some((h) => startIdx < h.endIdx && endIdx > h.startIdx)) return;
            const startCell = stringCellWidth(text.slice(0, startIdx)) + 1;
            const endCell = startCell + Math.max(1, stringCellWidth(matched)) - 1;
            hits.push({
              startIdx, endIdx,
              link: {
                text: matched,
                range: { start: { x: startCell, y: bufferLineNumber }, end: { x: endCell, y: bufferLineNumber } },
                activate: activateFn,
              },
            });
          };

          for (const m of text.matchAll(HTTP_URL_RE)) {
            const startIdx = m.index ?? -1;
            const matched = m[0].replace(TRAILING_PUNCT_RE, '');
            if (startIdx >= 0 && matched && !hasUnsafeSchemePrefix(text, startIdx) && isSafeHttpUrl(matched)) {
              pushHit(startIdx, matched, () => this.dispatchWebLocator(matched), false);
            }
          }
          for (const m of text.matchAll(LOCALHOST_URL_RE)) {
            const startIdx = m.index ?? -1;
            const matched = m[0].replace(TRAILING_PUNCT_RE, '');
            if (startIdx >= 0 && matched && !hasUnsafeSchemePrefix(text, startIdx)) {
              const normalized = `http://${matched}`;
              if (isSafeHttpUrl(normalized)) {
                pushHit(startIdx, matched, () => this.dispatchWebLocator(normalized), true);
              }
            }
          }
          for (const m of text.matchAll(FILE_LOCATOR_RE)) {
            const startIdx = m.index ?? -1;
            const matched = m[0].replace(TRAILING_PUNCT_RE, '');
            if (startIdx >= 0 && matched && !hasUnsafeSchemePrefix(text, startIdx)) {
              const safeDetail = sanitizeWorkspaceFileLocator(matched);
              if (safeDetail) {
                pushHit(
                  startIdx,
                  matched,
                  () => dispatchWorkspaceFileLocator(matched, safeDetail, this.onOpenLocator),
                  true,
                );
              }
            }
          }

          hits.sort((a, b) => a.startIdx - b.startIdx);
          callback(hits.length > 0 ? hits.map((h) => h.link) : undefined);
        },
      });
    } catch { /* mock terminal without link provider support */ }
  }

  dispose(): void {
    try { this.disposable?.dispose?.(); } catch { /* ignore */ }
    this.disposable = null;
  }
}

export function unicode11CharProperties(cp: number, _preceding = 0): number {
  return (unicode11Wcwidth(cp) & 3) << 1;
}

export class Unicode11TerminalAddon implements ITerminalAddon {
  activate(terminal: Terminal): void {
    try {
      const unicodeApi = terminal?.unicode;
      if (!unicodeApi?.register) return;
      if (!unicodeApi.versions?.includes('11')) {
        unicodeApi.register({
          version: '11',
          wcwidth: unicode11Wcwidth,
          charProperties: unicode11CharProperties,
        });
      }
      unicodeApi.activeVersion = '11';
    } catch { /* proposed API not enabled or minimal test mock */ }
  }
  dispose(): void { /* unicode provider lives with terminal instance */ }
}

export class SerializeTerminalAddon implements ITerminalAddon {
  private terminal: Terminal | null = null;
  activate(terminal: Terminal): void { this.terminal = terminal; }

  serialize(maxLines?: number): string {
    const buf = this.terminal?.buffer?.active;
    if (!buf || typeof buf.length !== 'number' || buf.length <= 0) return '';
    const startRow = maxLines && maxLines > 0 ? Math.max(0, buf.length - maxLines) : 0;
    const logicalLines: string[] = [];
    for (let y = startRow; y < buf.length; y++) {
      const line = buf.getLine?.(y);
      if (!line) continue;
      const chunk = line.translateToString?.(true) ?? '';
      if (line.isWrapped && logicalLines.length > 0) logicalLines[logicalLines.length - 1] += chunk;
      else logicalLines.push(chunk);
    }
    while (logicalLines.length > 0 && logicalLines[logicalLines.length - 1] === '') logicalLines.pop();
    return logicalLines.join('\n');
  }

  dispose(): void { this.terminal = null; }
}
