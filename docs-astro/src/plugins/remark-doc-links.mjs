/**
 * remark-doc-links — rewrites repo-relative Markdown links to the routes the
 * built site serves. Source Markdown stays repo-relative so it still renders
 * on GitHub; only the built HTML changes.
 *
 * - A link to another docs page (`../reference/cli.md#vox-init`) becomes its
 *   Starlight route (`/reference/cli/#vox-init`), using the same docSlug()
 *   the sidebar and llms exclude list use.
 * - Anything else in the repository (source, contracts, directories, repo
 *   Markdown outside docs/src, the unpublished archive and superpowers plans)
 *   becomes a GitHub blob/tree URL; a `:N` / `:N-M` suffix becomes `#LN` /
 *   `#LN-LM`.
 * - A relative link whose target does not exist, or that resolves outside the
 *   repository, throws for pages under docs/src, so a dead link fails the
 *   build instead of shipping a 404. Targets are only ever stat'ed, never read.
 *
 * Astro's content loader catches a remark error, logs it and keeps building,
 * so the throw alone does not fail `astro build`. Every dead link is also
 * recorded here and docLinksGate() fails the build at astro:build:done.
 */

import { existsSync, realpathSync, statSync } from 'node:fs';
import { basename, dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { docSlug } from '../utils/doc-slug.mjs';
import { MIRROR_EXCLUDED } from '../../scripts/setup-content.mjs';

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
  let fragment = hashAt === -1 ? '' : url.slice(hashAt);
  const lines = path.match(LINE_SUFFIX_RE);
  if (lines) path = path.slice(0, -lines[0].length);
  if (!path) return null;
  try {
    path = decodeURI(path);
  } catch {
    // a malformed escape is kept raw and reported as missing below
  }

  const target = resolve(dirname(fromFile), path);
  const fail = (reason) => {
    const message = `remark-doc-links: ${reason} '${url}' in ${fromFile} (resolved: ${target})`;
    if (ctx.strict) throw new Error(message);
    console.warn(message);
    return null;
  };
  if (!isWithin(ctx.repoRoot, target)) return fail('link leaves the repository');
  if (!existsSync(target)) return fail('dead link');

  const isDir = statSync(target).isDirectory();
  const docsRel = isWithin(ctx.docsSrc, target) ? toPosix(relative(ctx.docsSrc, target)) : null;
  if (docsRel !== null && !MIRROR_EXCLUDED.includes(docsRel.split('/')[0])) {
    const page = isDir ? ['index.md', 'index.mdx'].find((name) => existsSync(join(target, name))) : docsRel;
    if (page && /\.mdx?$/.test(page) && !basename(page).startsWith('_')) {
      const slug = docSlug(isDir ? `${docsRel}/${page}` : docsRel);
      return { href: (slug ? `/${slug}/` : '/') + fragment };
    }
  }

  if (lines) fragment = lines[2] ? `#L${lines[1]}-L${lines[2]}` : `#L${lines[1]}`;
  const repoRel = encodeURI(toPosix(relative(ctx.repoRoot, target)));
  return { href: `${ctx.repoUrl}/${isDir ? 'tree' : 'blob'}/main/${repoRel}${fragment}` };
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

function visit(node, fn) {
  fn(node);
  if (Array.isArray(node.children)) {
    for (const child of node.children) visit(child, fn);
  }
}

/** Calls `fn` on every link and definition, skipping definitions an image uses. */
function visitLinks(tree, fn) {
  const imageIds = new Set();
  visit(tree, (node) => {
    if (node.type === 'imageReference') imageIds.add(node.identifier);
  });
  visit(tree, (node) => {
    if (node.type === 'link' || (node.type === 'definition' && !imageIds.has(node.identifier))) fn(node);
  });
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
