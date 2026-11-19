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
    if (
      svg.closest('details:not([open]), .sr-only, [hidden]') ||
      svg.classList.contains('react-flow__edges')
    ) {
      continue;
    }
    let hiddenByCss = false;
    let cur: Element | null = svg;
    while (cur) {
      if (window.getComputedStyle(cur).display === 'none') {
        hiddenByCss = true;
        break;
      }
      cur = cur.parentElement;
    }
    const r = svg.getBoundingClientRect();
    const drawable = svg.querySelector('path, circle, ellipse, rect, line, polyline, polygon, use, text');
    if (!drawable) {
      issues.push({ kind: 'empty-svg', id: svg.id, testid: svg.getAttribute('data-testid') ?? '', selectorHint: hint(svg) });
    } else if (!hiddenByCss && (r.width === 0 || r.height === 0)) {
      issues.push({ kind: 'zero-size-svg', id: svg.id, testid: svg.getAttribute('data-testid') ?? '', selectorHint: hint(svg) });
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
  composerToolbarRowCount: number;
  composerToolbarHeightPx: number;
  duplicateControlGroupCount: number;
  skeletonCount: number;
  rawBacktickCount: number;
  digitalColorLeakCount: number;
  chartPanelFillRatio: number;
  bracketedLabelCount: number;
  statusBarClippedSegmentCount: number;
  sidebarTruncatedLabelCount: number;
  surfaceToolbarHeightSpreadPx: number;
  unreadableSmallFontCount: number;
  discoveredSubTabLabels: string[];
  discoveredPopoverTriggers: string[];
}

/**
 * Self-contained in-page DOM quality audit for dynamic all-surface CV Flywheel
 * and 6-pillar UI review (`page.evaluate(auditDomQualityInPage, surfaceId)`).
 */
