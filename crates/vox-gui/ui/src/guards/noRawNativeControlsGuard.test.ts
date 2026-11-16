import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

/**
 * AXIS-LIVE-CONTROLS-03 / AXIS-LIVE-CONTROLS-04 static regression guard:
 * Production UI surfaces must use `<VoxSelect>` (or Radix `Select` primitives) and
 * `<Slider>` / `<RomanRotaryKnob>` instead of raw native `<select>` or `<input type="range">`.
 */
const SRC_ROOT = join(import.meta.dirname, '..');
const EXCLUDED_DIRS = new Set(['guards', '__tests__']);

const NATIVE_SELECT_RE = /<select\b/;
const NATIVE_RANGE_RE = /\btype\s*=\s*(?:["']range["']|\{\s*["']range["']\s*\})/;

function* walkProductionSourceFiles(dir: string): Generator<string> {
  for (const entry of readdirSync(dir)) {
    if (EXCLUDED_DIRS.has(entry)) continue;
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      yield* walkProductionSourceFiles(full);
      continue;
    }
    if (!/\.(ts|tsx)$/.test(entry)) continue;
    if (/\.test\.(ts|tsx)$/.test(entry)) continue;
    yield full;
  }
}

function stripCommentsPreservingLines(source: string): string {
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

describe('no raw native <select> or <input type="range"> controls guard', () => {
  it('has zero raw <select> JSX tags under src/ (use VoxSelect instead)', () => {
    const violations: string[] = [];

    for (const file of walkProductionSourceFiles(SRC_ROOT)) {
      const rel = relative(SRC_ROOT, file).replace(/\\/g, '/');
      const stripped = stripCommentsPreservingLines(readFileSync(file, 'utf8'));
      const lines = stripped.split('\n');

      for (let idx = 0; idx < lines.length; idx++) {
        const line = lines[idx]!;
        if (rel === 'lib/cvFlywheel.ts' && /['"`].*<select\b.*['"`]/.test(line)) {
          continue;
        }
        if (NATIVE_SELECT_RE.test(line)) {
          violations.push(`src/${rel}:${idx + 1}: ${line.trim()}`);
        }
      }
    }

    expect(
      violations,
      `Raw native <select> JSX tags are banned in production UI (use <VoxSelect> from components/ui/Select):\n${violations.join('\n')}`,
    ).toEqual([]);
  });

  it('has zero raw <input type="range"> tags under src/ (use Slider or RomanRotaryKnob instead)', () => {
    const violations: string[] = [];

    for (const file of walkProductionSourceFiles(SRC_ROOT)) {
      const rel = relative(SRC_ROOT, file).replace(/\\/g, '/');
      const stripped = stripCommentsPreservingLines(readFileSync(file, 'utf8'));
      const lines = stripped.split('\n');

      for (let idx = 0; idx < lines.length; idx++) {
        const line = lines[idx]!;
        if (rel === 'lib/cvFlywheel.ts' && /['"`].*range.*['"`]/.test(line)) {
          continue;
        }
        if (NATIVE_RANGE_RE.test(line)) {
          violations.push(`src/${rel}:${idx + 1}: ${line.trim()}`);
        }
      }
    }

    expect(
      violations,
      `Raw native <input type="range"> tags are banned in production UI (use <Slider> or <RomanRotaryKnob>):\n${violations.join('\n')}`,
    ).toEqual([]);
  });
});
