import { test, expect, type Page } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const STAGE = (process.env.PERF_STAGE ?? 'after').toLowerCase() === 'before' ? 'before' : 'after';
const SCRATCH_DIR =
  process.env.PERF_OUT_DIR ?? join(process.cwd(), 'test-results', 'perf');

interface ImgAudit {
  src: string;
  alt: string;
  hasWidthAttr: boolean;
  hasHeightAttr: boolean;
  attrWidth: number;
  attrHeight: number;
  naturalWidth: number;
  naturalHeight: number;
  renderedWidth: number;
  renderedHeight: number;
  loadingAttr: string | null;
  decodingAttr: string | null;
  fetchPriorityAttr: string | null;
  objectFit: string;
  complete: boolean;
  inInitialViewport: boolean;
  aspectDrift: number;
  attrAspectDrift: number;
}

interface RunMetrics {
  ttfbMs: number;
  fcpMs: number;
  lcpMs: number;
  domContentLoadedMs: number;
  loadEventMs: number;
  clsScore: number;
  eagerRequestCount: number;
  eagerTotalBytes: number;
  eagerImageBytes: number;
  htmlBytes: number;
  jsBytes: number;
  cssBytes: number;
  fontBytes: number;
  totalScrolledTransferBytes: number;
  totalScrolledImageBytes: number;
  clsAfterScroll: number;
  preloadCount: number;
  images: ImgAudit[];
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return Number(
    (sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid]).toFixed(2),
  );
}

