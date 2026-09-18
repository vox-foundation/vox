// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { checkErrorLeakInvariant } from './sentryErrorLeak';

describe('checkErrorLeakInvariant', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('detects raw error leak in alert banner', () => {
    const alert = document.createElement('div');
    alert.setAttribute('role', 'alert');
    alert.textContent = 'TypeError: Cannot read properties of undefined';
    document.body.appendChild(alert);

    const violations = checkErrorLeakInvariant(document.body);
    expect(violations).toHaveLength(1);
    expect(violations[0].kind).toBe('raw_error_leak');
  });

  it('detects LEAK_PATTERN in toast item', () => {
    const toast = document.createElement('div');
    toast.setAttribute('data-testid', 'toast-item');
    toast.textContent = 'Failed to invoke command on backend';
    document.body.appendChild(toast);

    const violations = checkErrorLeakInvariant(document.body);
    expect(violations).toHaveLength(1);
    expect(violations[0].kind).toBe('raw_error_leak');
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
