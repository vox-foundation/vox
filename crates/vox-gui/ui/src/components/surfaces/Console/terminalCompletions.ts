import * as transport from '../../../transport';
import type { Suggestion, TermCompletionItemDto, TermCompletionKindDto } from '../../../transport';
import {
  computeIntelliSenseItems,
  parseCompletionTrigger,
  type CompletionCategory,
  type IntelliSenseItem,
} from '../../../lib/intellisenseEngine';
import type { TerminalBridgeState } from '../../../lib/terminalBridge';
import type { ShellMode } from './intentRouter';

export type TerminalCompletionBadge =
  | 'BIN' | 'DIR' | 'FILE' | 'GIT' | 'FLAG' | 'SUB' | 'MACRO' | 'SLASH' | 'HIST';

export interface TerminalSuggestion extends IntelliSenseItem {
  badge: TerminalCompletionBadge;
  replaceStart: number;
  replaceEnd: number;
  insertText: string;
}

export interface ResolveTerminalCompletionsOptions {
  cwd?: string;
  shellMode?: ShellMode;
  knownSetKeys?: string[];
  catalogSuggestions?: Suggestion[];
  termBridge?: TerminalBridgeState;
  forceOpen?: boolean;
  historyCommands?: string[];
}

const utf8Encoder = new TextEncoder();
const utf8Decoder = new TextDecoder();

export function utf16IndexToUtf8ByteOffset(str: string, utf16Index: number): number {
  const clamped = Math.max(0, Math.min(utf16Index, str.length));
  return utf8Encoder.encode(str.slice(0, clamped)).byteLength;
}

export function utf8ByteOffsetToUtf16Index(str: string, byteOffset: number): number {
  if (byteOffset <= 0) return 0;
  const bytes = utf8Encoder.encode(str);
  if (byteOffset >= bytes.byteLength) return str.length;
  let validEnd = byteOffset;
  while (validEnd > 0 && (bytes[validEnd] & 0xc0) === 0x80) validEnd--;
  return utf8Decoder.decode(bytes.subarray(0, validEnd)).length;
}

export interface TerminalCompletionsResult {
  items: TerminalSuggestion[];
  ghostSuffix: string;
  activeActionId: string | null;
}

const DTO_KIND_MAP: Record<TermCompletionKindDto, { badge: TerminalCompletionBadge; category: CompletionCategory }> = {
  executable: { badge: 'BIN', category: 'command' },
  builtin: { badge: 'BIN', category: 'command' },
  directory: { badge: 'DIR', category: 'file' },
  path: { badge: 'FILE', category: 'file' },
  git_ref: { badge: 'GIT', category: 'command' },
  flag: { badge: 'FLAG', category: 'command' },
  subcommand: { badge: 'SUB', category: 'command' },
  vox_macro: { badge: 'MACRO', category: 'term' },
  slash: { badge: 'SLASH', category: 'slash' },
  history: { badge: 'HIST', category: 'history' },
};

const LOCAL_TERMINAL_SLASH = [
  { cmd: '/clear', insertText: '/clear ', desc: 'Clear active terminal scrollback and blocks', usage: '/clear' },
  { cmd: '/ask', insertText: '/ask ', desc: 'Ask the orchestrator agent using terminal context', usage: '/ask <prompt>' },
];

function getTransportFn<K extends keyof typeof transport>(key: K): (typeof transport)[K] | undefined {
  try {
    const fn = transport[key];
    return typeof fn === 'function' ? fn : undefined;
  } catch {
    return undefined;
  }
}

function mapLocalItemBadge(item: IntelliSenseItem): TerminalCompletionBadge {
  if (item.category === 'slash' || item.category === 'skill') return 'SLASH';
  if (item.category === 'term' || item.category === 'mcp' || item.category === 'symbol') return 'MACRO';
  if (item.category === 'history') return 'HIST';
  if (item.category === 'file') {
    if (item.insertText.startsWith('@')) return 'MACRO';
    return item.insertText.endsWith('/') ? 'DIR' : 'FILE';
  }
  if (item.badge.toLowerCase() === 'flag') return 'FLAG';
  if (item.category === 'nu' || item.id.startsWith('catalog-')) return 'SUB';
  return 'BIN';
}

