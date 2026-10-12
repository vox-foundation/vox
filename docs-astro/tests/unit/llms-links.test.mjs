import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkUrls, extractLlmsUrls } from '../lib/llms-links.mjs';

const SAMPLE = `# Vox
> Maturity: https://voxlang.org/reference/stability/ — see also https://voxlang.org/reference/stability/.
- [AGENTS.md](https://voxlang.org/repo/agents-md/): policy
- [Syntax](https://voxlang.org/reference/ref-syntax/#tables "Syntax"): spec
- [Duplicate](https://voxlang.org/repo/agents-md/): again
- [GitHub](https://github.com/vox-foundation/vox): external
- Not ours: https://www.voxlang.org/ and http://voxlang.org.evil.example/x
- Plain text in parentheses (https://voxlang.org/how-to/).
`;

test('extractLlmsUrls keeps same-origin Markdown and bare URLs, deduped, fragments dropped', () => {
  assert.deepEqual(extractLlmsUrls(SAMPLE), [
    'https://voxlang.org/how-to/',
    'https://voxlang.org/reference/ref-syntax/',
    'https://voxlang.org/reference/stability/',
    'https://voxlang.org/repo/agents-md/',
  ]);
});

test('extractLlmsUrls rewrites the origin to baseUrl', () => {
  const urls = extractLlmsUrls('[a](https://voxlang.org/a/) https://voxlang.org/b/', {
    baseUrl: 'http://localhost:4321/',
  });
  assert.deepEqual(urls, ['http://localhost:4321/a/', 'http://localhost:4321/b/']);
});

test('extractLlmsUrls honours a custom origin', () => {
  const urls = extractLlmsUrls('https://example.org/x https://voxlang.org/y', { origin: 'https://example.org' });
  assert.deepEqual(urls, ['https://example.org/x']);
});

test('checkUrls is ok when every URL answers 2xx', async () => {
  const result = await checkUrls(['u1', 'u2', 'u3'], async () => ({ status: 200 }));
  assert.deepEqual(result, { ok: true, failures: [] });
});

test('checkUrls reports a 404 (a broken llms link fails the check)', async () => {
  const result = await checkUrls(['ok-1', 'broken', 'ok-2'], async (url) => ({ status: url === 'broken' ? 404 : 200 }));
  assert.equal(result.ok, false);
  assert.deepEqual(result.failures, [{ url: 'broken', status: 404 }]);
});

test('checkUrls reports a fetcher that throws', async () => {
  const result = await checkUrls(['ok', 'boom'], async (url) => {
    if (url === 'boom') throw new Error('ECONNREFUSED');
    return { status: 204 };
  });
  assert.equal(result.ok, false);
  assert.equal(result.failures.length, 1);
  assert.equal(result.failures[0].url, 'boom');
  assert.match(result.failures[0].error, /ECONNREFUSED/);
});

test('checkUrls never runs more than 8 fetches at once', async () => {
  let inFlight = 0;
  let peak = 0;
  const urls = Array.from({ length: 30 }, (_, i) => `u${i}`);
  const result = await checkUrls(urls, async () => {
    inFlight += 1;
    peak = Math.max(peak, inFlight);
    await new Promise((resolve) => setTimeout(resolve, 2));
    inFlight -= 1;
    return { status: 200 };
  });
  assert.equal(result.ok, true);
  assert.ok(peak <= 8, `peak concurrency ${peak}`);
  assert.ok(peak > 1, 'fetches should overlap');
});
