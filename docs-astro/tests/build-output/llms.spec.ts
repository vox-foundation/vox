import { test, expect } from '@playwright/test';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { DIST_DIR } from '../lib/dist';
// @ts-ignore
import { listDocPages, internalsDocIds } from '../../src/utils/page-index.mjs';
// @ts-ignore
import { statusPolicy } from '../../src/utils/page-status.mjs';
// @ts-ignore
import { docSlug } from '../../src/utils/doc-slug.mjs';

const REPO_ROOT = resolve(DIST_DIR, '..', '..');
const DOCS_SRC = join(REPO_ROOT, 'docs/src');

test.describe('Build Output: LLMS exclusion (HONEST-02, DEPLOY-03)', () => {
  const allPages = listDocPages(DOCS_SRC);
  const internalsIds = new Set(internalsDocIds(allPages));

  // Build sets of titles
  const nonInternalsTitles = new Set<string>();
  const rawInternalsTitles = new Set<string>();

  for (const page of allPages) {
    if (internalsIds.has(page.id)) {
      if (page.title) rawInternalsTitles.add(page.title);
    } else {
      if (page.title) nonInternalsTitles.add(page.title);
    }
  }

  // Skip shared titles (log them)
  const internalsTitles = new Set<string>();
  for (const title of rawInternalsTitles) {
    if (nonInternalsTitles.has(title)) {
      console.warn(`[llms.spec] Skipping shared title between internals and non-internals: "${title}"`);
    } else {
      internalsTitles.add(title);
    }
  }

  test('llms-full.txt and llms-small.txt exist and exclude Internals pages', () => {
    const fullPath = join(DIST_DIR, 'llms-full.txt');
    const smallPath = join(DIST_DIR, 'llms-small.txt');
    expect(existsSync(fullPath)).toBe(true);
    expect(existsSync(smallPath)).toBe(true);

    const generatedFiles = [
      'llms-full.txt',
      'llms-small.txt',
      'llms.txt',
    ];

    const customDir = join(DIST_DIR, '_llms-txt');
    if (existsSync(customDir)) {
      const files = readdirSync(customDir).filter((f) => f.endsWith('.txt'));
      for (const f of files) {
        generatedFiles.push(`_llms-txt/${f}`);
      }
    }

    for (const rel of generatedFiles) {
      const p = join(DIST_DIR, rel);
      if (!existsSync(p)) continue;
      const content = readFileSync(p, 'utf8');
      const lines = content.split('\n');

      const violations: string[] = [];
      for (const line of lines) {
        if (/^#+\s+/.test(line)) {
          const heading = line.replace(/^#+\s+/, '').trim();
          if (internalsTitles.has(heading)) {
            violations.push(heading);
          }
        }
      }

      expect(
        violations.length,
        `Found internals page headings in ${rel}: ${violations.slice(0, 5).join(', ')}`
      ).toBe(0);
    }
  });

  test('retired .well-known/llms-full.txt is deleted and 301-redirected in _redirects (TRUST-03)', () => {
    const fullDist = join(DIST_DIR, '.well-known/llms-full.txt');
    const fullSrc = join(DOCS_SRC, '.well-known/llms-full.txt');
    expect(existsSync(fullDist), '.well-known/llms-full.txt must not exist in dist').toBe(false);
    expect(existsSync(fullSrc), '.well-known/llms-full.txt must not exist in docs/src').toBe(false);

    const redirectsPath = join(DIST_DIR, '_redirects');
    expect(existsSync(redirectsPath), '_redirects file must exist in dist').toBe(true);
    const redirects = readFileSync(redirectsPath, 'utf8');
    expect(
      redirects,
      'Expected _redirects to contain 301 redirect for /.well-known/llms-full.txt to /llms-full.txt'
    ).toMatch(/\/\.well-known\/llms-full\.txt\s+\/llms-full\.txt\s+301/);
  });

  test('curated .well-known/llms.txt links point only to verified reader-facing pages (TRUST-02)', () => {
    const p = join(DOCS_SRC, '.well-known/llms.txt');
    expect(existsSync(p), 'docs/src/.well-known/llms.txt must exist').toBe(true);
    const content = readFileSync(p, 'utf8');

    const ledgerPath = join(REPO_ROOT, 'contracts/reports/docs-freshness/ledger.v1.json');
    expect(existsSync(ledgerPath), 'Freshness ledger must exist').toBe(true);
    const ledger = JSON.parse(readFileSync(ledgerPath, 'utf8'));

    const linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
    let m;
    const missingUrls: string[] = [];
    const nonVerifiedUrls: string[] = [];
    const internalsUrls: string[] = [];

    // Helper to resolve an extracted URL slug to its ledger key
    function resolveUrlToLedger(cleanPath: string) {
      for (const [k, pageEntry] of Object.entries<any>(ledger.pages)) {
        const relUnderDocs = k.replace(/^docs\/src\//, '');
        const slug = docSlug(relUnderDocs);
        const kClean = relUnderDocs.replace(/\.mdx?$/, '');
        if (slug === cleanPath || kClean === cleanPath || k === cleanPath) {
          return { key: k, page: pageEntry };
        }
      }
      return null;
    }

    while ((m = linkRegex.exec(content)) !== null) {
      const label = m[1];
      const url = m[2].trim();

      if (!url.startsWith('https://voxlang.org/')) {
        continue;
      }

      const cleanPath = url
        .slice('https://voxlang.org/'.length)
        .split('#')[0]
        .split('?')[0]
        .replace(/^\/+|\/+$/g, '');

      // 1. Must exist in dist
      const targetExists =
        cleanPath === ''
          ? existsSync(join(DIST_DIR, 'index.html'))
          : existsSync(join(DIST_DIR, cleanPath, 'index.html')) || existsSync(join(DIST_DIR, cleanPath));

      if (!targetExists) {
        missingUrls.push(`${label} (${url}) -> target '${cleanPath}' missing in dist`);
      }

      // 2. Must resolve in ledger
      const resolved = resolveUrlToLedger(cleanPath);
      if (!resolved) {
        missingUrls.push(`${label} (${url}) -> not found in ledger.v1.json`);
      } else {
        const { page } = resolved;
        // 3. Must be verified
        if (page.state !== 'verified') {
          nonVerifiedUrls.push(`${label} (${url}) -> state: '${page.state}' (must be 'verified')`);
        }
        // 4. Must not be Internals
        const policy = statusPolicy(page.status);
        if (policy.internals || policy.noindex || page.category === 'Internals') {
          internalsUrls.push(`${label} (${url}) -> status: '${page.status}', category: '${page.category}'`);
        }
      }
    }

    expect(missingUrls, `Missing URLs in curated llms.txt: ${missingUrls.join('; ')}`).toHaveLength(0);
    expect(nonVerifiedUrls, `Non-verified URLs in curated llms.txt: ${nonVerifiedUrls.join('; ')}`).toHaveLength(0);
    expect(internalsUrls, `Internals URLs in curated llms.txt: ${internalsUrls.join('; ')}`).toHaveLength(0);
  });
});
