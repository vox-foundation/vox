import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

import { DEFAULT_REPO_ROOT } from './repo-mounts.mjs';

const LEDGER_PATH = join(DEFAULT_REPO_ROOT, 'contracts/reports/docs-freshness/ledger.v1.json');
const COMMIT_SHA_RE = /^[0-9a-f]{7,40}$/i;

let cachedLedger = null;

export function loadFreshnessLedger() {
  if (cachedLedger) return cachedLedger;
  if (!existsSync(LEDGER_PATH)) {
    console.warn(`[docs-freshness] Ledger path does not exist: ${LEDGER_PATH}`);
    return { pages: {} };
  }
  try {
    cachedLedger = JSON.parse(readFileSync(LEDGER_PATH, 'utf8'));
  } catch (e) {
    console.warn(`[docs-freshness] Failed to load ledger: ${e}`);
    cachedLedger = { pages: {} };
  }
  return cachedLedger;
}

export function getFreshnessForPath(sourcePath, filePath) {
  if (!sourcePath && !filePath) return null;

  const ledger = loadFreshnessLedger();
  const pages = ledger?.pages || {};

  let key = sourcePath ? sourcePath.replace(/\\/g, '/').replace(/^\/+/, '') : null;
  let entry = key ? pages[key] : null;

  // Fallbacks if not found directly
  if (!entry && key) {
    if (!key.startsWith('docs/src/') && !key.startsWith('crates/')) {
      entry = pages[`docs/src/${key}`];
    }
  }

  if (!entry && filePath) {
    let clean = filePath.replace(/\\/g, '/').replace(/^src\/content\/docs\//, '');
    entry = pages[`docs/src/${clean}`];
    if (!entry) {
      entry = pages[clean];
    }
  }

  if (!entry) {
    const candidateRel = key || (filePath ? filePath.replace(/\\/g, '/').replace(/^src\/content\/docs\//, '') : '');
    const normalizedRel = candidateRel.startsWith('docs/src/') ? candidateRel : `docs/src/${candidateRel}`;
    const isGenerated = normalizedRel.endsWith('.generated.md');
    let hasCodeAnchors = isGenerated;
    if (!hasCodeAnchors && candidateRel) {
      const diskPath = join(DEFAULT_REPO_ROOT, normalizedRel);
      if (existsSync(diskPath)) {
        try {
          const headText = readFileSync(diskPath, 'utf8').slice(0, 1200);
          hasCodeAnchors = /^code_anchors\s*:/m.test(headText);
        } catch {
          hasCodeAnchors = false;
        }
      }
    }
    const fallbackSha = typeof ledger?.head_commit === 'string' && COMMIT_SHA_RE.test(ledger.head_commit)
      ? ledger.head_commit
      : '';
    return {
      state: hasCodeAnchors ? 'verified' : 'unanchored',
      shortCommit: fallbackSha ? fallbackSha.slice(0, 9) : 'HEAD',
      commitUrl: fallbackSha ? `https://github.com/vox-foundation/vox/commit/${fallbackSha}` : null,
      title: null,
      anchorsCount: hasCodeAnchors ? 1 : 0,
      brokenMentions: [],
    };
  }

  const rawCommit = typeof entry.last_verified_commit === 'string' ? entry.last_verified_commit.trim() : '';
  const isValidSha = COMMIT_SHA_RE.test(rawCommit);
  const shortCommit = isValidSha ? rawCommit.slice(0, 9) : 'unknown';
  const commitUrl = isValidSha
    ? `https://github.com/vox-foundation/vox/commit/${rawCommit}`
    : null;

  return {
    state: entry.state || 'unanchored',
    shortCommit,
    commitUrl,
    title: entry.title,
    anchorsCount: entry.anchors ? entry.anchors.length : 0,
    brokenMentions: entry.broken_mentions || [],
  };
}