async function measureFrontpageRun(
  page: Page,
  throttledWan: boolean,
): Promise<RunMetrics> {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.setCacheDisabled', { cacheDisabled: true });
  if (throttledWan) {
    // 10 Mbps down (1,250,000 B/s), 5 Mbps up, 40ms RTT
    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: 1_250_000,
      uploadThroughput: 625_000,
      latency: 40,
    });
  } else {
    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: -1,
      uploadThroughput: -1,
      latency: 0,
    });
  }

  // Track exact wire bytes transferred per request URL via CDP
  const wireByReqId = new Map<
    string,
    { url: string; mimeType: string; resourceType: string; bytes: number }
  >();
  let phaseScrolled = false;
  const eagerReqIds = new Set<string>();

  cdp.on('Network.requestWillBeSent', (ev) => {
    wireByReqId.set(ev.requestId, {
      url: ev.request.url,
      mimeType: '',
      resourceType: ev.type ?? 'Other',
      bytes: 0,
    });
    if (!phaseScrolled) eagerReqIds.add(ev.requestId);
  });
  cdp.on('Network.responseReceived', (ev) => {
    const entry = wireByReqId.get(ev.requestId);
    if (entry) {
      entry.mimeType = ev.response.mimeType ?? '';
      entry.resourceType = ev.type ?? entry.resourceType;
      if (ev.response.encodedDataLength > 0 && entry.bytes === 0) {
        entry.bytes = ev.response.encodedDataLength;
      }
    }
  });
  cdp.on('Network.loadingFinished', (ev) => {
    const entry = wireByReqId.get(ev.requestId);
    if (entry && ev.encodedDataLength > 0) {
      entry.bytes = ev.encodedDataLength;
    }
  });

  await page.addInitScript(() => {
    (window as unknown as Record<string, unknown>).__voxPerf = {
      lcp: 0,
      cls: 0,
    };
    try {
      new PerformanceObserver((list) => {
        const entries = list.getEntries();
        const last = entries[entries.length - 1];
        if (last) {
          ((window as unknown as Record<string, { lcp: number }>).__voxPerf).lcp = last.startTime;
        }
      }).observe({ type: 'largest-contentful-paint', buffered: true });
    } catch {}
    try {
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries() as Array<PerformanceEntry & { value?: number; hadRecentInput?: boolean }>) {
          if (!entry.hadRecentInput && typeof entry.value === 'number') {
            ((window as unknown as Record<string, { cls: number }>).__voxPerf).cls += entry.value;
          }
        }
      }).observe({ type: 'layout-shift', buffered: true });
    } catch {}
  });

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/', { waitUntil: 'load', timeout: 90_000 });
  await page.waitForTimeout(250);

  const initialEval = await page.evaluate(() => {
    const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined;
    const fcpEntry = performance.getEntriesByName('first-contentful-paint')[0];
    const perfBag = (window as unknown as { __voxPerf?: { lcp: number; cls: number } }).__voxPerf;
    const preloadCount = document.querySelectorAll('head link[rel="preload"][as="image"]').length;
    return {
      ttfbMs: nav ? Math.max(0, nav.responseStart - nav.requestStart) : 0,
      fcpMs: fcpEntry ? fcpEntry.startTime : 0,
      lcpMs: perfBag?.lcp ?? 0,
      domContentLoadedMs: nav ? Math.max(0, nav.domContentLoadedEventEnd - nav.fetchStart) : 0,
      loadEventMs: nav ? Math.max(0, nav.loadEventEnd - nav.fetchStart) : 0,
      clsScore: Number((perfBag?.cls ?? 0).toFixed(5)),
      preloadCount,
    };
  });

  let eagerRequestCount = 0;
  let eagerTotalBytes = 0;
  let eagerImageBytes = 0;
  let htmlBytes = 0;
  let jsBytes = 0;
  let cssBytes = 0;
  let fontBytes = 0;

  for (const reqId of eagerReqIds) {
    const rec = wireByReqId.get(reqId);
    if (!rec) continue;
    eagerRequestCount += 1;
    eagerTotalBytes += rec.bytes;
    const rt = rec.resourceType;
    if (rt === 'Image' || /\.(webp|png|jpe?g|svg|gif|avif)(\?|$)/i.test(rec.url)) {
      eagerImageBytes += rec.bytes;
    } else if (rt === 'Document') {
      htmlBytes += rec.bytes;
    } else if (rt === 'Script') {
      jsBytes += rec.bytes;
    } else if (rt === 'Stylesheet') {
      cssBytes += rec.bytes;
    } else if (rt === 'Font') {
      fontBytes += rec.bytes;
    }
  }

  // Trigger full scroll to measure full-page footprint + scroll CLS + inspect every image
  phaseScrolled = true;
  await page.evaluate(async () => {
    const imgs = Array.from(document.querySelectorAll('main img')) as HTMLImageElement[];
    for (const img of imgs) {
      img.scrollIntoView({ behavior: 'instant', block: 'center' });
      if (!img.complete) {
        await new Promise<void>((res) => {
          img.addEventListener('load', () => res(), { once: true });
          img.addEventListener('error', () => res(), { once: true });
          setTimeout(() => res(), 8000);
        });
      }
    }
    window.scrollTo(0, 0);
  });
  await page.waitForTimeout(300);

  let totalScrolledTransferBytes = 0;
  let totalScrolledImageBytes = 0;
  for (const rec of wireByReqId.values()) {
    totalScrolledTransferBytes += rec.bytes;
    if (rec.resourceType === 'Image' || /\.(webp|png|jpe?g|svg|gif|avif)(\?|$)/i.test(rec.url)) {
      totalScrolledImageBytes += rec.bytes;
    }
  }

  const afterScroll = await page.evaluate(() => {
    const perfBag = (window as unknown as { __voxPerf?: { cls: number } }).__voxPerf;
    const imgs = Array.from(document.querySelectorAll('main img')) as HTMLImageElement[];
    const audits: ImgAudit[] = imgs.map((img) => {
      const rect = img.getBoundingClientRect();
      const cs = window.getComputedStyle(img);
      const attrW = Number(img.getAttribute('width') ?? '0');
      const attrH = Number(img.getAttribute('height') ?? '0');
      const natRatio = img.naturalHeight > 0 ? img.naturalWidth / img.naturalHeight : 0;
      const renRatio = rect.height > 0 ? rect.width / rect.height : 0;
      const attrRatio = attrH > 0 ? attrW / attrH : 0;
      return {
        src: img.getAttribute('src') ?? '',
        alt: img.getAttribute('alt') ?? '',
        hasWidthAttr: img.hasAttribute('width'),
        hasHeightAttr: img.hasAttribute('height'),
        attrWidth: attrW,
        attrHeight: attrH,
        naturalWidth: img.naturalWidth,
        naturalHeight: img.naturalHeight,
        renderedWidth: Number(rect.width.toFixed(1)),
        renderedHeight: Number(rect.height.toFixed(1)),
        loadingAttr: img.getAttribute('loading'),
        decodingAttr: img.getAttribute('decoding'),
        fetchPriorityAttr: img.getAttribute('fetchpriority'),
        objectFit: cs.objectFit,
        complete: img.complete,
        inInitialViewport: rect.top < window.innerHeight,
        aspectDrift: Number(Math.abs(natRatio - renRatio).toFixed(4)),
        attrAspectDrift: attrRatio > 0 ? Number(Math.abs(natRatio - attrRatio).toFixed(4)) : 1,
      };
    });
    return {
      clsAfterScroll: Number((perfBag?.cls ?? 0).toFixed(5)),
      images: audits,
    };
  });

  await cdp.detach();

  return {
    ...initialEval,
    eagerRequestCount,
    eagerTotalBytes,
    eagerImageBytes,
    htmlBytes,
    jsBytes,
    cssBytes,
    fontBytes,
    totalScrolledTransferBytes,
    totalScrolledImageBytes,
    clsAfterScroll: afterScroll.clsAfterScroll,
    images: afterScroll.images,
  };
}

