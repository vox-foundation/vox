import test from 'node:test';
import assert from 'node:assert/strict';
import { extractLlmsUrls, checkUrls } from '../lib/llms-links.mjs';

test('extractLlmsUrls extracts and dedupes origin URLs and rewrites baseUrl', () => {
  const sample = `
# Sample Index
- [Getting Started](https://voxlang.org/tutorials/tut-getting-started/)
- [CLI Reference](https://voxlang.org/reference/cli/)
- [Duplicate](https://voxlang.org/reference/cli/)
- [External](https://github.com/vox-foundation/vox)
- Bare URL: https://voxlang.org/reference/stability/
- Bare external: https://google.com/test
- Anchor URL: https://voxlang.org/tutorials/tut-getting-started/#install
`;

  const defaultExtracted = extractLlmsUrls(sample);
  assert.equal(defaultExtracted.length, 3);
  assert.ok(defaultExtracted.includes('https://voxlang.org/tutorials/tut-getting-started/'));
  assert.ok(defaultExtracted.includes('https://voxlang.org/reference/cli/'));
  assert.ok(defaultExtracted.includes('https://voxlang.org/reference/stability/'));

  // Test baseUrl rewriting
  const rewritten = extractLlmsUrls(sample, { baseUrl: 'http://localhost:4321' });
  assert.equal(rewritten.length, 3);
  assert.ok(rewritten.includes('http://localhost:4321/tutorials/tut-getting-started/'));
  assert.ok(rewritten.includes('http://localhost:4321/reference/cli/'));
  assert.ok(rewritten.includes('http://localhost:4321/reference/stability/'));
});

test('checkUrls returns ok when all URLs return 200', async () => {
  const urls = [
    'https://voxlang.org/a',
    'https://voxlang.org/b',
    'https://voxlang.org/c',
  ];

  const fakeFetcher = async (url) => ({ status: 200 });
  const result = await checkUrls(urls, fakeFetcher);

  assert.equal(result.ok, true);
  assert.equal(result.failures.length, 0);
});

test('checkUrls fails and reports URL when fetcher returns 404', async () => {
  const urls = [
    'https://voxlang.org/good',
    'https://voxlang.org/bad-404',
  ];

  const fakeFetcher = async (url) => {
    if (url.includes('bad-404')) {
      return { status: 404 };
    }
    return { status: 200 };
  };

  const result = await checkUrls(urls, fakeFetcher);
  assert.equal(result.ok, false);
  assert.equal(result.failures.length, 1);
  assert.equal(result.failures[0].url, 'https://voxlang.org/bad-404');
  assert.equal(result.failures[0].status, 404);
});

test('checkUrls fails and reports URL when fetcher throws error', async () => {
  const urls = [
    'https://voxlang.org/good',
    'https://voxlang.org/throw-err',
  ];

  const fakeFetcher = async (url) => {
    if (url.includes('throw-err')) {
      throw new Error('Connection refused');
    }
    return { status: 200 };
  };

  const result = await checkUrls(urls, fakeFetcher);
  assert.equal(result.ok, false);
  assert.equal(result.failures.length, 1);
  assert.equal(result.failures[0].url, 'https://voxlang.org/throw-err');
  assert.ok(result.failures[0].error.includes('Connection refused'));
});
