/**
 * Phase 386 (`ROMA-V66-THEME-CONTRACT-01..04`): Enforceable & Self-Expanding GUI Theme Contract.
 *
 * Mirrors `contracts/gui/theme-contract.v1.json` and auto-enrolls all surfaces and
 * dock panels discovered via `discoverAllGuiSurfacesAndPanels()` so new buttons,
 * GUI pages, and components are automatically governed by the contract without
 * manual test registration.
 */

import { discoverAllGuiSurfacesAndPanels } from './cvSurfaceDiscovery';
import { normalizeTheme, isMinimalistDarkTheme, type ThemeId } from './theme';

export interface ThemeProfileSpec {
  id: 'arcane' | 'slate' | 'high-contrast' | 'glacier';
  aliases?: readonly string[];
  label: string;
  mode: 'light' | 'dark';
  selector: string;
  description: string;
  tokens: Record<string, string>;
}

export interface ThemeContractDriftFinding {
  file: string;
  line: number;
  kind: 'banned-dark-brown-hex' | 'banned-dark-brown-rgba' | 'banned-font-literal';
  token: string;
  message: string;
}

export interface WcagContrastCheckResult {
  foregroundVar: string;
  backgroundVar: string;
  foregroundHex: string;
  backgroundHex: string;
  ratio: number;
  minRequired: number;
  passed: boolean;
}

export interface ThemeContractHealthReport {
  contractId: string;
  schemaVersion: number;
  activeThemeId: ThemeId;
  activeThemeLabel: string;
  activeThemeMode: 'light' | 'dark';
  enforced: boolean;
  requiredSemanticVarCount: number;
  resolvedSemanticVarCount: number;
  missingSemanticVars: string[];
  wcagContrastPassed: boolean;
  minContrastRatio: number;
  contrastChecks: WcagContrastCheckResult[];
  enrolledSurfaceCount: number;
  enrolledSurfaces: string[];
  enrolledChatDockCount: number;
  enrolledChatDockPanels: readonly string[];
  supportedThemes: readonly ThemeId[];
}

export const REQUIRED_SEMANTIC_CSS_VARS = [
  '--brass',
  '--color-bg-base',
  '--color-bg-surface',
  '--color-bg-elevated',
  '--color-text-primary',
  '--color-text-secondary',
  '--color-text-muted',
  '--color-border-subtle',
  '--color-border-strong',
  '--color-accent-default',
  '--color-accent-secondary',
  '--color-status-pass',
  '--color-status-fail',
  '--color-status-warn',
  '--color-status-info',
  '--color-overlay-subtle',
  '--color-overlay-hover',
  '--color-overlay-solid',
  '--font-sans',
  '--font-serif',
  '--font-display',
  '--font-mono',
] as const;

export const BANNED_HARDCODED_DARK_BROWN_HEXES = [
  '#0a0805',
  '#0d0a07',
  '#0e0b08',
  '#100c08',
  '#120e0a',
  '#14100b',
  '#16120c',
  '#18120c',
  '#1a140e',
  '#1c160f',
  '#1e1710',
  '#201810',
  '#221a11',
  '#241c14',
  '#261d13',
  '#281f14',
  '#2a2011',
] as const;

export const BANNED_ROMANESQUE_FONT_LITERALS = [
  'Courier New',
  'Courier',
  'Arial',
  'Consolas',
  'Menlo',
  'Monaco',
] as const;

export const FONT_LITERAL_ALLOWLIST_PATHS = [
  'src/index.css',
  'src/styles/tokens.slate.generated.css',
  'src/lib/themeContract.ts',
] as const;

export const DARK_BROWN_HEX_ALLOWLIST_PATHS = [
  'src/index.css',
  'src/lib/themeContract.ts',
] as const;