test.describe('Phase 533 — voxlang.org Frontpage Load-Time Benchmark & CV Flywheel', () => {
  test(`captures ${STAGE.toUpperCase()} performance + CV audit metrics and verifies budgets`, async ({
    browser,
  }) => {
    test.setTimeout(120_000);

    const wanRuns: RunMetrics[] = [];
    for (let i = 0; i < 3; i++) {
      const ctx = await browser.newContext();
      const page = await ctx.newPage();
      try {
        wanRuns.push(await measureFrontpageRun(page, true));
      } finally {
        await ctx.close();
      }
    }

    const localRuns: RunMetrics[] = [];
    for (let i = 0; i < 3; i++) {
      const ctx = await browser.newContext();
      const page = await ctx.newPage();
      try {
        localRuns.push(await measureFrontpageRun(page, false));
      } finally {
        await ctx.close();
      }
    }

    // Visual & Responsive CV check (desktop light/dark + mobile 390x844)
    const cvChecks: Array<{ viewport: string; theme: string; overflow: boolean; brokenCount: number }> = [];
    for (const [vpName, vp] of [
      ['1440x900', { width: 1440, height: 900 }],
      ['390x844', { width: 390, height: 844 }],
    ] as const) {
      for (const theme of ['dark', 'light'] as const) {
        const ctx = await browser.newContext({ viewport: vp });
        const page = await ctx.newPage();
        await page.goto('/', { waitUntil: 'load' });
        await page.evaluate((t) => {
          document.documentElement.dataset.theme = t;
        }, theme);
        const overflow = await page.evaluate(
          () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
        );
        const brokenCount = await page.evaluate(async () => {
          const imgs = Array.from(document.querySelectorAll('main img')) as HTMLImageElement[];
          for (const img of imgs) {
            img.scrollIntoView({ behavior: 'instant', block: 'center' });
            if (!img.complete) {
              await new Promise<void>((res) => {
                img.addEventListener('load', () => res(), { once: true });
                img.addEventListener('error', () => res(), { once: true });
                setTimeout(() => res(), 5000);
              });
            }
          }
          return imgs.filter((img) => !img.complete || img.naturalWidth === 0).length;
        });
        cvChecks.push({ viewport: vpName, theme, overflow, brokenCount });
        await ctx.close();
      }
    }

    const summary = {
      stage: STAGE,
      recordedAt: new Date().toISOString(),
      wanProfile: '10 Mbps down / 5 Mbps up / 40ms RTT / cache disabled',
      wanMedian: {
        ttfbMs: median(wanRuns.map((r) => r.ttfbMs)),
        fcpMs: median(wanRuns.map((r) => r.fcpMs)),
        lcpMs: median(wanRuns.map((r) => r.lcpMs)),
        domContentLoadedMs: median(wanRuns.map((r) => r.domContentLoadedMs)),
        loadEventMs: median(wanRuns.map((r) => r.loadEventMs)),
        clsScore: median(wanRuns.map((r) => r.clsScore)),
        clsAfterScroll: median(wanRuns.map((r) => r.clsAfterScroll)),
        eagerRequestCount: wanRuns[0].eagerRequestCount,
        eagerTotalBytes: median(wanRuns.map((r) => r.eagerTotalBytes)),
        eagerImageBytes: median(wanRuns.map((r) => r.eagerImageBytes)),
        htmlBytes: median(wanRuns.map((r) => r.htmlBytes)),
        jsBytes: median(wanRuns.map((r) => r.jsBytes)),
        cssBytes: median(wanRuns.map((r) => r.cssBytes)),
        fontBytes: median(wanRuns.map((r) => r.fontBytes)),
        totalScrolledTransferBytes: median(wanRuns.map((r) => r.totalScrolledTransferBytes)),
        totalScrolledImageBytes: median(wanRuns.map((r) => r.totalScrolledImageBytes)),
        preloadCount: wanRuns[0].preloadCount,
      },
      localMedian: {
        fcpMs: median(localRuns.map((r) => r.fcpMs)),
        lcpMs: median(localRuns.map((r) => r.lcpMs)),
        domContentLoadedMs: median(localRuns.map((r) => r.domContentLoadedMs)),
        loadEventMs: median(localRuns.map((r) => r.loadEventMs)),
      },
      wanRuns,
      localRuns,
      images: wanRuns[0].images,
      cvChecks,
    };

    const outDir = process.env.CI ? join(process.cwd(), 'test-results') : SCRATCH_DIR;
    mkdirSync(outDir, { recursive: true });
    writeFileSync(join(outDir, `frontpage-perf-${STAGE}.json`), JSON.stringify(summary, null, 2));

    console.log(`[PERF:${STAGE}] WAN median:`, JSON.stringify(summary.wanMedian));
    console.log(`[PERF:${STAGE}] Local median:`, JSON.stringify(summary.localMedian));

    // In both BEFORE and AFTER, images must decode and viewports must not overflow
    for (const c of cvChecks) {
      expect(c.overflow, `Horizontal overflow at ${c.viewport} (${c.theme})`).toBe(false);
      expect(c.brokenCount, `Broken images at ${c.viewport} (${c.theme})`).toBe(0);
    }

    if (STAGE === 'after') {
      expect(summary.wanMedian.eagerImageBytes, 'Eager image payload budget').toBeLessThan(200_000);
      expect(summary.wanMedian.eagerTotalBytes, 'Total initial transfer budget').toBeLessThan(500_000);
      expect(summary.wanMedian.clsAfterScroll, 'Cumulative Layout Shift budget').toBeLessThanOrEqual(0.015);
      expect(summary.wanMedian.preloadCount, 'Hero banner preload hint count').toBeGreaterThanOrEqual(1);

      const imgs = summary.images;
      expect(imgs.length).toBeGreaterThanOrEqual(5);
      for (const img of imgs) {
        expect(img.hasWidthAttr, `${img.src} missing width attr`).toBe(true);
        expect(img.hasHeightAttr, `${img.src} missing height attr`).toBe(true);
        expect(img.naturalWidth, `${img.src} failed to load`).toBeGreaterThan(0);
        expect(img.objectFit, `${img.src} must not crop via cover`).not.toBe('cover');
        expect(img.aspectDrift, `${img.src} rendered aspect distortion`).toBeLessThanOrEqual(0.03);
        expect(img.attrAspectDrift, `${img.src} width/height attr aspect drift`).toBeLessThanOrEqual(0.03);
        if (img.src.includes('vox_hero_banner')) {
          expect(img.fetchPriorityAttr, 'Hero banner must have fetchpriority=high').toBe('high');
          expect(img.loadingAttr, 'Hero banner must not lazy load').not.toBe('lazy');
        } else {
          expect(img.loadingAttr, `${img.src} below-fold image must lazy load`).toBe('lazy');
          expect(img.decodingAttr, `${img.src} must decode async`).toBe('async');
        }
      }
    }
  });
});
