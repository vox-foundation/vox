/**
 * Source-scanning helpers for the chat visual-language guard tests in this folder
 * (englishVocabulary, statusColorTokens, chatTypeScale, chatContrast).
 * Deliberately NOT named *.test.ts: vitest collects only test files, and tsc
 * type-checks this module the same way it checks lib/dashboardBundleBudgetInProcess.ts.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Absolute path of `crates/vox-gui/ui/src`. */
export const SRC_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

const SCOPE_DIRS = ['components/surfaces/Chat', 'components/surfaces/Loquela'];
/** The status bar, its research chip, and the shared status vocabulary. */
const SCOPE_EXTRA = [
  'components/layout/BottomStatusBar.tsx',
  'components/common/StatusBarCluster.tsx',
  'components/surfaces/VoxGraph/VoxGraphStatusPanel.tsx',
  // The status vocabulary every pill, badge and toast draws from.
  'styles/tokens.ts',
  'components/ui/StatusPill.tsx',
  'components/ui/Pill.tsx',
  'components/ui/Toasts.tsx',
];

/** Chat, composer and status-bar component sources (non-test `.tsx`), relative to SRC_ROOT, sorted. */
export function chatScopeFiles(): string[] {
  const fromDirs = SCOPE_DIRS.flatMap((dir) =>
    readdirSync(join(SRC_ROOT, dir))
      .filter((name) => name.endsWith('.tsx') && !name.endsWith('.test.tsx'))
      .map((name) => `${dir}/${name}`),
  );
  return [...fromDirs, ...SCOPE_EXTRA].sort();
}

export function readSrc(rel: string): string {
  return readFileSync(join(SRC_ROOT, rel), 'utf8');
}

/** Removes block and line comments so prose in comments is never scanned; keeps `://` and quoted `//`. */
export function stripComments(src: string): string {
  return src.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/(^|[^:'"`])\/\/.*$/gm, '$1');
}

const CODE_MARKERS = /[;=]|=>|\b(?:const|let|return|function|import|export)\b/;
const TEXT_ATTRS =
  /\b(?:aria-label|title|placeholder|alt|label)=(?:"([^"]*)"|'([^']*)'|\{\s*["'`]([^"'`]*)["'`]\s*\})/g;

/**
 * User-visible literal text in a TSX source: JSX text children (with `{expr}` parts dropped) and the
 * literal values of aria-label / title / placeholder / alt / label attributes. Fragments that look like
 * code (text between a generic's `>` and the next `<`) are skipped.
 */
export function visibleStrings(src: string): string[] {
  const code = stripComments(src);
  const out: string[] = [];
  for (const m of code.matchAll(/>([^<>]+)</g)) {
    const text = m[1].replace(/\{[^{}]*\}/g, ' ').replace(/\s+/g, ' ').trim();
    if (text && !CODE_MARKERS.test(text)) out.push(text);
  }
  for (const m of code.matchAll(TEXT_ATTRS)) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}

/**
 * Single-line string values after a colon: object-literal properties such as `label: "Auto · Router"` or
 * `moderate: 'Confirm + enforce grounding.'` (label data that renders as chrome without being JSX text).
 * Ternary else-branches (`: "cls"`) are included too; they are class strings and never contain vocabulary.
 */
export function propertyStrings(src: string): string[] {
  return [...stripComments(src).matchAll(/:\s*(?:'([^'\n]*)'|"([^"\n]*)")/g)].map((m) => m[1] ?? m[2]);
}
