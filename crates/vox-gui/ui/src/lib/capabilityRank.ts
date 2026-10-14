/**
 * Pure capability-index ranking for the palette (D-19). Mirrors the alias set of
 * crates/vox-capability-registry/src/explain.rs. Tiers: 0 exact alias, 1 alias prefix,
 * 2 every query token prefixes a row token, 3 Jaro-Winkler typo fallback (only when
 * tiers 0-2 found nothing).
 */
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';
import type { PalettePrefixMode } from '../components/layout/paletteSources';

export type RankTier = 0 | 1 | 2 | 3;
export interface RankHit {
  row: CapabilityIndexEntry;
  tier: RankTier;
  score: number;
}
export interface RankEntry {
  row: CapabilityIndexEntry;
  aliases: string[];
  tokens: string[];
}
export interface RankOptions {
  kinds?: CapabilityIndexEntry['kind'][];
  limit?: number;
  /** Ids of recently activated rows, most recent first (tie-break only). */
  recent?: readonly string[];
}

type Kind = CapabilityIndexEntry['kind'];
const ALL_KINDS: Kind[] = ['cli', 'mcp_tool', 'surface', 'crate', 'skill', 'doc'];

/** Palette prefix mode -> index row kinds it may show (D-10). */
export function kindsForMode(mode: PalettePrefixMode): Kind[] {
  switch (mode) {
    case 'commands':
      return ['cli'];
    case 'skills':
      return ['skill', 'doc'];
    case 'crates':
      return ['crate'];
    case 'agents':
      return [];
    default:
      return [...ALL_KINDS];
  }
}

const MAX_RECENT = 20;
let recent: string[] = [];
/** Session-only recency (not persisted: that would need a new shell-persistence contract key). */
export function noteRecentCapability(id: string): void {
  recent = [id, ...recent.filter((x) => x !== id)].slice(0, MAX_RECENT);
}
export function recentCapabilities(): readonly string[] {
  return [...recent];
}

/**
 * Jaro-Winkler similarity, a port of strsim 0.11 (generic_jaro + jaro_winkler: window
 * max(len)/2 - 1, integer-halved transpositions, prefix scale 0.1 over at most 4 chars,
 * boost only above 0.7) so the GUI agrees with explain.rs.
 */
export function jaroWinkler(a: string, b: string): number {
  if (a === b) return 1;
  const x = [...a];
  const y = [...b];
  if (!x.length || !y.length) return 0;
  const range = Math.max(0, Math.floor(Math.max(x.length, y.length) / 2) - 1);
  const flagsX = new Array<boolean>(x.length).fill(false);
  const flagsY = new Array<boolean>(y.length).fill(false);
  let matches = 0;
  for (let i = 0; i < x.length; i++) {
    const end = Math.min(y.length, i + range + 1);
    for (let j = Math.max(0, i - range); j < end; j++) {
      if (!flagsY[j] && x[i] === y[j]) {
        flagsX[i] = flagsY[j] = true;
        matches++;
        break;
      }
    }
  }
  if (!matches) return 0;
  let t = 0;
  let j = 0;
  for (let i = 0; i < x.length; i++) {
    if (!flagsX[i]) continue;
    while (!flagsY[j]) j++;
    if (x[i] !== y[j]) t++;
    j++;
  }
  t = Math.floor(t / 2);
  const jaro = (matches / x.length + matches / y.length + (matches - t) / matches) / 3;
  if (jaro <= 0.7) return jaro;
  let prefix = 0;
  while (prefix < 4 && prefix < x.length && prefix < y.length && x[prefix] === y[prefix]) prefix++;
  return jaro + 0.1 * prefix * (1 - jaro);
}

/** Same bound as explain.rs MAX_QUERY_CHARS: scoring cost never depends on pasted text. */
const MAX_QUERY_CHARS = 256;

export function normalizeQuery(raw: string): string {
  const q = raw.slice(0, MAX_QUERY_CHARS).toLowerCase().trim();
  return q.startsWith('vox ') ? q.slice(4).trim() : q;
}

const splitTokens = (s: string) => s.split(/[^\p{L}\p{N}]+/u).filter(Boolean);

function aliasesOf(row: CapabilityIndexEntry): string[] {
  const a = [row.id.toLowerCase()];
  if (row.cli) {
    const inv = row.cli.toLowerCase();
    const path = inv.replace(/^vox\s+/, '');
    a.push(inv, path.replace(/ /g, '.'), path);
  }
  if (row.mcp) {
    const name = row.mcp.toLowerCase();
    if (name.startsWith('vox_')) a.push(name.slice(4));
    a.push(name);
  }
  const tail = (prefix: string) => (row.id.startsWith(prefix) ? row.id.slice(prefix.length).toLowerCase() : null);
  if (row.kind === 'surface' && row.viewKey) a.push(row.viewKey.toLowerCase());
  if (row.kind === 'crate') {
    const name = tail('crate.');
    if (name) a.push(name, ...(name.startsWith('vox-') ? [name.slice(4)] : []));
  }
  if (row.kind === 'skill') {
    const name = tail('skill.');
    if (name) a.push(name);
  }
  a.push(row.title.toLowerCase());
  return a;
}

