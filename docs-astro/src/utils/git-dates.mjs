import { execFileSync } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

export const DEFAULT_IGNORE_SUBJECTS = [
  /^chore\(ssot\): auto-regenerate/i,
  /^style(\([^)]*\))?:/i,
  /^chore(\([^)]*\))?: (cargo )?(fmt|format|rustfmt)\b/i,
];

/**
 * Reads `.git-blame-ignore-revs` from the repo root if it exists.
 * Returns a Set of full 40-character SHAs.
 */
export function readIgnoreRevs(repoRoot = gitRoot()) {
  const file = join(repoRoot, '.git-blame-ignore-revs');
  if (!existsSync(file)) return new Set();

  const content = readFileSync(file, 'utf8');
  const shas = new Set();
  for (const line of content.split('\n')) {
    const trimmed = line.trim();
    if (trimmed && !trimmed.startsWith('#')) {
      shas.add(trimmed);
    }
  }
  return shas;
}

/**
 * Parses git log output formatted with:
 * `git log --format=C|%H|%cI|%s --name-status -M`
 *
 * Traverses history newest-to-oldest, tracks renames via an alias map,
 * excludes mechanical/ignored/bulk commits, and returns a Map from repo-relative
 * path to the ISO date of its newest substantive commit.
 */
export function parseGitLog(
  text,
  {
    ignoreShas = new Set(),
    ignoreSubjects = DEFAULT_IGNORE_SUBJECTS,
    bulkThreshold = 100,
  } = {}
) {
  const substantiveDates = new Map();
  const fallbackDates = new Map();
  const deletedPaths = new Set();
  const aliasMap = new Map(); // oldPath -> canonicalNewestPath

  function resolvePath(p) {
    let current = p;
    while (aliasMap.has(current)) {
      current = aliasMap.get(current);
    }
    return current;
  }

  const lines = text.split('\n');
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    if (!line.startsWith('C|')) {
      i++;
      continue;
    }

    const parts = line.slice(2).split('|');
    const sha = parts[0]?.trim();
    const date = parts[1]?.trim();
    const subject = parts.slice(2).join('|').trim();

    // Collect file status lines for this commit
    i++;
    const fileEntries = [];
    while (i < lines.length && !lines[i].startsWith('C|')) {
      const fLine = lines[i].trim();
      if (fLine) {
        fileEntries.push(fLine);
      }
      i++;
    }

    const isIgnoredSha = sha && ignoreShas.has(sha);
    const isIgnoredSubject = ignoreSubjects.some((re) => re.test(subject));
    const isBulk = fileEntries.length > bulkThreshold;
    const isIgnored = isIgnoredSha || isIgnoredSubject || isBulk;

    for (const fEntry of fileEntries) {
      const cols = fEntry.split('\t');
      const statusRaw = cols[0];
      const statusCode = statusRaw ? statusRaw[0] : '';

      if (statusCode === 'R') {
        // Rename: cols = [Rxxx, oldPath, newPath]
        const oldPath = cols[1];
        const newPath = cols[2];
        if (oldPath && newPath) {
          const canonicalNew = resolvePath(newPath);
          aliasMap.set(oldPath, canonicalNew);

          if (!isIgnored) {
            if (!substantiveDates.has(canonicalNew)) {
              substantiveDates.set(canonicalNew, date);
            }
          }
          if (!fallbackDates.has(canonicalNew)) {
            fallbackDates.set(canonicalNew, date);
          }
        }
      } else {
        const filePath = cols[1];
        if (!filePath) continue;

        const canonicalPath = resolvePath(filePath);

        if (statusCode === 'D') {
          // If the first time we see this path is a deletion, mark deleted
          if (!substantiveDates.has(canonicalPath) && !fallbackDates.has(canonicalPath)) {
            deletedPaths.add(canonicalPath);
          }
        } else {
          // Added or Modified
          if (!isIgnored) {
            if (!substantiveDates.has(canonicalPath)) {
              substantiveDates.set(canonicalPath, date);
            }
          }
          if (!fallbackDates.has(canonicalPath)) {
            fallbackDates.set(canonicalPath, date);
          }
        }
      }
    }
  }

  const result = new Map();
  // Populate result prioritizing substantive dates, falling back to ignored dates
  for (const [path, date] of fallbackDates.entries()) {
    if (deletedPaths.has(path)) continue;
    if (substantiveDates.has(path)) {
      result.set(path, substantiveDates.get(path));
    } else {
      result.set(path, date);
    }
  }

  return result;
}

/**
 * Returns a Map from repo-relative path (e.g. `docs/src/...`) to ISO committer date.
 */
export function getGitDates(repoRoot = gitRoot(), { paths = ['docs/src'] } = {}) {
  try {
    const ignoreShas = readIgnoreRevs(repoRoot);
    const out = execFileSync(
      'git',
      [
        '-c',
        'core.quotepath=off',
        'log',
        '--format=C|%H|%cI|%s',
        '--name-status',
        '-M',
        '--',
        ...paths,
      ],
      { cwd: repoRoot, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 }
    );

    const dates = parseGitLog(out, { ignoreShas });
    console.warn(`[git-dates] loaded ${dates.size} date entries for ${repoRoot}`);
    return dates;
  } catch (err) {
    console.warn(`[git-dates] failed to read git dates: ${err.message}`);
    return new Map();
  }
}

export function gitRoot() {
  return execFileSync('git', ['rev-parse', '--show-toplevel'], {
    encoding: 'utf8',
  }).trim();
}
