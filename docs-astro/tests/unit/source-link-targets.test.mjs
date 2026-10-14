import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../../..');

export function relativeLinkTargets(markdown) {
  // 1. Strip fenced code blocks (``` or ~~~)
  const lines = markdown.split(/\r?\n/);
  const nonFencedLines = [];
  let inFence = false;
  let fenceMarker = '';
  let fenceLen = 0;

  for (const line of lines) {
    const trimmed = line.trimStart();
    if (!inFence) {
      const match = trimmed.match(/^(`{3,}|~{3,})/);
      if (match) {
        inFence = true;
        fenceMarker = match[1][0];
        fenceLen = match[1].length;
        continue;
      }
      nonFencedLines.push(line);
    } else {
      const match = trimmed.match(/^(`{3,}|~{3,})/);
      if (match && match[1][0] === fenceMarker && match[1].length >= fenceLen) {
        inFence = false;
      }
    }
  }

  // 2. Strip inline code
  let text = nonFencedLines.join('\n');
  text = text.replace(/`[^`\r\n]*`/g, '');

  // 3. Collect inline link targets: ](target) and ](target "title")
  const targets = [];
  const inlineRegex = /\]\(\s*([^\s)"']+)(?:\s+["'][^"']*["'])?\s*\)/g;
  let match;
  while ((match = inlineRegex.exec(text)) !== null) {
    targets.push(match[1]);
  }

  // 4. Collect reference definitions: ^[label]: target (excluding footnotes [^...])
  const refRegex = /^[ \t]*\[(?!\^)[^\]]+\]:\s*([^\s"']+)/gm;
  while ((match = refRegex.exec(text)) !== null) {
    targets.push(match[1]);
  }

  // 5. Filter and normalize relative targets
  const cleaned = [];
  for (let t of targets) {
    if (!t) continue;
    // Skip targets that start with a URL scheme, #, /, <, or mailto:
    if (t.startsWith('<') || t.startsWith('#') || t.startsWith('/') || t.startsWith('mailto:')) {
      continue;
    }
    if (/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(t)) {
      continue;
    }

    // Strip #fragment
    const hashIdx = t.indexOf('#');
    if (hashIdx !== -1) {
      t = t.slice(0, hashIdx);
    }
    if (!t) continue;

    // Strip trailing :N or :N-M
    t = t.replace(/:\d+(?:-\d+)?$/, '');
    if (!t) continue;

    try {
      t = decodeURI(t);
    } catch {
      // Keep original if decodeURI fails
    }

    cleaned.push(t);
  }

  return cleaned;
}

// Enumerate files under docs/src tracked in git
const docsFiles = execFileSync('git', ['ls-files', 'docs/src'], {
  cwd: REPO_ROOT,
  encoding: 'utf8',
})
  .split(/\r?\n/)
  .filter(
    (f) =>
      (f.endsWith('.md') || f.endsWith('.mdx')) &&
      !f.startsWith('docs/src/archive/') &&
      !f.startsWith('docs/src/.well-known/')
  )
  .sort();

for (const repoRelPath of docsFiles) {
  test(repoRelPath, () => {
    const fullPath = join(REPO_ROOT, repoRelPath);
    const content = readFileSync(fullPath, 'utf8');
    const targets = relativeLinkTargets(content);
    const docDir = dirname(fullPath);

    const missing = [];
    for (const target of targets) {
      const resolved = resolve(docDir, target);
      if (!existsSync(resolved)) {
        missing.push({ target, resolved });
      }
    }

    assert.equal(
      missing.length,
      0,
      `Dead relative link(s) in ${repoRelPath}:\n` +
        missing.map((m) => `  - "${m.target}" (resolved: ${m.resolved})`).join('\n')
    );
  });
}