function computeCaseSensitiveGhost(
  line: string, cursor: number, replaceStart: number, replaceEnd: number, insertText: string,
): string {
  if (cursor !== line.length || replaceEnd !== line.length) return '';
  const activeToken = line.slice(replaceStart, replaceEnd);
  return activeToken && insertText.startsWith(activeToken) ? insertText.slice(activeToken.length) : '';
}

function assembleCompletions(
  line: string,
  cursor: number,
  opts: ResolveTerminalCompletionsOptions | undefined,
  catalogList: Suggestion[],
  backendList: TermCompletionItemDto[] | null,
): TerminalCompletionsResult {
  const clamped = Math.max(0, Math.min(cursor, line.length));
  const trigger = parseCompletionTrigger(line, clamped, 'terminal');
  const engineMode = opts?.shellMode === 'nu' || opts?.shellMode === 'vox' ? opts.shellMode : 'default';
  const localRaw = computeIntelliSenseItems({
    input: line,
    cursorOffset: clamped,
    surface: 'terminal',
    shellMode: engineMode,
    catalogSuggestions: catalogList,
    termBridge: opts?.termBridge,
    forceOpen: opts?.forceOpen,
  });

  const merged: TerminalSuggestion[] = [];
  const seenInsert = new Set<string>();

  if (trigger.kind === 'slash' || (opts?.forceOpen && !trigger.rawToken)) {
    const q = trigger.query.toLowerCase();
    for (const entry of LOCAL_TERMINAL_SLASH) {
      const bare = entry.cmd.slice(1);
      if (q && !bare.startsWith(q) && !entry.desc.toLowerCase().includes(q)) continue;
      seenInsert.add(entry.insertText);
      seenInsert.add(entry.cmd);
      merged.push({
        id: `local-slash-${bare}`,
        label: entry.cmd,
        insertText: entry.insertText,
        ghostSuffix: computeCaseSensitiveGhost(line, clamped, trigger.replaceStart, trigger.replaceEnd, entry.insertText),
        category: 'slash',
        badge: 'SLASH',
        description: entry.desc,
        signature: entry.usage,
        actionId: `slash.${bare}`,
        score: bare.startsWith(q) ? 930 : 700,
        replaceStart: trigger.replaceStart,
        replaceEnd: trigger.replaceEnd,
      });
    }
  }

  if (trigger.kind === 'at_all' || (trigger.kind === 'at_file' && !trigger.query)) {
    const atFile = '@file:';
    if (atFile.startsWith(trigger.rawToken)) {
      seenInsert.add(atFile);
      merged.push({
        id: 'macro-at-file-prefix',
        label: atFile,
        insertText: atFile,
        ghostSuffix: computeCaseSensitiveGhost(line, clamped, trigger.replaceStart, trigger.replaceEnd, atFile),
        category: 'file',
        badge: 'MACRO',
        description: 'Reference a workspace file path in the prompt',
        signature: '@file:<path>',
        score: 910,
        replaceStart: trigger.replaceStart,
        replaceEnd: trigger.replaceEnd,
      });
    }
  }

  for (const dto of backendList ?? []) {
    if (seenInsert.has(dto.insert_text)) continue;
    seenInsert.add(dto.insert_text);
    const { badge, category } = DTO_KIND_MAP[dto.kind];
    const replaceStart = utf8ByteOffsetToUtf16Index(line, dto.replace_start);
    const replaceEnd = utf8ByteOffsetToUtf16Index(line, dto.replace_end);
    const normalizedScore = dto.score <= 120 ? Math.min(990, dto.score * 10) : Math.min(990, dto.score);
    merged.push({
      id: `ipc-${dto.kind}-${dto.insert_text}`,
      label: dto.label,
      insertText: dto.insert_text,
      ghostSuffix: computeCaseSensitiveGhost(line, clamped, replaceStart, replaceEnd, dto.insert_text),
      category,
      badge,
      description: dto.detail ?? `${badge} completion`,
      signature: dto.insert_text,
      score: normalizedScore,
      replaceStart,
      replaceEnd,
    });
  }

  for (const item of localRaw) {
    if (seenInsert.has(item.insertText)) continue;
    seenInsert.add(item.insertText);
    merged.push({
      ...item,
      badge: mapLocalItemBadge(item),
      replaceStart: trigger.replaceStart,
      replaceEnd: trigger.replaceEnd,
      ghostSuffix: computeCaseSensitiveGhost(line, clamped, trigger.replaceStart, trigger.replaceEnd, item.insertText),
    });
  }

  if (opts?.knownSetKeys?.length && trigger.rawToken.startsWith('$')) {
    const prefix = trigger.rawToken.slice(1);
    for (const key of opts.knownSetKeys) {
      const token = `$${key}`;
      if (!key.startsWith(prefix) || seenInsert.has(token)) continue;
      seenInsert.add(token);
      merged.push({
        id: `env-${key}`,
        label: token,
        insertText: token,
        ghostSuffix: computeCaseSensitiveGhost(line, clamped, trigger.replaceStart, trigger.replaceEnd, token),
        category: 'symbol',
        badge: 'MACRO',
        description: 'Environment or session variable',
        signature: token,
        score: 760,
        replaceStart: trigger.replaceStart,
        replaceEnd: trigger.replaceEnd,
      });
    }
  }

  if (opts?.historyCommands?.length && !line.includes('\n') && clamped === line.length) {
    const trimmedLead = line.trimStart();
    const leadOffset = line.length - trimmedLead.length;
    if (trimmedLead.length > 0) {
      for (const histCmd of opts.historyCommands) {
        if (!histCmd.startsWith(trimmedLead) || histCmd === trimmedLead || seenInsert.has(histCmd)) continue;
        seenInsert.add(histCmd);
        merged.push({
          id: `local-hist-${histCmd}`,
          label: histCmd,
          insertText: histCmd,
          ghostSuffix: computeCaseSensitiveGhost(line, clamped, leadOffset, line.length, histCmd),
          category: 'history',
          badge: 'HIST',
          description: 'Recent workspace command',
          signature: histCmd,
          score: 845,
          replaceStart: leadOffset,
          replaceEnd: line.length,
        });
      }
    }
  }

  merged.sort((a, b) => b.score - a.score || a.label.localeCompare(b.label));
  const items = merged.slice(0, 50);
  const top = items[0];
  const ghostSuffix = top
    ? computeCaseSensitiveGhost(line, clamped, top.replaceStart, top.replaceEnd, top.insertText)
    : '';
  return {
    items,
    ghostSuffix,
    activeActionId: top?.actionId ?? catalogList[0]?.action_id ?? null,
  };
}

