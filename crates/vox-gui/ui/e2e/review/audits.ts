/**
 * Per-capture in-page audits. Each function is passed to page.evaluate(fn)
 * — Playwright serializes the SOURCE, so keep them fully self-contained.
 */

export interface IconIssue {
  kind: 'zero-size-svg' | 'empty-svg' | 'broken-img';
  id: string;
  testid: string;
  selectorHint: string;
}

export function auditIconsInPage(): IconIssue[] {
  const issues: IconIssue[] = [];
  // getAttribute('class'): el.className is an SVGAnimatedString on SVG.
  const hint = (el: Element) =>
    `${el.tagName.toLowerCase()}${el.id ? `#${el.id}` : ''}.${(el.getAttribute('class') || '').split(/\s+/)[0]}`;
  for (const svg of Array.from(document.querySelectorAll('svg'))) {
    const r = svg.getBoundingClientRect();
    const drawable = svg.querySelector('path, circle, rect, line, polyline, polygon, use, text');
    if (r.width === 0 || r.height === 0) {
      issues.push({ kind: 'zero-size-svg', id: svg.id, testid: svg.getAttribute('data-testid') ?? '', selectorHint: hint(svg) });
    } else if (!drawable) {
      issues.push({ kind: 'empty-svg', id: svg.id, testid: svg.getAttribute('data-testid') ?? '', selectorHint: hint(svg) });
    }
  }
  for (const img of Array.from(document.querySelectorAll('img'))) {
    if (img.complete && img.naturalWidth === 0) {
      issues.push({ kind: 'broken-img', id: img.id, testid: img.getAttribute('data-testid') ?? '', selectorHint: hint(img) });
    }
  }
  return issues;
}

export interface OverflowReport {
  bodyHorizontalOverflowPx: number;
  scrollHostHorizontalOverflowPx: number;
  contentHeightPx: number;
}

export function auditOverflowInPage(): OverflowReport {
  const body = document.body;
  const host = document.querySelector('[data-testid="surface-scroll-host"]') as HTMLElement | null;
  return {
    bodyHorizontalOverflowPx: Math.max(0, body.scrollWidth - body.clientWidth),
    scrollHostHorizontalOverflowPx: host ? Math.max(0, host.scrollWidth - host.clientWidth) : 0,
    contentHeightPx: Math.max(body.scrollHeight, document.documentElement.scrollHeight),
  };
}

export interface DomQualityReport {
  outerOverflowPx: number;
  scrollHostOverflowPx: number;
  clippedInteractiveCount: number;
  overlappingControlPairs: number;
  unstyledNativeSelectCount: number;
  lightThemeContrastLeakCount: number;
  controlHeightSpreadPx: number;
  rawEmojiCount: number;
  unlitLandmarkCount: number;
  missingDockPanelIds: string[];
  viewportCoverageRatio: number;
}

/**
 * Self-contained in-page DOM quality audit for dynamic all-surface CV Flywheel
 * and 6-pillar UI review (`page.evaluate(auditDomQualityInPage, surfaceId)`).
 */
