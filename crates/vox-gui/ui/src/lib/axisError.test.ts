import { describe, it, expect } from 'vitest';
import { parseAxisError } from './axisError';

describe('parseAxisError', () => {
  const envelope = JSON.stringify({ error: { code: 'tls_pin_required', message: 'first', fingerprint: 'ab' } });

  it('decodes a rejected string envelope', () => {
    expect(parseAxisError(envelope)).toEqual({ code: 'tls_pin_required', message: 'first', fingerprint: 'ab' });
  });

  it('decodes an Error whose message is the envelope', () => {
    expect(parseAxisError(new Error(envelope))?.code).toBe('tls_pin_required');
  });

  it('returns null for plain strings and non-envelope JSON', () => {
    expect(parseAxisError('daemon unreachable')).toBeNull();
    expect(parseAxisError(JSON.stringify({ ok: true }))).toBeNull();
    expect(parseAxisError(undefined)).toBeNull();
  });
});
