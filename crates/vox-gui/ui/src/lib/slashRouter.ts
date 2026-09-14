/** Mode slashes handled inside Loquela (no IPC). `/plan` moved to
 *  `APP_SLASH_COMMANDS` below — it now dispatches a real turn (`vox_plan`
 *  via `Execution::Plan`) instead of just switching Loquela's mode chip. */
export const INTERNAL_MODE_SLASHES = {
  '/verify': 'verify',
  '/act': 'act',
} as const;

/** `'plan'` stays a valid mode id — the composer's mode chip and
 *  `buildChatTurn`'s `mode` field both still use it — even though `/plan`
 *  no longer resolves through `INTERNAL_MODE_SLASHES`. */
export type LoquelaModeId = 'plan' | 'act' | 'verify';

/** App-level slash commands routed through `onSlashCommand` in App.tsx. */
export const APP_SLASH_COMMANDS = [
  '/plan',
  '/memory',
  '/audit',
  '/spawn',
  '/rollback',
  '/doubt',
  '/diff',
  '/research',
  '/deepresearch',
  '/research-search',
] as const;

export type AppSlashCommand = (typeof APP_SLASH_COMMANDS)[number];

/** Normalize a composer token to the bare slash command (e.g. `/plan foo` → `/plan`). */
export function slashCommandBase(cmd: string): string {
  return cmd.trim().split(/\s+/)[0]!.toLowerCase();
}

/** Returns a Loquela mode id when `cmd` is `/plan`, `/verify`, or `/act`. */
export function resolveInternalModeSlash(cmd: string): LoquelaModeId | null {
  const base = slashCommandBase(cmd);
  const mode = INTERNAL_MODE_SLASHES[base as keyof typeof INTERNAL_MODE_SLASHES];
  return (mode as LoquelaModeId | undefined) ?? null;
}

export function isAppSlashCommand(cmd: string): boolean {
  const base = slashCommandBase(cmd);
  return (APP_SLASH_COMMANDS as readonly string[]).includes(base);
}

/** Display string for session budget next to token estimate. */
export function formatSessionBudget(spent: number, cap: number): string {
  return `session $${spent.toFixed(2)} / $${cap.toFixed(2)}`;
}

export interface ParsedResearchSlashCommand {
  query: string;
  isDeep: boolean;
  waves: number;
  domainMode: 'general' | 'shopping' | 'codegen';
  siteScope?: string;
  subcommand?: 'search' | 'run';
  minConfidence?: number;
  verifiedOnly?: boolean;
}

/** Tokenize command line preserving double and single quotes */
export function tokenizeCommandLine(input: string): string[] {
  const tokens: string[] = [];
  let current = '';
  let inQuotes: '"' | "'" | null = null;

  for (let i = 0; i < input.length; i++) {
    const ch = input[i];
    if (inQuotes) {
      if (ch === inQuotes) {
        inQuotes = null;
      } else {
        current += ch;
      }
    } else if (ch === '"' || ch === "'") {
      inQuotes = ch;
    } else if (/\s/.test(ch)) {
      if (current.length > 0) {
        tokens.push(current);
        current = '';
      }
    } else {
      current += ch;
    }
  }
  if (current.length > 0) {
    tokens.push(current);
  }
  return tokens;
}

/**
 * Parses `/research`, `/deepresearch`, and `/research-search` with typed flags and subcommands:
 * e.g. `/research --deep --waves=3 --domain=codegen --site=docs.rs tokio async`
 * e.g. `/research search "rust async concurrency" --min-confidence=0.8`
 */
export function parseResearchSlashCommand(input: string): ParsedResearchSlashCommand | null {
  const trimmed = input.trim();
  const base = slashCommandBase(trimmed);
  if (base !== '/research' && base !== '/deepresearch' && base !== '/research-search') {
    return null;
  }

  const remainder = trimmed.slice(base.length).trim();
  const rawTokens = tokenizeCommandLine(remainder);

  let isDeep = base === '/deepresearch';
  let waves = isDeep ? 3 : 1;
  let domainMode: 'general' | 'shopping' | 'codegen' = 'general';
  let siteScope: string | undefined = undefined;
  let subcommand: 'search' | 'run' | undefined = base === '/research-search' ? 'search' : undefined;
  let minConfidence: number | undefined = undefined;
  let verifiedOnly: boolean | undefined = undefined;
  const queryTokens: string[] = [];

  for (let i = 0; i < rawTokens.length; i++) {
    const token = rawTokens[i];

    if (i === 0 && (token === 'search' || token === 'run')) {
      subcommand = token;
    } else if (token === '--search') {
      subcommand = 'search';
    } else if (token === '--deep' || token === '-d') {
      isDeep = true;
      if (waves === 1) waves = 3;
    } else if (token === '--verified-only') {
      verifiedOnly = true;
    } else if (token.startsWith('--waves=')) {
      const w = parseInt(token.slice(8), 10);
      if (!isNaN(w) && w > 0) {
        waves = Math.min(Math.max(w, 1), 5);
        if (waves > 1) isDeep = true;
      }
    } else if (token === '--waves' && i + 1 < rawTokens.length) {
      const w = parseInt(rawTokens[++i], 10);
      if (!isNaN(w) && w > 0) {
        waves = Math.min(Math.max(w, 1), 5);
        if (waves > 1) isDeep = true;
      }
    } else if (token.startsWith('--domain=')) {
      const mode = token.slice(9).toLowerCase();
      if (mode === 'codegen' || mode === 'shopping' || mode === 'general') {
        domainMode = mode;
      }
    } else if (token === '--domain' && i + 1 < rawTokens.length) {
      const mode = rawTokens[++i].toLowerCase();
      if (mode === 'codegen' || mode === 'shopping' || mode === 'general') {
        domainMode = mode;
      }
    } else if (token.startsWith('--site=')) {
      siteScope = token.slice(7);
    } else if (token === '--site' && i + 1 < rawTokens.length) {
      siteScope = rawTokens[++i];
    } else if (token.startsWith('--min-confidence=')) {
      const conf = parseFloat(token.slice(17));
      if (!isNaN(conf)) minConfidence = conf;
    } else if (token === '--min-confidence' && i + 1 < rawTokens.length) {
      const conf = parseFloat(rawTokens[++i]);
      if (!isNaN(conf)) minConfidence = conf;
    } else {
      queryTokens.push(token);
    }
  }

  const result: ParsedResearchSlashCommand = {
    query: queryTokens.join(' '),
    isDeep,
    waves,
    domainMode,
    siteScope,
  };

  if (subcommand !== undefined) result.subcommand = subcommand;
  if (minConfidence !== undefined) result.minConfidence = minConfidence;
  if (verifiedOnly !== undefined) result.verifiedOnly = verifiedOnly;

  return result;
}

