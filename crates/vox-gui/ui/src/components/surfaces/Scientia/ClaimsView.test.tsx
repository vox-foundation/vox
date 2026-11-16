import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';

const CLAIM = {
  claim_id: 1,
  text: 'The widget improves throughput by 12%.',
  is_numeric: true,
  verifiability_score: 0.8,
  verdict: 'Supported',
  confidence: 0.9,
  verifier_model: 'm1',
  created_at_ms: 0,
};

const invokeMock = vi.fn((cmd: string, _args?: unknown) => {
  if (cmd === 'scientia_publication_claims') {
    return Promise.resolve({
      publication_id: 'pub-1',
      session_id: 1,
      claims: [CLAIM],
    });
  }
  if (cmd === 'scientia_publication_extract_claims') {
    return Promise.resolve({
      publication_id: 'pub-1',
      total_atomic: 1,
      supported: 1,
      verifier_model: 'calibrated-hybrid:lexical',
    });
  }
  return Promise.resolve(null);
});
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

import { ClaimsView } from './ClaimsView';
import { LanguageProvider } from '../../../hooks/useLanguage';

describe('ClaimsView', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockClear();
  });

  it('action buttons are explicit type="button"', () => {
    render(<LanguageProvider><ClaimsView pushToast={vi.fn()} /></LanguageProvider>);
    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('renders initialClaims directly without invoking backend', () => {
    const claims = [
      { ...CLAIM, claim_id: 10, text: 'Direct claim 10', verdict: 'Supported' },
      { ...CLAIM, claim_id: 11, text: 'Direct claim 11', verdict: 'Contradicted' },
    ];
    render(
      <LanguageProvider>
        <ClaimsView initialClaims={claims} />
      </LanguageProvider>,
    );
    expect(screen.getByText('Direct claim 10')).toBeTruthy();
    expect(screen.getByText('Direct claim 11')).toBeTruthy();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('filters claims by verdict tabs (All, Supported, Contradicted, Unverified)', () => {
    const claims = [
      { ...CLAIM, claim_id: 1, text: 'Supported fact', verdict: 'Supported' },
      { ...CLAIM, claim_id: 2, text: 'Contradicted assertion', verdict: 'Contradicted' },
      { ...CLAIM, claim_id: 3, text: 'Unverified speculation', verdict: 'Unverified' },
    ];
    render(
      <LanguageProvider>
        <ClaimsView initialClaims={claims} />
      </LanguageProvider>,
    );

    // Initial state: 'all' filter active
    expect(screen.getByText('Supported fact')).toBeTruthy();
    expect(screen.getByText('Contradicted assertion')).toBeTruthy();
    expect(screen.getByText('Unverified speculation')).toBeTruthy();

    // Click 'Supported' tab
    fireEvent.click(screen.getByRole('tab', { name: 'Supported' }));
    expect(screen.getByText('Supported fact')).toBeTruthy();
    expect(screen.queryByText('Contradicted assertion')).toBeNull();
    expect(screen.queryByText('Unverified speculation')).toBeNull();

    // Click 'Contradicted' tab
    fireEvent.click(screen.getByRole('tab', { name: 'Contradicted' }));
    expect(screen.queryByText('Supported fact')).toBeNull();
    expect(screen.getByText('Contradicted assertion')).toBeTruthy();
    expect(screen.queryByText('Unverified speculation')).toBeNull();

    // Click 'Unverified' tab
    fireEvent.click(screen.getByRole('tab', { name: 'Unverified' }));
    expect(screen.queryByText('Supported fact')).toBeNull();
    expect(screen.queryByText('Contradicted assertion')).toBeNull();
    expect(screen.getByText('Unverified speculation')).toBeTruthy();

    // Click 'all' tab
    fireEvent.click(screen.getByRole('tab', { name: 'all' }));
    expect(screen.getByText('Supported fact')).toBeTruthy();
    expect(screen.getByText('Contradicted assertion')).toBeTruthy();
    expect(screen.getByText('Unverified speculation')).toBeTruthy();
  });

  it('sanitizes citation links against javascript: and data: XSS payloads', () => {
    const claims = [
      { ...CLAIM, claim_id: 1, text: 'Safe link claim', source_url: 'https://example.com/source' },
      { ...CLAIM, claim_id: 2, text: 'Malicious js claim', source_url: 'javascript:alert("pwned")' },
      { ...CLAIM, claim_id: 3, text: 'Malicious data claim', source_url: 'data:text/html,<script>alert(1)</script>' },
    ];
    render(
      <LanguageProvider>
        <ClaimsView initialClaims={claims} />
      </LanguageProvider>,
    );

    const safeLink = screen.getByTestId('safe-url');
    expect(safeLink.getAttribute('href')).toBe('https://example.com/source');

    const unsafeSpans = screen.getAllByTestId('unsafe-url');
    expect(unsafeSpans).toHaveLength(2);
    expect(unsafeSpans[0].tagName.toLowerCase()).toBe('span');
    expect(unsafeSpans[1].tagName.toLowerCase()).toBe('span');
    expect(screen.queryByRole('link', { name: /javascript/i })).toBeNull();
    expect(screen.queryByRole('link', { name: /data:text/i })).toBeNull();
  });

  it('renders loaded claims from backend inside an aria-live list via typed scientia_publication_claims RPC', async () => {
    render(
      <LanguageProvider>
        <ClaimsView pushToast={vi.fn()} />
      </LanguageProvider>,
    );
    fireEvent.change(screen.getByPlaceholderText('publication id'), { target: { value: 'pub-1' } });
    fireEvent.click(screen.getByText('Load'));
    const claim = await screen.findByText('The widget improves throughput by 12%.');
    expect(claim).toBeTruthy();
    expect(
      invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_claims'),
    ).toBe(true);
    await waitFor(() => {
      expect(screen.getByRole('list')).toBeTruthy();
    });
  });

  it('extracts atomic claims via typed scientia_publication_extract_claims RPC and reloads claims', async () => {
    const pushToast = vi.fn();
    render(
      <LanguageProvider>
        <ClaimsView pushToast={pushToast} />
      </LanguageProvider>,
    );
    fireEvent.change(screen.getByPlaceholderText('publication id'), { target: { value: 'pub-1' } });
    fireEvent.click(screen.getByText('Extract claims'));
    await waitFor(() => {
      expect(
        invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_extract_claims'),
      ).toBe(true);
    });
    expect(await screen.findByText('The widget improves throughput by 12%.')).toBeTruthy();
  });

  it('displays empty filter fallback when no claims match', () => {
    const claims = [
      { ...CLAIM, claim_id: 1, text: 'Only supported fact', verdict: 'Supported' },
    ];
    render(
      <LanguageProvider>
        <ClaimsView initialClaims={claims} />
      </LanguageProvider>,
    );
    fireEvent.click(screen.getByRole('tab', { name: 'Contradicted' }));
    expect(screen.getByText('No claims matching verdict filter "Contradicted".')).toBeTruthy();
  });
});