export function computeLocalTerminalCompletions(
  line: string, cursor: number, opts?: ResolveTerminalCompletionsOptions,
): TerminalCompletionsResult {
  return assembleCompletions(line, cursor, opts, opts?.catalogSuggestions ?? [], null);
}

export async function resolveTerminalCompletions(
  line: string, cursor: number, opts?: ResolveTerminalCompletionsOptions,
): Promise<TerminalCompletionsResult> {
  const clamped = Math.max(0, Math.min(cursor, line.length));
  const trimmed = line.trimStart();
  const shouldQueryCatalog =
    opts?.catalogSuggestions === undefined &&
    (trimmed.startsWith('vox') || trimmed.startsWith('/') || trimmed.startsWith('!nu') || (opts?.shellMode === 'vox' && trimmed.length > 0));

  const discoverFn = shouldQueryCatalog ? getTransportFn('discoverySuggest') : undefined;
  const completeFn = getTransportFn('termComplete');

  const [catalogList, backendList] = await Promise.all([
    discoverFn
      ? Promise.resolve(discoverFn(line, 8)).catch(() => [] as Suggestion[])
      : Promise.resolve(opts?.catalogSuggestions ?? []),
    completeFn && (line.trim().length > 0 || opts?.forceOpen)
      ? Promise.resolve(
          completeFn({
            line,
            cursor: utf16IndexToUtf8ByteOffset(line, clamped),
            cwd: opts?.cwd,
            shellMode: opts?.shellMode,
            limit: 30,
          }),
        ).catch(() => null)
      : Promise.resolve<TermCompletionItemDto[] | null>(null),
  ]);

  return assembleCompletions(line, clamped, opts, Array.isArray(catalogList) ? catalogList : [], backendList);
}

export function applyTerminalSuggestion(
  line: string, item: TerminalSuggestion,
): { value: string; cursor: number } {
  const start = Math.max(0, Math.min(item.replaceStart, line.length));
  const end = Math.max(start, Math.min(item.replaceEnd, line.length));
  const prefix = line.slice(0, start);
  return { value: `${prefix}${item.insertText}${line.slice(end)}`, cursor: prefix.length + item.insertText.length };
}
