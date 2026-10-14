import { existsSync, realpathSync, statSync } from 'node:fs';
import { dirname, extname, isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { docSlug } from '../utils/doc-slug.mjs';
import { discoverMounts, loadMountContract, DEFAULT_REPO_ROOT, DEFAULT_DOCS_SRC } from '../utils/repo-mounts.mjs';

/**
 * Rewrites a relative image URL in an out-of-tree mounted page to GitHub raw URL.
 */
export function rewriteImageUrl(url, fromFile, ctx) {
  if (typeof url !== 'string' || !url.trim()) return url;
  const trimmed = url.trim();

  if (
    trimmed.startsWith('http://') ||
    trimmed.startsWith('https://') ||
    trimmed.startsWith('//') ||
    trimmed.startsWith('/') ||
    trimmed.startsWith('data:') ||
    trimmed.startsWith('#')
  ) {
    return url;
  }

  const hashIndex = trimmed.indexOf('#');
  let beforeHash = hashIndex !== -1 ? trimmed.slice(0, hashIndex) : trimmed;
  const fragment = hashIndex !== -1 ? trimmed.slice(hashIndex) : '';

  const queryIndex = beforeHash.indexOf('?');
  let pathPart = queryIndex !== -1 ? beforeHash.slice(0, queryIndex) : beforeHash;
  const query = queryIndex !== -1 ? beforeHash.slice(queryIndex) : '';

  let decodedPath = pathPart;
  try {
    decodedPath = decodeURI(pathPart);
  } catch {
    // Keep raw
  }

  const fromDir = dirname(fromFile);
  const absTarget = resolve(fromDir, decodedPath);

  const relToRepo = relative(ctx.repoRoot, absTarget).replace(/\\/g, '/');
  if (relToRepo.startsWith('..') || isAbsolute(relToRepo)) {
    throw new Error(
      `remark-doc-links: Image '${url}' in '${fromFile}' resolves outside repository root: '${absTarget}'`
    );
  }

  if (!existsSync(absTarget)) {
    console.warn(`[remark-doc-links] Mounted page image target not found: '${url}' in '${fromFile}'`);
  }

  const cleanRepoRel = relToRepo.replace(/^\/+/, '');
  return `${ctx.repoUrl}/raw/main/${cleanRepoRel}${query}${fragment}`;
}

/**
 * Resolves a Markdown link URL relative to the source file.
 *
 * @param {string} url - Link URL from Markdown AST
 * @param {string} fromFile - Absolute path to the source file
 * @param {{ repoRoot: string, docsSrc: string, repoUrl: string, strict: boolean, mountsMap?: Map<string, string> }} ctx
 * @returns {{ href: string } | null}
 */
export function resolveDocLink(url, fromFile, ctx) {
  if (typeof url !== 'string' || !url.trim()) return null;

  const trimmed = url.trim();

  // Leave untouched: protocol-relative, anchors, site-absolute, or URLs with scheme (http, https, mailto, etc.)
  if (
    trimmed.startsWith('#') ||
    trimmed.startsWith('/') ||
    trimmed.startsWith('//') ||
    /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(trimmed)
  ) {
    return null;
  }

  // Split off #fragment
  const hashIndex = trimmed.indexOf('#');
  let beforeHash = hashIndex !== -1 ? trimmed.slice(0, hashIndex) : trimmed;
  const fragment = hashIndex !== -1 ? trimmed.slice(hashIndex + 1) : null;

  // Split off query string if present
  const queryIndex = beforeHash.indexOf('?');
  let beforeQuery = queryIndex !== -1 ? beforeHash.slice(0, queryIndex) : beforeHash;
  const queryString = queryIndex !== -1 ? beforeHash.slice(queryIndex) : '';

  // Extract trailing line numbers (:N or :N-M)
  let pathPart = beforeQuery;
  let lineFragment = null;
  const lineMatch = pathPart.match(/:(\d+)(?:-(\d+))?$/);
  if (lineMatch) {
    const [, start, end] = lineMatch;
    lineFragment = end ? `L${start}-L${end}` : `L${start}`;
    pathPart = pathPart.slice(0, lineMatch.index);
  }

  // Decode URI components in path
  let decodedPath = pathPart;
  try {
    decodedPath = decodeURI(pathPart);
  } catch {
    // Keep raw pathPart if malformed URI
  }

  const fromDir = dirname(fromFile);
  const absTarget = resolve(fromDir, decodedPath);

  // Check if target escapes repository root
  const relToRepo = relative(ctx.repoRoot, absTarget).replace(/\\/g, '/');
  if (relToRepo.startsWith('..') || isAbsolute(relToRepo)) {
    throw new Error(
      `remark-doc-links: Link '${url}' in '${fromFile}' resolves outside repository root: '${absTarget}'`
    );
  }

  const relToDocs = relative(ctx.docsSrc, absTarget).replace(/\\/g, '/');
  const isInsideDocs = !relToDocs.startsWith('..') && !isAbsolute(relToDocs);
  const isArchive = relToDocs === 'archive' || relToDocs.startsWith('archive/');
  const isSuperpowers = relToRepo === 'docs/superpowers' || relToRepo.startsWith('docs/superpowers/');

  // Check existence
  const exists = existsSync(absTarget);
  if (!exists) {
    if (ctx.strict) {
      throw new Error(
        `remark-doc-links: Target not found: '${url}' in '${fromFile}' (resolved: '${absTarget}')`
      );
    }
    console.warn(`[remark-doc-links] Target not found: '${url}' in '${fromFile}'`);
    if (!isInsideDocs) {
      const cleanRepoRel = relToRepo.replace(/^\/+/, '');
      const finalFragment = lineFragment ? `#${lineFragment}` : fragment ? `#${fragment}` : '';
      if (extname(absTarget)) {
        return { href: `${ctx.repoUrl}/blob/main/${cleanRepoRel}${queryString}${finalFragment}` };
      }
      return { href: `${ctx.repoUrl}/tree/main/${cleanRepoRel}${queryString}${finalFragment}` };
    }
    return null;
  }

  const stat = statSync(absTarget);
  const isDir = stat.isDirectory();

  // In-docs Markdown pages (excluding archive/)
  if (isInsideDocs && !isArchive && !isSuperpowers) {
    const ext = extname(absTarget).toLowerCase();
    if (ext === '.md' || ext === '.mdx') {
      const slug = docSlug(relToDocs);
      let href = slug ? `/${slug}/` : '/';
      if (queryString) href += queryString;
      if (fragment) href += `#${fragment}`;
      return { href };
    }
  }

  const cleanRepoRel = relToRepo.replace(/^\/+/, '');

  // Out-of-tree Markdown pages that are mounted -> /repo/<route>/
  if (!isInsideDocs && !isSuperpowers && ctx.mountsMap && ctx.mountsMap.has(cleanRepoRel)) {
    const route = ctx.mountsMap.get(cleanRepoRel);
    let href = `/repo/${route}/`;
    if (queryString) href += queryString;
    if (fragment) href += `#${fragment}`;
    return { href };
  }

  // Non-docs, directories, archive, or non-Markdown files -> GitHub
  const finalFragment = lineFragment
    ? `#${lineFragment}`
    : fragment
      ? `#${fragment}`
      : '';

  if (isDir) {
    const href = `${ctx.repoUrl}/tree/main/${cleanRepoRel}${queryString}${finalFragment}`;
    return { href };
  }

  const href = `${ctx.repoUrl}/blob/main/${cleanRepoRel}${queryString}${finalFragment}`;
  return { href };
}

/**
 * Recursive AST walker for specified node types.
 */
function visit(tree, types, fn) {
  const typeSet = Array.isArray(types) ? new Set(types) : new Set([types]);
  function walk(node) {
    if (typeSet.has(node.type)) fn(node);
    if (Array.isArray(node.children)) {
      for (const child of node.children) walk(child);
    }
  }
  walk(tree);
}

/**
 * Remark plugin to rewrite documentation links.
 */
export function remarkDocLinks(options = {}) {
  const repoUrl = options.repoUrl || 'https://github.com/vox-foundation/vox';
  const repoRoot = options.repoRoot || DEFAULT_REPO_ROOT;
  const docsSrc = options.docsSrc || DEFAULT_DOCS_SRC;

  let mountsMap = options.mountsMap;
  let routeToRepoMap = options.routeToRepoMap;
  if (!mountsMap || !routeToRepoMap) {
    const contract = options.contract || loadMountContract();
    const mounts = discoverMounts({ repoRoot, docsSrc, contract });
    if (!mountsMap) {
      mountsMap = new Map();
      for (const m of mounts) {
        mountsMap.set(m.repoPath, m.route);
      }
    }
    if (!routeToRepoMap) {
      routeToRepoMap = new Map();
      for (const m of mounts) {
        routeToRepoMap.set(m.route, m.repoPath);
      }
    }
  }

  return function transformer(tree, file) {
    let filePath = file.path;
    if (!filePath && file.history && file.history.length > 0) {
      filePath = file.history[0];
    }
    if (!filePath) return;
    if (filePath.startsWith('file://')) {
      filePath = fileURLToPath(filePath);
    }

    let realFilePath = filePath;
    try {
      realFilePath = realpathSync(filePath);
    } catch {
      // Fallback
    }

    let effectiveFile = realFilePath;
    const frontmatter = file?.data?.astro?.frontmatter;
    if (frontmatter && typeof frontmatter.mounted_from === 'string' && frontmatter.mounted_from.trim()) {
      effectiveFile = resolve(repoRoot, frontmatter.mounted_from);
    } else {
      const normalizedPath = filePath.replace(/\\/g, '/');
      const relMatch = normalizedPath.match(/\/repo\/([^/]+)\.(md|mdx)$/);
      if (relMatch && routeToRepoMap && routeToRepoMap.has(relMatch[1])) {
        const repoPath = routeToRepoMap.get(relMatch[1]);
        effectiveFile = resolve(repoRoot, repoPath);
      }
    }

    const relToDocs = relative(docsSrc, effectiveFile).replace(/\\/g, '/');
    const isInsideDocs = !relToDocs.startsWith('..') && !isAbsolute(relToDocs);
    const isArchive = relToDocs === 'archive' || relToDocs.startsWith('archive/');

    const strict = options.strict ?? (isInsideDocs && !isArchive);
    const ctx = { repoRoot, docsSrc, repoUrl, strict, mountsMap };
    const isOutsideDocs = !isInsideDocs;

    visit(tree, ['link', 'definition', 'html', 'image'], (node) => {
      if (node.type === 'link' || node.type === 'definition') {
        if (typeof node.url !== 'string') return;
        const res = resolveDocLink(node.url, effectiveFile, ctx);
        if (res && res.href) {
          node.url = res.href;
        }
      } else if (node.type === 'image') {
        if (isOutsideDocs && typeof node.url === 'string') {
          node.url = rewriteImageUrl(node.url, effectiveFile, ctx);
        }
      } else if (node.type === 'html' && typeof node.value === 'string') {
        node.value = node.value.replace(
          /(<a\s+[^>]*?href=["'])([^"']+)(["'][^>]*>)/gi,
          (match, prefix, url, suffix) => {
            const res = resolveDocLink(url, effectiveFile, ctx);
            if (res && res.href) {
              return `${prefix}${res.href}${suffix}`;
            }
            return match;
          }
        );
        if (isOutsideDocs) {
          node.value = node.value.replace(
            /(<img\s+[^>]*?src=["'])([^"']+)(["'][^>]*>)/gi,
            (match, prefix, url, suffix) => {
              const rewritten = rewriteImageUrl(url, effectiveFile, ctx);
              return `${prefix}${rewritten}${suffix}`;
            }
          );
        }
      }
    });
  };
}
