/**
 * remark-vox-include — processes {{#include path:anchor}} directives inside
 * fenced code blocks, identical to the mdBook include syntax.
 *
 * Build-time errors are thrown for any unresolved path or anchor so that CI
 * (the Starlight pnpm build step) fails immediately instead of silently
 * emitting the raw directive as visible text.
 *
 * Selectors after the colon:
 *   name            region between `// ANCHOR: name` and `// ANCHOR_END: name`
 *                   (or AsciiDoc `// tag::name[]` / `// end::name[]`; `#` leaders too)
 *   a,b             several regions, concatenated in order
 *   10..20          1-based inclusive line range
 * Nested markers are stripped and the result is dedented.
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// Matches {{#include relative/path/file.ext:selector}} or {{#include path}} (no selector).
// selector = anchor/tag name, comma-separated names (concatenated in order), or a
// 1-based inclusive line range `10..20` / `10-20`.
const INCLUDE_RE = /^\s*\{\{#include\s+([^}:\s]+?)(?::([^}\s]+))?\}\}\s*$/;

// Any region marker line: mdBook `ANCHOR: x` / `ANCHOR_END: x` or AsciiDoc `tag::x[]` / `end::x[]`,
// behind a `//` or `#` comment leader. Stripped from output so markers never leak.
const MARKER_RE = /^\s*(?:\/\/|#)\s*(?:ANCHOR(?:_END)?:\s*[\w.-]+|(?:tag|end)::[\w.-]+\[\])\s*$/;
const RANGE_RE = /^(\d+)(?:\.\.|-)(\d+)$/;

const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** Lines of the first region delimited by `name` markers; throws if absent. */
function regionLines(lines, name, srcPath) {
  const n = esc(name);
  const leader = '^\\s*(?:\\/\\/|#)\\s*';
  const startRe = new RegExp(`${leader}(?:ANCHOR:\\s*${n}|tag::${n}\\[\\])\\s*$`);
  const endRe = new RegExp(`${leader}(?:ANCHOR_END:\\s*${n}|end::${n}\\[\\])\\s*$`);
  let inside = false;
  let found = false;
  const out = [];
  for (const line of lines) {
    if (!inside) {
      if (startRe.test(line)) { inside = true; found = true; }
    } else if (endRe.test(line)) {
      break;
    } else {
      out.push(line);
    }
  }
  if (!found) throw new Error(`Anchor '${name}' not found in ${srcPath}`);
  return out;
}

/**
 * Select a region (or the whole file), strip markers, trim blank edges, dedent.
 * Exported for unit tests.
 * @param {string} content
 * @param {string | undefined} selector
 * @param {string} srcPath  for error messages
 * @returns {string}
 */
export function extractRegion(content, selector, srcPath) {
  const lines = content.replace(/\r\n/g, '\n').split('\n');
  let picked = lines;
  if (selector) {
    const range = selector.match(RANGE_RE);
    if (range) {
      const [from, to] = [Number(range[1]), Number(range[2])];
      if (from < 1 || from > to || from > lines.length) {
        throw new Error(`Invalid line range '${selector}' in ${srcPath} (${lines.length} lines)`);
      }
      picked = lines.slice(from - 1, to);
    } else {
      picked = selector.split(',').map((t) => t.trim()).filter(Boolean)
        .flatMap((name) => regionLines(lines, name, srcPath));
    }
  }
  const out = picked.filter((l) => !MARKER_RE.test(l));
  while (out.length && !out[0].trim()) out.shift();
  while (out.length && !out[out.length - 1].trim()) out.pop();
  const indents = out.filter((l) => l.trim()).map((l) => l.match(/^[ \t]*/)[0].length);
  const cut = indents.length ? Math.min(...indents) : 0;
  return out.map((l) => l.slice(Math.min(cut, l.length))).join('\n');
}

/**
 * Simple recursive node visitor (avoids adding unist-util-visit as a dep).
 * @param {object} tree
 * @param {string} type
 * @param {(node: object) => void} fn
 */
function visit(tree, type, fn) {
  if (tree.type === type) fn(tree);
  if (Array.isArray(tree.children)) {
    for (const child of tree.children) visit(child, type, fn);
  }
}

/**
 * Remark plugin factory.
 *
 * Usage in astro.config.mjs:
 *   import { remarkVoxInclude } from './src/plugins/remark-vox-include.mjs';
 *   // markdown: { remarkPlugins: [remarkVoxInclude] }
 */
export function remarkVoxInclude() {
  return function transformer(tree, file) {
    // Resolve the directory of the file being processed.
    // `file.path` is the absolute path set by Astro's content pipeline.
    let fileDir;
    if (file.path) {
      // file.path may be a file:// URL on some platforms
      const rawPath = file.path.startsWith('file://')
        ? fileURLToPath(file.path)
        : file.path;
      fileDir = dirname(rawPath);
    } else if (file.history && file.history.length > 0) {
      fileDir = dirname(file.history[0]);
    } else {
      // No path info — skip silently (e.g., virtual content in tests)
      return;
    }

    const errors = [];

    visit(tree, 'code', (node) => {
      const match = node.value.match(INCLUDE_RE);
      if (!match) return;

      const [, relativePath, anchor] = match;
      const absPath = resolve(fileDir, relativePath);

      let fileContent;
      try {
        fileContent = readFileSync(absPath, 'utf8');
      } catch {
        errors.push(`  Cannot read '${relativePath}' (resolved: ${absPath})`);
        return;
      }

      try {
        node.value = extractRegion(fileContent, anchor, absPath);
      } catch (err) {
        errors.push(`  ${relativePath}:${anchor} — ${err.message}`);
      }
    });

    if (errors.length) {
      const location = file.path || '(unknown file)';
      throw new Error(
        `remark-vox-include: ${errors.length} unresolved include(s) in ${location}:\n${errors.join('\n')}\n` +
        `Fix the paths/anchors or run \`cargo run -p vox-doc-pipeline -- --lint-only\` to validate all includes.`
      );
    }
  };
}
