// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { checkHonestyInvariant } from './sentryHonesty';

describe('checkHonestyInvariant', () => {
  it('returns null when step is not completed', () => {
    const violation = checkHonestyInvariant({
      status: 'active',
      payload: {},
      requiredFields: ['sources'],
      stageName: 'retrieving',
    });
    expect(violation).toBeNull();
  });

  it('flags fake success when required sources are empty without empty-notice', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [] },
      requiredFields: ['sources'],
      stageName: 'retrieving',
    });
    expect(violation).not.toBeNull();
    expect(violation?.kind).toBe('fake_success');
    expect(violation?.severity).toBe('critical');
  });

  it('flags fake success when required field is empty string', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { query: '' },
      requiredFields: ['query'],
      stageName: 'planning',
    });
    expect(violation).not.toBeNull();
    expect(violation?.kind).toBe('fake_success');
  });

  it('does not treat 0 or false as empty field', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { count: 0, active: false },
      requiredFields: ['count', 'active'],
      stageName: 'executing',
    });
    expect(violation).toBeNull();
  });

  it('allows empty results when emptyStateAcknowledged is true', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [], emptyStateAcknowledged: true },
      requiredFields: ['sources'],
      stageName: 'retrieving',
    });
    expect(violation).toBeNull();
  });

  it('allows empty results when domContainer renders empty-results-notice child', () => {
    const container = document.createElement('div');
    const notice = document.createElement('div');
    notice.setAttribute('data-testid', 'empty-results-notice');
    container.appendChild(notice);

    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [] },
      requiredFields: ['sources'],
      stageName: 'retrieving',
      domContainer: container,
    });
    expect(violation).toBeNull();
  });

  it('allows empty results when domContainer itself matches empty-results-notice', () => {
    const container = document.createElement('div');
    container.setAttribute('data-testid', 'empty-results-notice');

    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [] },
      requiredFields: ['sources'],
      stageName: 'retrieving',
      domContainer: container,
    });
    expect(violation).toBeNull();
  });

  it('flags fake success if providers failed completely even when requiredFields is empty', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { unrelated: 'data' },
      requiredFields: [],
      stageName: 'retrieving',
      providerProbeStatuses: [{ provider: 'searxng', ok: false, httpStatus: 502, hitCount: 0 }],
    });
    expect(violation).not.toBeNull();
    expect(violation?.message).toContain('all search providers failed');
  });

  it('flags fake success if providers failed completely even when payload is populated', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: ['hallucinated result'] },
      requiredFields: ['sources'],
      stageName: 'retrieving',
      providerProbeStatuses: [{ provider: 'searxng', ok: false, httpStatus: 502, hitCount: 0 }],
    });
    expect(violation).not.toBeNull();
    expect(violation?.message).toContain('all search providers failed');
  });
});
