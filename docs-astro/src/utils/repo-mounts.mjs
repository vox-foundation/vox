/**
 * Repo Markdown outside docs/src that the site renders under `/repo/<route>/`
 * (P20-D9). The mount set is every `.md` file a docs page links to, plus the
 * contract's `always_mount` list, minus `never_mount_prefixes`. Files are
 * mounted from the repo, never copied into docs/src: setup-content.mjs links
 * (or wraps) them into the content mirror, remark-doc-links routes links to
 * them, and routeData points their edit link and date at the repo file.
 */
import { existsSync, readFileSync, realpathSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import matter from 'gray-matter';
import { listDocPages } from './page-index.mjs';

export const MOUNT_CONTRACT = 'contracts/documentation/site-mounted-repo-docs.v1.json';
/** Site route prefix (and mirror subdirectory) for mounted repo files. */
export const MOUNT_DIR = 'repo';

const EMPTY_CONTRACT = Object.freeze({ always_mount: [], never_mount_prefixes: [], titles: {} });

/** `AGENTS.md` -> `agents-md`, `crates/vox-cli/README.md` -> `crates-vox-cli-readme-md`. */
export function mountRoute(repoPath) {
  return repoPath
    .toLowerCase()
    .replace(/[/._]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/** The mount contract at `repoRoot`, or an empty one if the file is absent. */
export function loadMountContract(repoRoot) {
  const file = join(repoRoot, MOUNT_CONTRACT);
  if (!existsSync(file)) return EMPTY_CONTRACT;
  return { ...EMPTY_CONTRACT, ...JSON.parse(readFileSync(file, 'utf8')) };
}

/** Remove fenced code blocks and inline code spans so example links are not checked. */
export function stripCode(markdown) {
  const out = [];
  let fence = null;
  for (const line of markdown.split('\n')) {
    const m = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (fence) {
      // A closing fence carries no info string (CommonMark), so ```bash inside ```markdown does not close it.
      if (m && m[1][0] === fence[0] && m[1].length >= fence.length && !line.slice(line.indexOf(m[1]) + m[1].length).trim()) fence = null;
      out.push('');
      continue;
    }
    if (m) {
      fence = m[1];
      out.push('');
      continue;
    }
    out.push(line.replace(/(`+)[\s\S]*?\1/g, ''));
  }
  return out.join('\n');
}

/**
 * Relative link targets in `markdown`: inline `](target "title")` and reference
 * definitions `[label]: target` (footnotes `[^label]:` are not links), with code
 * stripped. URLs, anchors, absolute paths, autolinks and mailto are skipped.
 * Returned targets have the `#fragment` and a trailing `:N` / `:N-M` line suffix
 * removed and are URI-decoded.
 */
export function relativeLinkTargets(markdown) {
  const text = stripCode(markdown);
  const raw = [];
  for (const m of text.matchAll(/\]\(\s*([^)\s]+)(?:\s+"[^"]*")?\s*\)/g)) raw.push(m[1]);
  for (const m of text.matchAll(/^\s{0,3}\[(?!\^)[^\]]+\]:\s*(\S+)/gm)) raw.push(m[1]);
  const targets = [];
  for (const t of raw) {
    if (/^[a-z][a-z0-9+.-]*:/i.test(t) || /^[#/<]/.test(t)) continue;
    let path = t.split('#')[0].replace(/:\d+(-\d+)?$/, '');
    if (!path) continue;
    try {
      path = decodeURI(path);
    } catch {
      // keep the raw path; a malformed escape is still reported as missing
    }
    targets.push(path);
  }
  return targets;
}

function isWithin(parent, child) {
  const rel = relative(parent, child);
  return rel === '' || (!rel.startsWith('..') && !rel.startsWith(sep) && !/^[a-z]:/i.test(rel));
}

const toPosix = (path) => path.split(sep).join('/');

function hasTitle(file) {
  try {
    const { data } = matter(readFileSync(file, 'utf8'));
    return typeof data.title === 'string' && data.title.trim() !== '';
  } catch {
    return false;
  }
}

/**
 * Every repo Markdown file to mount, sorted by repo path:
 * `{ repoPath, route, hasTitle }`.
 *
 * @param {{ repoRoot: string, docsSrc?: string, contract?: object }} options
 */
export function discoverMounts({ repoRoot, docsSrc, contract }) {
  const root = realpathSync(repoRoot);
  const src = docsSrc ? realpathSync(docsSrc) : join(root, 'docs', 'src');
  const { always_mount = [], never_mount_prefixes = [] } = contract ?? loadMountContract(root);

  const paths = new Set();
  const consider = (abs) => {
    if (!/\.md$/i.test(abs) || !existsSync(abs)) return;
    let real;
    try {
      real = realpathSync(abs);
    } catch {
      return;
    }
    if (!statSync(real).isFile() || !isWithin(root, real) || isWithin(src, real)) return;
    const repoPath = toPosix(relative(root, real));
    if (never_mount_prefixes.some((prefix) => repoPath.startsWith(prefix))) return;
    paths.add(repoPath);
  };

  for (const page of listDocPages(src)) {
    const file = join(src, page.relPath);
    for (const target of relativeLinkTargets(readFileSync(file, 'utf8'))) consider(resolve(dirname(file), target));
  }
  for (const repoPath of always_mount) consider(join(root, repoPath));

  const mounts = [...paths].sort().map((repoPath) => ({
    repoPath,
    route: mountRoute(repoPath),
    hasTitle: hasTitle(join(root, repoPath)),
  }));
  const seen = new Map();
  for (const { repoPath, route } of mounts) {
    if (seen.has(route)) throw new Error(`[repo-mounts] ${seen.get(route)} and ${repoPath} both map to /${MOUNT_DIR}/${route}/`);
    seen.set(route, repoPath);
  }
  return mounts;
}

/** Text of the first `# ` heading outside code, or null. */
export function firstHeading(markdown) {
  const code = stripCode(markdown).split('\n');
  const lines = markdown.split('\n');
  for (let i = 0; i < lines.length; i++) {
    if (code[i] === '' && lines[i] !== '') continue;
    const m = lines[i].match(/^\s{0,3}#\s+(.+?)\s*#*\s*$/);
    if (m) return { text: m[1], line: i };
  }
  return null;
}

/**
 * Page source for a mount whose file has no frontmatter title: a `title`
 * (contract `titles`, else the first `# ` heading, else the repo path),
 * `mounted_from` and `status`, then the body without its first `# ` heading.
 * Strings are JSON-quoted, which is valid YAML, so a heading cannot inject
 * frontmatter keys.
 */
export function wrapperSource(repoPath, raw, titles = {}) {
  const { data, content } = matter(raw);
  const heading = firstHeading(content);
  const title =
    titles[repoPath] ??
    (typeof data.title === 'string' && data.title.trim() ? data.title : null) ??
    heading?.text ??
    repoPath;
  let body = content;
  if (heading) {
    const lines = content.split('\n');
    lines.splice(heading.line, 1);
    body = lines.join('\n');
  }
  const front = [`title: ${JSON.stringify(title)}`];
  if (typeof data.description === 'string') front.push(`description: ${JSON.stringify(data.description)}`);
  front.push(`mounted_from: ${JSON.stringify(repoPath)}`, `status: ${JSON.stringify('current')}`);
  return `---\n${front.join('\n')}\n---\n${body.replace(/^\n+/, '\n')}`;
}
