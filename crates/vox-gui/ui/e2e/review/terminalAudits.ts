import type { Page } from '@playwright/test';

export interface TerminalAuditMetrics {
  terminalStartupSpamCount: number;
  xtermHelperLeakCount: number;
  terminalFontProportionalLeakCount: number;
  terminalContrastViolationCount: number;
  samples: string[];
}

/**
 * Self-contained in-page terminal DOM/CSS/canvas invariant audit (`TERM-ROMA-CV-03`).
 * Passed directly to `page.evaluate(collectTerminalAuditMetrics)` — MUST NOT reference
 * any outer-scope imports, helpers, or constants.
 */
export function collectTerminalAuditMetrics(doc?: Document): TerminalAuditMetrics {
  const rootDoc = doc ?? document;
  const win = rootDoc.defaultView ?? window;
  const samples: string[] = [];
  const pushSample = (msg: string) => {
    if (samples.length < 16) samples.push(msg);
  };

  const hasLayout =
    (win.innerWidth || 0) > 0 && rootDoc.body != null && rootDoc.body.getBoundingClientRect().width > 0;

  const isXtermRowChild = (el: HTMLElement): boolean =>
    Boolean(el.closest('.xterm-rows, .xterm-selection'));

  const isVisible = (el: HTMLElement): boolean => {
    if (el.classList.contains('sr-only') || el.classList.contains('xterm-helper-textarea')) {
      return false;
    }
    if (el.closest('.sr-only, [hidden], details:not([open])')) {
      return false;
    }
    const ariaHiddenAnc = el.closest('[aria-hidden="true"]');
    if (ariaHiddenAnc) {
      const ownXtermOverlay =
        isXtermRowChild(el) &&
        (ariaHiddenAnc.classList.contains('xterm-rows') ||
          ariaHiddenAnc.classList.contains('xterm-selection'));
      if (!ownXtermOverlay) return false;
      if (ariaHiddenAnc.parentElement?.closest('[aria-hidden="true"]')) return false;
    }
    let cur: HTMLElement | null = el;
    while (cur) {
      const cs = win.getComputedStyle(cur);
      if (
        cs.display === 'none' ||
        cs.visibility === 'hidden' ||
        Number.parseFloat(cs.opacity === '' ? '1' : cs.opacity) <= 0.05
      ) {
        return false;
      }
      cur = cur.parentElement;
    }
    if (!hasLayout) return true;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  };

  // 1. Shell startup / prompt integration spam check
  const spamRe =
    /(\x1b\]133;|\]133;[ABCD]|__vox_(?:ps|preexec|precmd|trappost)\b|PROMPT_COMMAND=|precmd_functions|chpwd_functions|export\s+PS1=)/;
  let terminalStartupSpamCount = 0;
  const outputSelector =
    '[data-testid="term-block-output"], [data-testid="chat-term-block-output"], .xterm-rows, [data-testid="terminal-viewport"]';
  for (const el of Array.from(rootDoc.querySelectorAll<HTMLElement>(outputSelector))) {
    if (
      el.getAttribute('data-testid') === 'terminal-viewport' &&
      el.querySelector('[data-testid="term-block-output"], .xterm-rows')
    ) {
      continue;
    }
    if (!isVisible(el)) continue;
    const txt = el.textContent || '';
    const m = txt.match(spamRe);
    if (m) {
      terminalStartupSpamCount++;
      pushSample(`startup-spam:${m[0]}`);
    }
  }

  // 2. .xterm-helper-textarea zero-box containment check
  let xtermHelperLeakCount = 0;
  for (const ta of Array.from(rootDoc.querySelectorAll<HTMLElement>('.xterm-helper-textarea'))) {
    const cs = win.getComputedStyle(ta);
    if (cs.display === 'none' || cs.visibility === 'hidden') continue;
    const rect = ta.getBoundingClientRect();
    const opacity = Number.parseFloat(cs.opacity === '' ? '1' : cs.opacity);
    const bg = (cs.backgroundColor || '').trim();
    const hasBg = bg !== '' && bg !== 'transparent' && bg !== 'rgba(0, 0, 0, 0)';
    const outlineW = Number.parseFloat(cs.outlineWidth || '0');
    const hasOutline = cs.outlineStyle !== '' && cs.outlineStyle !== 'none' && outlineW > 0;
    const hasShadow = cs.boxShadow !== '' && cs.boxShadow !== 'none';
    if (rect.width > 1 || rect.height > 1 || opacity > 0.01 || hasBg || hasOutline || hasShadow) {
      xtermHelperLeakCount++;
      pushSample(
        `xterm-helper-leak:${Math.round(rect.width)}x${Math.round(rect.height)}:op=${opacity}:bg=${bg}`,
      );
    }
  }

  // 3. Proportional font family & M-vs-i fixed-width glyph drift check
  const proportionalRe = /(palatino|book antiqua|cinzel|georgia|times|garamond)/i;
  const isUnmockedJsdom =
    /jsdom/i.test(win.navigator?.userAgent || '') &&
    !(win.HTMLCanvasElement && 'mock' in win.HTMLCanvasElement.prototype.getContext);
  let measureCtx: CanvasRenderingContext2D | null = null;
  if (!isUnmockedJsdom) {
    try {
      measureCtx = rootDoc.createElement('canvas').getContext('2d');
    } catch {
      measureCtx = null;
    }
  }
  let terminalFontProportionalLeakCount = 0;
  const monoTargets = Array.from(
    rootDoc.querySelectorAll<HTMLElement>(
      '.xterm, [data-terminal-mono="true"], [data-testid="term-block-output"], [data-testid="chat-term-block-output"], [aria-label="console input"]',
    ),
  );
  for (const el of monoTargets) {
    if (!isVisible(el)) continue;
    const cs = win.getComputedStyle(el);
    const ff = cs.fontFamily || '';
    let leaked = proportionalRe.test(ff);
    let delta = 0;
    if (!leaked && measureCtx && typeof measureCtx.measureText === 'function') {
      measureCtx.font = cs.font || `${cs.fontSize || '13px'} ${ff || 'monospace'}`;
      const wM = measureCtx.measureText('MMMMMMMMMM').width;
      const wI = measureCtx.measureText('iiiiiiiiii').width;
      delta = Math.abs(wM - wI);
      if ((wM > 0 || wI > 0) && delta > 0.1) leaked = true;
    }
    if (leaked) {
      terminalFontProportionalLeakCount++;
      pushSample(`font-leak:${el.tagName.toLowerCase()}:ff=${ff}:delta=${delta.toFixed(2)}`);
    }
  }

  // 4. WCAG 2.1 relative luminance contrast check across [data-terminal-well="true"] and .xterm
  let colorCtx: CanvasRenderingContext2D | null | undefined;
  const getColorCtx = (): CanvasRenderingContext2D | null => {
    if (colorCtx !== undefined) return colorCtx;
    if (isUnmockedJsdom) {
      colorCtx = null;
      return null;
    }
    try {
      const c = rootDoc.createElement('canvas');
      c.width = 1;
      c.height = 1;
      const ctx = c.getContext('2d');
      colorCtx =
        ctx && typeof ctx.fillRect === 'function' && typeof ctx.getImageData === 'function' ? ctx : null;
    } catch {
      colorCtx = null;
    }
    return colorCtx;
  };

  const parseColor = (raw: string): [number, number, number, number] | null => {
    const s = (raw || '').trim();
    if (!s || s === 'transparent') return [0, 0, 0, 0];
    const rgbaMatch = s.match(
      /^rgba?\(\s*([\d.]+)\s*[,\s]\s*([\d.]+)\s*[,\s]\s*([\d.]+)(?:\s*[,/]\s*([\d.%]+))?\s*\)$/i,
    );
    if (rgbaMatch) {
      const aRaw = rgbaMatch[4];
      const a =
        aRaw === undefined
          ? 1
          : aRaw.endsWith('%')
            ? Number.parseFloat(aRaw) / 100
            : Number.parseFloat(aRaw);
      return [
        Number.parseFloat(rgbaMatch[1]!),
        Number.parseFloat(rgbaMatch[2]!),
        Number.parseFloat(rgbaMatch[3]!),
        Number.isFinite(a) ? a : 1,
      ];
    }
    if (s.startsWith('#')) {
      const hex = s.slice(1);
      if (hex.length === 3) {
        return [
          Number.parseInt(hex[0]! + hex[0]!, 16),
          Number.parseInt(hex[1]! + hex[1]!, 16),
          Number.parseInt(hex[2]! + hex[2]!, 16),
          1,
        ];
      }
      if (hex.length >= 6) {
        const a = hex.length >= 8 ? Number.parseInt(hex.slice(6, 8), 16) / 255 : 1;
        return [
          Number.parseInt(hex.slice(0, 2), 16),
          Number.parseInt(hex.slice(2, 4), 16),
          Number.parseInt(hex.slice(4, 6), 16),
          a,
        ];
      }
    }
    const ctx = getColorCtx();
    if (ctx) {
      try {
        ctx.clearRect(0, 0, 1, 1);
        ctx.fillStyle = s;
        ctx.fillRect(0, 0, 1, 1);
        const d = ctx.getImageData(0, 0, 1, 1).data;
        return [d[0] ?? 0, d[1] ?? 0, d[2] ?? 0, (d[3] ?? 255) / 255];
      } catch {
        return null;
      }
    }
    return null;
  };

  const relLum = (rgb: [number, number, number]): number => {
    const lin = (c: number) => {
      const v = Math.max(0, Math.min(255, c)) / 255;
      return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2]);
  };

  const resolveBg = (el: HTMLElement): [number, number, number] => {
    const layers: Array<[number, number, number, number]> = [];
    let cur: HTMLElement | null = el;
    while (cur) {
      const parsed = parseColor(win.getComputedStyle(cur).backgroundColor);
      if (parsed && parsed[3] > 0.001) {
        layers.push(parsed);
        if (parsed[3] >= 0.999) break;
      }
      cur = cur.parentElement;
    }
    let bg: [number, number, number] = [21, 17, 13];
    for (let i = layers.length - 1; i >= 0; i--) {
      const l = layers[i]!;
      const a = Math.max(0, Math.min(1, l[3]));
      bg = [l[0] * a + bg[0] * (1 - a), l[1] * a + bg[1] * (1 - a), l[2] * a + bg[2] * (1 - a)];
    }
    return bg;
  };

  const theme = (rootDoc.documentElement.getAttribute('data-theme') || 'arcane').toLowerCase();
  const minRatio = theme === 'high-contrast' ? 7.0 : 4.5;
  let terminalContrastViolationCount = 0;
  const visited = new Set<HTMLElement>();
  for (const well of Array.from(rootDoc.querySelectorAll<HTMLElement>('[data-terminal-well="true"], .xterm'))) {
    const rowsEl = well.classList.contains('xterm') ? well.querySelector<HTMLElement>('.xterm-rows') : null;
    if (rowsEl && isVisible(well) && (rowsEl.textContent || '').trim().length === 0) {
      terminalContrastViolationCount++;
      pushSample('xterm-blank-rows');
    }
    const candidates = [
      well,
      ...Array.from(well.querySelectorAll<HTMLElement>('span, code, pre, button, td, th, p, div, label, a')),
    ];
    for (const el of candidates) {
      if (visited.has(el)) continue;
      visited.add(el);
      if ((el as HTMLButtonElement).disabled || el.getAttribute('aria-disabled') === 'true') continue;
      const hasDirectText = Array.from(el.childNodes).some(
        (n) => n.nodeType === 3 && (n.nodeValue || '').trim().length > 0,
      );
      if (!hasDirectText || !isVisible(el)) continue;
      const fgStyle = win.getComputedStyle(el);
      const fgParsed = parseColor(fgStyle.color);
      if (!fgParsed || fgParsed[3] <= 0.05) continue;
      const bg = resolveBg(el);
      const rawOpacity = Number.parseFloat(fgStyle.opacity === '' ? '1' : fgStyle.opacity);
      const op = Number.isFinite(rawOpacity) ? Math.max(0, Math.min(1, rawOpacity)) : 1;
      const a = Math.max(0, Math.min(1, fgParsed[3] * op));
      const fg: [number, number, number] = [
        fgParsed[0] * a + bg[0] * (1 - a),
        fgParsed[1] * a + bg[1] * (1 - a),
        fgParsed[2] * a + bg[2] * (1 - a),
      ];
      const l1 = relLum(fg);
      const l2 = relLum(bg);
      const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
      if (ratio < minRatio) {
        terminalContrastViolationCount++;
        pushSample(`contrast:${ratio.toFixed(2)}<${minRatio}:${(el.textContent || '').trim().slice(0, 24)}`);
      }
    }
  }

  return {
    terminalStartupSpamCount,
    xtermHelperLeakCount,
    terminalFontProportionalLeakCount,
    terminalContrastViolationCount,
    samples,
  };
}

export async function runTerminalDomAudits(page: Page): Promise<TerminalAuditMetrics> {
  return page.evaluate(collectTerminalAuditMetrics);
}
