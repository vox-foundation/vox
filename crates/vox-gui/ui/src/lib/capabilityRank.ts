/**
 * Pure capability-index ranking for the palette. Mirrors the alias set of
 * crates/vox-capability-registry/src/explain.rs. Tiers: 0 exact alias, 1 alias prefix
 * (tiers 2-3 and tie-breaks arrive in a later plan behind this same API).
 */
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';

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

export function toRankEntries(rows: CapabilityIndexEntry[], extraAliases: Record<string, string[]> = {}): RankEntry[] {
  return rows.map((row) => {
    const aliases = [...aliasesOf(row), ...(extraAliases[row.id] ?? []).map((s) => s.toLowerCase())];
    return { row, aliases, tokens: [...new Set(aliases.flatMap(splitTokens))] };
  });
}

export function rankCapabilities(entries: RankEntry[], rawQuery: string, opts: RankOptions = {}): RankHit[] {
  const q = normalizeQuery(rawQuery);
  if (!q) return [];
  const { kinds, limit = 50 } = opts;
  const hits: RankHit[] = [];
  for (const e of entries) {
    if (kinds && !kinds.includes(e.row.kind)) continue;
    if (e.aliases.includes(q)) {
      hits.push({ row: e.row, tier: 0, score: 1 });
      continue;
    }
    let best = 0;
    for (const alias of e.aliases) if (alias.startsWith(q)) best = Math.max(best, q.length / alias.length);
    if (best > 0) hits.push({ row: e.row, tier: 1, score: best });
  }
  hits.sort((x, y) => x.tier - y.tier || y.score - x.score || (x.row.id < y.row.id ? -1 : x.row.id > y.row.id ? 1 : 0));
  return hits.slice(0, limit);
}
