import { test, expect } from '@playwright/test';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { readDist, listDistHtml, DIST_DIR } from '../lib/dist';

const READER_PREFIXES = ['index.html', 'tutorials/', 'how-to/', 'explanation/', 'contributors/'];

function isReaderFacingRel(rel: string): boolean {
  return READER_PREFIXES.some((p) => (p === 'index.html' ? rel === p : rel.startsWith(p)));
}

test.describe('Phase 530 — .mdx Routes, Anchors, Images, Single-H1 & Zero Backtick Leaks', () => {
  test('every internal href, #anchor, and <img> across index, tutorials, how-to, explanation, and contributors resolves in dist/', () => {
    const allHtml = listDistHtml();
    const idsByRoute = new Map<string, Set<string>>();

    for (const rel of allHtml) {
      const html = readFileSync(join(DIST_DIR, rel), 'utf8');
      const ids = new Set<string>();
      for (const m of html.matchAll(/\bid=["']([^"']+)["']/g)) {
        ids.add(m[1]);
      }
      const route = '/' + rel.replace(/(?:^|\/)index\.html$/, (s) => (s.startsWith('/') ? '/' : ''));
      const normRoute = route === '/' ? '/' : route.endsWith('/') ? route : `${route}/`;
      idsByRoute.set(normRoute, ids);
    }

    const badLinks: string[] = [];
    const badAnchors: string[] = [];
    const badImages: string[] = [];

    for (const rel of allHtml.filter(isReaderFacingRel)) {
      const filePath = join(DIST_DIR, rel);
      const html = readFileSync(filePath, 'utf8');
      const baseRouteRaw = '/' + rel.replace(/(?:^|\/)index\.html$/, (s) => (s.startsWith('/') ? '/' : ''));
      const baseRoute = baseRouteRaw === '/' ? '/' : baseRouteRaw.endsWith('/') ? baseRouteRaw : `${baseRouteRaw}/`;

      for (const m of html.matchAll(/\bhref=["']([^"']+)["']/g)) {
        const href = m[1];
        if (/^(https?:|mailto:|javascript:|tel:|\/\/)/.test(href)) continue;

        const [rawPathAndQuery, rawHash] = href.split('#');
        const rawPath = rawPathAndQuery.split('?')[0];

        let targetRoute = rawPath
          ? rawPath.startsWith('/')
            ? rawPath
            : resolve(baseRoute, rawPath)
          : baseRoute;

        if (!/\.[a-z0-9]+$/i.test(targetRoute) && !targetRoute.endsWith('/')) {
          targetRoute += '/';
        }

        if (rawPath) {
          const diskTarget = targetRoute.endsWith('/')
            ? join(DIST_DIR, targetRoute, 'index.html')
            : join(DIST_DIR, targetRoute);
          if (!existsSync(diskTarget)) {
            badLinks.push(`${rel} -> ${href}`);
            continue;
          }
        }

        if (rawHash && targetRoute.endsWith('/')) {
          const targetIds = idsByRoute.get(targetRoute);
          if (targetIds && !targetIds.has(rawHash) && !targetIds.has(decodeURIComponent(rawHash))) {
            badAnchors.push(`${rel} -> ${href}`);
          }
        }
      }

      for (const m of html.matchAll(/<img\b[^>]*\bsrc=["']([^"']+)["']/g)) {
        const src = m[1];
        if (/^(https?:|data:|\/\/)/.test(src)) continue;
        const cleanSrc = src.split(/[?#]/)[0];
        const diskImg = cleanSrc.startsWith('/') ? join(DIST_DIR, cleanSrc) : resolve(dirname(filePath), cleanSrc);
        if (!existsSync(diskImg) || statSync(diskImg).size === 0) {
          badImages.push(`${rel} -> ${src}`);
        }
      }
    }

    expect(badLinks, `Unresolved internal links:\n${badLinks.join('\n')}`).toEqual([]);
    expect(badAnchors, `Broken #anchor fragments:\n${badAnchors.join('\n')}`).toEqual([]);
    expect(badImages, `Missing or empty images:\n${badImages.join('\n')}`).toEqual([]);
  });

  test('.mdx tutorials render strictly a single <h1> (_top) without duplicate body headings', () => {
    const mdxTutorials = [
      'tutorials/tut-fullstack-walkthrough',
      'tutorials/tut-deep-research-and-scientia',
      'tutorials/tut-knowledgebase-and-openwiki',
    ];

    for (const route of mdxTutorials) {
      const html = readDist(route);
      const h1Matches = html.match(/<h1\b[^>]*>/gi) ?? [];
      expect(h1Matches.length, `Expected single <h1> on ${route}, saw ${h1Matches.length}`).toBe(1);
    }
  });

  test('showcase cards and Starlight cards on index, tutorials, and how-to contain zero literal backtick leaks', () => {
    const targetPages = listDistHtml().filter(
      (rel) => rel === 'index.html' || rel.startsWith('tutorials/') || rel.startsWith('how-to/'),
    );
    const leaks: string[] = [];

    for (const rel of targetPages) {
      const html = readFileSync(join(DIST_DIR, rel), 'utf8');
      const cardBlocks = [
        ...html.matchAll(/<div class="vox-showcase-body"[\s\S]*?<\/div>\s*<\/div>/g),
        ...html.matchAll(/<div class="vox-media-caption"[\s\S]*?<\/div>/g),
        ...html.matchAll(/<article class="card[\s\S]*?<\/article>/g),
      ];
      for (const block of cardBlocks) {
        const stripped = block[0]
          .replace(/<code\b[^>]*>[\s\S]*?<\/code>/gi, ' ')
          .replace(/<pre\b[^>]*>[\s\S]*?<\/pre>/gi, ' ')
          .replace(/<[^>]+>/g, ' ');
        for (const tick of stripped.matchAll(/`[^`\n]{1,60}`/g)) {
          leaks.push(`${rel} -> ${tick[0]}`);
        }
      }
    }

    expect(leaks, `Found unrendered backticks inside card UI:\n${leaks.join('\n')}`).toEqual([]);
  });
});
