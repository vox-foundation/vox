// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { checkErrorLeakInvariant } from './sentryErrorLeak';

describe('checkErrorLeakInvariant', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('returns empty array when root is null', () => {
    const violations = checkErrorLeakInvariant(null);
    expect(violations).toEqual([]);
  });

  it('detects raw error leak in alert banner with descriptive elementSelector', () => {
    const alert = document.createElement('div');
    alert.setAttribute('role', 'alert');
    alert.textContent = 'TypeError: Cannot read properties of undefined';
    document.body.appendChild(alert);

    const violations = checkErrorLeakInvariant(document.body);
    expect(violations).toHaveLength(1);
    expect(violations[0].kind).toBe('raw_error_leak');
    expect(violations[0].elementSelector).toBe('div[role="alert"]');
  });

  it('detects LEAK_PATTERN in toast item with testid elementSelector', () => {
    const toast = document.createElement('div');
    toast.setAttribute('data-testid', 'toast-item');
    toast.textContent = 'Failed to invoke command on backend';
    document.body.appendChild(toast);

    const violations = checkErrorLeakInvariant(document.body);
    expect(violations).toHaveLength(1);
    expect(violations[0].kind).toBe('raw_error_leak');
    expect(violations[0].elementSelector).toBe('[data-testid="toast-item"]');
  });

  it('detects header error element', () => {
    const header = document.createElement('header');
    const headerError = document.createElement('span');
    headerError.setAttribute('data-testid', 'header-error');
    headerError.textContent = 'TypeError: Backend connection reset';
    header.appendChild(headerError);
    document.body.appendChild(header);

    const violations = checkErrorLeakInvariant(document.body);
    expect(violations).toHaveLength(1);
    expect(violations[0].elementSelector).toBe('[data-testid="header-error"]');
  });

  it('does not flag legitimate header buttons containing "invoke"', () => {
    const header = document.createElement('header');
    const button = document.createElement('button');
    button.textContent = 'Invoke Agent';
    header.appendChild(button);
    document.body.appendChild(header);

    const violations = checkErrorLeakInvariant(document.body);
    expect(violations).toHaveLength(0);
  });

  it('ignores errors inside excluded prose content or chat transcripts', () => {
    const transcript = document.createElement('div');
    transcript.setAttribute('data-testid', 'chat-transcript');
    const msg = document.createElement('pre');
    msg.textContent = 'Here is how to fix TypeError: foo is not a function';
    transcript.appendChild(msg);
    document.body.appendChild(transcript);

    const violations = checkErrorLeakInvariant(document.body);
    expect(violations).toHaveLength(0);
  });
});