export function toRankEntries(
  rows: CapabilityIndexEntry[],
  extraAliases: Record<string, string[]> | ((row: CapabilityIndexEntry) => string[]) = {},
): RankEntry[] {
  const extra = typeof extraAliases === 'function' ? extraAliases : (r: CapabilityIndexEntry) => extraAliases[r.id] ?? [];
  return rows.map((row) => {
    const aliases = [...aliasesOf(row), ...extra(row).map((s) => s.toLowerCase())];
    return { row, aliases, tokens: [...new Set(aliases.flatMap(splitTokens))] };
  });
}

/** Tier 3 floor: JW alone is noisy (755 rows at 0.75 for `vox fabrica fmt`, D-19), so it only fires as a typo fallback. */
const JW_MIN = 0.85;
const JW_MIN_QUERY_CHARS = 3;
const TIER_RANK: Record<string, number> = { recommended: 0, advanced: 1, feature_gated: 2 };

function tier2Score(qTokens: string[], tokens: string[]): number {
  let sum = 0;
  for (const qt of qTokens) {
    if (tokens.includes(qt)) sum += 1;
    else if (tokens.some((t) => t.startsWith(qt))) sum += 0.5;
    else return 0;
  }
  return sum / qTokens.length;
}

export function rankCapabilities(entries: RankEntry[], rawQuery: string, opts: RankOptions = {}): RankHit[] {
  const q = normalizeQuery(rawQuery);
  if (!q) return [];
  const { kinds, limit = 50, recent: recentIds = [] } = opts;
  const pool = kinds ? entries.filter((e) => kinds.includes(e.row.kind)) : entries;

  let hits: RankHit[] = [];
  const layer = kinds?.length === 1 && kinds[0] === 'crate' ? /^l(?:ayer)?\s*(\d+)$/.exec(q) : null;
  if (layer) {
    hits = pool.filter((e) => e.row.layer === Number(layer[1])).map((e) => ({ row: e.row, tier: 0 as const, score: 1 }));
  } else {
    const qTokens = splitTokens(q);
    for (const e of pool) {
      if (e.aliases.includes(q)) {
        hits.push({ row: e.row, tier: 0, score: 1 });
        continue;
      }
      let best = 0;
      for (const alias of e.aliases) if (alias.startsWith(q)) best = Math.max(best, q.length / alias.length);
      if (best > 0) {
        hits.push({ row: e.row, tier: 1, score: best });
        continue;
      }
      const s2 = qTokens.length ? tier2Score(qTokens, e.tokens) : 0;
      if (s2 > 0) hits.push({ row: e.row, tier: 2, score: s2 });
    }
    if (!hits.length && q.length >= JW_MIN_QUERY_CHARS) {
      for (const e of pool) {
        let best = 0;
        for (const alias of e.aliases) {
          // JW >= 0.85 needs jaro >= 0.75, i.e. min/max length >= 0.25 (cheap exact prune).
          if (Math.min(alias.length, q.length) / Math.max(alias.length, q.length) < 0.25) continue;
          best = Math.max(best, jaroWinkler(q, alias));
        }
        if (best >= JW_MIN) hits.push({ row: e.row, tier: 3, score: best });
      }
    }
  }

  // Tie-breaks are Claude's discretion (CONTEXT, ranking): tier, score to 2 dp, dedicated
  // surface, catalog tier, session recency, then id so the order is deterministic.
  const recency = (id: string) => {
    const i = recentIds.indexOf(id);
    return i < 0 ? Infinity : i;
  };
  hits.sort(
    (x, y) =>
      x.tier - y.tier ||
      Math.round(y.score * 100) - Math.round(x.score * 100) ||
      Number(!!y.row.viewKey) - Number(!!x.row.viewKey) ||
      (TIER_RANK[x.row.tier ?? ''] ?? 3) - (TIER_RANK[y.row.tier ?? ''] ?? 3) ||
      (recency(x.row.id) === recency(y.row.id) ? 0 : recency(x.row.id) < recency(y.row.id) ? -1 : 1) ||
      (x.row.id < y.row.id ? -1 : x.row.id > y.row.id ? 1 : 0),
  );
  return hits.slice(0, limit);
}
