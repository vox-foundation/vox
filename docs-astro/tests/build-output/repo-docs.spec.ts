import { test, expect } from '@playwright/test';
import { existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { readDist, DIST_DIR } from '../lib/dist';
import { getGitDates } from '../../src/utils/git-dates.mjs';

test.describe('Build Output: Mounted Repo Docs (LINKS-02, P20-D9)', () => {
  test('dist/repo/agents-md/index.html exists and contains root AGENTS.md content', () => {
    const agentsMdHtml = readDist('repo/agents-md');
    expect(agentsMdHtml).toContain('Agents Policy');
  });

  test('dist/agents/index.html still exists (the scoped docs page)', () => {
    expect(() => readDist('agents')).not.toThrow();
  });

  test('no dist/repo/* page has a route starting with docs-superpowers', () => {
    const repoDistDir = join(DIST_DIR, 'repo');
    expect(existsSync(repoDistDir)).toBe(true);

    const entries = readdirSync(repoDistDir);
    for (const entry of entries) {
      expect(entry.startsWith('docs-superpowers')).toBe(false);
    }
  });

  test('/repo/agents-md/ edit link points to true repo file on GitHub', () => {
    const html = readDist('repo/agents-md');
    expect(html).toContain('https://github.com/vox-foundation/vox/edit/main/AGENTS.md');
  });

  test('/repo/agents-md/ has <time datetime> matching git-dates of AGENTS.md', () => {
    const html = readDist('repo/agents-md');
    const match = html.match(/<time datetime="([^"]+)"/);
    expect(match).toBeTruthy();

    const gitDates = getGitDates(undefined, { paths: ['AGENTS.md'] });
    const gitIso = gitDates.get('AGENTS.md');
    expect(gitIso).toBeTruthy();

    const expectedDay = new Date(gitIso!).toISOString().slice(0, 10);
    const renderedDay = new Date(match![1]).toISOString().slice(0, 10);
    expect(renderedDay).toBe(expectedDay);
  });

  test('docs page linking to ../../AGENTS.md rewrites href to /repo/agents-md/', () => {
    const html = readDist('contributors/contributor-hub');
    expect(html).toContain('href="/repo/agents-md/"');
  });
});
