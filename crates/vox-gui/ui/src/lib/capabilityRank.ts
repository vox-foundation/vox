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

export function normalizeQuery(raw: string): string {
  return raw;
}
export function toRankEntries(_rows: CapabilityIndexEntry[], _extraAliases?: Record<string, string[]>): RankEntry[] {
  return [];
}
export function rankCapabilities(_entries: RankEntry[], _rawQuery: string, _opts: RankOptions = {}): RankHit[] {
  return [];
}
