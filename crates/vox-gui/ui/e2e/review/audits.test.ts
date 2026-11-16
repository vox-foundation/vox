// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import type { Page } from '@playwright/test';
import { auditDomQualityInPage, auditIconsInPage, auditOverflowInPage } from './audits';
import { extractChromaticSpatialVisionSignals, inferPlateMimeType } from './vlmVisionFlywheel';

describe('auditIconsInPage', () => {
  it('flags zero-size svgs, drawless svgs, and broken imgs; passes healthy ones', () => {
    document.body.innerHTML = `
      <svg id="ok"><path d="M0 0h16v16z"/></svg>
      <svg id="drawless"></svg>
      <svg id="zero"><path d="M0 0h16v16z"/></svg>
      <img id="broken" src="x.png" alt="icon" />
    `;
    // jsdom rects are 0x0 — emulate rendered sizes explicitly.
    const rect16 = () => ({ width: 16, height: 16 }) as DOMRect;
    (document.getElementById('ok') as any).getBoundingClientRect = rect16;
    (document.getElementById('drawless') as any).getBoundingClientRect = rect16;
    // 'zero' keeps the 0x0 default -> zero-size branch.
    // jsdom imgs: complete=false by default -> force the loaded-but-broken shape.
    const broken = document.getElementById('broken') as HTMLImageElement;
    Object.defineProperty(broken, 'complete', { value: true });
    // naturalWidth is 0 in jsdom already.

    const issues = auditIconsInPage();
    expect(issues.some((i) => i.kind === 'empty-svg' && i.id === 'drawless')).toBe(true);
    expect(issues.some((i) => i.kind === 'zero-size-svg' && i.id === 'zero')).toBe(true);
    expect(issues.some((i) => i.kind === 'broken-img' && i.id === 'broken')).toBe(true);
    expect(issues.some((i) => i.id === 'ok')).toBe(false);
  });
});

describe('auditOverflowInPage', () => {
  it('reports body horizontal overflow', () => {
    Object.defineProperty(document.body, 'scrollWidth', { value: 1600, configurable: true });
    Object.defineProperty(document.body, 'clientWidth', { value: 1440, configurable: true });
    expect(auditOverflowInPage().bodyHorizontalOverflowPx).toBe(160);
  });
});

