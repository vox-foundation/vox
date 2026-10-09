/**
 * remark-doc-links — rewrites repo-relative Markdown links to the routes the
 * built site serves. Source Markdown stays repo-relative so it still renders
 * on GitHub; only the built HTML changes.
 *
 * - A link to another docs page (`../reference/cli.md#vox-init`) becomes its
 *   Starlight route (`/reference/cli/#vox-init`), using the same docSlug()
 *   the sidebar and llms exclude list use.
 * - A relative link whose target does not exist throws for pages under
 *   docs/src, so a dead link fails the build instead of shipping a 404.
 *
 * Astro's content loader catches a remark error, logs it and keeps building,
 * so the throw alone does not fail `astro build`. Every dead link is also
 * recorded here and docLinksGate() fails the build at astro:build:done.
 */

import { existsSync, realpathSync, statSync } from 'node:fs';
import { dirname, extname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { docSlug } from '../utils/doc-slug.mjs';

const SCHEME_RE = /^[a-z][a-z0-9+.-]*:/i;
const LINE_SUFFIX_RE = /:(\d+)(?:-(\d+))?$/;

/** Dead-link messages from every file rendered in this process. */
const deadLinks = new Set();

/** True when `child` is `parent` or lies below it. */
function isWithin(parent, child) {
  const rel = relative(parent, child);
  return rel === '' || (!rel.startsWith('..') && !rel.startsWith(sep) && !/^[a-z]:/i.test(rel));
}

const toPosix = (path) => path.split(sep).join('/');

/**
 * Where a link `url` written in `fromFile` (a real path) should point on the
 * built site. Returns `{ href }` to rewrite, `null` to leave the URL as is,
 * or throws for a dead target when `ctx.strict` is set.
 *
 * @param {string} url
 * @param {string} fromFile
 * @param {{ repoRoot: string, docsSrc: string, repoUrl: string, strict: boolean }} ctx
 */
export function resolveDocLink(url, fromFile, ctx) {
  if (!url || SCHEME_RE.test(url) || /^[#/]/.test(url)) return null;

  const hashAt = url.indexOf('#');
  let path = hashAt === -1 ? url : url.slice(0, hashAt);
  const fragment = hashAt === -1 ? '' : url.slice(hashAt);
  if (LINE_SUFFIX_RE.test(path)) path = path.replace(LINE_SUFFIX_RE, '');
  if (!path) return null;
  try {
    path = decodeURI(path);
  } catch {
    // a malformed escape is kept raw and reported as missing below
  }

  const target = resolve(dirname(fromFile), path);
  const docsRel = isWithin(ctx.docsSrc, target) ? toPosix(relative(ctx.docsSrc, target)) : null;
  const archived = docsRel !== null && (docsRel === 'archive' || docsRel.startsWith('archive/'));

  if (!existsSync(target)) {
    const message = `remark-doc-links: dead link '${url}' in ${fromFile} (resolved: ${target})`;
    if (ctx.strict) throw new Error(message);
    console.warn(message);
    return null;
  }

  if (docsRel !== null && !archived && /\.mdx?$/.test(extname(target)) && statSync(target).isFile()) {
    const slug = docSlug(docsRel);
    return { href: (slug ? `/${slug}/` : '/') + fragment };
  }

  return null;
}

/** Real path of the file being processed, or null for virtual content. */
function sourcePath(file) {
  const raw = file.path || (file.history && file.history[0]);
  if (!raw) return null;
  const path = raw.startsWith('file://') ? fileURLToPath(raw) : raw;
  try {
    return realpathSync(path);
  } catch {
    return path;
  }
}

function visitLinks(node, fn) {
  if (node.type === 'link' || node.type === 'definition') fn(node);
  if (Array.isArray(node.children)) {
    for (const child of node.children) visitLinks(child, fn);
  }
}

/**
 * Remark plugin factory.
 *
 * @param {{ repoRoot: string, repoUrl: string, docsSrc?: string }} options
 */
export function remarkDocLinks(options) {
  const repoRoot = realpathSync(options.repoRoot);
  const docsSrc = options.docsSrc ? realpathSync(options.docsSrc) : resolve(repoRoot, 'docs/src');
  const archive = resolve(docsSrc, 'archive');

  return function transformer(tree, file) {
    const fromFile = sourcePath(file);
    if (!fromFile) return;
    const ctx = {
      repoRoot,
      docsSrc,
      repoUrl: options.repoUrl,
      strict: isWithin(docsSrc, fromFile) && !isWithin(archive, fromFile),
    };
    const errors = [];
    visitLinks(tree, (node) => {
      try {
        const resolved = resolveDocLink(node.url, fromFile, ctx);
        if (resolved) node.url = resolved.href;
      } catch (err) {
        errors.push(err.message);
      }
    });
    if (errors.length) {
      for (const message of errors) deadLinks.add(message);
      throw new Error(errors.join('\n'));
    }
  };
}

/** Astro integration that fails the build if any page had a dead link. */
export function docLinksGate() {
  return {
    name: 'remark-doc-links-gate',
    hooks: {
      'astro:build:done': () => {
        if (deadLinks.size) {
          throw new Error(`remark-doc-links: ${deadLinks.size} dead link(s):\n${[...deadLinks].join('\n')}`);
        }
      },
    },
  };
}
