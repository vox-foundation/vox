// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SafeExternalLink, isSafeUrl } from './SafeExternalLink';

describe('SafeExternalLink and isSafeUrl', () => {
  it('validates safe http and https protocols', () => {
    expect(isSafeUrl('https://docs.rs/tokio')).toBe(true);
    expect(isSafeUrl('http://example.com/api')).toBe(true);
  });

  it('rejects dangerous protocols', () => {
    expect(isSafeUrl('javascript:alert(1)')).toBe(false);
    expect(isSafeUrl('data:text/html,<script>alert(1)</script>')).toBe(false);
    expect(isSafeUrl('vbscript:msgbox(1)')).toBe(false);
    expect(isSafeUrl('file:///etc/passwd')).toBe(false);
    expect(isSafeUrl('')).toBe(false);
  });

  it('renders anchor tag with noopener noreferrer for safe URLs', () => {
    render(<SafeExternalLink url="https://docs.rs/tokio">Tokio Docs</SafeExternalLink>);
    const link = screen.getByRole('link', { name: /tokio docs/i });
    expect(link.getAttribute('href')).toBe('https://docs.rs/tokio');
    expect(link.getAttribute('target')).toBe('_blank');
    expect(link.getAttribute('rel')).toBe('noopener noreferrer');
  });

  it('renders safe span instead of anchor for malicious URLs', () => {
    render(<SafeExternalLink url="javascript:alert('pwned')">Exploit</SafeExternalLink>);
    expect(screen.queryByRole('link')).toBeNull();
    expect(screen.getByText('Exploit')).toBeTruthy();
  });

  it('normalizes bare DOIs and doi: prefixes to https://doi.org/', () => {
    expect(isSafeUrl('10.1038/s41586-020-2649-2')).toBe(true);
    expect(isSafeUrl('doi:10.1038/s41586-020-2649-2')).toBe(true);

    render(<SafeExternalLink url="10.1038/s41586-020-2649-2">Nature Paper</SafeExternalLink>);
    const link = screen.getByRole('link', { name: /nature paper/i });
    expect(link.getAttribute('href')).toBe('https://doi.org/10.1038/s41586-020-2649-2');
  });
});