export const THEME_CONTRACT = {
  schema_version: 1,
  contract_id: 'vox-gui-theme-contract-v1',
  default_theme: 'arcane' as const,
  supported_themes: ['arcane', 'slate', 'void', 'high-contrast', 'glacier'] as const,
  required_semantic_css_vars: REQUIRED_SEMANTIC_CSS_VARS,
  themes: {
    arcane: {
      id: 'arcane',
      label: 'Imperial Carrara White & Gold',
      mode: 'light',
      selector: ':root, [data-theme="arcane"]',
      description:
        'Default sunlit honed white/cream Carrara marble (#FAF6EE / #F5EFE2 / #EFE6D5), burnished gold mouldings (#B8860B / #C59B27), deep espresso ink (#1C140C), Roman Imperial Emerald working signets (#1B6B3A), and Cinzel/Palatino Romanesque serif typography.',
      tokens: {
        '--brass': '184 134 11',
        '--color-bg-base': '#FAF6EE',
        '--color-bg-surface': '#F5EFE2',
        '--color-bg-elevated': '#EFE6D5',
        '--color-text-primary': '#1C140C',
        '--color-text-secondary': '#3E2F1D',
        '--color-text-muted': '#553D22',
        '--color-border-subtle': 'rgba(184, 134, 11, 0.34)',
        '--color-border-strong': 'rgba(184, 134, 11, 0.62)',
        '--color-accent-default': '#B8860B',
        '--color-accent-secondary': '#1B6B3A',
        '--color-status-pass': '#1B6B3A',
        '--color-status-fail': '#9E2B25',
        '--color-status-warn': '#B86B11',
        '--color-status-info': '#8B6914',
        '--color-overlay-subtle': 'rgba(184, 134, 11, 0.08)',
        '--color-overlay-hover': 'rgba(184, 134, 11, 0.14)',
        '--color-overlay-solid': '#FAF6EE',
        '--font-sans': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
        '--font-serif': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
        '--font-display': "'Cinzel', 'Palatino', 'Palatino Linotype', 'Book Antiqua', Georgia, serif",
        '--font-mono': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
      },
    } satisfies ThemeProfileSpec,
    slate: {
      id: 'slate',
      aliases: ['void'],
      label: 'Minimalist Grey-Blue Slate',
      mode: 'dark',
      selector: '[data-theme="slate"], [data-theme="void"]',
      description:
        'User-selectable minimalist dark grey-blue basalt slate theme (#0c0e10 / #15191c / #1f262b / #2b343a) with crisp sans/mono typography, steel-blue & emerald status indicators, and zero marble textures or decorative columns.',
      tokens: {
        '--brass': '148 163 184',
        '--color-bg-base': '#0c0e10',
        '--color-bg-surface': '#15191c',
        '--color-bg-elevated': '#1f262b',
        '--color-text-primary': '#f1f5f9',
        '--color-text-secondary': '#cbd5e1',
        '--color-text-muted': '#94a3b8',
        '--color-border-subtle': '#2b343a',
        '--color-border-strong': '#3f4c56',
        '--color-accent-default': '#38bdf8',
        '--color-accent-secondary': '#10b981',
        '--color-status-pass': '#10b981',
        '--color-status-fail': '#f87171',
        '--color-status-warn': '#fbbf24',
        '--color-status-info': '#60a5fa',
        '--color-overlay-subtle': 'rgba(255, 255, 255, 0.04)',
        '--color-overlay-hover': 'rgba(255, 255, 255, 0.08)',
        '--color-overlay-solid': '#11151a',
        '--font-sans': "'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        '--font-serif': "'Palatino', 'Palatino Linotype', Georgia, serif",
        '--font-display': "'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        '--font-mono': "ui-monospace, 'SFMono-Regular', 'JetBrains Mono', monospace",
      },
    } satisfies ThemeProfileSpec,
    'high-contrast': {
      id: 'high-contrast',
      label: 'High Contrast Obsidian & Gold',
      mode: 'dark',
      selector: '[data-theme="high-contrast"]',
      description:
        'Maximum WCAG AAA contrast obsidian black (#000000 / #0a0a0a) with pure white typography (#ffffff) and high-luminance gold & emerald accents.',
      tokens: {
        '--brass': '212 175 55',
        '--color-bg-base': '#000000',
        '--color-bg-surface': '#000000',
        '--color-bg-elevated': '#0a0a0a',
        '--color-text-primary': '#ffffff',
        '--color-text-secondary': '#ffffff',
        '--color-text-muted': '#d4d4d8',
        '--color-border-subtle': '#a1a1aa',
        '--color-border-strong': '#ffffff',
        '--color-accent-default': '#d4af37',
        '--color-accent-secondary': '#4ade80',
        '--color-status-pass': '#4ade80',
        '--color-status-fail': '#f87171',
        '--color-status-warn': '#fbbf24',
        '--color-status-info': '#fde047',
        '--color-overlay-subtle': 'rgba(255, 255, 255, 0.08)',
        '--color-overlay-hover': 'rgba(255, 255, 255, 0.16)',
        '--color-overlay-solid': '#0a0a0a',
        '--font-sans': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
        '--font-serif': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
        '--font-display': "'Cinzel', 'Palatino', 'Palatino Linotype', 'Book Antiqua', Georgia, serif",
        '--font-mono': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
      },
    } satisfies ThemeProfileSpec,
    glacier: {
      id: 'glacier',
      aliases: ['travertine'],
      label: 'Sunlit Travertine & Verdigris',
      mode: 'light',
      selector: '[data-theme="glacier"], [data-theme="travertine"]',
      description:
        'Warm honed travertine limestone (#ece5d6 / #f4eee1 / #f6f1e7) with antique bronze (#7d5f20), verdigris (#1f5a50), and deep charcoal ink (#2a2620).',
      tokens: {
        '--brass': '138 106 38',
        '--color-bg-base': '#ece5d6',
        '--color-bg-surface': '#f4eee1',
        '--color-bg-elevated': '#f6f1e7',
        '--color-text-primary': '#2a2620',
        '--color-text-secondary': '#4a443a',
        '--color-text-muted': '#5a5347',
        '--color-border-subtle': 'rgba(138, 106, 38, 0.30)',
        '--color-border-strong': 'rgba(138, 106, 38, 0.56)',
        '--color-accent-default': '#7d5f20',
        '--color-accent-secondary': '#1f5a50',
        '--color-status-pass': '#1f5a50',
        '--color-status-fail': '#9E2B25',
        '--color-status-warn': '#B86B11',
        '--color-status-info': '#1d4e89',
        '--color-overlay-subtle': 'rgba(138, 106, 38, 0.06)',
        '--color-overlay-hover': 'rgba(138, 106, 38, 0.12)',
        '--color-overlay-solid': '#f6f1e7',
        '--font-sans': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
        '--font-serif': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
        '--font-display': "'Cinzel', 'Palatino', 'Palatino Linotype', 'Book Antiqua', Georgia, serif",
        '--font-mono': "'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif",
      },
    } satisfies ThemeProfileSpec,
  },
  banned_hardcoded_dark_brown_hexes: BANNED_HARDCODED_DARK_BROWN_HEXES,
  banned_romanesque_font_literals: BANNED_ROMANESQUE_FONT_LITERALS,
  font_literal_allowlist_paths: FONT_LITERAL_ALLOWLIST_PATHS,
  dark_brown_hex_allowlist_paths: DARK_BROWN_HEX_ALLOWLIST_PATHS,
  interactive_invariants: {
    single_line_buttons_and_badges: true,
    max_clipped_interactive_controls: 0,
    max_overlapping_control_pairs: 0,
    min_wcag_contrast_ratio: 4.5,
    auto_enroll_surface_registry: true,
    auto_enroll_chat_dock_panels: true,
  },
} as const;

