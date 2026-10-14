import { test, expect } from '@playwright/test';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { gunzipSync } from 'node:zlib';
import { readDist, DIST_DIR } from '../lib/dist';

test.describe('Build Output: Archive Exclusion & Retirement (LINKS-02, P20-D8)', () => {
  test('dist/archive/ directory does not exist', () => {
    const archivePath = join(DIST_DIR, 'archive');
    expect(existsSync(archivePath)).toBe(false);
  });

  test('no sitemap XML file references /archive/', () => {
    const files = readdirSync(DIST_DIR);
    const sitemapFiles = files.filter((f) => f.startsWith('sitemap') && (f.endsWith('.xml') || f.endsWith('.xml.gz')));

    expect(sitemapFiles.length).toBeGreaterThan(0);

    for (const file of sitemapFiles) {
      let content: string;
      const fullPath = join(DIST_DIR, file);
      if (file.endsWith('.gz')) {
        content = gunzipSync(readFileSync(fullPath)).toString('utf8');
      } else {
        content = readFileSync(fullPath, 'utf8');
      }
      expect(content).not.toContain('/archive/');
    }
  });

  test('canonical reader pages exist', () => {
    expect(() => readDist('tutorials/tut-getting-started')).not.toThrow();
    expect(() => readDist('reference/cli')).not.toThrow();
  });

  test('dist/retired/index.html exists and is unindexed and not in pagefind', () => {
    const retiredHtml = readDist('retired');
    expect(retiredHtml).toContain('<meta name="robots" content="noindex"');
    expect(retiredHtml).toContain('https://github.com/vox-foundation/vox/tree/main/docs/src/archive');
    expect(retiredHtml).not.toContain('data-pagefind-body');
  });

  test('dist/_redirects contains archive redirect and preserves existing rules', () => {
    const distRedirectsPath = join(DIST_DIR, '_redirects');
    expect(existsSync(distRedirectsPath)).toBe(true);

    const content = readFileSync(distRedirectsPath, 'utf8');
    const matches = content.match(/\/archive\/\*\s+\/retired\/\s+301/g);
    expect(matches).not.toBeNull();
    expect(matches?.length).toBe(1);

    // Ensure earlier redirect lines from public/_redirects are present
    expect(content).toContain('/tutorials/tut-getting-started.html   /tutorials/tut-getting-started/  301');
    expect(content).toContain('/reference/cli.html                   /reference/cli/                   301');
  });
});
