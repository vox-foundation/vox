import { test, expect } from '@playwright/test';
import { readDist } from '../lib/dist';

const hrefs = (html: string) => [...html.matchAll(/\bhref="([^"]*)"/g)].map((m) => m[1]);
const isRelative = (href: string) => !/^([a-z][a-z0-9+.-]*:|\/|#)/i.test(href);
const isMarkdown = (href: string) => /\.mdx?(#|$)/.test(href);

test.describe('repo-relative links', () => {
  test('tutorial links to other docs pages render as site routes', () => {
    const links = hrefs(readDist('tutorials/tut-getting-started'));
    expect(links).toContain('/reference/installation/');
    expect(links.filter((href) => isRelative(href) && isMarkdown(href))).toEqual([]);
  });
});