export function auditDomQualityInPage(surfId: string): DomQualityReport {
  const docEl = document.documentElement;
  const isXtermHelper = (el: Element): boolean => el.classList.contains('xterm-helper-textarea');
  const isTerminalWellEl = (el: Element): boolean =>
    isXtermHelper(el) || Boolean(el.closest('.xterm, [data-terminal-well="true"]'));
  const outerOverflowPx = Math.max(0, Math.round(docEl.scrollWidth - window.innerWidth));
  const scrollHost =
    (document.querySelector('[data-testid="surface-scroll-host"]') as HTMLElement | null) ??
    (document.querySelector('main') as HTMLElement | null);
  const scrollHostOverflowPx = scrollHost ? Math.max(0, Math.round(scrollHost.scrollWidth - scrollHost.clientWidth)) : 0;

  const visibleButtons = Array.from(
    document.querySelectorAll<HTMLElement>('button, select, [role="button"], [role="radio"]'),
  ).filter((el) => {
    if (isXtermHelper(el)) return false;
    const style = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    const opacity = style.opacity === '' ? 1 : Number(style.opacity);
    return style.display !== 'none' && style.visibility !== 'hidden' && opacity > 0 && r.width > 2 && r.height > 2;
  });

  let clippedInteractiveCount = 0;
  for (const el of visibleButtons) {
    const style = window.getComputedStyle(el);
    const r = el.getBoundingClientRect();
    const inDialog = Boolean(el.closest('[role="dialog"]'));
    const railAncestor = el.closest('[data-testid="chat-execution-rail"], [data-testid="context-window-meter"]');
    const railRect = railAncestor ? railAncestor.getBoundingClientRect() : null;
    const overflowsRailRight = railRect != null && r.right > railRect.right + 2;
    const isTruncatedCompactButton =
      el.tagName === 'BUTTON' && r.width < 120 && el.scrollWidth > el.clientWidth + 4 && !el.hasAttribute('data-allow-ellipsis') && !el.closest('nav');
    let overflowsHiddenAncestor = false;
    if (!el.closest('nav')) {
      let anc: HTMLElement | null = el.parentElement;
      while (anc && anc !== document.body && anc !== document.documentElement) {
        const ancStyle = window.getComputedStyle(anc);
        const ovX = ancStyle.overflowX || ancStyle.overflow;
        if (ovX === 'hidden' || ovX === 'clip' || ancStyle.overflow === 'hidden' || ancStyle.overflow === 'clip') {
          const ancRect = anc.getBoundingClientRect();
          if (ancRect.width > 4 && (r.left < ancRect.left - 2 || r.right > ancRect.right + 2)) {
            overflowsHiddenAncestor = true;
            break;
          }
        }
        anc = anc.parentElement;
      }
    }
    if (
      ((style.overflowX === 'hidden' || style.overflowX === 'clip') && el.scrollWidth > el.clientWidth + 4 && style.textOverflow !== 'ellipsis') ||
      (inDialog && el.scrollWidth > el.clientWidth + 4) ||
      overflowsRailRight ||
      overflowsHiddenAncestor ||
      isTruncatedCompactButton
    ) {
      clippedInteractiveCount++;
    }
  }

  const allSelects = Array.from(document.querySelectorAll('select')).filter((s) => {
    if (isXtermHelper(s)) return false;
    const st = window.getComputedStyle(s);
    const r = s.getBoundingClientRect();
    return st.display !== 'none' && st.visibility !== 'hidden' && r.width > 4 && r.height > 4;
  });
  let unstyledNativeSelectCount = 0;
  for (const s of allSelects) {
    if (window.getComputedStyle(s).appearance !== 'none' && !s.dataset.unifiedKnob) unstyledNativeSelectCount++;
  }

  const activeTheme = (docEl.dataset.theme || 'arcane').toLowerCase();
  const isLightModeTheme =
    activeTheme === 'arcane' || activeTheme === 'glacier' || activeTheme === 'travertine';
  let lightThemeContrastLeakCount = 0;
  if (docEl.dataset.theme !== 'travertine') {
    for (const el of Array.from(document.querySelectorAll<HTMLElement>('main span, main div, main button, main p'))) {
      if (isTerminalWellEl(el)) continue;
      const r = el.getBoundingClientRect();
      if (r.width < 12 || r.height < 12 || (el.textContent || '').trim().length === 0) continue;
      if (el.closest('.react-flow__controls, [data-theme-preview], [data-roman-frieze="true"], [data-roman-gauge="true"], [data-roman-wax-seal="true"], [data-roman-medallion="true"], .vox-marble-tablet')) continue;
      const style = window.getComputedStyle(el);
      const m = style.backgroundColor.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
      const fgMatch = style.color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
      const fgRed = fgMatch ? Number(fgMatch[1]) : 255;
      const fgGreen = fgMatch ? Number(fgMatch[2]) : 255;
      const fgBlue = fgMatch ? Number(fgMatch[3]) : 255;
      const fgLuma = fgMatch ? 0.2126 * fgRed + 0.7152 * fgGreen + 0.0722 * fgBlue : 255;
      if (m) {
        const red = Number(m[1]), green = Number(m[2]), blue = Number(m[3]);
        const alpha = m[4] !== undefined ? Number(m[4]) : 1;
        const isWarmMarble = red >= green && green >= blue && red - blue >= 10;
        if (!isLightModeTheme && alpha > 0.65 && red > 215 && green > 215 && blue > 215 && !isWarmMarble && fgLuma > 110) {
          lightThemeContrastLeakCount++;
        } else if (isLightModeTheme && (alpha <= 0.1 || (0.2126 * red + 0.7152 * green + 0.0722 * blue) > 180)) {
          const isCreamHex =
            (fgRed === 243 && fgGreen === 229 && fgBlue === 171) ||
            (fgRed === 214 && fgGreen === 206 && fgBlue === 191) ||
            (fgRed === 200 && fgGreen === 185 && fgBlue === 158);
          if (isCreamHex) {
            lightThemeContrastLeakCount++;
          }
        }
      }
    }
  }

  const composerRoot = document.querySelector('[data-testid="loquela-composer"]');
  let controlHeightSpreadPx = 0, overlappingControlPairs = 0, composerToolbarRowCount = 0, composerToolbarHeightPx = 0;
  if (composerRoot) {
    const barControls = Array.from(composerRoot.querySelectorAll<HTMLElement>('[data-loquela-toolbar-item="true"]')).filter((el) => {
      const r = el.getBoundingClientRect();
      return r.width > 8 && r.height > 8 && !el.closest('[role="dialog"]');
    });
    if (barControls.length > 0) {
      const rects = barControls.map((c) => c.getBoundingClientRect());
      composerToolbarHeightPx = Math.round(Math.max(...rects.map((r) => r.bottom)) - Math.min(...rects.map((r) => r.top)));
      const rowTops: number[] = [];
      for (const r of rects) if (!rowTops.some((t) => Math.abs(t - r.top) <= 12)) rowTops.push(r.top);
      composerToolbarRowCount = rowTops.length;
    }
    if (barControls.length >= 2) {
      const heights = barControls.map((c) => Math.round(c.getBoundingClientRect().height));
      controlHeightSpreadPx = Math.max(...heights) - Math.min(...heights);
      for (let i = 0; i < barControls.length; i++) {
        for (let j = i + 1; j < barControls.length; j++) {
          if (barControls[i]!.contains(barControls[j]!) || barControls[j]!.contains(barControls[i]!)) continue;
          const a = barControls[i]!.getBoundingClientRect(), b = barControls[j]!.getBoundingClientRect();
          if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 2 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 2) {
            overlappingControlPairs++;
          }
        }
      }
    }
  }

  const visibleClippedRect = (el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    let left = r.left;
    let top = r.top;
    let right = r.right;
    let bottom = r.bottom;
    if (window.innerWidth > 0 && window.innerHeight > 0) {
      left = Math.max(left, 0);
      top = Math.max(top, 0);
      right = Math.min(right, window.innerWidth);
      bottom = Math.min(bottom, window.innerHeight);
    }
    for (let anc = el.parentElement; anc && anc !== document.body; anc = anc.parentElement) {
      const st = window.getComputedStyle(anc);
      const clipX = st.overflowX === 'auto' || st.overflowX === 'scroll' || st.overflowX === 'hidden' || st.overflowX === 'clip';
      const clipY = st.overflowY === 'auto' || st.overflowY === 'scroll' || st.overflowY === 'hidden' || st.overflowY === 'clip';
      if (!clipX && !clipY) continue;
      const ar = anc.getBoundingClientRect();
      if (ar.width <= 0 || ar.height <= 0) continue;
      if (clipX) {
        left = Math.max(left, ar.left);
        right = Math.min(right, ar.right);
      }
      if (clipY) {
        top = Math.max(top, ar.top);
        bottom = Math.min(bottom, ar.bottom);
      }
    }
    return { left, top, right, bottom, width: right - left, height: bottom - top };
  };

  const mainCtas = Array.from(document.querySelectorAll<HTMLElement>('main [data-testid="open-chat-cta"], main button[aria-expanded]'))
    .filter((el) => !isXtermHelper(el))
    .map((el) => ({ el, rect: visibleClippedRect(el) }))
    .filter(({ rect }) => rect.width > 8 && rect.height > 8);
  for (let i = 0; i < mainCtas.length; i++) {
    for (let j = i + 1; j < mainCtas.length; j++) {
      const { el: elA, rect: a } = mainCtas[i]!;
      const { el: elB, rect: b } = mainCtas[j]!;
      if (elA.contains(elB) || elB.contains(elA)) continue;
      if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 2 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 2) {
        overlappingControlPairs++;
      }
    }
  }

  const bannedGlyphRe =
    /[\p{Extended_Pictographic}\u{2600}-\u{27BF}\u{23F0}-\u{23FE}\u{25A0}-\u{25FF}\u{2197}\u{21BA}-\u{21C4}\u{21E2}\u{2B06}\u{2302}\u{2442}\u{2295}\u{21B5}\u{23CE}\u{21E5}]/gu;
  const glyphScanRoots = Array.from(
    document.querySelectorAll<HTMLElement>('main, header, footer, [data-radix-popper-content-wrapper], [role="dialog"], [role="listbox"], [role="menu"]'),
  );
  const isVisibleForGlyphScan = (el: HTMLElement): boolean => {
    if (el.classList.contains('sr-only') || isXtermHelper(el)) return false;
    const st = window.getComputedStyle(el);
    if (st.display === 'none' || st.visibility === 'hidden' || Number(st.opacity) === 0) return false;
    if (st.display === 'contents') return el.parentElement ? isVisibleForGlyphScan(el.parentElement) : false;
    const r = el.getBoundingClientRect();
    return r.width >= 4 && r.height >= 4;
  };
  let rawEmojiCount = 0;
  const scannedTextNodes = new Set<Node>();
  for (const root of glyphScanRoots) {
    for (const el of [root, ...Array.from(root.querySelectorAll<HTMLElement>('*'))]) {
      if (!isVisibleForGlyphScan(el)) continue;
      for (const child of Array.from(el.childNodes)) {
        if (child.nodeType !== Node.TEXT_NODE || scannedTextNodes.has(child)) continue;
        scannedTextNodes.add(child);
        const matches = (child.nodeValue || '').match(bannedGlyphRe);
        if (matches) rawEmojiCount += matches.length;
      }
    }
  }

  let bracketedLabelCount = 0;
  for (const btn of visibleButtons) {
    if (btn.tagName === 'BUTTON' && /^\[\s*[^\]]{2,}\s*\]$/.test((btn.textContent || '').trim())) bracketedLabelCount++;
  }

  let statusBarClippedSegmentCount = 0;
  const statusBarRoot = document.querySelector<HTMLElement>('[data-testid="bottom-status-bar"]') ?? document.querySelector<HTMLElement>('footer');
  if (statusBarRoot) {
    const scrollStrips = Array.from(statusBarRoot.querySelectorAll<HTMLElement>('div')).filter((d) => {
      const st = window.getComputedStyle(d);
      return st.overflowX === 'auto' || st.overflowX === 'scroll' || st.overflowX === 'hidden';
    });
    for (const strip of scrollStrips) {
      if (strip.clientWidth > 40 && strip.scrollWidth > strip.clientWidth + 4) statusBarClippedSegmentCount++;
      const sr = strip.getBoundingClientRect();
      if (sr.width <= 40) continue;
      for (const child of Array.from(strip.querySelectorAll<HTMLElement>('button'))) {
        const cr = child.getBoundingClientRect();
        if (cr.width > 2 && (cr.right > sr.right + 2 || cr.left < sr.left - 2)) statusBarClippedSegmentCount++;
      }
    }
  }

  let sidebarTruncatedLabelCount = 0;
  const navEl = document.querySelector<HTMLElement>('nav');
  if (navEl && navEl.getBoundingClientRect().width >= 120) {
    for (const lbl of Array.from(navEl.querySelectorAll<HTMLElement>('span, div, p'))) {
      if (lbl.children.length > 0 || (lbl.textContent || '').trim().length === 0) continue;
      if (lbl.hasAttribute('data-allow-ellipsis') || lbl.closest('[data-allow-ellipsis="true"]')) continue;
      if (lbl.getBoundingClientRect().width > 4 && lbl.scrollWidth > lbl.clientWidth + 3) sidebarTruncatedLabelCount++;
    }
  }

  const chartRatios: number[] = [];
  const braidSvg = document.querySelector<HTMLElement>('[data-testid="harness-agent-braid"]')?.querySelector('svg');
  if (braidSvg && braidSvg.getBoundingClientRect().width > 60) {
    const sr = braidSvg.getBoundingClientRect();
    const prs = Array.from(braidSvg.querySelectorAll('path')).map((p) => p.getBoundingClientRect()).filter((r) => r.width > 2);
    if (prs.length > 0) {
      chartRatios.push(Math.min(1, Math.max(0, (Math.max(...prs.map((r) => r.right)) - Math.min(...prs.map((r) => r.left))) / sr.width)));
    }
  }
  for (const card of Array.from(document.querySelectorAll<HTMLElement>('main .vox-marble-tablet, main [data-cv-kpi-card="true"]'))) {
    const cr = card.getBoundingClientRect();
    if (cr.width < 120 || cr.height < 36) continue;
    const sparkSvg = card.querySelector<SVGSVGElement>('svg[data-cv-sparkline="true"], svg[viewBox="0 0 60 18"]');
    if (sparkSvg && sparkSvg.getBoundingClientRect().width > 4) {
      chartRatios.push(Math.min(1, Math.max(0, sparkSvg.getBoundingClientRect().width / Math.max(1, cr.width - 16))));
    }
  }
  for (const ch of Array.from(document.querySelectorAll<HTMLElement>('main [data-cv-chart="true"]'))) {
    const cr = ch.getBoundingClientRect(), pr = ch.parentElement?.getBoundingClientRect();
    if (cr.width > 20 && pr && pr.width > 60) chartRatios.push(Math.min(1, Math.max(0, cr.width / Math.max(1, pr.width - 24))));
  }
  const chartPanelFillRatio = chartRatios.length > 0 ? Number((chartRatios.reduce((a, v) => a + v, 0) / chartRatios.length).toFixed(3)) : 1;

  const landmarkBadges = Array.from(document.querySelectorAll('[data-urbs-landmark-state]'));
  let unlitLandmarkCount = landmarkBadges.filter((el) => el.getAttribute('data-urbs-landmark-state') === 'unlit').length;
  if ((surfId === 'urbs-civilization' || surfId === 'gamify') && landmarkBadges.length === 0) {
    if (document.querySelectorAll('[data-testid="urbs-citizen-sprite"]').length === 0) unlitLandmarkCount = 2;
  }

  const missingDockPanelIds: string[] = [];
  const panelsMenu = document.querySelector('[aria-label="Workspace panels"]');
  if (panelsMenu) {
    const menuText = (panelsMenu.textContent || '').toLowerCase();
    if (!menuText.includes('terminal')) missingDockPanelIds.push('terminal');
    if (!menuText.includes('civilization') && !menuText.includes('urbs')) missingDockPanelIds.push('urbs');
  }

  const mainRect = document.querySelector('main')?.getBoundingClientRect();
  const viewportArea = Math.max(1, window.innerWidth * window.innerHeight);
  const viewportCoverageRatio = Number(Math.min(1, (mainRect ? mainRect.width * mainRect.height : viewportArea * 0.75) / viewportArea).toFixed(3));

  let duplicateControlGroupCount = 0;
  const checkDupPair = (selA: string, selB: string) => {
    const a = document.querySelector<HTMLElement>(selA), b = document.querySelector<HTMLElement>(selB);
    if (!a || !b) return;
    const r1 = a.getBoundingClientRect(), r2 = b.getBoundingClientRect();
    if (r1.width > 4 && r1.height > 4 && r2.width > 4 && r2.height > 4) duplicateControlGroupCount++;
  };
  checkDupPair('[data-testid="kb-context-inspector-toggle"]', '[data-testid="execution-rail-context-pins"]');
  checkDupPair('nav [data-testid="sidebar-child-list"]', '[data-testid="breadcrumb-child-tabs"]');

  const discoveredSubTabLabels = Array.from(document.querySelectorAll<HTMLElement>('main [role="tab"], main [data-cv-subsurface]'))
    .filter((el) => el.getBoundingClientRect().width > 4 && el.getBoundingClientRect().height > 4)
    .map((el) => (el.textContent || el.getAttribute('aria-label') || '').trim())
    .filter((l) => l.length > 0);

  const discoveredPopoverTriggers = Array.from(
    document.querySelectorAll<HTMLElement>('main [data-cv-popover-trigger], main button[aria-haspopup], [data-testid="loquela-composer"] button[aria-expanded]'),
  )
    .filter((el) => el.getBoundingClientRect().width > 4 && el.getBoundingClientRect().height > 4)
    .map((el) => (el.getAttribute('data-testid') || el.getAttribute('aria-label') || el.textContent || '').trim())
    .filter((l) => l.length > 0);

  let skeletonCount = 0;
  for (const el of Array.from(document.querySelectorAll<HTMLElement>('main [aria-busy="true"], main .animate-pulse'))) {
    const r = el.getBoundingClientRect();
    if (r.width > 24 && r.height >= 10 && (el.textContent || '').trim().length === 0) skeletonCount++;
  }
  if ((document.querySelector('main')?.innerText || '').includes('Loading the capability index')) skeletonCount += 1;

  let rawBacktickCount = 0;
  for (const el of Array.from(document.querySelectorAll<HTMLElement>('main button, main h1, main h2, main h3, main label, main td, aside button'))) {
    if (isXtermHelper(el) || el.closest('code, pre, [data-code-literal="true"], .xterm')) continue;
    const r = el.getBoundingClientRect();
    if (r.width > 4 && r.height > 4 && (el.textContent || '').includes('`')) rawBacktickCount++;
  }

  const isSlateTheme = docEl.dataset.theme === 'slate' || docEl.dataset.theme === 'void';
  const isNeonDigitalOklch = (val: string): boolean => {
    if (!val || !val.includes('oklch(')) return false;
    const m = val.match(/oklch\(\s*([\d.]+%?)\s+([\d.]+%?)\s+([\d.]+)(?:deg)?(?:\s*\/\s*([\d.]+%?))?\s*\)/i);
    if (!m) return false;
    const chroma = m[2]!.endsWith('%') ? (parseFloat(m[2]!) / 100) * 0.4 : parseFloat(m[2]!);
    const hue = ((parseFloat(m[3]!) % 360) + 360) % 360;
    const alpha = m[4] === undefined ? 1 : m[4].endsWith('%') ? parseFloat(m[4]) / 100 : parseFloat(m[4]);
    if (!Number.isFinite(chroma) || !Number.isFinite(hue) || alpha <= 0.2) return false;
    return chroma >= 0.14 && ((hue >= 192 && hue <= 212) || (hue >= 158 && hue <= 168));
  };

  const wavePaletteLeaks = new Set([
    'rgb(46, 16, 101)', 'rgb(30, 27, 75)', 'rgb(8, 51, 68)',
    'rgb(4, 47, 46)', 'rgb(6, 78, 59)', 'rgb(107, 33, 168)',
    'rgb(55, 48, 163)', 'rgb(14, 116, 144)', 'rgb(15, 118, 110)',
    'rgb(103, 232, 249)', 'rgb(196, 181, 253)', 'rgb(167, 139, 250)',
  ]);

  let digitalColorLeakCount = 0;
  for (const el of Array.from(document.querySelectorAll<Element>('main section, main article, main div, main button, main span, main svg rect'))) {
    if (isTerminalWellEl(el)) continue;
    const r = el.getBoundingClientRect();
    if (r.width < 16 || r.height < 12) continue;
    const st = window.getComputedStyle(el);
    const bg = st.backgroundColor, fg = st.color;
    const fillAttr = el.tagName.toLowerCase() === 'rect' ? (el.getAttribute('fill') || '').toLowerCase() : '';
    const hasWaveHex =
      wavePaletteLeaks.has(bg) || wavePaletteLeaks.has(fg) ||
      ['#2e1065', '#1e1b4b', '#083344', '#042f2e', '#064e3b', '#6b21a8', '#3730a3', '#0e7490'].includes(fillAttr);
    const isTranslucentBlackCard =
      /^rgba\(\s*0,\s*0,\s*0,\s*0\.[1-6]/.test(bg) && !el.classList.contains('fixed') && el.getAttribute('role') !== 'presentation';
    if (
      bg === 'rgb(12, 18, 27)' || bg === 'rgb(14, 19, 26)' ||
      fg === 'rgb(34, 211, 238)' || fg === 'rgb(6, 182, 212)' ||
      (!isSlateTheme && fg === 'rgb(16, 185, 129)') ||
      fg === 'rgb(52, 211, 153)' || fg === 'rgb(110, 231, 183)' ||
      hasWaveHex ||
      isTranslucentBlackCard ||
      isNeonDigitalOklch(fg) || isNeonDigitalOklch(bg)
    ) {
      digitalColorLeakCount++;
    }
  }

  let surfaceToolbarHeightSpreadPx = 0;
  const barSelectors = 'main [role="tablist"], main [role="group"], main [data-testid="kb-library-tabs"]';
  for (const bar of Array.from(document.querySelectorAll<HTMLElement>(barSelectors))) {
    const items = Array.from(bar.querySelectorAll<HTMLElement>('button, [role="tab"], input')).filter((el) => {
      if (isXtermHelper(el)) return false;
      const st = window.getComputedStyle(el);
      const r = el.getBoundingClientRect();
      return st.display !== 'none' && st.visibility !== 'hidden' && r.width > 12 && r.height > 8;
    });
    if (items.length >= 2) {
      const heights = items.map((c) => Math.round(c.getBoundingClientRect().height));
      const spread = Math.max(...heights) - Math.min(...heights);
      if (spread > surfaceToolbarHeightSpreadPx) surfaceToolbarHeightSpreadPx = spread;
    }
  }

  let unreadableSmallFontCount = 0;
  for (const el of Array.from(document.querySelectorAll<HTMLElement>('main button, main p, main td, main th, main [role="tab"]'))) {
    if (isTerminalWellEl(el) || el.closest('.sr-only, [data-allow-small-font="true"]')) continue;
    const r = el.getBoundingClientRect();
    if (r.width < 12 || r.height < 10 || (el.textContent || '').trim().length === 0) continue;
    const st = window.getComputedStyle(el);
    if (st.display === 'none' || st.visibility === 'hidden') continue;
    const fs = parseFloat(st.fontSize || '');
    if (Number.isFinite(fs) && fs > 0 && fs < 10.5) {
      unreadableSmallFontCount++;
    }
  }

  return {
    outerOverflowPx, scrollHostOverflowPx, clippedInteractiveCount, overlappingControlPairs,
    unstyledNativeSelectCount, lightThemeContrastLeakCount, controlHeightSpreadPx, rawEmojiCount,
    unlitLandmarkCount, missingDockPanelIds, viewportCoverageRatio, composerToolbarRowCount,
    composerToolbarHeightPx, duplicateControlGroupCount, skeletonCount, rawBacktickCount,
    digitalColorLeakCount, chartPanelFillRatio, bracketedLabelCount, statusBarClippedSegmentCount,
    sidebarTruncatedLabelCount, surfaceToolbarHeightSpreadPx, unreadableSmallFontCount,
    discoveredSubTabLabels, discoveredPopoverTriggers,
  };
}