describe('auditDomQualityInPage', () => {
  it('detects light-theme contrast leaks, overlapping CTAs, and unstyled selects', () => {
    document.documentElement.dataset.theme = 'obsidian';
    document.body.innerHTML = `
      <main>
        <div id="leak" style="background-color: rgb(238, 237, 254); color: rgb(244, 244, 245)">Leak</div>
        <button data-testid="open-chat-cta">Open Chat</button>
        <button aria-expanded="false" id="customize">Customize</button>
        <select id="styled" data-unified-knob="true"><option>A</option></select>
      </main>
    `;
    const rectOverlap = () => ({ left: 100, right: 220, top: 20, bottom: 52, width: 120, height: 32 }) as DOMRect;
    (document.getElementById('leak') as any).getBoundingClientRect = rectOverlap;
    (document.querySelector('[data-testid="open-chat-cta"]') as any).getBoundingClientRect = rectOverlap;
    (document.getElementById('customize') as any).getBoundingClientRect = rectOverlap;
    (document.getElementById('styled') as any).getBoundingClientRect = rectOverlap;

    const report = auditDomQualityInPage('dashboard');
    expect(report.lightThemeContrastLeakCount).toBe(1);
    expect(report.overlappingControlPairs).toBe(1);
    expect(report.unstyledNativeSelectCount).toBe(0);
  });

  it('ignores scroll-clipped off-screen aria-expanded controls while catching visible overlaps', () => {
    document.documentElement.dataset.theme = 'slate';
    document.body.innerHTML = `
      <main>
        <div id="pane-scroll" style="overflow-y: auto">
          <button id="clipped-a" aria-expanded="false">Correct</button>
        </div>
        <div id="rail-scroll" style="overflow-y: auto">
          <button id="clipped-b" aria-expanded="true">Runbook</button>
        </div>
        <button id="visible-a" aria-expanded="false">Visible A</button>
        <button id="visible-b" aria-expanded="false">Visible B</button>
      </main>
    `;
    const boxRect = () => ({ left: 0, right: 900, top: 350, bottom: 540, width: 900, height: 190 }) as DOMRect;
    (document.getElementById('pane-scroll') as any).getBoundingClientRect = boxRect;
    (document.getElementById('rail-scroll') as any).getBoundingClientRect = boxRect;
    (document.getElementById('clipped-a') as any).getBoundingClientRect = () =>
      ({ left: 777, right: 853, top: 758, bottom: 782, width: 76, height: 24 }) as DOMRect;
    (document.getElementById('clipped-b') as any).getBoundingClientRect = () =>
      ({ left: 646, right: 857, top: 753, bottom: 789, width: 211, height: 36 }) as DOMRect;
    (document.getElementById('visible-a') as any).getBoundingClientRect = () =>
      ({ left: 100, right: 220, top: 120, bottom: 150, width: 120, height: 30 }) as DOMRect;
    (document.getElementById('visible-b') as any).getBoundingClientRect = () =>
      ({ left: 110, right: 230, top: 125, bottom: 155, width: 120, height: 30 }) as DOMRect;

    const rehydrated = new Function(`return (${auditDomQualityInPage.toString()});`)() as typeof auditDomQualityInPage;
    expect(rehydrated('console').overlappingControlPairs).toBe(1);

    (document.getElementById('visible-b') as any).getBoundingClientRect = () =>
      ({ left: 300, right: 420, top: 125, bottom: 155, width: 120, height: 30 }) as DOMRect;
    expect(rehydrated('console').overlappingControlPairs).toBe(0);
  });

  it('flags Tailwind v4 oklch() neon cyan-400 and emerald-400 digital color leaks', () => {
    document.documentElement.dataset.theme = 'arcane';
    document.body.innerHTML = `
      <main>
        <span id="cyan-oklch" style="color: oklch(0.789 0.154 211.53)">Cyan400</span>
        <span id="cyan-pct" style="background-color: oklch(78.9% 0.154 211.53)">CyanBg</span>
        <span id="emerald-oklch" style="color: oklch(0.765 0.177 163.223)">Emerald400</span>
        <span id="cyan-rgb" style="color: rgb(34, 211, 238)">LegacyCyan</span>
      </main>
    `;
    const rect = () => ({ width: 80, height: 24, left: 0, right: 80, top: 0, bottom: 24 }) as DOMRect;
    for (const id of ['cyan-oklch', 'cyan-pct', 'emerald-oklch', 'cyan-rgb']) {
      (document.getElementById(id) as any).getBoundingClientRect = rect;
    }

    const report = auditDomQualityInPage('oratio');
    expect(report.digitalColorLeakCount).toBe(4);
  });

  it('passes valid Romanesque (arcane) and Slate theme semantic tokens with 0 digital color leaks', () => {
    const rect = () => ({ width: 96, height: 24, left: 0, right: 96, top: 0, bottom: 24 }) as DOMRect;

    // 1. Romanesque Carrara / Arcane semantic tokens (brass, laurel/sage, verdigris, terracotta, oxblood)
    document.documentElement.dataset.theme = 'arcane';
    document.body.innerHTML = `
      <main>
        <span id="r1" style="color: oklch(0.7665 0.1387 91.06); background-color: #FAF6EE">Brass</span>
        <span id="r2" style="color: oklch(0.5636 0.132 152.79)">RomanEmerald500</span>
        <span id="r3" style="color: oklch(0.4678 0.1091 152.01)">RomanStatusPass</span>
        <span id="r4" style="color: oklch(0.6427 0.0851 180.17)">Verdigris400</span>
        <span id="r5" style="color: oklch(0.7 0.15 140)">ContextSafe</span>
        <span id="r6" style="color: oklch(0.4867 0.1146 55.99)">TerracottaWarn</span>
        <span id="r7" style="color: oklch(0.4683 0.1522 27.66)">OxbloodDanger</span>
      </main>
    `;
    for (const id of ['r1', 'r2', 'r3', 'r4', 'r5', 'r6', 'r7']) {
      (document.getElementById(id) as any).getBoundingClientRect = rect;
    }
    expect(auditDomQualityInPage('oratio').digitalColorLeakCount).toBe(0);

    // 2. Slate / Void semantic tokens (basalt surfaces, sky accent, slate status pass/warn/fail/info)
    document.documentElement.dataset.theme = 'slate';
    document.body.innerHTML = `
      <main>
        <div id="s1" style="background-color: #15191c; color: #f1f5f9">BasaltCard</div>
        <span id="s2" style="color: oklch(0.7535 0.139 232.66)">SlateAccentSky</span>
        <span id="s3" style="color: #10b981">SlateStatusPass</span>
        <span id="s4" style="color: oklch(0.8369 0.1644 84.43)">SlateWarn</span>
        <span id="s5" style="color: oklch(0.7106 0.1661 22.22)">SlateFail</span>
        <span id="s6" style="color: oklch(0.7137 0.1434 254.62)">SlateInfo</span>
      </main>
    `;
    for (const id of ['s1', 's2', 's3', 's4', 's5', 's6']) {
      (document.getElementById(id) as any).getBoundingClientRect = rect;
    }
    expect(auditDomQualityInPage('populi').digitalColorLeakCount).toBe(0);
  });

  it('ignores .xterm-helper-textarea and exempts Dual-Surface [data-terminal-well="true"] / .xterm ANSI spans while staying self-contained', () => {
    document.documentElement.dataset.theme = 'arcane';
    document.body.innerHTML = `
      <main>
        <div id="well" data-terminal-well="true" style="background-color: rgb(21, 17, 13); color: rgb(245, 239, 226)">
          <textarea id="helper" class="xterm-helper-textarea" role="button"></textarea>
          <span id="ansi-cyan" style="color: rgb(34, 211, 238)">ANSI Cyan</span>
          <span id="ansi-inv" style="background-color: rgb(229, 229, 229); color: rgb(244, 244, 245)">Inverse</span>
        </div>
        <div id="xterm-host" class="xterm">
          <span id="xterm-green" style="color: rgb(110, 231, 183)">ANSI Green</span>
        </div>
      </main>
    `;
    const rect = () => ({ width: 96, height: 24, left: 0, right: 96, top: 0, bottom: 24 }) as DOMRect;
    for (const id of ['well', 'helper', 'ansi-cyan', 'ansi-inv', 'xterm-host', 'xterm-green']) {
      (document.getElementById(id) as any).getBoundingClientRect = rect;
    }

    const rehydrated = new Function(`return (${auditDomQualityInPage.toString()});`)() as typeof auditDomQualityInPage;
    const report = rehydrated('console');
    expect(report.lightThemeContrastLeakCount).toBe(0);
    expect(report.digitalColorLeakCount).toBe(0);
    expect(report.clippedInteractiveCount).toBe(0);
  });

  it('flags interactive buttons clipped horizontally by overflow:hidden ancestors', () => {
    document.documentElement.dataset.theme = 'arcane';
    document.body.innerHTML = `
      <main>
        <div id="split-leaf" style="overflow: hidden;">
          <button id="clipped-close">Close Terminal Pane</button>
        </div>
      </main>
    `;
    (document.getElementById('split-leaf') as any).getBoundingClientRect = () =>
      ({ left: 20, right: 320, top: 20, bottom: 200, width: 300, height: 180 }) as DOMRect;
    (document.getElementById('clipped-close') as any).getBoundingClientRect = () =>
      ({ left: 250, right: 365, top: 28, bottom: 52, width: 115, height: 24 }) as DOMRect;
    const rehydrated = new Function(`return (${auditDomQualityInPage.toString()});`)() as typeof auditDomQualityInPage;
    expect(rehydrated('console').clippedInteractiveCount).toBe(1);
  });
});

describe('vlmVisionFlywheel', () => {
  it('throws an explicit error when a concept-art reference plate is missing and infers MIME types', async () => {
    expect(inferPlateMimeType('foo/plate.png')).toBe('image/png');
    expect(inferPlateMimeType('foo/plate.webp')).toBe('image/webp');
    expect(inferPlateMimeType('foo/plate.jpg')).toBe('image/jpeg');

    const fakePage = { evaluate: async () => ({}) } as unknown as Page;
    await expect(
      extractChromaticSpatialVisionSignals(
        fakePage,
        Buffer.from('89504e47', 'hex'),
        'public/assets/romanesque/nonexistent-plate-404.jpg',
      ),
    ).rejects.toThrow(/Missing concept-art reference plate:/);
  });
});
