import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { resolve, join, dirname, relative, isAbsolute, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import matter from 'gray-matter';

function findRepoRoot() {
  try {
    return execFileSync('git', ['rev-parse', '--show-toplevel'], {
      encoding: 'utf8',
    }).trim();
  } catch {
    const thisDir = dirname(fileURLToPath(import.meta.url));
    return resolve(thisDir, '../../..');
  }
}

export const DEFAULT_REPO_ROOT = findRepoRoot();
export const DEFAULT_DOCS_SRC = join(DEFAULT_REPO_ROOT, 'docs/src');
export const DEFAULT_CONTRACT_PATH = join(
  DEFAULT_REPO_ROOT,
  'contracts/documentation/site-mounted-repo-docs.v1.json'
);

/**
 * Derives a clean Starlight URL slug for an out-of-tree repo markdown file.
 * Lower-cases, replaces '/', '.', and '_' with '-', and collapses repeats.
 * E.g. 'AGENTS.md' -> 'agents-md', 'crates/vox-cli/README.md' -> 'crates-vox-cli-readme-md'.
 */
export function mountRoute(repoPath) {
  return repoPath
    .toLowerCase()
    .replace(/[/._]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/**
 * Loads the mount contract from disk or returns default if not found.
 */
export function loadMountContract(contractPath = DEFAULT_CONTRACT_PATH) {
  if (existsSync(contractPath)) {
    return JSON.parse(readFileSync(contractPath, 'utf8'));
  }
  return {
    'x-vox-version': 1,
    always_mount: ['AGENTS.md', 'CLAUDE.md', 'GEMINI.md'],
    never_mount_prefixes: ['docs/superpowers/'],
    titles: {},
  };
}

/**
 * Extracts candidate link URLs from markdown content, stripping code blocks.
 */
function extractLinks(markdown) {
  // Strip fenced code blocks
  const withoutFences = markdown.replace(/```[\s\S]*?```|~~~[\s\S]*?~~~/g, '');
  // Strip inline code
  const withoutCode = withoutFences.replace(/`[^`]*`/g, '');

  const urls = [];
  // Markdown links: [text](url)
  const mdLinkRegex = /\[(?:[^\]]*)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g;
  let match;
  while ((match = mdLinkRegex.exec(withoutCode)) !== null) {
    urls.push(match[1]);
  }

  // HTML links: <a href="url">
  const htmlLinkRegex = /<a\s+[^>]*?href=["']([^"']+)["'][^>]*>/gi;
  while ((match = htmlLinkRegex.exec(withoutCode)) !== null) {
    urls.push(match[1]);
  }

  return urls;
}

/**
 * Recursively walks directory to list markdown files.
 */
function walkDocs(dir, fileList = []) {
  if (!existsSync(dir)) return fileList;
  const entries = readdirSync(dir);
  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      if (entry === 'archive' || entry === '.well-known') continue;
      walkDocs(fullPath, fileList);
    } else if (entry.endsWith('.md') || entry.endsWith('.mdx')) {
      fileList.push(fullPath);
    }
  }
  return fileList;
}

/**
 * Discovers out-of-tree repository Markdown files linked from docs/src
 * and unions them with contract.always_mount.
 */
export function discoverMounts(options = {}) {
  const repoRoot = options.repoRoot || DEFAULT_REPO_ROOT;
  const docsSrc = options.docsSrc || DEFAULT_DOCS_SRC;
  const contract = options.contract || loadMountContract(options.contractPath || DEFAULT_CONTRACT_PATH);

  const neverPrefixes = contract.never_mount_prefixes || ['docs/superpowers/'];
  const mountPaths = new Set();

  // 1. Scan docs/src for out-of-tree markdown links
  const docsFiles = walkDocs(docsSrc);
  for (const docFile of docsFiles) {
    const content = readFileSync(docFile, 'utf8');
    const links = extractLinks(content);
    const docDir = dirname(docFile);

    for (const link of links) {
      if (
        link.startsWith('http://') ||
        link.startsWith('https://') ||
        link.startsWith('//') ||
        link.startsWith('/') ||
        link.startsWith('#') ||
        link.startsWith('mailto:') ||
        link.startsWith('javascript:')
      ) {
        continue;
      }

      const cleanUrl = link.split('#')[0].split('?')[0];
      if (!cleanUrl) continue;

      let decoded;
      try {
        decoded = decodeURI(cleanUrl);
      } catch {
        decoded = cleanUrl;
      }

      const absTarget = resolve(docDir, decoded);
      if (!existsSync(absTarget)) continue;

      const ext = extname(absTarget).toLowerCase();
      if (ext !== '.md' && ext !== '.mdx') continue;

      const relToDocs = relative(docsSrc, absTarget).replace(/\\/g, '/');
      const isInsideDocs = !relToDocs.startsWith('..') && !isAbsolute(relToDocs);
      if (isInsideDocs) continue;

      const relToRepo = relative(repoRoot, absTarget).replace(/\\/g, '/');
      const isInsideRepo = !relToRepo.startsWith('..') && !isAbsolute(relToRepo);
      if (!isInsideRepo) continue;

      const isIgnored = neverPrefixes.some((p) => relToRepo === p || relToRepo.startsWith(p));
      if (isIgnored) continue;

      mountPaths.add(relToRepo);
    }
  }

  // 2. Union with always_mount
  if (Array.isArray(contract.always_mount)) {
    for (const p of contract.always_mount) {
      const abs = join(repoRoot, p);
      if (existsSync(abs)) {
        mountPaths.add(p.replace(/\\/g, '/'));
      }
    }
  }

  // 3. Inspect each file for frontmatter title
  const results = [];
  const sortedPaths = Array.from(mountPaths).sort();

  for (const repoPath of sortedPaths) {
    const absPath = join(repoRoot, repoPath);
    let hasTitle = false;
    try {
      const raw = readFileSync(absPath, 'utf8');
      const { data } = matter(raw);
      hasTitle = Boolean(data && typeof data.title === 'string' && data.title.trim().length > 0);
    } catch {
      hasTitle = false;
    }

    results.push({
      repoPath,
      route: mountRoute(repoPath),
      hasTitle,
    });
  }

  return results;
}