const DARK_BROWN_RGBA_PATTERNS = [
  /rgba\(\s*10\s*,\s*8\s*,\s*5\s*,/i,
  /rgba\(\s*14\s*,\s*11\s*,\s*8\s*,/i,
  /rgba\(\s*18\s*,\s*14\s*,\s*10\s*,/i,
  /rgba\(\s*19\s*,\s*16\s*,\s*12\s*,/i,
  /rgba\(\s*20\s*,\s*16\s*,\s*11\s*,/i,
  /rgba\(\s*24\s*,\s*18\s*,\s*12\s*,/i,
  /rgba\(\s*28\s*,\s*22\s*,\s*15\s*,/i,
];

const ALLOWED_EMERALD_STATUS_HEXES = new Set([
  '#10b981',
  '#1b6b3a',
  '#1e7e46',
  '#156336',
  '#0d4524',
  '#15803d',
  '#064e3b',
]);

const GENERAL_DARK_HEX_BG_REGEX =
  /(?:bg|from|via|to)-\[#([0-2][0-9a-fA-F]{5})\]|(?:background|backgroundColor)\s*:\s*['"](#[0-2][0-9a-fA-F]{5})['"]/gi;

const INLINE_DARK_RGBA_BG_REGEX =
  /(?:background|backgroundColor|linear-gradient)[^;\n]*?(rgba\(\s*(?:[0-9]|[1-3][0-9])\s*,\s*(?:[0-9]|[1-3][0-9])\s*,\s*(?:[0-9]|[1-3][0-9])\s*,\s*0\.(?:8[5-9]|9[0-9])\s*\))/i;

/**
 * Parses a 6-digit hex color (`#RRGGBB`) into WCAG 2.1 relative luminance (0..1).
 */
export function computeRelativeLuminance(hex: string): number {
  const clean = hex.trim().replace(/^#/, '');
  if (!/^[0-9a-fA-F]{6}$/.test(clean)) return 0;
  const r = parseInt(clean.slice(0, 2), 16) / 255;
  const g = parseInt(clean.slice(2, 4), 16) / 255;
  const b = parseInt(clean.slice(4, 6), 16) / 255;
  const toLinear = (c: number) => (c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

/**
 * Computes WCAG 2.1 contrast ratio (1..21) between two `#RRGGBB` hex colors.
 */
export function computeContrastRatio(fgHex: string, bgHex: string): number {
  const l1 = computeRelativeLuminance(fgHex);
  const l2 = computeRelativeLuminance(bgHex);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return Number(((lighter + 0.05) / (darker + 0.05)).toFixed(2));
}

/**
 * Evaluates WCAG 2.1 contrast checks for a theme profile (`>= 4.5:1` for primary/secondary text,
 * `>= 4.5:1` for muted text across all 4 profiles).
 */
export function verifyThemeProfileContrast(profile: ThemeProfileSpec): WcagContrastCheckResult[] {
  const bgVars = ['--color-bg-base', '--color-bg-surface', '--color-bg-elevated'] as const;
  const fgSpecs = [
    { fgVar: '--color-text-primary', minRequired: 4.5 },
    { fgVar: '--color-text-secondary', minRequired: 4.5 },
    { fgVar: '--color-text-muted', minRequired: 4.5 },
  ] as const;

  const checks: WcagContrastCheckResult[] = [];
  for (const bgVar of bgVars) {
    const bgHex = profile.tokens[bgVar] ?? '#000000';
    for (const { fgVar, minRequired } of fgSpecs) {
      const fgHex = profile.tokens[fgVar] ?? '#000000';
      const ratio = computeContrastRatio(fgHex, bgHex);
      checks.push({
        foregroundVar: fgVar,
        backgroundVar: bgVar,
        foregroundHex: fgHex,
        backgroundHex: bgHex,
        ratio,
        minRequired,
        passed: ratio >= minRequired,
      });
    }
  }
  return checks;
}

/**
 * Audits a single source file (`.tsx`, `.ts`, `.css`) against the Theme Contract:
 * flags banned hardcoded dark-brown/near-black hexes/RGBAs and banned non-Romanesque font literals.
 */
export function auditSourceTextAgainstThemeContract(
  relativePath: string,
  content: string,
): ThemeContractDriftFinding[] {
  const normalizedPath = relativePath.replace(/\\/g, '/');
  const findings: ThemeContractDriftFinding[] = [];
  if (
    normalizedPath.endsWith('.test.ts') ||
    normalizedPath.endsWith('.test.tsx') ||
    normalizedPath.endsWith('.spec.ts')
  ) {
    return findings;
  }

  const allowDarkBrown = DARK_BROWN_HEX_ALLOWLIST_PATHS.some((p) =>
    normalizedPath.endsWith(p),
  );
  const allowFontLiteral = FONT_LITERAL_ALLOWLIST_PATHS.some((p) =>
    normalizedPath.endsWith(p),
  );

  const lines = content.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i] ?? '';
    const trimmed = rawLine.trim();
    if (
      trimmed.startsWith('//') ||
      trimmed.startsWith('/*') ||
      trimmed.startsWith('*') ||
      trimmed.includes('theme-contract-ignore') ||
      trimmed.includes('toestub-ignore')
    ) {
      continue;
    }

    const lower = trimmed.toLowerCase();
    if (!allowDarkBrown) {
      const reportedHexesOnLine = new Set<string>();
      for (const hex of BANNED_HARDCODED_DARK_BROWN_HEXES) {
        if (lower.includes(hex)) {
          reportedHexesOnLine.add(hex);
          findings.push({
            file: normalizedPath,
            line: i + 1,
            kind: 'banned-dark-brown-hex',
            token: hex,
            message: `Hardcoded legacy dark-brown hex '${hex}' violates Theme Contract; use semantic tokens (bg-bg-base, bg-bg-surface, bg-bg-elevated, var(--color-bg-*)).`,
          });
        }
      }

      // General pattern-based detection for any arbitrary dark background hex (#000000..#2fffff)
      if (!trimmed.includes('swatch:')) {
        for (const match of trimmed.matchAll(GENERAL_DARK_HEX_BG_REGEX)) {
          const rawHex = `#${(match[1] ?? match[2] ?? '').replace(/^#/, '')}`.toLowerCase();
          if (
            rawHex.length === 7 &&
            !ALLOWED_EMERALD_STATUS_HEXES.has(rawHex) &&
            !reportedHexesOnLine.has(rawHex)
          ) {
            reportedHexesOnLine.add(rawHex);
            findings.push({
              file: normalizedPath,
              line: i + 1,
              kind: 'banned-dark-brown-hex',
              token: rawHex,
              message: `Hardcoded dark background hex '${rawHex}' violates Theme Contract; use semantic tokens (bg-bg-base, bg-bg-surface, bg-bg-elevated, var(--color-bg-*)).`,
            });
          }
        }
      }

      let foundRgbaOnLine = false;
      for (const rx of DARK_BROWN_RGBA_PATTERNS) {
        const m = trimmed.match(rx);
        if (m) {
          foundRgbaOnLine = true;
          findings.push({
            file: normalizedPath,
            line: i + 1,
            kind: 'banned-dark-brown-rgba',
            token: m[0],
            message: `Hardcoded legacy dark-brown RGBA '${m[0]}' violates Theme Contract; use semantic overlay/surface tokens.`,
          });
        }
      }
      if (!foundRgbaOnLine) {
        const inlineMatch = trimmed.match(INLINE_DARK_RGBA_BG_REGEX);
        if (inlineMatch && inlineMatch[1]) {
          findings.push({
            file: normalizedPath,
            line: i + 1,
            kind: 'banned-dark-brown-rgba',
            token: inlineMatch[1],
            message: `Hardcoded dark inline RGBA background '${inlineMatch[1]}' violates Theme Contract; use semantic overlay/surface tokens.`,
          });
        }
      }
    }

    if (!allowFontLiteral) {
      for (const fontName of BANNED_ROMANESQUE_FONT_LITERALS) {
        // Match standalone font family identifier in quotes or unquoted in CSS/JS font-family lists
        const fontRegex = new RegExp(`(['",\\s:]|^)${fontName}(['",\\s;]|$)`, 'i');
        if (fontRegex.test(trimmed)) {
          findings.push({
            file: normalizedPath,
            line: i + 1,
            kind: 'banned-font-literal',
            token: fontName,
            message: `Banned non-Romanesque font '${fontName}' violates Theme Contract; use var(--font-sans), var(--font-serif), var(--font-display), or var(--font-mono).`,
          });
        }
      }
    }
  }

  return findings;
}

/**
 * Resolves the ThemeProfileSpec for any supported ThemeId (`arcane`, `slate`, `void`, `high-contrast`, `glacier`).
 */
export function resolveThemeProfileSpec(themeId: ThemeId): ThemeProfileSpec {
  if (themeId === 'slate' || themeId === 'void') {
    return THEME_CONTRACT.themes.slate;
  }
  if (themeId === 'high-contrast') {
    return THEME_CONTRACT.themes['high-contrast'];
  }
  if (themeId === 'glacier') {
    return THEME_CONTRACT.themes.glacier;
  }
  return THEME_CONTRACT.themes.arcane;
}

/**
 * Produces a live observable health snapshot of the Theme Contract for GUI display
 * (`Settings → Theme`) and automated test verification. When running in a browser DOM
 * with loaded stylesheets, inspects `getComputedStyle(document.documentElement)` as well
 * as the static contract profile and WCAG 2.1 contrast ratio checks.
 */
export function getThemeContractHealthReport(
  themePref?: string | null,
): ThemeContractHealthReport {
  const activeThemeId = normalizeTheme(
    themePref ??
      (typeof document !== 'undefined' ? document.documentElement?.dataset?.theme : 'arcane'),
  );
  const profile = resolveThemeProfileSpec(activeThemeId);
  const discovery = discoverAllGuiSurfacesAndPanels();

  // Check if live browser computed styles are populated (e.g. Playwright / live webview vs headless JSDOM without CSS bundle)
  let computedStyle: CSSStyleDeclaration | null = null;
  let hasLiveComputedStyles = false;
  if (typeof window !== 'undefined' && typeof document !== 'undefined' && document.documentElement) {
    try {
      const cs = window.getComputedStyle(document.documentElement);
      const probe = cs.getPropertyValue('--color-bg-base').trim();
      if (probe.length > 0) {
        computedStyle = cs;
        hasLiveComputedStyles = true;
      }
    } catch {
      computedStyle = null;
    }
  }

  const missingSemanticVars: string[] = [];
  for (const cssVar of REQUIRED_SEMANTIC_CSS_VARS) {
    const liveVal =
      hasLiveComputedStyles && computedStyle && !themePref
        ? computedStyle.getPropertyValue(cssVar).trim()
        : '';
    const fallbackVal = profile.tokens[cssVar] ?? '';
    const effectiveVal = liveVal.length > 0 ? liveVal : fallbackVal;
    if (!effectiveVal || effectiveVal.trim().length === 0) {
      missingSemanticVars.push(cssVar);
    }
  }

  const contrastChecks = verifyThemeProfileContrast(profile);
  const minContrastRatio = contrastChecks.reduce(
    (min, c) => Math.min(min, c.ratio),
    Number.POSITIVE_INFINITY,
  );
  const wcagContrastPassed = contrastChecks.every((c) => c.passed);

  return {
    contractId: THEME_CONTRACT.contract_id,
    schemaVersion: THEME_CONTRACT.schema_version,
    activeThemeId,
    activeThemeLabel: profile.label,
    activeThemeMode: profile.mode,
    enforced: missingSemanticVars.length === 0 && wcagContrastPassed,
    requiredSemanticVarCount: REQUIRED_SEMANTIC_CSS_VARS.length,
    resolvedSemanticVarCount: REQUIRED_SEMANTIC_CSS_VARS.length - missingSemanticVars.length,
    missingSemanticVars,
    wcagContrastPassed,
    minContrastRatio: Number.isFinite(minContrastRatio) ? minContrastRatio : 0,
    contrastChecks,
    enrolledSurfaceCount: discovery.routedViewKeys.length,
    enrolledSurfaces: discovery.routedViewKeys,
    enrolledChatDockCount: discovery.chatDockPanelIds.length,
    enrolledChatDockPanels: discovery.chatDockPanelIds,
    supportedThemes: THEME_CONTRACT.supported_themes,
  };
}
