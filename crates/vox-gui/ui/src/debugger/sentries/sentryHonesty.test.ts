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

  it('allows empty results when emptyStateAcknowledged is true', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [], emptyStateAcknowledged: true },
      requiredFields: ['sources'],
      stageName: 'retrieving',
    });
    expect(violation).toBeNull();
  });

  it('allows empty results when domContainer renders empty-results-notice', () => {
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

  it('flags fake success if providers failed completely', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [] },
      requiredFields: ['sources'],
      stageName: 'retrieving',
      providerProbeStatuses: [{ provider: 'searxng', ok: false, httpStatus: 502, hitCount: 0 }],
    });
    expect(violation?.message).toContain('all search providers failed');
  });
});
