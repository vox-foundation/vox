import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Commit subjects that never represent a reviewed content change: the
 * `ssot-autoregen` CI bot and formatting-only commits.
 */
export const DEFAULT_IGNORE_SUBJECTS = [
  /^chore\(ssot\): auto-regenerate/,
  /^style(\([^)]*\))?:/,
  /^chore(\([^)]*\))?: (cargo )?(fmt|format|rustfmt)\b/,
];

const TOMBSTONE = Symbol('tombstone');

/**
 * Parse `git log --format=C|%H|%cI|%s --name-status -M` output (newest first)
 * into a Map of current repo-relative path -> ISO date of the newest commit
 * that is not mechanical.
 *
 * A commit is mechanical when its full SHA is in `ignoreShas`, its subject
 * matches one of `ignoreSubjects`, or it lists more than `bulkThreshold`
 * files. A path touched only by mechanical commits falls back to the newest
 * of those. Renames are followed: walking back in time, an older commit's
 * `old` path is credited to the newest path it was renamed to. Paths whose
 * newest record is a delete are omitted; an `A`/`D` record ends a path's
 * history, so a previous file that lived at the same path is not credited.
 */
export function parseGitLog(
  text,
  { ignoreShas = new Set(), ignoreSubjects = DEFAULT_IGNORE_SUBJECTS, bulkThreshold = 100 } = {},
) {
  const commits = [];
  let current = null;
  for (const line of text.split('\n')) {
    if (line.startsWith('C|')) {
      const [sha, date, ...subject] = line.slice(2).split('|');
      current = { sha: sha.trim(), date: date.trim(), subject: subject.join('|'), files: [] };
      commits.push(current);
    } else if (current && line.includes('\t')) {
      const [status, ...paths] = line.split('\t');
      current.files.push({ status: status.trim(), paths });
    }
  }

  const alias = new Map();
  const state = new Map();
  const resolve = (path) => (alias.has(path) ? alias.get(path) : path);

  for (const commit of commits) {
    const ignored =
      ignoreShas.has(commit.sha) ||
      ignoreSubjects.some((re) => re.test(commit.subject)) ||
      commit.files.length > bulkThreshold;

    for (const { status, paths } of commit.files) {
      const kind = status[0];
      const raw = kind === 'R' || kind === 'C' ? paths[1] : paths[0];
      const target = resolve(raw);
      if (target === TOMBSTONE) continue;

      let entry = state.get(target);
      if (!entry) {
        entry = { deleted: kind === 'D', date: null, fallback: null };
        state.set(target, entry);
      }
      if (!entry.deleted) {
        if (!ignored && !entry.date) entry.date = commit.date;
        if (ignored && !entry.fallback) entry.fallback = commit.date;
      }

      if (kind === 'R') alias.set(paths[0], target);
      if (kind === 'A' || kind === 'D') alias.set(raw, TOMBSTONE);
    }
  }

  const dates = new Map();
  for (const [path, entry] of state) {
    if (entry.deleted) continue;
    const date = entry.date ?? entry.fallback;
    if (date) dates.set(path, date);
  }
  return dates;
}

/** Full SHAs listed in the repo-root `.git-blame-ignore-revs`, if present. */
export function readIgnoreRevs(repoRoot) {
  const file = join(repoRoot, '.git-blame-ignore-revs');
  if (!existsSync(file)) return new Set();
  return new Set(
    readFileSync(file, 'utf8')
      .split('\n')
      .map((line) => line.replace(/#.*/, '').trim())
      .filter(Boolean),
  );
}

const cache = new Map();

/**
 * Last-commit date for every doc under `docs/src`, keyed by repo-relative
 * path (`docs/src/tutorials/tut-getting-started.md`).
 *
 * Starlight's own git lookup runs `git log` against `src/content/docs`, which
 * is a gitignored symlink to `docs/src`, so it finds no history and no page
 * gets a date. This reads `docs/src` from the repo root instead, in one
 * subprocess for the whole tree.
 *
 * The repo root is resolved with `git rev-parse --show-toplevel` rather than
 * from `import.meta.url`. At build time this module is bundled into
 * `dist/.prerender/chunks/`, so a path computed relative to the module
 * resolves to `docs-astro/`, not the repo root -- and `git log -- docs/src`
 * from there matches nothing and exits 0, yielding an empty map with no
 * error. Asking git is correct from any working directory inside the repo.
 */
export function getGitDates(repoRoot, { paths = ['docs/src'] } = {}) {
  try {
    const root = repoRoot ?? gitRoot();
    const key = `${root}\0${paths.join('\0')}`;
    if (cache.has(key)) return cache.get(key);

    const out = execFileSync(
      'git',
      ['-c', 'core.quotepath=off', 'log', '--format=C|%H|%cI|%s', '--name-status', '-M', '--', ...paths],
      { cwd: root, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 },
    );
    const dates = parseGitLog(out, { ignoreShas: readIgnoreRevs(root) });
    console.warn(`[git-dates] ${dates.size} dated paths from git log in ${root}`);
    cache.set(key, dates);
    return dates;
  } catch (err) {
    console.warn(`[git-dates] git log unavailable, no page dates: ${err.message}`);
    return new Map();
  }
}

function gitRoot() {
  return execFileSync('git', ['rev-parse', '--show-toplevel'], {
    encoding: 'utf8',
  }).trim();
}