export function auditDomQualityInPage(surfId: string): DomQualityReport {
  const docEl = document.documentElement;
  const outerOverflowPx = Math.max(0, Math.round(docEl.scrollWidth - window.innerWidth));
  const scrollHost =
    (document.querySelector('[data-testid="surface-scroll-host"]') as HTMLElement | null) ??
    (document.querySelector('main') as HTMLElement | null);
  const scrollHostOverflowPx = scrollHost
    ? Math.max(0, Math.round(scrollHost.scrollWidth - scrollHost.clientWidth))
    : 0;

  const visibleButtons = Array.from(
    document.querySelectorAll<HTMLElement>('button, select, [role="button"], [role="radio"]'),
  ).filter((el) => {
    const style = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    return (
      style.display !== 'none' &&
      style.visibility !== 'hidden' &&
      Number(style.opacity) > 0 &&
      r.width > 2 &&
      r.height > 2
    );
  });

  let clippedInteractiveCount = 0;
  for (const el of visibleButtons) {
    const style = window.getComputedStyle(el);
    const inDialog = Boolean(el.closest('[role="dialog"]'));
    if (
      ((style.overflowX === 'hidden' || style.overflowX === 'clip') &&
        el.scrollWidth > el.clientWidth + 4 &&
        style.textOverflow !== 'ellipsis') ||
      (inDialog && el.scrollWidth > el.clientWidth + 4)
    ) {
      clippedInteractiveCount++;
    }
  }

  // Page-wide unstyled native <select> check
  const allSelects = Array.from(document.querySelectorAll('select')).filter((s) => {
    const st = window.getComputedStyle(s);
    const r = s.getBoundingClientRect();
    return st.display !== 'none' && st.visibility !== 'hidden' && r.width > 4 && r.height > 4;
  });
  let unstyledNativeSelectCount = 0;
  for (const s of allSelects) {
    const st = window.getComputedStyle(s);
    if (st.appearance !== 'none' && !s.dataset.unifiedKnob) {
      unstyledNativeSelectCount++;
    }
  }

  // Light-theme pastel contrast leak check on dark theme
  let lightThemeContrastLeakCount = 0;
  if (document.documentElement.dataset.theme !== 'travertine') {
    const mainEls = Array.from(document.querySelectorAll<HTMLElement>('main span, main div, main button'));
    for (const el of mainEls) {
      const r = el.getBoundingClientRect();
      if (r.width < 20 || r.height < 14) continue;
      if ((el.textContent || '').trim().length === 0) continue;
      if (el.closest('.react-flow__controls, [data-theme-preview]')) continue;
      const style = window.getComputedStyle(el);
      const bg = style.backgroundColor;
      const fg = style.color;
      const m = bg.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
      const fgMatch = fg.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
      if (m) {
        const red = Number(m[1]);
        const green = Number(m[2]);
        const blue = Number(m[3]);
        const alpha = m[4] !== undefined ? Number(m[4]) : 1;
        // Flag light pastel card/pill backgrounds (>215 RGB) that aren't intentional high-contrast badges with dark text
        const fgLuma = fgMatch
          ? 0.2126 * Number(fgMatch[1]) + 0.7152 * Number(fgMatch[2]) + 0.0722 * Number(fgMatch[3])
          : 255;
        if (alpha > 0.65 && red > 215 && green > 215 && blue > 215 && fgLuma > 60) {
          lightThemeContrastLeakCount++;
        }
      }
    }
  }

  // Measure composer toolbar control height consistency & button overlap across surface headers
  const composerRoot = document.querySelector('[data-testid="loquela-composer"]');
  let controlHeightSpreadPx = 0;
  let overlappingControlPairs = 0;

  if (composerRoot) {
    const barControls = Array.from(
      composerRoot.querySelectorAll<HTMLElement>('[data-loquela-toolbar-item="true"]'),
    ).filter((el) => {
      const r = el.getBoundingClientRect();
      return r.width > 8 && r.height > 8 && !el.closest('[role="dialog"]');
    });

    if (barControls.length >= 2) {
      const heights = barControls.map((c) => Math.round(c.getBoundingClientRect().height));
      controlHeightSpreadPx = Math.max(...heights) - Math.min(...heights);

      for (let i = 0; i < barControls.length; i++) {
        for (let j = i + 1; j < barControls.length; j++) {
          const a = barControls[i]!.getBoundingClientRect();
          const b = barControls[j]!.getBoundingClientRect();
          if (barControls[i]!.contains(barControls[j]!) || barControls[j]!.contains(barControls[i]!)) {
            continue;
          }
          const ix = Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
          const iy = Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
          if (ix > 2 && iy > 2) {
            overlappingControlPairs++;
          }
        }
      }
    }
  }

  // Check for overlapping interactive CTAs in main surface area (e.g. floating Customize vs Open Chat)
  const mainCtas = Array.from(
    document.querySelectorAll<HTMLElement>(
      'main [data-testid="open-chat-cta"], main button[aria-expanded]',
    ),
  ).filter((el) => {
    const r = el.getBoundingClientRect();
    return r.width > 8 && r.height > 8;
  });
  for (let i = 0; i < mainCtas.length; i++) {
    for (let j = i + 1; j < mainCtas.length; j++) {
      const a = mainCtas[i]!.getBoundingClientRect();
      const b = mainCtas[j]!.getBoundingClientRect();
      if (mainCtas[i]!.contains(mainCtas[j]!) || mainCtas[j]!.contains(mainCtas[i]!)) continue;
      const ix = Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
      const iy = Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
      if (ix > 2 && iy > 2) {
        overlappingControlPairs++;
      }
    }
  }

  // Raw emoji audit (`🧍` inside Urbs/Chat HUD)
  const bodyText = document.body.innerText || '';
  const rawEmojiCount = (bodyText.match(/🧍/g) ?? []).length;

  // Urbs unlit landmark count
  const landmarkBadges = Array.from(document.querySelectorAll('[data-urbs-landmark-state]'));
  let unlitLandmarkCount = landmarkBadges.filter(
    (el) => el.getAttribute('data-urbs-landmark-state') === 'unlit',
  ).length;
  if (
    (surfId === 'urbs-civilization' || surfId === 'gamify') &&
    landmarkBadges.length === 0
  ) {
    const hasCitizenSprites = document.querySelectorAll('[data-testid="urbs-citizen-sprite"]').length > 0;
    if (!hasCitizenSprites) {
      unlitLandmarkCount = 2;
    }
  }

  // Check missing dock panel IDs when Panels menu is open
  const missingDockPanelIds: string[] = [];
  const panelsMenu = document.querySelector('[aria-label="Workspace panels"]');
  if (panelsMenu) {
    const menuText = (panelsMenu.textContent || '').toLowerCase();
    if (!menuText.includes('terminal')) missingDockPanelIds.push('terminal');
    if (!menuText.includes('civilization') && !menuText.includes('urbs')) {
      missingDockPanelIds.push('urbs');
    }
  }

  const mainRect = document.querySelector('main')?.getBoundingClientRect();
  const viewportArea = Math.max(1, window.innerWidth * window.innerHeight);
  const coveredArea = mainRect ? mainRect.width * mainRect.height : viewportArea * 0.75;
  const viewportCoverageRatio = Number(Math.min(1, coveredArea / viewportArea).toFixed(3));

  return {
    outerOverflowPx,
    scrollHostOverflowPx,
    clippedInteractiveCount,
    overlappingControlPairs,
    unstyledNativeSelectCount,
    lightThemeContrastLeakCount,
    controlHeightSpreadPx,
    rawEmojiCount,
    unlitLandmarkCount,
    missingDockPanelIds,
    viewportCoverageRatio,
  };
}
