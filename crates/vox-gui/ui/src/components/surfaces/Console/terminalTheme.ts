import { useEffect, useState, type CSSProperties } from 'react';
import type { ITheme } from '@xterm/xterm';
import { normalizeTheme, type ThemeId } from '../../../lib/theme';

export const TERMINAL_MONO_FONT_STACK =
  "var(--font-terminal-mono, 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace)";

export const XTERM_FONT_FAMILY = "'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace";

export const TERMINAL_MONO_STYLE: CSSProperties = {
  fontFamily: TERMINAL_MONO_FONT_STACK,
  fontVariantLigatures: 'none',
  fontFeatureSettings: '"calt" 0, "liga" 0',
  letterSpacing: '0px',
};

export interface Ansi16Palette {
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

export interface TerminalWellSpec {
  wellBg: string;
  wellFg: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  palette: Ansi16Palette;
}

export const ARCANE_ANSI_16: Ansi16Palette = {
  black: '#8b7e6d',
  red: '#e57368',
  green: '#7cc47f',
  yellow: '#e0b44c',
  blue: '#7aa8e8',
  magenta: '#c98fd8',
  cyan: '#68bfc2',
  white: '#ebe3d3',
  brightBlack: '#a89a87',
  brightRed: '#f48fb1',
  brightGreen: '#98db9b',
  brightYellow: '#f0cb69',
  brightBlue: '#9ec2f7',
  brightMagenta: '#dfb0ec',
  brightCyan: '#8ad8db',
  brightWhite: '#fbf7ee',
};

export const TRAVERTINE_ANSI_16: Ansi16Palette = {
  black: '#8e8477',
  red: '#e5766b',
  green: '#7ec684',
  yellow: '#dfb653',
  blue: '#7daae8',
  magenta: '#ca93d9',
  cyan: '#6bc1c4',
  white: '#ede6d8',
  brightBlack: '#aba092',
  brightRed: '#f2948a',
  brightGreen: '#9bdd9f',
  brightYellow: '#f1ce70',
  brightBlue: '#a0c4f7',
  brightMagenta: '#e1b3ee',
  brightCyan: '#8ddadc',
  brightWhite: '#fcf9f2',
};

export const SLATE_ANSI_16: Ansi16Palette = {
  black: '#7c8b9e',
  red: '#f87171',
  green: '#4ade80',
  yellow: '#fbbf24',
  blue: '#60a5fa',
  magenta: '#c084fc',
  cyan: '#38bdf8',
  white: '#e2e8f0',
  brightBlack: '#94a3b8',
  brightRed: '#fca5a5',
  brightGreen: '#86efac',
  brightYellow: '#fde047',
  brightBlue: '#93c5fd',
  brightMagenta: '#d8b4fe',
  brightCyan: '#7dd3fc',
  brightWhite: '#f8fafc',
};

export const HIGH_CONTRAST_ANSI_16: Ansi16Palette = {
  black: '#a3b1c6',
  red: '#ff9494',
  green: '#86efac',
  yellow: '#fde047',
  blue: '#93c5fd',
  magenta: '#e9d5ff',
  cyan: '#67e8f9',
  white: '#ffffff',
  brightBlack: '#cbd5e1',
  brightRed: '#fecaca',
  brightGreen: '#bbf7d0',
  brightYellow: '#fef08a',
  brightBlue: '#bfdbfe',
  brightMagenta: '#f3e8ff',
  brightCyan: '#a5f3fc',
  brightWhite: '#ffffff',
};

const TERMINAL_WELL_SPECS: Record<'arcane' | 'travertine' | 'slate' | 'high-contrast', TerminalWellSpec> = {
  arcane: {
    wellBg: '#15110d',
    wellFg: '#f5efe2',
    cursor: '#e0c478',
    cursorAccent: '#15110d',
    selectionBackground: 'rgba(212, 175, 55, 0.28)',
    palette: ARCANE_ANSI_16,
  },
  travertine: {
    wellBg: '#171512',
    wellFg: '#f6f1e7',
    cursor: '#79c8ba',
    cursorAccent: '#171512',
    selectionBackground: 'rgba(121, 200, 186, 0.26)',
    palette: TRAVERTINE_ANSI_16,
  },
  slate: {
    wellBg: '#0c0e10',
    wellFg: '#f1f5f9',
    cursor: '#38bdf8',
    cursorAccent: '#0c0e10',
    selectionBackground: 'rgba(56, 189, 248, 0.24)',
    palette: SLATE_ANSI_16,
  },
  'high-contrast': {
    wellBg: '#000000',
    wellFg: '#ffffff',
    cursor: '#fde047',
    cursorAccent: '#000000',
    selectionBackground: 'rgba(253, 224, 71, 0.32)',
    palette: HIGH_CONTRAST_ANSI_16,
  },
};

function resolveActiveThemeId(themePref?: string | null): ThemeId {
  const domTheme =
    typeof document !== 'undefined'
      ? document.documentElement?.getAttribute('data-theme') ?? undefined
      : undefined;
  const candidate = !themePref || themePref === 'system' ? domTheme : themePref;
  return normalizeTheme(candidate ?? 'arcane');
}

export function resolveTerminalWellSpec(themePref?: string | null): TerminalWellSpec {
  const activeId = resolveActiveThemeId(themePref);
  if (activeId === 'slate' || activeId === 'void') return TERMINAL_WELL_SPECS.slate;
  if (activeId === 'travertine' || activeId === 'glacier') return TERMINAL_WELL_SPECS.travertine;
  if (activeId === 'high-contrast') return TERMINAL_WELL_SPECS['high-contrast'];
  return TERMINAL_WELL_SPECS.arcane;
}

function readLiveCssVar(cssVar: string): string {
  if (typeof window === 'undefined' || typeof document === 'undefined' || !document.documentElement) {
    return '';
  }
  try {
    return window.getComputedStyle(document.documentElement).getPropertyValue(cssVar).trim();
  } catch {
    return '';
  }
}

function paletteToOrderedArray(p: Ansi16Palette): readonly string[] {
  return [
    p.black, p.red, p.green, p.yellow, p.blue, p.magenta, p.cyan, p.white,
    p.brightBlack, p.brightRed, p.brightGreen, p.brightYellow, p.brightBlue, p.brightMagenta, p.brightCyan, p.brightWhite,
  ];
}

export const LIGHT_ANSI_16_PALETTE: readonly string[] = paletteToOrderedArray(ARCANE_ANSI_16);
export const DARK_ANSI_16_PALETTE: readonly string[] = paletteToOrderedArray(SLATE_ANSI_16);

export function resolveAnsi16Palette(themePref?: string | null): Ansi16Palette {
  return { ...resolveTerminalWellSpec(themePref).palette };
}

export function resolveAnsi16PaletteArray(themePref?: string | null): readonly string[] {
  return paletteToOrderedArray(resolveTerminalWellSpec(themePref).palette);
}

export function readDocumentThemeSignature(): string {
  if (typeof document === 'undefined' || !document.documentElement) return 'arcane';
  const ds = document.documentElement.dataset?.theme ?? '';
  const cls = document.documentElement.className ?? '';
  return `${ds}|${cls}`;
}

export function useDocumentThemeAttr(): string {
  const [sig, setSig] = useState<string>(() => readDocumentThemeSignature());
  useEffect(() => {
    if (typeof document === 'undefined' || !document.documentElement || typeof MutationObserver === 'undefined') {
      return;
    }
    const obs = new MutationObserver(() => {
      setSig(readDocumentThemeSignature());
    });
    obs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme', 'class'],
    });
    return () => obs.disconnect();
  }, []);
  return sig;
}

export function resolveTerminalTheme(themePref?: string | null): ITheme {
  const domTheme =
    typeof document !== 'undefined'
      ? document.documentElement?.getAttribute('data-theme') ?? undefined
      : undefined;
  const activeId = resolveActiveThemeId(themePref);
  const spec = resolveTerminalWellSpec(activeId);
  const canUseLiveDomVars = themePref == null || themePref === 'system' || normalizeTheme(domTheme) === activeId;

  const bgToken = (canUseLiveDomVars ? readLiveCssVar('--terminal-well-bg') : '') || spec.wellBg;
  const fgToken = (canUseLiveDomVars ? readLiveCssVar('--terminal-well-fg') : '') || spec.wellFg;
  const cursorToken = (canUseLiveDomVars ? readLiveCssVar('--terminal-well-cursor') : '') || spec.cursor;
  const selectionBgToken =
    (canUseLiveDomVars ? readLiveCssVar('--terminal-well-selection') : '') || spec.selectionBackground;

  return {
    background: bgToken,
    foreground: fgToken,
    cursor: cursorToken,
    cursorAccent: bgToken,
    selectionBackground: selectionBgToken,
    ...spec.palette,
  };
}
