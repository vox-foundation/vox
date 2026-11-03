import { describe, expect, it } from 'vitest';
import { isLoopbackPreviewUrl, previewUrlHost } from './previewUrl';

describe('isLoopbackPreviewUrl', () => {
  it('accepts loopback hosts', () => {
    expect(isLoopbackPreviewUrl('http://127.0.0.1:3000')).toBe(true);
    expect(isLoopbackPreviewUrl('http://localhost:5173/app')).toBe(true);
    expect(isLoopbackPreviewUrl('http://[::1]:3000')).toBe(true);
  });

  it('rejects remote, file, and backslash authority', () => {
    expect(isLoopbackPreviewUrl('https://evil.example')).toBe(false);
    expect(isLoopbackPreviewUrl('https://example.com')).toBe(false);
    expect(isLoopbackPreviewUrl('file:///etc/passwd')).toBe(false);
    expect(previewUrlHost('http://evil\\example')).toBe('');
    expect(isLoopbackPreviewUrl('http://evil\\example')).toBe(false);
  });
});
