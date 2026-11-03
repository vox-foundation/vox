import { test, expect } from '@playwright/test';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { readDist, listDistHtml, DIST_DIR } from '../lib/dist';

const REPO_ROOT = resolve(DIST_DIR, '..', '..');

test.describe('Build Output: Link Rewriting (LINKS-01, LINKS-02)', () => {
  test('tutorials/tut-getting-started has clean installation links and no .md href', () => {
    const html = readDist('tutorials/tut-getting-started');
    expect(html).toContain('href="/reference/installation/"');
    expect(html).toContain('href="/reference/installation/#quick-install-from-source"');

    // Find all relative hrefs inside this page
    const hrefRegex = /href=["']([^"']+)["']/g;
    let match;
    const mdHrefs: string[] = [];
    while ((match = hrefRegex.exec(html)) !== null) {
      const href = match[1];
      if (
        !href.startsWith('http://') &&
        !href.startsWith('https://') &&
        !href.startsWith('//') &&
        !href.startsWith('/') &&
        !href.startsWith('#') &&
        !href.startsWith('mailto:') &&
        !href.startsWith('javascript:')
      ) {
        if (href.endsWith('.md') || href.includes('.md#') || href.includes('.md?')) {
          mdHrefs.push(href);
        }
      }
    }
    expect(mdHrefs, `Found relative .md hrefs in tut-getting-started: ${mdHrefs.join(', ')}`).toEqual([]);
  });

  test('no built HTML page contains relative href ending in .md or containing .md#', () => {
    const htmlFiles = listDistHtml();
    const offenders: { file: string; href: string }[] = [];
    const hrefRegex = /href=["']([^"']+)["']/g;

    for (const relFile of htmlFiles) {
      // Skip 404 or non-content if any
      const filePath = join(DIST_DIR, relFile);
      const html = readFileSync(filePath, 'utf8');

      let match;
      while ((match = hrefRegex.exec(html)) !== null) {
        const href = match[1];
        // Only inspect relative links (not external http/https, protocol-relative, absolute /, mailto:, javascript:)
        if (
          !href.startsWith('http://') &&
          !href.startsWith('https://') &&
          !href.startsWith('//') &&
          !href.startsWith('/') &&
          !href.startsWith('#') &&
          !href.startsWith('mailto:') &&
          !href.startsWith('javascript:')
        ) {
          if (href.endsWith('.md') || href.includes('.md#') || href.includes('.md?')) {
            offenders.push({ file: relFile, href });
          }
        }
      }
    }

    if (offenders.length > 0) {
      const summary = offenders
        .slice(0, 20)
        .map((o) => `${o.file} -> ${o.href}`)
        .join('\n');
      expect(offenders.length, `Found ${offenders.length} relative .md hrefs:\n${summary}`).toBe(0);
    }
    expect(offenders.length).toBe(0);
  });

  test('no relative href climbs above the site root', () => {
    const htmlFiles = listDistHtml();
    const climbingOffenders: { file: string; href: string }[] = [];
    const hrefRegex = /href=["']([^"']+)["']/g;

    for (const relFile of htmlFiles) {
      const filePath = join(DIST_DIR, relFile);
      const fileDir = dirname(filePath);
      const html = readFileSync(filePath, 'utf8');

      let match;
      while ((match = hrefRegex.exec(html)) !== null) {
        const href = match[1];
        if (
          !href.startsWith('http://') &&
          !href.startsWith('https://') &&
          !href.startsWith('//') &&
          !href.startsWith('/') &&
          !href.startsWith('#') &&
          !href.startsWith('mailto:') &&
          !href.startsWith('javascript:')
        ) {
          // Clean fragment and query
          const cleanHref = href.split('#')[0].split('?')[0];
          if (!cleanHref) continue;

          const targetAbs = resolve(fileDir, cleanHref);
          // If targetAbs resolves outside DIST_DIR, it climbs out of the site root
          if (!targetAbs.startsWith(DIST_DIR)) {
            climbingOffenders.push({ file: relFile, href });
          }
        }
      }
    }

    if (climbingOffenders.length > 0) {
      const summary = climbingOffenders
        .slice(0, 20)
        .map((o) => `${o.file} -> ${o.href}`)
        .join('\n');
      expect(climbingOffenders.length, `Found ${climbingOffenders.length} escaping hrefs:\n${summary}`).toBe(0);
    }
    expect(climbingOffenders.length).toBe(0);
  });

  test('every GitHub blob/tree link in built HTML names a path that exists in repo working tree', () => {
    const htmlFiles = listDistHtml();
    const missingTargets: { file: string; href: string; repoPath: string }[] = [];
    const hrefRegex = /href=["']([^"']+)["']/g;
    const GITHUB_PREFIX = 'https://github.com/vox-foundation/vox/';
    let githubLinkCount = 0;

    for (const relFile of htmlFiles) {
      const filePath = join(DIST_DIR, relFile);
      const html = readFileSync(filePath, 'utf8');

      let match;
      while ((match = hrefRegex.exec(html)) !== null) {
        const href = match[1];
        if (href.startsWith(GITHUB_PREFIX)) {
          const rest = href.slice(GITHUB_PREFIX.length);
          if (rest.startsWith('blob/main/') || rest.startsWith('tree/main/')) {
            githubLinkCount++;
            const repoPathWithFrag = rest.startsWith('blob/main/')
              ? rest.slice('blob/main/'.length)
              : rest.slice('tree/main/'.length);

            // Strip fragment and query parameters
            const cleanRepoPath = repoPathWithFrag.split('#')[0].split('?')[0];
            const absRepoTarget = join(REPO_ROOT, decodeURIComponent(cleanRepoPath));

            if (!existsSync(absRepoTarget)) {
              missingTargets.push({ file: relFile, href, repoPath: cleanRepoPath });
            }
          }
        }
      }
    }

    console.log(`[links.spec.ts] Checked ${githubLinkCount} GitHub blob/tree links across ${htmlFiles.length} HTML files.`);

    if (missingTargets.length > 0) {
      const summary = missingTargets
        .slice(0, 20)
        .map((m) => `${m.file} -> ${m.href} (missing: ${m.repoPath})`)
        .join('\n');
      expect(missingTargets.length, `Found ${missingTargets.length} GitHub links to non-existent repo paths:\n${summary}`).toBe(0);
    }
    expect(missingTargets.length).toBe(0);
  });
});
