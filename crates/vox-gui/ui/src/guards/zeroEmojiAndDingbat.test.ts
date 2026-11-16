import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const BANNED_GLYPH_RE =
  /[\p{Extended_Pictographic}\u2600-\u27BF\u23F0-\u23FE\u25A0-\u25FF\u2197\u21BA-\u21C4\u21E2\u2B06\u2302\u2442\u2295\u21B5\u23CE\u21E5]/gu;

const SRC_ROOT = join(import.meta.dirname, '..');
const EXCLUDED_DIRS = new Set(['generated', '__tests__']);
const EXEMPT_REL_PATHS = new Set(['lib/keybinds.ts']);

function* walkSourceFiles(dir: string): Generator<string> {
  for (const entry of readdirSync(dir)) {
    if (EXCLUDED_DIRS.has(entry)) continue;
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      yield* walkSourceFiles(full);
      continue;
    }
    if (!/\.(ts|tsx)$/.test(entry)) continue;
    if (/\.test\.(ts|tsx)$/.test(entry)) continue;
    const rel = relative(SRC_ROOT, full).replace(/\\/g, '/');
    if (EXEMPT_REL_PATHS.has(rel)) continue;
    yield full;
  }
}

export function stripCommentsPreservingLines(source: string): string {
  type State =
    | 'code'
    | 'line_comment'
    | 'block_comment'
    | 'single_quote'
    | 'double_quote'
    | 'template';

  let state: State = 'code';
  const out: string[] = [];
  let i = 0;

  while (i < source.length) {
    const ch = source[i]!;
    const next = source[i + 1];

    if (state === 'code') {
      if (ch === '/' && next === '/') {
        out.push('  ');
        i += 2;
        state = 'line_comment';
        continue;
      }
      if (ch === '/' && next === '*') {
        out.push('  ');
        i += 2;
        state = 'block_comment';
        continue;
      }
      if (ch === "'" && (i === 0 || !/[A-Za-z0-9]/.test(source[i - 1]!))) {
        out.push(ch);
        i += 1;
        state = 'single_quote';
        continue;
      }
      if (ch === '"') {
        out.push(ch);
        i += 1;
        state = 'double_quote';
        continue;
      }
      if (ch === '`') {
        out.push(ch);
        i += 1;
        state = 'template';
        continue;
      }
      out.push(ch);
      i += 1;
      continue;
    }

    if (state === 'line_comment') {
      if (ch === '\n') {
        out.push('\n');
        state = 'code';
      } else if (ch === '\r') {
        out.push('\r');
      } else {
        out.push(' ');
      }
      i += 1;
      continue;
    }

    if (state === 'block_comment') {
      if (ch === '*' && next === '/') {
        out.push('  ');
        i += 2;
        state = 'code';
        continue;
      }
      out.push(ch === '\n' || ch === '\r' ? ch : ' ');
      i += 1;
      continue;
    }

    if (state === 'single_quote') {
      if (ch === '\\' && next !== undefined) {
        out.push(ch, next);
        i += 2;
        continue;
      }
      out.push(ch);
      if (ch === "'" || ch === '\n') {
        state = 'code';
      }
      i += 1;
      continue;
    }

    if (state === 'double_quote') {
      if (ch === '\\' && next !== undefined) {
        out.push(ch, next);
        i += 2;
        continue;
      }
      out.push(ch);
      if (ch === '"' || ch === '\n') {
        state = 'code';
      }
      i += 1;
      continue;
    }

    // state === 'template'
    if (ch === '\\' && next !== undefined) {
      out.push(ch, next);
      i += 2;
      continue;
    }
    out.push(ch);
    if (ch === '`') {
      state = 'code';
    }
    i += 1;
  }

  return out.join('');
}

function formatCodepoint(glyph: string): string {
  const cp = glyph.codePointAt(0) ?? 0;
  return `U+${cp.toString(16).toUpperCase().padStart(4, '0')}`;
}

describe('zero emoji and dingbat source guard', () => {
  it('strips line and block comments without shifting line numbers or truncating URLs', () => {
    const sample = [
      'const url = "https://vox.dev/path"; // \u25B8 comment',
      '/* block \u26A0',
      '   still comment \u{1F9CD} */',
      'const label = "clean";',
    ].join('\n');
    const stripped = stripCommentsPreservingLines(sample);
    expect(stripped.split('\n')).toHaveLength(4);
    expect(stripped).toContain('https://vox.dev/path');
    expect(stripped.match(BANNED_GLYPH_RE)).toBeNull();
  });

  it('has zero banned emoji or dingbat glyphs in production UI source files', () => {
    const violations: string[] = [];

    for (const file of walkSourceFiles(SRC_ROOT)) {
      const rel = `src/${relative(SRC_ROOT, file).replace(/\\/g, '/')}`;
      const raw = readFileSync(file, 'utf8');
      const stripped = stripCommentsPreservingLines(raw);
      const lines = stripped.split('\n');

      for (let idx = 0; idx < lines.length; idx++) {
        const line = lines[idx]!;
        BANNED_GLYPH_RE.lastIndex = 0;
        let match: RegExpExecArray | null;
        while ((match = BANNED_GLYPH_RE.exec(line)) !== null) {
          const glyph = match[0];
          violations.push(
            `${rel}:${idx + 1}:${match.index + 1} ${formatCodepoint(glyph)} (${JSON.stringify(glyph)})`,
          );
        }
      }
    }

    expect(
      violations,
      `Banned raw emoji/dingbat glyphs found (use Icon.*, VoxIcon, KeycapIcon, PlanStepGlyph, RoleRankEmblem, or RomanCrestPlate instead):\n${violations.join('\n')}`,
    ).toEqual([]);
  });
});
