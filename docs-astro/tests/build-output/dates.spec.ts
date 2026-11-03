import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readDist, listDistHtml } from '../lib/dist';
import { getGitDates } from '../../src/utils/git-dates.mjs';

test.describe('Build Output: Page Dates (DEPLOY-04)', () => {
  test('at least 90% of docs pages contain <time datetime="', () => {
    const allHtml = listDistHtml();
    const candidatePages = allHtml.filter((file) => {
      // Exclude special, non-doc, or splash pages
      if (file === 'index.html' || file === '404.html') return false;
      if (file.startsWith('pagefind/') || file.startsWith('_llms-txt/') || file.startsWith('retired/')) return false;
      return file.endsWith('/index.html');
    });

    expect(candidatePages.length).toBeGreaterThan(100);

    let withTimeTag = 0;
    for (const page of candidatePages) {
      const html = readDist(page.replace(/\/index\.html$/, ''));
      if (html.includes('<time datetime="')) {
        withTimeTag++;
      }
    }

    const percentage = (withTimeTag / candidatePages.length) * 100;
    console.log(`Page date coverage: ${withTimeTag}/${candidatePages.length} (${percentage.toFixed(1)}%)`);
    expect(percentage).toBeGreaterThanOrEqual(90);
  });

  test('calendar day matches git-dates for key reference/tutorial pages', () => {
    const gitDates = getGitDates();

    const targets = [
      { route: 'tutorials/tut-getting-started', repoPath: 'docs/src/tutorials/tut-getting-started.md' },
      { route: 'reference/installation', repoPath: 'docs/src/reference/installation.md' },
    ];

    for (const { route, repoPath } of targets) {
      const gitIso = gitDates.get(repoPath);
      expect(gitIso).toBeTruthy();
      const expectedDay = new Date(gitIso!).toISOString().slice(0, 10);

      const html = readDist(route);
      const match = html.match(/<time datetime="([^"]+)"/);
      expect(match).toBeTruthy();
      const renderedDay = new Date(match![1]).toISOString().slice(0, 10);

      expect(renderedDay).toBe(expectedDay);
    }
  });

  test('mechanical commits in ignore-revs do not drive page dates', () => {
    // Pick the first file touched by 4e98a0f87ee3cbae99925d6f14843e8fc00af192
    // whose newest commit in git log is that SHA, and verify the rendered date is earlier.
    const targetSha = '4e98a0f87ee3cbae99925d6f14843e8fc00af192';
    let sampleFile: string | null = null;
    let ignoredCommitDate: string | null = null;

    try {
      const out = execFileSync('git', ['show', '--format=%cI', '--name-only', targetSha], {
        encoding: 'utf8',
      });
      const lines = out.trim().split('\n');
      ignoredCommitDate = lines[0];
      const files = lines.slice(1).filter((f) => f.startsWith('docs/src/') && f.endsWith('.md'));

      for (const f of files) {
        const newestSha = execFileSync('git', ['log', '-1', '--format=%H', '--', f], { encoding: 'utf8' }).trim();
        if (newestSha === targetSha) {
          sampleFile = f;
          break;
        }
      }
    } catch {
      // If git commands fail (e.g. shallow clone), skip gracefully
    }

    if (!sampleFile || !ignoredCommitDate) {
      test.skip(true, 'No docs page found where 4e98a0f87 was the latest commit');
      return;
    }

    const route = sampleFile.replace(/^docs\/src\//, '').replace(/\.md$/, '');
    const html = readDist(route);
    const match = html.match(/<time datetime="([^"]+)"/);
    expect(match).toBeTruthy();

    const renderedTime = new Date(match![1]).getTime();
    const ignoredTime = new Date(ignoredCommitDate).getTime();

    // Rendered date must be strictly earlier than the mechanical commit date
    expect(renderedTime).toBeLessThan(ignoredTime);
  });
});
